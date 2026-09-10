#![no_std]
#![no_main]

use uno_systems_lab::adc::{Adc, Channel};
use uno_systems_lab::debounce::Debouncer;
use uno_systems_lab::event_queue::SharedEventQueue;
use uno_systems_lab::events::{
    BinaryInputId, BinaryState, DeviceEvent, FieldId, Measurement, SensorId, TimestampedEvent,
};
use uno_systems_lab::gpio::{Input, Output, Pin, Pull, board};
use uno_systems_lab::interrupts::{enable_global, with_disabled};
use uno_systems_lab::serial::Usart0;
use uno_systems_lab::state::{SharedDeviceState, TimestampedMeasurement};
use uno_systems_lab::timer::{Prescaler, Timer1};

use core::panic::PanicInfo;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU8, Ordering};

// ── Capacity and timing configuration ────────────────────────────

const EVENT_QUEUE_CAPACITY: usize = 16;
const BINARY_INPUT_CAPACITY: usize = 4;
const MEASUREMENT_CAPACITY: usize = 8;

const TIMER_COMPARE: u16 = 2_499;
const SENSOR_SAMPLE_INTERVAL_TICKS: u32 = 10;

const POTENTIOMETER_CHANGE_THRESHOLD: u16 = 4;
const PHOTORESISTOR_CHANGE_THRESHOLD: u16 = 8;

// ── Hardware peripherals and pins ────────────────────────────────

const LED: Pin<Output> = Pin::<Output>::new(board::D13);
const EXTERNAL_LED: Pin<Output> = Pin::<Output>::new(board::D7);

const MOTOR_IN1: Pin<Output> = Pin::<Output>::new(board::D11);
const MOTOR_IN2: Pin<Output> = Pin::<Output>::new(board::D10);
const MOTOR_IN3: Pin<Output> = Pin::<Output>::new(board::D9);
const MOTOR_IN4: Pin<Output> = Pin::<Output>::new(board::D8);

const BUTTON: Pin<Input> = Pin::<Input>::new(board::D12);

const ADC: Adc = Adc::new();
const SERIAL: Usart0 = Usart0::new();
const SYSTEM_TIMER: Timer1 = Timer1::new();

// ── Protocol identifiers ─────────────────────────────────────────

const BUTTON_INPUT: BinaryInputId = BinaryInputId(1);

const POTENTIOMETER: SensorId = SensorId(1);
const PHOTORESISTOR: SensorId = SensorId(2);

const RAW_ADC: FieldId = FieldId(0);

// ── Shared runtime state ──────────────────────────────────────────

static EVENTS: SharedEventQueue<TimestampedEvent, EVENT_QUEUE_CAPACITY> = SharedEventQueue::new();

static DEVICE_STATE: SharedDeviceState<BINARY_INPUT_CAPACITY, MEASUREMENT_CAPACITY> =
    SharedDeviceState::new();

static mut UPTIME_TICKS: u32 = 0;

static QUEUE_OVERFLOWED: AtomicU8 = AtomicU8::new(0);
static STATE_CAPACITY_EXHAUSTED: AtomicU8 = AtomicU8::new(0);

static BUTTON_DEBOUNCER: Debouncer = Debouncer::new(3);

// ── Example-application state ────────────────────────────────────

// These values implement the demo LED behavior and are not part of
// the reusable device runtime.
static HEARTBEAT_PHASE: AtomicU8 = AtomicU8::new(0);

static DEMO_LED_BEHAVIOR: AtomicU8 = AtomicU8::new(DemoLedBehavior::Heartbeat as u8);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_entry() -> ! {
    BUTTON.configure(Pull::Up);
    BUTTON_DEBOUNCER.initialize(BUTTON.is_low());

    LED.configure();
    LED.set_low();

    EXTERNAL_LED.configure();
    EXTERNAL_LED.set_high();

    MOTOR_IN1.configure();
    MOTOR_IN2.configure();
    MOTOR_IN3.configure();
    MOTOR_IN4.configure();

    MOTOR_IN1.set_low();
    MOTOR_IN2.set_low();
    MOTOR_IN3.set_low();
    MOTOR_IN4.set_low();

    ADC.configure();

    HEARTBEAT_PHASE.store(0, Ordering::Relaxed);
    DEMO_LED_BEHAVIOR.store(DemoLedBehavior::Heartbeat as u8, Ordering::Relaxed);

    SYSTEM_TIMER.configure_ctc(TIMER_COMPARE);
    SYSTEM_TIMER.enable_compare_a_interrupt();
    SYSTEM_TIMER.start(Prescaler::Div64);

    SERIAL.configure(9_600);
    SERIAL.write_program_str("0 BOOT\r\n");

    unsafe {
        // Enable interrupts
        enable_global();
    }

    let mut last_sample_tick = 0u32;
    let mut last_reported_value: Option<u16> = None;
    let mut last_reported_light: Option<u16> = None;

    loop {
        if let Some(byte) = SERIAL.try_read_byte() {
            SERIAL.write_program_str("RX:");
            SERIAL.write_byte(byte);
            SERIAL.write_program_str("\r\n");
        }

        let uptime = with_disabled(|| unsafe { read_volatile(&raw const UPTIME_TICKS) });

        // Sample every 10 ticks = 100 ms.
        if uptime.wrapping_sub(last_sample_tick) >= SENSOR_SAMPLE_INTERVAL_TICKS {
            last_sample_tick = uptime;

            // Potentiometer
            let value = ADC.read(Channel::Adc0);
            let changed_enough = last_reported_value
                .is_none_or(|previous| value.abs_diff(previous) >= POTENTIOMETER_CHANGE_THRESHOLD);
            if changed_enough {
                record_measurement(uptime, POTENTIOMETER, RAW_ADC, value);

                last_reported_value = Some(value);
            }

            // Photoresistor
            let light = ADC.read(Channel::Adc1);
            let light_changed = last_reported_light
                .is_none_or(|previous| light.abs_diff(previous) >= PHOTORESISTOR_CHANGE_THRESHOLD);
            if light_changed {
                record_measurement(uptime, PHOTORESISTOR, RAW_ADC, light);

                last_reported_light = Some(light);
            }
        }

        while let Some(event) = EVENTS.pop() {
            report_event(event);
        }

        while let Some(current) = DEVICE_STATE.take_dirty_measurement() {
            report_event(TimestampedEvent {
                ticks: current.ticks,
                event: DeviceEvent::Measurement(current.measurement),
            });
        }

        let overflowed: bool = with_disabled(|| {
            let value = QUEUE_OVERFLOWED.load(Ordering::Relaxed);
            QUEUE_OVERFLOWED.store(0, Ordering::Relaxed);
            value != 0
        });

        if overflowed {
            let ticks = with_disabled(|| unsafe { read_volatile(&raw const UPTIME_TICKS) });

            SERIAL.write_u32(ticks);
            SERIAL.write_program_str(" ERROR QUEUE_OVERFLOW\r\n");
        }
    }
}

