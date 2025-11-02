#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};
use core::mem;

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive, Input, Pull};
use embassy_nrf::{bind_interrupts, pac, peripherals, usb};
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};
use defmt::*;
use usbd_hid::descriptor::{KeyboardReport};


use nrf_softdevice::{Softdevice, raw};
use nrf_softdevice::ble::advertisement_builder::{
    Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList, ServiceUuid16,
    AdvertisementDataType
};
use nrf_softdevice::ble::gatt_server::builder::ServiceBuilder;
use nrf_softdevice::ble::gatt_server::characteristic::{Attribute, Metadata, Properties};
use nrf_softdevice::ble::gatt_server::{CharacteristicHandles, RegisterError, WriteOp};
use nrf_softdevice::ble::{gatt_server, peripheral, Connection, Uuid};
use nrf_softdevice::ble::security::SecurityHandler;

use static_cell::StaticCell;
// mod usb;
mod keyboard;


macro_rules! count {
	() => { 0u8 };
	($x:tt $($xs:tt)*) => {1u8 + count!($($xs)*)};
}

macro_rules! hid {
	($(( $($xs:tt),*)),+ $(,)?) => { &[ $( (count!($($xs)*)-1) | $($xs),* ),* ] };
}

const DEVICE_INFORMATION: Uuid = Uuid::new_16(0x180a);
const BATTERY_SERVICE: Uuid = Uuid::new_16(0x180f);

const BATTERY_LEVEL: Uuid = Uuid::new_16(0x2a19);
const MODEL_NUMBER: Uuid = Uuid::new_16(0x2a24);
const SERIAL_NUMBER: Uuid = Uuid::new_16(0x2a25);
const FIRMWARE_REVISION: Uuid = Uuid::new_16(0x2a26);
const HARDWARE_REVISION: Uuid = Uuid::new_16(0x2a27);
const SOFTWARE_REVISION: Uuid = Uuid::new_16(0x2a28);
const MANUFACTURER_NAME: Uuid = Uuid::new_16(0x2a29);
const PNP_ID: Uuid = Uuid::new_16(0x2a50);

const HID_INFO: Uuid = Uuid::new_16(0x2a4a);
const REPORT_MAP: Uuid = Uuid::new_16(0x2a4b);
const HID_CONTROL_POINT: Uuid = Uuid::new_16(0x2a4c);
const HID_REPORT: Uuid = Uuid::new_16(0x2a4d);
const PROTOCOL_MODE: Uuid = Uuid::new_16(0x2a4e);

// Main items
pub const HIDINPUT: u8 = 0x80;
pub const HIDOUTPUT: u8 = 0x90;
pub const FEATURE: u8 = 0xb0;
pub const COLLECTION: u8 = 0xa0;
pub const END_COLLECTION: u8 = 0xc0;

// Global items
pub const USAGE_PAGE: u8 = 0x04;
pub const LOGICAL_MINIMUM: u8 = 0x14;
pub const LOGICAL_MAXIMUM: u8 = 0x24;
pub const PHYSICAL_MINIMUM: u8 = 0x34;
pub const PHYSICAL_MAXIMUM: u8 = 0x44;
pub const UNIT_EXPONENT: u8 = 0x54;
pub const UNIT: u8 = 0x64;
pub const REPORT_SIZE: u8 = 0x74; //bits
pub const REPORT_ID: u8 = 0x84;
pub const REPORT_COUNT: u8 = 0x94; //bytes
pub const PUSH: u8 = 0xa4;
pub const POP: u8 = 0xb4;

// Local items
pub const USAGE: u8 = 0x08;
pub const USAGE_MINIMUM: u8 = 0x18;
pub const USAGE_MAXIMUM: u8 = 0x28;
pub const DESIGNATOR_INDEX: u8 = 0x38;
pub const DESIGNATOR_MINIMUM: u8 = 0x48;
pub const DESIGNATOR_MAXIMUM: u8 = 0x58;
pub const STRING_INDEX: u8 = 0x78;
pub const STRING_MINIMUM: u8 = 0x88;
pub const STRING_MAXIMUM: u8 = 0x98;
pub const DELIMITER: u8 = 0xa8;

