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
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock, interrupt::software::SoftwareInterruptControl, timer::timg::TimerGroup,
};
use esp_println::println;
use esp_radio::Controller;

use crate::{
    hardware::mesh::Mesh,
    logic::{message, node::Node},
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

    let esp_now = interfaces.esp_now;
    esp_now.set_channel(11).unwrap();
    esp_println::println!("esp-now version {}", esp_now.version().unwrap());
    let (_, sender, receiver) = esp_now.split();

    let send_queue: Channel<NoopRawMutex, message::SendMessage, 16> = Channel::new();
    let receive_queue: Channel<NoopRawMutex, message::ReceiveMessage, 16> = Channel::new();
    let return_queue: Channel<NoopRawMutex, message::MessageData, 16> = Channel::new();
    let send_queue = mk_static!(Channel<NoopRawMutex, message::SendMessage, 16>, send_queue);
    let receive_queue =
        mk_static!(Channel<NoopRawMutex, message::ReceiveMessage, 16>, receive_queue);
    let return_queue = mk_static!(Channel<NoopRawMutex, message::MessageData, 16>, return_queue);

    let mesh = unwrap_print!(Mesh::new(
        spawner,
        send_queue,
        receive_queue,
        return_queue,
        receiver,
        sender
    ));
    match mesh.send(b"hallo123test", Node::new([1, 2, 3, 4, 5, 6])) {
        Ok(_) => (),
        Err(e) => println!("error:  {}", e),
    }

    let mut ticker = Ticker::every(Duration::from_millis(500));
    loop {
        if mesh.has_message() {
            println!("{:?}", mesh.get_message().expect("failed to get message"));
        }
        ticker.next().await;
    }
}
