#ifndef QUADRA_GAMEPAD_EVDEV_H
#define QUADRA_GAMEPAD_EVDEV_H
/* Linux evdev gamepads, read with raw Linux system calls.
 *
 * Darling's GameController framework is stubs and its IOHID has no Linux
 * devices behind it, so the client sees no controller at all. This module
 * finds the kernel's gamepads itself and keeps one normalised state per pad.
 * It issues Linux numbers directly (as thread_kick.c does), because Darling
 * does not translate evdev ioctls, and needs no libc headers, so the same file
 * builds into the shim and into a native test.
 *
 * Mapping follows the Linux gamepad convention (xpad, hid-playstation,
 * hid-nintendo, hid-steam, generic HID pads that follow it): buttons by
 * position, sticks on X/Y and RX/RY, triggers on Z and RZ, d-pad on HAT0.
 * Pads that report something else are shown with what they do report.
 *
 * Single-threaded: qgp_wait and qgp_pad_get belong to one thread.
 */
#include <stdint.h>

#define QGP_MAX_PADS 8

enum {
    QGP_BUTTON_A, QGP_BUTTON_B, QGP_BUTTON_X, QGP_BUTTON_Y,
    QGP_BUTTON_LEFT_SHOULDER, QGP_BUTTON_RIGHT_SHOULDER,
    QGP_BUTTON_MENU, QGP_BUTTON_OPTIONS, QGP_BUTTON_HOME,
    QGP_BUTTON_LEFT_STICK, QGP_BUTTON_RIGHT_STICK,
    QGP_DPAD_UP, QGP_DPAD_DOWN, QGP_DPAD_LEFT, QGP_DPAD_RIGHT,
    QGP_BUTTON_COUNT
};

typedef struct {
    int connected;
    char name[80];
    uint16_t bustype, vendor, product;
    uint32_t buttons;           /* bit n set = QGP_* button n is down */
    float lx, ly, rx, ry;       /* -1...1, up and right positive */
    float lt, rt;               /* 0...1 */
} qgp_pad;

enum { QGP_EVENT_CONNECTED = 1, QGP_EVENT_DISCONNECTED, QGP_EVENT_CHANGED };
typedef struct { int type; int slot; } qgp_event;

/* Wait up to `timeout_ms` for input, rescanning /dev/input about once a
 * second for pads that came or went. Returns the number of events written to
 * `events` (at most `capacity`), which is zero on a timeout. A pad that
 * changed several times in one call is reported once. */
int qgp_wait(int timeout_ms, qgp_event *events, int capacity);

/* Whether a pad with this vendor id and name reports X on BTN_NORTH (the Xbox
 * 360 convention) rather than by position. Exposed for the tests. */
int qgp_label_xy(unsigned short vendor, const char *name);

/* The pad in `slot`, or 0 when the slot is empty. Valid until the next
 * qgp_wait. */
const qgp_pad *qgp_pad_get(int slot);

#endif
