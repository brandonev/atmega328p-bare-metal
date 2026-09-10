//! USART0 serial output for the ATmega328P.

use core::ptr::{read_volatile, write_volatile};

const CPU_HZ: u32 = 16_000_000;

const UCSR0A: *mut u8 = 0xC0 as *mut u8;
const UCSR0B: *mut u8 = 0xC1 as *mut u8;
const UCSR0C: *mut u8 = 0xC2 as *mut u8;

const UBRR0L: *mut u8 = 0xC4 as *mut u8;
const UBRR0H: *mut u8 = 0xC5 as *mut u8;

const UDR0: *mut u8 = 0xC6 as *mut u8;

const UDRE0: u8 = 1 << 5;
const TXEN0: u8 = 1 << 3;
const UCSZ01: u8 = 1 << 2;
const UCSZ00: u8 = 1 << 1;

const RXC0: u8 = 1 << 7;
const RXEN0: u8 = 1 << 4;

/// The ATmega328P USART0 peripheral.
pub struct Usart0;

impl Default for Usart0 {
    fn default() -> Self {
        Self::new()
    }
}

unsafe extern "C" {
    fn read_program_byte(address: u16) -> u8;
}

impl Usart0 {
    /// Creates a USART0 peripheral handle.
    pub const fn new() -> Self {
        Self
    }

    /// Configures USART0 for the requested baud rate using 8N1 framing.
    pub fn configure(&self, baud: u32) {
        assert!(baud > 0);

        let divisor = CPU_HZ / (16 * baud) - 1;
        assert!(divisor <= 0x0FFF);

        unsafe {
            // Normal speed USART operation
            write_volatile(UCSR0A, 0);

            write_volatile(UBRR0H, (divisor >> 8) as u8);
            write_volatile(UBRR0L, divisor as u8);

            // Enable transmitter
            write_volatile(UCSR0B, RXEN0 | TXEN0);

            // Enable data bits, no parity, one stop bit
            write_volatile(UCSR0C, UCSZ01 | UCSZ00);
        }
    }

    /// Blocks until the transmit register is ready, then sends one byte.
    pub fn write_byte(&self, byte: u8) {
        unsafe {
            while read_volatile(UCSR0A) & UDRE0 == 0 {}

            write_volatile(UDR0, byte);
        }
    }

    /// Sends every byte in a slice.
    pub fn write_bytes(&self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_byte(byte);
        }
    }

    /// Writes a string stored in AVR program memory
    pub fn write_program_str(&self, text: &'static str) {
        let start = text.as_ptr() as u16;
        let length = text.len() as u16;

        for offset in 0..length {
            let address = start.wrapping_add(offset);

            let byte = unsafe { read_program_byte(address) };

            self.write_byte(byte);
        }
    }

    /// Writes a `u32` as decimal ASCII digits.
    pub fn write_u32(&self, mut value: u32) {
        if value == 0 {
            self.write_byte(b'0');
            return;
        }

        let mut digits = [0u8; 10];
        let mut index = digits.len();

        while value > 0 {
            index -= 1;
            digits[index] = b'0' + (value % 10) as u8;
            value /= 10;
        }

        self.write_bytes(&digits[index..]);
    }

    /// Writes a `i32` as decimal ASCII digits
    pub fn write_i32(&self, value: i32) {
        if value == 0 {
            self.write_byte(b'0');
            return;
        }

        if value < 0 {
            self.write_byte(b'-');
            self.write_u32(value.unsigned_abs());
        } else {
            self.write_u32(value as u32);
        }
    }

    /// Returns the next received byte, or None when no byte is available
    pub fn try_read_byte(&self) -> Option<u8> {
        unsafe {
            if read_volatile(UCSR0A) & RXC0 == 0 {
                return None;
            }

            Some(read_volatile(UDR0))
        }
    }

    /// Waits until a byte is received.
    pub fn read_byte(&self) -> u8 {
        loop {
            if let Some(byte) = self.try_read_byte() {
                return byte;
            }
        }
    }
}
