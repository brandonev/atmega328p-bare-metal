//! Storage for the device's latest known input and measurement states.

use core::cell::UnsafeCell;

use crate::events::{BinaryInputId, BinaryState, Measurement};
use crate::interrupts::with_disabled;

/// Interrupt-safe shared access to a [`DeviceState`].
///
/// Access is serialized using short critical sections.
pub struct SharedDeviceState<const INPUTS: usize, const MEASUREMENTS: usize> {
    inner: UnsafeCell<DeviceState<INPUTS, MEASUREMENTS>>,
}

impl<const INPUTS: usize, const MEASUREMENTS: usize> Default
    for SharedDeviceState<INPUTS, MEASUREMENTS>
{
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<const INPUTS: usize, const MEASUREMENTS: usize> Sync
    for SharedDeviceState<INPUTS, MEASUREMENTS>
{
}

impl<const INPUTS: usize, const MEASUREMENTS: usize> SharedDeviceState<INPUTS, MEASUREMENTS> {
    /// Creates an empty shared device state.
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(DeviceState::new()),
        }
    }

    /// Inserts or updates a binary input state.
    ///
    /// Returns `Ok(true)` if the state changed, `Ok(false)` if it was unchanged,
    /// or `Err(state)` if no storage slot is available.
    pub fn update_binary_input(&self, state: BinaryInputState) -> Result<bool, BinaryInputState> {
        with_disabled(|| {
            let device = unsafe { &mut *self.inner.get() };
            device.update_binary_input(state)
        })
    }

    /// Inserts or updates a timestamped measurement.
    ///
    /// Returns `Ok(true)` if the value changed, `Ok(false)` if it was unchanged,
    /// or `Err(measurement)` if no storage slot is available.
    pub fn update_measurement(
        &self,
        measurement: TimestampedMeasurement,
    ) -> Result<bool, TimestampedMeasurement> {
        with_disabled(|| {
            let device = unsafe { &mut *self.inner.get() };
            device.update_measurement(measurement)
        })
    }

    /// Returns one changed measurement and marks it as clean.
    ///
    /// Returns `None` when no measurements are dirty.
    pub fn take_dirty_measurement(&self) -> Option<TimestampedMeasurement> {
        let device = unsafe { &mut *self.inner.get() };
        device.take_dirty_measurement()
    }
}

/// Stores the latest known binary-input and measurement values.
///
/// Measurements are coalesced by sensor and field. Changed measurements remain
/// dirty until retrieved with [`Self::take_dirty_measurement`].
pub struct DeviceState<const INPUTS: usize, const MEASUREMENTS: usize> {
    binary_inputs: [Option<BinaryInputState>; INPUTS],
    measurements: [Option<TimestampedMeasurement>; MEASUREMENTS],

    dirty_inputs: [bool; INPUTS],
    dirty_measurements: [bool; MEASUREMENTS],
}

impl<const INPUTS: usize, const MEASUREMENTS: usize> Default for DeviceState<INPUTS, MEASUREMENTS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const INPUTS: usize, const MEASUREMENTS: usize> DeviceState<INPUTS, MEASUREMENTS> {
    /// Creates an empty shared device state.
    pub const fn new() -> Self {
        Self {
            binary_inputs: [None; INPUTS],
            measurements: [None; MEASUREMENTS],
            dirty_inputs: [false; INPUTS],
            dirty_measurements: [false; MEASUREMENTS],
        }
    }

    /// Inserts or updates a binary input state.
    ///
    /// Returns `Ok(true)` if the state changed, `Ok(false)` if it was unchanged,
    /// or `Err(state)` if no storage slot is available.
    pub fn update_binary_input(
        &mut self,
        new_state: BinaryInputState,
    ) -> Result<bool, BinaryInputState> {
        let mut empty_slot = None;

        for index in 0..INPUTS {
            match self.binary_inputs[index] {
                Some(current) if current.input == new_state.input => {
                    if current.state == new_state.state {
                        return Ok(false);
                    }

                    self.binary_inputs[index] = Some(new_state);
                    self.dirty_inputs[index] = true;
                    return Ok(true);
                }

                None if empty_slot.is_none() => {
                    empty_slot = Some(index);
                }

                _ => {}
            }
        }

        let Some(index) = empty_slot else {
            return Err(new_state);
        };

        self.binary_inputs[index] = Some(new_state);
        self.dirty_inputs[index] = true;

        Ok(true)
    }

