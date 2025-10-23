#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive, Input, Pull};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let mut led: Output<'_> = Output::new(p.P0_15, Level::Low, OutputDrive::Standard);

    let mut cols = [
        Output::new(p.P0_29, Level::High, OutputDrive::Standard),
        Output::new(p.P0_02, Level::High, OutputDrive::Standard),
        Output::new(p.P1_15, Level::High, OutputDrive::Standard),
        Output::new(p.P1_13, Level::High, OutputDrive::Standard),
        Output::new(p.P1_11, Level::High, OutputDrive::Standard),
    ];

    let rows= [
        Input::new(p.P1_04, Pull::Up),
        Input::new(p.P1_06, Pull::Up),
        // Input::new(p.P0_09, Pull::Up),
        // Input::new(p.P0_10, Pull::Up),
    ];

    led.set_high();
    Timer::after_millis(300).await;

    loop {
        let mut state = [[false; 5]; 2];

        for(ci, col) in cols.iter_mut().enumerate() {
            col.set_low();

            Timer::after(Duration::from_micros(50)).await;

            for(ri, row) in rows.iter().enumerate() {
                if row.is_low() {
                    state[ri][ci] = true;
                }
            }

            col.set_high();
        }

        for (ri, row) in state.iter().enumerate() {
            for (ci, &pressed) in row.iter().enumerate() {
                if pressed {
                    info!("Key pressed: row={} col={}", ri, ci);
                }
            }
        }

        Timer::after(Duration::from_millis(20)).await;

        
        // led.set_low();
        // Timer::after_millis(300).await;
        info!("loop");
    }
}
