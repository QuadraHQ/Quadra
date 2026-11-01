//! What happens when Vulkan reports the device lost.
//!
//! A lost `VkDevice` never comes back, and the runtime has no way to make a new one under a running
//! game: every later command buffer and buffer fails. The game, handed `nil` objects it does not
//! expect, then dereferences a NULL `contents` pointer and dies with a segmentation fault that
//! says nothing. This module instead writes down what the GPU's memory looked like, takes the last
//! sample of the performance trace, and ends the game with a status the launch log can be read
//! against.
//!
//! `MACOBLOX_METAL_DEVICE_LOSS=continue` keeps the old behaviour for debugging a crash under a
//! debugger.
use metal2vulkan_runtime::{CommandBufferError, Device, Error};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// The exit status of a game whose GPU device was lost.
pub const EXIT_STATUS: i32 = 87;
/// How long the final trace sample may take before the process ends regardless: sampling takes the
/// runtime's locks, which a lost device could leave held.
const SAMPLE_LIMIT: Duration = Duration::from_secs(2);

const MIB: u64 = 1 << 20;

static HANDLED: AtomicBool = AtomicBool::new(false);

/// Whether this process has seen its device lost (the trace's last sample says so).
pub fn happened() -> bool {
    HANDLED.load(Ordering::SeqCst)
}

pub fn is_lost(error: &Error) -> bool {
    matches!(error, Error::DeviceLost)
}

pub fn command_buffer_lost(error: Option<CommandBufferError>) -> bool {
    matches!(error, Some(CommandBufferError::DeviceLost))
}

/// The numbers the report gives, gathered apart from the device so the wording can be tested.
#[derive(Default, Debug, Clone, Copy)]
pub struct Picture {
    pub live: u64,
    pub reserved: u64,
    pub device_local_reserved: u64,
    /// GTT total and in use, where the driver reports them.
    pub gtt: Option<(u64, u64)>,
    pub buffers_held: u64,
    pub buffer_bytes_held: u64,
}

/// The launch-log lines for a device lost `during` the named operation.
pub fn describe(during: &str, picture: &Picture, exiting: bool) -> String {
    let mut text = format!(
        "[Quadra Metal] The GPU device was lost during {during}. Vulkan cannot recover a lost \
         device, so {}.\n[Quadra Metal] Graphics memory then: {} MiB live, {} MiB reserved ({} MiB \
         device-local); the game holds {} buffers of {} MiB.",
        if exiting {
            "the game is being closed here instead of crashing later"
        } else {
            "everything after this fails"
        },
        picture.live / MIB,
        picture.reserved / MIB,
        picture.device_local_reserved / MIB,
        picture.buffers_held,
        picture.buffer_bytes_held / MIB,
    );
    if let Some((total, used)) = picture.gtt {
        text.push_str(&format!(
            "\n[Quadra Metal] The GPU's GTT (system memory it may map) was {} MiB in use of {} MiB. \
             Running out of it is how an integrated GPU loses its device; include this log when \
             reporting it.",
            used / MIB,
            total / MIB
        ));
    }
    text
}

fn exits() -> bool {
    !std::env::var("MACOBLOX_METAL_DEVICE_LOSS")
        .is_ok_and(|value| value.trim().eq_ignore_ascii_case("continue"))
}

/// Records the loss once and, unless told not to, ends the process.
pub fn on_loss(device: &Device, during: &str) {
    if HANDLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let memory = device.memory_statistics();
    let held = crate::resources::snapshot();
    let picture = Picture {
        live: memory.live_bytes,
        reserved: memory.reserved_bytes,
        device_local_reserved: memory.device_local_reserved_bytes,
        gtt: crate::gtt::counters(),
        buffers_held: held["buffers_held"].as_u64().unwrap_or(0),
        buffer_bytes_held: held["buffer_bytes_held"].as_u64().unwrap_or(0),
    };
    let exiting = exits();
    crate::util::log_once(&describe(during, &picture, exiting));
    if !exiting {
        return;
    }
    // Whatever the sample does, the process ends.
    std::thread::spawn(|| {
        std::thread::sleep(SAMPLE_LIMIT);
        terminate();
    });
    crate::diagnostics::sample(device, true);
    terminate();
}

fn terminate() -> ! {
    use std::io::Write;
    let _ = std::io::stderr().flush();
    // _exit: the guest's atexit handlers and the other threads' locks are exactly what a lost
    // device leaves unreliable.
    unsafe { libc::_exit(EXIT_STATUS) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_lost_device_is_a_loss() {
        assert!(is_lost(&Error::DeviceLost));
        assert!(command_buffer_lost(Some(CommandBufferError::DeviceLost)));
        assert!(!command_buffer_lost(None));
        assert!(!command_buffer_lost(Some(CommandBufferError::PageFault)));
    }

    #[test]
    fn the_report_names_the_memory_the_gpu_was_holding_and_the_gtt_when_known() {
        let picture = Picture {
            live: 3246 * MIB,
            reserved: 3633 * MIB,
            device_local_reserved: 689 * MIB,
            gtt: Some((3248 * MIB, 3246 * MIB)),
            buffers_held: 1065,
            buffer_bytes_held: 3000 * MIB,
        };
        let text = describe("newBufferWithLength", &picture, true);
        assert!(text.contains("lost during newBufferWithLength"));
        assert!(text.contains("closed here instead of crashing later"));
        assert!(text.contains("3246 MiB live, 3633 MiB reserved (689 MiB device-local)"));
        assert!(text.contains("1065 buffers of 3000 MiB"));
        assert!(text.contains("3246 MiB in use of 3248 MiB"));
        assert!(text.lines().all(|line| line.starts_with("[Quadra Metal]")));

        let kept = describe("commit", &Picture::default(), false);
        assert!(kept.contains("everything after this fails"));
        assert!(!kept.contains("GTT"), "no counters, no claim about them");
    }

    #[test]
    fn the_exit_status_is_not_one_a_crash_or_a_shell_would_give() {
        assert!(EXIT_STATUS > 2 && EXIT_STATUS < 126 && EXIT_STATUS != 11);
    }
}
