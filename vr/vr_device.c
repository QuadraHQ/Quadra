/* VR for Roblox's macOS client.
 *
 * The macOS RobloxPlayer still contains Roblox's VR pipeline: VRService, the
 * RBX::Graphics::DeviceVR interface, the stereo render path and the client's
 * VR emulator (DebugDeviceVR). Only the OpenXR device that the Windows and
 * Quest builds attach is missing. DeviceGL keeps a VR device pointer that
 * macOS never fills:
 *
 *     DeviceVR *getVR() { return vr && vrEnabled ? vr : NULL; }
 *     void setVR(bool on) { vrEnabled = on; }
 *
 * and its beginFrame and present already call the device's beginFrame and
 * endFrame. DeviceVR's virtual methods are the same table in the macOS
 * DebugDeviceVR and the Quest OpenXrDevice of the same release.
 *
 * Quadra attaches the client's own DebugDeviceVR there, with a copy of its
 * vtable: the emulator keeps creating the "VR Eye" targets and holding the
 * state the engine reads (getState copies it from the object), while Quadra's
 * methods fill that state from OpenXR and submit the eyes (vr_xr.c).
 *
 *   QUADRA_VR=openxr  present on the host OpenXR runtime (WiVRn, Monado...)
 *                       and mirror the left eye into the game window
 *   QUADRA_VR=openxr-headset  the headset only; the window is not presented
 *   ...-lag0            (development) submit the views the frame opened with,
 *                       instead of the previous frame's (see vr_xr.c)
 *   QUADRA_VR=1       emulated head only, left eye mirrored into the window
 *   QUADRA_VR=debug   also the client's own emulator mirror
 *                       (FFlagDebugEnableVREmulator)
 *
 * Quadra finds the classes, slots and offsets by disassembling RobloxPlayer
 * (src/backend/vr/layout.rs) and passes them in QUADRA_VR_LAYOUT, tied to the
 * binary's UUID. Any other client leaves VR off.
 */
#include <stddef.h>
#include <stdint.h>
#include "vr_xr.h"

extern const void *_dyld_get_image_header(unsigned int);
extern const char *_dyld_get_image_name(unsigned int);
extern char *getenv(const char *);
extern long write(int, const void *, unsigned long);
extern void *calloc(unsigned long, unsigned long);
extern void free(void *);
extern int snprintf(char *, unsigned long, const char *, ...);
extern int getpagesize(void);
extern int mprotect(void *, unsigned long, int);
extern unsigned long strlen(const char *);
extern void *memmem(const void *, unsigned long, const void *, unsigned long);
extern void *dlsym(void *, const char *);
extern void *pthread_self(void);
extern uint64_t mach_absolute_time(void);
extern int mach_timebase_info(unsigned int info[2]);

typedef struct {
    unsigned char uuid[16];
    unsigned long gl_vtable, debug_vtable, debug_constructor, emulator_flag, debug_methods;
    unsigned long gl_begin_frame_slot, gl_main_framebuffer_slot, gl_get_vr_slot, gl_set_vr_slot;
    unsigned long device_offset, enabled_offset, state_offset;
    unsigned long create_resources_slot, begin_frame_slot, end_frame_slot, present_window_slot;
    unsigned long display_rate_slot, set_display_rate_slot, mirror;
} VrLayout;

typedef struct {
    const char *name;
    unsigned long offset;
    int required;
} VrLayoutField;

static const VrLayoutField vr_layout_fields[] = {
    {"gl_vtable", offsetof(VrLayout, gl_vtable), 1},
    {"debug_vtable", offsetof(VrLayout, debug_vtable), 1},
    {"debug_constructor", offsetof(VrLayout, debug_constructor), 1},
    {"emulator_flag", offsetof(VrLayout, emulator_flag), 0},
    {"debug_methods", offsetof(VrLayout, debug_methods), 1},
    {"gl_begin_frame_slot", offsetof(VrLayout, gl_begin_frame_slot), 1},
    {"gl_main_framebuffer_slot", offsetof(VrLayout, gl_main_framebuffer_slot), 1},
    {"gl_get_vr_slot", offsetof(VrLayout, gl_get_vr_slot), 1},
    {"gl_set_vr_slot", offsetof(VrLayout, gl_set_vr_slot), 1},
    {"device_offset", offsetof(VrLayout, device_offset), 1},
    {"enabled_offset", offsetof(VrLayout, enabled_offset), 1},
    {"state_offset", offsetof(VrLayout, state_offset), 1},
    {"create_resources_slot", offsetof(VrLayout, create_resources_slot), 1},
    {"begin_frame_slot", offsetof(VrLayout, begin_frame_slot), 1},
    {"end_frame_slot", offsetof(VrLayout, end_frame_slot), 1},
    {"present_window_slot", offsetof(VrLayout, present_window_slot), 1},
    {"display_rate_slot", offsetof(VrLayout, display_rate_slot), 1},
    {"set_display_rate_slot", offsetof(VrLayout, set_display_rate_slot), 1},
    {"mirror", offsetof(VrLayout, mirror), 1},
};
#define VR_LAYOUT_FIELDS (sizeof vr_layout_fields / sizeof vr_layout_fields[0])

