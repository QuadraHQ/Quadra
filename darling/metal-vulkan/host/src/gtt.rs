//! The memory ceiling an integrated AMD GPU really has. Host-visible Metal buffers live in GTT, the
//! system RAM the GPU may map, and the amdgpu driver fails a command submission with "Not enough
//! memory for command submission" once that address space is full, which Vulkan reports as
//! `VK_ERROR_DEVICE_LOST` and which cannot be recovered. A reported budget is only advice to the
//! game, so this module reads the driver's own GTT counters from sysfs and refuses a host-visible
//! buffer that would take the last of it, while the game can still be told "no" without losing the
//! device.
//!
//! Only integrated AMD GPUs with readable counters are touched. Everything else (discrete cards,
//! NVIDIA, Intel, a container without sysfs) gets no check and behaves as it always did.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

const MIB: u64 = 1 << 20;
const AMD: u32 = 0x1002;
/// The smallest budget this module will report, as in `memory.rs`.
const MINIMUM: u64 = 64 * MIB;
/// A GTT smaller than this is not a real ceiling worth guarding (an unreadable or bogus counter).
const MINIMUM_GTT: u64 = 256 * MIB;

/// Buffers below this size are not checked: they cannot be what fills a heap, and the check reads
/// a file.
pub const CHECK_FROM: u64 = MIB;
/// How long a buffer that does not fit waits for in-flight work to give memory back before the
/// refusal is final.
const WAIT_LIMIT: Duration = Duration::from_millis(250);
const WAIT_STEP: Duration = Duration::from_millis(10);

/// One adapter's GTT counters under `/sys/class/drm/cardN/device`.
#[derive(Debug)]
pub struct Ceiling {
    total: u64,
    used: PathBuf,
}

fn read_number(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_id(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    u32::from_str_radix(text.trim().trim_start_matches("0x"), 16).ok()
}

fn is_card(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .and_then(|name| name.strip_prefix("card"))
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

impl Ceiling {
    /// The `drm` entry whose PCI ids are the Vulkan device's and which exposes `mem_info_gtt_*`.
    pub fn find(drm: &Path, vendor: u32, device: u32) -> Option<Ceiling> {
        let mut cards: Vec<_> = std::fs::read_dir(drm)
            .ok()?
            .flatten()
            .filter(|entry| is_card(&entry.file_name()))
            .collect();
        cards.sort_by_key(|entry| entry.file_name());
        cards.into_iter().find_map(|entry| {
            let directory = entry.path().join("device");
            if read_id(&directory.join("vendor"))? != vendor
                || read_id(&directory.join("device"))? != device
            {
                return None;
            }
            let total = read_number(&directory.join("mem_info_gtt_total"))?;
            (total >= MINIMUM_GTT).then(|| Ceiling {
                total,
                used: directory.join("mem_info_gtt_used"),
            })
        })
    }

    /// GTT in use by every process on the adapter, now.
    pub fn used(&self) -> Option<u64> {
        read_number(&self.used)
    }
}

/// GTT left alone for the compositor, the driver and other programs: a sixteenth, at least
/// 128 MiB, and never more than a quarter of a small GTT.
pub fn reserve(total: u64) -> u64 {
    (total / 16).max(128 * MIB).min(total / 4)
}

/// Whether `request` more bytes keep `used` out of the reserve.
pub fn fits(total: u64, used: u64, request: u64) -> bool {
    used.saturating_add(request) <= total.saturating_sub(reserve(total))
}

/// A recommended working set that follows what the GPU can still map, three quarters of the GTT
/// that is neither in use nor reserved.
pub fn share(total: u64, used: u64) -> u64 {
    (total.saturating_sub(used).saturating_sub(reserve(total)) / 4 * 3).max(MINIMUM)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Admission {
    Admitted { waited: Duration },
    Refused { used: u64, waited: Duration },
}

/// Lets `request` bytes in if they fit, waiting a little for GPU work in flight to release memory
/// first. An unreadable counter admits: with no evidence of a shortage there is nothing to refuse.
pub fn admit(
    total: u64,
    request: u64,
    mut used: impl FnMut() -> Option<u64>,
    mut pause: impl FnMut(Duration),
) -> Admission {
    let mut waited = Duration::ZERO;
    loop {
        let Some(now) = used() else {
            return Admission::Admitted { waited };
        };
        if fits(total, now, request) {
            return Admission::Admitted { waited };
        }
        if waited >= WAIT_LIMIT {
            return Admission::Refused { used: now, waited };
        }
        pause(WAIT_STEP);
        waited += WAIT_STEP;
    }
}

/// Metal's storage modes Shared (0) and Managed (1), in bits 4 to 7 of the resource options, are
/// the host-visible ones. Private and memoryless resources are the GPU's alone.
pub fn host_visible(options: u64) -> bool {
    matches!((options >> 4) & 0xf, 0 | 1)
}

struct State {
    ceiling: Ceiling,
    unified: bool,
}

static STATE: OnceLock<Option<State>> = OnceLock::new();

fn state() -> Option<&'static State> {
    STATE.get().and_then(Option::as_ref)
}

/// `MACOBLOX_METAL_GTT_GUARD=0` (or `off`) turns the refusal off and leaves the reporting on.
fn guard_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        !matches!(
            std::env::var("MACOBLOX_METAL_GTT_GUARD")
                .map(|value| value.trim().to_ascii_lowercase())
                .as_deref(),
            Ok("0" | "off" | "false" | "no")
        )
    })
}

