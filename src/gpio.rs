//! GPIO abstractions for the ATmega328P.

use core::marker::PhantomData;
use core::ptr::{read_volatile, write_volatile};

/// A general-purpose I/O port on the ATmega328P.
#[derive(Clone, Copy)]
pub enum Port {
    /// GPIO port B.
    B,

    /// GPIO port C.
    C,

    /// GPIO port D.
    D,
}

impl Port {
    const fn pin_register(self) -> *mut u8 {
        match self {
            Port::B => 0x23 as *mut u8,
            Port::C => 0x26 as *mut u8,
            Port::D => 0x29 as *mut u8,
        }
    }

    const fn direction_register(self) -> *mut u8 {
        match self {
            Port::B => 0x24 as *mut u8,
            Port::C => 0x27 as *mut u8,
            Port::D => 0x2A as *mut u8,
        }
    }

    const fn output_register(self) -> *mut u8 {
        match self {
            Port::B => 0x25 as *mut u8,
            Port::C => 0x28 as *mut u8,
            Port::D => 0x2B as *mut u8,
        }
    }
}

/// Identifies a GPIO pin by its hardware port and bit position.
#[derive(Clone, Copy)]
pub struct PinId {
    port: Port,
    bit: u8,
}

impl PinId {
    /// Creates a pin identifier for `bit` within `port`.
    ///
    /// # Panics
    ///
    /// Panics if `bit` is greater than seven.
    pub const fn new(port: Port, bit: u8) -> Self {
        assert!(bit < 8);
        Self { port, bit }
    }

    const fn mask(self) -> u8 {
        1 << self.bit
    }
}

/// Hardware specific mappings to pins
pub mod board {
    use super::{PinId, Port};

    /// Mapped pin for on board D0
    pub const D0: PinId = PinId::new(Port::D, 0);

    /// Mapped pin for on board D1
    pub const D1: PinId = PinId::new(Port::D, 1);

    /// Mapped pin for on board D2
    pub const D2: PinId = PinId::new(Port::D, 2);

    /// Mapped pin for on board D3
    pub const D3: PinId = PinId::new(Port::D, 3);

    /// Mapped pin for on board D4
    pub const D4: PinId = PinId::new(Port::D, 4);

    /// Mapped pin for on board D5
    pub const D5: PinId = PinId::new(Port::D, 5);

    /// Mapped pin for on board D6
    pub const D6: PinId = PinId::new(Port::D, 6);

    /// Mapped pin for on board D7
    pub const D7: PinId = PinId::new(Port::D, 7);

    /// Mapped pin for on board B0
    pub const D8: PinId = PinId::new(Port::B, 0);

    /// Mapped pin for on board B1
    pub const D9: PinId = PinId::new(Port::B, 1);

    /// Mapped pin for on board B2
    pub const D10: PinId = PinId::new(Port::B, 2);

    /// Mapped pin for on board B3
    pub const D11: PinId = PinId::new(Port::B, 3);

    /// Mapped pin for on board B4
    pub const D12: PinId = PinId::new(Port::B, 4);

    /// Mapped pin for on board B5
    pub const D13: PinId = PinId::new(Port::B, 5);

    /// Mapped pin for on board C0
    pub const A0: PinId = PinId::new(Port::C, 0);

    /// Mapped pin for on board C1
    pub const A1: PinId = PinId::new(Port::C, 1);

    /// Mapped pin for on board C2
    pub const A2: PinId = PinId::new(Port::C, 2);

    /// Mapped pin for on board C3
    pub const A3: PinId = PinId::new(Port::C, 3);

    /// Mapped pin for on board C4
    pub const A4: PinId = PinId::new(Port::C, 4);

    /// Mapped pin for on board C5
    pub const A5: PinId = PinId::new(Port::C, 5);
}

/// The electrical pull configuration for an input pin.
pub enum Pull {
    /// Leaves the input floating.
    Floating,

    /// Enables the internal pull-up resistor.
    Up,
}

/// Marks a pin as an output.
pub struct Output;

/// Marks a pin as an input.
pub struct Input;

/// A GPIO pin configured with mode `Mode`.
pub struct Pin<Mode> {
    id: PinId,
    _mode: PhantomData<Mode>,
}

impl Pin<Output> {
    /// Creates an output-pin handle.
    ///
    /// Call [`Self::configure`] before driving the pin.
    pub const fn new(id: PinId) -> Self {
        Self {
            id,
            _mode: PhantomData,
        }
    }

    /// Configures the pin as an output.
    pub fn configure(&self) {
        unsafe {
            let register = self.id.port.direction_register();
            let value = read_volatile(register);

            write_volatile(register, value | self.id.mask());
        }
    }

    /// Drives the output pin high.
    pub fn set_high(&self) {
        unsafe {
            let register = self.id.port.output_register();
            let value = read_volatile(register);

            write_volatile(register, value | self.id.mask());
        }
    }

    /// Drives the output pin low.
    pub fn set_low(&self) {
        unsafe {
            let register = self.id.port.output_register();
            let value = read_volatile(register);

            write_volatile(register, value & !self.id.mask());
        }
    }

    /// Drives the pin high when `high` is `true`, or low otherwise.
    pub fn set(&self, high: bool) {
        if high {
            self.set_high();
        } else {
            self.set_low();
        }
    }
}

impl Pin<Input> {
    /// Creates an input-pin handle.
    ///
    /// Call [`Self::configure`] before reading the pin.
    pub const fn new(id: PinId) -> Self {
        Self {
            id,
            _mode: PhantomData,
        }
    }

    /// Configures the pin as an input with the selected pull configuration.
    pub fn configure(&self, pull: Pull) {
        unsafe {
            let mask = self.id.mask();
            let direction = self.id.port.direction_register();
            let output = self.id.port.output_register();

            // Clear DDRx bit to configure the pin as input.
            let directions = read_volatile(direction);
            write_volatile(direction, directions & !mask);

            // For an input, the PORTx bit controls its pull-up resistor.
            let outputs = read_volatile(output);

            match pull {
                Pull::Floating => {
                    write_volatile(output, outputs & !mask);
                }
                Pull::Up => {
                    write_volatile(output, outputs | mask);
                }
            }
        }
    }

    /// Returns `true` when the pin's electrical level is low.
    pub fn is_low(&self) -> bool {
        unsafe {
            let register = self.id.port.pin_register();
            read_volatile(register) & self.id.mask() == 0
        }
    }

    /// Returns `true` when the pin's electrical level is high.
    pub fn is_high(&self) -> bool {
        !self.is_low()
    }
}
