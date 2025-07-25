use prjcombine_interconnect::{bels, db::BelSlotId};

use crate::tslots;

bels![
    INT: tslots::INT,
    CLE_BC_INT: tslots::CLE_BC,
    INTF_INT: tslots::INTF,
    INTF_DELAY: tslots::INTF,
    RCLK_INT: tslots::RCLK_INT,
    IRI0: tslots::INTF,
    IRI1: tslots::INTF,
    IRI2: tslots::INTF,
    IRI3: tslots::INTF,
    SLICE0: tslots::BEL,
    SLICE1: tslots::BEL,
    LAGUNA: tslots::CLE_BC,
    BRAM_F: tslots::BEL,
    BRAM_H0: tslots::BEL,
    BRAM_H1: tslots::BEL,
    URAM: tslots::BEL,
    URAM_CAS_DLY: tslots::BEL,
    DSP0: tslots::BEL,
    DSP1: tslots::BEL,
    DSP_CPLX: tslots::BEL,
    PCIE4: tslots::BEL,
    PCIE5: tslots::BEL,
    MRMAC: tslots::BEL,
    SDFEC: tslots::BEL,
    DFE_CFC_BOT: tslots::BEL,
    DFE_CFC_TOP: tslots::BEL,
    DCMAC: tslots::BEL,
    ILKN: tslots::BEL,
    HSC: tslots::BEL,
    HDIOLOGIC0: tslots::BEL,
    HDIOLOGIC1: tslots::BEL,
    HDIOLOGIC2: tslots::BEL,
    HDIOLOGIC3: tslots::BEL,
    HDIOLOGIC4: tslots::BEL,
    HDIOLOGIC5: tslots::BEL,
    HDIOLOGIC6: tslots::BEL,
    HDIOLOGIC7: tslots::BEL,
    HDIOLOGIC8: tslots::BEL,
    HDIOLOGIC9: tslots::BEL,
    HDIOLOGIC10: tslots::BEL,
    HDIOB0: tslots::BEL,
    HDIOB1: tslots::BEL,
    HDIOB2: tslots::BEL,
    HDIOB3: tslots::BEL,
    HDIOB4: tslots::BEL,
    HDIOB5: tslots::BEL,
    HDIOB6: tslots::BEL,
    HDIOB7: tslots::BEL,
    HDIOB8: tslots::BEL,
    HDIOB9: tslots::BEL,
    HDIOB10: tslots::BEL,
    BUFGCE_HDIO0: tslots::BEL,
    BUFGCE_HDIO1: tslots::BEL,
    BUFGCE_HDIO2: tslots::BEL,
    BUFGCE_HDIO3: tslots::BEL,
    DPLL_HDIO: tslots::BEL,
    HDIO_BIAS: tslots::BEL,
    RPI_HD_APB: tslots::BEL,
    HDLOGIC_APB: tslots::BEL,
    VCC_HDIO: tslots::BEL,
    RCLK_HDIO: tslots::RCLK_BEL,
    RCLK_HB_HDIO: tslots::RCLK_BEL,
    RCLK_HDIO_DPLL: tslots::RCLK_BEL,
    VCC_HDIO_DPLL: tslots::BEL,
    VDU: tslots::BEL,
    BFR_B: tslots::BEL,
    SYSMON_SAT_VNOC: tslots::SYSMON_SAT,
    MISR: tslots::BEL,
    VNOC_NSU512: tslots::BEL,
    VNOC_NMU512: tslots::BEL,
    VNOC_NPS_A: tslots::BEL,
    VNOC_NPS_B: tslots::BEL,
    VNOC2_NSU512: tslots::BEL,
    VNOC2_NMU512: tslots::BEL,
    VNOC2_NPS_A: tslots::BEL,
    VNOC2_NPS_B: tslots::BEL,
    VNOC2_SCAN: tslots::BEL,
    VNOC4_NSU512: tslots::BEL,
    VNOC4_NMU512: tslots::BEL,
    VNOC4_NPS_A: tslots::BEL,
    VNOC4_NPS_B: tslots::BEL,
    VNOC4_SCAN: tslots::BEL,
    SYSMON_SAT_GT: tslots::SYSMON_SAT,
    DPLL_GT: tslots::DPLL,
    BUFDIV_LEAF_S0: tslots::RCLK_INTF,
    BUFDIV_LEAF_S1: tslots::RCLK_INTF,
    BUFDIV_LEAF_S2: tslots::RCLK_INTF,
    BUFDIV_LEAF_S3: tslots::RCLK_INTF,
    BUFDIV_LEAF_S4: tslots::RCLK_INTF,
    BUFDIV_LEAF_S5: tslots::RCLK_INTF,
    BUFDIV_LEAF_S6: tslots::RCLK_INTF,
    BUFDIV_LEAF_S7: tslots::RCLK_INTF,
    BUFDIV_LEAF_S8: tslots::RCLK_INTF,
    BUFDIV_LEAF_S9: tslots::RCLK_INTF,
    BUFDIV_LEAF_S10: tslots::RCLK_INTF,
    BUFDIV_LEAF_S11: tslots::RCLK_INTF,
    BUFDIV_LEAF_S12: tslots::RCLK_INTF,
    BUFDIV_LEAF_S13: tslots::RCLK_INTF,
    BUFDIV_LEAF_S14: tslots::RCLK_INTF,
    BUFDIV_LEAF_S15: tslots::RCLK_INTF,
    BUFDIV_LEAF_N0: tslots::RCLK_INTF,
    BUFDIV_LEAF_N1: tslots::RCLK_INTF,
    BUFDIV_LEAF_N2: tslots::RCLK_INTF,
    BUFDIV_LEAF_N3: tslots::RCLK_INTF,
    BUFDIV_LEAF_N4: tslots::RCLK_INTF,
    BUFDIV_LEAF_N5: tslots::RCLK_INTF,
    BUFDIV_LEAF_N6: tslots::RCLK_INTF,
    BUFDIV_LEAF_N7: tslots::RCLK_INTF,
    BUFDIV_LEAF_N8: tslots::RCLK_INTF,
    BUFDIV_LEAF_N9: tslots::RCLK_INTF,
    BUFDIV_LEAF_N10: tslots::RCLK_INTF,
    BUFDIV_LEAF_N11: tslots::RCLK_INTF,
    BUFDIV_LEAF_N12: tslots::RCLK_INTF,
    BUFDIV_LEAF_N13: tslots::RCLK_INTF,
    BUFDIV_LEAF_N14: tslots::RCLK_INTF,
    BUFDIV_LEAF_N15: tslots::RCLK_INTF,
    RCLK_HDISTR_LOC: tslots::RCLK_INTF,
    VCC_RCLK: tslots::RCLK_INTF,
    RCLK_DFX_TEST: tslots::RCLK_BEL,
    GCLK_PD_CLKBUF0: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF1: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF2: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF3: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF4: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF5: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF6: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF7: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF8: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF9: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF10: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF11: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF12: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF13: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF14: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF15: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF16: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF17: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF18: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF19: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF20: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF21: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF22: tslots::RCLK_SPLITTER,
    GCLK_PD_CLKBUF23: tslots::RCLK_SPLITTER,
    RCLK_CLKBUF: tslots::RCLK_SPLITTER,
];

