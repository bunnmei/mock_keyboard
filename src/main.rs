#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive, Input, Pull};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};
use defmt::*;
use core::sync::atomic::{AtomicBool, Ordering};
use usbd_hid::descriptor::{KeyboardReport};

use embassy_nrf::{bind_interrupts, pac, peripherals, usb};

// mod usb;
mod keyboard;
mod lusb;

// --- 割り込み定義 ---
// (usbモジュール内で InterruptHandler を使うため pub(crate) に)
bind_interrupts!(pub struct Irqs {
    USBD => usb::InterruptHandler<peripherals::USBD>;
    CLOCK_POWER => usb::vbus_detect::InterruptHandler;
});

// --- グローバル変数 (タスク間通信用) ---

/// キーボードタスクからUSBタスクへコマンドを送るためのチャネル
pub static KEY_COMMAND_CHANNEL: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    KeyboardReport,
    4, // キューのサイズ (4つまでバッファリング可能)
> = embassy_sync::channel::Channel::new();

/// USBのサスペンド状態を共有する
pub static SUSPENDED: AtomicBool = AtomicBool::new(false);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
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


    spawner.spawn(lusb::usb_task(p.USBD)).unwrap();
    spawner.spawn(keyboard::keyboard_task(keyboard_pins)).unwrap();

    loop {
        Timer::after(Duration::from_micros(1000)).await;

        // let k =  KEY_COMMAND_CHANNEL.receive().await;
        // info!("Received keycode: {}", k.keycodes[0]);
    
    }
    

}
