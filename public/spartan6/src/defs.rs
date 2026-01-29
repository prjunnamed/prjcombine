use prjcombine_tablegen::target_defs;

target_defs! {

    enum BRAM_RSTTYPE { SYNC, ASYNC }

    enum DSP_B_INPUT { DIRECT, CASCADE }
    enum DSP_CARRYINSEL { CARRYIN, OPMODE5 }
    bel_class DSP {
        input A[18];
        input B[18];
        input C[48];
        input D[18];
        input OPMODE[8];
        input CLK;
        input CEA, CEB, CEC, CED, CEOPMODE, CECARRYIN, CEM, CEP;
        input RSTA, RSTB, RSTC, RSTD, RSTOPMODE, RSTCARRYIN, RSTM, RSTP;
        output M[36];
        output P[48];
        output CARRYOUTF;

        attribute B_INPUT: DSP_B_INPUT;
        attribute CARRYINSEL: DSP_CARRYINSEL;
        attribute A0REG: bool;
        attribute A1REG: bool;
        attribute B0REG: bool;
        attribute B1REG: bool;
        attribute CREG: bool;
        attribute DREG: bool;
        attribute MREG: bool;
        attribute PREG: bool;
        attribute OPMODEREG: bool;
        attribute CARRYINREG: bool;
        attribute CARRYOUTREG: bool;
        attribute RSTTYPE: BRAM_RSTTYPE;
    }

    // TODO: enums, bel slots

    region_slot HCLK;
    region_slot LEAF;

    wire PULLUP: pullup;
    wire TIE_0: tie 0;
    wire TIE_1: tie 1;
    wire GCLK[16]: regional LEAF;

    wire SNG_W0[8]: mux;
    wire SNG_W1[8]: branch E;
    wire SNG_W1_N3: branch S;
    wire SNG_W1_S4: branch N;
    wire SNG_E0[8]: mux;
    wire SNG_E1[8]: branch W;
    wire SNG_E1_S0: branch N;
    wire SNG_E1_N7: branch S;
    wire SNG_S0[8]: mux;
    wire SNG_S1[8]: branch N;
    wire SNG_S1_N7: branch S;
    wire SNG_N0[8]: mux;
    wire SNG_N1[8]: branch S;
    wire SNG_N1_S0: branch N;

    wire DBL_WW0[4]: mux;
    wire DBL_WW1[4]: branch E;
    wire DBL_WW2[4]: branch E;
    wire DBL_WW2_N3: branch S;
    wire DBL_EE0[4]: mux;
    wire DBL_EE1[4]: branch W;
    wire DBL_EE2[4]: branch W;
    wire DBL_SS0[4]: mux;
    wire DBL_SS1[4]: branch N;
    wire DBL_SS2[4]: branch N;
    wire DBL_SS2_N3: branch S;
    wire DBL_SW0[4]: mux;
    wire DBL_SW1[4]: branch N;
    wire DBL_SW2[4]: branch E;
    wire DBL_SW2_N3: branch S;
    wire DBL_SE0[4]: mux;
    wire DBL_SE1[4]: branch N;
    wire DBL_SE2[4]: branch W;
    wire DBL_NN0[4]: mux;
    wire DBL_NN1[4]: branch S;
    wire DBL_NN2[4]: branch S;
    wire DBL_NN2_S0: branch N;
    wire DBL_NW0[4]: mux;
    wire DBL_NW1[4]: branch S;
    wire DBL_NW2[4]: branch E;
    wire DBL_NW2_S0: branch N;
    wire DBL_NE0[4]: mux;
    wire DBL_NE1[4]: branch S;
    wire DBL_NE2[4]: branch W;
    wire DBL_NE2_S0: branch N;

    wire QUAD_WW0[4]: mux;
    wire QUAD_WW1[4]: branch E;
    wire QUAD_WW2[4]: branch E;
    wire QUAD_WW3[4]: branch E;
    wire QUAD_WW4[4]: branch E;
    wire QUAD_WW4_S0: branch N;
    wire QUAD_EE0[4]: mux;
    wire QUAD_EE1[4]: branch W;
    wire QUAD_EE2[4]: branch W;
    wire QUAD_EE3[4]: branch W;
    wire QUAD_EE4[4]: branch W;
    wire QUAD_SS0[4]: mux;
    wire QUAD_SS1[4]: branch N;
    wire QUAD_SS2[4]: branch N;
    wire QUAD_SS3[4]: branch N;
    wire QUAD_SS4[4]: branch N;
    wire QUAD_SS4_N3: branch S;
    wire QUAD_SW0[4]: mux;
    wire QUAD_SW1[4]: branch N;
    wire QUAD_SW2[4]: branch N;
    wire QUAD_SW3[4]: branch E;
    wire QUAD_SW4[4]: branch E;
    wire QUAD_SW4_N3: branch S;
    wire QUAD_SE0[4]: mux;
    wire QUAD_SE1[4]: branch N;
    wire QUAD_SE2[4]: branch N;
    wire QUAD_SE3[4]: branch W;
    wire QUAD_SE4[4]: branch W;
    wire QUAD_NN0[4]: mux;
    wire QUAD_NN1[4]: branch S;
    wire QUAD_NN2[4]: branch S;
    wire QUAD_NN3[4]: branch S;
    wire QUAD_NN4[4]: branch S;
    wire QUAD_NW0[4]: mux;
    wire QUAD_NW1[4]: branch S;
    wire QUAD_NW2[4]: branch S;
    wire QUAD_NW3[4]: branch E;
    wire QUAD_NW4[4]: branch E;
    wire QUAD_NW4_S0: branch N;
    wire QUAD_NE0[4]: mux;
    wire QUAD_NE1[4]: branch S;
    wire QUAD_NE2[4]: branch S;
    wire QUAD_NE3[4]: branch W;
    wire QUAD_NE4[4]: branch W;

    wire IMUX_GFAN[2]: mux;
    wire IMUX_CLK[2]: mux;
    wire IMUX_SR[2]: mux;
    wire IMUX_LOGICIN[64]: mux;
    wire IMUX_LOGICIN20_BOUNCE: mux;
    wire IMUX_LOGICIN36_BOUNCE: mux;
    wire IMUX_LOGICIN44_BOUNCE: mux;
    wire IMUX_LOGICIN62_BOUNCE: mux;
    wire IMUX_LOGICIN21_BOUNCE: mux;
    wire IMUX_LOGICIN28_BOUNCE: mux;
    wire IMUX_LOGICIN52_BOUNCE: mux;
    wire IMUX_LOGICIN60_BOUNCE: mux;
    wire IMUX_LOGICIN20_S: branch N;
    wire IMUX_LOGICIN36_S: branch N;
    wire IMUX_LOGICIN44_S: branch N;
    wire IMUX_LOGICIN62_S: branch N;
    wire IMUX_LOGICIN21_N: branch S;
    wire IMUX_LOGICIN28_N: branch S;
    wire IMUX_LOGICIN52_N: branch S;
    wire IMUX_LOGICIN60_N: branch S;
    wire OUT[24]: bel;
    wire OUT_BEL[24]: bel;
    wire OUT_TEST[24]: test;
    wire IMUX_CLK_GCLK[2]: mux;

    bitrect INT = vertical (22, rev 64);
    bitrect CLEXL = vertical (30, rev 64);
    bitrect CLEXM = vertical (31, rev 64);
    bitrect CLE_CLK = vertical (31, rev 64);
    bitrect BRAM = vertical (25, rev 64);
    bitrect DSP = vertical (24, rev 64);
    bitrect CLK_V = vertical (4, rev 64);
    bitrect HCLK = vertical (22, rev 16);
    bitrect BRAM_DATA = vertical (1, 18720);
    bitrect IOB = vertical (1, 128);
    bitrect CLK = vertical (1, 384);

    tile_slot INT {
        bel_slot INT: routing;

        tile_class INT, INT_IOI {
            cell CELL;
            bitrect MAIN: INT;
        }
    }

    tile_slot INTF {
        bel_slot INTF_INT: routing;
        bel_slot INTF_TESTMUX: routing;

        tile_class INTF, INTF_IOI, INTF_CMT, INTF_CMT_IOI {
            cell CELL;
            if tile_class [INTF, INTF_IOI] {
                bitrect MAIN: INT;
            } else {
                bitrect MAIN: CLE_CLK;
            }
        }
    }

    tile_slot BEL {
        bel_slot SLICE[2]: legacy;
        tile_class CLEXL {
            cell CELL;
            bitrect MAIN: CLEXL;
        }
        tile_class CLEXM {
            cell CELL;
            bitrect MAIN: CLEXM;
        }

        bel_slot BRAM_F: legacy;
        bel_slot BRAM_H[2]: legacy;
        tile_class BRAM {
            cell CELL[4];
            bitrect MAIN[4]: BRAM;
            bitrect DATA: BRAM_DATA;
        }

        bel_slot DSP: DSP;
        tile_class DSP {
            cell CELL[4];
            bitrect MAIN[4]: DSP;
        }

        bel_slot ILOGIC[2]: legacy;
        bel_slot OLOGIC[2]: legacy;
        bel_slot IODELAY[2]: legacy;
        bel_slot IOICLK[2]: legacy;
        bel_slot IOI: legacy;
        tile_class IOI_WE, IOI_SN {
            cell CELL;
            bitrect MAIN: CLEXL;
        }

        bel_slot DCM[2]: legacy;
        bel_slot PLL: legacy;
        bel_slot CMT: legacy;
        tile_class CMT_DCM {
            cell CELL[2];
            bitrect MAIN[16]: CLE_CLK;
            bitrect CLK[16]: CLK_V;
        }
        tile_class CMT_PLL {
            cell CELL[2];
            bitrect MAIN[16]: CLE_CLK;
            bitrect CLK[16]: CLK_V;
        }

        bel_slot MCB: legacy;
        tile_class MCB {
            cell CELL[12];
            cell CELL_MUI[16];
            bitrect MAIN[12]: CLEXL;
            bitrect MAIN_MUI[16]: CLEXL;
        }

        bel_slot PCIE: legacy;
        tile_class PCIE {
            cell W[16];
            cell E[16];
            bitrect MAIN_W[16]: INT;
            bitrect MAIN_E[16]: INT;
        }

        bel_slot GTP: legacy;
        bel_slot BUFDS[2]: legacy;
        tile_class GTP {
            cell W[8];
            cell E[8];
            bitrect MAIN_W[8]: CLEXL;
            bitrect MAIN_E[8]: CLEXL;
        }

        bel_slot PCILOGICSE: legacy;
        tile_class PCILOGICSE {
            cell CELL;
            bitrect MAIN: CLEXL;
        }

        bel_slot OCT_CAL[6]: legacy;
        bel_slot PMV: legacy;
        bel_slot DNA_PORT: legacy;
        bel_slot ICAP: legacy;
        bel_slot SPI_ACCESS: legacy;
        bel_slot SUSPEND_SYNC: legacy;
        bel_slot POST_CRC_INTERNAL: legacy;
        bel_slot STARTUP: legacy;
        bel_slot SLAVE_SPI: legacy;
        bel_slot BSCAN[4]: legacy;
        tile_class CNR_SW, CNR_NW {
            cell CELL;
            bitrect MAIN: CLEXL;
        }
        tile_class CNR_SE, CNR_NE {
            cell CELL[2];
            bitrect MAIN[2]: CLEXL;
        }

        bel_slot BUFGMUX[16]: legacy;
        bel_slot CLKC: legacy;
        bel_slot CLKC_BUFPLL: legacy;
        tile_class CLKC {
            cell CELL;
            bitrect MAIN: CLE_CLK;
        }

        // RE-only detritus?
        bel_slot MCB_TIE_CLK: legacy;
        bel_slot MCB_TIE_DQS0: legacy;
        bel_slot MCB_TIE_DQS1: legacy;
        bel_slot TIEOFF_IOI: legacy;
        bel_slot TIEOFF_PLL: legacy;
        bel_slot TIEOFF_CLK: legacy;
        bel_slot TIEOFF_DQS0: legacy;
        bel_slot TIEOFF_DQS1: legacy;
        bel_slot IPAD_CLKP[2]: legacy;
        bel_slot IPAD_CLKN[2]: legacy;
        bel_slot IPAD_RXP[2]: legacy;
        bel_slot IPAD_RXN[2]: legacy;
        bel_slot OPAD_TXP[2]: legacy;
        bel_slot OPAD_TXN[2]: legacy;
        bel_slot GTP_BUF: legacy;
    }

    tile_slot IOB {
        bel_slot IOB[2]: legacy;
        tile_class IOB {
            bitrect MAIN: IOB;
        }
    }

    tile_slot HCLK {
        bel_slot HCLK: legacy;
        tile_class HCLK {
            cell S, N;
            bitrect MAIN: HCLK;
        }
    }

    tile_slot HCLK_BEL {
        tile_class HCLK_CLEXL, HCLK_CLEXM, HCLK_IOI, HCLK_GTP {
            bitrect MAIN: HCLK;
        }
    }

    tile_slot HCLK_ROW {
        bel_slot BUFH_W[16]: legacy;
        bel_slot BUFH_E[16]: legacy;
        bel_slot HCLK_ROW: legacy;
        tile_class HCLK_ROW {
            bitrect MAIN: CLK_V;
        }
    }

    tile_slot CLK {
        bel_slot REG_INT: routing;
        bel_slot BUFIO2[8]: legacy;
        bel_slot BUFIO2FB[8]: legacy;
        bel_slot BUFPLL[2]: legacy;
        bel_slot BUFPLL_MCB: legacy;
        bel_slot BUFPLL_OUT: legacy;
        bel_slot BUFPLL_INS_WE: legacy;
        bel_slot BUFPLL_INS_SN: legacy;
        // RE-only detritus?
        bel_slot BUFIO2_INS: legacy;
        bel_slot BUFIO2_CKPIN: legacy;
        bel_slot BUFPLL_BUF: legacy;
        bel_slot GTP_H_BUF: legacy;
        bel_slot TIEOFF_REG: legacy;

        tile_class CLK_W, CLK_E {
            cell CELL[2];
            bitrect MAIN: CLK;
        }
        tile_class CLK_S, CLK_N {
            cell CELL;
            bitrect MAIN: CLK;
        }
    }

    tile_slot CMT_BUF {
        bel_slot DCM_BUFPLL_BUF_S: legacy;
        tile_class DCM_BUFPLL_BUF_S {
            bitrect MAIN: CLK_V;
        }

        bel_slot DCM_BUFPLL_BUF_S_MID: legacy;
        tile_class DCM_BUFPLL_BUF_S_MID {
            bitrect MAIN: CLK_V;
        }

        bel_slot DCM_BUFPLL_BUF_N: legacy;
        tile_class DCM_BUFPLL_BUF_N {
            bitrect MAIN: CLK_V;
        }

        bel_slot DCM_BUFPLL_BUF_N_MID: legacy;
        tile_class DCM_BUFPLL_BUF_N_MID {
            bitrect MAIN: CLK_V;
        }

        bel_slot PLL_BUFPLL: legacy;
        tile_class PLL_BUFPLL_OUT0 {
            bitrect MAIN: CLK_V;
        }
        tile_class PLL_BUFPLL_OUT1 {
            bitrect MAIN: CLK_V;
        }
        tile_class PLL_BUFPLL_S {
            bitrect MAIN: CLK_V;
        }
        tile_class PLL_BUFPLL_N {
            bitrect MAIN: CLK_V;
        }
    }

    tile_slot IOI_CLK {
        // RE-only detritus?
        bel_slot IOI_CLK_SN: legacy;
        tile_class IOI_CLK_SN {
            // no cells
        }

        bel_slot IOI_CLK_WE: legacy;
        tile_class IOI_CLK_WE {
            // no cells
        }

        bel_slot IOI_CLK_WE_TERM: legacy;
    }

    tile_slot CLK_BUF {
        // RE-only detritus?
        bel_slot HCLK_V_MIDBUF: legacy;
        tile_class HCLK_V_MIDBUF {
            // no cells
        }

        bel_slot HCLK_H_MIDBUF: legacy;
        tile_class HCLK_H_MIDBUF {
            // no cells
        }

        bel_slot CKPIN_V_MIDBUF: legacy;
        tile_class CKPIN_V_MIDBUF {
            // no cells
        }

        bel_slot CKPIN_H_MIDBUF: legacy;
        tile_class CKPIN_H_MIDBUF {
            // no cells
        }

        bel_slot CLKPIN_BUF: legacy;
        tile_class CLKPIN_BUF {
            // no cells
        }
    }

    tile_slot PCI_CE_TRUNK_BUF {
        // RE-only detritus?
        bel_slot PCI_CE_TRUNK_BUF: legacy;
        tile_class PCI_CE_TRUNK_BUF {
            // no cells
        }
    }

    tile_slot PCI_CE_BUF {
        // RE-only detritus?
        bel_slot PCI_CE_V_BUF: legacy;
        tile_class PCI_CE_V_BUF {
            // no cells
        }

        bel_slot PCI_CE_H_BUF: legacy;
        tile_class PCI_CE_H_BUF {
            // no cells
        }

        bel_slot PCI_CE_SPLIT: legacy;
        tile_class PCI_CE_SPLIT {
            // no cells
        }
    }

    connector_slot W {
        opposite E;

        connector_class PASS_W {
            pass SNG_E1 = SNG_E0;
            pass DBL_EE1 = DBL_EE0;
            pass DBL_EE2 = DBL_EE1;
            pass DBL_SE2 = DBL_SE1;
            pass DBL_NE2 = DBL_NE1;
            pass QUAD_EE1 = QUAD_EE0;
            pass QUAD_EE2 = QUAD_EE1;
            pass QUAD_EE3 = QUAD_EE2;
            pass QUAD_EE4 = QUAD_EE3;
            pass QUAD_SE3 = QUAD_SE2;
            pass QUAD_SE4 = QUAD_SE3;
            pass QUAD_NE3 = QUAD_NE2;
            pass QUAD_NE4 = QUAD_NE3;

        }
        connector_class TERM_W;
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            pass SNG_W1 = SNG_W0;
            pass DBL_WW1 = DBL_WW0;
            pass DBL_WW2 = DBL_WW1;
            pass DBL_SW2 = DBL_SW1;
            pass DBL_NW2 = DBL_NW1;
            pass QUAD_WW1 = QUAD_WW0;
            pass QUAD_WW2 = QUAD_WW1;
            pass QUAD_WW3 = QUAD_WW2;
            pass QUAD_WW4 = QUAD_WW3;
            pass QUAD_SW3 = QUAD_SW2;
            pass QUAD_SW4 = QUAD_SW3;
            pass QUAD_NW3 = QUAD_NW2;
            pass QUAD_NW4 = QUAD_NW3;
        }
        connector_class TERM_E;
    }

    connector_slot S {
        opposite N;

        connector_class PASS_S {
            pass SNG_N1 = SNG_N0;
            pass SNG_W1_N3 = SNG_W1[3];
            pass SNG_E1_N7 = SNG_E1[7];
            pass SNG_S1_N7 = SNG_S1[7];
            pass DBL_NN1 = DBL_NN0;
            pass DBL_NN2 = DBL_NN1;
            pass DBL_NW1 = DBL_NW0;
            pass DBL_NE1 = DBL_NE0;
            pass DBL_WW2_N3 = DBL_WW2[3];
            pass DBL_SS2_N3 = DBL_SS2[3];
            pass DBL_SW2_N3 = DBL_SW2[3];
            pass QUAD_NN1 = QUAD_NN0;
            pass QUAD_NN2 = QUAD_NN1;
            pass QUAD_NN3 = QUAD_NN2;
            pass QUAD_NN4 = QUAD_NN3;
            pass QUAD_NW1 = QUAD_NW0;
            pass QUAD_NW2 = QUAD_NW1;
            pass QUAD_NE1 = QUAD_NE0;
            pass QUAD_NE2 = QUAD_NE1;
            pass QUAD_SS4_N3 = QUAD_SS4[3];
            pass QUAD_SW4_N3 = QUAD_SW4[3];
            pass IMUX_LOGICIN21_N = IMUX_LOGICIN21_BOUNCE;
            pass IMUX_LOGICIN28_N = IMUX_LOGICIN28_BOUNCE;
            pass IMUX_LOGICIN52_N = IMUX_LOGICIN52_BOUNCE;
            pass IMUX_LOGICIN60_N = IMUX_LOGICIN60_BOUNCE;
        }
        connector_class TERM_S;
    }

    connector_slot N {
        opposite S;

        connector_class PASS_N {
            pass SNG_S1 = SNG_S0;
            pass SNG_W1_S4 = SNG_W1[4];
            pass SNG_E1_S0 = SNG_E1[0];
            pass SNG_N1_S0 = SNG_N1[0];
            pass DBL_SS1 = DBL_SS0;
            pass DBL_SS2 = DBL_SS1;
            pass DBL_SW1 = DBL_SW0;
            pass DBL_SE1 = DBL_SE0;
            pass DBL_NN2_S0 = DBL_NN2[0];
            pass DBL_NW2_S0 = DBL_NW2[0];
            pass DBL_NE2_S0 = DBL_NE2[0];
            pass QUAD_SS1 = QUAD_SS0;
            pass QUAD_SS2 = QUAD_SS1;
            pass QUAD_SS3 = QUAD_SS2;
            pass QUAD_SS4 = QUAD_SS3;
            pass QUAD_SW1 = QUAD_SW0;
            pass QUAD_SW2 = QUAD_SW1;
            pass QUAD_SE1 = QUAD_SE0;
            pass QUAD_SE2 = QUAD_SE1;
            pass QUAD_WW4_S0 = QUAD_WW4[0];
            pass QUAD_NW4_S0 = QUAD_NW4[0];
            pass IMUX_LOGICIN20_S = IMUX_LOGICIN20_BOUNCE;
            pass IMUX_LOGICIN36_S = IMUX_LOGICIN36_BOUNCE;
            pass IMUX_LOGICIN44_S = IMUX_LOGICIN44_BOUNCE;
            pass IMUX_LOGICIN62_S = IMUX_LOGICIN62_BOUNCE;
        }
        connector_class TERM_N;
    }
}
