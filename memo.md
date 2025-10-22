picoprove 
3v3 -> VDD
GP2 -> CLK
GP3 -> DIO
GND -> GND

$ cargo build --release
$ probe-rs info --chip nrf52840_xxAA

$ probe-rs run --chip nrf52840_xxAA target/thumbv7em-none-eabihf/release/nrf_pro_f

$ probe-rs list でプローブがUSB接続されているか確認できすr
$ prove-rs info 接続されているデバッグ可能デバイスが確認できる。