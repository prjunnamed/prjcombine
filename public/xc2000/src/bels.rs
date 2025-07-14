pub mod xc2000 {
    use prjcombine_interconnect::{bels, db::BelSlotId};

    use crate::tslots;

    bels![
        INT: tslots::MAIN,
        CLB: tslots::MAIN,
        TBUF0: tslots::MAIN,
        TBUF1: tslots::MAIN,
        TBUF0_E: tslots::MAIN,
        TBUF1_E: tslots::MAIN,
        PULLUP_TBUF0: tslots::MAIN,
        PULLUP_TBUF1: tslots::MAIN,
        IO_W0: tslots::MAIN,
        IO_W1: tslots::MAIN,
        IO_E0: tslots::MAIN,
        IO_E1: tslots::MAIN,
        IO_S0: tslots::MAIN,
        IO_S1: tslots::MAIN,
        IO_N0: tslots::MAIN,
        IO_N1: tslots::MAIN,
        CLKIOB: tslots::MAIN,
        BUFG: tslots::MAIN,
        OSC: tslots::MAIN,
        LLH: tslots::EXTRA_COL,
        LLV: tslots::EXTRA_ROW,
    ];

    pub const IO_W: [BelSlotId; 2] = [IO_W0, IO_W1];
    pub const IO_E: [BelSlotId; 2] = [IO_E0, IO_E1];
    pub const IO_S: [BelSlotId; 2] = [IO_S0, IO_S1];
    pub const IO_N: [BelSlotId; 2] = [IO_N0, IO_N1];
}

pub mod xc4000 {
    use prjcombine_interconnect::{bels, db::BelSlotId};

    use crate::tslots;

    bels![
        INT: tslots::MAIN,
        CLB: tslots::MAIN,
        TBUF0: tslots::MAIN,
        TBUF1: tslots::MAIN,
        PULLUP_TBUF0: tslots::MAIN,
        PULLUP_TBUF1: tslots::MAIN,
        PULLUP_TBUF0_W: tslots::EXTRA_COL,
        PULLUP_TBUF1_W: tslots::EXTRA_COL,
        PULLUP_TBUF0_E: tslots::EXTRA_COL,
        PULLUP_TBUF1_E: tslots::EXTRA_COL,
        TBUF_SPLITTER0: tslots::EXTRA_COL,
        TBUF_SPLITTER1: tslots::EXTRA_COL,
        IO0: tslots::MAIN,
        IO1: tslots::MAIN,
        HIO0: tslots::MAIN,
        HIO1: tslots::MAIN,
        HIO2: tslots::MAIN,
        HIO3: tslots::MAIN,
        DEC0: tslots::MAIN,
        DEC1: tslots::MAIN,
        DEC2: tslots::MAIN,
        BUFF: tslots::EXTRA_ROW,
        PULLUP_DEC0_H: tslots::MAIN,
        PULLUP_DEC1_H: tslots::MAIN,
        PULLUP_DEC2_H: tslots::MAIN,
        PULLUP_DEC3_H: tslots::MAIN,
        PULLUP_DEC0_V: tslots::MAIN,
        PULLUP_DEC1_V: tslots::MAIN,
        PULLUP_DEC2_V: tslots::MAIN,
        PULLUP_DEC3_V: tslots::MAIN,
        PULLUP_DEC0_W: tslots::EXTRA_COL,
        PULLUP_DEC1_W: tslots::EXTRA_COL,
        PULLUP_DEC2_W: tslots::EXTRA_COL,
        PULLUP_DEC3_W: tslots::EXTRA_COL,
        PULLUP_DEC0_E: tslots::EXTRA_COL,
        PULLUP_DEC1_E: tslots::EXTRA_COL,
        PULLUP_DEC2_E: tslots::EXTRA_COL,
        PULLUP_DEC3_E: tslots::EXTRA_COL,
        PULLUP_DEC0_S: tslots::EXTRA_ROW,
        PULLUP_DEC1_S: tslots::EXTRA_ROW,
        PULLUP_DEC2_S: tslots::EXTRA_ROW,
        PULLUP_DEC3_S: tslots::EXTRA_ROW,
        PULLUP_DEC0_N: tslots::EXTRA_ROW,
        PULLUP_DEC1_N: tslots::EXTRA_ROW,
        PULLUP_DEC2_N: tslots::EXTRA_ROW,
        PULLUP_DEC3_N: tslots::EXTRA_ROW,
        BUFGLS_H: tslots::MAIN,
        BUFGLS_V: tslots::MAIN,
        BUFGE_H: tslots::MAIN,
        BUFGE_V: tslots::MAIN,
        BUFG_H: tslots::MAIN,
        BUFG_V: tslots::MAIN,
        CIN: tslots::MAIN,
        COUT: tslots::MAIN,
        STARTUP: tslots::MAIN,
        READCLK: tslots::MAIN,
        UPDATE: tslots::MAIN,
        OSC: tslots::MAIN,
        TDO: tslots::MAIN,
        MD0: tslots::MAIN,
        MD1: tslots::MAIN,
        MD2: tslots::MAIN,
        RDBK: tslots::MAIN,
        BSCAN: tslots::MAIN,
        LLH: tslots::EXTRA_COL,
        LLV: tslots::EXTRA_ROW,
        CLKH: tslots::EXTRA_ROW,
        CLKC: tslots::EXTRA_CROSS,
        CLKQC: tslots::EXTRA_CROSS,
        CLKQ: tslots::EXTRA_CROSS,
    ];

