#![no_std]
#![no_main]

use core::any::Any;
use core::ptr::addr_of;

use cortex_m::register::apsr::read;
use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{AnyPin, Input, Level, Output, Pull, Flex, OutputDrive};
use embassy_nrf::pac::pwm;
use embassy_nrf::Peripherals;
use embassy_time::{Duration, Timer};

// use embassy_nrf::chip::Peripherals;
use embassy_nrf::spim::{self, Spim};
use {defmt_rtt as _, panic_probe as _};

mod pmw3610;

// pub struct Spi3Wire {
//     sclk: Output<'static, AnyPin>,
//     sdio: embassy_nrf::gpio::Flex<'static, AnyPin>,
//     ncs: Output<static, AnyPin>
// }

pub struct Spi3Wire {
    sclk: Output<'static>,
    sdio: Flex<'static>,
    ncs: Output<'static>
}

#[derive(Debug)]
pub enum Error {
    Timeout,
    InvalidProduct(u8),
}

const BURST_READ_BYTES: usize = 7;

impl Spi3Wire {
    pub fn new(mut sclk: Output<'static>, sdio: Flex<'static>, mut ncs: Output<'static>) -> Self {
         // CPOL=1 のため、SCLKをHighで初期化
        sclk.set_low(); 
        // NCSは通信しない時はHigh
        ncs.set_high();
        Self {
            sclk,
            sdio,
            ncs,
        }
    }

    async fn transfer_byte_impl(&mut self, addr_byte: u8, write_data: Option<u8>) -> Result<u8, Error> {

        let is_write = write_data.is_some();
        let mut received_byte = 0u8;

        // --- 1. アドレス転送 ---
        
        // 書き込みアドレス: MSBを1にセット (例: 0x00 -> 0x80)
        // 読み込みアドレス: MSBを0のまま (例: 0x00 -> 0x00)
        let tx_addr = if is_write { addr_byte | 0x80 } else { addr_byte};
        let mut tx_buffer = [tx_addr, write_data.unwrap_or(0x00)];

        // SDIOを出力モードに設定
        self.sdio.set_as_output(OutputDrive::Standard);

        self.ncs.set_low(); // CS Low (通信開始)
        
        // データシート t_CS_TO_SCK (typ. 120ns) の待機
        Timer::after(Duration::from_nanos(150)).await; // 150nsを確保

        // 8ビットのアドレスを送信
        for i in (0..8).rev() { 
            let bit = (tx_addr >> i) & 1;

            // CPHA=1 (立ち下がりエッジでサンプリング)
            
            // 1. クロックを非アイドルレベル (Low) にする
            
            // 2. データをセット (Low期間中)
            self.sdio.set_level(if bit == 1 { Level::High } else { Level::Low });
            
            self.sclk.set_high();
            Timer::after(Duration::from_nanos(500)).await; // t_SCK_HALF
            // 3. クロックをアイドルレベル (High) に戻す (サンプリングエッジ)
            self.sclk.set_low(); 
            Timer::after(Duration::from_nanos(500)).await; // t_SCK_HALF
        }

        Timer::after(Duration::from_micros(20)).await;
        // --- 2. データ転送 ---
        
        if is_write {
            // 書き込みの場合：データバイトを送信
            let data_to_write = write_data.unwrap();

            for i in (0..8).rev() { 
                let bit = (data_to_write >> i) & 1;

                
                // self.sclk.set_high();
                self.sdio.set_level(if bit == 1 { Level::High } else { Level::Low });
                Timer::after(Duration::from_nanos(500)).await;
                self.sclk.set_high();
                self.sclk.set_low();
                Timer::after(Duration::from_nanos(500)).await;
            }
            
        } else {
            info!("read block");
            // 読み込みの場合：SDIOを入力に切り替え、データバイトを受信
            
            // データシート t_SCLK_TO_DATA_READ (typ. 140ns) の待機が必要な場合がある
            // 今回はアドレス送信直後なので、モード切り替え時間で対応
            
            // SDIOを入力モードに切り替え (データシート推奨に従いPull::Upを仮定)
            self.sdio.set_as_input(Pull::Up);
            Timer::after(Duration::from_micros(20)).await;
            // Timer::after(Duration::from_nanos(50)).await; // ピンモード切替の安定化待ち
            // Timer::after(Duration::from_nanos(50)).await; 
            for _ in 0..8 {
                // 1. クロックを非アイドルレベル (Low) にする
                // self.sclk.set_high();
                Timer::after(Duration::from_nanos(500)).await; 
                
                // 2. クロックがLow期間中にSDIOラインからデータを読み取る
                if self.sdio.is_high() {
                    received_byte = (received_byte << 1) | 1;
                } else {
                    received_byte <<= 1;
                }
                self.sclk.set_high();

                // 3. クロックをアイドルレベル (High) に戻す
                self.sclk.set_low(); 
                Timer::after(Duration::from_nanos(500)).await;
            }
        }

        self.ncs.set_high(); // CS High (通信終了)
        // データシート t_CS_HIGH (min 1us) の待機
        Timer::after(Duration::from_micros(2)).await;

        Ok(received_byte)
    }


