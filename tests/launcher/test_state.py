import json
import unittest

from quadra import core


class SettingsStoreTests(unittest.TestCase):
    def setUp(self):
        self.saved = []
        self.store = core.SettingsStore(
            {"renderer": "opengl", "playtime_seconds": 0, "mangohud": False},
            save=self.saved.append,
        )

    def test_reads_like_a_mapping(self):
        self.assertEqual(self.store["renderer"], "opengl")
        self.assertEqual(self.store.get("missing", "fallback"), "fallback")
        self.assertIn("mangohud", self.store)
        self.assertEqual(dict(self.store), self.store.snapshot())
        self.assertEqual(json.loads(json.dumps(dict(self.store))), self.store.snapshot())

    def test_set_saves_and_notifies_watchers_of_that_key(self):
        renderer_changes, other_changes = [], []
        self.store.watch("renderer", lambda key, value: renderer_changes.append((key, value)))
        self.store.watch(("mangohud",), lambda key, value: other_changes.append(value))
        self.store["renderer"] = "metal"
        self.assertEqual(renderer_changes, [("renderer", "metal")])
        self.assertEqual(other_changes, [])
        self.assertEqual(self.saved[-1]["renderer"], "metal")

    def test_an_unchanged_value_neither_saves_nor_notifies(self):
        changes = []
        self.store.watch("renderer", lambda key, value: changes.append(value))
        self.store.set("renderer", "opengl")
        self.assertEqual(changes, [])
        self.assertEqual(self.saved, [])

    def test_a_value_of_another_type_counts_as_a_change(self):
        self.store.set("mangohud", 0)
        self.assertIs(type(self.store["mangohud"]), int)
        self.assertEqual(len(self.saved), 1)

    def test_stage_notifies_without_saving_until_save(self):
        changes = []
        self.store.watch("playtime_seconds", lambda key, value: changes.append(value))
        self.store.stage("playtime_seconds", 60)
        self.assertEqual(changes, [60])
        self.assertEqual(self.saved, [])
        self.store.save()
        self.assertEqual(self.saved[-1]["playtime_seconds"], 60)

    def test_unsubscribing_stops_notifications(self):
        changes = []
        unsubscribe = self.store.watch("renderer", lambda key, value: changes.append(value))
        unsubscribe()
        unsubscribe()
        self.store.set("renderer", "vulkan")
        self.assertEqual(changes, [])

    def test_snapshot_is_independent_of_later_changes(self):
        snapshot = self.store.snapshot()
        self.store.set("renderer", "vulkan")
        self.assertEqual(snapshot["renderer"], "opengl")

    def test_a_watcher_may_unsubscribe_while_being_notified(self):
        calls = []

        def once(key, value):
            calls.append(value)
            unsubscribe()

        unsubscribe = self.store.watch("renderer", once)
        self.store.set("renderer", "vulkan")
        self.store.set("renderer", "metal")
        self.assertEqual(calls, ["vulkan"])


class ObservableValueTests(unittest.TestCase):
    def test_notifies_on_change_only(self):
        value = core.ObservableValue()
        seen = []
        value.watch(seen.append)
        value.set(None)
        value.set((0.5, "half"))
        value.set((0.5, "half"))
        value.set(None)
        self.assertEqual(seen, [(0.5, "half"), None])
        self.assertIsNone(value.value)

    def test_the_same_object_is_not_a_change(self):
        session = object()
        value = core.ObservableValue(session)
        seen = []
        value.watch(seen.append)
        value.set(session)
        self.assertEqual(seen, [])


if __name__ == "__main__":
    unittest.main()