const KEYBOARD_ID: u8 = 0x01;
const MEDIA_KEYS_ID: u8 = 0x02;

const HID_REPORT_DESCRIPTOR: &[u8] = hid!(
    (USAGE_PAGE, 0x01), // USAGE_PAGE (Generic Desktop Ctrls)
    (USAGE, 0x06),      // USAGE (Keyboard)
    (COLLECTION, 0x01), // COLLECTION (Application)
    // ------------------------------------------------- Keyboard
    (REPORT_ID, KEYBOARD_ID), //   REPORT_ID (1)
    (USAGE_PAGE, 0x07),       //   USAGE_PAGE (Kbrd/Keypad)
    (USAGE_MINIMUM, 0xE0),    //   USAGE_MINIMUM (0xE0)
    (USAGE_MAXIMUM, 0xE7),    //   USAGE_MAXIMUM (0xE7)
    (LOGICAL_MINIMUM, 0x00),  //   LOGICAL_MINIMUM (0)
    (LOGICAL_MAXIMUM, 0x01),  //   Logical Maximum (1)
    (REPORT_SIZE, 0x01),      //   REPORT_SIZE (1)
    (REPORT_COUNT, 0x08),     //   REPORT_COUNT (8)
    (HIDINPUT, 0x02),         //   INPUT (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (REPORT_COUNT, 0x01),     //   REPORT_COUNT (1) ; 1 byte (Reserved)
    (REPORT_SIZE, 0x08),      //   REPORT_SIZE (8)
    (HIDINPUT, 0x01),         //   INPUT (Const,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (REPORT_COUNT, 0x05),     //   REPORT_COUNT (5) ; 5 bits (Num lock, Caps lock, Scroll lock, Compose, Kana)
    (REPORT_SIZE, 0x01),      //   REPORT_SIZE (1)
    (USAGE_PAGE, 0x08),       //   USAGE_PAGE (LEDs)
    (USAGE_MINIMUM, 0x01),    //   USAGE_MINIMUM (0x01) ; Num Lock
    (USAGE_MAXIMUM, 0x05),    //   USAGE_MAXIMUM (0x05) ; Kana
    (HIDOUTPUT, 0x02),        //   OUTPUT (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    (REPORT_COUNT, 0x01),     //   REPORT_COUNT (1) ; 3 bits (Padding)
    (REPORT_SIZE, 0x03),      //   REPORT_SIZE (3)
    (HIDOUTPUT, 0x01),        //   OUTPUT (Const,Array,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    (REPORT_COUNT, 0x06),     //   REPORT_COUNT (6) ; 6 bytes (Keys)
    (REPORT_SIZE, 0x08),      //   REPORT_SIZE(8)
    (LOGICAL_MINIMUM, 0x00),  //   LOGICAL_MINIMUM(0)
    (LOGICAL_MAXIMUM, 0x65),  //   LOGICAL_MAXIMUM(0x65) ; 101 keys
    (USAGE_PAGE, 0x07),       //   USAGE_PAGE (Kbrd/Keypad)
    (USAGE_MINIMUM, 0x00),    //   USAGE_MINIMUM (0)
    (USAGE_MAXIMUM, 0x65),    //   USAGE_MAXIMUM (0x65)
    (HIDINPUT, 0x00),         //   INPUT (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (END_COLLECTION),         // END_COLLECTION
    // ------------------------------------------------- Media Keys
    (USAGE_PAGE, 0x0C),         // USAGE_PAGE (Consumer)
    (USAGE, 0x01),              // USAGE (Consumer Control)
    (COLLECTION, 0x01),         // COLLECTION (Application)
    (REPORT_ID, MEDIA_KEYS_ID), //   REPORT_ID (2)
    (USAGE_PAGE, 0x0C),         //   USAGE_PAGE (Consumer)
    (LOGICAL_MINIMUM, 0x00),    //   LOGICAL_MINIMUM (0)
    (LOGICAL_MAXIMUM, 0x01),    //   LOGICAL_MAXIMUM (1)
    (REPORT_SIZE, 0x01),        //   REPORT_SIZE (1)
    (REPORT_COUNT, 0x10),       //   REPORT_COUNT (16)
    (USAGE, 0xB5),              //   USAGE (Scan Next Track)     ; bit 0: 1
    (USAGE, 0xB6),              //   USAGE (Scan Previous Track) ; bit 1: 2
    (USAGE, 0xB7),              //   USAGE (Stop)                ; bit 2: 4
    (USAGE, 0xCD),              //   USAGE (Play/Pause)          ; bit 3: 8
    (USAGE, 0xE2),              //   USAGE (Mute)                ; bit 4: 16
    (USAGE, 0xE9),              //   USAGE (Volume Increment)    ; bit 5: 32
    (USAGE, 0xEA),              //   USAGE (Volume Decrement)    ; bit 6: 64
    (USAGE, 0x23, 0x02),        //   Usage (WWW Home)            ; bit 7: 128
    (USAGE, 0x94, 0x01),        //   Usage (My Computer) ; bit 0: 1
    (USAGE, 0x92, 0x01),        //   Usage (Calculator)  ; bit 1: 2
    (USAGE, 0x2A, 0x02),        //   Usage (WWW fav)     ; bit 2: 4
    (USAGE, 0x21, 0x02),        //   Usage (WWW search)  ; bit 3: 8
    (USAGE, 0x26, 0x02),        //   Usage (WWW stop)    ; bit 4: 16
    (USAGE, 0x24, 0x02),        //   Usage (WWW back)    ; bit 5: 32
    (USAGE, 0x83, 0x01),        //   Usage (Media sel)   ; bit 6: 64
    (USAGE, 0x8A, 0x01),        //   Usage (Mail)        ; bit 7: 128
    (HIDINPUT, 0x02),           // INPUT (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (END_COLLECTION),           // END_COLLECTION
);