fn report_event(timestamped: TimestampedEvent) {
    SERIAL.write_u32(timestamped.ticks);

    match timestamped.event {
        DeviceEvent::BinaryInputChanged { input, state } => {
            SERIAL.write_program_str(" INPUT ");
            SERIAL.write_u32(input.0 as u32);

            match state {
                BinaryState::Active => {
                    SERIAL.write_program_str(" ACTIVE\r\n");
                }
                BinaryState::Inactive => {
                    SERIAL.write_program_str(" INACTIVE\r\n");
                }
            }
        }

        DeviceEvent::Measurement(measurement) => {
            SERIAL.write_program_str(" MEASUREMENT ");
            SERIAL.write_u32(measurement.sensor.0 as u32);
            SERIAL.write_program_str(" ");
            SERIAL.write_u32(measurement.field.0 as u32);
            SERIAL.write_program_str(" ");
            SERIAL.write_i32(measurement.value);
            SERIAL.write_program_str("\r\n");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn timer1_compare_tick() {
    let ticks = unsafe {
        let current = read_volatile(&raw const UPTIME_TICKS);
        let next = current.wrapping_add(1);

        write_volatile(&raw mut UPTIME_TICKS, next);
        next
    };

    if let Some(active) = BUTTON_DEBOUNCER.update(BUTTON.is_low()) {
        let state = if active {
            BinaryState::Active
        } else {
            BinaryState::Inactive
        };

        // Application behavior happens independently of telemetry.
        if active {
            let current = DemoLedBehavior::from_u8(DEMO_LED_BEHAVIOR.load(Ordering::Relaxed));

            DEMO_LED_BEHAVIOR.store(current.next() as u8, Ordering::Relaxed);

            HEARTBEAT_PHASE.store(0, Ordering::Relaxed);
        }

        let event = TimestampedEvent {
            ticks,
            event: DeviceEvent::BinaryInputChanged {
                input: BUTTON_INPUT,
                state,
            },
        };

        if EVENTS.push(event).is_err() {
            QUEUE_OVERFLOWED.store(1, Ordering::Relaxed);
        }
    }

    heartbeat_phase();
}

fn heartbeat_phase() {
    let behavior = DemoLedBehavior::from_u8(DEMO_LED_BEHAVIOR.load(Ordering::Relaxed));

    match behavior {
        DemoLedBehavior::Heartbeat => {
            let phase = HEARTBEAT_PHASE.load(Ordering::Relaxed);

            if phase < 10 || (phase >= 20 && phase < 30) {
                LED.set_high();
            } else {
                LED.set_low();
            }

            HEARTBEAT_PHASE.store(if phase == 99 { 0 } else { phase + 1 }, Ordering::Relaxed);
        }

        DemoLedBehavior::SolidOn => LED.set_high(),
        DemoLedBehavior::Off => LED.set_low(),
    }
}

fn record_measurement(ticks: u32, sensor: SensorId, field: FieldId, value: u16) {
    let current = TimestampedMeasurement {
        ticks,
        measurement: Measurement {
            sensor,
            field,
            value: value as i32,
        },
    };

    if DEVICE_STATE.update_measurement(current).is_err() {
        STATE_CAPACITY_EXHAUSTED.store(1, Ordering::Relaxed);
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum DemoLedBehavior {
    Heartbeat = 0,
    SolidOn = 1,
    Off = 2,
}

impl DemoLedBehavior {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::SolidOn,
            2 => Self::Off,
            _ => Self::Heartbeat,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Heartbeat => Self::SolidOn,
            Self::SolidOn => Self::Off,
            Self::Off => Self::Heartbeat,
        }
    }
}
