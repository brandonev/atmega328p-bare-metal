#![no_std]
#![warn(missing_docs)]

//! Bare-metal support for the ATmega328P used by the Arduino Uno R3.
//!
//! The crate provides GPIO, ADC, Timer1, USART0, interrupt control,
//! debouncing, event queues, and shared device-state primitives.

pub mod adc;
pub mod debounce;
pub mod event_queue;
pub mod events;
pub mod gpio;
pub mod interrupts;
pub mod serial;
pub mod state;
pub mod timer;
