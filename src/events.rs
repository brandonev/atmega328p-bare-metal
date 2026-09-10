//! Events produced by the device firmware.

/// Identifies a binary input within an application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryInputId(pub u8);

/// The state of a binary input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryState {
    /// The input is not active.
    Inactive,
    /// The input is active.
    Active,
}

/// A measurement produced by a sensor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    /// Identifies the sensor that produced the measurement.
    pub sensor: SensorId,
    /// Identifies the measured field.
    pub field: FieldId,
    /// Contains the measured value.
    pub value: i32,
}

/// Identifies a sensor within an application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensorId(pub u8);

/// Identifies a measurable field belonging to a sensor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldId(pub u8);

/// An event produced by the device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceEvent {
    /// A binary input changed to a new stable state.
    BinaryInputChanged {
        /// Identifies the input that changed.
        input: BinaryInputId,
        /// Contains the input's new stable state.
        state: BinaryState,
    },

    /// A sensor produced a measurement.
    Measurement(Measurement),
}

/// A device event paired with the device tick at which it occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimestampedEvent {
    /// The device tick at which the event occurred.
    pub ticks: u32,
    /// The event that occurred.
    pub event: DeviceEvent,
}
