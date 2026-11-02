"""Lets a package answer for the names of the modules it is split into.

`quadra.core` and `quadra.ui` are folders of modules, and each name lives in
exactly one of those modules (the one whose __all__ lists it). After
`share_names`, `package.NAME` reads, sets and deletes NAME in that module. So
`core.load_settings()` works as before, and a test that patches
`core.roblox_pids` changes it for every module that calls it, because they all
look it up in the same place (`process_control.roblox_pids`).
"""
import sys
import types


class _SharedPackage(types.ModuleType):
    """A package module whose names are kept in its modules; see share_names."""

    _homes = {}

    def __getattr__(self, name):  # only called for names the package itself lacks
        home = type(self)._homes.get(name)
        if home is None:
            raise AttributeError(f"module {self.__name__!r} has no attribute {name!r}")
        return getattr(home, name)

    def __setattr__(self, name, value):
        home = type(self)._homes.get(name)
        if home is None:
            super().__setattr__(name, value)
        else:
            setattr(home, name, value)

    def __delattr__(self, name):
        home = type(self)._homes.get(name)
        if home is None:
            super().__delattr__(name)
        else:
            delattr(home, name)

    def __dir__(self):
        return sorted(set(super().__dir__()) | set(type(self)._homes))


def share_names(package_name, modules):
    """Make the package `package_name` stand for the names in `modules`' __all__."""
    homes = {}
    for module in modules:
        for name in module.__all__:
            if name in homes:
                raise ImportError(f"{name} is in both {homes[name].__name__} and {module.__name__}")
            homes[name] = module
    package = sys.modules[package_name]
    clashes = [name for name in homes if name in vars(package)]
    if clashes:
        raise ImportError(f"{package_name} already has {clashes}, which its modules also define")
    package.__class__ = type("SharedPackage", (_SharedPackage,), {"_homes": homes})
