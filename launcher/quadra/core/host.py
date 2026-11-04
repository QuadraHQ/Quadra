"""What the client is told about the host: the window icon and the GPU's memory."""

import csv
import struct
import subprocess
from pathlib import Path

from . import locations

__all__ = [
    "host_vram_bytes",
    "icon_argb_file",
]


def icon_argb_file():
    """Write the logo in _NET_WM_ICON layout for the shim (see QUADRA_ICON_ARGB)."""
    target = locations.CACHE_DIR / "icon-quadrahq.argb"
    sources = [
        (locations.ICONS / f"quadrahq-{size}.png", size) for size in (32, 64, 128)
    ]
    if target.exists() and all(
        source.exists() and target.stat().st_mtime >= source.stat().st_mtime
        for source, _size in sources
    ):
        return target
    from gi.repository import GdkPixbuf  # only needed here

    words = bytearray()
    for source, size in sources:
        if not source.exists():
            continue
        pixbuf = GdkPixbuf.Pixbuf.new_from_file_at_scale(str(source), size, size, True)
        if not pixbuf.get_has_alpha():
            pixbuf = pixbuf.add_alpha(False, 0, 0, 0)
        width, height, stride = (
            pixbuf.get_width(),
            pixbuf.get_height(),
            pixbuf.get_rowstride(),
        )
        pixels = pixbuf.get_pixels()
        words += struct.pack("<II", width, height)
        for y in range(height):
            row = pixels[y * stride : y * stride + width * 4]
            for x in range(0, width * 4, 4):
                r, g, b, a = row[x], row[x + 1], row[x + 2], row[x + 3]
                words += struct.pack("<I", (a << 24) | (r << 16) | (g << 8) | b)
    locations.CACHE_DIR.mkdir(parents=True, exist_ok=True)
    target.write_bytes(bytes(words))
    return target


def host_vram_bytes(renderer=None, adapter=None):
    """Conservative graphics budget for the selected GPU, never the largest.

    Reserve memory for the compositor/driver and account for current use. If
    the renderer does not identify one GPU, use the smallest measured budget
    rather than claiming the memory of a different adapter. Unknown adapters
    get 512 MiB instead of the former invented 8 GiB.
    """
    candidates = []

    def budget(total, free):
        if total < 64 * 1024 * 1024 or free < 0 or free > total:
            return None
        reserve = min(256 * 1024 * 1024, total // 4)
        return max(64 * 1024 * 1024, min(total * 3 // 4, max(0, free - reserve)))

    for path in Path("/sys/class/drm").glob("card*/device/mem_info_vram_total"):
        try:
            total = int(path.read_text().strip())
            used_file = path.with_name("mem_info_vram_used")
            used = (
                int(used_file.read_text().strip())
                if used_file.is_file()
                else total // 4
            )
            value = budget(total, total - used)
            if value is not None:
                candidates.append((path.parent.resolve().name, "", value))
        except (OSError, ValueError):
            pass
    try:
        fields = "name,memory.total,memory.free"
        if adapter:
            fields = "pci.bus_id," + fields
        out = subprocess.check_output(
            ["nvidia-smi", "--query-gpu=" + fields, "--format=csv,noheader,nounits"],
            stderr=subprocess.DEVNULL,
            text=True,
            timeout=1,
        )
        from ..gpus import pci_address

        for row in csv.reader(out.splitlines()):
            try:
                address = None
                parts = [part.strip() for part in row]
                if adapter:
                    address, name, total, free = parts
                    address = pci_address(address)
                else:
                    name, total, free = parts
                value = budget(int(total) * 1024 * 1024, int(free) * 1024 * 1024)
                if value is not None:
                    candidates.append((address, name, value))
            except ValueError:
                continue
    except (OSError, csv.Error, subprocess.SubprocessError):
        pass
    if adapter:
        values = [
            value for address, _name, value in candidates if address == adapter["pci"]
        ]
    else:
        selected = [
            value
            for _address, name, value in candidates
            if name and renderer and name.lower() in renderer.lower()
        ]
        values = selected or [value for _address, _name, value in candidates]
    return min(values) if values else 512 * 1024 * 1024