/// Finds the selected device's counters once. The first device a process opens decides; a
/// process plays on one.
pub fn configure(vendor: u32, device: u32, unified: bool) {
    STATE.get_or_init(|| {
        if vendor != AMD {
            return None;
        }
        Ceiling::find(Path::new("/sys/class/drm"), vendor, device).map(|ceiling| {
            crate::util::log_once(&format!(
                "[Quadra Metal] GTT ceiling: {} MiB (checked before host-visible buffers: {})",
                ceiling.total / MIB,
                if unified && guard_enabled() {
                    "yes"
                } else {
                    "no"
                }
            ));
            State { ceiling, unified }
        })
    });
}

/// The adapter's GTT total and what is in use, where the driver reports them.
pub fn counters() -> Option<(u64, u64)> {
    let state = state()?;
    Some((state.ceiling.total, state.ceiling.used()?))
}

/// `bytes`, lowered to what an integrated GPU can still map.
pub fn limit_budget(bytes: u64) -> u64 {
    match state() {
        Some(State {
            ceiling,
            unified: true,
        }) => limited(bytes, ceiling),
        _ => bytes,
    }
}

fn limited(bytes: u64, ceiling: &Ceiling) -> u64 {
    ceiling
        .used()
        .map_or(bytes, |used| bytes.min(share(ceiling.total, used)))
}

/// Why a buffer was turned away, and how long it waited for memory first.
#[derive(Debug, PartialEq, Eq)]
pub struct Refusal {
    pub message: String,
    pub waited: Duration,
}

/// Admits a new buffer of `length` bytes with the resource `options`, or says why it cannot be.
/// The `Ok` is how long it waited for memory.
pub fn guard(length: u64, options: u64) -> Result<Duration, Refusal> {
    let Some(State {
        ceiling,
        unified: true,
    }) = state()
    else {
        return Ok(Duration::ZERO);
    };
    if !guard_enabled() {
        return Ok(Duration::ZERO);
    }
    guard_with(ceiling, length, options, std::thread::sleep)
}

