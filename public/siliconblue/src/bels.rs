use prjcombine_interconnect::{bels, db::BelSlotId};

bels![
    LC0,
    LC1,
    LC2,
    LC3,
    LC4,
    LC5,
    LC6,
    LC7,
    IO0,
    IO1,
    BRAM,
    IO_LATCH,
    GB_FABRIC,
    GB_OUT,
    WARMBOOT,
    PLL,
    MAC16,
    SPI,
    I2C,
    I2C_FIFO,
    HSOSC,
    LSOSC,
    HFOSC,
    LFOSC,
    LEDD_IP,
    LEDDA_IP,
    IR_IP,
    IO0_I3C,
    IO1_I3C,
    RGB_DRV,
    IR_DRV,
    RGBA_DRV,
    IR400_DRV,
    BARCODE_DRV,
    LED_DRV_CUR,
    SPRAM0,
    SPRAM1,
    FILTER0,
    FILTER1,
    SMCCLK,
];

pub const LC: [BelSlotId; 8] = [LC0, LC1, LC2, LC3, LC4, LC5, LC6, LC7];
pub const IO: [BelSlotId; 2] = [IO0, IO1];
pub const IO_I3C: [BelSlotId; 2] = [IO0_I3C, IO1_I3C];
pub const SPRAM: [BelSlotId; 2] = [SPRAM0, SPRAM1];
pub const FILTER: [BelSlotId; 2] = [FILTER0, FILTER1];