static VrLayout vr_layout;
#define VR_SLOTS_MAX 64
#define VR_SLOT_DELETE 1
#define VR_SLOT_UPDATE 2
#define VR_SLOT_HAPTICS 3
#define VR_SLOT_RECENTER 4
#define VR_SLOT_GET_STATE 5
#define VR_SLOT_EYE_FRAMEBUFFER 9
#define VR_SLOT_CREATE_RESOURCES (vr_layout.create_resources_slot)
#define VR_SLOT_BEGIN_FRAME (vr_layout.begin_frame_slot)
#define VR_SLOT_END_FRAME (vr_layout.end_frame_slot)
#define VR_SLOT_PRESENT_WINDOW (vr_layout.present_window_slot)
#define VR_SLOT_DISPLAY_RATE (vr_layout.display_rate_slot)
#define VR_SLOT_SET_DISPLAY_RATE (vr_layout.set_display_rate_slot)
#define VR_MIRROR_EYE 0 /* the left eye is what the window shows */
#define GL_SLOT_BEGIN_FRAME (vr_layout.gl_begin_frame_slot)
#define GL_SLOT_MAIN_FRAMEBUFFER (vr_layout.gl_main_framebuffer_slot)
/* DeviceVR state (getState copies 0x138 bytes), as filled by the Quest
 * OpenXrDevice of the same release: */
#define STATE_HEAD 0x00          /* u8 valid, float position[3] at +4, quaternion xyzw at +0x10 */
#define STATE_FLOOR 0x20         /* VRService UserCFrame.Floor, same layout */
#define STATE_HANDS 0x40         /* left, right: poses like the head, 0x20 apart */
#define STATE_CONTROLLERS 0x80   /* u8, UserInputService Gamepad1 connected */
#define STATE_CHANGED_INPUTS 0x84 /* u32, bit per input slot changed this frame */
#define STATE_INPUTS 0x88        /* float per input slot (VR_XR_INPUTS) */
#define STATE_EYE_OFFSETS 0xf8   /* float[2][3], eye positions in the head space, meters */
#define STATE_EYE_TANGENTS 0x110 /* float[2][4], tan of up, down, left, right */
#define STATE_VIBRATION 0x132    /* u8, the controllers can vibrate (HapticService LeftHand, RightHand) */
#define STATE_SESSION 0x134      /* u32, 0 idle, 1 visible, 2 focused, 3 stopping (VRService.VRSessionState) */
/* RBX::Graphics::Framebuffer size, and FramebufferGL's GL framebuffer name
 * (checked against its destructor). */
#define FRAMEBUFFER_WIDTH 0x70
#define FRAMEBUFFER_HEIGHT 0x74
#define FRAMEBUFFER_GL_NAME 0x8c

typedef struct {
    const unsigned char *header;
    unsigned char uuid[16];
    unsigned int data_protection;
} VrImage;

static void **vr_gl_vtable;
static void **vr_debug_vtable;
static void (*vr_debug_construct)(void *, int, int, void *);
static void *(*vr_gl_begin_frame)(void *); /* returns the frame's DeviceContext */
static void (*vr_gl_set_vr)(void *, int);
static volatile unsigned char *vr_emulator_flag;
static int vr_eye_size = 1024;
static int vr_openxr;
static unsigned long vr_next_start;
static void *vr_render_thread;
static int vr_frame_open;
static int vr_window; /* present the game window too (the mirror) */
static void vr_log(const char *text);

/* Per-frame phases, summed over a reporting window (ns). */
static struct {
    uint64_t open_start, open_end, draw_start, submit_start, submit_end, last_submit_end;
    uint64_t wait, camera, draw, submit, window, frames, late;
    uint64_t window_start;
} vr_timing;

static uint64_t vr_now(void) {
    static unsigned int timebase[2];
    if (!timebase[1]) mach_timebase_info(timebase);
    return mach_absolute_time() * timebase[0] / timebase[1];
}

static void vr_report_timing(void) {
    uint64_t frames = vr_timing.frames, span = vr_timing.submit_end - vr_timing.window_start;
    if (!frames || !span) return;
    char line[256];
    double scale = 1e-6 / (double)frames;
    snprintf(line, sizeof line,
             "[Quadra VR] %llu frames: %.1f fps at %.0f Hz, %llu late; per frame: wait %.1f ms, camera %.1f ms, "
             "draw %.1f ms, submit %.1f ms, window %.1f ms\n",
             (unsigned long long)frames, frames * 1e9 / (double)span, (double)vr_xr_display_rate(),
             (unsigned long long)vr_timing.late,
             vr_timing.wait * scale, vr_timing.camera * scale, vr_timing.draw * scale, vr_timing.submit * scale,
             vr_timing.window * scale);
    vr_log(line);
    vr_timing.wait = vr_timing.camera = vr_timing.draw = vr_timing.submit = vr_timing.window = 0;
    vr_timing.frames = vr_timing.late = 0;
    vr_timing.window_start = vr_timing.submit_end;
}
static int vr_state; /* 0 idle, 1 creating, 2 attached, 3 failed */
static void *vr_table[2 + VR_SLOTS_MAX]; /* offset-to-top, typeinfo, methods */
static unsigned long vr_frames, vr_begins, vr_ends, vr_eye_requests;
static void *vr_graphics_device;
static int vr_mirror_ready;