pub const IRI: [BelSlotId; 4] = [IRI0, IRI1, IRI2, IRI3];

pub const SLICE: [BelSlotId; 2] = [SLICE0, SLICE1];

pub const DSP: [BelSlotId; 2] = [DSP0, DSP1];
pub const BRAM_H: [BelSlotId; 2] = [BRAM_H0, BRAM_H1];

pub const HDIOLOGIC: [BelSlotId; 11] = [
    HDIOLOGIC0,
    HDIOLOGIC1,
    HDIOLOGIC2,
    HDIOLOGIC3,
    HDIOLOGIC4,
    HDIOLOGIC5,
    HDIOLOGIC6,
    HDIOLOGIC7,
    HDIOLOGIC8,
    HDIOLOGIC9,
    HDIOLOGIC10,
];

pub const HDIOB: [BelSlotId; 11] = [
    HDIOB0, HDIOB1, HDIOB2, HDIOB3, HDIOB4, HDIOB5, HDIOB6, HDIOB7, HDIOB8, HDIOB9, HDIOB10,
];

pub const BUFGCE_HDIO: [BelSlotId; 4] = [BUFGCE_HDIO0, BUFGCE_HDIO1, BUFGCE_HDIO2, BUFGCE_HDIO3];

pub const BUFDIV_LEAF_S: [BelSlotId; 16] = [
    BUFDIV_LEAF_S0,
    BUFDIV_LEAF_S1,
    BUFDIV_LEAF_S2,
    BUFDIV_LEAF_S3,
    BUFDIV_LEAF_S4,
    BUFDIV_LEAF_S5,
    BUFDIV_LEAF_S6,
    BUFDIV_LEAF_S7,
    BUFDIV_LEAF_S8,
    BUFDIV_LEAF_S9,
    BUFDIV_LEAF_S10,
    BUFDIV_LEAF_S11,
    BUFDIV_LEAF_S12,
    BUFDIV_LEAF_S13,
    BUFDIV_LEAF_S14,
    BUFDIV_LEAF_S15,
];

