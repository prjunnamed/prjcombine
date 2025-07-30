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
    // patched to CLK for XO2
    DQS0: tslots::BEL,
    DQS1: tslots::BEL,
    DQSTEST: tslots::BEL,
    DQSDLL: tslots::BEL,
    DQSDLLTEST: tslots::BEL,
    SERDES: tslots::BEL,
    CIBTEST_SEL: tslots::BEL,
    EBR0: tslots::BEL,
    DSP0: tslots::BEL,
    DSP1: tslots::BEL,
    PLL: tslots::BEL,
    PLLREFCS: tslots::BEL,
    DLL: tslots::BEL,
    // patched to CLK for XO2
    DLLDEL0: tslots::BEL,
    DLLDEL1: tslots::BEL,
    DLLDEL2: tslots::BEL,
    // patched to CLK for XO2
    CLKDIV0: tslots::BEL,
    CLKDIV1: tslots::BEL,
    ECLK_ALT_ROOT: tslots::BEL,
    SPLL: tslots::BEL,
    START: tslots::BEL,
    OSC: tslots::BEL,
    JTAG: tslots::BEL,
    RDBK: tslots::BEL,
    GSR: tslots::BEL,
    TSALL: tslots::BEL,
    SED: tslots::BEL,
    SPIM: tslots::BEL,
    SSPI: tslots::BEL,
    WAKEUP: tslots::BEL,
    STF: tslots::BEL,
    AMBOOT: tslots::BEL,
    PERREG: tslots::BEL,
    PCNTR: tslots::BEL,
    EFB: tslots::BEL,
    ESB: tslots::BEL,
    BCPG: tslots::BC,
    BCINRD: tslots::BC,
    BCLVDSO: tslots::BC,
    BCSLEWRATE: tslots::BC,
    DCC_SW0: tslots::CLK,
    DCC_SW1: tslots::CLK,
    DCC_SW2: tslots::CLK,
    DCC_SW3: tslots::CLK,
    DCC_SW4: tslots::CLK,
    DCC_SW5: tslots::CLK,
    DCC_SE0: tslots::CLK,
    DCC_SE1: tslots::CLK,
    DCC_SE2: tslots::CLK,
    DCC_SE3: tslots::CLK,
    DCC_SE4: tslots::CLK,
    DCC_SE5: tslots::CLK,
    DCC_NW0: tslots::CLK,
    DCC_NW1: tslots::CLK,
    DCC_NW2: tslots::CLK,
    DCC_NW3: tslots::CLK,
    DCC_NW4: tslots::CLK,
    DCC_NW5: tslots::CLK,
    DCC_NE0: tslots::CLK,
    DCC_NE1: tslots::CLK,
    DCC_NE2: tslots::CLK,
    DCC_NE3: tslots::CLK,
    DCC_NE4: tslots::CLK,
    DCC_NE5: tslots::CLK,
    DCS_SW0: tslots::CLK,
    DCS_SW1: tslots::CLK,
    DCS_SE0: tslots::CLK,
    DCS_SE1: tslots::CLK,
    DCS_NW0: tslots::CLK,
    DCS_NW1: tslots::CLK,
    DCS_NE0: tslots::CLK,
    DCS_NE1: tslots::CLK,
    DCC0: tslots::CLK,
    DCC1: tslots::CLK,
    DCC2: tslots::CLK,
    DCC3: tslots::CLK,
    DCC4: tslots::CLK,
    DCC5: tslots::CLK,
    DCC6: tslots::CLK,
    DCC7: tslots::CLK,
    DCM0: tslots::CLK,
    DCM1: tslots::CLK,
    ECLKBRIDGECS0: tslots::CLK,
    ECLKBRIDGECS1: tslots::CLK,
    CLKFBBUF0: tslots::CLK,
    CLKFBBUF1: tslots::CLK,
    CENTEST: tslots::CLK,
    CLK_ROOT: tslots::CLK,
    SCLK_SOURCE: tslots::SCLK_SOURCE,
    PCLK_SOURCE_W: tslots::PCLK_SOURCE,
    PCLK_SOURCE_E: tslots::PCLK_SOURCE,
    PCLK_DCC0: tslots::PCLK_SOURCE,
    PCLK_DCC1: tslots::PCLK_SOURCE,
    ECLK_ROOT: tslots::CLK,
    ECLKSYNC0: tslots::CLK,
    ECLKSYNC1: tslots::CLK,
    ECLK_TAP: tslots::ECLK_TAP,
    HSDCLK_ROOT: tslots::HSDCLK_SPLITTER,
    HSDCLK_SPLITTER: tslots::HSDCLK_SPLITTER,
    TESTIN: tslots::BEL,
    TESTOUT: tslots::BEL,
    DTS: tslots::BEL,
];

pub const SLICE: [BelSlotId; 4] = [SLICE0, SLICE1, SLICE2, SLICE3];

pub const IO: [BelSlotId; 6] = [IO0, IO1, IO2, IO3, IO4, IO5];
pub const DQS: [BelSlotId; 2] = [DQS0, DQS1];

pub const DSP: [BelSlotId; 2] = [DSP0, DSP1];

pub const DCS_SW: [BelSlotId; 2] = [DCS_SW0, DCS_SW1];
pub const DCS_SE: [BelSlotId; 2] = [DCS_SE0, DCS_SE1];
pub const DCS_NW: [BelSlotId; 2] = [DCS_NW0, DCS_NW1];
pub const DCS_NE: [BelSlotId; 2] = [DCS_NE0, DCS_NE1];

pub const DCC_SW: [BelSlotId; 6] = [DCC_SW0, DCC_SW1, DCC_SW2, DCC_SW3, DCC_SW4, DCC_SW5];
pub const DCC_SE: [BelSlotId; 6] = [DCC_SE0, DCC_SE1, DCC_SE2, DCC_SE3, DCC_SE4, DCC_SE5];
pub const DCC_NW: [BelSlotId; 6] = [DCC_NW0, DCC_NW1, DCC_NW2, DCC_NW3, DCC_NW4, DCC_NW5];
pub const DCC_NE: [BelSlotId; 6] = [DCC_NE0, DCC_NE1, DCC_NE2, DCC_NE3, DCC_NE4, DCC_NE5];

pub const DCC: [BelSlotId; 8] = [DCC0, DCC1, DCC2, DCC3, DCC4, DCC5, DCC6, DCC7];
pub const DCM: [BelSlotId; 2] = [DCM0, DCM1];
pub const ECLKBRIDGECS: [BelSlotId; 2] = [ECLKBRIDGECS0, ECLKBRIDGECS1];

pub const PCLK_DCC: [BelSlotId; 2] = [PCLK_DCC0, PCLK_DCC1];

pub const ECLKSYNC: [BelSlotId; 2] = [ECLKSYNC0, ECLKSYNC1];

pub const CLKDIV: [BelSlotId; 2] = [CLKDIV0, CLKDIV1];
pub const DLLDEL: [BelSlotId; 3] = [DLLDEL0, DLLDEL1, DLLDEL2];
pub const CLKFBBUF: [BelSlotId; 2] = [CLKFBBUF0, CLKFBBUF1];
