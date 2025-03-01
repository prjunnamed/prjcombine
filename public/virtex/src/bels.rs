use prjcombine_interconnect::{bels, db::BelSlotId};

bels![
    SLICE0,
    SLICE1,
    TBUF0,
    TBUF1,
    TBUS,
    IO0,
    IO1,
    IO2,
    IO3,
    BRAM,
    CAPTURE,
    STARTUP,
    BSCAN,
    DLL,
    GCLK_IO0,
    GCLK_IO1,
    BUFG0,
    BUFG1,
    IOFB0,
    IOFB1,
    PCILOGIC,
    CLKC,
    GCLKC,
    CLKH,
    BRAM_CLKH,
    CLKV,
    CLKV_BRAM,
    CLKV_BRAM_S,
    CLKV_BRAM_N,
];

pub const SLICE: [BelSlotId; 2] = [SLICE0, SLICE1];
pub const TBUF: [BelSlotId; 2] = [TBUF0, TBUF1];

pub const IO: [BelSlotId; 4] = [IO0, IO1, IO2, IO3];
pub const GCLK_IO: [BelSlotId; 2] = [GCLK_IO0, GCLK_IO1];
pub const BUFG: [BelSlotId; 2] = [BUFG0, BUFG1];