/// キーボードタスクからUSBタスクへコマンドを送るためのチャネル
pub static KEY_COMMAND_CHANNEL: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    KeyboardReport,
    4, // キューのサイズ (4つまでバッファリング可能)
> = embassy_sync::channel::Channel::new();

/// USBのサスペンド状態を共有する
pub static SUSPENDED: AtomicBool = AtomicBool::new(false);

static SERVER: StaticCell<Server> = StaticCell::new();

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[embassy_executor::task]
async fn send_keys_task(conn: Connection, server: &'static Server) {
    // 少し待って（接続確立＋HID Notify有効化を待つ）
    Timer::after(Duration::from_secs(2)).await;
    
    loop {
        let report = KEY_COMMAND_CHANNEL.receive().await;
        server.hid.send_key(&conn, report.keycodes[0]);
        // info!("Sending key report: {:?}", report);
        // let _ = server.hid.on_write(&conn, handle, data);
    }
    // Timer::after(Duration::from_millis(100)).await;
    // let _ = server.input_report_notify(&conn, &KeyboardReport::default());
}
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(config());
    Timer::after(Duration::from_millis(100)).await;

    let sd_config = nrf_softdevice::Config {
        clock: Some(nrf_softdevice::raw::nrf_clock_lf_cfg_t {
            source: nrf_softdevice::raw::NRF_CLOCK_LF_SRC_XTAL as u8,
            rc_ctiv: 0,
            rc_temp_ctiv: 0,
            accuracy: nrf_softdevice::raw::NRF_CLOCK_LF_ACCURACY_50_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 6, // 接続可能な同時接続数
            event_length: 24, // イベント長 1.25ms 単位 (1.25 * 24 -> 30ms)
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: raw::BLE_GATTS_ATTR_TAB_SIZE_DEFAULT,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1, // アドバタイジングセット数 同時にアドバタイズできる数
            periph_role_count: 3, // ペリフェラル役割の同時接続数
            central_role_count: 3, // セントラル役割の同時接続数 
            central_sec_count: 0, //Centralとして接続する際に、同時にセキュリティ手順を実行できる数 ?
            _bitfield_1: raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"HelloRust" as *const u8 as _,
            current_len: 9,
            max_len: 9,
            write_perm: unsafe { mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };


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
    let sd: &'static mut Softdevice = Softdevice::enable(&sd_config);
    let server = SERVER.init(Server::new(sd, "123456").unwrap()); 
    // let server= Server::new(sd, "123456").unwrap();
    unwrap!(spawner.spawn(softdevice_task(sd)));

    spawner.spawn(keyboard::keyboard_task(keyboard_pins)).unwrap();

    // GATTサーバーの初期化
    static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
        .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
        .services_16(
            ServiceList::Incomplete,
            &[ServiceUuid16::BATTERY, ServiceUuid16::HUMAN_INTERFACE_DEVICE],
        )
        .full_name("HelloRust")
        // Change the appearance (icon of the bluetooth device) to a keyboard
        .raw(AdvertisementDataType::APPEARANCE, &[0xC1, 0x03])
        .build();

    static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
        .services_16(
            ServiceList::Complete,
            &[
                ServiceUuid16::DEVICE_INFORMATION,
                ServiceUuid16::BATTERY,
                ServiceUuid16::HUMAN_INTERFACE_DEVICE,
            ],
        )
        .build();
        static SEC: HidSecurityHandler = HidSecurityHandler {};


    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data: &ADV_DATA,
            scan_data: &SCAN_DATA,
        };
        let conn = peripheral::advertise_pairable(sd, adv, &config, &SEC).await.unwrap();
        info!("advertising done!");
        Timer::after(Duration::from_micros(1000)).await;

        spawner.spawn(send_keys_task(conn.clone(), server)).unwrap();
    
        let e = gatt_server::run(&conn, server, |_| {
            
            info!("GATT event");
            
        }).await;


        info!("gatt_server run exited with error: {:?}", e);
    
    }
    

}

