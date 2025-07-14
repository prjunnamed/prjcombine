use prjcombine_interconnect::{bels, db::BelSlotId};

use crate::tslots;

bels![
    INT: tslots::MAIN,
    LC0: tslots::MAIN,
    LC1: tslots::MAIN,
    LC2: tslots::MAIN,
    LC3: tslots::MAIN,
    LC4: tslots::MAIN,
    LC5: tslots::MAIN,
    LC6: tslots::MAIN,
    LC7: tslots::MAIN,
    IO0: tslots::MAIN,
    IO1: tslots::MAIN,
    BRAM: tslots::BEL,
    IO_LATCH: tslots::BEL,
    GB_FABRIC: tslots::BEL,
    GB_ROOT: tslots::GB_ROOT,
    WARMBOOT: tslots::WARMBOOT,
    PLL: tslots::BEL,
    MAC16: tslots::BEL,
    SPI: tslots::BEL,
    I2C: tslots::BEL,
    I2C_FIFO: tslots::BEL,
    HSOSC: tslots::OSC,
    LSOSC: tslots::OSC,
    HFOSC: tslots::OSC,
    LFOSC: tslots::OSC,
    LEDD_IP: tslots::LED_IP,
    LEDDA_IP: tslots::LED_IP,
    IR_IP: tslots::LED_IP,
    IO0_I3C: tslots::BEL,
    IO1_I3C: tslots::BEL,
    RGB_DRV: tslots::LED_DRV,
    IR_DRV: tslots::LED_DRV,
    RGBA_DRV: tslots::LED_DRV,
    IR400_DRV: tslots::LED_DRV,
    BARCODE_DRV: tslots::LED_DRV,
    LED_DRV_CUR: tslots::LED_DRV_CUR,
    SPRAM0: tslots::BEL,
    SPRAM1: tslots::BEL,
    FILTER0: tslots::BEL,
    FILTER1: tslots::BEL,
    SMCCLK: tslots::SMCCLK,
];

pub const LC: [BelSlotId; 8] = [LC0, LC1, LC2, LC3, LC4, LC5, LC6, LC7];
pub const IO: [BelSlotId; 2] = [IO0, IO1];
pub const IO_I3C: [BelSlotId; 2] = [IO0_I3C, IO1_I3C];
pub const SPRAM: [BelSlotId; 2] = [SPRAM0, SPRAM1];
pub const FILTER: [BelSlotId; 2] = [FILTER0, FILTER1];
