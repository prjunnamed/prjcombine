use prjcombine_tablegen::target_defs;

target_defs! {
    region_slot LEAF;

    // wires common between two halves of the interconnect tile

    wire TIE_1: tie 1;

    wire X6_W0[8]: mux;
    for i in 1..=3 {
        wire "X6_W{i}"[8]: branch LE;
    }
    wire X6_E0[8]: mux;
    for i in 1..=3 {
        wire "X6_E{i}"[8]: branch LW;
    }
    wire X7_S0[8]: mux;
    for i in 1..=7 {
        wire "X7_S{i}"[8]: branch N;
    }
    wire X7_N0[8]: mux;
    for i in 1..=7 {
        wire "X7_N{i}"[8]: branch S;
    }
    wire X6_E3_S0: branch N;

    wire X10_W0[8]: mux;
    for i in 1..=5 {
        wire "X10_W{i}"[8]: branch LE;
    }
    wire X10_E0[8]: mux;
    for i in 1..=5 {
        wire "X10_E{i}"[8]: branch LW;
    }
    wire X12_S0[8]: mux;
    for i in 1..=12 {
        wire "X12_S{i}"[8]: branch N;
    }
    wire X12_N0[8]: mux;
    for i in 1..=12 {
        wire "X12_N{i}"[8]: branch S;
    }
    wire X10_E5_N7: branch S;
    wire X10_E5_E7: branch W;

    wire IMUX_LAG[6]: mux;
    wire OUT_LAG[6]: bel;

    // wires belonging to interconnect left/right half-tiles

    wire OUT[48]: bel;
    // only a few actually exist
    wire OUT_S[48]: branch N;
    wire OUT_TMIN[48]: bel;
    wire OUT_CLE[48]: branch INTF;

    wire SDQNODE[128]: mux;
    wire SDQNODE_S[128]: branch N;
    wire SDQNODE_N[128]: branch S;

    wire X1_W0[16]: mux;
    wire X1_W1[16]: branch E;
    wire X1_W0_S0: branch N;
    wire X1_E0[16]: mux;
    wire X1_E1[16]: branch W;
    wire X1_S0[16]: mux;
    wire X1_S1[16]: branch N;
    wire X1_N0[16]: mux;
    wire X1_N1[16]: branch S;

    wire X2_W0[8]: mux;
    wire X2_W1[8]: branch E;
    wire X2_W2[8]: branch E;
    wire X2_E0[8]: mux;
    wire X2_E1[8]: branch W;
    wire X2_E2[8]: branch W;
    wire X2_S0[8]: mux;
    wire X2_S1[8]: branch N;
    wire X2_S2[8]: branch N;
    wire X2_N0[8]: mux;
    wire X2_N1[8]: branch S;
    wire X2_N2[8]: branch S;

    wire X4_W0[8]: mux;
    for i in 1..=4 {
        wire "X4_W{i}"[8]: branch E;
    }
    wire X4_E0[8]: mux;
    for i in 1..=4 {
        wire "X4_E{i}"[8]: branch W;
    }
    wire X4_S0[8]: mux;
    for i in 1..=4 {
        wire "X4_S{i}"[8]: branch N;
    }
    wire X4_N0[8]: mux;
    for i in 1..=4 {
        wire "X4_N{i}"[8]: branch S;
    }
    wire X4_E3_S0: branch N;

    wire INODE[128]: mux;
    wire IMUX_IMUX[96]: mux;
    wire IMUX_BOUNCE[32]: mux;
    wire BNODE[64]: branch INTF;

    wire TEST_TMR_DFT: test;

    wire BNODE_CLE[32]: mux;
    wire CNODE_CLE[12]: mux;
    wire IMUX_CLE_CTRL[13]: mux;
    for i in 0..4 {
        wire "IMUX_BLI_CLE_IRI{i}_FAKE_CE"[4]: tie 0;
    }
    wire GCLK_CLE[16]: regional LEAF;

    for i in 0..4 {
        wire "IMUX_INTF_IRI{i}_CLK": mux;
        wire "IMUX_INTF_IRI{i}_RST": mux;
        wire "IMUX_INTF_IRI{i}_CE"[4]: mux;
    }
    wire CNODE_INTF[24]: mux;
    wire GCLK_INTF[16]: regional LEAF;

    wire INODE_RCLK[40]: mux;
    wire IMUX_RCLK[40]: mux;

    for i in 0..4 {
        wire "IRI{i}_CLK": mux;
        wire "IRI{i}_RST": mux;
        wire "IRI{i}_CE"[4]: mux;
        wire "IRI{i}_IMUX"[24]: mux;
        wire "IRI{i}_IMUX_DELAY"[24]: mux;
    }

    tile_slot INT {
        bel_slot INT: routing;

        tile_class INT {
            cell CELL[2];
        }
    }

    tile_slot CLE_BC {
        bel_slot CLE_BC_INT: routing;
        bel_slot LAGUNA: legacy;

        tile_class CLE_BC, CLE_BC_SLL, CLE_BC_SLL2 {
            cell CELL[2];
        }
    }

    tile_slot INTF {
        bel_slot INTF_INT: routing;
        bel_slot INTF_TESTMUX: routing;
        bel_slot INTF_DELAY: routing;
        bel_slot IRI[4]: legacy;

        tile_class INTF_W, INTF_W_HB, INTF_W_HDIO, INTF_W_PSS {
            cell CELL;
        }
        tile_class INTF_W_TERM_PSS, INTF_W_TERM_GT {
            cell CELL;
        }
        tile_class INTF_E, INTF_E_HB, INTF_E_HDIO {
            cell CELL;
        }
        tile_class INTF_E_TERM_GT {
            cell CELL;
        }
        tile_class
            INTF_BLI_CLE_W_S0, INTF_BLI_CLE_W_S1, INTF_BLI_CLE_W_S2, INTF_BLI_CLE_W_S3,
            INTF_BLI_CLE_W_N0, INTF_BLI_CLE_W_N1, INTF_BLI_CLE_W_N2, INTF_BLI_CLE_W_N3
        {
            cell CELL;
        }
        tile_class
            INTF_BLI_CLE_E_S0, INTF_BLI_CLE_E_S1, INTF_BLI_CLE_E_S2, INTF_BLI_CLE_E_S3,
            INTF_BLI_CLE_E_N0, INTF_BLI_CLE_E_N1, INTF_BLI_CLE_E_N2, INTF_BLI_CLE_E_N3
        {
            cell CELL;
        }
    }

    tile_slot BEL {
        bel_slot SLICE[2]: legacy;
        tile_class CLE_W, CLE_W_VR {
            cell CELL[2];
        }
        tile_class CLE_E, CLE_E_VR {
            cell CELL;
        }

        bel_slot BRAM_F: legacy;
        bel_slot BRAM_H[2]: legacy;
        tile_class BRAM_W, BRAM_E {
            cell CELL[4];
        }

        bel_slot URAM: legacy;
        bel_slot URAM_CAS_DLY: legacy;
        tile_class URAM, URAM_DELAY {
            cell CELL[4];
        }

        bel_slot DSP[2]: legacy;
        bel_slot DSP_CPLX: legacy;
        tile_class DSP {
            cell CELL_W[2];
            cell CELL_E[2];
        }

        bel_slot PCIE4: legacy;
        tile_class PCIE4 {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot PCIE5: legacy;
        tile_class PCIE5 {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot MRMAC: legacy;
        tile_class MRMAC {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot SDFEC: legacy;
        tile_class SDFEC {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot DFE_CFC_S: legacy;
        tile_class DFE_CFC_S {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot DFE_CFC_N: legacy;
        tile_class DFE_CFC_N {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot DCMAC: legacy;
        tile_class DCMAC {
            cell CELL_W[96];
            cell CELL_E[96];
        }

        bel_slot ILKN: legacy;
        tile_class ILKN {
            cell CELL_W[96];
            cell CELL_E[96];
        }

        bel_slot HSC: legacy;
        tile_class HSC {
            cell CELL_W[96];
            cell CELL_E[96];
        }

        bel_slot HDIOLOGIC[11]: legacy;
        bel_slot HDIOB[11]: legacy;
        bel_slot BUFGCE_HDIO[4]: legacy;
        bel_slot DPLL_HDIO: legacy;
        bel_slot HDIO_BIAS: legacy;
        bel_slot RPI_HD_APB: legacy;
        bel_slot HDLOGIC_APB: legacy;
        bel_slot VCC_HDIO: legacy;
        bel_slot VCC_HDIO_DPLL: legacy;
        tile_class HDIO {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot MISR: legacy;
        tile_class MISR {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot VNOC_NSU512: legacy;
        bel_slot VNOC_NMU512: legacy;
        bel_slot VNOC_NPS_A: legacy;
        bel_slot VNOC_NPS_B: legacy;
        tile_class VNOC {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot VNOC2_NSU512: legacy;
        bel_slot VNOC2_NMU512: legacy;
        bel_slot VNOC2_NPS_A: legacy;
        bel_slot VNOC2_NPS_B: legacy;
        bel_slot VNOC2_SCAN: legacy;
        tile_class VNOC2 {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot VNOC4_NSU512: legacy;
        bel_slot VNOC4_NMU512: legacy;
        bel_slot VNOC4_NPS_A: legacy;
        bel_slot VNOC4_NPS_B: legacy;
        bel_slot VNOC4_SCAN: legacy;
        tile_class VNOC4 {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot VDU: legacy;
        tile_class VDU_E {
            cell CELL[48];
        }

        bel_slot BFR_B: legacy;
        tile_class BFR_B_E {
            cell CELL[48];
        }
    }

    tile_slot SYSMON_SAT {
        bel_slot SYSMON_SAT_VNOC: legacy;
        tile_class SYSMON_SAT_VNOC {
            cell CELL_W[48];
            cell CELL_E[48];
        }

        bel_slot SYSMON_SAT_GT: legacy;
        tile_class SYSMON_SAT_GT_W, SYSMON_SAT_GT_E {
            cell CELL[48];
        }
    }

    tile_slot DPLL {
        bel_slot DPLL_GT: legacy;

        tile_class DPLL_GT_W, DPLL_GT_E {
            cell CELL[48];
        }
    }

    tile_slot RCLK_INT {
        bel_slot RCLK_INT: routing;

        tile_class RCLK {
            cell CELL[2];
        }
    }

    tile_slot RCLK_INTF {
        bel_slot BUFDIV_LEAF[32]: legacy;
        bel_slot RCLK_HDISTR_LOC: legacy;
        bel_slot VCC_RCLK: legacy;

        tile_class RCLK_INTF_W {
            cell N, S;
        }
        tile_class RCLK_INTF_E {
            cell N, S;
        }
        tile_class RCLK_INTF_W_HALF {
            cell N;
        }
        tile_class RCLK_INTF_E_HALF {
            cell N;
        }

        tile_class RCLK_CLE {
            cell N, S;
        }
        tile_class RCLK_CLE_HALF {
            cell N;
        }
    }

    tile_slot RCLK_BEL {
        bel_slot RCLK_DFX_TEST: legacy;
        tile_class RCLK_DFX_W, RCLK_DFX_E {
            cell CELL;
        }

        bel_slot RCLK_HDIO: legacy;
        bel_slot RCLK_HB_HDIO: legacy;
        bel_slot RCLK_HDIO_DPLL: legacy;
        tile_class RCLK_HDIO, RCLK_HB_HDIO {
        }
    }

    tile_slot RCLK_SPLITTER {
        bel_slot GCLK_PD_CLKBUF[24]: legacy;
        bel_slot RCLK_CLKBUF: legacy;

        tile_class RCLK_CLKBUF, RCLK_CLKBUF_VR, RCLK_CLKBUF_NOPD, RCLK_CLKBUF_NOPD_VR {
            cell CELL;
        }
    }

    connector_slot W {
        opposite E;
        connector_class PASS_W {
            pass X1_E1 = X1_E0;
            pass X2_E1 = X2_E0;
            pass X2_E2 = X2_E1;
            pass X4_E1 = X4_E0;
            pass X4_E2 = X4_E1;
            pass X4_E3 = X4_E2;
            pass X4_E4 = X4_E3;
            pass X10_E5_E7 = X10_E5[7];
        }
        connector_class TERM_W {
            reflect X1_E1 = X1_W0;
            reflect X2_E1 = X2_W0;
            reflect X2_E2 = X2_W1;
            reflect X4_E1 = X4_W0;
            reflect X4_E2 = X4_W1;
            reflect X4_E3 = X4_W2;
            reflect X4_E4 = X4_W3;
        }
    }
    connector_slot E {
        opposite W;
        connector_class PASS_E {
            pass X1_W1 = X1_W0;
            pass X2_W1 = X2_W0;
            pass X2_W2 = X2_W1;
            pass X4_W1 = X4_W0;
            pass X4_W2 = X4_W1;
            pass X4_W3 = X4_W2;
            pass X4_W4 = X4_W3;
        }
        connector_class TERM_E {
            reflect X1_W1 = X1_E0;
            reflect X2_W1 = X2_E0;
            reflect X2_W2 = X2_E1;
            reflect X4_W1 = X4_E0;
            reflect X4_W2 = X4_E1;
            reflect X4_W3 = X4_E2;
            reflect X4_W4 = X4_E3;
        }
    }
    connector_slot LW {
        opposite LE;
        connector_class PASS_LW {
            for i in 0..3 {
                pass "X6_E{i+1}" = "X6_E{i}";
            }
            for i in 0..5 {
                pass "X10_E{i+1}" = "X10_E{i}";
            }
        }
        connector_class TERM_LW {
            for i in 0..3 {
                reflect "X6_E{i+1}" = "X6_W{i}";
            }
            for i in 0..5 {
                reflect "X10_E{i+1}" = "X10_W{i}";
            }
        }
    }
    connector_slot LE {
        opposite LW;
        connector_class PASS_LE {
            for i in 0..3 {
                pass "X6_W{i+1}" = "X6_W{i}";
            }
            for i in 0..5 {
                pass "X10_W{i+1}" = "X10_W{i}";
            }
        }
        connector_class TERM_LE {
            for i in 0..3 {
                reflect "X6_W{i+1}" = "X6_E{i}";
            }
            for i in 0..5 {
                reflect "X10_W{i+1}" = "X10_E{i}";
            }
        }
    }
    connector_slot S {
        opposite N;
        connector_class PASS_S {
            pass SDQNODE_N = SDQNODE;
            pass X1_N1 = X1_N0;
            pass X2_N1 = X2_N0;
            pass X2_N2 = X2_N1;
            pass X4_N1 = X4_N0;
            pass X4_N2 = X4_N1;
            pass X4_N3 = X4_N2;
            pass X4_N4 = X4_N3;
            for i in 0..7 {
                pass "X7_N{i+1}" = "X7_N{i}";
            }
            for i in 0..12 {
                pass "X12_N{i+1}" = "X12_N{i}";
            }
            pass X10_E5_N7 = X10_E5[7];
        }
        connector_class TERM_S {
            reflect X1_N1 = X1_S0;
            reflect X2_N1 = X2_S0;
            reflect X2_N2 = X2_S1;
            reflect X4_N1 = X4_S0;
            reflect X4_N2 = X4_S1;
            reflect X4_N3 = X4_S2;
            reflect X4_N4 = X4_S3;
            for i in 0..7 {
                reflect "X7_N{i+1}" = "X7_S{i}";
            }
            for i in 0..12 {
                reflect "X12_N{i+1}" = "X12_S{i}";
            }
            reflect X10_E5_N7 = X6_E3[0];
            reflect SDQNODE_N[29] = SDQNODE[0];
            reflect SDQNODE_N[31] = SDQNODE[2];
            reflect SDQNODE_N[95] = SDQNODE[32];
            reflect SDQNODE_N[127] = SDQNODE[96];
        }
    }
    connector_slot N {
        opposite S;
        connector_class PASS_N {
            pass OUT_S = OUT;
            pass SDQNODE_S = SDQNODE;
            pass X1_S1 = X1_S0;
            pass X1_W0_S0 = X1_W0[0];
            pass X2_S1 = X2_S0;
            pass X2_S2 = X2_S1;
            pass X4_S1 = X4_S0;
            pass X4_S2 = X4_S1;
            pass X4_S3 = X4_S2;
            pass X4_S4 = X4_S3;
            pass X4_E3_S0 = X4_E3[0];
            for i in 0..7 {
                pass "X7_S{i+1}" = "X7_S{i}";
            }
            for i in 0..12 {
                pass "X12_S{i+1}" = "X12_S{i}";
            }
            pass X6_E3_S0 = X6_E3[0];
        }
        connector_class TERM_N {
            reflect X1_S1 = X1_N0;
            reflect X2_S1 = X2_N0;
            reflect X2_S2 = X2_N1;
            reflect X4_S1 = X4_N0;
            reflect X4_S2 = X4_N1;
            reflect X4_S3 = X4_N2;
            reflect X4_S4 = X4_N3;
            for i in 0..7 {
                reflect "X7_S{i+1}" = "X7_N{i}";
            }
            for i in 0..12 {
                reflect "X12_S{i+1}" = "X12_N{i}";
            }
            reflect X6_E3_S0 = X10_E5[7];
            reflect OUT_S[1] = X10_E5_E7;
            reflect SDQNODE_S[0] = SDQNODE[29];
            reflect SDQNODE_S[2] = SDQNODE[31];
            reflect SDQNODE_S[32] = SDQNODE[95];
            reflect SDQNODE_S[96] = SDQNODE[127];
            reflect SDQNODE_S[98] = SDQNODE[127];
        }
    }
    connector_slot INTF {
        opposite INTF;
        connector_class CLE_W, CLE_BLI_W {
            if connector_class CLE_W {
                reflect OUT_CLE = OUT;
            }
        }
        connector_class CLE_E, CLE_BLI_E {
            if connector_class CLE_E {
                reflect OUT_CLE = OUT;
            }
        }
    }
}

pub mod wiredata {
    use prjcombine_entity::id::EntityStaticRange;
    use prjcombine_interconnect::db::WireSlotId;

    use crate::defs::wires;

    pub const X1_W: [EntityStaticRange<WireSlotId, 16>; 2] = [wires::X1_W0, wires::X1_W1];
    pub const X1_E: [EntityStaticRange<WireSlotId, 16>; 2] = [wires::X1_E0, wires::X1_E1];
    pub const X1_S: [EntityStaticRange<WireSlotId, 16>; 2] = [wires::X1_S0, wires::X1_S1];
    pub const X1_N: [EntityStaticRange<WireSlotId, 16>; 2] = [wires::X1_N0, wires::X1_N1];

    pub const X2_W: [EntityStaticRange<WireSlotId, 8>; 3] =
        [wires::X2_W0, wires::X2_W1, wires::X2_W2];
    pub const X2_E: [EntityStaticRange<WireSlotId, 8>; 3] =
        [wires::X2_E0, wires::X2_E1, wires::X2_E2];
    pub const X2_S: [EntityStaticRange<WireSlotId, 8>; 3] =
        [wires::X2_S0, wires::X2_S1, wires::X2_S2];
    pub const X2_N: [EntityStaticRange<WireSlotId, 8>; 3] =
        [wires::X2_N0, wires::X2_N1, wires::X2_N2];

    pub const X4_W: [EntityStaticRange<WireSlotId, 8>; 5] = [
        wires::X4_W0,
        wires::X4_W1,
        wires::X4_W2,
        wires::X4_W3,
        wires::X4_W4,
    ];
    pub const X4_E: [EntityStaticRange<WireSlotId, 8>; 5] = [
        wires::X4_E0,
        wires::X4_E1,
        wires::X4_E2,
        wires::X4_E3,
        wires::X4_E4,
    ];
    pub const X4_S: [EntityStaticRange<WireSlotId, 8>; 5] = [
        wires::X4_S0,
        wires::X4_S1,
        wires::X4_S2,
        wires::X4_S3,
        wires::X4_S4,
    ];
    pub const X4_N: [EntityStaticRange<WireSlotId, 8>; 5] = [
        wires::X4_N0,
        wires::X4_N1,
        wires::X4_N2,
        wires::X4_N3,
        wires::X4_N4,
    ];

    pub const X6_W: [EntityStaticRange<WireSlotId, 8>; 4] =
        [wires::X6_W0, wires::X6_W1, wires::X6_W2, wires::X6_W3];
    pub const X6_E: [EntityStaticRange<WireSlotId, 8>; 4] =
        [wires::X6_E0, wires::X6_E1, wires::X6_E2, wires::X6_E3];
    pub const X7_S: [EntityStaticRange<WireSlotId, 8>; 8] = [
        wires::X7_S0,
        wires::X7_S1,
        wires::X7_S2,
        wires::X7_S3,
        wires::X7_S4,
        wires::X7_S5,
        wires::X7_S6,
        wires::X7_S7,
    ];
    pub const X7_N: [EntityStaticRange<WireSlotId, 8>; 8] = [
        wires::X7_N0,
        wires::X7_N1,
        wires::X7_N2,
        wires::X7_N3,
        wires::X7_N4,
        wires::X7_N5,
        wires::X7_N6,
        wires::X7_N7,
    ];
    pub const X10_W: [EntityStaticRange<WireSlotId, 8>; 6] = [
        wires::X10_W0,
        wires::X10_W1,
        wires::X10_W2,
        wires::X10_W3,
        wires::X10_W4,
        wires::X10_W5,
    ];
    pub const X10_E: [EntityStaticRange<WireSlotId, 8>; 6] = [
        wires::X10_E0,
        wires::X10_E1,
        wires::X10_E2,
        wires::X10_E3,
        wires::X10_E4,
        wires::X10_E5,
    ];

    pub const X12_S: [EntityStaticRange<WireSlotId, 8>; 13] = [
        wires::X12_S0,
        wires::X12_S1,
        wires::X12_S2,
        wires::X12_S3,
        wires::X12_S4,
        wires::X12_S5,
        wires::X12_S6,
        wires::X12_S7,
        wires::X12_S8,
        wires::X12_S9,
        wires::X12_S10,
        wires::X12_S11,
        wires::X12_S12,
    ];
    pub const X12_N: [EntityStaticRange<WireSlotId, 8>; 13] = [
        wires::X12_N0,
        wires::X12_N1,
        wires::X12_N2,
        wires::X12_N3,
        wires::X12_N4,
        wires::X12_N5,
        wires::X12_N6,
        wires::X12_N7,
        wires::X12_N8,
        wires::X12_N9,
        wires::X12_N10,
        wires::X12_N11,
        wires::X12_N12,
    ];

    pub const IRI_CLK: [WireSlotId; 4] = [
        wires::IRI0_CLK,
        wires::IRI1_CLK,
        wires::IRI2_CLK,
        wires::IRI3_CLK,
    ];
    pub const IRI_RST: [WireSlotId; 4] = [
        wires::IRI0_RST,
        wires::IRI1_RST,
        wires::IRI2_RST,
        wires::IRI3_RST,
    ];
    pub const IRI_CE: [EntityStaticRange<WireSlotId, 4>; 4] = [
        wires::IRI0_CE,
        wires::IRI1_CE,
        wires::IRI2_CE,
        wires::IRI3_CE,
    ];
    pub const IRI_IMUX: [EntityStaticRange<WireSlotId, 24>; 4] = [
        wires::IRI0_IMUX,
        wires::IRI1_IMUX,
        wires::IRI2_IMUX,
        wires::IRI3_IMUX,
    ];
    pub const IRI_IMUX_DELAY: [EntityStaticRange<WireSlotId, 24>; 4] = [
        wires::IRI0_IMUX_DELAY,
        wires::IRI1_IMUX_DELAY,
        wires::IRI2_IMUX_DELAY,
        wires::IRI3_IMUX_DELAY,
    ];
}
