use prjcombine_interconnect::db::WireSlotId;
use prjcombine_tablegen::target_defs;

target_defs! {
    enum SLICE_CYINIT { BX, CIN }
    enum SLICE_CY0 { CONST_0, CONST_1, F1_G1, PROD }
    enum SLICE_CYSELF { CONST_1, F }
    enum SLICE_CYSELG { CONST_1, G }
    enum SLICE_DIF_MUX { BX, BY }
    enum SLICE_DXMUX { BX, X }
    enum SLICE_DYMUX { BY, Y }
    enum SLICE_FXMUX { F, F5, FXOR }
    enum SLICE_GYMUX { G, F6, GXOR }
    enum SLICE_YBMUX { GCY, BY }

    bel_class SLICE {
        input F1, F2, F3, F4;
        input G1, G2, G3, G4;
        input BX, BY;
        input CLK, SR, CE;
        output X, Y;
        output XQ, YQ;
        output XB, YB;

        attribute F, G: bitvec[16];

        attribute DIF_MUX: SLICE_DIF_MUX;
        attribute F_RAM_ENABLE, G_RAM_ENABLE: bool;
        attribute F_SHIFT_ENABLE, G_SHIFT_ENABLE: bool;
        attribute WA4_ENABLE: bool;

        attribute CYINIT: SLICE_CYINIT;
        attribute CY0: SLICE_CY0;
        attribute CYSELF: SLICE_CYSELF;
        attribute CYSELG: SLICE_CYSELG;

        attribute FFX_INIT, FFY_INIT: bitvec[1];
        attribute FFX_READBACK, FFY_READBACK: bitvec[1];
        attribute FF_LATCH: bool;
        attribute FF_REV_ENABLE: bool;
        attribute FF_SR_SYNC: bool;
        attribute FF_SR_ENABLE: bool;

        attribute FXMUX: SLICE_FXMUX;
        attribute GYMUX: SLICE_GYMUX;
        attribute DXMUX: SLICE_DXMUX;
        attribute DYMUX: SLICE_DYMUX;

        attribute YBMUX: SLICE_YBMUX;
    }

    bel_class TBUF {
        input I, T;
        attribute OUT_A, OUT_B: bool;
    }

    bel_class TBUS {
        output OUT;
        attribute JOINER_E: bool;
    }

    bel_class TBUS_WE {
        output BUS0, BUS1, BUS2, BUS3;

        // IO_W only
        attribute JOINER: bool;
        attribute JOINER_E: bool;
    }

    enum BRAM_DATA_WIDTH { _1, _2, _4, _8, _16 }
    bel_class BRAM {
        input CLKA, CLKB;
        input ENA, ENB;
        input RSTA, RSTB;
        input WEA, WEB;
        input ADDRA[12], ADDRB[12];
        input DIA[16], DIB[16];
        output DOA[16], DOB[16];

        attribute DATA_WIDTH_A: BRAM_DATA_WIDTH;
        attribute DATA_WIDTH_B: BRAM_DATA_WIDTH;
        attribute INIT: bitvec[0x1000];
    }

    enum IO_MUX_O { O, FFO }
    enum IO_MUX_T { T, FFT }
    bel_class IOI {
        // the inputs are tied together, but with separate inverts
        input ICLK, OCLK, TCLK;
        input SR;
        input ICE, OCE, TCE;
        input O, T;
        output I, IQ;

        attribute SHORTEN_JTAG_CHAIN: bool;

        // input path
        attribute FFI_INIT: bitvec[1];
        attribute FFI_READBACK: bitvec[1];
        attribute FFI_LATCH: bool;
        attribute FFI_SR_ENABLE: bool;
        attribute FFI_SR_SYNC: bool;
        attribute FFI_DELAY_ENABLE: bool;
        attribute I_DELAY_ENABLE: bool;

        // output path
        attribute FFO_INIT: bitvec[1];
        attribute FFO_READBACK: bitvec[1];
        attribute FFO_LATCH: bool;
        attribute FFO_SR_ENABLE: bool;
        attribute FFO_SR_SYNC: bool;
        attribute FFT_INIT: bitvec[1];
        attribute FFT_READBACK: bitvec[1];
        attribute FFT_LATCH: bool;
        attribute FFT_SR_ENABLE: bool;
        attribute FFT_SR_SYNC: bool;
        attribute MUX_O: IO_MUX_O;
        attribute MUX_T: IO_MUX_T;
    }

    enum IOB_PULL { NONE, PULLUP, PULLDOWN, KEEPER }
    // VREF_LV, VREF_HV are virtex only
    // VREF, DIFF are virtexe only
    enum IOB_IBUF_MODE { NONE, VREF_LV, VREF_HV, VREF, DIFF, CMOS }
    bel_class IOB {
        pad PAD: inout;

        attribute PULL: IOB_PULL;
        attribute IBUF_MODE: IOB_IBUF_MODE;
        attribute READBACK_I: bitvec[1];
        attribute VREF: bool;
        // duplicated with IOI for unknown reasons
        attribute MUX_T: IO_MUX_T;
        attribute MUX_O: IO_MUX_O;

        attribute PDRIVE: bitvec[4];
        attribute NDRIVE: bitvec[5];
        attribute V_SLEW: bitvec[4];
        attribute V_OUTPUT_MISC: bitvec[2];
        attribute V_IOSTD_MISC: bitvec[3];
        attribute VE_SLEW: bitvec[5];
        attribute VE_OUTPUT_MISC: bitvec[3];
        attribute VE_IOSTD_MISC: bitvec[1];
    }

    table IOB_DATA_V {
        field PDRIVE: bitvec[4];
        field NDRIVE: bitvec[5];
        field SLEW_FAST: bitvec[4];
        field SLEW_SLOW: bitvec[4];
        field OUTPUT_MISC: bitvec[2];
        field IOSTD_MISC: bitvec[3];

        row OFF;

        row LVTTL_2, LVTTL_4, LVTTL_6, LVTTL_8, LVTTL_12, LVTTL_16, LVTTL_24;
        row LVCMOS2;
        row PCI33_3, PCI33_5, PCI66_3;

        row AGP, CTT;
        row GTL, GTLP;
        row HSTL_I, HSTL_III, HSTL_IV;
        row SSTL2_I, SSTL2_II;
        row SSTL3_I, SSTL3_II;
    }

    table IOB_DATA_VE {
        field PDRIVE: bitvec[4];
        field NDRIVE: bitvec[5];
        field SLEW_FAST: bitvec[5];
        field SLEW_SLOW: bitvec[5];
        field OUTPUT_MISC: bitvec[3];
        field IOSTD_MISC: bitvec[1];

        row OFF;

        row LVTTL_2, LVTTL_4, LVTTL_6, LVTTL_8, LVTTL_12, LVTTL_16, LVTTL_24;
        row LVCMOS2, LVCMOS18;
        row PCI33_3, PCI66_3, PCIX66_3;

        row AGP, CTT;
        row GTL, GTLP;
        row HSTL_I, HSTL_III, HSTL_IV;
        row SSTL2_I, SSTL2_II;
        row SSTL3_I, SSTL3_II;

        row LVDS, LVPECL;
    }

    bel_class CAPTURE {
        input CLK;
        input CAP;
    }

    bel_class STARTUP {
        input CLK;
        input GSR, GWE, GTS;

        attribute USER_GTS_GWE_GSR_ENABLE: bool;
        attribute GSR_SYNC: bool;
        attribute GWE_SYNC: bool;
        attribute GTS_SYNC: bool;
    }

    bel_class BSCAN {
        input TDO1, TDO2;
        output DRCK1, DRCK2;
        output SEL1, SEL2;
        output TDI;
        output RESET, SHIFT, UPDATE;

        attribute USERCODE: bitvec[32];
    }

    bel_class BUFGCE {
        input I, CE;
        output O;
        attribute INIT_OUT: bitvec[1];
    }

    bel_class GCLK_IOB {
        output I;

        pad PAD: input;

        attribute DELAY: bitvec[5];
        attribute IBUF_MODE: IOB_IBUF_MODE;
    }

    bel_class IOFB {
        output I;
        attribute IBUF_MODE: IOB_IBUF_MODE;
    }

    bel_class PCILOGIC {
        input I1, I2, I3;
        output PCI_CE;
        attribute PCI_DELAY: bitvec[2];
    }

    enum DLL_CLKDV_MODE { HALF, INT }
    enum DLL_TEST_OSC { _90, _180, _270, _360 }
    bel_class DLL {
        input CLKIN, CLKFB, RST;
        output CLK0, CLK90, CLK180, CLK270, CLK2X, CLK2X90, CLKDV;
        output LOCKED;

        attribute ENABLE: bool;
        attribute CLK_FEEDBACK_2X: bool;
        attribute DUTY_CYCLE_CORRECTION: bitvec[4];
        attribute CLKIN_PAD: bool;
        attribute CLKFB_PAD: bool;
        attribute HIGH_FREQUENCY: bool;

        attribute CLKDV_COUNT_MAX: bitvec[4];
        attribute CLKDV_COUNT_FALL: bitvec[4];
        attribute CLKDV_COUNT_FALL_2: bitvec[4];
        attribute CLKDV_PHASE_RISE: bitvec[2];
        attribute CLKDV_PHASE_FALL: bitvec[2];
        attribute CLKDV_MODE: DLL_CLKDV_MODE;

        attribute FACTORY_JF1: bitvec[8];
        attribute FACTORY_JF2: bitvec[8];

        attribute CFG_O_14: bitvec[1];
        attribute LVL1_MUX_20: bitvec[1];
        attribute LVL1_MUX_21: bitvec[1];
        attribute LVL1_MUX_22: bitvec[1];
        attribute LVL1_MUX_23: bitvec[1];
        attribute LVL1_MUX_24: bitvec[1];
        attribute TESTDLL: bitvec[6];
        attribute TESTZD2OSC: bool;
        attribute TEST_OSC: DLL_TEST_OSC;
    }

    enum POWERUP_DELAY { _100US, _200US, _400US }
    bel_class MISC_SW {
        pad M0, M1, M2: input;
        pad POWERDOWN_B: input;

        attribute M0_PULL: IOB_PULL;
        attribute M1_PULL: IOB_PULL;
        attribute M2_PULL: IOB_PULL;
        attribute POWERDOWN_PULL: IOB_PULL;
        // ?????
        attribute PDSTATUS_PULL: IOB_PULL;

        attribute DRIVE_PD_STATUS: bool;
        attribute POWERUP_DELAY: POWERUP_DELAY;
    }

    bel_class MISC_SE {
        pad DONE: inout;
        pad PROG_B: input;

        attribute DONE_PULL: IOB_PULL;
        attribute PROG_PULL: IOB_PULL;
    }

    enum POWERUP_CLK { INTOSC, CCLK, USERCLK }
    bel_class MISC_NW {
        pad TCK: input;
        pad TMS: input;

        attribute TCK_PULL: IOB_PULL;
        attribute TMS_PULL: IOB_PULL;

        attribute DLL_ENABLE: bool;
        attribute POWERUP_CLK: POWERUP_CLK;
        attribute BCLK_DIV2: bitvec[5];
    }

    bel_class MISC_NE {
        pad CCLK: inout;
        pad TDI: input;
        pad TDO: output;

        attribute CCLK_PULL: IOB_PULL;
        attribute TDI_PULL: IOB_PULL;
        attribute TDO_PULL: IOB_PULL;
    }

    enum STARTUP_CYCLE { _0, _1, _2, _3, _4, _5, _6, DONE, KEEP, NOWAIT }
    enum STARTUP_CLOCK { CCLK, USERCLK, JTAGCLK }
    enum CONFIG_RATE { _4, _5, _7, _8, _9, _10, _13, _15, _20, _26, _30, _34, _41, _51, _55, _60, _130 }
    enum SECURITY { NONE, LEVEL1, LEVEL2 }
    bel_class GLOBAL {
        // COR
        attribute GSR_CYCLE: STARTUP_CYCLE;
        attribute GWE_CYCLE: STARTUP_CYCLE;
        attribute GTS_CYCLE: STARTUP_CYCLE;
        attribute LOCK_CYCLE: STARTUP_CYCLE;
        attribute DONE_CYCLE: STARTUP_CYCLE;
        attribute SHUTDOWN: bool;
        attribute LOCK_WAIT_SW: bool;
        attribute LOCK_WAIT_SE: bool;
        attribute LOCK_WAIT_NW: bool;
        attribute LOCK_WAIT_NE: bool;
        attribute STARTUP_CLOCK: STARTUP_CLOCK;
        attribute CONFIG_RATE: CONFIG_RATE;
        attribute CAPTURE_ONESHOT: bool;
        attribute DRIVE_DONE: bool;
        attribute DONE_PIPE: bool;

        // CTL
        attribute GTS_USR_B: bool;
        attribute DISPMP2: bool;
        attribute DISPMP1: bool;
        attribute PERSIST: bool;
        attribute SECURITY: SECURITY;
    }

    region_slot GLOBAL;
    region_slot LEAF;
    region_slot PCI_CE;

    wire PULLUP: pullup;

    wire GCLK[4]: regional GLOBAL;
    wire GCLK_LEAF[4]: regional LEAF;
    wire GCLK_BUF[4]: mux;

    wire PCI_CE: regional PCI_CE;

    wire SINGLE_W[24]: multi_branch W;
    wire SINGLE_E[24]: multi_root;
    wire SINGLE_S[24]: multi_branch S;
    wire SINGLE_N[24]: multi_root;
    wire SINGLE_W_BUF[24]: mux;
    wire SINGLE_E_BUF[24]: mux;
    wire SINGLE_S_BUF[24]: mux;
    wire SINGLE_N_BUF[24]: mux;

    wire BRAM_QUAD_ADDR[32]: multi_root;
    wire BRAM_QUAD_ADDR_S[32]: multi_branch N;
    wire BRAM_QUAD_DIN[32]: multi_root;
    wire BRAM_QUAD_DIN_S[32]: multi_branch N;
    wire BRAM_QUAD_DOUT[32]: multi_root;
    wire BRAM_QUAD_DOUT_S[32]: multi_branch N;

    wire BRAM_QUAD_ADDR_MUX[32]: mux;
    wire BRAM_QUAD_DIN_MUX[32]: mux;

    wire HEX_H0[6]: multi_branch E;
    wire HEX_H1[6]: multi_branch E;
    wire HEX_H2[6]: multi_branch E;
    wire HEX_H3[6]: multi_root;
    wire HEX_H4[6]: multi_branch W;
    wire HEX_H5[6]: multi_branch W;
    wire HEX_H6[6]: multi_branch W;
    wire HEX_H0_BUF[4]: mux;
    wire HEX_H1_BUF[4]: mux;
    wire HEX_H2_BUF[4]: mux;
    wire HEX_H3_BUF[4]: mux;
    wire HEX_H4_BUF[4]: mux;
    wire HEX_H5_BUF[4]: mux;
    wire HEX_H6_BUF[4]: mux;
    wire HEX_H0_MUX[6]: mux;
    wire HEX_H1_MUX[6]: mux;
    wire HEX_H2_MUX[6]: mux;
    wire HEX_H3_MUX[6]: mux;
    wire HEX_H4_MUX[6]: mux;
    wire HEX_H5_MUX[6]: mux;
    wire HEX_H6_MUX[6]: mux;

    wire HEX_W0[4]: mux;
    wire HEX_W1[4]: branch E;
    wire HEX_W2[4]: branch E;
    wire HEX_W3[4]: branch E;
    wire HEX_W4[4]: branch E;
    wire HEX_W5[4]: branch E;
    wire HEX_W6[4]: branch E;

    wire HEX_E0[4]: mux;
    wire HEX_E1[4]: branch W;
    wire HEX_E2[4]: branch W;
    wire HEX_E3[4]: branch W;
    wire HEX_E4[4]: branch W;
    wire HEX_E5[4]: branch W;
    wire HEX_E6[4]: branch W;

    wire HEX_V0[4]: multi_branch N;
    wire HEX_V1[4]: multi_branch N;
    wire HEX_V2[4]: multi_branch N;
    wire HEX_V3[4]: multi_root;
    wire HEX_V4[4]: multi_branch S;
    wire HEX_V5[4]: multi_branch S;
    wire HEX_V6[4]: multi_branch S;
    wire HEX_V0_BUF[4]: mux;
    wire HEX_V1_BUF[4]: mux;
    wire HEX_V2_BUF[4]: mux;
    wire HEX_V3_BUF[4]: mux;
    wire HEX_V4_BUF[4]: mux;
    wire HEX_V5_BUF[4]: mux;
    wire HEX_V6_BUF[4]: mux;
    wire HEX_V0_MUX[4]: mux;
    wire HEX_V1_MUX[4]: mux;
    wire HEX_V2_MUX[4]: mux;
    wire HEX_V3_MUX[4]: mux;
    wire HEX_V4_MUX[4]: mux;
    wire HEX_V5_MUX[4]: mux;
    wire HEX_V6_MUX[4]: mux;

    wire HEX_S0[4]: mux;
    wire HEX_S1[4]: branch N;
    wire HEX_S2[4]: branch N;
    wire HEX_S3[4]: branch N;
    wire HEX_S4[4]: branch N;
    wire HEX_S5[4]: branch N;
    wire HEX_S6[4]: branch N;

    wire HEX_N0[4]: mux;
    wire HEX_N1[4]: branch S;
    wire HEX_N2[4]: branch S;
    wire HEX_N3[4]: branch S;
    wire HEX_N4[4]: branch S;
    wire HEX_N5[4]: branch S;
    wire HEX_N6[4]: branch S;

    wire LH_MUX[12]: mux;
    wire LH[12]: multi_branch W;

    wire LV_MUX[12]: mux;
    wire LV[12]: multi_branch S;

    wire IMUX_CLB_CLK[2]: mux;
    wire IMUX_CLB_SR[2]: mux;
    wire IMUX_CLB_CE[2]: mux;
    wire IMUX_CLB_BX[2]: mux;
    wire IMUX_CLB_BY[2]: mux;
    wire IMUX_CLB_F1[2]: mux;
    wire IMUX_CLB_F2[2]: mux;
    wire IMUX_CLB_F3[2]: mux;
    wire IMUX_CLB_F4[2]: mux;
    wire IMUX_CLB_G1[2]: mux;
    wire IMUX_CLB_G2[2]: mux;
    wire IMUX_CLB_G3[2]: mux;
    wire IMUX_CLB_G4[2]: mux;
    wire IMUX_TBUF_T[2]: mux;
    wire IMUX_TBUF_I[2]: mux;
    wire IMUX_IO_CLK[4]: mux;
    wire IMUX_IO_SR[4]: mux;
    wire IMUX_IO_ICE[4]: mux;
    wire IMUX_IO_OCE[4]: mux;
    wire IMUX_IO_TCE[4]: mux;
    wire IMUX_IO_O[4]: mux;
    wire IMUX_IO_T[4]: mux;
    wire IMUX_CAP_CLK: mux;
    wire IMUX_CAP_CAP: mux;
    wire IMUX_STARTUP_CLK: mux;
    wire IMUX_STARTUP_GSR: mux;
    wire IMUX_STARTUP_GTS: mux;
    wire IMUX_STARTUP_GWE: mux;
    wire IMUX_BSCAN_TDO1: mux;
    wire IMUX_BSCAN_TDO2: mux;
    wire IMUX_BRAM_DIA[16]: mux;
    wire IMUX_BRAM_DIB[16]: mux;
    wire IMUX_BRAM_ADDRA[12]: mux;
    wire IMUX_BRAM_ADDRB[12]: mux;
    wire IMUX_BRAM_CLKA: mux;
    wire IMUX_BRAM_CLKB: mux;
    wire IMUX_BRAM_RSTA: mux;
    wire IMUX_BRAM_RSTB: mux;
    wire IMUX_BRAM_SELA: mux;
    wire IMUX_BRAM_SELB: mux;
    wire IMUX_BRAM_WEA: mux;
    wire IMUX_BRAM_WEB: mux;
    wire IMUX_BUFGCE_CLK[2]: mux;
    wire IMUX_BUFGCE_CE[2]: mux;
    wire IMUX_PCI_I1: mux;
    wire IMUX_PCI_I2: mux;
    wire IMUX_PCI_I3: mux;
    wire IMUX_DLL_CLKIN: mux;
    wire IMUX_DLL_CLKFB: mux;
    wire IMUX_DLL_RST: mux;

    wire OMUX[8]: mux;
    wire OMUX_E0: branch W;
    wire OMUX_E1: branch W;
    wire OMUX_W6: branch E;
    wire OMUX_W7: branch E;

    wire OUT_CLB_X[2]: bel;
    wire OUT_CLB_Y[2]: bel;
    wire OUT_CLB_XQ[2]: bel;
    wire OUT_CLB_YQ[2]: bel;
    wire OUT_CLB_XB[2]: bel;
    wire OUT_CLB_YB[2]: bel;
    wire OUT_TBUF: bel;
    wire OUT_TBUF_W[4]: bel;
    wire OUT_TBUF_E[4]: bel;
    wire OUT_IO_I[4]: bel;
    wire OUT_IO_IQ[4]: bel;
    wire OUT_BSCAN_RESET: bel;
    wire OUT_BSCAN_DRCK1: bel;
    wire OUT_BSCAN_DRCK2: bel;
    wire OUT_BSCAN_SHIFT: bel;
    wire OUT_BSCAN_TDI: bel;
    wire OUT_BSCAN_UPDATE: bel;
    wire OUT_BSCAN_SEL1: bel;
    wire OUT_BSCAN_SEL2: bel;
    wire OUT_BRAM_DOA[16]: bel;
    wire OUT_BRAM_DOB[16]: bel;
    wire OUT_BUFGCE_O[2]: bel;
    wire OUT_CLKPAD[2]: bel;
    wire OUT_IOFB[2]: bel;
    wire OUT_DLL_CLK0: bel;
    wire OUT_DLL_CLK90: bel;
    wire OUT_DLL_CLK180: bel;
    wire OUT_DLL_CLK270: bel;
    wire OUT_DLL_CLK2X: bel;
    wire OUT_DLL_CLK2X90: bel;
    wire OUT_DLL_CLKDV: bel;
    wire OUT_DLL_LOCKED: bel;

    bitrect MAIN = vertical (48, rev 18);
    bitrect IO_WE = vertical (54, rev 18);
    bitrect BRAM = vertical (27, rev 18);
    bitrect CLK = vertical (8, rev 18);
    bitrect CLKV = vertical (1, rev 18);
    bitrect BRAM_DATA = vertical (64, rev 72);

    bitrect REG32 = horizontal (1, rev 32);

    tile_slot MAIN {
        bel_slot INT: routing;
        bel_slot SLICE[2]: SLICE;
        bel_slot TBUF[2]: TBUF;
        bel_slot TBUS: TBUS;
        bel_slot TBUS_WE: TBUS_WE;

        tile_class CLB {
            cell CELL;
            bitrect MAIN: MAIN;
        }

        bel_slot IOI[4]: IOI;

        tile_class IO_W {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
        tile_class IO_E {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
        tile_class IO_S {
            cell CELL;
            bitrect MAIN: MAIN;
        }
        tile_class IO_N {
            cell CELL;
            bitrect MAIN: MAIN;
        }

        bel_slot BRAM: BRAM;
        tile_class BRAM_W, BRAM_E, BRAM_M, BRAM_W_S2, BRAM_E_S2 {
            cell CELL[4];
            cell CELL_W[4];
            cell CELL_E[4];
            bitrect MAIN[4]: BRAM;
            bitrect DATA: BRAM_DATA;
        }

        bel_slot CAPTURE: CAPTURE;
        bel_slot STARTUP: STARTUP;
        bel_slot BSCAN: BSCAN;
        bel_slot MISC_SW: MISC_SW;
        bel_slot MISC_SE: MISC_SE;
        bel_slot MISC_NW: MISC_NW;
        bel_slot MISC_NE: MISC_NE;
        tile_class CNR_SW, CNR_SW_S2 {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
        tile_class CNR_SE {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
        tile_class CNR_NW, CNR_NW_S2 {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
        tile_class CNR_NE {
            cell CELL;
            bitrect MAIN: IO_WE;
        }

        tile_class BRAM_S, BRAM_N {
            cell CELL, CELL_W;
            bitrect MAIN: BRAM;
        }

    }

    tile_slot DLL {
        bel_slot DLL_INT: routing;
        bel_slot DLL: DLL;

        tile_class DLL_S, DLLS_S, DLL_N, DLLS_N {
            cell CELL, CELL_W, CLK;
            bitrect MAIN: BRAM;
        }

        tile_class DLLP_S, DLLP_N {
            cell CELL, CELL_W, CLK, DLLS;
            bitrect MAIN: BRAM;
        }
    }

    tile_slot IOB {
        bel_slot IOB[4]: IOB;
        tile_class IOB_W_V, IOB_W_VE {
            bitrect MAIN: IO_WE;
            bel IOB[1];
            bel IOB[2];
            bel IOB[3];
        }
        tile_class IOB_E_V, IOB_E_VE {
            bitrect MAIN: IO_WE;
            bel IOB[1];
            bel IOB[2];
            bel IOB[3];
        }
        tile_class IOB_S_V, IOB_S_VE {
            bitrect MAIN: MAIN;
            bel IOB[1];
            bel IOB[2];
        }
        tile_class IOB_N_V, IOB_N_VE {
            bitrect MAIN: MAIN;
            bel IOB[1];
            bel IOB[2];
        }
    }

    tile_slot PCILOGIC {
        bel_slot PCI_INT: routing;
        bel_slot PCILOGIC: PCILOGIC;

        tile_class PCI_W_V, PCI_E_V, PCI_W_VE, PCI_E_VE {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
    }

    tile_slot CLK {
        bel_slot CLK_INT: routing;
        bel_slot GCLK_IOB[2]: GCLK_IOB;
        bel_slot IOFB[2]: IOFB;
        bel_slot BUFGCE[2]: BUFGCE;

        tile_class CLK_S_V, CLK_N_V {
            cell W, E, DLL_W, DLL_E;
            bitrect CLK[2]: CLK;
        }
        tile_class CLK_S_VE_4DLL, CLK_S_VE_2DLL, CLK_N_VE_4DLL, CLK_N_VE_2DLL {
            cell W, E, DLLP_W, DLLP_E, DLLS_W, DLLS_E;
            bitrect CLK[2]: CLK;
        }

        tile_class CLKV_CLKV, CLKV_GCLKV {
            cell W, E;
            bitrect CLKV: CLKV;
        }
        tile_class CLKV_IO {
            cell W, E;
        }
        tile_class CLKV_BRAM_S, CLKV_BRAM_N, CLKV_BRAM_S_S2, CLKV_BRAM_N_S2 {
            cell W, E;
            bitrect MAIN: BRAM;
        }
    }

    tile_slot GLOBAL {
        bel_slot GLOBAL: GLOBAL;
        tile_class GLOBAL {
            bitrect COR: REG32;
            bitrect CTL: REG32;
            bel GLOBAL;
        }
    }

    connector_slot W {
        opposite E;

        connector_class PASS_W {
            pass SINGLE_W = SINGLE_E;

            pass HEX_H4 = HEX_H3;
            pass HEX_H5 = HEX_H4;
            pass HEX_H6 = HEX_H5;
            pass HEX_E1 = HEX_E0;
            pass HEX_E2 = HEX_E1;
            pass HEX_E3 = HEX_E2;
            pass HEX_E4 = HEX_E3;
            pass HEX_E5 = HEX_E4;
            pass HEX_E6 = HEX_E5;

            for i in 0..11 {
                pass LH[i] = LH[i+1];
            }
            pass LH[11] = LH[0];

            pass OMUX_E0 = OMUX[0];
            pass OMUX_E1 = OMUX[1];
        }
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            pass HEX_H0 = HEX_H1;
            pass HEX_H1 = HEX_H2;
            pass HEX_H2 = HEX_H3;
            pass HEX_W1 = HEX_W0;
            pass HEX_W2 = HEX_W1;
            pass HEX_W3 = HEX_W2;
            pass HEX_W4 = HEX_W3;
            pass HEX_W5 = HEX_W4;
            pass HEX_W6 = HEX_W5;

            pass OMUX_W6 = OMUX[6];
            pass OMUX_W7 = OMUX[7];
        }
    }

    connector_slot S {
        opposite N;

        connector_class PASS_S {
            pass SINGLE_S = SINGLE_N;

            pass HEX_V4 = HEX_V3;
            pass HEX_V5 = HEX_V4;
            pass HEX_V6 = HEX_V5;
            pass HEX_N1 = HEX_N0;
            pass HEX_N2 = HEX_N1;
            pass HEX_N3 = HEX_N2;
            pass HEX_N4 = HEX_N3;
            pass HEX_N5 = HEX_N4;
            pass HEX_N6 = HEX_N5;

            for i in 0..11 {
                pass LV[i] = LV[i+1];
            }
            pass LV[11] = LV[0];
        }
    }

    connector_slot N {
        opposite S;

        connector_class PASS_N {
            pass HEX_V0 = HEX_V1;
            pass HEX_V1 = HEX_V2;
            pass HEX_V2 = HEX_V3;
            pass HEX_S1 = HEX_S0;
            pass HEX_S2 = HEX_S1;
            pass HEX_S3 = HEX_S2;
            pass HEX_S4 = HEX_S3;
            pass HEX_S5 = HEX_S4;
            pass HEX_S6 = HEX_S5;

            pass BRAM_QUAD_ADDR_S = BRAM_QUAD_ADDR;
            pass BRAM_QUAD_DIN_S = BRAM_QUAD_DIN;
            pass BRAM_QUAD_DOUT_S = BRAM_QUAD_DOUT;
        }
    }
}

pub fn wire_to_mux(wire: WireSlotId) -> Option<WireSlotId> {
    if let Some(idx) = wires::HEX_H0.index_of(wire) {
        Some(wires::HEX_H0_MUX[idx])
    } else if let Some(idx) = wires::HEX_H1.index_of(wire) {
        Some(wires::HEX_H1_MUX[idx])
    } else if let Some(idx) = wires::HEX_H2.index_of(wire) {
        Some(wires::HEX_H2_MUX[idx])
    } else if let Some(idx) = wires::HEX_H3.index_of(wire) {
        Some(wires::HEX_H3_MUX[idx])
    } else if let Some(idx) = wires::HEX_H4.index_of(wire) {
        Some(wires::HEX_H4_MUX[idx])
    } else if let Some(idx) = wires::HEX_H5.index_of(wire) {
        Some(wires::HEX_H5_MUX[idx])
    } else if let Some(idx) = wires::HEX_H6.index_of(wire) {
        Some(wires::HEX_H6_MUX[idx])
    } else if let Some(idx) = wires::HEX_V0.index_of(wire) {
        Some(wires::HEX_V0_MUX[idx])
    } else if let Some(idx) = wires::HEX_V1.index_of(wire) {
        Some(wires::HEX_V1_MUX[idx])
    } else if let Some(idx) = wires::HEX_V2.index_of(wire) {
        Some(wires::HEX_V2_MUX[idx])
    } else if let Some(idx) = wires::HEX_V3.index_of(wire) {
        Some(wires::HEX_V3_MUX[idx])
    } else if let Some(idx) = wires::HEX_V4.index_of(wire) {
        Some(wires::HEX_V4_MUX[idx])
    } else if let Some(idx) = wires::HEX_V5.index_of(wire) {
        Some(wires::HEX_V5_MUX[idx])
    } else if let Some(idx) = wires::HEX_V6.index_of(wire) {
        Some(wires::HEX_V6_MUX[idx])
    } else if let Some(idx) = wires::LH.index_of(wire) {
        Some(wires::LH_MUX[idx])
    } else if let Some(idx) = wires::LV.index_of(wire) {
        Some(wires::LV_MUX[idx])
    } else if let Some(idx) = wires::BRAM_QUAD_DIN.index_of(wire) {
        Some(wires::BRAM_QUAD_DIN_MUX[idx])
    } else if let Some(idx) = wires::BRAM_QUAD_ADDR.index_of(wire) {
        Some(wires::BRAM_QUAD_ADDR_MUX[idx])
    } else {
        None
    }
}

pub fn wire_from_mux(wire: WireSlotId) -> Option<WireSlotId> {
    if let Some(idx) = wires::HEX_H0_MUX.index_of(wire) {
        Some(wires::HEX_H0[idx])
    } else if let Some(idx) = wires::HEX_H1_MUX.index_of(wire) {
        Some(wires::HEX_H1[idx])
    } else if let Some(idx) = wires::HEX_H2_MUX.index_of(wire) {
        Some(wires::HEX_H2[idx])
    } else if let Some(idx) = wires::HEX_H3_MUX.index_of(wire) {
        Some(wires::HEX_H3[idx])
    } else if let Some(idx) = wires::HEX_H4_MUX.index_of(wire) {
        Some(wires::HEX_H4[idx])
    } else if let Some(idx) = wires::HEX_H5_MUX.index_of(wire) {
        Some(wires::HEX_H5[idx])
    } else if let Some(idx) = wires::HEX_H6_MUX.index_of(wire) {
        Some(wires::HEX_H6[idx])
    } else if let Some(idx) = wires::HEX_V0_MUX.index_of(wire) {
        Some(wires::HEX_V0[idx])
    } else if let Some(idx) = wires::HEX_V1_MUX.index_of(wire) {
        Some(wires::HEX_V1[idx])
    } else if let Some(idx) = wires::HEX_V2_MUX.index_of(wire) {
        Some(wires::HEX_V2[idx])
    } else if let Some(idx) = wires::HEX_V3_MUX.index_of(wire) {
        Some(wires::HEX_V3[idx])
    } else if let Some(idx) = wires::HEX_V4_MUX.index_of(wire) {
        Some(wires::HEX_V4[idx])
    } else if let Some(idx) = wires::HEX_V5_MUX.index_of(wire) {
        Some(wires::HEX_V5[idx])
    } else if let Some(idx) = wires::HEX_V6_MUX.index_of(wire) {
        Some(wires::HEX_V6[idx])
    } else if let Some(idx) = wires::LH_MUX.index_of(wire) {
        Some(wires::LH[idx])
    } else if let Some(idx) = wires::LV_MUX.index_of(wire) {
        Some(wires::LV[idx])
    } else if let Some(idx) = wires::BRAM_QUAD_DIN_MUX.index_of(wire) {
        Some(wires::BRAM_QUAD_DIN[idx])
    } else if let Some(idx) = wires::BRAM_QUAD_ADDR_MUX.index_of(wire) {
        Some(wires::BRAM_QUAD_ADDR[idx])
    } else {
        None
    }
}
