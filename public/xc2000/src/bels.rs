pub mod xc2000 {
    use prjcombine_interconnect::{bels, db::BelSlotId};

    bels![
        CLB,
        TBUF0,
        TBUF1,
        TBUF0_E,
        TBUF1_E,
        PULLUP_TBUF0,
        PULLUP_TBUF1,
        IO_W0,
        IO_W1,
        IO_E0,
        IO_E1,
        IO_S0,
        IO_S1,
        IO_N0,
        IO_N1,
        CLKIOB,
        BUFG,
        OSC,
    ];

    pub const IO_W: [BelSlotId; 2] = [IO_W0, IO_W1];
    pub const IO_E: [BelSlotId; 2] = [IO_E0, IO_E1];
    pub const IO_S: [BelSlotId; 2] = [IO_S0, IO_S1];
    pub const IO_N: [BelSlotId; 2] = [IO_N0, IO_N1];
}

pub mod xc4000 {
    use prjcombine_interconnect::{bels, db::BelSlotId};

    bels![
        CLB,
        TBUF0,
        TBUF1,
        PULLUP_TBUF0,
        PULLUP_TBUF1,
        PULLUP_TBUF0_W,
        PULLUP_TBUF1_W,
        PULLUP_TBUF0_E,
        PULLUP_TBUF1_E,
        TBUF_SPLITTER0,
        TBUF_SPLITTER1,
        IO0,
        IO1,
        HIO0,
        HIO1,
        HIO2,
        HIO3,
        DEC0,
        DEC1,
        DEC2,
        BUFF,
        PULLUP_DEC0_H,
        PULLUP_DEC1_H,
        PULLUP_DEC2_H,
        PULLUP_DEC3_H,
        PULLUP_DEC0_V,
        PULLUP_DEC1_V,
        PULLUP_DEC2_V,
        PULLUP_DEC3_V,
        PULLUP_DEC0_W,
        PULLUP_DEC1_W,
        PULLUP_DEC2_W,
        PULLUP_DEC3_W,
        PULLUP_DEC0_E,
        PULLUP_DEC1_E,
        PULLUP_DEC2_E,
        PULLUP_DEC3_E,
        PULLUP_DEC0_S,
        PULLUP_DEC1_S,
        PULLUP_DEC2_S,
        PULLUP_DEC3_S,
        PULLUP_DEC0_N,
        PULLUP_DEC1_N,
        PULLUP_DEC2_N,
        PULLUP_DEC3_N,
        BUFGLS_H,
        BUFGLS_V,
        BUFGE_H,
        BUFGE_V,
        BUFG_H,
        BUFG_V,
        CIN,
        COUT,
        STARTUP,
        READCLK,
        UPDATE,
        OSC,
        TDO,
        MD0,
        MD1,
        MD2,
        RDBK,
        BSCAN,
        CLKH,
        CLKC,
        CLKQC,
        CLKQ,
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

    bels![
        LC0, LC1, LC2, LC3, TBUF0, TBUF1, TBUF2, TBUF3, VCC_GND, IO0, IO1, IO2, IO3, BUFR,
        SCANTEST, CIN, COUT, BUFG, CLKIOB, RDBK, STARTUP, BSCAN, OSC, BYPOSC, BSUPD,
    ];

    pub const LC: [BelSlotId; 4] = [LC0, LC1, LC2, LC3];
    pub const TBUF: [BelSlotId; 4] = [TBUF0, TBUF1, TBUF2, TBUF3];
    pub const IO: [BelSlotId; 4] = [IO0, IO1, IO2, IO3];
}