fn config() -> embassy_nrf::config::Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P5;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P5;
    config
}

struct HidSecurityHandler {}

impl SecurityHandler for HidSecurityHandler {}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum VidSource {
    BluetoothSIG = 1,
    UsbIF = 2,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct PnPID {
    pub vid_source: VidSource,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_version: u16,
}

#[derive(Debug, Default, defmt::Format)]
pub struct DeviceInformation {
    pub manufacturer_name: Option<&'static str>,
    pub model_number: Option<&'static str>,
    pub serial_number: Option<&'static str>,
    pub hw_rev: Option<&'static str>,
    pub fw_rev: Option<&'static str>,
    pub sw_rev: Option<&'static str>,
}
pub struct DeviceInformationService {}

impl DeviceInformationService{
    pub fn new(sd: &mut Softdevice, pnp_id: &PnPID, info: DeviceInformation) -> Result<Self, RegisterError> {

        let mut sb = ServiceBuilder::new(sd, DEVICE_INFORMATION)?;

        Self::add_pnp_characteristic(&mut sb, pnp_id)?;
        Self::add_opt_str_characteristic(&mut sb, MANUFACTURER_NAME, info.manufacturer_name)?;
        Self::add_opt_str_characteristic(&mut sb, MODEL_NUMBER, info.model_number)?;
        Self::add_opt_str_characteristic(&mut sb, SERIAL_NUMBER, info.serial_number)?;
        Self::add_opt_str_characteristic(&mut sb, HARDWARE_REVISION, info.hw_rev)?;
        Self::add_opt_str_characteristic(&mut sb, FIRMWARE_REVISION, info.fw_rev)?;
        Self::add_opt_str_characteristic(&mut sb, SOFTWARE_REVISION, info.sw_rev)?;

        let _service_handle = sb.build();

        Ok(DeviceInformationService {})
    }

