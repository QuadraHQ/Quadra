"""One run of the Roblox client through Darling: preparing it, starting it and watching it."""

import json
import logging
import os
import re
import subprocess
import threading
import time

from .. import __version__
from ..darling.runtime import guest_variables
from ..gpus import GPU_ENVIRONMENT
from ..i18n import _
from ..transport_monitor import TransportMonitor
from . import (
    audio,
    builds,
    config,
    darling_server,
    host,
    locations,
    logs,
    prefix_files,
    process_control,
    roblox,
    x11,
)

__all__ = [
    "LAUNCH_SCRIPT",
    "QUIT_GRACE_SECONDS",
    "RobloxSession",
]

# shellspawn can inherit the environment of an already-running Darling server.
# Clear GPU selectors before restoring this session's host selections so an
# Automatic launch cannot retain the previous game's offload/ICD overrides.
LAUNCH_SCRIPT = (
    "unset "
    + " ".join(GPU_ENVIRONMENT)
    + "\n"
    + r"""
project=$1 shim_dir=$2 launch_uri=$3; shift 3
unset MACOBLOX_METAL MACOBLOX_METAL_FRAMEWORKS MACOBLOX_METAL_HOST_LIBRARY MACOBLOX_METAL_SHADER_CACHE DYLD_FRAMEWORK_PATH
for kv in "$@"; do export "$kv"; done
# Roblox's own frame rate limit (FramerateCap, its Maximum Frame Rate
# setting), from the launcher's FPS limit. Set here, inside Darling, right
# before the game starts: Roblox rewrites the file when it quits, and a file
# in the prefix replaced from the host is not always seen by a running
# Darling. Bash only (3.2), which splits ${var/a/b} at a "/" even inside
# quotes, so the patterns come from variables.
case ${QUADRA_FRAMERATE_CAP:-} in
  '' | *[!0-9]*) ;;
  *)
    cap_file="$HOME/Library/Roblox/GlobalBasicSettings_13.xml"
    if [ -f "$cap_file" ]; then
      content=$(<"$cap_file")
      wanted="<int name=\"FramerateCap\">$QUADRA_FRAMERATE_CAP</int>"
      pattern='<int name="FramerateCap">-?[0-9]+</int>'
      if [[ $content =~ $pattern ]]; then
        old=${BASH_REMATCH[0]}
        content=${content/$old/$wanted}
      else
        close='</Properties>'
        insert=$'\t'"$wanted"$'\n\t\t</Properties>'
        content=${content/$close/$insert}
      fi
      printf '%s\n' "$content" > "$cap_file"
    fi ;;
esac
app="$project/RobloxPlayer.app/Contents/MacOS"
cd "$app" || exit 1
# Nothing may run between these exports and exec: every program started
# after them would get the shim injected too.
export DYLD_FORCE_FLAT_NAMESPACE=1
export DYLD_INSERT_LIBRARIES="$shim_dir/libQuadraShims.dylib"
export DYLD_LIBRARY_PATH="$shim_dir:$app"
if [ -n "${MACOBLOX_METAL_FRAMEWORKS:-}" ]; then
  export DYLD_FRAMEWORK_PATH="$MACOBLOX_METAL_FRAMEWORKS"
fi
if [ -n "$launch_uri" ]; then
  # Roblox's native macOS browser handoff is the -protocolString argument.
  # Keep the value as one opaque argument; the client owns the protocol
  # format. The shim logs this argument and suppresses Cocoa reinjection when
  # it is present, so one launch has one URL delivery path.
  export QUADRA_PENDING_URI="$launch_uri"
  exec ./RobloxPlayer -protocolString "$launch_uri"
fi
exec ./RobloxPlayer
"""
)


# Time Roblox gets to finish its own quit (see RobloxSession.finish).
QUIT_GRACE_SECONDS = 6


