# uno-systems-lab

A small bare-metal Rust support crate for the ATmega328P used by the Arduino Uno R3.

This project avoids the Arduino framework and implements startup, memory initialization, peripheral access, interrupts, and synchronization directly.

## Current support

- GPIO ports B, C, and D
- Arduino Uno pin mappings: D0–D13 and A0–A5
- ADC channels A0–A5
- Timer1 CTC mode
- Global interrupt control and interrupt-safe critical sections
- USART0 transmit and receive
- Digital-input debouncing
- Fixed-capacity event queues
- Coalesced device-state storage

## Pin mapping

| Uno pin | ATmega328P port |
|---|---|
| D0–D7 | PD0–PD7 |
| D8–D13 | PB0–PB5 |
| A0–A5 | PC0–PC5 |

D0 and D1 are used by USART0 for receive and transmit.

A0–A5 may be used either as digital GPIO pins or as ADC channels.

## Requirements

- Rust nightly
- Rust source component
- AVR GCC toolchain
- `avrdude`
- Arduino Uno R3 or compatible ATmega328P board

Install the Rust source component:

```bash
rustup component add rust-src --toolchain nightly
```

## Build

Assemble the startup code:

```bash
avr-gcc \
  -mmcu=atmega328p \
  -c startup.S \
  -o startup.o
```

Build the firmware:

```bash
RUSTFLAGS="-C target-cpu=atmega328p" \
cargo +nightly rustc \
  -Z build-std=core \
  --target avr-none \
  --release -- \
  -C linker=avr-gcc \
  -C link-arg=-mmcu=atmega328p \
  -C link-arg=-nostartfiles \
  -C link-arg=startup.o \
  -C link-arg=-Wl,-Tlinker.ld
```

Create an Intel HEX image:

```bash
avr-objcopy \
  -O ihex \
  -R .eeprom \
  target/avr-none/release/uno-systems-lab.elf \
  uno-systems-lab.hex
```

## Flash

Find the serial port on macOS:

```bash
ls /dev/cu.usbmodem*
```

Flash the board, replacing the port when necessary:

```bash
avrdude \
  -p atmega328p \
  -c arduino \
  -P /dev/cu.usbmodem14401 \
  -b 115200 \
  -D \
  -U flash:w:uno-systems-lab.hex:i
```

## Tests

Run host-compatible logic tests:

```bash
cargo test --lib
```

Peripheral register access is verified on physical hardware rather than through host unit tests.

## Safety and concurrency

Shared foreground and interrupt state uses short critical sections that temporarily disable interrupts and then restore the previous interrupt state.

Do not perform serial transmission, ADC conversion, delays, or other blocking operations inside a critical section.

GPIO pins must not directly power motors or other high-current devices. Use an appropriate driver board. LEDs require a current-limiting resistor.

## Scope

This crate provides low-level ATmega328P and Arduino Uno primitives.
