import threading
from collections.abc import Mapping

__all__ = [
    "ObservableValue",
    "SettingsStore",
    "Subscribers",
    "same_value",
]


class Subscribers:
    def __init__(self):
        self._callbacks = []

    def add(self, callback):
        self._callbacks.append(callback)
        return lambda: self.remove(callback)

    def remove(self, callback):
        if callback in self._callbacks:
            self._callbacks.remove(callback)

    def notify(self, *args):
        for callback in list(self._callbacks):
            callback(*args)


def same_value(old, new):
    return type(old) is type(new) and old == new


class SettingsStore(Mapping):
    def __init__(self, values, save):
        self._values = dict(values)
        self._save = save
        self._lock = threading.Lock()
        self._subscribers = Subscribers()

    def __getitem__(self, key):
        with self._lock:
            return self._values[key]

    def __iter__(self):
        return iter(self.snapshot())

    def __len__(self):
        with self._lock:
            return len(self._values)

    def __setitem__(self, key, value):
        self.set(key, value)

    def snapshot(self):
        with self._lock:
            return dict(self._values)

    def set(self, key, value):
        if self._replace(key, value):
            self.save()
            self._subscribers.notify(key, value)

    def stage(self, key, value):
        if self._replace(key, value):
            self._subscribers.notify(key, value)

    def save(self):
        self._save(self.snapshot())

    def watch(self, keys, callback):
        watched = frozenset([keys] if isinstance(keys, str) else keys)

        def forward(key, value):
            if key in watched:
                callback(key, value)

        return self._subscribers.add(forward)

    def _replace(self, key, value):
        with self._lock:
            if key in self._values and same_value(self._values[key], value):
                return False
            self._values[key] = value
            return True


class ObservableValue:
    def __init__(self, value=None):
        self._value = value
        self._subscribers = Subscribers()

    @property
    def value(self):
        return self._value

    def set(self, value):
        if value is self._value or same_value(self._value, value):
            return
        self._value = value
        self._subscribers.notify(value)

    def watch(self, callback):
        return self._subscribers.add(callback)