/* Window mirror: one eye, blitted before DeviceGL presents. */
static void (*vr_glBindFramebuffer)(unsigned int, unsigned int);
static void (*vr_glBlitFramebuffer)(int, int, int, int, int, int, int, int, unsigned int, unsigned int);
static void (*vr_glGetIntegerv)(unsigned int, int *);
static void (*vr_glGetBooleanv)(unsigned int, unsigned char *);
static unsigned char (*vr_glIsEnabled)(unsigned int);
static void (*vr_glEnable)(unsigned int);
static void (*vr_glDisable)(unsigned int);
static void (*vr_glColorMask)(unsigned char, unsigned char, unsigned char, unsigned char);

static void vr_log(const char *text) {
    write(2, text, strlen(text));
}

static void vr_logf2(const char *format, unsigned long first, unsigned long second) {
    char line[256];
    int length = snprintf(line, sizeof line, format, first, second);
    if (length > 0) write(2, line, (unsigned long)length < sizeof line ? (unsigned long)length : sizeof line - 1);
}

static unsigned int vr_u32(const void *pointer) {
    unsigned int value;
    __builtin_memcpy(&value, pointer, sizeof value);
    return value;
}

static int vr_bytes(const unsigned char *pointer, const unsigned char *expected, unsigned long size) {
    for (unsigned long index = 0; index < size; ++index)
        if (pointer[index] != expected[index]) return 0;
    return 1;
}

static int vr_section_name(const unsigned char *field, const char *name) {
    for (int index = 0; index < 16; ++index) {
        if (field[index] != (unsigned char)name[index]) return 0;
        if (!name[index]) return 1;
    }
    return 1;
}

/* The main executable as dyld loaded it; only RobloxPlayer, never the crash
 * handler that inherits the injected library. */
static int vr_image(VrImage *image) {
    const char *name = _dyld_get_image_name(0);
    unsigned long name_length = name ? strlen(name) : 0;
    if (name_length < 13 || !vr_bytes((const unsigned char *)name + name_length - 13,
                                      (const unsigned char *)"/RobloxPlayer", 13))
        return 0;
    const unsigned char *header = _dyld_get_image_header(0);
    if (!header || vr_u32(header) != 0xfeedfacf || vr_u32(header + 4) != 0x01000007) return 0;
    unsigned int count = vr_u32(header + 16), commands = vr_u32(header + 20);
    if (count > 512 || commands > 1048576) return 0;
    image->header = header;
    int has_uuid = 0;
    unsigned int position = 32;
    for (unsigned int index = 0; index < count; ++index) {
        if (position - 32 + 8 > commands) return 0;
        const unsigned char *command = header + position;
        unsigned int type = vr_u32(command), size = vr_u32(command + 4);
        if (size < 8 || size > commands - (position - 32)) return 0;
        if (type == 0x19 && size >= 72 && vr_section_name(command + 8, "__DATA")) /* LC_SEGMENT_64 */
            image->data_protection = vr_u32(command + 60);
        if (type == 0x1b && size >= 24) { /* LC_UUID */
            __builtin_memcpy(image->uuid, command + 8, sizeof image->uuid);
            has_uuid = 1;
        }
        position += size;
    }
    return has_uuid;
}

static int vr_named(const char *name, unsigned long length, const char *expected) {
    return strlen(expected) == length && vr_bytes((const unsigned char *)name, (const unsigned char *)expected, length);
}

static int vr_hex_digit(char digit) {
    if (digit >= '0' && digit <= '9') return digit - '0';
    if (digit >= 'a' && digit <= 'f') return digit - 'a' + 10;
    return -1;
}

static int vr_read_hex(const char *text, unsigned long length, unsigned long *value) {
    if (!length || length > 2 * sizeof *value) return 0;
    *value = 0;
    for (unsigned long index = 0; index < length; ++index) {
        int digit = vr_hex_digit(text[index]);
        if (digit < 0) return 0;
        *value = *value << 4 | (unsigned long)digit;
    }
    return 1;
}

static int vr_read_uuid(const char *text, unsigned long length, unsigned char uuid[16]) {
    if (length != 32) return 0;
    for (int index = 0; index < 16; ++index) {
        int high = vr_hex_digit(text[2 * index]), low = vr_hex_digit(text[2 * index + 1]);
        if (high < 0 || low < 0) return 0;
        uuid[index] = (unsigned char)(high << 4 | low);
    }
    return 1;
}

static int vr_read_layout(const char *text, VrLayout *layout) {
    unsigned long found = 0;
    int has_uuid = 0;
    while (*text) {
        const char *name = text;
        while (*text && *text != '=' && *text != ',') ++text;
        unsigned long name_length = (unsigned long)(text - name);
        if (*text != '=') return 0;
        const char *value = ++text;
        while (*text && *text != ',') ++text;
        unsigned long value_length = (unsigned long)(text - value);
        if (*text == ',') ++text;
        if (vr_named(name, name_length, "uuid")) {
            if (!vr_read_uuid(value, value_length, layout->uuid)) return 0;
            has_uuid = 1;
        }
        for (unsigned long index = 0; index < VR_LAYOUT_FIELDS; ++index) {
            if (!vr_named(name, name_length, vr_layout_fields[index].name)) continue;
            unsigned long *field = (unsigned long *)((unsigned char *)layout + vr_layout_fields[index].offset);
            if (!vr_read_hex(value, value_length, field)) return 0;
            found |= 1ul << index;
        }
    }
    for (unsigned long index = 0; index < VR_LAYOUT_FIELDS; ++index)
        if (vr_layout_fields[index].required && !(found & (1ul << index))) return 0;
    return has_uuid;
}

