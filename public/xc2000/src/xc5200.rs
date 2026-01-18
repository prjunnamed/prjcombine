use prjcombine_tablegen::target_defs;

target_defs! {
    // TODO: bel classes
    bel_class LC {
        input F1, F2, F3, F4;
        input DI;
        input CE, CK, CLR;
        output X, Q, DO;
    }

    bel_class TBUF {
        input I, T;
        output O;
    }

    bel_class VCC_GND {
        output O;
    }

    bel_class IO {
        input O, T;
        output I;
    }

    bel_class SCANTEST;
    bel_class CIN {
        input IN;
    }
    bel_class COUT {
        output OUT;
    }
    bel_class CLKIOB {
        output OUT;
    }

    bel_class RDBK {
        input CK, TRIG;
        output DATA, RIP;
    }
    bel_class STARTUP {
        input CLK, GR, GTS;
        output DONEIN, Q1Q4, Q2, Q3;
    }
    bel_class BSCAN {
        input TDO1, TDO2;
        output DRCK, IDLE, RESET, SEL1, SEL2, SHIFT, UPDATE;
    }
    bel_class OSC {
        input C;
        output OSC1, OSC2;
    }
    bel_class BYPOSC {
        input I;
    }
    bel_class BSUPD {
        output O;
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
    wire OUT_PWRGND: bel;
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
        bel_slot VCC_GND: VCC_GND;

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

            bel VCC_GND {
                output O = OUT_PWRGND;
            }
            // TODO
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
                    input O = IMUX_IO_O[i];
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
            // TODO
        }

        bel_slot BUFG: routing;
        bel_slot CLKIOB: CLKIOB;
        bel_slot RDBK: RDBK;
        bel_slot STARTUP: STARTUP;
        bel_slot BSCAN: BSCAN;
        bel_slot OSC: OSC;
        bel_slot BYPOSC: BYPOSC;
        bel_slot BSUPD: BSUPD;

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

            bel OSC {
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
            switchbox LLV;
        }
        tile_class LLV_E {
            cell S, N;
            bitrect LLV: LLV_E;
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
