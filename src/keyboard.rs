use embassy_nrf::
  {Peri, gpio::{Input, Level, Output, OutputDrive,Pull}, 
  peripherals::{P0_09, P0_29, P1_04, P1_06, P1_11, P1_13, P1_15, P0_02, P0_10}};
use defmt::*;
use embassy_time::{Timer, Duration};
use keycode::{ KeyMap, KeyMappingId};
use usbd_hid::descriptor::{KeyboardReport};

use crate::KEY_COMMAND_CHANNEL;

#[derive(Clone, Copy)]
struct KeyState {
    pressed: bool,
    hold_time_ms: u64, // 押され続けた時間
}

// struct KeyboardCommand {
//     event: KeyEvent,
// }

// #[derive(Clone, Copy, Debug)]
// enum KeyEvent {
//     Tap(u8, u8),
//     Repeat(u8, u8),
//     Release(u8, u8),
// }

const KEY_MAP: [[KeyMappingId; 5]; 4] = [
    // [KeyMappingId::Escape, KeyMappingId::Digit1, KeyMappingId::Digit2, KeyMappingId::Digit3,KeyMappingId::Digit4],
    [KeyMappingId::UsQ, KeyMappingId::UsW, KeyMappingId::UsE, KeyMappingId::UsR , KeyMappingId::UsT],
    [KeyMappingId::UsA, KeyMappingId::UsS, KeyMappingId::UsD, KeyMappingId::UsF, KeyMappingId::UsG],
    [KeyMappingId::UsZ, KeyMappingId::UsX, KeyMappingId::UsC, KeyMappingId::UsV, KeyMappingId::UsB],
    [KeyMappingId::ControlLeft, KeyMappingId::Comma, KeyMappingId::AltLeft, KeyMappingId::Space, KeyMappingId::ControlRight]
];

const ROWS: usize = 4;
const COLS: usize = 5;

// リピート設定
const REPEAT_DELAY_MS: u64 = 400;   // 押してからリピート開始まで
const REPEAT_INTERVAL_MS: u64 = 40; // リピート間隔

pub struct KeyboardPins {
    cols: [Output<'static>; 5],
    rows: [Input<'static>; 4]
}

impl KeyboardPins {
    pub fn new(
        p0_29:  Peri<'static, P0_29>,
        p0_02:  Peri<'static, P0_02>,
        p1_15:  Peri<'static, P1_15>, 
        p1_13:  Peri<'static, P1_13>,
        p1_11:  Peri<'static, P1_11>,
        p1_04:  Peri<'static, P1_04>,
        p1_06:  Peri<'static, P1_06>,
        p0_09:  Peri<'static, P0_09>,
        p0_10:  Peri<'static, P0_10>, 
    ) -> Self {
        Self {
            cols: [
                Output::new( p0_29, Level::Low, OutputDrive::Standard), // High -> Low
                Output::new(p0_02, Level::Low, OutputDrive::Standard), // High -> Low
                Output::new(p1_15, Level::Low, OutputDrive::Standard), // High -> Low
                Output::new(p1_13, Level::Low, OutputDrive::Standard), // High -> Low
                Output::new(p1_11, Level::Low, OutputDrive::Standard), // High -> Low
            ],
            rows: [
                Input::new(p1_04, Pull::Down), // Up -> Down
                Input::new(p1_06, Pull::Down), // Up -> Down
                Input::new(p0_09, Pull::Down), // Up -> Down
                Input::new(p0_10, Pull::Down), // Up -> Down
            ],
        }
    }
    
}

#[embassy_executor::task]
pub async fn keyboard_task(mut keypins: KeyboardPins) {

    let mut prev = [[KeyState { pressed: false, hold_time_ms: 0 }; COLS]; ROWS];
    let scan_interval = Duration::from_millis(20);

    loop {

      let mut state = [[false; 5]; 4];

        for(ci, col) in keypins.cols.iter_mut().enumerate() {
            col.set_high();

            Timer::after(Duration::from_micros(50)).await;

            for(ri, row) in keypins.rows.iter().enumerate() {
                if row.is_high() { // is_low() -> is_high()
                    state[ri][ci] = true;
                    // info!("Key pressed: row={} col={}", ri, ci);
                }
            }

            col.set_low();
        }


         // --- 判定 ---
        for (ri, row) in state.iter().enumerate() {
            for (ci, &pressed) in row.iter().enumerate() {
                let key = &mut prev[ri][ci];

                if pressed {
                    // 押された瞬間
                    if !key.pressed {
                        info!("TAP: row={} col={}", ri, ci); //ここでキーを送る
                        let key_map = KeyMap::from(KEY_MAP[ri][ci]);
                        let usb_code = key_map.usb; 
                        // info!("USB Keycode: {}", usb_code);
                
                        KEY_COMMAND_CHANNEL.send(KeyboardReport {
                            keycodes: [usb_code as u8, 0, 0, 0, 0, 0],
                            modifier: 0,
                            reserved: 0,
                            leds: 0,
                        }).await;
                        key.hold_time_ms = 0;
                    } else {
                        key.hold_time_ms += scan_interval.as_millis();

                        // 長押しリピート判定
                        if key.hold_time_ms >= REPEAT_DELAY_MS
                            && (key.hold_time_ms - REPEAT_DELAY_MS) % REPEAT_INTERVAL_MS == 0
                        {
                            info!("REPEAT: row={} col={}", ri, ci); //ここでキーを送る
                            let key_map = KeyMap::from(KEY_MAP[ri][ci]);
                            let usb_code = key_map.usb; 
                            // info!("USB Keycode: {}", usb_code);
                    
                            KEY_COMMAND_CHANNEL.send(KeyboardReport {
                                keycodes: [usb_code as u8, 0, 0, 0, 0, 0],
                                modifier: 0,
                                reserved: 0,
                                leds: 0,
                            }).await;
                            }
                    }
                } else {
                    // 離されたとき
                    if key.pressed {
                        // info!("RELEASE: row={} col={}", ri, ci);
                    }
                    key.hold_time_ms = 0;
                }

                key.pressed = pressed;
            }
        }

        Timer::after(Duration::from_millis(20)).await;
    }
}