    pub const TBUF: [BelSlotId; 2] = [TBUF0, TBUF1];
    pub const PULLUP_TBUF: [BelSlotId; 2] = [PULLUP_TBUF0, PULLUP_TBUF1];
    pub const PULLUP_TBUF_E: [BelSlotId; 2] = [PULLUP_TBUF0_E, PULLUP_TBUF1_E];
    pub const PULLUP_TBUF_W: [BelSlotId; 2] = [PULLUP_TBUF0_W, PULLUP_TBUF1_W];
    pub const IO: [BelSlotId; 2] = [IO0, IO1];
    pub const HIO: [BelSlotId; 4] = [HIO0, HIO1, HIO2, HIO3];
    pub const DEC: [BelSlotId; 3] = [DEC0, DEC1, DEC2];
    pub const PULLUP_DEC_H: [BelSlotId; 4] =
        [PULLUP_DEC0_H, PULLUP_DEC1_H, PULLUP_DEC2_H, PULLUP_DEC3_H];
    pub const PULLUP_DEC_V: [BelSlotId; 4] =
        [PULLUP_DEC0_V, PULLUP_DEC1_V, PULLUP_DEC2_V, PULLUP_DEC3_V];
    pub const PULLUP_DEC_W: [BelSlotId; 4] =
        [PULLUP_DEC0_W, PULLUP_DEC1_W, PULLUP_DEC2_W, PULLUP_DEC3_W];
    pub const PULLUP_DEC_E: [BelSlotId; 4] =
        [PULLUP_DEC0_E, PULLUP_DEC1_E, PULLUP_DEC2_E, PULLUP_DEC3_E];
    pub const PULLUP_DEC_S: [BelSlotId; 4] =
        [PULLUP_DEC0_S, PULLUP_DEC1_S, PULLUP_DEC2_S, PULLUP_DEC3_S];
    pub const PULLUP_DEC_N: [BelSlotId; 4] =
        [PULLUP_DEC0_N, PULLUP_DEC1_N, PULLUP_DEC2_N, PULLUP_DEC3_N];
}

pub mod xc5200 {
    use prjcombine_interconnect::{bels, db::BelSlotId};

    use crate::tslots;

    bels![
        INT: tslots::MAIN,
        LC0: tslots::MAIN,
        LC1: tslots::MAIN,
        LC2: tslots::MAIN,
        LC3: tslots::MAIN,
        TBUF0: tslots::MAIN,
        TBUF1: tslots::MAIN,
        TBUF2: tslots::MAIN,
        TBUF3: tslots::MAIN,
        VCC_GND: tslots::MAIN,
        IO0: tslots::MAIN,
        IO1: tslots::MAIN,
        IO2: tslots::MAIN,
        IO3: tslots::MAIN,
        BUFR: tslots::MAIN,
        SCANTEST: tslots::MAIN,
        CIN: tslots::MAIN,
        COUT: tslots::MAIN,
        BUFG: tslots::MAIN,
        CLKIOB: tslots::MAIN,
        RDBK: tslots::MAIN,
        STARTUP: tslots::MAIN,
        BSCAN: tslots::MAIN,
        OSC: tslots::MAIN,
        BYPOSC: tslots::MAIN,
        BSUPD: tslots::MAIN,
        LLH: tslots::EXTRA_COL,
        LLV: tslots::EXTRA_ROW,
    ];

    pub const LC: [BelSlotId; 4] = [LC0, LC1, LC2, LC3];
    pub const TBUF: [BelSlotId; 4] = [TBUF0, TBUF1, TBUF2, TBUF3];
    pub const IO: [BelSlotId; 4] = [IO0, IO1, IO2, IO3];
}
