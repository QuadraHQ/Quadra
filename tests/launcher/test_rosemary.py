"""Tests for the Rosemary backend (client download/unpack, runtime, session).

Run: PYTHONPATH=launcher python3 -m unittest discover -s tests/launcher -v
A fake `rosemary` script stands in for the real program."""
import os
import stat
import tempfile
import threading
import time
import unittest
import zipfile
from pathlib import Path
from unittest import mock

from quadra import rosemary_client as client
from quadra import rosemary_install as installer
from quadra import rosemary_runtime as runtime

PLIST = b"""<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>RobloxPlayer</string>
<key>CFBundleShortVersionString</key><string>0.745.0.1</string></dict></plist>"""
MACHO_X86 = (0xFEEDFACF).to_bytes(4, "little") + (0x01000007).to_bytes(4, "little") + b"\0" * 32


def make_zip(path, extra=()):
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("RobloxPlayer.app/Contents/Info.plist", PLIST)
        info = zipfile.ZipInfo("RobloxPlayer.app/Contents/MacOS/RobloxPlayer")
        info.external_attr = (stat.S_IFREG | 0o755) << 16
        info.create_system = 3
        archive.writestr(info, MACHO_X86)
        for name, data in extra:
            archive.writestr(name, data)


class ClientTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        patcher = mock.patch.object(client, "data_dir", lambda: self.root / "rosemary")
        patcher.start()
        self.addCleanup(patcher.stop)
        self.addCleanup(self.tmp.cleanup)

    def build(self):
        return client.Build("version-abc123", "0.745.0.1", "x86_64")

    def test_install_and_verify(self):
        archive = self.root / "a.zip"
        make_zip(archive)
        build = self.build()
        program = client.install(archive, build, lambda *a: None, threading.Event())
        self.assertTrue(program.is_file() and os.access(program, os.X_OK))
        self.assertEqual(client.macho_architectures(program), {"x86_64"})
        self.assertEqual([b.key for b, _ in client.installed_builds()], [build.key])

    def test_wrong_architecture_is_refused_and_nothing_is_left(self):
        archive = self.root / "a.zip"
        make_zip(archive)
        build = client.Build("version-abc123", "", "arm64")
        with self.assertRaises(client.ClientError):
            client.install(archive, build, lambda *a: None, threading.Event())
        self.assertEqual(list(client.versions_dir().iterdir()), [])  # staging removed

    def test_zip_slip_and_bad_links_are_refused(self):
        for name, data in (("RobloxPlayer.app/../evil", b"x"), ("/abs", b"x"), ("Other.app/x", b"x")):
            archive = self.root / "bad.zip"
            make_zip(archive, [(name, data)])
            with self.assertRaises(client.ClientError, msg=name):
                client.install(archive, self.build(), lambda *a: None, threading.Event())
        link = zipfile.ZipInfo("RobloxPlayer.app/link")
        link.external_attr = (stat.S_IFLNK | 0o777) << 16
        link.create_system = 3
        archive = self.root / "link.zip"
        make_zip(archive)
        with zipfile.ZipFile(archive, "a") as handle:
            handle.writestr(link, "../../etc/passwd")
        with self.assertRaises(client.ClientError):
            client.install(archive, self.build(), lambda *a: None, threading.Event())

    def test_macho_detection(self):
        for data, expected in ((MACHO_X86, {"x86_64"}), (b"MZ" + b"\0" * 20, set()),
                               ((0xCAFEBABE).to_bytes(4, "big") + (52).to_bytes(4, "big") + b"\0" * 20, set())):
            path = self.root / "f"
            path.write_bytes(data)
            self.assertEqual(client.macho_architectures(path), expected)

    def test_fetch_latest_parses_and_validates(self):
        class R:
            def __init__(self, body): self.body = body
            def __enter__(self): return self
            def __exit__(self, *a): return False
            def read(self, n): return self.body
        good = b'{"version": "0.745.0.1", "clientVersionUpload": "version-0123abcd"}'
        build = client.fetch_latest("x86_64", opener=lambda *a, **k: R(good))
        self.assertEqual((build.guid, build.version), ("version-0123abcd", "0.745.0.1"))
        with self.assertRaises(client.ClientError):
            client.fetch_latest("x86_64", opener=lambda *a, **k: R(b'{"clientVersionUpload": "../x"}'))

    def test_write_fast_flags(self):
        bundle = self.root / "Roblox.app"
        client.write_fast_flags(bundle, {"A": 1})
        self.assertIn('"A": 1', (bundle / "Contents/MacOS/ClientSettings/ClientAppSettings.json").read_text())