    fn add_opt_str_characteristic(
        sb: &mut ServiceBuilder,
        uuid: Uuid,
        val: Option<&'static str>,
    ) -> Result<Option<CharacteristicHandles>, RegisterError> {
        if let Some(val) = val {
            let attr = Attribute::new(val);
            let md = Metadata::new(Properties::new().read());
            Ok(Some(sb.add_characteristic(uuid, attr, md)?.build()))
        } else {
            Ok(None)
        }
    }

    fn add_pnp_characteristic(sb: &mut ServiceBuilder, pnp_id: &PnPID) -> Result<CharacteristicHandles, RegisterError> {
        // SAFETY: `PnPID` is `repr(C, packed)` so viewing it as an immutable slice of bytes is safe.
        let val =
            unsafe { core::slice::from_raw_parts(pnp_id as *const _ as *const u8, core::mem::size_of::<PnPID>()) };

        let attr = Attribute::new(val);
        let md = Metadata::new(Properties::new().read());
        Ok(sb.add_characteristic(PNP_ID, attr, md)?.build())
    }
}

#[allow(dead_code)]
pub struct HidService {
    hid_info: u16,
    report_map: u16,
    hid_control: u16,
    protocol_mode: u16,
    input_keyboard: u16,
    input_keyboard_cccd: u16,
    input_keyboard_descriptor: u16,
    output_keyboard: u16,
    output_keyboard_descriptor: u16,
    input_media_keys: u16,
    input_media_keys_cccd: u16,
    input_media_keys_descriptor: u16,
}

impl HidService {
    pub fn new(sd: &mut Softdevice) -> Result<Self, RegisterError> {
        let mut service_builder = ServiceBuilder::new(sd, Uuid::new_16(0x1812))?;

        let hid_info = service_builder.add_characteristic(
            HID_INFO,
            Attribute::new([0x11u8, 0x1u8, 0x00u8, 0x01u8]),
            Metadata::new(Properties::new().read()),
        )?;
        let hid_info_handle = hid_info.build();

        let report_map = service_builder.add_characteristic(
            REPORT_MAP,
            Attribute::new(HID_REPORT_DESCRIPTOR),
            Metadata::new(Properties::new().read()),
        )?;
        let report_map_handle = report_map.build();

        let hid_control = service_builder.add_characteristic(
            HID_CONTROL_POINT,
            Attribute::new([0u8]),
            Metadata::new(Properties::new().write_without_response()),
        )?;
        let hid_control_handle = hid_control.build();

        let protocol_mode = service_builder.add_characteristic(
            PROTOCOL_MODE,
            Attribute::new([1u8]),
            Metadata::new(Properties::new().read().write_without_response()),
        )?;
        let protocol_mode_handle = protocol_mode.build();

        let mut input_keyboard = service_builder.add_characteristic(
            HID_REPORT,
            Attribute::new([0u8; 8]),
            Metadata::new(Properties::new().read().notify()),
        )?;
        let input_keyboard_desc =
            input_keyboard.add_descriptor(Uuid::new_16(0x2908), Attribute::new([KEYBOARD_ID, 1u8]))?; // First is ID (e.g. 1 for keyboard 2 for media keys), second is in/out
        let input_keyboard_handle = input_keyboard.build();

        let mut output_keyboard = service_builder.add_characteristic(
            HID_REPORT,
            Attribute::new([0u8; 8]),
            Metadata::new(Properties::new().read().write().write_without_response()),
        )?;
        let output_keyboard_desc =
            output_keyboard.add_descriptor(Uuid::new_16(0x2908), Attribute::new([KEYBOARD_ID, 2u8]))?; // First is ID (e.g. 1 for keyboard 2 for media keys)
        let output_keyboard_handle = output_keyboard.build();

        let mut input_media_keys = service_builder.add_characteristic(
            HID_REPORT,
            Attribute::new([0u8; 16]),
            Metadata::new(Properties::new().read().notify()),
        )?;
        let input_media_keys_desc =
            input_media_keys.add_descriptor(Uuid::new_16(0x2908), Attribute::new([MEDIA_KEYS_ID, 1u8]))?;
        let input_media_keys_handle = input_media_keys.build();

        let _service_handle = service_builder.build();

        Ok(HidService {
            hid_info: hid_info_handle.value_handle,
            report_map: report_map_handle.value_handle,
            hid_control: hid_control_handle.value_handle,
            protocol_mode: protocol_mode_handle.value_handle,
            input_keyboard: input_keyboard_handle.value_handle,
            input_keyboard_cccd: input_keyboard_handle.cccd_handle,
            input_keyboard_descriptor: input_keyboard_desc.handle(),
            output_keyboard: output_keyboard_handle.value_handle,
            output_keyboard_descriptor: output_keyboard_desc.handle(),
            input_media_keys: input_media_keys_handle.value_handle,
            input_media_keys_cccd: input_media_keys_handle.cccd_handle,
            input_media_keys_descriptor: input_media_keys_desc.handle(),
        })
    }

