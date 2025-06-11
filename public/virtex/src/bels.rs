use prjcombine_interconnect::{bels, db::BelSlotId};

use crate::tslots;

bels![
    SLICE0: tslots::MAIN,
    SLICE1: tslots::MAIN,
    TBUF0: tslots::MAIN,
    TBUF1: tslots::MAIN,
    TBUS: tslots::MAIN,
    IO0: tslots::MAIN,
    IO1: tslots::MAIN,
    IO2: tslots::MAIN,
    IO3: tslots::MAIN,
    BRAM: tslots::MAIN,
    CAPTURE: tslots::MAIN,
    STARTUP: tslots::MAIN,
    BSCAN: tslots::MAIN,
    DLL: tslots::DLL,
    GCLK_IO0: tslots::CLKBT,
    GCLK_IO1: tslots::CLKBT,
    BUFG0: tslots::CLKBT,
    BUFG1: tslots::CLKBT,
    IOFB0: tslots::CLKBT,
    IOFB1: tslots::CLKBT,
    PCILOGIC: tslots::PCILOGIC,
    CLKC: tslots::CLKC,
    GCLKC: tslots::CLKC,
    CLKH: tslots::CLKC,
    BRAM_CLKH: tslots::CLKC,
    CLKV: tslots::CLKV,
    CLKV_BRAM: tslots::MAIN,
    CLKV_BRAM_S: tslots::CLKV,
    CLKV_BRAM_N: tslots::CLKV,
];

pub const SLICE: [BelSlotId; 2] = [SLICE0, SLICE1];
pub const TBUF: [BelSlotId; 2] = [TBUF0, TBUF1];

pub const IO: [BelSlotId; 4] = [IO0, IO1, IO2, IO3];
pub const GCLK_IO: [BelSlotId; 2] = [GCLK_IO0, GCLK_IO1];
pub const BUFG: [BelSlotId; 2] = [BUFG0, BUFG1];
