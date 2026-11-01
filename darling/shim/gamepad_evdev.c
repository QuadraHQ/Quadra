/* Linux evdev gamepads for the controller bridge. See gamepad_evdev.h. */
#include "gamepad_evdev.h"
#include <stddef.h>

#if !defined(__x86_64__)
#error "gamepad_evdev.c issues x86-64 Linux system calls directly"
#endif

enum { SYS_READ = 0, SYS_CLOSE = 3, SYS_POLL = 7, SYS_IOCTL = 16, SYS_OPEN = 2,
       SYS_GETDENTS64 = 217, SYS_CLOCK_GETTIME = 228 };
enum { LINUX_O_NONBLOCK = 0x800, LINUX_O_DIRECTORY = 0x10000, LINUX_O_CLOEXEC = 0x80000 };
enum { LINUX_EAGAIN = 11, LINUX_ENODEV = 19, LINUX_EINTR = 4 };

/* Linux _IOR('E', nr, size): direction bits 0x80000000, then size, type, nr. */
#define EVIOC_R(nr, size) (0x80000000L | ((long)(size) << 16) | (0x45L << 8) | (nr))
#define EVIOCGID EVIOC_R(0x02, 8)
#define EVIOCGNAME80 EVIOC_R(0x06, 80)
#define EVIOCGKEY96 EVIOC_R(0x18, 96)
#define EVIOCGBIT(ev, len) EVIOC_R(0x20 + (ev), (len))
#define EVIOCGABS(abs) EVIOC_R(0x40 + (abs), 24)

enum { EV_SYN = 0, EV_KEY = 1, EV_ABS = 3, SYN_REPORT = 0, SYN_DROPPED = 3 };
enum {
    ABS_X = 0, ABS_Y = 1, ABS_Z = 2, ABS_RX = 3, ABS_RY = 4, ABS_RZ = 5,
    ABS_GAS = 9, ABS_BRAKE = 10, ABS_HAT0X = 16, ABS_HAT0Y = 17
};
enum {
    BTN_LEFT = 0x110, BTN_SOUTH = 0x130, BTN_EAST = 0x131, BTN_NORTH = 0x133,
    BTN_WEST = 0x134, BTN_TL = 0x136, BTN_TR = 0x137, BTN_TL2 = 0x138, BTN_TR2 = 0x139,
    BTN_SELECT = 0x13a, BTN_START = 0x13b, BTN_MODE = 0x13c, BTN_THUMBL = 0x13d,
    BTN_THUMBR = 0x13e, BTN_TOOL_PEN = 0x140, BTN_TOUCH = 0x14a,
    BTN_DPAD_UP = 0x220, BTN_DPAD_DOWN = 0x221, BTN_DPAD_LEFT = 0x222, BTN_DPAD_RIGHT = 0x223
};

struct input_event { long sec, usec; unsigned short type, code; int value; };
struct input_absinfo { int value, minimum, maximum, fuzz, flat, resolution; };
struct pollfd_ { int fd; short events, revents; };
struct timespec_ { long sec, nsec; };

static long sys3(long number, long a, long b, long c) {
    long result;
    __asm__ volatile("syscall" : "=a"(result) : "a"(number), "D"(a), "S"(b), "d"(c)
                     : "rcx", "r11", "memory");
    return result;
}

static int ioctl_(int fd, long request, void *argument) {
    return (int)sys3(SYS_IOCTL, fd, request, (long)argument);
}

typedef struct { int present, code, min, max, flat; } axis_t;
enum { AXIS_LX, AXIS_LY, AXIS_RX, AXIS_RY, AXIS_LT, AXIS_RT, AXIS_COUNT };

typedef struct {
    int fd;
    char node[32];              /* "event7", to recognise a pad already open */
    qgp_pad pad;
    axis_t axes[AXIS_COUNT];
    int label_xy;               /* Microsoft pads report X on BTN_NORTH */
    uint32_t key_buttons, hat_buttons;
    int changed;
} slot_t;

static slot_t slots[QGP_MAX_PADS];
static int slots_ready;
static long last_scan_ms = -1000000;

static long monotonic_ms(void) {
    struct timespec_ now;
    sys3(SYS_CLOCK_GETTIME, 1 /* CLOCK_MONOTONIC */, (long)&now, 0);
    return now.sec * 1000 + now.nsec / 1000000;
}

static int bit_set(const unsigned long long *bits, int code) {
    return (bits[code / 64] >> (code % 64)) & 1;
}

static float absf(float v) { return v < 0 ? -v : v; }
static float clampf(float v, float lo, float hi) { return v < lo ? lo : v > hi ? hi : v; }