    pub fn on_write(&self, conn: &Connection, handle: u16, data: &[u8]) {
        let val = &[
            0, // Modifiers (Shift, Ctrl, Alt, GUI, etc.)
            0, // Reserved
            0x0E, 0, 0, 0, 0, 0, // Key code array - 0x04 is 'a' and 0x1d is 'z' - for example
        ];
        // gatt_server::notify_value(conn, self.input_keyboard_cccd, val).unwrap();
        // gatt_server::notify_value(conn, self.input_keyboard_descriptor, val).unwrap();
        if handle == self.input_keyboard_cccd {
            info!("HID input keyboard notify: {:?}", data);
        } else if handle == self.output_keyboard {
            // Fires if a keyboard output is changed - e.g. the caps lock LED
            info!("HID output keyboard: {:?}", data);

            if *data.get(0).unwrap() == 1 {
                gatt_server::notify_value(conn, self.input_keyboard, val).unwrap();
                info!("Keyboard report sent");
            } else {
                gatt_server::notify_value(conn, self.input_keyboard, &[0u8; 8]).unwrap();
                info!("Keyboard report cleared");
            }
        } else if handle == self.input_media_keys_cccd {
            info!("HID input media keys: {:?}", data);
        }
    }

    pub fn send_key(&self, conn: &Connection, keycode: u8) {
        let report = [0, 0, keycode, 0, 0, 0, 0, 0];
        gatt_server::notify_value(conn, self.input_keyboard, &report).unwrap();
        info!("Key 0x{:02X} sent", keycode);

        // 離す
        gatt_server::notify_value(conn, self.input_keyboard, &[0u8; 8]).unwrap();
    }
}


struct Server {
    _dis: DeviceInformationService,
    hid: HidService,
}

impl Server {
    pub fn new(sd: &mut Softdevice, serial_number: &'static str) -> Result<Self, RegisterError> {
        let dis = DeviceInformationService::new(
            sd,
            &PnPID {
                vid_source: VidSource::UsbIF,
                vendor_id: 0xDEAD,
                product_id: 0xBEEF,
                product_version: 0x0000,
            },
            DeviceInformation {
                manufacturer_name: Some("Embassy"),
                model_number: Some("M1234"),
                serial_number: Some(serial_number),
                ..Default::default()
            },
        )?;


        let hid = HidService::new(sd)?;

        Ok(Self { _dis: dis, hid })
    }
}

impl gatt_server::Server for Server {
    type Event = ();

    fn on_write(
        &self,
        conn: &Connection,
        handle: u16,
        _op: WriteOp,
        _offset: usize,
        data: &[u8],
    ) -> Option<Self::Event> {
        self.hid.on_write(conn, handle, data);
        // self.bas.on_write(handle, data);
        None
    }
}