static int vr_layout_fits(const VrLayout *layout) {
    const unsigned long slots[] = {VR_SLOT_EYE_FRAMEBUFFER,     layout->create_resources_slot,
                                   layout->begin_frame_slot,    layout->end_frame_slot,
                                   layout->present_window_slot, layout->display_rate_slot,
                                   layout->set_display_rate_slot};
    if (layout->debug_methods >= VR_SLOTS_MAX) return 0;
    for (unsigned long index = 0; index < sizeof slots / sizeof slots[0]; ++index)
        if (slots[index] >= layout->debug_methods) return 0;
    return 1;
}

/* Counting copies of the engine-facing methods; they forward first, so
 * floating-point arguments reach the client's method unchanged. */
static void *vr_eye_framebuffer(void *self, long eye) {
    __atomic_fetch_add(&vr_eye_requests, 1, __ATOMIC_RELAXED);
    return ((void *(*)(void *, long))vr_debug_vtable[VR_SLOT_EYE_FRAMEBUFFER])(self, eye);
}

typedef void *(*VrForward)(void *, void *, void *, void *, void *, void *);

static int vr_framebuffer(const unsigned char *framebuffer, unsigned int *name, int *width, int *height) {
    if (!framebuffer) return 0;
    __builtin_memcpy(name, framebuffer + FRAMEBUFFER_GL_NAME, sizeof *name);
    __builtin_memcpy(width, framebuffer + FRAMEBUFFER_WIDTH, sizeof *width);
    __builtin_memcpy(height, framebuffer + FRAMEBUFFER_HEIGHT, sizeof *height);
    return *width > 0 && *height > 0 && *width <= 16384 && *height <= 16384;
}

/* Runs inside DeviceGL::present on the render thread; leaves the client's
 * framebuffer bindings, scissor and color mask as it found them. */
static void vr_mirror(void *self) {
    if (!vr_mirror_ready) return;
    unsigned int window_name, eye_name;
    int window_width, window_height, eye_width, eye_height;
    const unsigned char *window = ((const unsigned char *(*)(void *))vr_gl_vtable[GL_SLOT_MAIN_FRAMEBUFFER])(
        vr_graphics_device);
    if (!vr_framebuffer(window, &window_name, &window_width, &window_height)) return;
    int read_binding = 0, draw_binding = 0;
    unsigned char mask[4] = {1, 1, 1, 1};
    vr_glGetIntegerv(0x8CAA, &read_binding); /* GL_READ_FRAMEBUFFER_BINDING */
    vr_glGetIntegerv(0x8CA6, &draw_binding); /* GL_DRAW_FRAMEBUFFER_BINDING */
    vr_glGetBooleanv(0x0C23, mask);          /* GL_COLOR_WRITEMASK */
    unsigned char scissor = vr_glIsEnabled(0x0C11);
    if (scissor) vr_glDisable(0x0C11);
    vr_glColorMask(1, 1, 1, 1);
    const unsigned char *framebuffer =
        ((const unsigned char *(*)(void *, long))vr_debug_vtable[VR_SLOT_EYE_FRAMEBUFFER])(self, VR_MIRROR_EYE);
    if (vr_framebuffer(framebuffer, &eye_name, &eye_width, &eye_height)) {
        /* Fit the eye into the window, keeping its aspect. */
        int width = window_width, height = (int)((long)window_width * eye_height / eye_width);
        if (height > window_height) {
            height = window_height;
            width = (int)((long)window_height * eye_width / eye_height);
        }
        int x = (window_width - width) / 2, y = (window_height - height) / 2;
        vr_glBindFramebuffer(0x8CA9, window_name); /* GL_DRAW_FRAMEBUFFER */
        vr_glBindFramebuffer(0x8CA8, eye_name);    /* GL_READ_FRAMEBUFFER */
        vr_glBlitFramebuffer(0, 0, eye_width, eye_height, x, y, x + width, y + height,
                             0x4000 /* GL_COLOR_BUFFER_BIT */, 0x2601 /* GL_LINEAR */);
    }
    vr_glBindFramebuffer(0x8CA8, (unsigned int)read_binding);
    vr_glBindFramebuffer(0x8CA9, (unsigned int)draw_binding);
    vr_glColorMask(mask[0], mask[1], mask[2], mask[3]);
    if (scissor) vr_glEnable(0x0C11);
}

static int vr_load_mirror(void) {
    void *any = (void *)-2; /* RTLD_DEFAULT */
    vr_glBindFramebuffer = dlsym(any, "glBindFramebuffer");
    vr_glBlitFramebuffer = dlsym(any, "glBlitFramebuffer");
    vr_glGetIntegerv = dlsym(any, "glGetIntegerv");
    vr_glGetBooleanv = dlsym(any, "glGetBooleanv");
    vr_glIsEnabled = dlsym(any, "glIsEnabled");
    vr_glEnable = dlsym(any, "glEnable");
    vr_glDisable = dlsym(any, "glDisable");
    vr_glColorMask = dlsym(any, "glColorMask");
    return vr_glBindFramebuffer && vr_glBlitFramebuffer && vr_glGetIntegerv && vr_glGetBooleanv &&
           vr_glIsEnabled && vr_glEnable && vr_glDisable && vr_glColorMask;
}

