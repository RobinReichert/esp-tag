//! Embassy ESP-NOW Example (Duplex)
//!
//! Asynchronously broadcasts, receives and sends messages via esp-now in
//! multiple embassy tasks

#![cfg(feature = "hardware")] // Compile this file only if hardware feature enabled
#![no_std]
#![no_main]

mod hardware;
mod logic;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock, interrupt::software::SoftwareInterruptControl, time::Rate,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_radio::{Controller, esp_now::BROADCAST_ADDRESS};

use crate::{
    hardware::{
        display::Display,
        shared_bus::{SharedBus, SharedBusInterface},
    },
    logic::{mesh, message, node::Node, tree::Tree},
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let esp_radio_ctrl = &*mk_static!(Controller<'static>, esp_radio::init().unwrap());
    let wifi = peripherals.WIFI;
    let (mut controller, interfaces) =
        esp_radio::wifi::new(&esp_radio_ctrl, wifi, Default::default()).unwrap();
    controller.set_mode(esp_radio::wifi::WifiMode::Sta).unwrap();
    controller.start().unwrap();

    let i2c_bus = esp_hal::i2c::master::I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_scl(peripherals.GPIO9)
    .with_sda(peripherals.GPIO8)
    .into_async();

    let shared_bus = SharedBus::new(i2c_bus);

    let mut display = Display::new(SharedBusInterface::new(&shared_bus));
    display.init().await;
    unwrap_print!(display.show_center_text("Welcome").await);
    Timer::after(Duration::from_millis(500)).await;
    unwrap_print!(display.clear().await);
    Timer::after(Duration::from_millis(100)).await;
    unwrap_print!(display.show_center_text("to").await);
    Timer::after(Duration::from_millis(500)).await;
    unwrap_print!(display.clear().await);
    Timer::after(Duration::from_millis(100)).await;
    unwrap_print!(display.show_logo().await);

    loop {}
}