    // 新しいヘルパー関数を Pmw3610Spi3Wire に追加
    pub async fn spi_clk_on(&mut self) -> Result<(), Error> {
        // PMW3610_SPI_CLK_ON_REQ (0x41) に SPI_CLOCK_ON_REQ_ON (0xBA) を書き込む
        self.write_reg(0x41, 0xBA).await?;
        
        // 300マイクロ秒待機 (Zephyrコードより)
        Timer::after(Duration::from_micros(300)).await;
        
        Ok(())
    }

    /// レジスタから1バイトのデータを読み込む
    pub async fn read_reg(&mut self, addr: u8) -> Result<u8, Error> {
        // self.spi_clk_on().await?;
        return self.transfer_byte_impl(addr, None).await;
    }


    pub async fn write_reg(&mut self, addr: u8, data: u8) -> Result<(), Error> {
        self.transfer_byte_impl(addr, Some(data)).await?;
        return Ok(())
    }

}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("PMW3610 init start");

    let p = embassy_nrf::init(Default::default());
    // sdio p0.22
    // sclk p0.17
    // ncs  p0.20
    // motion p0.24
    
    let mut sdio = Flex::new(p.P0_22);
    let mut sclk = Output::new(p.P0_17, Level::Low, OutputDrive::Standard);
    let mut ncs = Output::new(p.P0_20, Level::Low, OutputDrive::Standard);
    
    let mut pmw3610 = Spi3Wire::new(sclk, sdio, ncs);
    
    
    
    
    // sensor init sequence
    pmw3610.write_reg(pmw3610::reg::Reg::POWER_UP_RESET as u8, 0x5A).await.unwrap();
    
    // let pppp = pmw3610.read_reg(pmw3610::reg::Reg::POWER_UP_RESET as u8).await;
    Timer::after(Duration::from_millis(50)).await; // 50ms待機
    let res = pmw3610.read_reg(pmw3610::reg::Reg::PROD_ID as u8).await;
    match res {
        Ok(prod_id) => {
            info!("Product ID: 0x{:X}", prod_id);
        }
        Err(e) => {
            info!("can't read");
        }
    }
    // pmw3610.write_reg(pmw3610::reg::Reg::SHUTDOWN as u8, 0x00).await.unwrap();
    // pmw3610.read_reg(pmw3610::reg::Reg::MOTION as u8)  .await.unwrap();
    // // pmw3610.write_reg(pmw3610::reg::Reg::, data)

 

    let rr = pmw3610.read_reg(pmw3610::reg::Reg::RUN_DOWNSHIFT as u8).await;

    match rr {
        Ok(vul) => {
            info!("run downshift: 0x{:X}", vul);
        }
        Err(e) => {
            info!("can't read");
        }
    }

    pmw3610.spi_clk_on().await.unwrap();
    pmw3610.write_reg(pmw3610::reg::Reg::OBSERVATION1 as u8, 0x00).await.unwrap();
    Timer::after(Duration::from_millis(10)).await; 
    
    let rr2 = pmw3610.read_reg(pmw3610::reg::Reg::OBSERVATION1 as u8).await;
    match rr2 {
        Ok(vul) => {
            info!("observation1: 0x{:X}", vul);
            info!("observation1 masked: 0x{:X}", vul & 0x0F);

        }
        Err(e) => {
            info!("can't read");
        }
    }
    pmw3610.write_reg(0x85, 0x04).await.unwrap();
    let st = pmw3610.read_reg(0x05 as u8).await;
    match st {
        Ok(step) => {
            info!("res step: 0x{:X}", step);
        }
        Err(e) => {
            info!("can't read");
        }
    }
    loop {

    //     let xxx = pmw3610.read_reg(pmw3610::reg::Reg::DELTA_X_L as u8).await;
    // match xxx {
    //     Ok(vul) => {
    //         info!("c vul: 0x{:X}", vul);
    //     }
    //     Err(e) => {
    //         info!("can't read");
    //     }
    // }
        Timer::after(Duration::from_millis(1000)).await; 
        info!("hoge");
    }
}

//     pub async fn read_mm(&mut self) {
//         let motion = self.read_reg(pmw3610::reg::Reg::MOTION as u8).await.unwrap();
//         let p = motion & 0x80;

//         if (p != 0) {
//             // let (x, y) = self.read_motion_burst().await.unwrap();
//             let xl = self.read_reg(pmw3610::reg::Reg::DELTA_X_L as u8).await.unwrap();
//             let yl = self.read_reg(pmw3610::reg::Reg::DELTA_Y_L as u8).await.unwrap();
//             let xyh = self.read_reg(pmw3610::reg::Reg::DELTA_XY_H as u8).await.unwrap();

//             let mut x  = (((xyh >> 4) & 0x0F) << 8) as i16 | (xl as i16);
//             let mut y = ((xyh & 0x0F) << 8) as i16 | (yl as i16);

//             if (x & 0x800) != 0 {
//                 x |= (0xF000u16 as i16);
//             }

//             if y & 0x800 != 0 {
//                 y |= (0xF000u16 as i16);
//             }
            
//             info!("x 0x{:X} - y 0x{:X}", x, y);
//         }
//    }