pub const BUFDIV_LEAF_N: [BelSlotId; 16] = [
    BUFDIV_LEAF_N0,
    BUFDIV_LEAF_N1,
    BUFDIV_LEAF_N2,
    BUFDIV_LEAF_N3,
    BUFDIV_LEAF_N4,
    BUFDIV_LEAF_N5,
    BUFDIV_LEAF_N6,
    BUFDIV_LEAF_N7,
    BUFDIV_LEAF_N8,
    BUFDIV_LEAF_N9,
    BUFDIV_LEAF_N10,
    BUFDIV_LEAF_N11,
    BUFDIV_LEAF_N12,
    BUFDIV_LEAF_N13,
    BUFDIV_LEAF_N14,
    BUFDIV_LEAF_N15,
];

pub const BUFDIV_LEAF: [BelSlotId; 32] = [
    BUFDIV_LEAF_S0,
    BUFDIV_LEAF_S1,
    BUFDIV_LEAF_S2,
    BUFDIV_LEAF_S3,
    BUFDIV_LEAF_S4,
    BUFDIV_LEAF_S5,
    BUFDIV_LEAF_S6,
    BUFDIV_LEAF_S7,
    BUFDIV_LEAF_S8,
    BUFDIV_LEAF_S9,
    BUFDIV_LEAF_S10,
    BUFDIV_LEAF_S11,
    BUFDIV_LEAF_S12,
    BUFDIV_LEAF_S13,
    BUFDIV_LEAF_S14,
    BUFDIV_LEAF_S15,
    BUFDIV_LEAF_N0,
    BUFDIV_LEAF_N1,
    BUFDIV_LEAF_N2,
    BUFDIV_LEAF_N3,
    BUFDIV_LEAF_N4,
    BUFDIV_LEAF_N5,
    BUFDIV_LEAF_N6,
    BUFDIV_LEAF_N7,
    BUFDIV_LEAF_N8,
    BUFDIV_LEAF_N9,
    BUFDIV_LEAF_N10,
    BUFDIV_LEAF_N11,
    BUFDIV_LEAF_N12,
    BUFDIV_LEAF_N13,
    BUFDIV_LEAF_N14,
    BUFDIV_LEAF_N15,
];

pub const GCLK_PD_CLKBUF: [BelSlotId; 24] = [
    GCLK_PD_CLKBUF0,
    GCLK_PD_CLKBUF1,
    GCLK_PD_CLKBUF2,
    GCLK_PD_CLKBUF3,
    GCLK_PD_CLKBUF4,
    GCLK_PD_CLKBUF5,
    GCLK_PD_CLKBUF6,
    GCLK_PD_CLKBUF7,
    GCLK_PD_CLKBUF8,
    GCLK_PD_CLKBUF9,
    GCLK_PD_CLKBUF10,
    GCLK_PD_CLKBUF11,
    GCLK_PD_CLKBUF12,
    GCLK_PD_CLKBUF13,
    GCLK_PD_CLKBUF14,
    GCLK_PD_CLKBUF15,
    GCLK_PD_CLKBUF16,
    GCLK_PD_CLKBUF17,
    GCLK_PD_CLKBUF18,
    GCLK_PD_CLKBUF19,
    GCLK_PD_CLKBUF20,
    GCLK_PD_CLKBUF21,
    GCLK_PD_CLKBUF22,
    GCLK_PD_CLKBUF23,
];
