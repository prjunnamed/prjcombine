use prjcombine_tablegen::target_defs;

target_defs! {
    enum LC_MUX_DO { DI, F5O, CO }
    enum FF_MODE { FF, LATCH }
    enum LC_MUX_D { F, DO }
    bel_class LC {
        input F1, F2, F3, F4;
        input DI;
        input CE, CK, CLR;
        output X, Q, DO;

        attribute LUT: bitvec[16];
        // F5O only available at LC[0] and LC[2]
        attribute MUX_DO: LC_MUX_DO;
        attribute FF_MODE: FF_MODE;
        attribute MUX_D: LC_MUX_D;
        attribute CLR_ENABLE: bool;
        attribute CE_ENABLE: bool;
        attribute READBACK: bitvec[1];
    }

    bel_class TBUF {
        input I, T;
        output O;

        attribute T_ENABLE: bool;
    }

    bel_class PROGTIE {
        output O;

        attribute VAL: bitvec[1];
    }

    enum IO_SLEW { FAST, SLOW }
    enum IO_PULL { NONE, PULLUP, PULLDOWN }
    bel_class IO {
        input O, T;
        output I;
        pad PAD: inout;

        attribute SLEW: IO_SLEW;
        attribute PULL: IO_PULL;
        attribute DELAY_ENABLE: bool;
        attribute INV_I: bool;
    }

    enum SCANTEST_OUT { XI, YI, ZI, VI, SCANPASS }
    bel_class SCANTEST {
        attribute OUT: SCANTEST_OUT;
    }
    bel_class CIN {
        input IN;
    }
    bel_class COUT {
        output OUT;
    }
    bel_class CLKIOB {
        output OUT;
    }

    enum RDBK_MUX_CLK { CCLK, RDBK }
    bel_class RDBK {
        input CK, TRIG;
        output DATA, RIP;

        attribute MUX_CLK: RDBK_MUX_CLK;
        attribute READ_ABORT: bool;
        attribute READ_CAPTURE: bool;
    }
    enum SCAN_TEST { DISABLE, ENABLE, ENLL, NE7 }
    bel_class MISC_SW {
        attribute SCAN_TEST: SCAN_TEST;
    }

    enum CONFIG_RATE { SLOW, MED, FAST }
    enum DONE_TIMING { Q0, Q1Q4, Q2, Q3 }
    enum GTS_GSR_TIMING { Q1Q4, Q2, Q3, DONE_IN }
    enum STARTUP_MUX_CLK { CCLK, USERCLK }
    bel_class STARTUP {
        input CLK, GR, GTS;
        output DONEIN, Q1Q4, Q2, Q3;

        attribute GR_ENABLE: bool;
        attribute GTS_ENABLE: bool;
        attribute CONFIG_RATE: CONFIG_RATE;
        attribute CRC: bool;
        attribute DONE_TIMING: DONE_TIMING;
        attribute GTS_TIMING: GTS_GSR_TIMING;
        attribute GSR_TIMING: GTS_GSR_TIMING;
        attribute SYNC_TO_DONE: bool;
        attribute MUX_CLK: STARTUP_MUX_CLK;
    }
    bel_class MISC_SE {
        pad PROG_B: input;
        pad DONE: inout;
        attribute DONE_PULLUP: bool;
        attribute PROG_PULLUP: bool;
        attribute TCTEST: bool;
    }

    bel_class BSCAN {
        input TDO1, TDO2;
        output DRCK, IDLE, RESET, SEL1, SEL2, SHIFT, UPDATE;

        attribute ENABLE: bool;
        attribute RECONFIG: bool;
        attribute READBACK: bool;
    }
    enum IO_INPUT_MODE { TTL, CMOS }
    bel_class MISC_NW {
        attribute IO_INPUT_MODE: IO_INPUT_MODE;
    }

    bel_class OSC_NE {
        input C;
        output OSC1, OSC2;
    }
    enum OSC1_DIV { D2, D4, D6, D8 }
    enum OSC2_DIV { D1, D3, D5, D7, D10, D12, D14, D16 }
    enum OSC_MUX_CLK { CCLK, USERCLK }
    bel_class OSC_SE {
        attribute OSC1_DIV: OSC1_DIV;
        attribute OSC2_DIV: OSC2_DIV;
        attribute MUX_CLK: OSC_MUX_CLK;
    }
    bel_class BYPOSC {
        input I;
    }
    bel_class BSUPD {
        output O;
    }
    bel_class MISC_NE {
        pad CCLK: inout;

        attribute TAC: bool;
        attribute TLC: bool;
    }

    region_slot GCLK_H;
    region_slot GCLK_V;
    region_slot LONG_H;
    region_slot LONG_V;

    wire TIE_0: tie 0;

    wire CLB_M[24]: mux;
    wire CLB_M_BUF[24]: mux;
    wire IO_M[16]: mux;
    wire IO_M_BUF[16]: mux;

    wire SINGLE_E[12]: multi_root;
    wire SINGLE_W[12]: multi_branch W;
    wire SINGLE_S[12]: multi_root;
    wire SINGLE_N[12]: multi_branch N;
    wire SINGLE_IO_S_W[8]: multi_branch W;
    wire SINGLE_IO_S_E[8]: multi_branch W;
    wire SINGLE_IO_E_N[8]: multi_branch S;
    wire SINGLE_IO_E_S[8]: multi_branch S;
    wire SINGLE_IO_N_W[8]: multi_branch E;
    wire SINGLE_IO_N_E[8]: multi_branch E;
    wire SINGLE_IO_W_N[8]: multi_branch N;
    wire SINGLE_IO_W_S[8]: multi_branch N;

    wire DBL_H_W[2]: multi_branch E;
    wire DBL_H_M[2]: multi_root;
    wire DBL_H_E[2]: multi_branch W;
    wire DBL_V_S[2]: multi_branch N;
    wire DBL_V_M[2]: multi_root;
    wire DBL_V_N[2]: multi_branch S;

    wire LONG_H[8]: regional LONG_H;
    wire LONG_V[8]: regional LONG_V;

    wire GCLK_W: regional GCLK_H;
    wire GCLK_E: regional GCLK_H;
    wire GCLK_S: regional GCLK_V;
    wire GCLK_N: regional GCLK_V;
    wire GCLK_SW: regional GCLK_V;
    wire GCLK_SE: regional GCLK_H;
    wire GCLK_NW: regional GCLK_H;
    wire GCLK_NE: regional GCLK_V;

    wire OMUX[8]: mux;
    wire OMUX_BUF[8]: mux;
    wire OMUX_BUF_W[4]: branch E;
    wire OMUX_BUF_E[4]: branch W;
    wire OMUX_BUF_S[4]: branch N;
    wire OMUX_BUF_N[4]: branch S;

    wire OUT_LC_X[4]: bel;
    wire OUT_LC_Q[4]: bel;
    wire OUT_LC_DO[4]: bel;
    wire OUT_TBUF[4]: bel;
    wire OUT_PROGTIE: bel;
    wire OUT_IO_I[4]: bel;
    wire OUT_CLKIOB: bel;
    wire OUT_RDBK_RIP: bel;
    wire OUT_RDBK_DATA: bel;
    wire OUT_STARTUP_DONEIN: bel;
    wire OUT_STARTUP_Q1Q4: bel;
    wire OUT_STARTUP_Q2: bel;
    wire OUT_STARTUP_Q3: bel;
    wire OUT_BSCAN_DRCK: bel;
    wire OUT_BSCAN_IDLE: bel;
    wire OUT_BSCAN_RESET: bel;
    wire OUT_BSCAN_SEL1: bel;
    wire OUT_BSCAN_SEL2: bel;
    wire OUT_BSCAN_SHIFT: bel;
    wire OUT_BSCAN_UPDATE: bel;
    wire OUT_BSUPD: bel;
    wire OUT_OSC_OSC1: bel;
    wire OUT_OSC_OSC2: bel;
    wire OUT_TOP_COUT: bel;

    wire IMUX_LC_F1[4]: mux;
    wire IMUX_LC_F2[4]: mux;
    wire IMUX_LC_F3[4]: mux;
    wire IMUX_LC_F4[4]: mux;
    wire IMUX_LC_DI[4]: mux;
    wire IMUX_CLB_CE: mux;
    wire IMUX_CLB_CLK: mux;
    wire IMUX_CLB_RST: mux;
    wire IMUX_TS: mux;
    wire IMUX_GIN: mux;
    wire IMUX_IO_O[4]: mux;
    wire IMUX_IO_O_SN[4]: mux;
    wire IMUX_IO_T[4]: mux;
    wire IMUX_RDBK_RCLK: mux;
    wire IMUX_RDBK_TRIG: mux;
    wire IMUX_STARTUP_SCLK: mux;
    wire IMUX_STARTUP_GRST: mux;
    wire IMUX_STARTUP_GTS: mux;
    wire IMUX_BSCAN_TDO1: mux;
    wire IMUX_BSCAN_TDO2: mux;
    wire IMUX_OSC_OCLK: mux;
    wire IMUX_BYPOSC_PUMP: mux;
    wire IMUX_BUFG: mux;
    wire IMUX_BOT_CIN: mux;

    bitrect MAIN = vertical (rev 12, rev 34);
    bitrect MAIN_W = vertical (rev 7, rev 34);
    bitrect MAIN_E = vertical (rev 8, rev 34);
    bitrect MAIN_S = vertical (rev 12, rev 28);
    bitrect MAIN_SW = vertical (rev 7, rev 28);
    bitrect MAIN_SE = vertical (rev 8, rev 28);
    bitrect MAIN_N = vertical (rev 12, rev 28);
    bitrect MAIN_NW = vertical (rev 7, rev 28);
    bitrect MAIN_NE = vertical (rev 8, rev 28);

    bitrect LLH = vertical (rev 1, rev 34);
    bitrect LLH_S = vertical (rev 1, rev 28);
    bitrect LLH_N = vertical (rev 1, rev 28);

    bitrect LLV = vertical (rev 12, rev 4);
    bitrect LLV_W = vertical (rev 7, rev 4);
    bitrect LLV_E = vertical (rev 8, rev 4);

    tile_slot MAIN {
        bel_slot INT: routing;
        bel_slot LC[4]: LC;
        bel_slot TBUF[4]: TBUF;
        bel_slot PROGTIE: PROGTIE;

        tile_class CLB {
            cell CELL;
            bitrect MAIN: MAIN;

            switchbox INT;

            for i in 0..4 {
                bel LC[i] {
                    input F1 = IMUX_LC_F1[i];
                    input F2 = IMUX_LC_F2[i];
                    input F3 = IMUX_LC_F3[i];
                    input F4 = IMUX_LC_F4[i];
                    input DI = IMUX_LC_DI[i];
                    input CE = IMUX_CLB_CE;
                    input CK = IMUX_CLB_CLK;
                    input CLR = IMUX_CLB_RST;
                    output X = OUT_LC_X[i];
                    output Q = OUT_LC_Q[i];
                    output DO = OUT_LC_DO[i];
                }
            }

            for i in 0..4 {
                bel TBUF[i] {
                    input I = OMUX_BUF[i + 4];
                    input T = IMUX_TS;
                    output O = OUT_TBUF[i];
                }
            }

            bel PROGTIE {
                output O = OUT_PROGTIE;
            }
        }

        bel_slot IO[4]: IO;
        bel_slot BUFR: routing;
        bel_slot SCANTEST: SCANTEST;
        bel_slot CIN: CIN;
        bel_slot COUT: COUT;

        tile_class IO_W, IO_E, IO_S, IO_N {
            cell CELL;
            if tile_class IO_W {
                bitrect MAIN: MAIN_W;
            }
            if tile_class IO_E {
                bitrect MAIN: MAIN_E;
            }
            if tile_class IO_S {
                bitrect MAIN: MAIN_S;
            }
            if tile_class IO_N {
                bitrect MAIN: MAIN_N;
            }

            switchbox INT;

            for i in 0..4 {
                bel IO[i] {
                    if tile_class [IO_S, IO_N] {
                        input O = IMUX_IO_O_SN[i];
                    } else {
                        input O = IMUX_IO_O[i];
                    }
                    input T = IMUX_IO_T[i];
                    output I = OUT_IO_I[i];
                }
            }

            for i in 0..4 {
                bel TBUF[i] {
                    input I = OMUX_BUF[i];
                    input T = IMUX_TS;
                    output O = OUT_TBUF[i];
                }
            }

            switchbox BUFR {
                if tile_class IO_W {
                    permabuf GCLK_W = IMUX_GIN;
                }
                if tile_class IO_E {
                    permabuf GCLK_E = IMUX_GIN;
                }
                if tile_class IO_S {
                    permabuf GCLK_S = IMUX_GIN;
                }
                if tile_class IO_N {
                    permabuf GCLK_N = IMUX_GIN;
                }
            }

            if tile_class IO_S {
                bel CIN {
                    input IN = IMUX_BOT_CIN;
                }
                bel SCANTEST;
            }
            if tile_class IO_N {
                bel COUT {
                    output OUT = OUT_TOP_COUT;
                }
            }
        }

        bel_slot BUFG: routing;
        bel_slot CLKIOB: CLKIOB;
        bel_slot RDBK: RDBK;
        bel_slot STARTUP: STARTUP;
        bel_slot BSCAN: BSCAN;
        bel_slot OSC_SE: OSC_SE;
        bel_slot OSC_NE: OSC_NE;
        bel_slot BYPOSC: BYPOSC;
        bel_slot BSUPD: BSUPD;
        bel_slot MISC_SW: MISC_SW;
        bel_slot MISC_SE: MISC_SE;
        bel_slot MISC_NW: MISC_NW;
        bel_slot MISC_NE: MISC_NE;

        tile_class CNR_SW {
            cell CELL;
            bitrect MAIN: MAIN_SW;

            switchbox INT;
            switchbox BUFG {
                permabuf GCLK_SW = IMUX_BUFG;
            }

            bel CLKIOB {
                output OUT = OUT_CLKIOB;
            }

            bel RDBK {
                input CK = IMUX_RDBK_RCLK;
                input TRIG = IMUX_RDBK_TRIG;
                output DATA = OUT_RDBK_DATA;
                output RIP = OUT_RDBK_RIP;
            }

            bel MISC_SW;
        }

        tile_class CNR_SE {
            cell CELL;
            bitrect MAIN: MAIN_SE;

            switchbox INT;
            switchbox BUFG {
                permabuf GCLK_SE = IMUX_BUFG;
            }

            bel CLKIOB {
                output OUT = OUT_CLKIOB;
            }

            bel STARTUP {
                input CLK = IMUX_STARTUP_SCLK;
                input GR = IMUX_STARTUP_GRST;
                input GTS = IMUX_STARTUP_GTS;
                output DONEIN = OUT_STARTUP_DONEIN;
                output Q1Q4 = OUT_STARTUP_Q1Q4;
                output Q2 = OUT_STARTUP_Q2;
                output Q3 = OUT_STARTUP_Q3;
            }

            bel OSC_SE;
            bel MISC_SE;
        }

        tile_class CNR_NW {
            cell CELL;
            bitrect MAIN: MAIN_NW;

            switchbox INT;
            switchbox BUFG {
                permabuf GCLK_NW = IMUX_BUFG;
            }

            bel CLKIOB {
                output OUT = OUT_CLKIOB;
            }

            bel BSCAN {
                input TDO1 = IMUX_BSCAN_TDO1;
                input TDO2 = IMUX_BSCAN_TDO2;
                output DRCK = OUT_BSCAN_DRCK;
                output IDLE = OUT_BSCAN_IDLE;
                output RESET = OUT_BSCAN_RESET;
                output SEL1 = OUT_BSCAN_SEL1;
                output SEL2 = OUT_BSCAN_SEL2;
                output SHIFT = OUT_BSCAN_SHIFT;
                output UPDATE = OUT_BSCAN_UPDATE;
            }

            bel MISC_NW;
        }

        tile_class CNR_NE {
            cell CELL;
            bitrect MAIN: MAIN_NE;

            switchbox INT;
            switchbox BUFG {
                permabuf GCLK_NE = IMUX_BUFG;
            }

            bel CLKIOB {
                output OUT = OUT_CLKIOB;
            }

            bel OSC_NE {
                input C = IMUX_OSC_OCLK;
                output OSC1 = OUT_OSC_OSC1;
                output OSC2 = OUT_OSC_OSC2;
            }

            bel BYPOSC {
                input I = IMUX_BYPOSC_PUMP;
            }

            bel BSUPD {
                output O = OUT_BSUPD;
            }

            bel MISC_NE;
        }
    }

    tile_slot LLH {
        bel_slot LLH: routing;

        tile_class LLH {
            cell W, E;
            bitrect LLH: LLH;
            switchbox LLH;
        }
        tile_class LLH_S {
            cell W, E;
            bitrect LLH: LLH_S;
            switchbox LLH;
        }
        tile_class LLH_N {
            cell W, E;
            bitrect LLH: LLH_N;
            switchbox LLH;
        }
    }

    tile_slot LLV {
        bel_slot LLV: routing;

        tile_class LLV {
            cell S, N;
            bitrect LLV: LLV;
            switchbox LLV;
        }
        tile_class LLV_W {
            cell S, N;
            bitrect LLV: LLV_W;
            bitrect MAIN: MAIN_W;
            switchbox LLV;
        }
        tile_class LLV_E {
            cell S, N;
            bitrect LLV: LLV_E;
            bitrect MAIN: MAIN_E;
            switchbox LLV;
        }
    }

    connector_slot W {
        opposite E;

        connector_class PASS_W {
            pass SINGLE_W = SINGLE_E;
            pass SINGLE_IO_S_W = SINGLE_IO_S_E;
            pass DBL_H_E = DBL_H_M;
            for i in 0..4 {
                pass OMUX_BUF_E[i] = OMUX_BUF[i];
            }
        }

        connector_class CNR_SW {
            reflect SINGLE_IO_S_E = SINGLE_IO_W_N;
        }
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            pass SINGLE_IO_N_E = SINGLE_IO_N_W;
            pass DBL_H_W = DBL_H_M;
            for i in 0..4 {
                pass OMUX_BUF_W[i] = OMUX_BUF[i];
            }
        }

        connector_class CNR_NE {
            reflect SINGLE_IO_N_W = SINGLE_IO_E_S;
        }
    }

    connector_slot S {
        opposite N;

        connector_class PASS_S {
            pass SINGLE_IO_E_S = SINGLE_IO_E_N;
            pass DBL_V_N = DBL_V_M;
            for i in 0..4 {
                pass OMUX_BUF_N[i] = OMUX_BUF[i];
            }
        }

        connector_class CNR_SE {
            reflect SINGLE_IO_E_N = SINGLE_IO_S_W;
        }
    }

    connector_slot N {
        opposite S;

        connector_class PASS_N {
            pass SINGLE_N = SINGLE_S;
            pass SINGLE_IO_W_N = SINGLE_IO_W_S;
            pass DBL_V_S = DBL_V_M;
            for i in 0..4 {
                pass OMUX_BUF_S[i] = OMUX_BUF[i];
            }
        }

        connector_class CNR_NW {
            reflect SINGLE_IO_W_S = SINGLE_IO_N_E;
        }
    }
}
