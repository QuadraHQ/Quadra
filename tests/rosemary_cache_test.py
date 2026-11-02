import tempfile
import unittest
from pathlib import Path

from quadra import core
from quadra.rosemary import runtime


def write(path, size=0):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"x" * size)
    return path


class ClearRobloxCacheTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.prefix = Path(self.tmp.name) / "prefix"
        root = self.prefix / "root"
        self.library = root / "Users" / "someone" / "Library"
        self.cookie = write(self.library / "Cookies" / "com.roblox.RobloxPlayer.cookies.tsv", 40)
        self.keep = [
            self.cookie,
            write(self.library / "Roblox" / "frm.cfg", 5),
            write(self.library / "Preferences" / "a.plist", 5),
            write(self.library / "Logs" / "Roblox" / "last.log", 5),
            write(root / "private" / "tmp" / "Roblox" / "cache" / "flag_cache.dat", 5),
            write(self.prefix / ".rosemary" / "state.yaml", 5),
        ]
        self.cache = [
            write(root / "private" / "tmp" / "Roblox" / "rbx-storage" / "ab" / "abcd", 100),
            write(self.library / "Roblox" / "rbx-storage" / "cd" / "cdef", 30),
            write(self.library / "Roblox" / "rbx-storage-sc" / "x", 7),
            write(self.library / "Roblox" / "rbx-storage.id", 3),
            write(self.library / "Roblox" / "rbx-storage.db", 50),
            write(self.library / "Roblox" / "rbx-storage.db-shm", 10),
            write(self.library / "Roblox" / "rbx-storage.db-wal", 0),
        ]

    def test_the_cache_goes_and_the_sign_in_stays(self):
        freed = runtime.clear_roblox_cache(self.prefix)
        for path in self.cache:
            self.assertFalse(path.exists(), path)
        for path in self.keep:
            self.assertTrue(path.exists(), path)
        self.assertEqual(self.cookie.read_bytes(), b"x" * 40)
        self.assertEqual(runtime.cookie_files(self.prefix), [self.cookie])
        self.assertEqual(freed, 100 + 30 + 7 + 3 + 50 + 10)

    def test_clearing_twice_is_harmless(self):
        runtime.clear_roblox_cache(self.prefix)
        self.assertEqual(runtime.clear_roblox_cache(self.prefix), 0)
        self.assertTrue(self.cookie.exists())

    def test_a_prefix_with_nothing_to_clear_or_nothing_at_all(self):
        empty = Path(self.tmp.name) / "empty"
        self.assertEqual(runtime.clear_roblox_cache(empty), 0)
        (empty / "root" / "Users").mkdir(parents=True)
        self.assertEqual(runtime.clear_roblox_cache(empty), 0)

    def test_a_link_is_removed_but_never_followed(self):
        outside = Path(self.tmp.name) / "outside"
        precious = write(outside / "precious.txt", 9)
        link = self.library / "Roblox" / "rbx-storage"
        for path in sorted(link.rglob("*"), reverse=True):
            path.unlink() if path.is_file() else path.rmdir()
        link.rmdir()
        link.symlink_to(outside, target_is_directory=True)
        runtime.clear_roblox_cache(self.prefix)
        self.assertFalse(link.is_symlink())
        self.assertTrue(precious.exists())

    def test_more_than_one_user_is_cleared(self):
        other = self.prefix / "root" / "Users" / "other" / "Library" / "Roblox"
        cache = write(other / "rbx-storage.db", 4)
        runtime.clear_roblox_cache(self.prefix)
        self.assertFalse(cache.exists())

    def test_the_launch_option_is_off_by_default(self):
        self.assertIs(core.DEFAULT_SETTINGS["rosemary_clear_cache_on_launch"], False)


if __name__ == "__main__":
    unittest.main()
