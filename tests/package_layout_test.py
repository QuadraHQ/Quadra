"""quadra.core and quadra.ui are split into modules that share one set of names.

`core.NAME` and `ui.NAME` only reach a name that its module lists in __all__
(see quadra/namespace.py), so a function added without listing it would fail
at run time. Read from the source: the UI needs no GTK for this.
"""
import ast
import unittest
from pathlib import Path

LAUNCHER = Path(__file__).resolve().parents[1] / "launcher" / "quadra"


def top_level_names(tree):
    names = []
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            names.append(node.name)
        elif isinstance(node, (ast.Assign, ast.AnnAssign)):
            targets = node.targets if isinstance(node, ast.Assign) else [node.target]
            names += [x.id for t in targets for x in ast.walk(t) if isinstance(x, ast.Name)]
    return [name for name in names if name != "__all__"]


def listed(tree):
    for node in tree.body:
        if isinstance(node, ast.Assign) and [getattr(t, "id", None) for t in node.targets] == ["__all__"]:
            return [element.value for element in node.value.elts]
    return None


def shared_modules(init):
    """The modules passed to namespace.share_names in a package's __init__."""
    for node in ast.walk(ast.parse(init.read_text())):
        if (isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                and node.func.attr == "share_names"):
            return [element.id for element in node.args[1].elts]
    return None


class PackageLayoutTests(unittest.TestCase):
    def check(self, package):
        folder = LAUNCHER / package
        modules = sorted(path.stem for path in folder.glob("*.py") if path.name != "__init__.py")
        self.assertEqual(sorted(shared_modules(folder / "__init__.py")), modules,
                         f"every {package}/ module is shared in {package}/__init__.py")
        owners = {}
        for module in modules:
            tree = ast.parse((folder / f"{module}.py").read_text())
            names = listed(tree)
            self.assertIsNotNone(names, f"{package}/{module}.py has __all__")
            self.assertEqual(sorted(names), sorted(set(top_level_names(tree))),
                             f"{package}/{module}.py lists exactly the names it defines in __all__")
            for name in names:
                self.assertNotIn(name, owners, f"{name} is in {owners.get(name)} and {module}")
                owners[name] = module

    def test_core(self):
        self.check("core")

    def test_ui(self):
        self.check("ui")


if __name__ == "__main__":
    unittest.main()
