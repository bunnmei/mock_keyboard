
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Reg {    // Read/Write  Default Value
    /// R, Default: 0x3e
    PROD_ID = 0x00,
    /// R, Default: 0x01
    REV_ID = 0x01,
    /// R/W, Default: 0x09
    MOTION = 0x02,
    /// R, Default: 0x00
    DELTA_X_L = 0x03,
    /// R, Default: 0x00
    DELTA_Y_L = 0x04,
    /// R, Default: 0x00
    DELTA_XY_H = 0x05,
    /// R, Default: 0x00
    SQUAL = 0x06,
    /// R/W, Default: 0x00
    SHUTTER_HIGHER = 0x07,
    /// R/W, Default: 0x22
    SHUTTER_LOWER = 0x08,
    /// R, Default: 0x60
    PIX_MAX = 0x09,
    /// R, Default: 0x4f
    PIX_AVG = 0x0a,
    /// R, Default: 0x7f
    PIX_MIN = 0x0b,
    /// R, Default: 0x00
    CRC0 = 0x0c,
    /// R, Default: 0x00
    CRC1 = 0x0d,
    /// R, Default: 0x00
    CRC2 = 0x0e,
    /// R, Default: 0x00
    CRC3 = 0x0f,
    /// W, Default: 0x00
    SELF_TEST = 0x10,
    /// R/W, Default: 0x01
    PERFORMANCE = 0x11,
    /// R/W, Default: 0x0b
    BURST_READ = 0x12,
    /// R/W, Default: 0x02
    RUN_DOWNSHIFT = 0x1b,
    /// R/W, Default: 0x04
    REST1_RATE = 0x1c,
    /// R/W, Default: 0x1f
    REST1_DOWNSHIFT = 0x1d,
    /// R/W, Default: 0x0a
    REST2_RATE = 0x1e,
    /// R/W, Default: 0x2f
    REST2_DOWNSHIFT = 0x1f,
    /// R/W, Default: 0x32
    REST3_RATE = 0x20,
    /// R/W, Default: 0x00
    OBSERVATION1 = 0x2d,
    /// R/W, Default: 0x00
    DTEST2_PAD = 0x32,
    /// R/W, Default: 0x00
    PIXEL_GRAB = 0x35,
    /// R/W, Default: 0x00
    FRAME_GRAB = 0x36,
    /// W, Default: NA
    POWER_UP_RESET = 0x3a,
    /// W, Default: NA
    SHUTDOWN = 0x3b,
    /// R, Default: 0xfe
    NOT_REV_ID = 0x3e,
    /// R, Default: 0xc1
    NOT_PROD_ID = 0x3f,
    /// W, Default: NA
    SPI_CLK_ON_REQ = 0x41,
    /// R/W, Default: 0x00
    PRBS_TEST_CTL = 0x47,
    /// R/W, Default: 0x00
    SPI_PAGE0 = 0x7f,
    /// R/W, Default: 0x86
    RES_STEP = 0x85,
    /// R/W, Default: 0x00
    VCSEL_CTL = 0x9e,
    /// R/W, Default: 0x00
    LSR_CONTROL = 0x9f,
    /// R/W, Default: 0x00
    SPI_PAGE1 = 0xff,
}