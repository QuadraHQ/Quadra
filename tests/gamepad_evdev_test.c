/* gamepad_evdev.c against a real kernel evdev device made with
 * tests/uinput_gamepad.py. Needs write access to /dev/uinput; without it the
 * test reports a skip and passes, so a CI machine without uinput still runs. */
#include <assert.h>
#include <math.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>
#include "../darling/shim/gamepad_evdev.c"

static FILE *to_fixture;
static void send_line(const char *line) { fputs(line, to_fixture); fputc('\n', to_fixture); fflush(to_fixture); }

/* Run qgp_wait until an event of `type` arrives for a pad. */
static qgp_event wait_for(int type) {
    qgp_event events[16];
    for (int i = 0; i < 100; i++) {
        int n = qgp_wait(100, events, 16);
        for (int k = 0; k < n; k++) if (events[k].type == type) return events[k];
    }
    fprintf(stderr, "timed out waiting for event %d\n", type);
    exit(1);
}

static int near(float a, float b) { return fabsf(a - b) < 0.01f; }

int main(void) {
    /* Which pads use the Xbox 360 button convention. Steam Input's virtual
     * pad reports Valve's vendor id but an Xbox name (from a Steam Deck). */
    assert(qgp_label_xy(0x28de, "Microsoft X-Box 360 pad 0"));
    assert(qgp_label_xy(0x045e, "Controller"));
    assert(qgp_label_xy(0x1234, "Generic XBOX Gamepad"));
    assert(!qgp_label_xy(0x054c, "Wireless Controller"));
    assert(!qgp_label_xy(0x057e, "Nintendo Switch Pro Controller"));
    assert(!qgp_label_xy(0x28de, "Valve Software Steam Controller"));

    if (access("/dev/uinput", W_OK) != 0) { puts("gamepad_evdev_test: skipped (no /dev/uinput)"); return 0; }
    int in[2], out[2];
    assert(pipe(in) == 0 && pipe(out) == 0);
    pid_t child = fork();
    if (child == 0) {
        dup2(in[0], 0); dup2(out[1], 1);
        close(in[1]); close(out[0]);
        execlp("python3", "python3", "-I", "tests/uinput_gamepad.py", (char *)0);
        _exit(127);
    }
    close(in[0]); close(out[1]);
    to_fixture = fdopen(in[1], "w");
    char ready[128] = {0};
    assert(read(out[0], ready, sizeof ready - 1) > 0 && strncmp(ready, "READY", 5) == 0);

    qgp_event added = wait_for(QGP_EVENT_CONNECTED);
    const qgp_pad *pad = qgp_pad_get(added.slot);
    assert(pad && strcmp(pad->name, "Quadra Test Gamepad") == 0);
    assert(pad->vendor == 0x045e && pad->product == 0x028e);
    assert(pad->buttons == 0 && near(pad->lx, 0) && near(pad->lt, 0));

    /* A and the left stick pushed up and right. Linux reports up as negative
     * Y; the pad must show it positive. */
    send_line("key 304 1"); send_line("abs 0 32767"); send_line("abs 1 -32768"); send_line("sync");
    wait_for(QGP_EVENT_CHANGED);
    pad = qgp_pad_get(added.slot);
    assert(pad->buttons & (1u << QGP_BUTTON_A));
    assert(near(pad->lx, 1) && near(pad->ly, 1));

    /* Triggers span 0...1023; the d-pad hat is two axes. */
    send_line("abs 2 1023"); send_line("abs 5 512"); send_line("abs 16 -1"); send_line("abs 17 1"); send_line("sync");
    wait_for(QGP_EVENT_CHANGED);
    pad = qgp_pad_get(added.slot);
    assert(near(pad->lt, 1) && near(pad->rt, 0.5f));
    assert(pad->buttons & (1u << QGP_DPAD_LEFT));
    assert(pad->buttons & (1u << QGP_DPAD_DOWN));
    assert(!(pad->buttons & (1u << QGP_DPAD_RIGHT)));

    /* Microsoft pads label X on BTN_NORTH (307); Y on BTN_WEST (308). */
    send_line("key 304 0"); send_line("key 307 1"); send_line("sync");
    wait_for(QGP_EVENT_CHANGED);
    pad = qgp_pad_get(added.slot);
    assert(!(pad->buttons & (1u << QGP_BUTTON_A)));
    assert(pad->buttons & (1u << QGP_BUTTON_X));
    assert(!(pad->buttons & (1u << QGP_BUTTON_Y)));

    /* The stick back inside its flat zone is exactly zero. */
    send_line("abs 0 50"); send_line("sync");
    wait_for(QGP_EVENT_CHANGED);
    assert(qgp_pad_get(added.slot)->lx == 0);

    /* Unplug. */
    send_line("quit");
    qgp_event removed = wait_for(QGP_EVENT_DISCONNECTED);
    assert(removed.slot == added.slot && qgp_pad_get(removed.slot) == 0);
    waitpid(child, 0, 0);
    puts("gamepad_evdev_test: ok");
    return 0;
}
