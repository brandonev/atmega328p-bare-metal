//! Digital-input debouncing

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Tracks a noisy binary input until its value remains stable.
///
/// A transition is accepted after the same raw value has been observed for the
/// configured number of consecutive samples.
pub struct Debouncer {
    last_raw: AtomicBool,
    stable: AtomicBool,
    matching_samples: AtomicU8,
    required_samples: u8,
}

impl Debouncer {
    /// Creates a debouncer requiring `required_samples` consecutive matches.
    ///
    /// # Panics
    ///
    /// Panics if `required_samples` is zero.
    pub const fn new(required_samples: u8) -> Self {
        assert!(required_samples > 0);

        Self {
            last_raw: AtomicBool::new(false),
            stable: AtomicBool::new(false),
            matching_samples: AtomicU8::new(0),
            required_samples,
        }
    }

    /// Initializes the debouncer with the input's current state.
    ///
    /// Call this before processing samples so the initial state is not
    /// reported as a transition.
    pub fn initialize(&self, pressed: bool) {
        self.last_raw.store(pressed, Ordering::Relaxed);
        self.stable.store(pressed, Ordering::Relaxed);
        self.matching_samples
            .store(self.required_samples, Ordering::Relaxed);
    }

    /// Processes one raw sample.
    ///
    /// Returns `Some(state)` once when a new state becomes stable, or `None`
    /// when no stable transition has occurred.
    pub fn update(&self, raw_active: bool) -> Option<bool> {
        let last_raw = self.last_raw.load(Ordering::Relaxed);
        let mut count = self.matching_samples.load(Ordering::Relaxed);

        if raw_active != last_raw {
            self.last_raw.store(raw_active, Ordering::Relaxed);
            count = 1;
            self.matching_samples.store(count, Ordering::Relaxed);
        } else if count < self.required_samples {
            count += 1;
            self.matching_samples.store(count, Ordering::Relaxed);
        }

        if count >= self.required_samples {
            let stable = self.stable.load(Ordering::Relaxed);

            if raw_active != stable {
                self.stable.store(raw_active, Ordering::Relaxed);
                return Some(raw_active);
            }
        }

        None
    }
}
