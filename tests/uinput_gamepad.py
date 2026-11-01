#!/usr/bin/env python3
"""A virtual Xbox-style gamepad on /dev/uinput, for the controller bridge tests.

Needs write access to /dev/uinput (the `input` group on most distributions).
It prints `READY <event-node>` once the device exists, then reads commands from
stdin, one per line, until EOF:

    key <code> <0|1>     press/release an EV_KEY button (BTN_SOUTH = 304)
    abs <code> <value>   move an EV_ABS axis (ABS_X = 0, ABS_HAT0X = 16)
    sync                 flush the pending events as one report
    sleep <seconds>
    quit

The device disappears when the process exits, which is the unplug case.
"""
import fcntl
import os
import struct
import sys
import time

EV_SYN, EV_KEY, EV_ABS = 0, 1, 3
UI_DEV_CREATE, UI_DEV_DESTROY = 0x5501, 0x5502
UI_DEV_SETUP, UI_ABS_SETUP = 0x405C5503, 0x401C5504
UI_SET_EVBIT, UI_SET_KEYBIT, UI_SET_ABSBIT = 0x40045564, 0x40045565, 0x40045567

BUTTONS = [0x130, 0x131, 0x133, 0x134, 0x136, 0x137, 0x13A, 0x13B, 0x13C, 0x13D, 0x13E]
# (code, minimum, maximum, flat) as xpad reports them.
AXES = [
    (0x00, -32768, 32767, 128), (0x01, -32768, 32767, 128),   # left stick
    (0x03, -32768, 32767, 128), (0x04, -32768, 32767, 128),   # right stick
    (0x02, 0, 1023, 0), (0x05, 0, 1023, 0),                   # triggers
    (0x10, -1, 1, 0), (0x11, -1, 1, 0),                       # d-pad hat
]


def create(name="Quadra Test Gamepad"):
    fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
    fcntl.ioctl(fd, UI_SET_EVBIT, EV_KEY)
    fcntl.ioctl(fd, UI_SET_EVBIT, EV_ABS)
    for code in BUTTONS:
        fcntl.ioctl(fd, UI_SET_KEYBIT, code)
    for code, lo, hi, flat in AXES:
        fcntl.ioctl(fd, UI_SET_ABSBIT, code)
        # struct uinput_abs_setup: u16 code, pad, input_absinfo{value,min,max,fuzz,flat,res}
        fcntl.ioctl(fd, UI_ABS_SETUP, struct.pack("=Hxx6i", code, 0, lo, hi, 0, flat, 0))
    setup = struct.pack("=HHHH80sI", 0x03, 0x045E, 0x028E, 0x0110, name.encode(), 0)
    fcntl.ioctl(fd, UI_DEV_SETUP, setup)
    fcntl.ioctl(fd, UI_DEV_CREATE)
    return fd


def emit(fd, kind, code, value):
    os.write(fd, struct.pack("=qqHHi", 0, 0, kind, code, value))


def event_node(name):
    """The /dev/input/eventN the kernel gave the new device."""
    for _ in range(100):
        block = open("/proc/bus/input/devices").read().split("\n\n")
        for entry in block:
            if f'Name="{name}"' in entry:
                for token in entry.replace("=", " ").split():
                    if token.startswith("event"):
                        return "/dev/input/" + token
        time.sleep(0.05)
    raise SystemExit("device node did not appear")


def main():
    name = "Quadra Test Gamepad"
    fd = create(name)
    print("READY", event_node(name), flush=True)
    try:
        for line in sys.stdin:
            parts = line.split()
            if not parts:
                continue
            if parts[0] == "key":
                emit(fd, EV_KEY, int(parts[1]), int(parts[2]))
            elif parts[0] == "abs":
                emit(fd, EV_ABS, int(parts[1]), int(parts[2]))
            elif parts[0] == "sync":
                emit(fd, EV_SYN, 0, 0)
            elif parts[0] == "sleep":
                time.sleep(float(parts[1]))
            elif parts[0] == "quit":
                break
    finally:
        fcntl.ioctl(fd, UI_DEV_DESTROY)
        os.close(fd)


if __name__ == "__main__":
    main()
