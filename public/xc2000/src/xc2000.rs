use prjcombine_tablegen::target_defs;

target_defs! {
    enum FF_MODE { FF, LATCH }
    enum CLB_MODE { FGM, FG }

    enum CLB_MUX_I1 { A, B }
    enum CLB_MUX_I2 { B, C }
    enum CLB_MUX_I3 { C, D, Q }
    enum CLB_MUX_XY { F, G, Q }
    enum CLB_MUX_RES { D, G, TIE_0 }
    enum CLB_MUX_SET { A, F, TIE_0 }

    bel_class CLB {
        input A, B, C, D;
        input K;
        output X, Y;
        attribute F, G: bitvec[8];
        attribute MODE: CLB_MODE;
        attribute FF_MODE: FF_MODE;
        attribute MUX_F1, MUX_G1: CLB_MUX_I1;
        attribute MUX_F2, MUX_G2: CLB_MUX_I2;
        attribute MUX_F3, MUX_G3: CLB_MUX_I3;
        attribute MUX_X, MUX_Y: CLB_MUX_XY;
        attribute MUX_RES: CLB_MUX_RES;
        attribute MUX_SET: CLB_MUX_SET;
        attribute READBACK_Q: bitvec[1];
    }

    enum IO_MUX_I { PAD, Q }

    bel_class IO {
        input O, T;
        input K;
        output I;
        pad PAD: inout;
        attribute MUX_I: IO_MUX_I;
        attribute READBACK_Q: bitvec[1];
    }

    bel_class OSC {
        output O;
    }

    enum READBACK_MODE { COMMAND, ONCE, DISABLE }

    bel_class MISC_SW {
        pad M0: input;
        pad M1: inout;
        attribute READBACK_MODE: READBACK_MODE;
    }

    enum STARTUP_SEQ { BEFORE, AFTER }

    bel_class MISC_SE {
        pad PROG_B: input;
        pad DONE: inout;
        attribute TLC: bool;
        attribute DONE_PULLUP: bool;
        attribute REPROGRAM_ENABLE: bool;
    }

    enum IO_INPUT_MODE { TTL, CMOS }

    bel_class MISC_NW {
        pad PWRDWN_B: input;
        attribute IO_INPUT_MODE: IO_INPUT_MODE;
    }

    bel_class MISC_NE {
        pad CCLK: inout;
        attribute TAC: bool;
    }

    bel_class MISC_E {
        attribute TLC: bool;
    }

    region_slot GLOBAL;
    region_slot LONG_H;
    region_slot LONG_V;

    wire TIE_0: tie 0;
    wire TIE_1: tie 1;
    wire SPECIAL_CLB_C: special;
    wire SPECIAL_CLB_G: special;

    wire SINGLE_H[4]: multi_root;
    wire SINGLE_H_E[4]: multi_branch W;
    wire SINGLE_HS[4]: multi_root;
    wire SINGLE_HS_E[4]: multi_branch W;
    wire SINGLE_HN[4]: multi_root;
    wire SINGLE_HN_E[4]: multi_branch W;

    wire SINGLE_V[5]: multi_root;
    wire SINGLE_V_S[5]: multi_branch N;
    wire SINGLE_VW[4]: multi_root;
    wire SINGLE_VW_S[4]: multi_branch N;
    wire SINGLE_VE[4]: multi_root;
    wire SINGLE_VE_S[4]: multi_branch N;

    wire LONG_H: regional LONG_H;
    wire LONG_HS: regional LONG_H;
    wire LONG_IO_S: regional LONG_H;
    wire LONG_IO_N: regional LONG_H;

    wire LONG_V[2]: regional LONG_V;
    wire LONG_VE[2]: regional LONG_V;
    wire LONG_IO_W: regional LONG_V;
    wire LONG_IO_E: regional LONG_V;

    wire GCLK: regional GLOBAL;
    wire ACLK: regional GLOBAL;
    wire IOCLK_W: regional GLOBAL;
    wire IOCLK_E: regional GLOBAL;
    wire IOCLK_S: regional GLOBAL;
    wire IOCLK_N: regional GLOBAL;

    wire IMUX_CLB_A: mux;
    wire IMUX_CLB_B: mux;
    wire IMUX_CLB_C: mux;
    wire IMUX_CLB_D: mux;
    wire IMUX_CLB_D_N: branch S;
    wire IMUX_CLB_K: mux;
    wire IMUX_IO_W_O[2]: mux;
    wire IMUX_IO_W_T[2]: mux;
    wire IMUX_IO_E_O[2]: mux;
    wire IMUX_IO_E_T[2]: mux;
    wire IMUX_IO_S_O[2]: mux;
    wire IMUX_IO_S_T[2]: mux;
    wire IMUX_IO_N_O[2]: mux;
    wire IMUX_IO_N_T[2]: mux;
    wire IMUX_BUFG: mux;

    wire OUT_CLB_X: bel;
    wire OUT_CLB_X_E: branch W;
    wire OUT_CLB_X_S: branch N;
    wire OUT_CLB_X_N: branch S;
    wire OUT_CLB_Y: bel;
    wire OUT_CLB_Y_E: branch W;
    wire OUT_IO_W_I[2]: bel;
    wire OUT_IO_W_I_S1: branch N;
    wire OUT_IO_E_I[2]: bel;
    wire OUT_IO_E_I_S1: branch N;
    wire OUT_IO_S_I[2]: bel;
    wire OUT_IO_S_I_E1: branch W;
    wire OUT_IO_N_I[2]: bel;
    wire OUT_IO_N_I_E1: branch W;
    wire OUT_OSC: bel;

    bitrect MAIN = vertical (rev 18, rev 8);
    bitrect MAIN_W = vertical (rev 21, rev 8);
    bitrect MAIN_E = vertical (rev 27, rev 8);
    bitrect MAIN_S = vertical (rev 18, rev 12);
    bitrect MAIN_SW = vertical (rev 21, rev 12);
    bitrect MAIN_SE = vertical (rev 27, rev 12);
    bitrect MAIN_N = vertical (rev 18, rev 9);
    bitrect MAIN_NW = vertical (rev 21, rev 9);
    bitrect MAIN_NE = vertical (rev 27, rev 9);

    bitrect BIDIH = vertical (rev 2, rev 8);
    bitrect BIDIH_S = vertical (rev 2, rev 12);
    bitrect BIDIH_N = vertical (rev 2, rev 9);

    bitrect BIDIV = vertical (rev 18, rev 1);
    bitrect BIDIV_W = vertical (rev 21, rev 1);
    bitrect BIDIV_E = vertical (rev 27, rev 1);

    tile_slot MAIN {
        bel_slot INT: routing;

        bel_slot CLB: CLB;
        bel_slot IO_W[2]: IO;
        bel_slot IO_E[2]: IO;
        bel_slot IO_S[2]: IO;
        bel_slot IO_N[2]: IO;
        bel_slot BUFG: routing;
        bel_slot OSC: OSC;
        bel_slot MISC_SW: MISC_SW;
        bel_slot MISC_SE: MISC_SE;
        bel_slot MISC_NW: MISC_NW;
        bel_slot MISC_NE: MISC_NE;
        bel_slot MISC_E: MISC_E;

        tile_class CLB, CLB_W, CLB_E, CLB_MW, CLB_ME,
                CLB_S, CLB_SW, CLB_SE, CLB_SE1,
                CLB_N, CLB_NW, CLB_NE, CLB_NE1
        {
            cell CELL;
            if tile_class [CLB_W, CLB_MW, CLB_NW, CLB_E, CLB_ME, CLB_NE] {
                cell S;
            }
            if tile_class [CLB_S, CLB_SE1, CLB_SW, CLB_N, CLB_NE1, CLB_NW] {
                cell E;
            }

            if tile_class CLB {
                bitrect MAIN: MAIN;
            }
            if tile_class [CLB_W, CLB_MW] {
                bitrect MAIN: MAIN_W;
            }
            if tile_class [CLB_E, CLB_ME] {
                bitrect MAIN: MAIN_E;
            }
            if tile_class CLB_S {
                bitrect MAIN: MAIN_S;
                bitrect MAIN_E: MAIN_S;
            }
            if tile_class CLB_SE1 {
                bitrect MAIN: MAIN_S;
                bitrect MAIN_E: MAIN_SE;
            }
            if tile_class CLB_SW {
                bitrect MAIN: MAIN_SW;
                bitrect MAIN_E: MAIN_S;
            }
            if tile_class CLB_SE {
                bitrect MAIN: MAIN_SE;
            }
            if tile_class CLB_N {
                bitrect MAIN: MAIN_N;
                bitrect MAIN_E: MAIN_N;
            }
            if tile_class CLB_NE1 {
                bitrect MAIN: MAIN_N;
                bitrect MAIN_E: MAIN_NE;
            }
            if tile_class CLB_NW {
                bitrect MAIN: MAIN_NW;
                bitrect MAIN_E: MAIN_N;
            }
            if tile_class CLB_NE {
                bitrect MAIN: MAIN_NE;
            }

            switchbox INT {
                // filled elsewhere
            }

            bel CLB {
                input A = CELL.IMUX_CLB_A;
                input B = CELL.IMUX_CLB_B;
                input C = CELL.IMUX_CLB_C;
                input D = CELL.IMUX_CLB_D_N;
                input K = CELL.IMUX_CLB_K;
                output X = CELL.OUT_CLB_X;
                output Y = CELL.OUT_CLB_Y;

                if tile_class [CLB, CLB_W, CLB_MW, CLB_N, CLB_NE1, CLB_NW] {
                    attribute READBACK_Q @!MAIN[3][2];
                }
                if tile_class [CLB_E, CLB_ME, CLB_NE] {
                    attribute READBACK_Q @!MAIN[12][2];
                }
                if tile_class [CLB_SE, CLB_SE1, CLB_SW] {
                    attribute READBACK_Q @!MAIN[3][6];
                }
                if tile_class [CLB_SE] {
                    attribute READBACK_Q @!MAIN[12][6];
                }
            }

            if tile_class [CLB_W, CLB_SW] {
                bel IO_W[0] {
                    input O = CELL.IMUX_IO_W_O[0];
                    input T = CELL.IMUX_IO_W_T[0];
                    input K = CELL.IOCLK_W;
                    output I = CELL.OUT_IO_W_I[0];

                    if tile_class CLB_W {
                        attribute READBACK_Q @!MAIN[18][5];
                    }
                    if tile_class CLB_SW {
                        attribute READBACK_Q @!MAIN[18][9];
                    }
                }
            }
            if tile_class [CLB_W, CLB_MW, CLB_NW] {
                bel IO_W[1] {
                    input O = CELL.IMUX_IO_W_O[1];
                    input T = CELL.IMUX_IO_W_T[1];
                    input K = CELL.IOCLK_W;
                    output I = CELL.OUT_IO_W_I[1];

                    attribute READBACK_Q @!MAIN[20][0];
                }
            }

            if tile_class [CLB_E, CLB_SE] {
                bel IO_E[0] {
                    input O = CELL.IMUX_IO_E_O[0];
                    input T = CELL.IMUX_IO_E_T[0];
                    input K = CELL.IOCLK_E;
                    output I = CELL.OUT_IO_E_I[0];

                    if tile_class CLB_E {
                        attribute READBACK_Q @!MAIN[0][3];
                    }
                    if tile_class CLB_SE {
                        attribute READBACK_Q @!MAIN[0][7];
                    }
                }
            }
            if tile_class [CLB_E, CLB_ME, CLB_NE] {
                bel IO_E[1] {
                    input O = CELL.IMUX_IO_E_O[1];
                    input T = CELL.IMUX_IO_E_T[1];
                    input K = CELL.IOCLK_E;
                    output I = CELL.OUT_IO_E_I[1];

                    attribute READBACK_Q @!MAIN[8][2];
                }
            }

            if tile_class [CLB_S, CLB_SW, CLB_SE, CLB_SE1] {
                for i in 0..2 {
                    bel IO_S[i] {
                        input O = CELL.IMUX_IO_S_O[i];
                        input T = CELL.IMUX_IO_S_T[i];
                        input K = CELL.IOCLK_S;
                        output I = CELL.OUT_IO_S_I[i];

                        if bel_slot IO_S[0] {
                            if tile_class [CLB_S, CLB_SW, CLB_SE1] {
                                attribute READBACK_Q @!MAIN[4][1];
                            } else {
                                attribute READBACK_Q @!MAIN[13][1];
                            }
                        } else {
                            if tile_class [CLB_S, CLB_SW, CLB_SE1] {
                                attribute READBACK_Q @!MAIN[8][0];
                            } else {
                                attribute READBACK_Q @!MAIN[17][0];
                            }
                        }
                    }
                }
            }

            if tile_class [CLB_N, CLB_NW, CLB_NE, CLB_NE1] {
                for i in 0..2 {
                    bel IO_N[i] {
                        input O = CELL.IMUX_IO_N_O[i];
                        input T = CELL.IMUX_IO_N_T[i];
                        input K = CELL.IOCLK_N;
                        output I = CELL.OUT_IO_N_I[i];

                        if bel_slot IO_N[0] {
                            if tile_class [CLB_N, CLB_NW, CLB_NE1] {
                                attribute READBACK_Q @!MAIN[4][7];
                            } else {
                                attribute READBACK_Q @!MAIN[13][7];
                            }
                        } else {
                            if tile_class [CLB_N, CLB_NW, CLB_NE1] {
                                attribute READBACK_Q @!MAIN[8][8];
                            } else {
                                attribute READBACK_Q @!MAIN[17][8];
                            }
                        }
                    }
                }
            }

            if tile_class CLB_SW {
                bel MISC_SW;
            }

            if tile_class CLB_NW {
                switchbox BUFG {
                    permabuf CELL.GCLK = CELL.IMUX_BUFG;
                }

                bel MISC_NW;
            }

            if tile_class CLB_SE {
                switchbox BUFG {
                    permabuf CELL.ACLK = CELL.IMUX_BUFG;
                }

                bel OSC {
                    output O = CELL.OUT_OSC;
                }

                bel MISC_SE {
                    attribute TLC @!MAIN[0][2];
                }
            }

            if tile_class CLB_NE {
                bel MISC_NE {
                    attribute TAC @!MAIN[8][8];
                }
            }

            if tile_class CLB_ME {
                bel MISC_E {
                    attribute TLC @!MAIN[0][1];
                }
            }
        }
    }

    tile_slot BIDIH {
        bel_slot BIDIH: routing;

        tile_class BIDIH, BIDIH_S, BIDIH_N {
            cell CELL;

            if tile_class BIDIH {
                bitrect BIDI: BIDIH;
            }
            if tile_class BIDIH_S {
                bitrect BIDI: BIDIH_S;
            }
            if tile_class BIDIH_N {
                bitrect BIDI: BIDIH_N;
            }
            switchbox BIDIH {
                if tile_class BIDIH {
                    for i in 0..4 {
                        bidi W CELL.SINGLE_H_E[i];
                    }
                }
                if tile_class BIDIH_S {
                    for i in 0..4 {
                        bidi W CELL.SINGLE_H_E[i];
                        bidi W CELL.SINGLE_HS_E[i];
                    }
                }
                if tile_class BIDIH_N {
                    for i in 0..4 {
                        bidi W CELL.SINGLE_HN_E[i];
                    }
                }
            }
        }
    }

    tile_slot BIDIV {
        bel_slot BIDIV: routing;

        tile_class BIDIV, BIDIV_W, BIDIV_E {
            cell CELL_S;

            if tile_class BIDIV {
                bitrect BIDI: BIDIV;
            }
            if tile_class BIDIV_W {
                bitrect BIDI: BIDIV_W;
            }
            if tile_class BIDIV_E {
                bitrect BIDI: BIDIV_E;
                bitrect MAIN_N: MAIN_E;
            }
            switchbox BIDIV {
                if tile_class BIDIV {
                    for i in 0..5 {
                        bidi N CELL_S.SINGLE_V_S[i];
                    }
                }
                if tile_class BIDIV_E {
                    for i in 0..5 {
                        bidi N CELL_S.SINGLE_V_S[i];
                    }
                    for i in 0..4 {
                        bidi N CELL_S.SINGLE_VE_S[i];
                    }
                }
                if tile_class BIDIV_W {
                    for i in 0..4 {
                        bidi N CELL_S.SINGLE_VW_S[i];
                    }
                }
            }
        }
    }

    connector_slot W {
        opposite E;

        connector_class PASS_W {
            pass SINGLE_H_E = SINGLE_H;
            pass SINGLE_HS_E = SINGLE_HS;
            pass SINGLE_HN_E = SINGLE_HN;

            pass OUT_CLB_X_E = OUT_CLB_X;
            pass OUT_CLB_Y_E = OUT_CLB_Y;
            pass OUT_IO_S_I_E1 = OUT_IO_S_I[1];
            pass OUT_IO_N_I_E1 = OUT_IO_N_I[1];
        }
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            // nothing
        }
    }

    connector_slot S {
        opposite N;

        connector_class PASS_S {
            pass IMUX_CLB_D_N = IMUX_CLB_D;

            pass OUT_CLB_X_N = OUT_CLB_X;
        }
    }

    connector_slot N {
        opposite S;

        connector_class PASS_N {
            pass SINGLE_V_S = SINGLE_V;
            pass SINGLE_VW_S = SINGLE_VW;
            pass SINGLE_VE_S = SINGLE_VE;

            pass OUT_CLB_X_S = OUT_CLB_X;
            pass OUT_IO_W_I_S1 = OUT_IO_W_I[1];
            pass OUT_IO_E_I_S1 = OUT_IO_E_I[1];
        }
    }
}