static float stick_value(const axis_t *a, int raw, int invert) {
    if (!a->present || a->max <= a->min) return 0;
    float mid = (a->min + a->max) / 2.0f, half = (a->max - a->min) / 2.0f;
    float offset = (float)raw - mid;
    if (absf(offset) <= (float)a->flat) return 0;
    float x = clampf(offset / half, -1.0f, 1.0f);
    return invert ? -x : x;
}

static float trigger_value(const axis_t *a, int raw) {
    if (!a->present || a->max <= a->min) return 0;
    return clampf((float)(raw - a->min) / (float)(a->max - a->min), 0.0f, 1.0f);
}

static void set_key(slot_t *s, int code, int down) {
    int bit = -1;
    switch (code) {
    case BTN_SOUTH: bit = QGP_BUTTON_A; break;
    case BTN_EAST: bit = QGP_BUTTON_B; break;
    case BTN_NORTH: bit = s->label_xy ? QGP_BUTTON_X : QGP_BUTTON_Y; break;
    case BTN_WEST: bit = s->label_xy ? QGP_BUTTON_Y : QGP_BUTTON_X; break;
    case BTN_TL: bit = QGP_BUTTON_LEFT_SHOULDER; break;
    case BTN_TR: bit = QGP_BUTTON_RIGHT_SHOULDER; break;
    case BTN_START: bit = QGP_BUTTON_MENU; break;
    case BTN_SELECT: bit = QGP_BUTTON_OPTIONS; break;
    case BTN_MODE: bit = QGP_BUTTON_HOME; break;
    case BTN_THUMBL: bit = QGP_BUTTON_LEFT_STICK; break;
    case BTN_THUMBR: bit = QGP_BUTTON_RIGHT_STICK; break;
    case BTN_DPAD_UP: bit = QGP_DPAD_UP; break;
    case BTN_DPAD_DOWN: bit = QGP_DPAD_DOWN; break;
    case BTN_DPAD_LEFT: bit = QGP_DPAD_LEFT; break;
    case BTN_DPAD_RIGHT: bit = QGP_DPAD_RIGHT; break;
    /* Digital-only triggers (Switch Pro): shown as fully pressed triggers when
     * the pad has no analog ones. */
    case BTN_TL2: if (!s->axes[AXIS_LT].present) s->pad.lt = down ? 1.0f : 0.0f; return;
    case BTN_TR2: if (!s->axes[AXIS_RT].present) s->pad.rt = down ? 1.0f : 0.0f; return;
    default: return;
    }
    if (down) s->key_buttons |= 1u << bit; else s->key_buttons &= ~(1u << bit);
}

static void set_abs(slot_t *s, int code, int value) {
    qgp_pad *p = &s->pad;
    if (code == ABS_HAT0X || code == ABS_HAT0Y) {
        uint32_t lo = code == ABS_HAT0X ? 1u << QGP_DPAD_LEFT : 1u << QGP_DPAD_UP;
        uint32_t hi = code == ABS_HAT0X ? 1u << QGP_DPAD_RIGHT : 1u << QGP_DPAD_DOWN;
        s->hat_buttons &= ~(lo | hi);
        if (value < 0) s->hat_buttons |= lo; else if (value > 0) s->hat_buttons |= hi;
        return;
    }
    for (int i = 0; i < AXIS_COUNT; i++) {
        const axis_t *a = &s->axes[i];
        if (!a->present || a->code != code) continue;
        switch (i) {
        case AXIS_LX: p->lx = stick_value(a, value, 0); break;
        case AXIS_LY: p->ly = stick_value(a, value, 1); break;
        case AXIS_RX: p->rx = stick_value(a, value, 0); break;
        case AXIS_RY: p->ry = stick_value(a, value, 1); break;
        case AXIS_LT: p->lt = trigger_value(a, value); break;
        case AXIS_RT: p->rt = trigger_value(a, value); break;
        }
    }
}

static void publish(slot_t *s) { s->pad.buttons = s->key_buttons | s->hat_buttons; }

/* After SYN_DROPPED the kernel lost events: read the whole state again. */
static void resync(slot_t *s) {
    unsigned long long keys[12] = {0};
    if (ioctl_(s->fd, EVIOCGKEY96, keys) >= 0) {
        s->key_buttons = 0;
        static const int codes[] = {BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST, BTN_TL, BTN_TR,
            BTN_START, BTN_SELECT, BTN_MODE, BTN_THUMBL, BTN_THUMBR};
        for (size_t i = 0; i < sizeof codes / sizeof codes[0]; i++)
            if (bit_set(keys, codes[i])) set_key(s, codes[i], 1);
    }
    for (int code = 0; code <= ABS_HAT0Y; code++) {
        struct input_absinfo info;
        if (ioctl_(s->fd, EVIOCGABS(code), &info) >= 0) set_abs(s, code, info.value);
    }
    publish(s);
}

