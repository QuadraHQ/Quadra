"""Sound: playing the client's audio on the host through PipeWire or PulseAudio."""

import functools
import os
import re
import shutil
import stat
import subprocess
import tempfile
import threading
import time
from pathlib import Path

from . import locations, process_control

__all__ = [
    "_pwcat_raw_supported",
    "HostAudio",
    "_pid_alive",
    "sweep_audio_fifos",
]


@functools.lru_cache(maxsize=4)
def _pwcat_raw_supported(program):
    """PipeWire 1.0.x has pw-cat, but cannot consume our raw PCM FIFO."""
    if not program:
        return False
    try:
        help_text = subprocess.run(
            [program, "--help"],
            capture_output=True,
            text=True,
            env=dict(os.environ, LC_ALL="C"),
            timeout=2,
        )
        return help_text.returncode == 0 and "--raw" in help_text.stdout
    except (OSError, subprocess.SubprocessError):
        return False


class HostAudio:
    """Game sound played on the host. The shim writes raw float32 stereo
    44.1 kHz audio into a FIFO and pw-cat plays it through PipeWire, or pacat
    through PulseAudio where only that is reachable (the Flatpak). Darling's
    own audio (CoreAudio over PulseAudio on GCD) overflows Darling's
    workqueue thread stacks within seconds, so it is not used. The launcher
    keeps the FIFO open read/write for the whole session, so pw-cat never
    sees end of file and the game can reopen it any time."""

    NAME = "Roblox (Quadra)"

    def __init__(self, fifo, keep=None, player=None):
        self.fifo, self.keep, self.player = fifo, keep, player
        self.restarted_at = 0.0
        self._state_lock = threading.RLock()
        self._stopped = False
        self._owned_paths = set()
        # The microphone (voice chat): the shim reads raw float32 audio from a
        # second FIFO. A recorder runs only while the game asks for it, with
        # a request file next to the FIFO naming the rate and channel count.
        self.input_fifo = fifo.with_name(fifo.name.replace("audio-", "audio-in-"))
        self.request = Path(str(self.input_fifo) + ".request")
        self.recorder = None
        self.recording = None
        self.error_output = None
        self.log_path = None
        self.pulse_fallback = False

    def _keep_recording(self):
        wanted = None
        try:
            rate, channels = self.request.read_text().split()[:2]
            if 8000 <= int(rate) <= 192000 and 1 <= int(channels) <= 2:
                wanted = (int(rate), int(channels))
        except (OSError, ValueError):
            pass
        if self.recorder and (
            self.recorder.poll() is not None or wanted != self.recording
        ):
            self._stop_recorder()
        if wanted and not self.recorder:
            command = self._recorder_command(self.input_fifo, *wanted)
            if command:
                try:
                    self.recorder = subprocess.Popen(
                        command,
                        stdin=subprocess.DEVNULL,
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    )
                    self.recording = wanted
                except OSError:
                    pass

    def _stop_recorder(self):
        with self._state_lock:
            recorder, self.recorder = self.recorder, None
            self.recording = None
            self._stop_process(recorder)

    @staticmethod
    def _stop_process(process):
        """Stop and reap an owned audio subprocess without skipping cleanup."""
        if process is None:
            return
        try:
            process.terminate()
        except OSError:
            pass
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            try:
                process.kill()
            except OSError:
                pass
            try:
                process.wait(timeout=2)
            except (OSError, subprocess.TimeoutExpired):
                pass
        except OSError:
            pass

    @classmethod
    def _recorder_command(cls, fifo, rate, channels):
        runtime = Path(os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}"))
        pipewire = (
            os.environ.get("PIPEWIRE_REMOTE") or (runtime / "pipewire-0").exists()
        )
        if pipewire and _pwcat_raw_supported(shutil.which("pw-cat")):
            return [
                "pw-cat",
                "--record",
                "--raw",
                "--format",
                "f32",
                "--rate",
                str(rate),
                "--channels",
                str(channels),
                "--latency",
                "20ms",
                "--media-role",
                "Communication",
                "-P",
                '{ application.name = "Roblox" application.icon-name = "quadrahq" '
                f'media.name = "Roblox microphone (Quadra)" }}',
                str(fifo),
            ]
        if shutil.which("pacat"):
            return [
                "pacat",
                "--record",
                "--raw",
                "--format=float32le",
                f"--rate={rate}",
                f"--channels={channels}",
                "--latency-msec=20",
                "--client-name=Roblox",
                "--stream-name=Roblox microphone (Quadra)",
                "--property=media.role=phone",
                str(fifo),
            ]
        return None

    @classmethod
    def _spawn(cls, fifo, error_output=None, force_pulse=False):
        command = cls._player_command(fifo, force_pulse=force_pulse)
        if not command:
            raise OSError("No compatible audio playback helper is installed")
        # Audio before the game (see PLAYER_NICE); `nice` takes a relative value.
        change = process_control._nice_target(process_control.PLAYER_NICE) - os.getpriority(os.PRIO_PROCESS, 0)
        if change < 0 and shutil.which("nice"):
            command = ["nice", "-n", str(change), *command]
        return subprocess.Popen(
            command,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=error_output if error_output is not None else subprocess.DEVNULL,
        )

    def _report_player_exit(self, status):
        detail = ""
        if self.error_output:
            try:
                self.error_output.seek(0)
                detail = self.error_output.read(4096).decode(errors="replace").strip()
                self.error_output.seek(0)
                self.error_output.truncate()
            except (OSError, ValueError):
                detail = "playback error output unavailable"
        if self.log_path:
            try:
                with open(self.log_path, "ab") as log:
                    log.write(
                        f"[Quadra Audio] Playback helper exited with code {status}; {detail or 'no error output'}\n".encode()
                    )
            except OSError:
                pass

    def keep_playing(self):
        """Restart the player if it has exited (PipeWire restarted, say):
        the game keeps writing into the FIFO and would stay silent."""
        with self._state_lock:
            if self._stopped:
                return
            self._keep_recording()
            status = self.player.poll()
            if status is None or time.monotonic() - self.restarted_at < 5:
                return
            self.restarted_at = time.monotonic()
            self._report_player_exit(status)
            # PipeWire's PulseAudio server is also usable when native playback
            # initialization fails. Keep the fallback for this game session.
            if shutil.which("pacat"):
                self.pulse_fallback = True
            if self._player_command(self.fifo, force_pulse=self.pulse_fallback):
                try:
                    self.player = self._spawn(
                        self.fifo, self.error_output, self.pulse_fallback
                    )
                except OSError:
                    pass

    @classmethod
    def _player_command(cls, fifo, force_pulse=False):
        runtime = Path(os.environ.get("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}"))
        pipewire = (
            os.environ.get("PIPEWIRE_REMOTE") or (runtime / "pipewire-0").exists()
        )
        if (
            not force_pulse
            and pipewire
            and _pwcat_raw_supported(shutil.which("pw-cat"))
        ):
            return [
                "pw-cat",
                "--playback",
                "--raw",
                "--format",
                "f32",
                "--rate",
                "44100",
                "--channels",
                "2",
                "--latency",
                "40ms",
                "--media-role",
                "Game",
                "-P",
                '{ application.name = "Roblox" application.icon-name = "quadrahq" '
                f'media.name = "{cls.NAME}" }}',
                str(fifo),
            ]
        if shutil.which("pacat"):
            return [
                "pacat",
                "--playback",
                "--raw",
                "--format=float32le",
                "--rate=44100",
                "--channels=2",
                "--latency-msec=40",
                "--client-name=Roblox",
                f"--stream-name={cls.NAME}",
                "--property=media.role=game",
                str(fifo),
            ]
        return None

    @classmethod
    def start(cls):
        if not cls._player_command(""):
            return None
        locations.CACHE_DIR.mkdir(parents=True, exist_ok=True)
        fifo = locations.CACHE_DIR / f"audio-{os.getpid()}.fifo"
        audio = cls(fifo)
        try:
            audio.error_output = tempfile.TemporaryFile(dir=locations.CACHE_DIR)
            fifo.unlink(missing_ok=True)
            os.mkfifo(fifo, 0o600)
            audio._owned_paths.add(fifo)
            audio.keep = os.open(fifo, os.O_RDWR)
            for path in (audio.input_fifo, audio.request):
                try:
                    path.unlink()
                except OSError:
                    pass
            os.mkfifo(audio.input_fifo, 0o600)
            audio._owned_paths.add(audio.input_fifo)
            audio.player = cls._spawn(fifo, audio.error_output)
            return audio
        except BaseException:
            audio.stop()
            raise

    def stop(self):
        # pw-cat sits in a blocking read on the FIFO and only sees SIGTERM
        # once that returns: closing the last writer ends the read (end of
        # file). A player that still hangs is killed and reaped.
        with self._state_lock:
            if self._stopped:
                return
            self._stopped = True
            self._stop_recorder()
            if self.input_fifo in self._owned_paths:
                for path in (self.input_fifo, self.request):
                    try:
                        path.unlink()
                    except OSError:
                        pass
            keep, self.keep = self.keep, None
            if keep is not None:
                try:
                    os.close(keep)
                except OSError:
                    pass
            player, self.player = self.player, None
            self._stop_process(player)
            if self.error_output:
                self.error_output.close()
                self.error_output = None
            if self.fifo in self._owned_paths:
                try:
                    self.fifo.unlink()
                except OSError:
                    pass
            self._owned_paths.clear()


def _pid_alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except OSError:
        return True  # another user's process (EPERM) is alive
    return True


def sweep_audio_fifos(directory=None):
    """Remove audio FIFOs left by launchers that were killed (HostAudio.stop
    never ran): only this user's FIFOs named audio-<pid>.fifo or
    audio-in-<pid>.fifo whose process is gone. Returns how many went."""
    directory = locations.CACHE_DIR if directory is None else Path(directory)
    removed = 0
    try:
        entries = list(directory.iterdir())
    except OSError:
        return 0
    for path in entries:
        match = re.fullmatch(r"audio-(?:in-)?([1-9]\d{0,9})\.fifo", path.name)
        if not match:
            continue
        pid = int(match.group(1))
        try:
            info = path.lstat()
            if (
                not stat.S_ISFIFO(info.st_mode)
                or info.st_uid != os.getuid()
                or pid == os.getpid()
                or _pid_alive(pid)
            ):
                continue
            path.unlink()
            removed += 1
        except OSError:
            pass
    return removed
