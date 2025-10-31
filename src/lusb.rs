use core::sync::atomic::{AtomicBool, Ordering};

use cortex_m::register::control;
use embassy_embedded_hal::shared_bus::asynch;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive, Input};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};
use defmt::*;

use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_nrf::usb::vbus_detect::HardwareVbusDetect;
// use embassy_nrf::usb::vbus_detect::HardwareVbusDetectSignal;
use embassy_nrf::usb::Driver;
use embassy_nrf::{bind_interrupts, pac, peripherals, usb, Peri};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

use embassy_sync::signal::Signal;
use embassy_usb::{class::hid::{HidReaderWriter, ReportId, RequestHandler, State}, driver, msos};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, Handler};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

use crate::{Irqs, SUSPENDED, KEY_COMMAND_CHANNEL};

#[embassy_executor::task]
pub async fn usb_task(usbd: Peri<'static, peripherals::USBD>) {
    pac::CLOCK.tasks_hfclkstart().write_value(1);
    while pac::CLOCK.events_hfclkstarted().read() != 1 {}
    let mut driver = Driver::new(
        usbd,
        Irqs,
        HardwareVbusDetect::new(Irqs),
    );

    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("HID Keyboard");
    config.serial_number = Some("123456");
    config.max_power = 100;
    config.max_packet_size_0 = 64;
    config.supports_remote_wakeup = true;

    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut request_handler = MyRequestHandler {};
    let mut device_handler = MyDeviceHandler::new();

    let mut state = State::new();


    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor, 
        &mut control_buf,
    );
    
    builder.handler(&mut device_handler);

    let config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };

    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, &mut state, config);

    let mut usb = builder.build();

    let remote_wakeup: Signal<CriticalSectionRawMutex, _> = Signal::new();

    let (reader, mut writer) = hid.split();

    let usb_fut = async {
        loop {
            usb.run_until_suspend().await;
            // 先に終わったを方を返す
            match select(usb.wait_resume(), remote_wakeup.wait()).await {
                Either::First(_) => (),
                Either::Second(_) => unwrap!(usb.remote_wakeup().await),
            }
        }
    };

    let in_fut = async {
        loop {
            
            // button.wait_for_low().await;
    
            if SUSPENDED.load(Ordering::Acquire) {
                info!("Triggering remote wakeup");
                remote_wakeup.signal(());
            } else {
                let report = KEY_COMMAND_CHANNEL.receive().await;
    
                match writer.write_serialize(&report).await {
                    Ok(()) => {},
                    Err(e) => warn!("Failed to write report: {:?}", e),
                }
    
            }
    
            info!("Button released");
            let report = KeyboardReport {
                keycodes: [0; 6],
                modifier: 0,
                reserved: 0,
                leds: 0,
            };
    
            match writer.write_serialize(&report).await {
                Ok(()) => {},
                Err(e) => warn!("Failed to write report: {:?}", e),
            };
        }
    };

    let out_fut = async {
        reader.run(false, &mut request_handler).await;
    };

    join(usb_fut, join(in_fut, out_fut)).await;
}


struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("Set report for {:?}: {=[u8]}", id, data);
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("Get idle rate for {:?}", id);
        None
    }
}


struct MyDeviceHandler {
    configured: AtomicBool,
}

impl MyDeviceHandler {
    pub const fn new() -> Self {
        Self {
            configured: AtomicBool::new(false),
        }
    }
    
}

impl Handler for MyDeviceHandler {
fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        SUSPENDED.store(false, Ordering::Release);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!("Device configured, it may now draw up to the configured current limit from Vbus.")
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }

    fn suspended(&mut self, suspended: bool) {
        if suspended {
            info!("Device suspended, the Vbus current limit is 500µA (or 2.5mA for high-power devices with remote wakeup enabled).");
            SUSPENDED.store(true, Ordering::Release);
        } else {
            SUSPENDED.store(false, Ordering::Release);
            if self.configured.load(Ordering::Relaxed) {
                info!("Device resumed, it may now draw up to the configured current limit from Vbus");
            } else {
                info!("Device resumed, the Vbus current limit is 100mA");
            }
        }
    }
}