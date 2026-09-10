//! Timer abstractions for the ATmega328p

use core::ptr::{read_volatile, write_volatile};

const TCCR1A: *mut u8 = 0x80 as *mut u8;
const TCCR1B: *mut u8 = 0x81 as *mut u8;
const OCR1AL: *mut u8 = 0x88 as *mut u8;
const OCR1AH: *mut u8 = 0x89 as *mut u8;
const TIFR1: *mut u8 = 0x36 as *mut u8;
const TIMSK1: *mut u8 = 0x6F as *mut u8;

const WGM12: u8 = 1 << 3;
const OCF1A: u8 = 1 << 1;
const OCIE1A: u8 = 1 << 1;

/// Timer clock prescaler
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Prescaler {
    /// Runs the timer at the full CPU clock rate.
    Div1 = 0b001,

    /// Divides the CPU clock by 8.
    Div8 = 0b010,

    /// Divides the CPU clock by 64.
    Div64 = 0b011,

    /// Divides the CPU clock by 256.
    Div256 = 0b100,

    /// Divides the CPU clock by 1024.
    Div1024 = 0b101,
}

impl Prescaler {
    const fn bits(self) -> u8 {
        self as u8
    }
}

/// The ATmega328p 16-bit Timer1 peripheral
pub struct Timer1;

impl Default for Timer1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer1 {
    /// Creates a Timer1 peripheral handle.
    pub const fn new() -> Self {
        Self
    }

    /// Configures Timer1 for CTC mode without starting it
    pub fn configure_ctc(&self, compare: u16) {
        unsafe {
            write_volatile(TCCR1A, 0);
            write_volatile(TCCR1B, 0);

            write_volatile(OCR1AH, (compare >> 8) as u8);
            write_volatile(OCR1AL, compare as u8);

            write_volatile(TIFR1, OCF1A);
        }
    }

    /// Enables the Timer1 Compare A interrupt
    pub fn enable_compare_a_interrupt(&self) {
        unsafe {
            let mask = read_volatile(TIMSK1);
            write_volatile(TIMSK1, mask | OCIE1A);
        }
    }

    /// Starts Timer1 in CTC mode
    pub fn start(&self, prescaler: Prescaler) {
        unsafe {
            write_volatile(TCCR1B, WGM12 | prescaler.bits());
        }
    }
}