/* Host (Linux) libraries are reached through Darling's ELF loader. */
typedef struct {
    void *(*open)(const char *, int);
    int (*close)(void *);
    void *(*symbol)(void *, const char *);
} VrElfCalls;
static VrElfCalls *vr_elf;
static void *(*vr_egl_get_current_display)(void);
static void *(*vr_egl_get_current_context)(void);
static void *(*vr_egl_get_proc_address)(const char *);
static int (*vr_egl_query_context)(void *, void *, int, int *);
static int (*vr_egl_choose_config)(void *, const int *, void **, int, int *);
static unsigned int (*vr_egl_swap_interval)(void *, int);

static void *vr_host_open(const char *library) { return vr_elf->open(library, 2 /* RTLD_NOW */); }
static void *vr_host_symbol(void *library, const char *name) { return vr_elf->symbol(library, name); }
static void *vr_gl_symbol(const char *name) { return dlsym((void *)-2 /* RTLD_DEFAULT */, name); }
static const VrXrHost vr_xr_host = {vr_host_open, vr_host_symbol, vr_gl_symbol, vr_log};

/* The OpenXR loader and runtime client run as host code and read the host
 * environment: the runtime manifest and the runtime's IPC directory. */
static int vr_prepare_host(void) {
    VrElfCalls **table = dlsym((void *)-2, "_elfcalls");
    if (!table || !*table) return 0;
    vr_elf = *table;
    void *libc = vr_host_open("libc.so.6"), *egl = vr_host_open("libEGL.so.1");
    if (!libc || !egl) return 0;
    char *(*host_getenv)(const char *) = vr_host_symbol(libc, "getenv");
    int (*host_setenv)(const char *, const char *, int) = vr_host_symbol(libc, "setenv");
    unsigned int (*host_getuid)(void) = vr_host_symbol(libc, "getuid");
    int (*host_access)(const char *, int) = vr_host_symbol(libc, "access");
    vr_egl_get_current_display = vr_host_symbol(egl, "eglGetCurrentDisplay");
    vr_egl_get_current_context = vr_host_symbol(egl, "eglGetCurrentContext");
    vr_egl_get_proc_address = vr_host_symbol(egl, "eglGetProcAddress");
    vr_egl_query_context = vr_host_symbol(egl, "eglQueryContext");
    vr_egl_choose_config = vr_host_symbol(egl, "eglChooseConfig");
    vr_egl_swap_interval = vr_host_symbol(egl, "eglSwapInterval");
    if (!host_getenv || !host_setenv || !host_getuid || !host_access || !vr_egl_get_current_display ||
        !vr_egl_get_current_context || !vr_egl_get_proc_address || !vr_egl_query_context || !vr_egl_choose_config)
        return 0;
    /* The launcher finds the host's active runtime (launcher/quadra/vr.py). */
    const char *runtime = getenv("QUADRA_XR_RUNTIME_JSON");
    if (runtime && *runtime) host_setenv("XR_RUNTIME_JSON", runtime, 1);
    char directory[64];
    snprintf(directory, sizeof directory, "/run/user/%u", host_getuid());
    const char *current = host_getenv("XDG_RUNTIME_DIR");
    if ((!current || host_access(current, 1)) && !host_access(directory, 1))
        host_setenv("XDG_RUNTIME_DIR", directory, 1);
    return 1;
}

static void vr_write_state(void *self, const VrXrViews *views) {
    unsigned char *state = (unsigned char *)self + vr_layout.state_offset;
    if (views->head_valid) {
        state[STATE_HEAD] = 1;
        __builtin_memcpy(state + STATE_HEAD + 4, views->head.position, 12);
        __builtin_memcpy(state + STATE_HEAD + 0x10, views->head.orientation, 16);
    }
    if (views->floor_valid) {
        state[STATE_FLOOR] = 1;
        __builtin_memcpy(state + STATE_FLOOR + 4, views->floor.position, 12);
        __builtin_memcpy(state + STATE_FLOOR + 0x10, views->floor.orientation, 16);
    }
    if (views->eyes_valid) {
        __builtin_memcpy(state + STATE_EYE_OFFSETS, views->eye_offset, 24);
        __builtin_memcpy(state + STATE_EYE_TANGENTS, views->tangent, 32);
    }
    unsigned int changed = views->controllers ? views->changed : 0;
    __builtin_memcpy(state + STATE_CHANGED_INPUTS, &changed, 4);
    if (!views->controllers) return;
    state[STATE_CONTROLLERS] = 1;
    state[STATE_VIBRATION] = 1;
    for (int slot = 0; slot < VR_XR_INPUTS; ++slot)
        if (changed & (1u << slot)) __builtin_memcpy(state + STATE_INPUTS + slot * 4, &views->input[slot], 4);
    for (int hand = 0; hand < 2; ++hand) {
        unsigned char *pose = state + STATE_HANDS + hand * 0x20;
        pose[0] = (unsigned char)views->hand_position_valid[hand];
        if (views->hand_position_valid[hand]) __builtin_memcpy(pose + 4, views->hand[hand].position, 12);
        if (views->hand_orientation_valid[hand]) __builtin_memcpy(pose + 0x10, views->hand[hand].orientation, 16);
    }
}

/* RenderView's per-frame VR update calls this right before reading the
 * state into VRService and the Gamepad1 events. The emulator's version
 * replays scripted presses and clears the change mask; with a headset the
 * state was already written when the frame opened, so nothing is left to do. */
static void vr_update(void *self) {
    (void)self;
}