    /// Inserts or updates a timestamped measurement.
    ///
    /// Returns `Ok(true)` if the value changed, `Ok(false)` if it was unchanged,
    /// or `Err(measurement)` if no storage slot is available.
    pub fn update_measurement(
        &mut self,
        new_measurement: TimestampedMeasurement,
    ) -> Result<bool, TimestampedMeasurement> {
        let mut empty_slot = None;

        for index in 0..MEASUREMENTS {
            match self.measurements[index] {
                Some(current)
                    if current.measurement.sensor == new_measurement.measurement.sensor
                        && current.measurement.field == new_measurement.measurement.field =>
                {
                    if current.measurement.value == new_measurement.measurement.value {
                        return Ok(false);
                    }

                    self.measurements[index] = Some(new_measurement);
                    self.dirty_measurements[index] = true;
                    return Ok(true);
                }

                None if empty_slot.is_none() => {
                    empty_slot = Some(index);
                }

                _ => {}
            }
        }

        let Some(index) = empty_slot else {
            return Err(new_measurement);
        };

        self.measurements[index] = Some(new_measurement);
        self.dirty_measurements[index] = true;

        Ok(true)
    }

    /// Returns one changed measurement and marks it as clean.
    ///
    /// Returns `None` when no measurements are dirty.
    pub fn take_dirty_measurement(&mut self) -> Option<TimestampedMeasurement> {
        for index in 0..MEASUREMENTS {
            if self.dirty_measurements[index] {
                self.dirty_measurements[index] = false;
                return self.measurements[index];
            }
        }

        None
    }
}

/// A sensor measurement paired with the device tick when it was captured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimestampedMeasurement {
    /// The device tick when the measurement was captured.
    pub ticks: u32,

    /// The captured sensor measurement.
    pub measurement: Measurement,
}

/// The latest known state of a binary input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryInputState {
    /// Identifies the binary input.
    pub input: BinaryInputId,

    /// The input's latest stable state.
    pub state: BinaryState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{BinaryInputId, BinaryState, FieldId, SensorId};

    #[test]
    fn new_measurement_becomes_dirty() {
        let mut state = DeviceState::<2, 2>::new();

        let measurement = TimestampedMeasurement {
            ticks: 10,
            measurement: Measurement {
                sensor: SensorId(1),
                field: FieldId(0),
                value: 500,
            },
        };

        assert_eq!(state.update_measurement(measurement), Ok(true));

        assert_eq!(state.take_dirty_measurement(), Some(measurement));

        assert_eq!(state.take_dirty_measurement(), None);
    }

    #[test]
    fn repeated_updates_are_coalesced() {
        let mut state = DeviceState::<2, 2>::new();

        state
            .update_measurement(TimestampedMeasurement {
                ticks: 10,
                measurement: Measurement {
                    sensor: SensorId(1),
                    field: FieldId(0),
                    value: 500,
                },
            })
            .unwrap();

        state
            .update_measurement(TimestampedMeasurement {
                ticks: 12,
                measurement: Measurement {
                    sensor: SensorId(1),
                    field: FieldId(0),
                    value: 600,
                },
            })
            .unwrap();

        assert_eq!(
            state.take_dirty_measurement(),
            Some(TimestampedMeasurement {
                ticks: 12,
                measurement: Measurement {
                    sensor: SensorId(1),
                    field: FieldId(0),
                    value: 600,
                }
            })
        );

        assert_eq!(state.take_dirty_measurement(), None);
    }

    #[test]
    fn identical_measurement_is_not_marked_dirty_again() {
        let mut state = DeviceState::<2, 2>::new();

        let measurement = TimestampedMeasurement {
            ticks: 10,
            measurement: Measurement {
                sensor: SensorId(1),
                field: FieldId(0),
                value: 500,
            },
        };

        assert_eq!(state.update_measurement(measurement), Ok(true));

        state.take_dirty_measurement();

        assert_eq!(state.update_measurement(measurement), Ok(false));

        assert_eq!(state.take_dirty_measurement(), None);
    }

    #[test]
    fn reports_measurement_capacity_exhaustion() {
        let mut state = DeviceState::<1, 1>::new();

        state
            .update_measurement(TimestampedMeasurement {
                ticks: 10,
                measurement: Measurement {
                    sensor: SensorId(1),
                    field: FieldId(0),
                    value: 500,
                },
            })
            .unwrap();

        let second = TimestampedMeasurement {
            ticks: 11,
            measurement: Measurement {
                sensor: SensorId(2),
                field: FieldId(0),
                value: 700,
            },
        };

        assert_eq!(state.update_measurement(second), Err(second));
    }

    #[test]
    fn tracks_binary_input_state_changes() {
        let mut state = DeviceState::<1, 1>::new();

        let active = BinaryInputState {
            input: BinaryInputId(1),
            state: BinaryState::Active,
        };

        assert_eq!(state.update_binary_input(active), Ok(true));

        assert_eq!(state.update_binary_input(active), Ok(false));

        let inactive = BinaryInputState {
            input: BinaryInputId(1),
            state: BinaryState::Inactive,
        };

        assert_eq!(state.update_binary_input(inactive), Ok(true));
    }
}
