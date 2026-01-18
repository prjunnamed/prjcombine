use prjcombine_tablegen::target_defs;

target_defs! {
    variant ultrascale;
    variant ultrascaleplus;

    region_slot LEAF;

    // wires common between two halves of the interconnect tile

    wire TIE_0: tie 0;
    wire TIE_1: tie 1;
    wire GCLK[16]: regional LEAF;
    wire GNODE[32]: mux;

    if variant ultrascale {
        wire X4_W0[16]: mux;
        for i in 1..=2 {
            wire "X4_W{i}"[16]: branch LE;
        }
        wire X4_E0[16]: mux;
        for i in 1..=2 {
            wire "X4_E{i}"[16]: branch LW;
        }
        wire X4_E2_S0: branch N;
        wire X4_E2_N15: branch S;
        wire X4_S0[8]: mux;
        for i in 1..=4 {
            wire "X4_S{i}"[8]: branch N;
        }
        wire X5_S0[8]: mux;
        for i in 1..=5 {
            wire "X5_S{i}"[8]: branch N;
        }
        wire X4_N0[8]: mux;
        for i in 1..=4 {
            wire "X4_N{i}"[8]: branch S;
        }
        wire X5_N0[8]: mux;
        for i in 1..=5 {
            wire "X5_N{i}"[8]: branch S;
        }
        wire X5_N5_N7: branch S;

        wire X12_W0[8]: mux;
        for i in 1..=6 {
            wire "X12_W{i}"[8]: branch LE;
        }
        wire X12_W6_N7: branch S;
        wire X12_E0[8]: mux;
        for i in 1..=6 {
            wire "X12_E{i}"[8]: branch LW;
        }
        wire X12_E6_S0: branch N;
        wire X12_S0[4]: mux;
        for i in 1..=12 {
            wire "X12_S{i}"[4]: branch N;
        }
        wire X12_S12_S0: branch N;
        wire X16_S0[4]: mux;
        for i in 1..=16 {
            wire "X16_S{i}"[4]: branch N;
        }
        wire X12_N0[4]: mux;
        for i in 1..=12 {
            wire "X12_N{i}"[4]: branch S;
        }
        wire X16_N0[4]: mux;
        for i in 1..=16 {
            wire "X16_N{i}"[4]: branch S;
        }
        wire X16_N16_N3: branch S;
    } else {
        wire X12_W0[8]: mux;
        for i in 1..=6 {
            wire "X12_W{i}"[8]: branch LE;
        }
        wire X12_E0[8]: mux;
        for i in 1..=6 {
            wire "X12_E{i}"[8]: branch LW;
        }
        wire X12_E6_S0: branch N;
        wire X12_E6_N7: branch S;
        wire X12_S0[8]: mux;
        for i in 1..=12 {
            wire "X12_S{i}"[8]: branch N;
        }
        wire X12_N0[8]: mux;
        for i in 1..=12 {
            wire "X12_N{i}"[8]: branch S;
        }
    }

    // wires belonging to interconnect left/right half-tiles

    wire OUT[32]: bel;
    wire OUT_TMIN[32]: bel;
    wire TEST[4]: test;

    if variant ultrascale {
        wire SDNODE[64]: mux;
        // only a few actually exist
        wire SDNODE_S[64]: branch N;
        wire SDNODE_N[64]: branch S;

        wire X1_W0[8]: mux;
        wire X1_W1[8]: branch E;
        wire X1_E0[8]: mux;
        wire X1_E1[8]: branch W;
        wire X1_E1_W0: branch E;
        wire X1_E1_E0: branch W;
        wire X1_E1_S0: branch N;
        wire X1_S0[8]: mux;
        wire X1_S1[8]: branch N;
        wire X1_S1_S0: branch N;
        wire X1_N0[8]: mux;
        wire X1_N1[8]: branch S;

        wire X2_W0[8]: mux;
        wire X2_W1[8]: branch E;
        wire X2_W2[8]: branch E;
        wire X2_E0[8]: mux;
        wire X2_E1[8]: branch W;
        wire X2_E2[8]: branch W;
        wire X2_E2_W7: branch E;
        wire X2_E2_E7: branch W;
        wire X2_E2_N7: branch S;
        wire X2_S0[8]: mux;
        wire X2_S1[8]: branch N;
        wire X2_S2[8]: branch N;
        wire X2_N0[8]: mux;
        wire X2_N1[8]: branch S;
        wire X2_N2[8]: branch S;
        wire X2_N2_N7: branch S;

        wire QLNODE[64]: mux;
        // only a few actually exist
        wire QLNODE_S[64]: branch N;
        wire QLNODE_N[64]: branch S;

    } else {
        wire SDQNODE[96]: mux;
        // only a few actually exist
        wire SDQNODE_S[96]: branch N;
        wire SDQNODE_N[96]: branch S;

        wire X1_W0[8]: mux;
        wire X1_W1[8]: branch E;
        wire X1_W1_W7: branch E;
        wire X1_W1_E7: branch W;
        wire X1_W1_N7: branch S;
        wire X1_E0[8]: mux;
        wire X1_E1[8]: branch W;
        wire X1_S0[8]: mux;
        wire X1_S1[8]: branch N;
        wire X1_N0[8]: mux;
        wire X1_N1[8]: branch S;

        wire X2_W0[8]: mux;
        wire X2_W1[8]: branch E;
        wire X2_W2[8]: branch E;
        wire X2_W2_W0: branch E;
        wire X2_W2_E0: branch W;
        wire X2_W2_S0: branch N;
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
        wire X4_S4_S0: branch N;
        wire X4_N0[8]: mux;
        for i in 1..=4 {
            wire "X4_N{i}"[8]: branch S;
        }
        wire X4_N4_N7: branch S;
    }

    wire INODE[64]: mux;
    // only a few actually exist
    wire INODE_S[64]: branch N;
    wire INODE_N[64]: branch S;

    wire IMUX_CTRL[10]: mux;
    wire IMUX_BYP[16]: mux;
    // only a few actually exist
    wire IMUX_BYP_S[16]: branch N;
    wire IMUX_BYP_N[16]: branch S;
    if variant ultrascaleplus {
        wire IMUX_BYP_DELAY[16]: mux;
    }
    wire IMUX_IMUX[48]: mux;
    wire IMUX_IMUX_DELAY[48]: mux;
    wire IMUX_RCLK[24]: mux;
    wire INODE_RCLK[24]: mux;
    wire RCLK_GND[24]: tie 0;

    tile_slot INT {
        bel_slot INT: routing;
        tile_class INT {
            cell CELL[2];
        }
    }

    tile_slot INTF {
        bel_slot INTF_DELAY: routing;
        bel_slot INTF_TESTMUX: routing;
        tile_class INTF {
            cell CELL;
        }
        tile_class INTF_DELAY {
            cell CELL;
        }
        // u+ only
        tile_class INTF_IO {
            cell CELL;
        }
    }

    tile_slot BEL {
        bel_slot SLICE: legacy;
        tile_class CLEL {
            cell CELL;
        }
        tile_class CLEM {
            cell CELL;
        }

        bel_slot LAGUNA[4]: legacy;
        bel_slot LAGUNA_EXTRA: legacy;
        bel_slot VCC_LAGUNA: legacy;
        tile_class LAGUNA {
            cell CELL;
        }

        bel_slot BRAM_F: legacy;
        bel_slot BRAM_H[2]: legacy;
        tile_class BRAM {
            cell CELL[5];
        }

        bel_slot DSP[2]: legacy;
        tile_class DSP {
            cell CELL[5];
        }

        // u+ only
        bel_slot URAM[4]: legacy;
        tile_class URAM {
            cell CELL_W[15];
            cell CELL_E[15];
        }

        // u+ only
        bel_slot HDIOB[42]: legacy;
        bel_slot HDIOB_DIFF_IN[21]: legacy;
        bel_slot HDIOLOGIC[42]: legacy;
        bel_slot HDLOGIC_CSSD[4]: legacy;
        bel_slot HDIO_VREF[3]: legacy;
        bel_slot HDIO_BIAS: legacy;
        tile_class HDIO_S {
            cell CELL[30];
        }
        tile_class HDIO_N {
            cell CELL[30];
        }
        tile_class HDIOL_S {
            cell CELL[30];
        }
        tile_class HDIOL_N {
            cell CELL[30];
        }
        tile_class HDIOS {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot LPDDRMC: legacy;
        bel_slot XP5IOB[33]: legacy;
        bel_slot XP5IO_VREF[11]: legacy;
        bel_slot X5PHY_LS[11]: legacy;
        bel_slot X5PHY_HS[11]: legacy;
        bel_slot X5PHY_PLL_SELECT[11]: legacy;
        bel_slot XP5PIO_CMU_ANA: legacy;
        bel_slot XP5PIO_CMU_DIG_TOP: legacy;
        bel_slot ABUS_SWITCH_XP5IO[2]: legacy;
        bel_slot VCC_XP5IO: legacy;
        tile_class XP5IO {
            cell CELL[60];
        }

        bel_slot CFG: legacy;
        bel_slot ABUS_SWITCH_CFG: legacy;
        tile_class CFG, CFG_CSEC, CFG_CSEC_V2 {
            cell CELL[60];
        }

        bel_slot PMV: legacy;
        bel_slot PMV2: legacy;
        bel_slot PMVIOB: legacy;
        bel_slot MTBF3: legacy;
        bel_slot CFGIO: legacy;
        tile_class CFGIO {
            cell CELL[30];
        }

        bel_slot SYSMON: legacy;
        tile_class AMS {
            cell CELL[30];
        }

        // u only
        bel_slot PCIE3: legacy;
        tile_class PCIE {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot PCIE4: legacy;
        tile_class PCIE4 {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot PCIE4C: legacy;
        tile_class PCIE4C {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot PCIE4CE: legacy;
        tile_class PCIE4CE {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        bel_slot CMAC: legacy;
        tile_class CMAC {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        bel_slot ILKN: legacy;
        tile_class ILKN {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot FE: legacy;
        tile_class FE {
            cell CELL[60];
        }

        // u+ only
        bel_slot DFE_A: legacy;
        tile_class DFE_A {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot DFE_B: legacy;
        tile_class DFE_B {
            cell CELL[60];
        }

        // u+ only
        bel_slot DFE_C: legacy;
        tile_class DFE_C {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot DFE_D: legacy;
        tile_class DFE_D {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot DFE_E: legacy;
        tile_class DFE_E {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot DFE_F: legacy;
        tile_class DFE_F {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        // u+ only
        bel_slot DFE_G: legacy;
        tile_class DFE_G {
            cell CELL_W[60];
            cell CELL_E[60];
        }

        bel_slot BUFG_GT[24]: legacy;
        bel_slot BUFG_GT_SYNC[15]: legacy;
        bel_slot ABUS_SWITCH_GT[5]: legacy;
        bel_slot GTH_COMMON: legacy;
        bel_slot GTH_CHANNEL[4]: legacy;
        bel_slot GTY_COMMON: legacy;
        bel_slot GTY_CHANNEL[4]: legacy;
        bel_slot GTF_COMMON: legacy;
        bel_slot GTF_CHANNEL[4]: legacy;
        bel_slot GTM_REFCLK: legacy;
        bel_slot GTM_DUAL: legacy;
        bel_slot HSDAC: legacy;
        bel_slot HSADC: legacy;
        bel_slot RFDAC: legacy;
        bel_slot RFADC: legacy;
        tile_class GTH {
            cell CELL[60];
        }
        tile_class GTY {
            cell CELL[60];
        }
        // u+ only
        tile_class GTF {
            cell CELL[60];
        }
        // u+ only
        tile_class GTM {
            cell CELL[60];
        }
        // u+ only
        tile_class HSDAC {
            cell CELL[60];
        }
        // u+ only
        tile_class HSADC {
            cell CELL[60];
        }
        // u+ only
        tile_class RFDAC {
            cell CELL[60];
        }
        // u+ only
        tile_class RFADC {
            cell CELL[60];
        }

        // u+ only
        bel_slot PS: legacy;
        tile_class PS {
            cell CELL[180];
        }

        // u+ only
        bel_slot VCU: legacy;
        tile_class VCU {
            cell CELL[60];
        }

        // u+ only
        bel_slot BLI_HBM_APB_INTF: legacy;
        bel_slot BLI_HBM_AXI_INTF: legacy;
        tile_class BLI {
            cell CELL[15];
        }

        bel_slot BITSLICE[52]: legacy;
        bel_slot BITSLICE_T[8]: legacy;
        bel_slot BITSLICE_CONTROL[8]: legacy;
        bel_slot PLL_SELECT[8]: legacy;
        bel_slot RIU_OR[4]: legacy;
        bel_slot XIPHY_FEEDTHROUGH[4]: legacy;
        bel_slot RCLK_GT: legacy;
        bel_slot VCC_GT: legacy;
        bel_slot XIPHY_BYTE: legacy;
        tile_class XIPHY {
            if variant ultrascale {
                cell CELL[60];
            } else {
                cell CELL[15];
            }
        }
    }

    tile_slot CMT {
        bel_slot BUFCE_ROW_CMT[24]: legacy;
        bel_slot GCLK_TEST_BUF_CMT[24]: legacy;
        bel_slot BUFGCE[24]: legacy;
        bel_slot BUFGCTRL[8]: legacy;
        bel_slot BUFGCE_DIV[4]: legacy;
        bel_slot PLL[2]: legacy;
        bel_slot PLLXP[2]: legacy;
        bel_slot MMCM: legacy;
        bel_slot CMT: legacy;
        bel_slot CMTXP: legacy;
        bel_slot VCC_CMT: legacy;
        bel_slot ABUS_SWITCH_CMT: legacy;
        bel_slot HBM_REF_CLK[2]: legacy;
        // CMT_HBM, CMTXP: u+ only
        tile_class CMT, CMT_HBM {
            cell CELL[60];
        }
        tile_class CMTXP {
            cell CELL[60];
        }

        // u+ only
        bel_slot ABUS_SWITCH_HBM[8]: legacy;
        tile_class HBM_ABUS_SWITCH {
        }
    }

    tile_slot IOB {
        bel_slot HPIOB[26]: legacy;
        bel_slot HPIOB_DIFF_IN[12]: legacy;
        bel_slot HPIOB_DIFF_OUT[12]: legacy;
        bel_slot HPIOB_DCI[2]: legacy;
        bel_slot HPIO_VREF: legacy;
        bel_slot HPIO_BIAS: legacy;
        tile_class HPIO {
            cell CELL[30];
        }

        // u only
        bel_slot HRIOB[26]: legacy;
        bel_slot HRIOB_DIFF_IN[12]: legacy;
        bel_slot HRIOB_DIFF_OUT[12]: legacy;
        tile_class HRIO {
            cell CELL[30];
        }
    }

    tile_slot RCLK_INT {
        bel_slot RCLK_INT: routing;
        bel_slot BUFCE_LEAF_X16_S: legacy;
        bel_slot BUFCE_LEAF_X16_N: legacy;
        bel_slot BUFCE_LEAF_S[16]: legacy;
        bel_slot BUFCE_LEAF_N[16]: legacy;
        bel_slot RCLK_INT_CLK: legacy;
        tile_class RCLK_INT {
            cell NW, NE, SW, SE;
        }
    }

    tile_slot RCLK_V {
        bel_slot BUFCE_ROW_RCLK[4]: legacy;
        bel_slot GCLK_TEST_BUF_RCLK[4]: legacy;
        bel_slot VBUS_SWITCH[3]: legacy;
        bel_slot VCC_RCLK_V: legacy;
        // LAG is u+ only
        tile_class RCLK_V_SINGLE_CLE, RCLK_V_SINGLE_LAG {
            cell CELL;
        }
        // BRAM is u only
        tile_class RCLK_V_DOUBLE_BRAM, RCLK_V_DOUBLE_DSP {
            cell CELL;
        }
        // u+ only
        tile_class RCLK_V_QUAD_BRAM, RCLK_V_QUAD_URAM {
            cell CELL;
        }

    }

    tile_slot RCLK_SPLITTER {
        bel_slot RCLK_SPLITTER: legacy;
        bel_slot VCC_RCLK_SPLITTER: legacy;
        bel_slot RCLK_HROUTE_SPLITTER: legacy;
        bel_slot VCC_RCLK_HROUTE_SPLITTER: legacy;
        tile_class RCLK_HROUTE_SPLITTER_HARD, RCLK_HROUTE_SPLITTER_CLE {
        }
        tile_class RCLK_SPLITTER {
        }
    }

    tile_slot RCLK_BEL {
        bel_slot HARD_SYNC[4]: legacy;
        tile_class HARD_SYNC {
            cell CELL;
        }

        // u+ only
        bel_slot BUFGCE_HDIO[4]: legacy;
        bel_slot ABUS_SWITCH_HDIO[12]: legacy;
        bel_slot RCLK_HDIO: legacy;
        bel_slot RCLK_HDIOS: legacy;
        bel_slot RCLK_HDIOL: legacy;
        bel_slot VCC_RCLK_HDIO: legacy;
        tile_class RCLK_HDIO {
            cell CELL[60];
        }
        tile_class RCLK_HDIOS {
            cell CELL[60];
        }
        tile_class RCLK_HDIOL {
            cell CELL[60];
        }

        // u+ only
        bel_slot BUFG_PS[24]: legacy;
        bel_slot RCLK_PS: legacy;
        bel_slot VCC_RCLK_PS: legacy;
        tile_class RCLK_PS {
            cell CELL;
        }

        // u+ only
        bel_slot RCLK_XIPHY: legacy;
        bel_slot VCC_RCLK_XIPHY: legacy;
        tile_class RCLK_XIPHY {
        }
    }

    tile_slot RCLK_IOB {
        bel_slot ABUS_SWITCH_HPIO[7]: legacy;
        bel_slot HPIO_ZMATCH: legacy;
        bel_slot HPIO_PRBS: legacy;
        tile_class RCLK_HPIO {
            cell CELL[60];
        }

        // u only
        bel_slot ABUS_SWITCH_HRIO[8]: legacy;
        tile_class RCLK_HRIO {
        }
    }

    connector_slot W {
        opposite E;

        connector_class PASS_W {
            if variant ultrascale {
                pass X1_E1 = X1_E0;
                pass X2_E1 = X2_E0;
                pass X2_E2 = X2_E1;
                pass X1_E1_E0 = X1_E1[0];
                pass X2_E2_E7 = X2_E2[7];
            } else {
                pass X1_E1 = X1_E0;
                pass X2_E1 = X2_E0;
                pass X2_E2 = X2_E1;
                pass X4_E1 = X4_E0;
                pass X4_E2 = X4_E1;
                pass X4_E3 = X4_E2;
                pass X4_E4 = X4_E3;
                pass X1_W1_E7 = X1_W1[7];
                pass X2_W2_E0 = X2_W2[0];
            }
        }
        connector_class IO_W;
        connector_class TERM_W {
            if variant ultrascale {
                reflect X1_E1 = X1_W0;
                // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaaaaaaaaaaaaaaaaaaaa
                reflect X2_E1 = X2_W1;
                reflect X2_E2 = X2_W0;
            } else {
                reflect X1_E1 = X1_W0;
                reflect X2_E1 = X2_W0;
                reflect X2_E2 = X2_W1;
                reflect X4_E1 = X4_W0;
                reflect X4_E2 = X4_W1;
                reflect X4_E3 = X4_W2;
                reflect X4_E4 = X4_W3;
            }
        }
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            if variant ultrascale {
                pass X1_W1 = X1_W0;
                pass X1_E1_W0 = X1_E1[0];
                pass X2_W1 = X2_W0;
                pass X2_W2 = X2_W1;
                pass X2_E2_W7 = X2_E2[7];
            } else {
                pass X1_W1 = X1_W0;
                pass X2_W1 = X2_W0;
                pass X2_W2 = X2_W1;
                pass X4_W1 = X4_W0;
                pass X4_W2 = X4_W1;
                pass X4_W3 = X4_W2;
                pass X4_W4 = X4_W3;
                pass X1_W1_W7 = X1_W1[7];
                pass X2_W2_W0 = X2_W2[0];
            }
        }
        connector_class IO_E;
        connector_class TERM_E {
            if variant ultrascale {
                reflect X1_W1 = X1_E0;
                // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaaaaaaaaaaaaaaaaaaaa
                reflect X2_W1 = X2_E1;
                reflect X2_W2 = X2_E0;
            } else {
                reflect X1_W1 = X1_E0;
                reflect X2_W1 = X2_E0;
                reflect X2_W2 = X2_E1;
                reflect X4_W1 = X4_E0;
                reflect X4_W2 = X4_E1;
                reflect X4_W3 = X4_E2;
                reflect X4_W4 = X4_E3;
            }
        }
    }

    connector_slot LW {
        opposite LE;

        connector_class PASS_LW {
            if variant ultrascale {
                for i in 0..2 {
                    pass "X4_E{i+1}" = "X4_E{i}";
                }
                for i in 0..6 {
                    pass "X12_E{i+1}" = "X12_E{i}";
                }
            } else {
                for i in 0..6 {
                    pass "X12_E{i+1}" = "X12_E{i}";
                }
            }
        }
        connector_class IO_LW;
        connector_class TERM_LW {
            if variant ultrascale {
                for i in 0..2 {
                    reflect "X4_E{i+1}" = "X4_W{i}";
                }
                for i in 0..6 {
                    reflect "X12_E{i+1}" = "X12_W{i}";
                }
            } else {
                for i in 0..6 {
                    reflect "X12_E{i+1}" = "X12_W{i}";
                }
            }
        }
    }

    connector_slot LE {
        opposite LW;

        connector_class PASS_LE {
            if variant ultrascale {
                for i in 0..2 {
                    pass "X4_W{i+1}" = "X4_W{i}";
                }
                for i in 0..6 {
                    pass "X12_W{i+1}" = "X12_W{i}";
                }
            } else {
                for i in 0..6 {
                    pass "X12_W{i+1}" = "X12_W{i}";
                }
            }
        }
        connector_class IO_LE;
        connector_class TERM_LE {
            if variant ultrascale {
                for i in 0..2 {
                    reflect "X4_W{i+1}" = "X4_E{i}";
                }
                for i in 0..6 {
                    reflect "X12_W{i+1}" = "X12_E{i}";
                }
            } else {
                for i in 0..6 {
                    reflect "X12_W{i+1}" = "X12_E{i}";
                }
            }
        }
    }

    connector_slot S {
        opposite N;

        connector_class PASS_S {
            if variant ultrascale {
                for i in 0..4 {
                    pass "X4_N{i+1}" = "X4_N{i}";
                }
                for i in 0..5 {
                    pass "X5_N{i+1}" = "X5_N{i}";
                }
                pass X4_E2_N15 = X4_E2[15];
                pass X5_N5_N7 = X5_N5[7];

                for i in 0..12 {
                    pass "X12_N{i+1}" = "X12_N{i}";
                }
                for i in 0..16 {
                    pass "X16_N{i+1}" = "X16_N{i}";
                }
                pass X12_W6_N7 = X12_W6[7];
                pass X16_N16_N3 = X16_N16[3];
            } else {
                for i in 0..12 {
                    pass "X12_N{i+1}" = "X12_N{i}";
                }
                pass X12_E6_N7 = X12_E6[7];
            }

            if variant ultrascale {
                pass SDNODE_N = SDNODE;

                pass X1_N1 = X1_N0;
                pass X2_N1 = X2_N0;
                pass X2_N2 = X2_N1;
                pass X2_E2_N7 = X2_E2[7];
                pass X2_N2_N7 = X2_N2[7];

                pass QLNODE_N = QLNODE;
            } else {
                pass SDQNODE_N = SDQNODE;

                pass X1_N1 = X1_N0;
                pass X2_N1 = X2_N0;
                pass X2_N2 = X2_N1;
                pass X4_N1 = X4_N0;
                pass X4_N2 = X4_N1;
                pass X4_N3 = X4_N2;
                pass X4_N4 = X4_N3;
                pass X1_W1_N7 = X1_W1[7];
                pass X4_N4_N7 = X4_N4[7];
            }
            pass INODE_N = INODE;
            pass IMUX_BYP_N = IMUX_BYP;
        }
        connector_class TERM_S0;
        connector_class TERM_S1;
    }

    connector_slot N {
        opposite S;

        connector_class PASS_N {
            if variant ultrascale {
                for i in 0..4 {
                    pass "X4_S{i+1}" = "X4_S{i}";
                }
                for i in 0..5 {
                    pass "X5_S{i+1}" = "X5_S{i}";
                }
                pass X4_E2_S0 = X4_E2[0];

                for i in 0..12 {
                    pass "X12_S{i+1}" = "X12_S{i}";
                }
                for i in 0..16 {
                    pass "X16_S{i+1}" = "X16_S{i}";
                }
                pass X12_E6_S0 = X12_E6[0];
                pass X12_S12_S0 = X12_S12[0];
            } else {
                for i in 0..12 {
                    pass "X12_S{i+1}" = "X12_S{i}";
                }
                pass X12_E6_S0 = X12_E6[0];
            }

            if variant ultrascale {
                pass SDNODE_S = SDNODE;

                pass X1_S1 = X1_S0;
                pass X2_S1 = X2_S0;
                pass X2_S2 = X2_S1;
                pass X1_E1_S0 = X1_E1[0];
                pass X1_S1_S0 = X1_S1[0];

                pass QLNODE_S = QLNODE;
            } else {
                pass SDQNODE_S = SDQNODE;

                pass X1_S1 = X1_S0;
                pass X2_S1 = X2_S0;
                pass X2_S2 = X2_S1;
                pass X4_S1 = X4_S0;
                pass X4_S2 = X4_S1;
                pass X4_S3 = X4_S2;
                pass X4_S4 = X4_S3;
                pass X2_W2_S0 = X2_W2[0];
                pass X4_S4_S0 = X4_S4[0];
            }
            pass INODE_S = INODE;
            pass IMUX_BYP_S = IMUX_BYP;
        }
        connector_class TERM_N0;
        connector_class TERM_N1;
    }
}

pub mod wiredata {
    pub mod ultrascale {
        use prjcombine_entity::id::EntityStaticRange;
        use prjcombine_interconnect::db::WireSlotId;

        use crate::defs::ultrascale::wires;

        pub const X4_W: [EntityStaticRange<WireSlotId, 16>; 3] =
            [wires::X4_W0, wires::X4_W1, wires::X4_W2];
        pub const X4_E: [EntityStaticRange<WireSlotId, 16>; 3] =
            [wires::X4_E0, wires::X4_E1, wires::X4_E2];
        pub const X4_S: [EntityStaticRange<WireSlotId, 8>; 5] = [
            wires::X4_S0,
            wires::X4_S1,
            wires::X4_S2,
            wires::X4_S3,
            wires::X4_S4,
        ];
        pub const X5_S: [EntityStaticRange<WireSlotId, 8>; 6] = [
            wires::X5_S0,
            wires::X5_S1,
            wires::X5_S2,
            wires::X5_S3,
            wires::X5_S4,
            wires::X5_S5,
        ];
        pub const X4_N: [EntityStaticRange<WireSlotId, 8>; 5] = [
            wires::X4_N0,
            wires::X4_N1,
            wires::X4_N2,
            wires::X4_N3,
            wires::X4_N4,
        ];
        pub const X5_N: [EntityStaticRange<WireSlotId, 8>; 6] = [
            wires::X5_N0,
            wires::X5_N1,
            wires::X5_N2,
            wires::X5_N3,
            wires::X5_N4,
            wires::X5_N5,
        ];

        pub const X12_W: [EntityStaticRange<WireSlotId, 8>; 7] = [
            wires::X12_W0,
            wires::X12_W1,
            wires::X12_W2,
            wires::X12_W3,
            wires::X12_W4,
            wires::X12_W5,
            wires::X12_W6,
        ];
        pub const X12_E: [EntityStaticRange<WireSlotId, 8>; 7] = [
            wires::X12_E0,
            wires::X12_E1,
            wires::X12_E2,
            wires::X12_E3,
            wires::X12_E4,
            wires::X12_E5,
            wires::X12_E6,
        ];
        pub const X12_S: [EntityStaticRange<WireSlotId, 4>; 13] = [
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
        pub const X16_S: [EntityStaticRange<WireSlotId, 4>; 17] = [
            wires::X16_S0,
            wires::X16_S1,
            wires::X16_S2,
            wires::X16_S3,
            wires::X16_S4,
            wires::X16_S5,
            wires::X16_S6,
            wires::X16_S7,
            wires::X16_S8,
            wires::X16_S9,
            wires::X16_S10,
            wires::X16_S11,
            wires::X16_S12,
            wires::X16_S13,
            wires::X16_S14,
            wires::X16_S15,
            wires::X16_S16,
        ];
        pub const X12_N: [EntityStaticRange<WireSlotId, 4>; 13] = [
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
        pub const X16_N: [EntityStaticRange<WireSlotId, 4>; 17] = [
            wires::X16_N0,
            wires::X16_N1,
            wires::X16_N2,
            wires::X16_N3,
            wires::X16_N4,
            wires::X16_N5,
            wires::X16_N6,
            wires::X16_N7,
            wires::X16_N8,
            wires::X16_N9,
            wires::X16_N10,
            wires::X16_N11,
            wires::X16_N12,
            wires::X16_N13,
            wires::X16_N14,
            wires::X16_N15,
            wires::X16_N16,
        ];
    }
    pub mod ultrascaleplus {
        use prjcombine_entity::id::EntityStaticRange;
        use prjcombine_interconnect::db::WireSlotId;

        use crate::defs::ultrascaleplus::wires;

        pub const X12_W: [EntityStaticRange<WireSlotId, 8>; 7] = [
            wires::X12_W0,
            wires::X12_W1,
            wires::X12_W2,
            wires::X12_W3,
            wires::X12_W4,
            wires::X12_W5,
            wires::X12_W6,
        ];
        pub const X12_E: [EntityStaticRange<WireSlotId, 8>; 7] = [
            wires::X12_E0,
            wires::X12_E1,
            wires::X12_E2,
            wires::X12_E3,
            wires::X12_E4,
            wires::X12_E5,
            wires::X12_E6,
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

        pub const X1_W: [EntityStaticRange<WireSlotId, 8>; 2] = [wires::X1_W0, wires::X1_W1];
        pub const X1_E: [EntityStaticRange<WireSlotId, 8>; 2] = [wires::X1_E0, wires::X1_E1];
        pub const X1_S: [EntityStaticRange<WireSlotId, 8>; 2] = [wires::X1_S0, wires::X1_S1];
        pub const X1_N: [EntityStaticRange<WireSlotId, 8>; 2] = [wires::X1_N0, wires::X1_N1];

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
    }
}
