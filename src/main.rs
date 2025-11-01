#![no_std]
#![no_main]

use defmt_rtt as _; // global logger
use embassy_nrf as _; // time driver
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive, Input, Pull};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};
use defmt::*;
use core::sync::atomic::{AtomicBool, Ordering};
use usbd_hid::descriptor::{KeyboardReport};

use embassy_nrf::{bind_interrupts, pac, peripherals, usb};

use nrf_softdevice::{raw, Softdevice};

// mod usb;
mod keyboard;
mod lusb;
mod sble;

use core::mem;

// use {defmt_rtt as _, panic_probe as _};
// use defmt::*;
// use embassy_time::Timer;
// use embassy_time::Duration;

use nrf_softdevice::ble::advertisement_builder::{
    Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList, ServiceUuid16,
};

use nrf_softdevice::ble::peripheral;
// use nrf_softdevice::{raw, Softdevice};

// --- 割り込み定義 ---
// (usbモジュール内で InterruptHandler を使うため pub(crate) に)
// bind_interrupts!(pub struct Irqs {
//     USBD => usb::InterruptHandler<peripherals::USBD>;
//     CLOCK_POWER => usb::vbus_detect::InterruptHandler;
// });

// --- グローバル変数 (タスク間通信用) ---

/// キーボードタスクからUSBタスクへコマンドを送るためのチャネル
pub static KEY_COMMAND_CHANNEL: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    KeyboardReport,
    4, // キューのサイズ (4つまでバッファリング可能)
> = embassy_sync::channel::Channel::new();

/// USBのサスペンド状態を共有する
pub static SUSPENDED: AtomicBool = AtomicBool::new(false);

static mut SD: Option<Softdevice> = None;

#[embassy_executor::main]
async fn main(spawner: Spawner) {

    let p = embassy_nrf::init(config());
    Timer::after(Duration::from_millis(100)).await;

    // let mut config = embassy_nrf::config::Config::default();
    // config.lfclk_source = embassy_nrf::config::LfclkSource::ExternalXtal;

    let sd_config = nrf_softdevice::Config {
        clock: Some(nrf_softdevice::raw::nrf_clock_lf_cfg_t {
            source: nrf_softdevice::raw::NRF_CLOCK_LF_SRC_XTAL as u8,
            rc_ctiv: 0,
            rc_temp_ctiv: 0,
            accuracy: nrf_softdevice::raw::NRF_CLOCK_LF_ACCURACY_50_PPM as u8,
        }),
        ..Default::default()
    };

    let sd: &'static mut Softdevice = Softdevice::enable(&sd_config);
    spawner.spawn(sble::softdevice_task(sd)).unwrap();

    let mut led: Output<'_> = Output::new(p.P0_15, Level::Low, OutputDrive::Standard);
    let keyboard_pins = keyboard::KeyboardPins::new(
        p.P0_29,
        p.P0_02,
        p.P1_15,
        p.P1_13,
        p.P1_11,
        p.P1_04,
        p.P1_06,
        p.P0_09,
        p.P0_10,
    );


    // spawner.spawn(lusb::usb_task(p.USBD)).unwrap();
    // spawner.spawn(sble::ble_task(&sd)).unwrap();
    spawner.spawn(keyboard::keyboard_task(keyboard_pins)).unwrap();
    spawner.spawn(sble::ble_task(sd)).unwrap();

    loop {
        Timer::after(Duration::from_micros(1000)).await;

        // let k =  KEY_COMMAND_CHANNEL.receive().await;
        // info!("Received keycode: {}", k.keycodes[0]);
    
    }
    

}


fn config() -> embassy_nrf::config::Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    config
}