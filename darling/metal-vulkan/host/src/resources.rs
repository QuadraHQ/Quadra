//! Counts of the buffers and textures the game has made and let go, for the performance trace.
//!
//! The runtime reports how many bytes are live, but not whether the game is still holding them or
//! the runtime is keeping them until the GPU finishes with them. The difference between what the
//! game holds (made minus released here) and the runtime's live bytes says which, so a report of
//! memory growing without bound shows where it is held. Plain relaxed counters: they are read once
//! a second and never gate anything.
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

static BUFFERS_MADE: AtomicU64 = AtomicU64::new(0);
static BUFFERS_RELEASED: AtomicU64 = AtomicU64::new(0);
static BUFFER_BYTES_MADE: AtomicU64 = AtomicU64::new(0);
static BUFFER_BYTES_RELEASED: AtomicU64 = AtomicU64::new(0);
static TEXTURES_MADE: AtomicU64 = AtomicU64::new(0);
static TEXTURES_RELEASED: AtomicU64 = AtomicU64::new(0);
static BUFFERS_FAILED: AtomicU64 = AtomicU64::new(0);
static BUFFERS_REFUSED: AtomicU64 = AtomicU64::new(0);
static BUFFER_BYTES_REFUSED: AtomicU64 = AtomicU64::new(0);
static BUFFERS_DELAYED: AtomicU64 = AtomicU64::new(0);
static DELAY_NANOS: AtomicU64 = AtomicU64::new(0);

pub fn buffer_made(bytes: u64) {
    BUFFERS_MADE.fetch_add(1, Relaxed);
    BUFFER_BYTES_MADE.fetch_add(bytes, Relaxed);
}

pub fn buffer_released(bytes: u64) {
    BUFFERS_RELEASED.fetch_add(1, Relaxed);
    BUFFER_BYTES_RELEASED.fetch_add(bytes, Relaxed);
}

pub fn texture_made() {
    TEXTURES_MADE.fetch_add(1, Relaxed);
}

pub fn texture_released() {
    TEXTURES_RELEASED.fetch_add(1, Relaxed);
}

/// A buffer the runtime could not make for a reason other than the GTT check.
pub fn buffer_failed() {
    BUFFERS_FAILED.fetch_add(1, Relaxed);
}

/// A buffer the GTT check turned away, after waiting for memory to come back.
pub fn buffer_refused(bytes: u64, waited: std::time::Duration) {
    BUFFERS_REFUSED.fetch_add(1, Relaxed);
    BUFFER_BYTES_REFUSED.fetch_add(bytes, Relaxed);
    delayed(waited);
}

/// A buffer that waited for memory and was then let in (or turned away: see above).
pub fn delayed(waited: std::time::Duration) {
    if !waited.is_zero() {
        BUFFERS_DELAYED.fetch_add(1, Relaxed);
        DELAY_NANOS.fetch_add(waited.as_nanos().min(u64::MAX as u128) as u64, Relaxed);
    }
}

pub fn snapshot() -> Value {
    let made = BUFFERS_MADE.load(Relaxed);
    let released = BUFFERS_RELEASED.load(Relaxed);
    let bytes_made = BUFFER_BYTES_MADE.load(Relaxed);
    let bytes_released = BUFFER_BYTES_RELEASED.load(Relaxed);
    let textures_made = TEXTURES_MADE.load(Relaxed);
    let textures_released = TEXTURES_RELEASED.load(Relaxed);
    json!({
        "buffers_made": made,
        "buffers_released": released,
        "buffers_held": made.saturating_sub(released),
        "buffer_bytes_made": bytes_made,
        "buffer_bytes_released": bytes_released,
        "buffer_bytes_held": bytes_made.saturating_sub(bytes_released),
        "textures_made": textures_made,
        "textures_released": textures_released,
        "textures_held": textures_made.saturating_sub(textures_released),
        "buffers_failed": BUFFERS_FAILED.load(Relaxed),
        "buffers_refused": BUFFERS_REFUSED.load(Relaxed),
        "buffer_bytes_refused": BUFFER_BYTES_REFUSED.load(Relaxed),
        "buffers_delayed": BUFFERS_DELAYED.load(Relaxed),
        "delay_ns": DELAY_NANOS.load(Relaxed),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // The counters are process-wide and tests run in parallel, so this one test owns them all and
    // compares differences, not absolute values.
    #[test]
    fn what_the_game_holds_is_what_it_made_less_what_it_released() {
        let field = |name: &str| snapshot()[name].as_u64().unwrap();
        let before = (
            field("buffers_held"),
            field("buffer_bytes_held"),
            field("textures_held"),
            field("buffers_refused"),
            field("buffers_delayed"),
        );
        buffer_made(12 << 20);
        buffer_made(12 << 20);
        buffer_released(12 << 20);
        texture_made();
        buffer_refused(12 << 20, Duration::from_millis(250));
        delayed(Duration::ZERO);
        delayed(Duration::from_millis(30));
        assert_eq!(field("buffers_held"), before.0 + 1);
        assert_eq!(field("buffer_bytes_held"), before.1 + (12 << 20));
        assert_eq!(field("textures_held"), before.2 + 1);
        assert_eq!(field("buffers_refused"), before.3 + 1);
        assert_eq!(
            field("buffers_delayed"),
            before.4 + 2,
            "a refusal and a late admission, not the instant one"
        );
        assert!(field("delay_ns") >= 280_000_000);
        // A release the counters never saw made cannot take the held count below zero.
        for _ in 0..3 {
            texture_released();
        }
        assert!(snapshot()["textures_held"].as_u64().is_some());
    }
}