class RobloxSession:
    """One run of the client. poll() returns None while it is running."""

    def __init__(self, settings, launch_uri=None):
        self.settings = settings
        self.shader_preload_progress = None
        self.preload_stage_progress = None
        self.shader_preload_cancel = threading.Event()
        self._metal_preload_skipped = False
        self.metal_diagnostics_path = locations.LOGS / (
            time.strftime("metal-performance-%Y%m%d-%H%M%S-")
            + str(os.getpid())
            + ".jsonl"
        )
        # This is intentionally opaque.  Do not split, decode, normalize or
        # otherwise inspect a browser handoff before giving it to Roblox.
        self.launch_uri = launch_uri
        self.log_path = None
        self.process = None
        self._lifecycle_logged = False
        self._lifecycle_lock = threading.Lock()
        self.seen_roblox = False
        self.gone_since = None
        self.game_pids = []
        self.scanned_at = 0.0
        self.dns = None
        self.audio = None
        # The launcher's browser window for Roblox's embedded pages (web.py):
        # its socket as the guest sees it, and WebKit's user agent.
        self.web_socket = None
        self.web_user_agent = None
        self.gpu_adapter = None
        self._gpu_environment = None
        self.trace_path = None
        self._frontend_detached = False
        self.transport_monitor = TransportMonitor()
        # Launch log lines collected before the log exists.
        self.notes = []
        # A sentinel left over from a quit that outlived the launcher must not
        # end this session before it starts.
        locations.QUIT_SENTINEL.unlink(missing_ok=True)

    def _graphics_environment(self):
        from .. import display, gpus, graphics

        renderer = self.settings.get("renderer", "opengl")
        if self._gpu_environment is None:
            self.gpu_adapter = gpus.selected_gpu(self.settings.get("gpu", "auto"))
            self._gpu_environment = gpus.gpu_environment(self.gpu_adapter, renderer)
        metal = (
            graphics.metal_environment(
                locations.METAL_DIR,
                locations.CACHE_DIR / "metal-shader-cache",
                preload=self.settings.get("metal_shader_preload", True),
                preload_key=roblox.installed_version() or "unknown-client",
                diagnostics=self.settings.get("metal_diagnostics", False),
                diagnostics_file=self.metal_diagnostics_path,
                tuning=graphics.metal_tuning_environment(self.settings),
            )
            if renderer == "metal"
            else {}
        )
        if metal and self._metal_preload_skipped:
            metal["MACOBLOX_METAL_PRELOAD_IN_PROCESS"] = "0"
        return {
            **graphics.renderer_environment(renderer),
            **metal,
            **self._gpu_environment,
            **graphics.mangohud_environment(
                renderer, self.settings.get("mangohud", False)
            ),
            **display.window_environment(
                self.settings, locations.SHIM_DIR / "libquadra-wayland.so"
            ),
            **gpus.host_graphics_paths(),
        }

    def environment(self):
        env = darling_server.darling_environment(
            self.settings, getattr(self, "notes", None)
        )
        # Rebuild the allowlisted selectors, including intentional removals
        # (a PCI choice must not compete with an inherited vendor-ID choice).
        for name in GPU_ENVIRONMENT:
            env.pop(name, None)
        env.update(self._graphics_environment())
        if env["QUADRA_WAYLAND"] == "1":
            env.pop("DISPLAY", None)
        return env

    def shim_variables(self):
        variables = guest_variables(locations.DARLING_RUNTIME["metadata"]) + [
            f"QUADRA_SESSION_DIRECTORY=Library/{locations.SESSION_DIR.name}",
            f"QUADRA_MOUSE_SENSITIVITY={self.settings['mouse_sensitivity']:.2f}",
            f"QUADRA_SCROLL_SENSITIVITY={self.settings.get('scroll_sensitivity', 1.5):.2f}",
            # Mesa is the host's library and sees the host's file system,
            # not the prefix: with HOME=/Users/<name> it could not create
            # its shader cache ("Failed to create /Users for shader cache
            # -- disabling") and every launch recompiled every shader. So
            # the cache directory is given as the host path, without the
            # /Volumes/SystemRoot the guest's paths carry (with it Mesa
            # failed the same way on /Volumes). NVIDIA's driver ignores it.
            f"MESA_SHADER_CACHE_DIR={locations.CACHE_DIR / 'mesa-shader-cache'}",
            f"MESA_GLSL_CACHE_DIR={locations.CACHE_DIR / 'mesa-shader-cache'}",
            f"__GL_SHADER_DISK_CACHE_PATH={locations.CACHE_DIR / 'nvidia-shader-cache'}",
            # Roblox keeps its own caches (flag cache, thumbnail temp files)
            # under the prefix's /private/tmp, which does not survive a
            # Darling restart: every launch re-downloaded 23k flags and every
            # menu return re-fetched every thumbnail (429 rate limits, blank
            # place tiles). A host directory makes those caches persistent.
            f"TMPDIR=/Volumes/SystemRoot{locations.CACHE_DIR / 'roblox-tmp'}",
        ]
        from .. import display

        variables.append(
            f"QUADRA_DPI_SCALE={display.validated_dpi_scale(self.settings.get('dpi_scale', 1.0)):.3f}"
        )
        if self.settings.get("renderer", "opengl") == "metal":
            # The launch script makes it DYLD_FRAMEWORK_PATH just before exec:
            # Quadra's Metal.framework in place of Darling's.
            variables.append(
                f"MACOBLOX_METAL_FRAMEWORKS=/Volumes/SystemRoot{locations.METAL_DIR}"
            )
        # Host graphics libraries see the guest environment after exec.
        variables.extend(
            f"{name}={value}" for name, value in self._graphics_environment().items()
        )
        vram = host.host_vram_bytes(
            getattr(self, "renderer_name", None), self.gpu_adapter
        )
        if self.settings.get("renderer") == "metal":
            from .. import graphics

            vram = graphics.metal_vram_cap(self.settings, vram)
        if vram:
            variables.append(f"QUADRA_VRAM_BYTES={vram}")
        if self.settings.get("hide_menu_bar"):
            variables.append("QUADRA_HIDE_MENU_BAR=1")
        from .. import vr

        variables.extend(vr.shim_variables(self.settings)[0])
        if not self.settings.get("raw_mouse", True):
            variables.append("QUADRA_RAW_MOUSE=0")
        if self.settings.get("framerate_cap", 0) > 0:
            variables.append(
                f"QUADRA_FRAMERATE_CAP={int(self.settings['framerate_cap'])}"
            )
        if self.web_socket:
            variables.append(f"QUADRA_WEB_SOCKET={self.web_socket}")
            if self.web_user_agent:
                variables.append(f"QUADRA_WEB_USER_AGENT={self.web_user_agent}")
        if self.dns:
            variables.append(f"QUADRA_DNS={self.dns.address}")
        if self.audio:
            variables.append(f"QUADRA_AUDIO={os.environ.get('QUADRA_AUDIO', '1')}")
            variables.append(f"QUADRA_AUDIO_FIFO=/Volumes/SystemRoot{self.audio.fifo}")
            variables.append(
                f"QUADRA_AUDIO_INPUT_FIFO=/Volumes/SystemRoot{self.audio.input_fifo}"
            )
        else:
            # Darling's own audio path crashes the game (see HostAudio).
            variables.append("QUADRA_AUDIO=0")
        for key, name in config.TRACE_ENV.items():
            if self.settings.get(key) or (
                key in config.TRACE_ENV_INHERITED and os.environ.get(name) == "1"
            ):
                variables.append(f"{name}=1")
        poll = self.settings.get("kevent_poll_ms", 0)
        if isinstance(poll, int) and not isinstance(poll, bool) and poll > 0:
            variables.append(f"QUADRA_KEVENT_POLL_MS={min(poll, 1000)}")
        variables.extend(self.ciderdarling_variables())
        # Selection diagnostics record lengths/ranges only and are enabled
        # explicitly for a reported typing issue, without changing settings.
        if (
            self.settings.get("trace_text_input", False)
            or os.environ.get("QUADRA_TRACE_TEXT_INPUT") == "1"
        ):
            variables.append("QUADRA_TRACE_TEXT_INPUT=1")
        # Cocoa URL delivery diagnostics are opt-in and stay outside URI
        # handling. They are copied verbatim into the Darling process so a
        # live run can compare selector order and launch timing.
        for name in ("QUADRA_URI_DELAY_MS", "QUADRA_URI_SELECTOR_ORDER"):
            value = os.environ.get(name)
            if value:
                variables.append(f"{name}={value}")
        # The shim normally receives the exact launch value above.  Keep the
        # host-side pending file available as a fallback for launches that did
        # not carry the value as an argument (for example a retry after the
        # launcher was already open).
        variables.append(
            f"QUADRA_PENDING_URI_FILE=/Volumes/SystemRoot{locations.CACHE_DIR / 'pending-uri'}"
        )
        variables.append(
            f"QUADRA_QUIT_SENTINEL=/Volumes/SystemRoot{locations.QUIT_SENTINEL}"
        )
        try:
            variables.append(
                f"QUADRA_ICON_ARGB=/Volumes/SystemRoot{host.icon_argb_file()}"
            )
        except Exception:
            pass
        return variables

    def ciderdarling_variables(self):
        """Guest settings from Settings -> CiderDarling, for that runtime only.
        Watchdog kicks are a shim feature: QUADRA_KICK/QUADRA_NO_KICK from
        the launcher's environment also reach the game with regular Darling."""
        variables = [
            f"{name}=1"
            for name in ("QUADRA_KICK", "QUADRA_NO_KICK")
            if os.environ.get(name) == "1"
        ]
        if locations.DARLING_RUNTIME["runtime"] != "ciderdarling":
            return variables
        if self.settings.get("cider_thread_kicks") and "QUADRA_KICK=1" not in variables:
            variables.append("QUADRA_KICK=1")
        trace = getattr(self, "trace_path", None)
        if trace:
            # A guest path: CiderDarling opens it inside Darling.
            variables.append(f"CIDER_TRACE_FILE=/Volumes/SystemRoot{trace}")
        return variables

    def _create_trace_file(self, log_path):
        """CiderDarling writes traces only to a private regular file."""
        if not (
            self.settings.get("cider_trace")
            and locations.DARLING_RUNTIME["runtime"] == "ciderdarling"
        ):
            return None
        path = log_path.with_name(
            log_path.name.replace("launch-", "ciderdarling-trace-", 1).removesuffix(
                ".log"
            )
            + ".jsonl"
        )
        try:
            os.close(
                os.open(
                    path,
                    os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                    0o600,
                )
            )
            os.chmod(path, 0o600)
        except OSError as error:
            self.notes.append(f"CiderDarling trace unavailable: {error}")
            return None
        return path

    def start(self):
        """Start the client; on failure nothing started is left running."""
        try:
            self._start()
        except BaseException:
            self.finish("startup_failure")
            raise

    def _validated_metal(self, env):
        """The environment and renderer name once the Metal host has opened a device and run a
        test copy. Where it cannot and `metal_fallback` allows, this launch (and only this one: the
        saved renderer is untouched) uses OpenGL, and the reason is kept for the log and the UI."""
        from .. import graphics

        try:
            return env, "Quadra M2VK (" + graphics.validate_metal(
                env, locations.METAL_HOST_LIBRARY
            ) + ")"
        except RuntimeError as error:
            if not self.settings.get("metal_fallback", True):
                raise
            reason = str(error).strip()
        self.metal_fallback_reason = reason
        self.notes.append(
            "Quadra M2VK could not start, so this launch uses OpenGL: " + reason
        )
        self.settings = dict(self.settings, renderer="opengl")
        self._gpu_environment = None
        return self.environment(), "OpenGL (Quadra M2VK could not start)"

    def _start(self):
        missing = builds.missing_tools()
        if missing:
            raise RuntimeError(
                _("Install these first: {programs}", programs=", ".join(missing))
            )
        # if not shim_built():
        #     ok, output = build_shim()
        #     if not ok:
        #         raise RuntimeError(
        #             _("Could not build the shim:\n{output}", output=output)
        #         )
        if self.settings.get("renderer", "opengl") == "vulkan":
            from .. import graphics

            graphics.ensure_vulkan_dependencies()
        if self.settings.get("renderer", "opengl") == "metal":
            from .. import graphics

            missing = graphics.missing_metal_dependencies()
            if missing:
                raise RuntimeError(
                    _(
                        "Quadra M2VK needs: {items}. Install them or select another renderer.",
                        items=", ".join(missing),
                    )
                )
            if not builds.metal_built():
                self.notes.append(
                    "Building Quadra M2VK (darling/metal-vulkan/build.sh)"
                )
                ok, output = builds.build_metal()
                if not ok:
                    raise RuntimeError(
                        _("Could not build Quadra M2VK:\n{output}", output=output)
                    )
            try:
                (locations.CACHE_DIR / "metal-shader-cache").mkdir(
                    parents=True, exist_ok=True
                )
            except OSError as error:
                self.notes.append(
                    f"Metal shader cache unavailable; using bounded memory caching: {error}"
                )
        env = self.environment()
        if env.get("QUADRA_WAYLAND") != "1":
            x11.ensure_x11(env)
        renderer_name = "OpenGL"
        if self.settings.get("renderer", "opengl") == "vulkan":
            from .. import graphics

            renderer_name = graphics.validate_vulkan(env)
        elif self.settings.get("renderer", "opengl") == "metal":
            env, renderer_name = self._validated_metal(env)
        self.renderer_name = renderer_name
        # A game closed a moment ago may still be shutting down. A new one next
        # to it shared its darlingserver, and when that went both died. Give it
        # time, then end it; crash handlers of earlier games are just ended.
        deadline = time.monotonic() + 20
        while (
            process_control.roblox_pids(("RobloxPlayer",))
            and time.monotonic() < deadline
        ):
            time.sleep(0.25)
        leftover = process_control.roblox_pids()
        if leftover:
            process_control._terminate_roblox(leftover, wait=3)
        orphans = process_control.clear_orphaned_darling()
        if process_control._foreign_darlingservers():
            darling_server.restart_darling()
            self.notes.append(
                "Restarted Darling: its server came from another runtime installation"
            )
        if self.settings.get("renderer", "opengl") == "metal":
            from ..darling import metal_preload

            try:
                result = metal_preload.run(
                    locations.METAL_HOST_LIBRARY,
                    env,
                    progress=self.shader_preload_progress,
                    cancel=self.shader_preload_cancel,
                    library_pack=(
                        locations.APP_BUNDLE
                        / "Contents/Resources/shaders/shaders_metal_osx.pack"
                        if self.settings.get("metal_library_preload", True)
                        else None
                    ),
                    resource_root=(
                        locations.APP_BUNDLE / "Contents/Resources"
                        if self.settings.get("metal_resource_preload", True)
                        else None
                    ),
                    stage_progress=self.preload_stage_progress,
                )
                self.notes.append(
                    "Shader preloading: {status}, {completed}/{total} pipelines, {failed} skipped".format(
                        **result
                    )
                )
                for phase, counts in result.get("stages", {}).items():
                    self.notes.append(
                        f"Preload {phase}: " + json.dumps(counts, sort_keys=True)
                    )
                if result["status"] in (
                    "skipped",
                    "timeout",
                    "failed",
                    "disabled",
                    "memory-limited",
                ):
                    self._metal_preload_skipped = True
                    env["MACOBLOX_METAL_PRELOAD_IN_PROCESS"] = "0"
            except (OSError, subprocess.SubprocessError) as error:
                self._metal_preload_skipped = True
                env["MACOBLOX_METAL_PRELOAD_IN_PROCESS"] = "0"
                self.notes.append(
                    f"Shader preloading unavailable; starting Roblox: {error}"
                )
        self.notes.extend(prefix_files.prepare_prefix(env) or [])
        # The shim reads the pending file as a fallback: never an old click.
        from .. import uri

        uri.expire_pending()
        try:
            from .. import mods

            mods.apply_mods(self.settings)
            if self.settings.get("enable_custom_mods", True):
                from .. import modguard

                modguard.begin_session(locations.CONFIG_DIR, mods.MODS_DIR)
        except Exception as e:
            logging.getLogger("quadra").warning("Failed to apply mods: %s", e)
        roblox.apply_throttle_patch()
        transport_status = roblox.ensure_raknet_transport()
        shader_status = roblox.ensure_shader_compatibility()
        provider = self.settings.get("dns", "system")
        if provider != "system" and (
            provider != "custom" or self.settings.get("dns_custom")
        ):
            from ..dns import TELEMETRY_DOMAINS, DnsForwarder

            blocked = (
                TELEMETRY_DOMAINS if self.settings.get("block_telemetry", False) else ()
            )
            self.dns = DnsForwarder(
                provider, self.settings.get("dns_custom", ""), blocked
            )
        self.audio = audio.HostAudio.start()
        darling_server.clear_stale_darling()
        if darling_server.rotate_dserver_log():
            self.notes.append("Rotated dserver.log (over 32 MB) to dserver.log.old")
        locations.LOGS.mkdir(parents=True, exist_ok=True)
        # The Mesa shader cache dir must exist before the game opens it.
        (locations.CACHE_DIR / "mesa-shader-cache").mkdir(parents=True, exist_ok=True)
        (locations.CACHE_DIR / "nvidia-shader-cache").mkdir(parents=True, exist_ok=True)
        # The guest TMPDIR (Roblox's flag and thumbnail caches) likewise.
        (locations.CACHE_DIR / "roblox-tmp").mkdir(parents=True, exist_ok=True)
        self.log_path = locations.LOGS / time.strftime("launch-%Y%m%d-%H%M%S.log")
        logs.cleanup_logs(
            int(self.settings.get("keep_logs", 30)) - 1, current=self.log_path
        )
        self.trace_path = self._create_trace_file(self.log_path)
        if self.audio:
            self.audio.log_path = self.log_path
        # Truncate for this launch, but keep O_APPEND on the inherited file
        # description: guest output must not overwrite a host lifecycle record.
        with open(
            self.log_path,
            "wb",
            opener=lambda path, flags: os.open(path, flags | os.O_APPEND),
        ) as log:
            log.write(f"Quadra {__version__}\n".encode())
            log.write(f"Darling: {darling_server.darling_version(env)}\n".encode())
            log.write(
                f"Darling selection: {locations.DARLING_RUNTIME['runtime']}; {locations.DARLING_EXECUTABLE}; root {locations.DARLING_SYSROOT}\n".encode()
            )
            log.write(f"Renderer requested: {renderer_name}\n".encode())
            log.write(
                f"MangoHud requested: {'Enabled' if self.settings.get('mangohud', False) else 'Disabled'}\n".encode()
            )
            from .. import vr

            log.write(f"{vr.shim_variables(self.settings)[1]}\n".encode())
            log.write(
                f"Window backend: {'Wayland' if env.get('QUADRA_WAYLAND') == '1' else 'X11 / Xwayland'}\n".encode()
            )
            if self.gpu_adapter:
                name = (
                    self.gpu_adapter["name"].replace("\n", " ").replace("\r", " ")[:200]
                )
                log.write(
                    f"Graphics card requested: {name}; PCI {self.gpu_adapter['pci']}\n".encode()
                )
            else:
                log.write(
                    b"Graphics card requested: Automatic (desktop/terminal selection)\n"
                )
            packaging = (
                "Flatpak"
                if os.environ.get("FLATPAK_ID")
                else ("prebuilt" if locations.PREBUILT_SHIM else "source")
            )
            version = re.sub(
                r"[^A-Za-z0-9._+-]", "?", (roblox.installed_version() or "unknown")[:80]
            )
            log.write(f"Packaging: {packaging}; Roblox version: {version}\n".encode())
            log.write(f"RakNet compatibility: {transport_status}\n".encode())
            log.write(f"GLSL compatibility: {shader_status}\n".encode())
            channel = self.settings.get("roblox_channel", "")
            channel = (
                re.sub(r"[^A-Za-z0-9_-]", "?", channel[:64])
                if isinstance(channel, str)
                else "invalid"
            )
            log.write(
                f"Last client channel: {channel or 'Release (default)'}\n".encode()
            )
            if self.audio:
                command = self.audio._player_command(self.audio.fifo)
                log.write(
                    f"Audio playback helper: {command[0] if command else 'unavailable'}\n".encode()
                )
            else:
                log.write(
                    b"Audio unavailable: install pacat (pulseaudio-utils on Mint/Ubuntu), or pw-cat with --raw support.\n"
                )
            if leftover:
                log.write(
                    f"Requested cleanup of {len(leftover)} selected-prefix Roblox process(es) of an earlier game\n".encode()
                )
            if orphans:
                log.write(
                    f"Requested cleanup of {orphans} selected-prefix Darling process(es) left without their darlingserver\n".encode()
                )
            if self.trace_path:
                log.write(f"CiderDarling trace: {self.trace_path}\n".encode())
            if self.settings.get("renderer") == "metal" and self.settings.get(
                "metal_diagnostics", False
            ):
                log.write(
                    f"Metal performance trace: {self.metal_diagnostics_path}\n".encode()
                )
            log.writelines(f"[Quadra Startup] {note}\n".encode(errors="replace") for note in self.notes)
            darling_server._prepare_darling_server(env, log)
            changed = process_control.raise_darling_priority()
            if changed:
                log.write(
                    f"Priority: {changed} Darling processes raised to nice {process_control._nice_target(process_control.SERVER_NICE)}\n".encode()
                )
            log.flush()
            command = [
                locations.DARLING_EXECUTABLE,
                "shell",
                "/bin/bash",
                "-c",
                LAUNCH_SCRIPT,
                "quadra",
                f"/Volumes/SystemRoot{locations.DATA_DIR}",
                f"/Volumes/SystemRoot{locations.SHIM.parent}",
                self.launch_uri if self.launch_uri is not None else "",
                *self.shim_variables(),
            ]
            self.process = subprocess.Popen(
                command,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
        self.started_at = time.monotonic()
        if not self.settings.get("allow_debugger", False):
            threading.Thread(target=self._suppress_crash_handler, daemon=True).start()

    def _suppress_crash_handler(self):
        """Terminate RobloxCrashHandler after initial handshake to prevent exit hangs and slow crash logs."""
        if self.settings.get("allow_debugger", False):
            return
        for _ in range(20):
            time.sleep(1)
            if not self.process or self.process.poll() is not None:
                return
            crash_pids = process_control.roblox_pids(("RobloxCrashHandler",))
            if crash_pids:
                time.sleep(2)
                process_control._kill_crash_handlers()
                break

    def poll(self):
        """None while running, otherwise the exit status (or -1 if unknown)."""
        self.transport_monitor.poll(self.log_path)
        status = self.process.poll() if self.process else -1
        if status is not None:
            # darling shell can disconnect while the guest it launched is
            # alive. Treating that status as Roblox's exit killed the game
            # during startup (issue #27: status=1 with a live prefix player).
            players = process_control.roblox_pids(("RobloxPlayer",))
            players = [
                pid
                for pid in players
                if process_control._process_state(pid) not in (None, "Z")
            ]
            if not players and not self._frontend_detached:
                self.finish("frontend_exit", frontend_status=status)
                return status
            if players:
                self.game_pids = players
                self.seen_roblox = True
                if not self._frontend_detached:
                    self._frontend_detached = True
                    if self.log_path:
                        try:
                            with open(self.log_path, "ab", buffering=0) as log:
                                log.write(
                                    f"\n[Quadra Lifecycle] frontend_detached status={status} live_players={len(players)}; monitoring guest\n".encode()
                                )
                        except OSError:
                            pass
        if self.audio:
            self.audio.keep_playing()
        # The shim touches the sentinel when Roblox starts terminating
        # (Cmd+Q, Quit in the menu). Roblox's own teardown takes seconds, and
        # the session can end for the user as soon as quitting began.
        # seen_roblox keeps a sentinel left from a dead session irrelevant.
        if self.seen_roblox and locations.QUIT_SENTINEL.exists():
            self.finish("quit_sentinel")
            return -1
        # Suppress any crash handler to prevent slow dumps and exit blockage
        if (
            not self.settings.get("allow_debugger", False)
            and self.seen_roblox
            and time.monotonic() - self.started_at > 3
        ):
            process_control._kill_crash_handlers()

        # darling shell can outlive a Roblox that was killed; watch the game
        # processes themselves as well. Known ones are checked each second,
        # the whole of /proc only when they are gone or every few seconds.
        now = time.monotonic()
        namespaces = process_control._prefix_namespaces()
        self.game_pids = [
            pid
            for pid in self.game_pids
            if process_control._process_state(pid) not in (None, "Z")
            and process_control._roblox_process_in_prefix(
                pid, namespaces, ("RobloxPlayer",)
            )
        ]
        if not self.game_pids or now - self.scanned_at > 1:
            self.game_pids = process_control.roblox_pids(("RobloxPlayer",))
            self.scanned_at = now
        if self.game_pids:
            self.seen_roblox = True
            self.gone_since = None
        elif self.seen_roblox:
            self.gone_since = self.gone_since or time.monotonic()
            if time.monotonic() - self.gone_since > 0.5:
                self.finish("roblox_disappeared")
                return -1
        return None

    def _record_lifecycle(self, reason, frontend_status=None):
        """Record the observed state once, before cleanup can alter it."""
        with self._lifecycle_lock:
            if self._lifecycle_logged or not self.log_path:
                return
            self._lifecycle_logged = True
            try:
                frontend = "not_started"
                if self.process:
                    status = (
                        frontend_status
                        if frontend_status is not None
                        else self.process.poll()
                    )
                    frontend = "running" if status is None else str(status)
                try:
                    game_count = str(
                        len(process_control.roblox_pids(("RobloxPlayer",)))
                    )
                except Exception:
                    game_count = "unknown"
                # Reasons and field values are local observations only; no
                # command line, URI, typed text or authenticated session data.
                if reason not in (
                    "frontend_exit",
                    "roblox_disappeared",
                    "quit_sentinel",
                    "startup_failure",
                    "explicit_cleanup",
                ):
                    reason = "explicit_cleanup"
                stamp = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
                record = (
                    f"\n[Quadra Lifecycle] time={stamp} reason={reason} "
                    f"frontend_status={frontend} selected_prefix_games={game_count} "
                    "guest_exit=unknown before_cleanup=1\n"
                )
                with open(self.log_path, "ab", buffering=0) as log:
                    log.write(record.encode())
            except Exception:
                # Failure to collect or append diagnostics cannot stop cleanup.
                pass

    def finish(self, reason="explicit_cleanup", frontend_status=None):
        leftover = process_control.roblox_pids()
        self._record_lifecycle(reason, frontend_status)
        if leftover and reason == "quit_sentinel":
            # Roblox is quitting by itself and saves GlobalBasicSettings while
            # it does; a signal now would lose them. Audio and DNS stay up too.
            deadline = time.monotonic() + QUIT_GRACE_SECONDS
            while (
                process_control.roblox_pids(("RobloxPlayer",))
                and time.monotonic() < deadline
            ):
                time.sleep(0.2)
            leftover = process_control.roblox_pids()
        if leftover:
            process_control._terminate_roblox(leftover, wait=1)
        if self.process and self.process.poll() is None:
            process_control._terminate_frontend(self.process)
        if self.dns:
            self.dns.stop()
            self.dns = None
        if self.audio:
            self.audio.stop()
            self.audio = None
