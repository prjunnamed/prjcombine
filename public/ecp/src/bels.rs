use prjcombine_interconnect::{bels, db::BelSlotId};

use crate::tslots;

bels![
    INT: tslots::INT,
    SLICE0: tslots::BEL,
    SLICE1: tslots::BEL,
    SLICE2: tslots::BEL,
    SLICE3: tslots::BEL,
    IO0: tslots::IO,
    IO1: tslots::IO,
    IO2: tslots::IO,
    IO3: tslots::IO,
    IO4: tslots::IO,
    IO5: tslots::IO,
    DQS: tslots::BEL,
    DQSDLL: tslots::CLK,
    CIBTEST_SEL: tslots::BEL,
    EBR0: tslots::BEL,
    DSP0: tslots::BEL,
    PLL: tslots::BEL,
    START: tslots::BEL,
    OSC: tslots::BEL,
    JTAG: tslots::BEL,
    RDBK: tslots::BEL,
    GSR: tslots::BEL,
    DCS0: tslots::CLK,
    DCS1: tslots::CLK,
    DCS2: tslots::CLK,
    DCS3: tslots::CLK,
    DCS4: tslots::CLK,
    DCS5: tslots::CLK,
    DCS6: tslots::CLK,
    DCS7: tslots::CLK,
    CLK_ROOT: tslots::CLK,
];

pub const SLICE: [BelSlotId; 4] = [SLICE0, SLICE1, SLICE2, SLICE3];

pub const IO: [BelSlotId; 6] = [IO0, IO1, IO2, IO3, IO4, IO5];

pub const DCS: [BelSlotId; 8] = [DCS0, DCS1, DCS2, DCS3, DCS4, DCS5, DCS6, DCS7];