FAKE = """#!/bin/sh
echo "$@" >> "$FAKE_LOG"
echo "PREFIX=$ROSEMARY_PREFIX SERVER=$ROSEMARY_SERVER DRIVER=$SDL_VIDEO_DRIVER VK=$VK_DRIVER_FILES PWD=$PWD" >> "$FAKE_LOG"
case "$1" in
  prefix) mkdir -p "$3/.rosemary"; echo "base:" > "$3/.rosemary/state.yaml"; exit 0 ;;
  server) exit 0 ;;
  run) echo "game output"; sleep "${FAKE_RUN_SECONDS:-0.3}"; exit "${FAKE_EXIT:-0}" ;;
esac
"""


class RuntimeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.fake = self.root / "fake-rosemary"
        self.fake.write_text(FAKE)
        self.fake.chmod(0o755)
        os.environ["FAKE_LOG"] = str(self.root / "calls.log")
        for patch in (mock.patch.object(client, "data_dir", lambda: self.root / "rosemary"),
                      mock.patch.dict(os.environ, {"XDG_DATA_HOME": str(self.root / "xdg")})):
            patch.start()
            self.addCleanup(patch.stop)
        self.settings = {"rosemary_program": str(self.fake), "rosemary_prefix": str(self.root / "prefix")}

    def install_fake_client(self):
        archive = self.root / "a.zip"
        make_zip(archive)
        client.install(archive, client.Build("version-abc123", "0.745.0.1", "x86_64"),
                       lambda *a: None, threading.Event())

    def test_find_rosemary_order(self):
        self.assertEqual(runtime.find_rosemary(self.settings), self.fake)
        with self.assertRaises(runtime.RosemaryError):
            runtime.find_rosemary({"rosemary_program": str(self.root / "missing")})
        runner = self.root / "xdg" / "butter" / "runners" / "r1"
        (runner / "bin").mkdir(parents=True)
        (runner / "bin" / "rosemary").write_text("#!/bin/sh\n")
        (runner / "bin" / "rosemary").chmod(0o755)
        (runner / "runner.toml").write_text('name = "Rosemary 1"\nexecutable = "bin/rosemary"\ntag = "v0.5.0"\n')
        self.assertEqual(runtime.runners()[0][1], runner / "bin" / "rosemary")

    def test_env_clears_inherited_managed_variables(self):
        base = {"ROSEMARY_SERVER": "off", "SDL_VIDEODRIVER": "x11", "PATH": "/bin"}
        env = runtime.run_env({"rosemary_prefix": "/p", "rosemary_force_opengl": True,
                               "rosemary_video_driver": "wayland", "rosemary_swap_interval": "0"}, base)
        self.assertEqual(env["ROSEMARY_PREFIX"], "/p")
        self.assertNotIn("ROSEMARY_SERVER", env)
        self.assertNotIn("SDL_VIDEODRIVER", env)
        self.assertEqual((env["SDL_VIDEO_DRIVER"], env["VK_DRIVER_FILES"], env["ROSEMARY_SWAP_INTERVAL"]),
                         ("wayland", "/dev/null", "0"))
        off = runtime.run_env({"rosemary_process_server": False}, {})
        self.assertEqual(off["ROSEMARY_SERVER"], "off")
        self.assertNotIn("VK_DRIVER_FILES", off)

    def test_mangohud_is_an_environment_variable_not_a_wrapper(self):
        with mock.patch("shutil.which", lambda name: "/usr/bin/mangohud"):
            argv = runtime.run_argv(Path("/r"), {"mangohud": True}, Path("/a"), None)
        self.assertEqual(argv, ["/r", "run", "/a"])
        self.assertEqual(runtime.run_env({"mangohud": True}, {})["MANGOHUD"], "1")
        self.assertNotIn("MANGOHUD", runtime.run_env({}, {}))

    def test_argv_keeps_the_link_opaque(self):
        link = "roblox-player:1+launchmode:play+gameinfo:abc%2Bdef"
        argv = runtime.run_argv(Path("/r"), {"rosemary_engine": "native"}, Path("/app/Roblox.app"), link)
        self.assertEqual(argv, ["/r", "run", "--native", "/app/Roblox.app", "-protocolString", link])
        self.assertEqual(runtime.run_argv(Path("/r"), {}, Path("/a"), None), ["/r", "run", "/a"])

    def test_session_lifecycle(self):
        self.install_fake_client()
        session = runtime.RosemarySession(self.settings, "roblox-player:x", logs_dir=self.root / "logs")
        session.start()
        self.assertIsNone(session.poll())
        end = time.time() + 5
        status = None
        while time.time() < end and status is None:
            time.sleep(0.1)
            status = session.poll()
        self.assertEqual(status, 0)
        calls = (self.root / "calls.log").read_text()
        self.assertIn("prefix create", calls)
        self.assertIn("run", calls)
        self.assertIn("-protocolString roblox-player:x", calls)
        self.assertIn("server --stop", calls)                     # finish() stops the process server
        self.assertIn(f"PWD={self.root / 'prefix' / 'root'}", calls)
        self.assertIn("game output", session.log_path.read_text())
        self.assertIn("runtime=rosemary", session.log_path.read_text())

    def test_exit_status_and_stop(self):
        self.install_fake_client()
        with mock.patch.dict(os.environ, {"FAKE_EXIT": "3"}):
            session = runtime.RosemarySession(self.settings, logs_dir=self.root / "logs")
            session.start()
            status = None
            end = time.time() + 5
            while time.time() < end and status is None:
                time.sleep(0.1)
                status = session.poll()
            self.assertEqual(status, 3)
        with mock.patch.dict(os.environ, {"FAKE_RUN_SECONDS": "30"}):
            session = runtime.RosemarySession(self.settings, logs_dir=self.root / "logs")
            session.start()
            started = time.time()
            session.stop_game()
            self.assertLess(time.time() - started, 8)
            self.assertIsNotNone(session.process.poll())
            session.finish()

    def test_start_without_a_client_fails_clearly(self):
        with self.assertRaises(runtime.RosemaryError):
            runtime.RosemarySession(self.settings, logs_dir=self.root / "logs").start()

    def test_player_log_discovery(self):
        prefix = self.root / "prefix"
        logs = prefix / "root" / "Users" / "me" / "Library" / "Logs" / "Roblox"
        logs.mkdir(parents=True)
        old = logs / "old.log"
        old.write_text("x")
        os.utime(old, (1, 1))
        new = logs / "new_Player.log"
        new.write_text("y")
        self.assertEqual(runtime.newest_player_log(prefix, since=time.time() - 60), new)
        self.assertIsNone(runtime.newest_player_log(prefix, since=time.time() + 60))