static void copy_name(char *to, size_t size, const char *from) {
    size_t i = 0;
    for (; from[i] && i + 1 < size; i++) to[i] = from[i];
    to[i] = 0;
}

static int starts_with(const char *s, const char *prefix) {
    while (*prefix) if (*s++ != *prefix++) return 0;
    return 1;
}

static void read_axis(int fd, unsigned long long *abs_bits, axis_t *axis, int code) {
    struct input_absinfo info;
    axis->present = 0;
    if (!bit_set(abs_bits, code) || ioctl_(fd, EVIOCGABS(code), &info) < 0) return;
    axis->present = 1;
    axis->code = code;
    axis->min = info.minimum;
    axis->max = info.maximum;
    axis->flat = info.flat;
}

/* Whether the pad follows the Xbox 360 button convention, where the face
 * button labelled X is reported as BTN_NORTH and Y as BTN_WEST (xpad's, which
 * Steam Input's virtual pad copies under Valve's own vendor id). Newer drivers
 * report buttons by position instead. Names catch third-party xpad clones;
 * the vendor catches Microsoft's pads on other drivers. */
static int contains_xbox(const char *name) {
    for (; *name; name++) {
        const char *n = name, *w = "x-box";
        while (*n && *w && ((*n | 0x20) == *w)) { n++; w++; }
        if (!*w) return 1;
        n = name; w = "xbox";
        while (*n && *w && ((*n | 0x20) == *w)) { n++; w++; }
        if (!*w) return 1;
    }
    return 0;
}

int qgp_label_xy(unsigned short vendor, const char *name) {
    return vendor == 0x045e || contains_xbox(name);
}

/* Open `node` if it is a gamepad. Returns the slot index or -1. */
static int open_pad(const char *node, int slot_index) {
    char path[48] = "/dev/input/";
    copy_name(path + 11, sizeof path - 11, node);
    long fd = sys3(SYS_OPEN, (long)path, LINUX_O_NONBLOCK | LINUX_O_CLOEXEC, 0);
    if (fd < 0) return -1;

    unsigned long long types[1] = {0}, keys[12] = {0}, abs_bits[1] = {0};
    if (ioctl_((int)fd, EVIOCGBIT(0, 8), types) < 0 || !bit_set(types, EV_KEY) ||
        !bit_set(types, EV_ABS) || ioctl_((int)fd, EVIOCGBIT(EV_KEY, 96), keys) < 0 ||
        ioctl_((int)fd, EVIOCGBIT(EV_ABS, 8), abs_bits) < 0 ||
        !bit_set(keys, BTN_SOUTH) || !bit_set(abs_bits, ABS_X) || !bit_set(abs_bits, ABS_Y) ||
        /* Mice, touchpads, tablets and pens that borrow a gamepad button. */
        bit_set(keys, BTN_LEFT) || bit_set(keys, BTN_TOUCH) || bit_set(keys, BTN_TOOL_PEN)) {
        sys3(SYS_CLOSE, fd, 0, 0);
        return -1;
    }

    slot_t *s = &slots[slot_index];
    *s = (slot_t){0};
    s->fd = (int)fd;
    copy_name(s->node, sizeof s->node, node);
    char name[80] = {0};
    if (ioctl_(s->fd, EVIOCGNAME80, name) < 0) copy_name(name, sizeof name, "Gamepad");
    copy_name(s->pad.name, sizeof s->pad.name, name);
    unsigned short id[4] = {0};
    if (ioctl_(s->fd, EVIOCGID, id) >= 0) {
        s->pad.bustype = id[0];
        s->pad.vendor = id[1];
        s->pad.product = id[2];
    }
    s->label_xy = qgp_label_xy(s->pad.vendor, s->pad.name);

    read_axis(s->fd, abs_bits, &s->axes[AXIS_LX], ABS_X);
    read_axis(s->fd, abs_bits, &s->axes[AXIS_LY], ABS_Y);
    read_axis(s->fd, abs_bits, &s->axes[AXIS_RX], ABS_RX);
    read_axis(s->fd, abs_bits, &s->axes[AXIS_RY], ABS_RY);
    read_axis(s->fd, abs_bits, &s->axes[AXIS_LT], bit_set(abs_bits, ABS_Z) ? ABS_Z : ABS_BRAKE);
    read_axis(s->fd, abs_bits, &s->axes[AXIS_RT], bit_set(abs_bits, ABS_RZ) ? ABS_RZ : ABS_GAS);

    s->pad.connected = 1;
    resync(s);
    return slot_index;
}

