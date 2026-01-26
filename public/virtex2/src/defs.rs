use prjcombine_tablegen::target_defs;

target_defs! {
    variant virtex2;
    variant spartan3;

    // TODO: enums and bel classes

    // A set of cells sharing a HCLK row.
    region_slot HCLK;
    // A set of cells sharing HCLK leaf.
    region_slot LEAF;

    if variant virtex2 {
        wire PULLUP: pullup;

        wire GCLK[8]: regional LEAF;
        wire DCM_CLKPAD[8]: bel;

        wire OMUX[16]: mux;
        wire OMUX_S0: branch N;
        wire OMUX_W1: branch E;
        wire OMUX_WS1: branch N;
        wire OMUX_E2: branch W;
        wire OMUX_S2: branch N;
        wire OMUX_S3: branch N;
        wire OMUX_SE3: branch W;
        wire OMUX_S4: branch N;
        wire OMUX_S5: branch N;
        wire OMUX_SW5: branch E;
        wire OMUX_W6: branch E;
        wire OMUX_E7: branch W;
        wire OMUX_ES7: branch N;
        wire OMUX_E8: branch W;
        wire OMUX_EN8: branch S;
        wire OMUX_W9: branch E;
        wire OMUX_N10: branch S;
        wire OMUX_NW10: branch E;
        wire OMUX_N11: branch S;
        wire OMUX_N12: branch S;
        wire OMUX_NE12: branch W;
        wire OMUX_E13: branch W;
        wire OMUX_N13: branch S;
        wire OMUX_W14: branch E;
        wire OMUX_WN14: branch S;
        wire OMUX_N15: branch S;

        wire DBL_W0[10]: mux;
        wire DBL_W1[10]: branch E;
        wire DBL_W2[10]: branch E;
        wire DBL_W2_N[10]: branch S;
        wire DBL_E0[10]: mux;
        wire DBL_E1[10]: branch W;
        wire DBL_E2[10]: branch W;
        wire DBL_E2_S[10]: branch N;
        wire DBL_S0[10]: mux;
        wire DBL_S1[10]: branch N;
        wire DBL_S2[10]: branch N;
        wire DBL_S3[10]: branch N;
        wire DBL_N0[10]: mux;
        wire DBL_N1[10]: branch S;
        wire DBL_N2[10]: branch S;
        wire DBL_N3[10]: branch S;

        wire HEX_W0[10]: mux;
        for i in 1..=6 {
            wire "HEX_W{i}"[10]: branch E;
        }
        wire HEX_W6_N[10]: branch S;
        wire HEX_E0[10]: mux;
        for i in 1..=6 {
            wire "HEX_E{i}"[10]: branch W;
        }
        wire HEX_E6_S[10]: branch N;
        wire HEX_S0[10]: mux;
        for i in 1..=7 {
            wire "HEX_S{i}"[10]: branch N;
        }
        wire HEX_N0[10]: mux;
        for i in 1..=7 {
            wire "HEX_N{i}"[10]: branch S;
        }

        wire LH[24]: multi_branch W;
        wire LV[24]: multi_branch S;

        wire IMUX_CLK[4]: mux;
        wire IMUX_CLK_OPTINV[4]: mux;
        wire IMUX_DCM_CLK[4]: mux;
        wire IMUX_DCM_CLK_OPTINV[4]: mux;
        wire IMUX_SR[4]: mux;
        wire IMUX_SR_OPTINV[4]: mux;
        wire IMUX_CE[4]: mux;
        wire IMUX_CE_OPTINV[4]: mux;
        wire IMUX_TI[2]: mux;
        wire IMUX_TI_OPTINV[2]: mux;
        wire IMUX_TS[2]: mux;
        wire IMUX_TS_OPTINV[2]: mux;

        for i in 1..=5 {
            wire "IMUX_CLB_F{i}"[4]: mux;
        }
        for i in 1..=5 {
            wire "IMUX_CLB_G{i}"[4]: mux;
        }
        wire IMUX_CLB_BX[4]: mux;
        wire IMUX_CLB_BY[4]: mux;

        for i in 0..4 {
            wire "IMUX_G{i}_FAN"[2]: mux;
            wire "IMUX_G{i}_DATA"[8]: mux;
        }
        wire IMUX_IOI_ICLK[4]: mux;
        wire IMUX_IOI_TS1[4]: mux;
        wire IMUX_IOI_TS2[4]: mux;
        wire IMUX_IOI_ICE[4]: mux;
        wire IMUX_IOI_TCE[4]: mux;

        wire IMUX_BRAM_ADDRA[4]: mux;
        for i in 1..=5 {
            wire "IMUX_BRAM_ADDRA_S{i}"[4]: branch N;
        }
        for i in 1..=5 {
            wire "IMUX_BRAM_ADDRA_N{i}"[4]: branch S;
        }
        wire IMUX_BRAM_ADDRB[4]: mux;
        for i in 1..=5 {
            wire "IMUX_BRAM_ADDRB_S{i}"[4]: branch N;
        }
        for i in 1..=5 {
            wire "IMUX_BRAM_ADDRB_N{i}"[4]: branch S;
        }

        wire OUT_FAN[8]: bel;
        wire OUT_FAN_TMIN[8]: bel;
        wire OUT_SEC[24]: bel;
        wire OUT_SEC_TMIN[24]: bel;
        wire OUT_HALF0[18]: bel;
        wire OUT_HALF1[18]: bel;
        wire OUT_TEST[16]: bel;
        wire OUT_TBUS: bel;
        wire OUT_PCI[2]: bel;

        wire IMUX_BUFG_CLK[8]: mux;
        wire IMUX_BUFG_SEL[8]: mux;
        wire OUT_BUFG[8]: bel;
    } else {
        wire PULLUP: pullup;

        wire GCLK[8]: regional LEAF;
        wire DCM_CLKPAD[4]: bel;

        wire OMUX[16]: mux;
        wire OMUX_S0: branch N;
        wire OMUX_W1: branch E;
        wire OMUX_WS1: branch N;
        wire OMUX_E2: branch W;
        wire OMUX_S2: branch N;
        wire OMUX_S3: branch N;
        wire OMUX_SE3: branch W;
        wire OMUX_S4: branch N;
        wire OMUX_S5: branch N;
        wire OMUX_SW5: branch E;
        wire OMUX_W6: branch E;
        wire OMUX_E7: branch W;
        wire OMUX_ES7: branch N;
        wire OMUX_E8: branch W;
        wire OMUX_EN8: branch S;
        wire OMUX_W9: branch E;
        wire OMUX_N9: branch S;
        wire OMUX_N10: branch S;
        wire OMUX_NW10: branch E;
        wire OMUX_N11: branch S;
        wire OMUX_N12: branch S;
        wire OMUX_NE12: branch W;
        wire OMUX_E13: branch W;
        wire OMUX_W14: branch E;
        wire OMUX_WN14: branch S;
        wire OMUX_N15: branch S;

        wire DBL_W0[8]: mux;
        wire DBL_W1[8]: branch E;
        wire DBL_W2[8]: branch E;
        wire DBL_W2_N[8]: branch S;
        wire DBL_E0[8]: mux;
        wire DBL_E1[8]: branch W;
        wire DBL_E2[8]: branch W;
        wire DBL_E2_S[8]: branch N;
        wire DBL_S0[8]: mux;
        wire DBL_S1[8]: branch N;
        wire DBL_S2[8]: branch N;
        wire DBL_S3[8]: branch N;
        wire DBL_N0[8]: mux;
        wire DBL_N1[8]: branch S;
        wire DBL_N2[8]: branch S;
        wire DBL_N3[8]: branch S;

        wire HEX_W0[8]: mux;
        for i in 1..=6 {
            wire "HEX_W{i}"[8]: branch E;
        }
        wire HEX_W6_N[8]: branch S;
        wire HEX_E0[8]: mux;
        for i in 1..=6 {
            wire "HEX_E{i}"[8]: branch W;
        }
        wire HEX_E6_S[8]: branch N;
        wire HEX_S0[8]: mux;
        for i in 1..=7 {
            wire "HEX_S{i}"[8]: branch N;
        }
        wire HEX_N0[8]: mux;
        for i in 1..=7 {
            wire "HEX_N{i}"[8]: branch S;
        }

        wire LH[24]: multi_branch W;
        wire LV[24]: multi_branch S;

        wire IMUX_CLK[4]: mux;
        wire IMUX_CLK_OPTINV[4]: mux;
        wire IMUX_SR[4]: mux;
        wire IMUX_SR_OPTINV[4]: mux;
        wire IMUX_CE[4]: mux;
        wire IMUX_CE_OPTINV[4]: mux;
        wire IMUX_IOCLK[8]: mux;

        wire IMUX_FAN_BX[4]: mux;
        wire IMUX_FAN_BY[4]: mux;
        wire IMUX_FAN_BX_BOUNCE[4]: mux;
        wire IMUX_FAN_BY_BOUNCE[4]: mux;
        wire IMUX_DATA[32]: mux;

        wire OUT_FAN[8]: bel;
        wire OUT_FAN_TMIN[8]: bel;
        wire OUT_SEC[16]: bel;
        wire OUT_SEC_TMIN[16]: bel;
        wire OUT_HALF0[4]: bel;
        wire OUT_HALF1[4]: bel;
        wire OUT_HALF0_TMIN[4]: bel;
        wire OUT_HALF1_TMIN[4]: bel;

        wire IMUX_BUFG_CLK[4]: mux;
        wire IMUX_BUFG_SEL[4]: mux;
        wire OUT_BUFG[4]: bel;
    }

    if variant virtex2 {
        bitrect MAIN = vertical (22, rev 80);
        bitrect CLK = vertical (4, rev 80);
        bitrect CLK_SN = vertical (4, rev 16);
        bitrect HCLK = vertical (22, rev 1);
        bitrect TERM_H = vertical (4, rev 80);
        bitrect TERM_V = vertical (22, rev 12);
        bitrect BRAM_DATA = vertical (64, rev 320);
    } else {
        bitrect MAIN = vertical (19, rev 64);
        bitrect CLK = vertical (1, rev 64);
        bitrect CLK_SN = vertical (1, rev 16);
        bitrect CLK_LL = vertical (2, rev 64);
        bitrect CLK_SN_LL = vertical (2, rev 16);
        bitrect HCLK = vertical (19, rev 1);
        bitrect TERM_H = vertical (2, rev 64);
        bitrect TERM_V_S3 = vertical (19, rev 5);
        bitrect TERM_V_S3A = vertical (19, rev 6);
        bitrect LLV_S = vertical (19, rev 1);
        bitrect LLV_N = vertical (19, rev 2);
        bitrect LLV = vertical (19, rev 3);
        bitrect BRAM_DATA = vertical (76, rev 256);
    }

    tile_slot INT {
        bel_slot INT: routing;
        bel_slot RLL: legacy;
        bel_slot PTE2OMUX[4]: legacy;

        if variant virtex2 {
            tile_class
                INT_CLB,
                INT_IOI,
                INT_IOI_CLK_S, // TODO: merge
                INT_IOI_CLK_N, // TODO: merge
                INT_BRAM,
                INT_DCM_V2,
                INT_DCM_V2P, // TODO: merge (if possible)
                INT_CNR,
                INT_PPC,
                INT_GT_CLKPAD
            {
                cell CELL;
                bitrect MAIN: MAIN;
            }
        } else {
            tile_class
                INT_CLB,
                INT_CLB_FC,
                INT_IOI_S3,
                INT_IOI_FC,
                INT_IOI_S3E, // TODO: merge
                INT_IOI_S3A_WE, // TODO: merge
                INT_IOI_S3A_SN, // TODO: merge
                INT_BRAM_S3,
                INT_BRAM_S3E, // TODO: merge
                INT_BRAM_S3A_03, // do *NOT* merge; evil one without CLK/CE
                INT_BRAM_S3A_12, // TODO: merge
                INT_BRAM_S3ADSP, // TODO: merge
                INT_DCM,
                INT_DCM_S3_DUMMY, // TODO: merge if possible
                INT_DCM_S3E_DUMMY // TODO: merge if possible
            {
                cell CELL;
                bitrect MAIN: MAIN;
            }
        }
    }

    tile_slot INTF {
        bel_slot INTF_TESTMUX: routing;

        if variant virtex2 {
            tile_class
                INTF_GT_S0,
                INTF_GT_S123,
                INTF_GT_S_CLKPAD,
                INTF_GT_N0,
                INTF_GT_N123,
                INTF_GT_N_CLKPAD,
                INTF_PPC
            {
                cell CELL;
                bitrect MAIN: MAIN;
            }
        }
    }

    tile_slot BEL {
        bel_slot SLICE[4]: legacy;
        bel_slot TBUF[2]: legacy;
        bel_slot TBUS: legacy;
        tile_class CLB {
            cell CELL;
            bitrect MAIN: MAIN;
        }

        bel_slot IOI[4]: legacy;
        bel_slot IBUF[4]: legacy;
        bel_slot OBUF[4]: legacy;
        if variant virtex2 {
            tile_class IOI, IOI_CLK_S, IOI_CLK_N { // TODO: possible to merge?
                cell CELL;
                bitrect MAIN: MAIN;
            }
        } else {
            tile_class IOI_S3, IOI_FC, IOI_S3E, IOI_S3A_WE, IOI_S3A_S, IOI_S3A_N {
                cell CELL;
                bitrect MAIN: MAIN;
            }
        }

        bel_slot BRAM: legacy;
        bel_slot MULT: legacy;
        if variant virtex2 {
            tile_class BRAM {
                cell CELL[4];
                bitrect MAIN[4]: MAIN;
                bitrect DATA: BRAM_DATA;
            }
        } else {
            tile_class BRAM_S3, BRAM_S3E, BRAM_S3A, BRAM_S3ADSP {
                cell CELL[4];
                bitrect MAIN[4]: MAIN;
                bitrect DATA: BRAM_DATA;
            }
        }

        bel_slot DSP: legacy;
        bel_slot DSP_TESTMUX: routing;
        if variant spartan3 {
            tile_class DSP {
                cell CELL[4];
                bitrect MAIN[4]: MAIN;
            }
        }

        bel_slot DCM: legacy;
        if variant virtex2 {
            tile_class DCM_V2, DCM_V2P {
                cell CELL;
                bitrect MAIN: MAIN;
                bitrect TERM: TERM_V;
            }
        } else {
            tile_class DCM_S3 {
                cell CELL;
                bitrect MAIN: MAIN;
            }

            tile_class
                DCM_S3E_SW,
                DCM_S3E_SE,
                DCM_S3E_NW,
                DCM_S3E_NE,
                DCM_S3E_WS,
                DCM_S3E_WN,
                DCM_S3E_ES,
                DCM_S3E_EN
            {
                cell CELL;
                bitrect MAIN_C[4]: MAIN;
                bitrect MAIN_S[4]: MAIN;
            }
        }

        bel_slot GT: legacy;
        bel_slot GT10: legacy;
        // TODO: remove
        bel_slot IPAD_RXP: legacy;
        bel_slot IPAD_RXN: legacy;
        bel_slot OPAD_TXP: legacy;
        bel_slot OPAD_TXN: legacy;
        if variant virtex2 {
            tile_class GIGABIT_S, GIGABIT_N {
                cell CELL_IO;
                cell CELL[4];
                bitrect MAIN_IO: MAIN;
                bitrect MAIN[4]: MAIN;
            }
            tile_class GIGABIT10_S, GIGABIT10_N {
                cell CELL_IO;
                cell CELL[8];
                bitrect MAIN_IO: MAIN;
                bitrect MAIN[8]: MAIN;
            }
        }

        bel_slot PPC405: legacy;
        if variant virtex2 {
            tile_class PPC_W, PPC_E {
                cell CELL_W[16];
                cell CELL_E[16];
                cell CELL_S[8];
                cell CELL_N[8];
            }
        }

        bel_slot DCI[2]: legacy;
        bel_slot DCIRESET[2]: legacy;
        bel_slot STARTUP: legacy;
        bel_slot CAPTURE: legacy;
        bel_slot ICAP: legacy;
        bel_slot SPI_ACCESS: legacy;
        bel_slot PMV: legacy;
        bel_slot DNA_PORT: legacy;
        bel_slot BSCAN: legacy;
        bel_slot JTAGPPC: legacy;
        if variant virtex2 {
            tile_class
                CNR_SW_V2,
                CNR_SW_V2P,
                CNR_SE_V2,
                CNR_SE_V2P,
                CNR_NW_V2,
                CNR_NW_V2P,
                CNR_NE_V2,
                CNR_NE_V2P
            {
                cell CELL;
                bitrect TERM_H: TERM_H;
                bitrect TERM_V: TERM_V;
            }
        } else {
            tile_class
                CNR_SW_S3,
                CNR_SW_FC,
                CNR_SW_S3E,
                CNR_SW_S3A,
                CNR_SE_S3,
                CNR_SE_FC,
                CNR_SE_S3E,
                CNR_SE_S3A,
                CNR_NW_S3,
                CNR_NW_FC,
                CNR_NW_S3E,
                CNR_NW_S3A,
                CNR_NE_S3,
                CNR_NE_FC,
                CNR_NE_S3E,
                CNR_NE_S3A
            {
                cell CELL;
                bitrect TERM_H: TERM_H;
            }
        }

        bel_slot DCMCONN_S3E: legacy;
        bel_slot BREFCLK_INT: legacy;
        bel_slot RANDOR_OUT: legacy;
        bel_slot MISR: legacy;
    }

    tile_slot TERM_H {
        bel_slot TERM_W: routing;
        bel_slot TERM_E: routing;
        bel_slot PPC_TERM_W: routing;
        bel_slot PPC_TERM_E: routing;
        bel_slot LLH: routing;

        if variant virtex2 {
            tile_class TERM_W {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class TERM_E {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class PPC_TERM_W {
                cell CELL;
                cell FAR;
                bitrect MAIN: MAIN;
            }
            tile_class PPC_TERM_E {
                cell CELL;
                cell FAR;
                bitrect MAIN: MAIN;
            }
        } else {
            tile_class LLH, LLH_S_S3A, LLH_N_S3A {
                cell W;
                cell E;
                bitrect CLK: CLK_LL;
            }
        }
    }

    tile_slot TERM_V {
        bel_slot TERM_S: routing;
        bel_slot TERM_N: routing;
        bel_slot PPC_TERM_S: routing;
        bel_slot PPC_TERM_N: routing;
        bel_slot LLV: routing;

        if variant virtex2 {
            tile_class TERM_S {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class TERM_N {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class PPC_TERM_S {
                cell CELL;
                cell FAR;
                bitrect MAIN: MAIN;
            }
            tile_class PPC_TERM_N {
                cell CELL;
                cell FAR;
                bitrect MAIN: MAIN;
            }
        } else {
            tile_class LLV_S3E {
                cell S;
                cell N;
                bitrect LLV_S: LLV_S;
                bitrect LLV_N: LLV_N;
            }
            tile_class LLV_S3A {
                cell S;
                cell N;
                bitrect LLV: LLV;
            }
        }
    }

    tile_slot IOB {
        if variant virtex2 {
            tile_class IOB_V2_SW2, IOB_V2_SE2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V;
            }
            tile_class IOB_V2_NW2, IOB_V2_NE2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V;
            }
            tile_class IOB_V2_WS2, IOB_V2_WN2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
            tile_class IOB_V2_ES2, IOB_V2_EN2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }

            tile_class IOB_V2P_SW2, IOB_V2P_SE2, IOB_V2P_SE2_CLK {
                cell CELL[2];
                bitrect TERM[2]: TERM_V;
            }
            tile_class IOB_V2P_SW1, IOB_V2P_SW1_ALT, IOB_V2P_SE1, IOB_V2P_SE1_ALT {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class IOB_V2P_NW2, IOB_V2P_NE2, IOB_V2P_NE2_CLK {
                cell CELL[2];
                bitrect TERM[2]: TERM_V;
            }
            tile_class IOB_V2P_NW1, IOB_V2P_NW1_ALT, IOB_V2P_NE1, IOB_V2P_NE1_ALT {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class IOB_V2P_WS2, IOB_V2P_WN2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
            tile_class IOB_V2P_ES2, IOB_V2P_EN2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
        } else {
            tile_class IOB_S3_W1 {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_S3_E1 {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_S3_S2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3;
            }
            tile_class IOB_S3_N2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3;
            }

            tile_class IOB_FC_W {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_FC_E {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_FC_S {
                cell CELL;
                bitrect TERM: TERM_V_S3;
            }
            tile_class IOB_FC_N {
                cell CELL;
                bitrect TERM: TERM_V_S3;
            }

            tile_class IOB_S3E_W1 {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_S3E_W2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
            tile_class IOB_S3E_W3 {
                cell CELL[3];
                bitrect TERM[3]: TERM_H;
            }
            tile_class IOB_S3E_W4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_H;
            }
            tile_class IOB_S3E_E1 {
                cell CELL;
                bitrect TERM: TERM_H;
            }
            tile_class IOB_S3E_E2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_H;
            }
            tile_class IOB_S3E_E3 {
                cell CELL[3];
                bitrect TERM[3]: TERM_H;
            }
            tile_class IOB_S3E_E4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_H;
            }
            tile_class IOB_S3E_S1 {
                cell CELL;
                bitrect TERM: TERM_V_S3;
            }
            tile_class IOB_S3E_S2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3;
            }
            tile_class IOB_S3E_S3 {
                cell CELL[3];
                bitrect TERM[3]: TERM_V_S3;
            }
            tile_class IOB_S3E_S4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_V_S3;
            }
            tile_class IOB_S3E_N1 {
                cell CELL;
                bitrect TERM: TERM_V_S3;
            }
            tile_class IOB_S3E_N2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3;
            }
            tile_class IOB_S3E_N3 {
                cell CELL[3];
                bitrect TERM[3]: TERM_V_S3;
            }
            tile_class IOB_S3E_N4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_V_S3;
            }

            tile_class IOB_S3A_W4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_H;
            }
            tile_class IOB_S3A_E4 {
                cell CELL[4];
                bitrect TERM[4]: TERM_H;
            }
            tile_class IOB_S3A_S2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3A;
            }
            tile_class IOB_S3A_N2 {
                cell CELL[2];
                bitrect TERM[2]: TERM_V_S3A;
            }
        }
    }

    tile_slot CLK {
        bel_slot CLK_INT: routing;
        bel_slot BUFGMUX[8]: legacy;
        bel_slot PCILOGIC: legacy;
        bel_slot PCILOGICSE: legacy;
        // TODO: remove
        bel_slot VCC: legacy;
        bel_slot GLOBALSIG_S[2]: legacy;
        bel_slot GLOBALSIG_N[2]: legacy;
        bel_slot GLOBALSIG_WE: legacy;
        bel_slot BREFCLK: legacy;
        if variant virtex2 {
            tile_class CLK_S_V2, CLK_S_V2P, CLK_S_V2PX {
                cell CELL[2];
                bitrect MAIN: CLK;
                bitrect TERM: CLK_SN;
            }
            tile_class CLK_N_V2, CLK_N_V2P, CLK_N_V2PX {
                cell CELL[2];
                bitrect MAIN: CLK;
                bitrect TERM: CLK_SN;
            }
        } else {
            tile_class CLK_S_S3, CLK_S_FC, CLK_S_S3E, CLK_S_S3A {
                cell CELL;
                if tile_class CLK_S_S3A {
                    bitrect MAIN: CLK;
                    bitrect TERM: CLK_SN_LL;
                } else {
                    bitrect MAIN: CLK;
                    bitrect TERM: CLK_SN;
                }
            }
            tile_class CLK_N_S3, CLK_N_FC, CLK_N_S3E, CLK_N_S3A {
                cell CELL;
                if tile_class CLK_N_S3A {
                    bitrect MAIN: CLK;
                    bitrect TERM: CLK_SN_LL;
                } else {
                    bitrect MAIN: CLK;
                    bitrect TERM: CLK_SN;
                }
            }
            tile_class CLK_W_S3E, CLK_W_S3A {
                cell CELL[2];
                bitrect MAIN[2]: MAIN;
                bitrect TERM[4]: TERM_H;
            }
            tile_class CLK_E_S3E, CLK_E_S3A {
                cell CELL[2];
                bitrect MAIN[2]: MAIN;
                bitrect TERM[4]: TERM_H;
            }
        }

        bel_slot DCMCONN: legacy;
        bel_slot GLOBALSIG_DSP: legacy;
        bel_slot CLKC: legacy;
        bel_slot CLKC_50A: legacy;
        bel_slot CLKQC: legacy;
        if variant virtex2 {
            tile_class CLKC;
            tile_class DCMCONN_S {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class DCMCONN_N {
                cell CELL;
                bitrect TERM: TERM_V;
            }
            tile_class PCI_W {
                cell CELL[4];
            }
            tile_class PCI_E {
                cell CELL[4];
            }
        } else {
            tile_class CLKC;
            tile_class CLKC_50A {
                bitrect MAIN: CLK_LL;
            }
            tile_class CLKQC_S3 {
                bitrect MAIN: CLK;
            }
            tile_class CLKQC_S3E {
                bitrect MAIN: CLK;
            }
            tile_class DCMCONN_S {
                cell CELL;
            }
            tile_class DCMCONN_N {
                cell CELL;
            }
            tile_class HCLK_DSP;
        }
    }

    tile_slot HROW {
        bel_slot HROW: legacy;
        if variant virtex2 {
            tile_class HROW {
                bitrect CLK[4]: CLK;
            }
            tile_class HROW_S {
                bitrect CLK_S: CLK_SN;
                bitrect CLK[3]: CLK;
            }
            tile_class HROW_N {
                bitrect CLK[3]: CLK;
                bitrect CLK_N: CLK_SN;
            }
        } else {
            tile_class HROW;
        }
    }

    tile_slot HCLK {
        bel_slot HCLK: legacy;
        bel_slot GLOBALSIG: legacy;
        if variant virtex2 {
            tile_class HCLK {
                cell S, N;
                bitrect MAIN: HCLK;
            }
        } else {
            tile_class HCLK, HCLK_S,  HCLK_N, HCLK_UNI, HCLK_UNI_S, HCLK_UNI_N, HCLK_0 {
                cell S, N;
                bitrect MAIN: HCLK;
            }
        }
    }

    tile_slot PCI_CE {
        bel_slot PCI_CE_W: legacy;
        bel_slot PCI_CE_E: legacy;
        bel_slot PCI_CE_S: legacy;
        bel_slot PCI_CE_N: legacy;
        bel_slot PCI_CE_CNR: legacy;
        if variant spartan3 {
            tile_class PCI_CE_W;
            tile_class PCI_CE_E;
            tile_class PCI_CE_S;
            tile_class PCI_CE_N;
            tile_class PCI_CE_CNR;
        }
    }

    tile_slot RANDOR {
        bel_slot RANDOR: legacy;
        if variant spartan3 {
            tile_class RANDOR {
                bitrect MAIN: MAIN;
            }
            tile_class RANDOR_FC {
                bitrect MAIN: MAIN;
            }
            tile_class RANDOR_INIT {
                bitrect MAIN: MAIN;
            }
            tile_class RANDOR_INIT_FC {
                bitrect MAIN: MAIN;
            }
        }
    }

    connector_slot W {
        opposite E;

        connector_class PASS_W {
            pass OMUX_E2 = OMUX[2];
            pass OMUX_SE3 = OMUX_S3;
            pass OMUX_E7 = OMUX[7];
            pass OMUX_E8 = OMUX[8];
            pass OMUX_NE12 = OMUX_N12;
            pass OMUX_E13 = OMUX[13];
            pass DBL_E1 = DBL_E0;
            pass DBL_E2 = DBL_E1;
            pass HEX_E1 = HEX_E0;
            pass HEX_E2 = HEX_E1;
            pass HEX_E3 = HEX_E2;
            pass HEX_E4 = HEX_E3;
            pass HEX_E5 = HEX_E4;
            pass HEX_E6 = HEX_E5;
            pass LH[0] = LH[23];
            for i in 0..23 {
                pass LH[i + 1] = LH[i];
            }
        }
        if variant spartan3 {
            connector_class PASS_W_FC {
                pass OMUX_E2 = OMUX[2];
                pass OMUX_SE3 = OMUX_S3;
                pass OMUX_E7 = OMUX[7];
                pass OMUX_E8 = OMUX[8];
                pass OMUX_NE12 = OMUX_N12;
                pass OMUX_E13 = OMUX[13];
                pass DBL_E1 = DBL_E0;
                pass DBL_E2 = DBL_E1;
                pass HEX_E1 = HEX_E0;
                pass HEX_E2 = HEX_E1;
                pass HEX_E3 = HEX_E2;
                pass HEX_E4 = HEX_E3;
                pass HEX_E5 = HEX_E4;
                pass HEX_E6 = HEX_E5;
                pass LH[0] = LH[11];
                for i in 0..11 {
                    pass LH[i + 1] = LH[i];
                }
            }
        }
        connector_class TERM_W;
        if variant virtex2 {
            connector_class PPC_W;
        } else {
            connector_class LLH_W;
            connector_class LLH_DCM_S3ADSP_W;
            connector_class DSPHOLE_W;
            connector_class HDCM_W;
        }
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            pass OMUX_W1 = OMUX[1];
            pass OMUX_SW5 = OMUX_S5;
            pass OMUX_W6 = OMUX[6];
            pass OMUX_W9 = OMUX[9];
            pass OMUX_NW10 = OMUX_N10;
            pass OMUX_W14 = OMUX[14];
            pass DBL_W1 = DBL_W0;
            pass DBL_W2 = DBL_W1;
            pass HEX_W1 = HEX_W0;
            pass HEX_W2 = HEX_W1;
            pass HEX_W3 = HEX_W2;
            pass HEX_W4 = HEX_W3;
            pass HEX_W5 = HEX_W4;
            pass HEX_W6 = HEX_W5;
        }
        connector_class TERM_E;
        if variant virtex2 {
            connector_class PPC_E;
        } else {
            connector_class LLH_E;
            connector_class LLH_DCM_S3ADSP_E;
            connector_class DSPHOLE_E;
            connector_class HDCM_E;
        }
    }

    connector_slot S {
        opposite N;
        connector_class PASS_S {
            pass OMUX_EN8 = OMUX_E8;
            pass OMUX_N10 = OMUX[10];
            pass OMUX_N11 = OMUX[11];
            pass OMUX_N12 = OMUX[12];
            if variant virtex2 {
                pass OMUX_N13 = OMUX[13];
            } else {
                pass OMUX_N9 = OMUX[9];
            }
            pass OMUX_WN14 = OMUX_W14;
            pass OMUX_N15 = OMUX[15];
            pass DBL_W2_N = DBL_W2;
            pass DBL_N1 = DBL_N0;
            pass DBL_N2 = DBL_N1;
            pass DBL_N3 = DBL_N2;
            pass HEX_W6_N = HEX_W6;
            pass HEX_N1 = HEX_N0;
            pass HEX_N2 = HEX_N1;
            pass HEX_N3 = HEX_N2;
            pass HEX_N4 = HEX_N3;
            pass HEX_N5 = HEX_N4;
            pass HEX_N6 = HEX_N5;
            pass HEX_N7 = HEX_N6;
            for i in 0..23 {
                pass LV[i] = LV[i + 1];
            }
            pass LV[23] = LV[0];
            if variant virtex2 {
                pass IMUX_BRAM_ADDRA_N1 = IMUX_BRAM_ADDRA;
                pass IMUX_BRAM_ADDRA_N2 = IMUX_BRAM_ADDRA_N1;
                pass IMUX_BRAM_ADDRA_N3 = IMUX_BRAM_ADDRA_N2;
                pass IMUX_BRAM_ADDRA_N4 = IMUX_BRAM_ADDRA_N3;
                pass IMUX_BRAM_ADDRB_N1 = IMUX_BRAM_ADDRB;
                pass IMUX_BRAM_ADDRB_N2 = IMUX_BRAM_ADDRB_N1;
                pass IMUX_BRAM_ADDRB_N3 = IMUX_BRAM_ADDRB_N2;
                pass IMUX_BRAM_ADDRB_N4 = IMUX_BRAM_ADDRB_N3;
            }
        }
        if variant spartan3 {
            connector_class PASS_S_FC {
                pass OMUX_EN8 = OMUX_E8;
                pass OMUX_N9 = OMUX[9];
                pass OMUX_N10 = OMUX[10];
                pass OMUX_N11 = OMUX[11];
                pass OMUX_N12 = OMUX[12];
                pass OMUX_WN14 = OMUX_W14;
                pass OMUX_N15 = OMUX[15];
                pass DBL_W2_N = DBL_W2;
                pass DBL_N1 = DBL_N0;
                pass DBL_N2 = DBL_N1;
                pass DBL_N3 = DBL_N2;
                pass HEX_W6_N = HEX_W6;
                pass HEX_N1 = HEX_N0;
                pass HEX_N2 = HEX_N1;
                pass HEX_N3 = HEX_N2;
                pass HEX_N4 = HEX_N3;
                pass HEX_N5 = HEX_N4;
                pass HEX_N6 = HEX_N5;
                pass HEX_N7 = HEX_N6;
                for i in 0..11 {
                    pass LV[i] = LV[i + 1];
                }
                pass LV[11] = LV[0];
            }
        }
        connector_class TERM_S;
        if variant virtex2 {
            connector_class PPC_S;
        } else {
            connector_class BRKH_S3_S;
            connector_class TERM_BRAM_S;
            connector_class LLV_S;
            connector_class LLV_CLK_WE_S3E_S;
            connector_class CLK_WE_S3E_S;
        }
    }

    connector_slot N {
        opposite S;
        connector_class PASS_N {
            pass OMUX_S0 = OMUX[0];
            pass OMUX_WS1 = OMUX_W1;
            pass OMUX_S2 = OMUX[2];
            pass OMUX_S3 = OMUX[3];
            pass OMUX_S4 = OMUX[4];
            pass OMUX_S5 = OMUX[5];
            pass OMUX_ES7 = OMUX_E7;
            pass DBL_E2_S = DBL_E2;
            pass DBL_S1 = DBL_S0;
            pass DBL_S2 = DBL_S1;
            pass DBL_S3 = DBL_S2;
            pass HEX_E6_S = HEX_E6;
            pass HEX_S1 = HEX_S0;
            pass HEX_S2 = HEX_S1;
            pass HEX_S3 = HEX_S2;
            pass HEX_S4 = HEX_S3;
            pass HEX_S5 = HEX_S4;
            pass HEX_S6 = HEX_S5;
            pass HEX_S7 = HEX_S6;
            if variant virtex2 {
                pass IMUX_BRAM_ADDRA_S1 = IMUX_BRAM_ADDRA;
                pass IMUX_BRAM_ADDRA_S2 = IMUX_BRAM_ADDRA_S1;
                pass IMUX_BRAM_ADDRA_S3 = IMUX_BRAM_ADDRA_S2;
                pass IMUX_BRAM_ADDRA_S4 = IMUX_BRAM_ADDRA_S3;
                pass IMUX_BRAM_ADDRB_S1 = IMUX_BRAM_ADDRB;
                pass IMUX_BRAM_ADDRB_S2 = IMUX_BRAM_ADDRB_S1;
                pass IMUX_BRAM_ADDRB_S3 = IMUX_BRAM_ADDRB_S2;
                pass IMUX_BRAM_ADDRB_S4 = IMUX_BRAM_ADDRB_S3;
            }
        }
        connector_class TERM_N;
        if variant virtex2 {
            connector_class PPC_N;
        } else {
            connector_class BRKH_S3_N;
            connector_class TERM_BRAM_N;
            connector_class LLV_N;
            connector_class LLV_CLK_WE_S3E_N;
            connector_class CLK_WE_S3E_N;
        }
    }
}