/* Starts the headset frame: xrWaitFrame paces the game to the headset, and
 * the pose predicted for its display time stays in the state until endFrame
 * submits it, so the compositor reprojects from the pose Roblox drew. */
static void vr_open_frame(void *self) {
    if (vr_frame_open) return;
    VrXrViews views;
    vr_timing.open_start = vr_now();
    int opened = vr_xr_begin_frame();
    /* RenderView's VR update hands HapticService's motors to the controllers
     * only while the session is focused. */
    unsigned int session = (unsigned int)vr_xr_session_state();
    __builtin_memcpy((unsigned char *)self + vr_layout.state_offset + STATE_SESSION, &session, 4);
    if (opened) {
        vr_frame_open = 1;
        vr_timing.open_end = vr_timing.draw_start = vr_now();
        if (vr_xr_locate(&views)) vr_write_state(self, &views);
    }
}

/* The render thread reads the state first while preparing the camera,
 * before DeviceGL::beginFrame; the frame opens at that read. */
static void *vr_get_state(void *out, void *self) {
    if (vr_openxr && pthread_self() == vr_render_thread && !vr_frame_open) vr_open_frame(self);
    return ((void *(*)(void *, void *))vr_debug_vtable[VR_SLOT_GET_STATE])(out, self);
}

/* DeviceGL::beginFrame calls this at the start of drawing. */
static void *vr_begin_frame(void *self, void *a, void *b, void *c, void *d, void *e) {
    __atomic_fetch_add(&vr_begins, 1, __ATOMIC_RELAXED);
    if (!vr_openxr) return ((VrForward)vr_debug_vtable[VR_SLOT_BEGIN_FRAME])(self, a, b, c, d, e);
    vr_open_frame(self);
    vr_timing.draw_start = vr_now();
    return 0;
}

static void vr_submit(void *self) {
    int timed = vr_frame_open;
    vr_timing.submit_start = vr_now();
    unsigned int names[2] = {0, 0};
    int width = 0, height = 0;
    for (long eye = 0; eye < 2; ++eye) {
        const unsigned char *framebuffer =
            ((const unsigned char *(*)(void *, long))vr_debug_vtable[VR_SLOT_EYE_FRAMEBUFFER])(self, eye);
        if (!vr_framebuffer(framebuffer, &names[eye], &width, &height)) names[eye] = 0;
    }
    vr_xr_end_frame(names, width, height);
    vr_frame_open = 0;
    if (!timed) return;
    uint64_t end = vr_now();
    if (!vr_timing.window_start) vr_timing.window_start = end;
    if (vr_timing.last_submit_end) {
        uint64_t period = (uint64_t)vr_xr_display_period();
        if (period && end - vr_timing.last_submit_end > period + period / 2) ++vr_timing.late;
        vr_timing.window += vr_timing.open_start > vr_timing.last_submit_end
            ? vr_timing.open_start - vr_timing.last_submit_end : 0;
    }
    vr_timing.wait += vr_timing.open_end - vr_timing.open_start;
    vr_timing.camera += vr_timing.draw_start - vr_timing.open_end;
    vr_timing.draw += vr_timing.submit_start - vr_timing.draw_start;
    vr_timing.submit += end - vr_timing.submit_start;
    vr_timing.last_submit_end = vr_timing.submit_end = end;
    if (++vr_timing.frames == 900) vr_report_timing();
}

/* DeviceGL::present calls this after the frame is drawn, before the swap. */
static void *vr_end_frame(void *self, void *a, void *b, void *c, void *d, void *e) {
    void *result = 0;
    if (vr_openxr) vr_submit(self);
    else result = ((VrForward)vr_debug_vtable[VR_SLOT_END_FRAME])(self, a, b, c, d, e);
    if (vr_window) vr_mirror(self);
    unsigned long ends = __atomic_add_fetch(&vr_ends, 1, __ATOMIC_RELAXED);
    if (!vr_openxr && (ends == 1 || ends % 600 == 0))
        vr_logf2("[Quadra VR] Engine VR frames: %lu ended, %lu eye framebuffer requests\n",
                 ends, __atomic_load_n(&vr_eye_requests, __ATOMIC_RELAXED));
    return result;
}

/* DeviceGL::present swaps the game window only when the device answers
 * true. The Quest OpenXrDevice does; the emulator does not, because Studio
 * presents it elsewhere, which left Quadra's window black. */
static long vr_present_window(void *self) {
    (void)self;
    return vr_window;
}

/* HapticService: (hand 0 left / 1 right, amplitude 0..1), 0 stops. */
static void vr_haptics(void *self, int hand, float amplitude) {
    (void)self;
    static int logged;
    if (amplitude > 0 && !__atomic_exchange_n(&logged, 1, __ATOMIC_RELAXED))
        vr_log("[Quadra VR] Roblox started controller vibration\n");
    vr_xr_set_haptic(hand, amplitude);
}

/* VRService:RecenterUserHeadCFrame(). */
static void vr_recenter(void *self) {
    (void)self;
    vr_log("[Quadra VR] Roblox asked to recenter\n");
    vr_xr_recenter();
}

/* RenderView's VR update reports the display rate and asks for one. */
static float vr_display_rate(void *self) {
    (void)self;
    return vr_xr_display_rate();
}

static void vr_set_display_rate(void *self, float hertz) {
    (void)self;
    vr_xr_request_display_rate(hertz);
}

