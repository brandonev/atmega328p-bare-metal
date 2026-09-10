//! Analog-to-digital conversion for the ATmega328p

use core::ptr::{read_volatile, write_volatile};

const ADCL: *mut u8 = 0x78 as *mut u8;
const ADCH: *mut u8 = 0x79 as *mut u8;
const ADCSRA: *mut u8 = 0x7A as *mut u8;
const ADMUX: *mut u8 = 0x7C as *mut u8;

const REFS0: u8 = 1 << 6;
const ADEN: u8 = 1 << 7;
const ADSC: u8 = 1 << 6;

const ADPS2: u8 = 1 << 2;
const ADPS1: u8 = 1 << 1;
const ADPS0: u8 = 1 << 0;

/// An analog input channel on the ATmega328P.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Channel {
    /// Analog channel ADC0, exposed as A0 on the Uno.
    Adc0 = 0,

    /// Analog channel ADC1, exposed as A1 on the Uno.
    Adc1 = 1,

    /// Analog channel ADC2, exposed as A2 on the Uno.
    Adc2 = 2,

    /// Analog channel ADC3, exposed as A3 on the Uno.
    Adc3 = 3,

    /// Analog channel ADC4, exposed as A4 on the Uno.
    Adc4 = 4,

    /// Analog channel ADC5, exposed as A5 on the Uno.
    Adc5 = 5,
}

/// The ATmega328P analog-to-digital converter.
pub struct Adc;

impl Default for Adc {
    fn default() -> Self {
        Self::new()
    }
}

impl Adc {
    /// Creates an ADC peripheral handle.
    pub const fn new() -> Self {
        Self
    }

    /// Enables the ADC using AVcc as the voltage reference.
    pub fn configure(&self) {
        unsafe {
            write_volatile(ADMUX, REFS0);
            write_volatile(ADCSRA, ADEN | ADPS2 | ADPS1 | ADPS0);
        }
    }

    /// Performs one blocking 10-bit conversion.
    pub fn read(&self, channel: Channel) -> u16 {
        unsafe {
            let admux = read_volatile(ADMUX);

            // Keep the reference selection and replace the channel bits.
            write_volatile(ADMUX, (admux & 0b1111_0000) | channel as u8);

            let control = read_volatile(ADCSRA);
            write_volatile(ADCSRA, control | ADSC);

            while read_volatile(ADCSRA) & ADSC != 0 {}

            // Reading ADCL first locks both result registers.
            let low = read_volatile(ADCL) as u16;
            let high = read_volatile(ADCH) as u16;

            (high << 8) | low
        }
    }
}