fn guard_with(
    ceiling: &Ceiling,
    length: u64,
    options: u64,
    pause: impl FnMut(Duration),
) -> Result<Duration, Refusal> {
    if length < CHECK_FROM || !host_visible(options) {
        return Ok(Duration::ZERO);
    }
    match admit(ceiling.total, length, || ceiling.used(), pause) {
        Admission::Admitted { waited } => Ok(waited),
        Admission::Refused { used, waited } => Err(Refusal {
            message: format!(
                "graphics memory is exhausted: {} MiB of the GPU's {} MiB GTT is in use and {} \
                 MiB more was requested (waited {} ms)",
                used / MIB,
                ceiling.total / MIB,
                length.div_ceil(MIB),
                waited.as_millis()
            ),
            waited,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Renoir report: a 3248 MiB GTT.
    const TOTAL: u64 = 3248 * MIB;

    fn card(drm: &Path, name: &str, vendor: &str, device: &str, files: &[(&str, &str)]) {
        let directory = drm.join(name).join("device");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("vendor"), format!("{vendor}\n")).unwrap();
        std::fs::write(directory.join("device"), format!("{device}\n")).unwrap();
        for (file, text) in files {
            std::fs::write(directory.join(file), text).unwrap();
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("quadra-gtt-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn the_reserve_scales_with_the_gtt_and_never_swallows_a_small_one() {
        assert_eq!(reserve(TOTAL), 203 * MIB);
        assert_eq!(reserve(8192 * MIB), 512 * MIB);
        assert_eq!(reserve(1024 * MIB), 128 * MIB);
        assert_eq!(reserve(256 * MIB), 64 * MIB);
    }

    #[test]
    fn a_buffer_fits_until_it_would_reach_into_the_reserve() {
        // The reported crash: 3246 MiB of 3248 MiB in use.
        assert!(!fits(TOTAL, 3246 * MIB, 12 * MIB));
        assert!(fits(TOTAL, 1000 * MIB, 12 * MIB));
        let room = TOTAL - reserve(TOTAL);
        assert!(fits(TOTAL, room - 12 * MIB, 12 * MIB));
        assert!(!fits(TOTAL, room - 12 * MIB + 1, 12 * MIB));
        assert!(!fits(TOTAL, u64::MAX, 1));
        assert!(!fits(TOTAL, 1, u64::MAX));
    }

    #[test]
    fn the_working_set_follows_what_the_gpu_can_still_map() {
        assert_eq!(share(TOTAL, 0), (TOTAL - reserve(TOTAL)) / 4 * 3);
        assert!(share(TOTAL, 1000 * MIB) < share(TOTAL, 0));
        assert_eq!(share(TOTAL, TOTAL), MINIMUM);
        assert_eq!(share(TOTAL, u64::MAX), MINIMUM);
    }

    #[test]
    fn admission_waits_for_memory_to_come_back_and_then_refuses() {
        let mut reads = 0;
        let mut pauses = 0;
        let freed = admit(
            TOTAL,
            12 * MIB,
            || {
                reads += 1;
                Some(if reads < 4 { 3200 * MIB } else { 1000 * MIB })
            },
            |_| pauses += 1,
        );
        assert_eq!(
            freed,
            Admission::Admitted {
                waited: WAIT_STEP * 3
            }
        );
        assert_eq!(pauses, 3);

        let mut slept = Duration::ZERO;
        let full = admit(TOTAL, 12 * MIB, || Some(3246 * MIB), |step| slept += step);
        assert_eq!(
            full,
            Admission::Refused {
                used: 3246 * MIB,
                waited: WAIT_LIMIT
            }
        );
        assert_eq!(slept, WAIT_LIMIT, "the wait is bounded");
    }

    #[test]
    fn a_counter_that_cannot_be_read_admits_instead_of_refusing_on_no_evidence() {
        let mut pauses = 0;
        let result = admit(TOTAL, 64 * MIB, || None, |_| pauses += 1);
        assert_eq!(
            result,
            Admission::Admitted {
                waited: Duration::ZERO
            }
        );
        assert_eq!(pauses, 0);
    }

    #[test]
    fn only_host_visible_storage_is_checked() {
        assert!(host_visible(0x0)); // shared, default cache
        assert!(host_visible(0x1)); // shared, write combined
        assert!(host_visible(0x10)); // managed
        assert!(!host_visible(0x20)); // private
        assert!(!host_visible(0x30)); // memoryless
        assert!(
            host_visible(0x100_0000),
            "a hazard bit above the storage field changes nothing"
        );
    }

    #[test]
    fn the_adapter_is_found_by_its_ids_and_only_when_it_exposes_a_real_gtt() {
        let drm = scratch("find");
        card(
            &drm,
            "card0",
            "0x10de",
            "0x2504",
            &[("mem_info_gtt_total", "9999999999")],
        );
        card(
            &drm,
            "card1",
            "0x1002",
            "0x1636",
            &[
                ("mem_info_gtt_total", &(TOTAL.to_string() + "\n")),
                ("mem_info_gtt_used", "1048576\n"),
            ],
        );
        // Connector entries and render nodes are not cards.
        std::fs::create_dir_all(drm.join("card1-eDP-1/device")).unwrap();
        std::fs::create_dir_all(drm.join("renderD128/device")).unwrap();

        let found = Ceiling::find(&drm, AMD, 0x1636).unwrap();
        assert_eq!(found.total, TOTAL);
        assert_eq!(found.used(), Some(MIB));
        assert!(
            Ceiling::find(&drm, AMD, 0x1638).is_none(),
            "another device id"
        );
        assert!(
            Ceiling::find(&drm, 0x8086, 0x1636).is_none(),
            "another vendor"
        );
        assert!(Ceiling::find(Path::new("/nonexistent/drm"), AMD, 0x1636).is_none());

        card(
            &drm,
            "card2",
            "0x1002",
            "0x15e7",
            &[("mem_info_gtt_total", "1024")],
        );
        assert!(
            Ceiling::find(&drm, AMD, 0x15e7).is_none(),
            "an implausibly small GTT is not a ceiling"
        );
        card(&drm, "card3", "0x1002", "0x1681", &[]);
        assert!(Ceiling::find(&drm, AMD, 0x1681).is_none(), "no counters");
        let _ = std::fs::remove_dir_all(&drm);
    }

    #[test]
    fn a_used_counter_that_goes_missing_or_junk_reads_as_unknown() {
        let drm = scratch("used");
        card(
            &drm,
            "card0",
            "0x1002",
            "0x1636",
            &[("mem_info_gtt_total", &TOTAL.to_string())],
        );
        let ceiling = Ceiling::find(&drm, AMD, 0x1636).unwrap();
        assert_eq!(ceiling.used(), None);
        std::fs::write(drm.join("card0/device/mem_info_gtt_used"), "lots").unwrap();
        assert_eq!(ceiling.used(), None);
        let _ = std::fs::remove_dir_all(&drm);
    }

    /// A Renoir-sized adapter whose "in use" counter the test sets.
    fn adapter(name: &str, used: u64) -> (PathBuf, Ceiling) {
        let drm = scratch(name);
        card(
            &drm,
            "card0",
            "0x1002",
            "0x1636",
            &[
                ("mem_info_gtt_total", &TOTAL.to_string()),
                ("mem_info_gtt_used", &used.to_string()),
            ],
        );
        let ceiling = Ceiling::find(&drm, AMD, 0x1636).unwrap();
        (drm, ceiling)
    }

    #[test]
    fn the_reported_crash_is_refused_with_the_numbers_and_after_a_bounded_wait() {
        // Skelly's trace: 3246 MiB of the 3248 MiB GTT in use, a 12 MiB shared buffer requested.
        let (drm, ceiling) = adapter("crash", 3246 * MIB);
        let mut slept = Duration::ZERO;
        let refusal = guard_with(&ceiling, 12 * MIB + 1, 0x0, |step| slept += step).unwrap_err();
        assert_eq!(refusal.waited, WAIT_LIMIT);
        assert_eq!(slept, WAIT_LIMIT);
        assert_eq!(
            refusal.message,
            "graphics memory is exhausted: 3246 MiB of the GPU's 3248 MiB GTT is in use and 13 MiB \
             more was requested (waited 250 ms)"
        );
        let _ = std::fs::remove_dir_all(&drm);
    }

    #[test]
    fn memory_that_comes_back_while_waiting_lets_the_buffer_in() {
        let (drm, ceiling) = adapter("returns", 3200 * MIB);
        let used = drm.join("card0/device/mem_info_gtt_used");
        let mut pauses = 0;
        let waited = guard_with(&ceiling, 12 * MIB, 0x0, |_| {
            pauses += 1;
            if pauses == 5 {
                std::fs::write(&used, (1000 * MIB).to_string()).unwrap();
            }
        })
        .unwrap();
        assert_eq!(waited, WAIT_STEP * 5);
        let _ = std::fs::remove_dir_all(&drm);
    }

    #[test]
    fn small_and_private_allocations_are_never_checked_even_when_the_gtt_is_full() {
        let (drm, ceiling) = adapter("exempt", TOTAL);
        let never = |_| panic!("must not wait");
        assert_eq!(
            guard_with(&ceiling, CHECK_FROM - 1, 0x0, never),
            Ok(Duration::ZERO)
        );
        assert_eq!(
            guard_with(&ceiling, 256 * MIB, 0x20, never),
            Ok(Duration::ZERO),
            "private"
        );
        assert_eq!(
            guard_with(&ceiling, 256 * MIB, 0x30, never),
            Ok(Duration::ZERO),
            "memoryless"
        );
        assert!(
            guard_with(&ceiling, CHECK_FROM, 0x10, |_| {}).is_err(),
            "managed is host-visible"
        );
        let _ = std::fs::remove_dir_all(&drm);
    }

    #[test]
    fn the_budget_shrinks_with_the_gtt_in_use_and_never_grows() {
        let (drm, ceiling) = adapter("budget", 0);
        let used = drm.join("card0/device/mem_info_gtt_used");
        // An empty GTT does not raise a budget the launcher already lowered.
        assert_eq!(limited(135 * MIB, &ceiling), 135 * MIB);
        assert_eq!(limited(u64::MAX, &ceiling), share(TOTAL, 0));
        std::fs::write(&used, (3000 * MIB).to_string()).unwrap();
        assert_eq!(
            limited(512 * MIB, &ceiling),
            MINIMUM,
            "nearly full: only the floor is left"
        );
        std::fs::remove_file(&used).unwrap();
        assert_eq!(
            limited(512 * MIB, &ceiling),
            512 * MIB,
            "an unreadable counter changes nothing"
        );
        let _ = std::fs::remove_dir_all(&drm);
    }

    #[test]
    fn without_a_configured_amd_adapter_nothing_is_checked_or_limited() {
        // No test configures the process-wide state, so these are the paths of every other GPU.
        assert_eq!(guard(512 * MIB, 0), Ok(Duration::ZERO));
        assert_eq!(limit_budget(777 * MIB), 777 * MIB);
        assert_eq!(counters(), None);
    }
}