class InstallerTests(unittest.TestCase):
    RELEASE = {"assets": [
        {"name": "SHA256SUMS", "browser_download_url": "https://github.com/microlift/rosemary/releases/download/v1/SHA256SUMS"},
        {"name": "Rosemary-1.2.3-x86_64.AppImage",
         "browser_download_url": "https://github.com/microlift/rosemary/releases/download/v1/Rosemary-1.2.3-x86_64.AppImage"}]}

    def test_parse_release_and_hash(self):
        version, name, url, sums = installer.parse_release(self.RELEASE)
        self.assertEqual((version, name), ("1.2.3", "Rosemary-1.2.3-x86_64.AppImage"))
        text = f"{'a' * 64}  Rosemary-1.2.3-x86_64.AppImage\n{'b' * 64}  other\n"
        self.assertEqual(installer.expected_hash(text, name), "a" * 64)
        with self.assertRaises(installer.InstallError):
            installer.expected_hash(text, "missing")
        with self.assertRaises(installer.InstallError):
            installer.parse_release({"assets": []})

    def test_only_github_https_is_allowed(self):
        for bad in ("http://github.com/x", "https://evil.example/x", "https://github.com.evil.io/x", "file:///etc/passwd"):
            with self.assertRaises(installer.InstallError):
                installer._check_url(bad)
        installer._check_url("https://objects.githubusercontent.com/x")

    def test_hash_mismatch_installs_nothing(self):
        class R:
            def __init__(self, body): self.body = body; self.headers = {"Content-Length": str(len(body))}; self.done = False
            def __enter__(self): return self
            def __exit__(self, *a): return False
            def read(self, n=-1):
                if self.done: return b""
                self.done = True; return self.body
        import json as _json
        def opener(request, timeout=0):
            url = request.full_url
            if "api.github.com" in url: return R(_json.dumps(InstallerTests.RELEASE).encode())
            if url.endswith("SHA256SUMS"): return R(f"{'0' * 64}  Rosemary-1.2.3-x86_64.AppImage\n".encode())
            return R(b"not an appimage")
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(installer, "runner_root", lambda: Path(tmp) / "r"):
            with self.assertRaises(installer.InstallError):
                installer.install_latest(lambda *a: None, opener)
            self.assertEqual([p.name for p in (Path(tmp) / "r").iterdir()], [])  # staging removed


if __name__ == "__main__":
    unittest.main()