/* DeviceGL's destructor deletes the device through its vtable. */
static void vr_delete(void *self) {
    ((void (*)(void *))vr_debug_vtable[0])(self);
    free(self);
    vr_log("[Quadra VR] Device released with the graphics device\n");
}

static void vr_attach(void *device, int width, int height) {
    void *object = calloc(1, 4096); /* DebugDeviceVR is 0x228 bytes in 0.743 */
    if (!object) {
        __atomic_store_n(&vr_state, 3, __ATOMIC_RELEASE);
        return;
    }
    vr_debug_construct(object, width, height, device);
    for (unsigned long index = 0; index < 2 + vr_layout.debug_methods; ++index) vr_table[index] = vr_debug_vtable[index - 2];
    vr_table[2 + VR_SLOT_DELETE] = (void *)vr_delete;
    vr_table[2 + VR_SLOT_EYE_FRAMEBUFFER] = (void *)vr_eye_framebuffer;
    vr_table[2 + VR_SLOT_BEGIN_FRAME] = (void *)vr_begin_frame;
    vr_table[2 + VR_SLOT_END_FRAME] = (void *)vr_end_frame;
    vr_table[2 + VR_SLOT_PRESENT_WINDOW] = (void *)vr_present_window;
    if (vr_openxr) {
        vr_table[2 + VR_SLOT_UPDATE] = (void *)vr_update;
        vr_table[2 + VR_SLOT_GET_STATE] = (void *)vr_get_state;
        vr_table[2 + VR_SLOT_HAPTICS] = (void *)vr_haptics;
        vr_table[2 + VR_SLOT_RECENTER] = (void *)vr_recenter;
        vr_table[2 + VR_SLOT_DISPLAY_RATE] = (void *)vr_display_rate;
        vr_table[2 + VR_SLOT_SET_DISPLAY_RATE] = (void *)vr_set_display_rate;
        vr_render_thread = pthread_self(); /* attach runs inside DeviceGL::beginFrame */
        /* The headset paces the game now; waiting for the desktop's vblank
         * as well only takes time from the headset frame. */
        if (vr_window && vr_egl_swap_interval && vr_egl_get_current_display &&
            vr_egl_swap_interval(vr_egl_get_current_display(), 0))
            vr_log("[Quadra VR] Game window presents without vsync while in VR\n");
    }
    *(void **)object = &vr_table[2];
    /* Creates the "VR Eye" color targets through the client's own device. */
    ((void (*)(void *, void *))vr_debug_vtable[VR_SLOT_CREATE_RESOURCES])(object, device);
    if (vr_emulator_flag) *vr_emulator_flag = 1;
    vr_graphics_device = device;
    if (vr_mirror_ready && !vr_load_mirror()) {
        vr_mirror_ready = 0;
        vr_log("[Quadra VR] Window mirror unavailable: OpenGL blit functions missing\n");
    }
    *(void **)((unsigned char *)device + vr_layout.device_offset) = object;
    *((unsigned char *)device + vr_layout.enabled_offset) = 1;
    void *reported = ((void *(*)(void *))vr_gl_vtable[vr_layout.gl_get_vr_slot])(device);
    vr_logf2("[Quadra VR] Attached the client's DebugDeviceVR (%lux%lu per eye)\n",
             (unsigned long)width, (unsigned long)height);
    vr_log(reported == object ? "[Quadra VR] Graphics device reports the VR device\n"
                              : "[Quadra VR] Graphics device does not report the VR device\n");
    __atomic_store_n(&vr_state, 2, __ATOMIC_RELEASE);
}

/* DeviceGL::beginFrame runs on the render thread with its context current. */
static void *vr_gl_begin_frame_hook(void *device) {
    void *context = vr_gl_begin_frame(device);
    if (__atomic_add_fetch(&vr_frames, 1, __ATOMIC_RELAXED) == 1)
        vr_log("[Quadra VR] First graphics frame\n");
    if (__atomic_load_n(&vr_state, __ATOMIC_ACQUIRE) != 0) return context;
    if (!vr_openxr) {
        __atomic_store_n(&vr_state, 1, __ATOMIC_RELAXED);
        vr_attach(device, vr_eye_size, vr_eye_size);
        return context;
    }
    /* Until a runtime and headset answer, retry every few hundred frames
     * and leave the game on the desktop. */
    if (vr_frames < vr_next_start) return context;
    vr_next_start = vr_frames + 300;
    if (!vr_elf && !vr_prepare_host()) {
        vr_log("[Quadra VR] Unavailable: Darling's host library bridge or host EGL is missing\n");
        __atomic_store_n(&vr_state, 3, __ATOMIC_RELEASE);
        return context;
    }
    int started = vr_xr_start(&vr_xr_host, vr_egl_get_current_display(), vr_egl_get_current_context(),
                              vr_egl_get_proc_address, vr_egl_query_context, vr_egl_choose_config);
    if (started < 0) {
        __atomic_store_n(&vr_state, 3, __ATOMIC_RELEASE);
    } else if (started > 0) {
        int width, height;
        vr_xr_eye_size(&width, &height);
        __atomic_store_n(&vr_state, 1, __ATOMIC_RELAXED);
        vr_attach(device, width, height);
    }
    return context;
}

static void vr_gl_set_vr_hook(void *device, int enabled) {
    static int logged;
    if (__atomic_add_fetch(&logged, 1, __ATOMIC_RELAXED) <= 16)
        vr_log(enabled & 0xff ? "[Quadra VR] Client requested VR on\n" : "[Quadra VR] Client requested VR off\n");
    vr_gl_set_vr(device, enabled);
}

