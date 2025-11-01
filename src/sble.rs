
use defmt_rtt as _; // global logger
use embassy_nrf as _; // time driver
use panic_probe as _;
use core::mem;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;
use embassy_time::Timer;
use embassy_time::Duration;

use nrf_softdevice::ble::advertisement_builder::{
    Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList, ServiceUuid16,
};

use nrf_softdevice::ble::peripheral;
use nrf_softdevice::{raw, Softdevice};

use embassy_executor::Spawner;
#[embassy_executor::task]
pub async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await;
}

#[embassy_executor::task]
pub async fn ble_task(sd: &'static Softdevice) {

    let mut config = peripheral::Config::default();
    config.interval = 50;

    static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
        .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
        .services_16(ServiceList::Complete, &[ServiceUuid16::HEALTH_THERMOMETER]) // if there were a lot of these there may not be room for the full name
        .short_name("Hello")
        .build();

    // but we can put it in the scan data
    // so the full name is visible once connected
    static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new().full_name("Hello, Rust!").build();

    let adv = peripheral::NonconnectableAdvertisement::ScannableUndirected {
        adv_data: &ADV_DATA,
        scan_data: &SCAN_DATA,
    };
    unwrap!(peripheral::advertise(sd, adv, &config).await);

}