static int node_is_open(const char *node) {
    for (int i = 0; i < QGP_MAX_PADS; i++)
        if (slots[i].pad.connected) {
            const char *a = slots[i].node, *b = node;
            while (*a && *a == *b) { a++; b++; }
            if (*a == *b) return 1;
        }
    return 0;
}

static void scan(qgp_event *events, int *n, int capacity) {
    long dir = sys3(SYS_OPEN, (long)"/dev/input", LINUX_O_DIRECTORY | LINUX_O_CLOEXEC, 0);
    if (dir < 0) return;
    char buffer[2048];
    for (;;) {
        long bytes = sys3(SYS_GETDENTS64, dir, (long)buffer, sizeof buffer);
        if (bytes <= 0) break;
        for (long offset = 0; offset < bytes;) {
            unsigned short length = *(unsigned short *)(buffer + offset + 16);
            const char *node = buffer + offset + 19;
            offset += length;
            if (!starts_with(node, "event") || node_is_open(node) || *n >= capacity) continue;
            int free_slot = -1;
            for (int i = 0; i < QGP_MAX_PADS; i++)
                if (!slots[i].pad.connected) { free_slot = i; break; }
            if (free_slot < 0) goto done;
            if (open_pad(node, free_slot) >= 0)
                events[(*n)++] = (qgp_event){QGP_EVENT_CONNECTED, free_slot};
        }
    }
done:
    sys3(SYS_CLOSE, dir, 0, 0);
}

static void drain(slot_t *s) {
    struct input_event e;
    for (;;) {
        long r = sys3(SYS_READ, s->fd, (long)&e, sizeof e);
        if (r == (long)sizeof e) {
            if (e.type == EV_KEY && e.code != 0) {
                /* Auto-repeat (2) is not a state change. */
                if (e.value != 2) { set_key(s, e.code, e.value != 0); s->changed = 1; }
            } else if (e.type == EV_ABS) {
                set_abs(s, e.code, e.value);
                s->changed = 1;
            } else if (e.type == EV_SYN && e.code == SYN_DROPPED) {
                resync(s);
                s->changed = 1;
            }
            continue;
        }
        if (r == -LINUX_ENODEV) s->pad.connected = 0;
        break;
    }
    publish(s);
}

int qgp_wait(int timeout_ms, qgp_event *events, int capacity) {
    int n = 0;
    if (!slots_ready) {
        for (int i = 0; i < QGP_MAX_PADS; i++) slots[i].fd = -1;
        slots_ready = 1;
    }
    long now = monotonic_ms();
    if (now - last_scan_ms >= 1000) {
        scan(events, &n, capacity);
        last_scan_ms = now;
    }
    long until_scan = 1000 - (monotonic_ms() - last_scan_ms);
    if (until_scan < 0) until_scan = 0;
    int wait = n ? 0 : (timeout_ms < until_scan ? timeout_ms : (int)until_scan);

    struct pollfd_ fds[QGP_MAX_PADS];
    int owner[QGP_MAX_PADS], count = 0;
    for (int i = 0; i < QGP_MAX_PADS; i++)
        if (slots[i].pad.connected) {
            fds[count] = (struct pollfd_){slots[i].fd, 1 /* POLLIN */, 0};
            owner[count++] = i;
        }
    long ready = sys3(SYS_POLL, (long)fds, count, wait);
    if (ready > 0)
        for (int k = 0; k < count; k++) {
            if (!fds[k].revents) continue;
            slot_t *s = &slots[owner[k]];
            s->changed = 0;
            drain(s);
            if (!s->pad.connected) {
                if (n < capacity) events[n++] = (qgp_event){QGP_EVENT_DISCONNECTED, owner[k]};
                sys3(SYS_CLOSE, s->fd, 0, 0);
                s->fd = -1;
            } else if (s->changed && n < capacity) {
                events[n++] = (qgp_event){QGP_EVENT_CHANGED, owner[k]};
            }
        }
    return n;
}

const qgp_pad *qgp_pad_get(int slot) {
    if (slot < 0 || slot >= QGP_MAX_PADS || !slots[slot].pad.connected) return 0;
    return &slots[slot].pad;
}
