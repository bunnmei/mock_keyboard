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
        Output::new(p.P0_29, Level::Low, OutputDrive::Standard), // High -> Low
        Output::new(p.P0_02, Level::Low, OutputDrive::Standard), // High -> Low
        Output::new(p.P1_15, Level::Low, OutputDrive::Standard), // High -> Low
        Output::new(p.P1_13, Level::Low, OutputDrive::Standard), // High -> Low
        Output::new(p.P1_11, Level::Low, OutputDrive::Standard), // High -> Low
    ];
    let rows= [
        Input::new(p.P1_04, Pull::Down), // Up -> Down
        Input::new(p.P1_06, Pull::Down), // Up -> Down
        // Input::new(p.P0_09, Pull::Down),
        // Input::new(p.P0_10, Pull::Down),
    ];

    // let mut col0: Output<'_> = Output::new(p.P0_29, Level::High, OutputDrive::Standard);

    // let row0 = Input::new(p.P1_04, Pull::Up);
    led.set_high();
    Timer::after_millis(300).await;

    loop {
        // col0.set_low();
        // if row0.is_low() {
        //     info!("row low")
        // } else {
        //     info!("row high")
        // }

        // Timer::after_millis(300).await;
        let mut state = [[false; 5]; 2];

        for(ci, col) in cols.iter_mut().enumerate() {
            col.set_high();

            Timer::after(Duration::from_micros(50)).await;

            for(ri, row) in rows.iter().enumerate() {
                if row.is_high() { // is_low() -> is_high()
                    state[ri][ci] = true;
                    // info!("Key pressed: row={} col={}", ri, ci);
                }
            }

            col.set_low();
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
        // info!("loop");
    }
}
