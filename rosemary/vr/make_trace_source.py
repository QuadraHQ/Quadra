"""Writes a copy of vr_device.c that logs what Roblox reads from getState.

Development check only: the original file is not modified, and the copy lands
in the build directory (see build_rosemary_vr.sh, QUADRA_VR_TRACE=1)."""
import sys

READ = "    return ((void *(*)(void *, void *))vr_debug_vtable[VR_SLOT_GET_STATE])(out, self);\n"
TRACE = r'''/* Trace copy: what Roblox gets from getState. */
static void vr_trace_state(const unsigned char *state) {
    static unsigned long reads, input_lines;
    unsigned long count = __atomic_add_fetch(&reads, 1, __ATOMIC_RELAXED);
    unsigned int changed;
    __builtin_memcpy(&changed, state + STATE_CHANGED_INPUTS, sizeof changed);
    if (changed && __atomic_add_fetch(&input_lines, 1, __ATOMIC_RELAXED) <= 40) {
        char text[200];
        int used = snprintf(text, sizeof text, "[Quadra VR trace] inputs changed 0x%x:", changed);
        for (int slot = 0; slot < VR_XR_INPUTS && used > 0 && used < (int)sizeof text - 24; ++slot) {
            float value;
            if (!(changed & (1u << slot))) continue;
            __builtin_memcpy(&value, state + STATE_INPUTS + slot * 4, sizeof value);
            used += snprintf(text + used, sizeof text - (unsigned long)used, " slot%d=%.2f", slot, (double)value);
        }
        if (used > 0 && used < (int)sizeof text - 1) text[used++] = '\n';
        if (used > 0) write(2, text, (unsigned long)used);
    }
    if (count > 3 && count % 600) return;
    float position[3], rotation[4], left[3], right[3];
    unsigned int session;
    __builtin_memcpy(position, state + STATE_HEAD + 4, sizeof position);
    __builtin_memcpy(rotation, state + STATE_HEAD + 0x10, sizeof rotation);
    __builtin_memcpy(left, state + STATE_HANDS + 4, sizeof left);
    __builtin_memcpy(right, state + STATE_HANDS + 0x20 + 4, sizeof right);
    __builtin_memcpy(&session, state + STATE_SESSION, sizeof session);
    char line[320];
    int length = snprintf(line, sizeof line,
        "[Quadra VR trace] getState #%lu: head valid=%d pos %.3f %.3f %.3f rot %.3f %.3f %.3f %.3f; "
        "hands valid=%d,%d; controllers=%d session=%u\n",
        count, state[STATE_HEAD], (double)position[0], (double)position[1], (double)position[2],
        (double)rotation[0], (double)rotation[1], (double)rotation[2], (double)rotation[3],
        state[STATE_HANDS], state[STATE_HANDS + 0x20], state[STATE_CONTROLLERS], session);
    if (length > 0) write(2, line, (unsigned long)length < sizeof line ? (unsigned long)length : sizeof line - 1);
}

'''
source = open(sys.argv[1]).read()
anchor = "/* The render thread reads the state first while preparing the camera,"
if source.count(READ) != 1 or source.count(anchor) != 1:
    sys.exit("vr_device.c is not the version this trace was written for")
source = source.replace(anchor, TRACE + anchor)
source = source.replace(READ, "    void *result = ((void *(*)(void *, void *))vr_debug_vtable[VR_SLOT_GET_STATE])(out, self);\n"
                             "    vr_trace_state(out);\n    return result;\n")
open(sys.argv[2], "w").write(source)
