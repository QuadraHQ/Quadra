"""Writing files safely."""

__all__ = [
    "_write_atomically",
]


def _write_atomically(path, text):
    """Replace `path` in one step, so a crash or a full disk leaves the old file."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_text(text)
    temporary.replace(path)