static int vr_patch_slot(const VrImage *image, void **slot, void *replacement) {
    unsigned long page = (unsigned long)getpagesize();
    unsigned long base = (unsigned long)slot & ~(page - 1);
    int writable = (image->data_protection & 2) != 0;
    if (!writable && mprotect((void *)base, page, 3)) return 0;
    __atomic_store_n(slot, replacement, __ATOMIC_RELEASE);
    if (!writable && mprotect((void *)base, page, 1))
        vr_log("[Quadra VR] Could not restore graphics vtable protection\n");
    return 1;
}

__attribute__((constructor)) static void vr_install(void) {
    char mode[16] = {0};
    const char *requested = getenv("QUADRA_VR"); /* from the launcher's VR setting */
    if (!requested) return;
    unsigned long length = strlen(requested);
    if (length >= sizeof mode) return;
    __builtin_memcpy(mode, requested, length);
    int debug = vr_bytes((const unsigned char *)mode, (const unsigned char *)"debug", 6);
    /* openxr[-headset][-lag0] */
    vr_openxr = vr_bytes((const unsigned char *)mode, (const unsigned char *)"openxr", 6);
    int headset_only = vr_openxr && memmem(mode, strlen(mode), "-headset", 8) != 0;
    if (vr_openxr) vr_xr_set_submit_lag(memmem(mode, strlen(mode), "-lag0", 5) == 0);
    vr_window = !headset_only;
    if (!debug && !vr_openxr && !vr_bytes((const unsigned char *)mode, (const unsigned char *)"1", 2)) return;
    VrImage image = {0};
    if (!vr_image(&image)) return; /* not RobloxPlayer */
    const char *size = getenv("QUADRA_VR_EYE_SIZE");
    if (size) {
        int value = 0;
        for (const char *digit = size; *digit >= '0' && *digit <= '9' && value < 100000; ++digit)
            value = value * 10 + (*digit - '0');
        if (value >= 256 && value <= 4096) vr_eye_size = value;
    }
    /* QUADRA_VR_SCALE: percent of the headset's recommended eye size. */
    const char *scale = getenv("QUADRA_VR_SCALE");
    int percent = 70;
    if (scale) {
        int value = 0;
        for (const char *digit = scale; *digit >= '0' && *digit <= '9' && value < 1000; ++digit)
            value = value * 10 + (*digit - '0');
        if (value >= 25 && value <= 200) percent = value;
    }
    vr_xr_set_render_scale(percent / 100.0f);
    const char *follow = getenv("QUADRA_VR_FOLLOW_RATE");
    vr_xr_follow_requested_rate(follow && follow[0] == '1');
    const char *layout = getenv("QUADRA_VR_LAYOUT");
    if (!layout || !vr_read_layout(layout, &vr_layout)) {
        vr_log("[Quadra VR] Unavailable: Quadra didn't pass a usable VR layout (QUADRA_VR_LAYOUT)\n");
        return;
    }
    if (!vr_bytes(vr_layout.uuid, image.uuid, sizeof image.uuid)) {
        vr_log("[Quadra VR] Unavailable: the VR layout is for a different Roblox build\n");
        return;
    }
    if (!vr_layout_fits(&vr_layout)) {
        vr_log("[Quadra VR] Unavailable: unexpected VR device method table\n");
        return;
    }
    vr_gl_vtable = (void **)(image.header + vr_layout.gl_vtable);
    vr_debug_vtable = (void **)(image.header + vr_layout.debug_vtable);
    vr_debug_construct = (void (*)(void *, int, int, void *))(image.header + vr_layout.debug_constructor);
    vr_logf2("[Quadra VR] VR device layout: %lu methods, beginFrame in slot %lu\n", vr_layout.debug_methods,
             VR_SLOT_BEGIN_FRAME);
    vr_mirror_ready = vr_layout.mirror != 0;
    if (!vr_mirror_ready) vr_log("[Quadra VR] Window mirror unavailable: unexpected framebuffer layout\n");
    if (debug) {
        if (vr_layout.emulator_flag)
            vr_emulator_flag = (volatile unsigned char *)(image.header + vr_layout.emulator_flag);
        else
            vr_log("[Quadra VR] Emulator mirror flag not found; continuing without it\n");
    }
    vr_gl_begin_frame = (void *(*)(void *))vr_gl_vtable[GL_SLOT_BEGIN_FRAME];
    vr_gl_set_vr = (void (*)(void *, int))vr_gl_vtable[vr_layout.gl_set_vr_slot];
    if (!vr_patch_slot(&image, &vr_gl_vtable[GL_SLOT_BEGIN_FRAME], (void *)vr_gl_begin_frame_hook) ||
        !vr_patch_slot(&image, &vr_gl_vtable[vr_layout.gl_set_vr_slot], (void *)vr_gl_set_vr_hook)) {
        vr_log("[Quadra VR] Unavailable: graphics vtable is not writable\n");
        return;
    }
    vr_log(vr_openxr ? "[Quadra VR] OpenXR mode: waiting for the first graphics frame\n"
                     : "[Quadra VR] Emulated head mode\n");
    vr_logf2("[Quadra VR] Graphics device VR slot +0x%lx, enabled flag +0x%lx\n", vr_layout.device_offset, vr_layout.enabled_offset);
}
