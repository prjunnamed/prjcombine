use prjcombine_tablegen::target_defs;

target_defs! {
    enum CLB_MODE { FGM, FG }
    enum CLB_MUX_I2 { B, QX, QY }
    enum CLB_MUX_I3 { C, QX, QY }
    enum CLB_MUX_I4 { D, E }
    enum CLB_MUX_D { DI, F, G }
    enum CLB_MUX_X { F, QX }
    enum CLB_MUX_Y { G, QY }

    bel_class CLB {
        input A, B, C, D, E;
        input DI, EC, RD, K;
        output X, Y;
        attribute F, G: bitvec[16];
        attribute MODE: CLB_MODE;
        attribute MUX_F2, MUX_G2: CLB_MUX_I2;
        attribute MUX_F3, MUX_G3: CLB_MUX_I3;
        attribute MUX_F4, MUX_G4: CLB_MUX_I4;
        attribute MUX_DX, MUX_DY: CLB_MUX_D;
        attribute MUX_X: CLB_MUX_X;
        attribute MUX_Y: CLB_MUX_Y;
        attribute EC_ENABLE: bool;
        attribute RD_ENABLE: bool;
        attribute READBACK_QX, READBACK_QY: bitvec[1];
    }

    bel_class TBUF {
        input I, T;
        bidir O;
    }

    bel_class PULLUP {
        bidir O;
        attribute ENABLE: bool;
    }

    enum FF_MODE { FF, LATCH }
    enum IO_MUX_O { O, OQ }
    enum IO_SLEW { SLOW, FAST }

    bel_class IO {
        input O, T;
        input IK, OK;
        output I;
        output Q;
        pad PAD: inout;
        attribute IFF_MODE: FF_MODE;
        attribute MUX_O: IO_MUX_O;
        attribute SLEW: IO_SLEW;
        attribute READBACK_I, READBACK_IFF: bitvec[1];
        // TODO: what is this actually
        attribute OSC_PULLUP: bool;
    }

    enum OSC_MODE { DISABLE, ENABLE, DIV2 }

    bel_class OSC {
        output O;
        attribute MODE: OSC_MODE;
    }

    bel_class CLKIOB {
        output I;
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
        // ???
        attribute REPROGRAM_ENABLE: bitvec[7];
        attribute DONETIME: STARTUP_SEQ;
        attribute RESETTIME: STARTUP_SEQ;
        attribute SLOWOSC_HALT: bool;
    }

    enum IO_INPUT_MODE { TTL, CMOS }

    bel_class MISC_NW {
        pad PWRDWN_B: input;
        attribute IO_INPUT_MODE: IO_INPUT_MODE;
    }

    bel_class MISC_NE {
        pad CCLK: inout;
        attribute TAC: bool;
        attribute POR: bool;
    }

    bel_class MISC_E {
        attribute TLC: bool;
    }

    region_slot GLOBAL;
    region_slot LONG_H;
    region_slot LONG_H_IO0;
    region_slot LONG_V;
    region_slot LONG_V_IO0;
    region_slot LONG_V_IO1;

    wire TIE_0: tie 0;
    wire TIE_1: tie 1;
    wire SPECIAL_IO_PULLUP: special;

    wire SINGLE_H[5]: multi_root;
    wire SINGLE_H_E[5]: multi_branch W;
    wire SINGLE_H_STUB[5]: multi_root;
    wire SINGLE_HS[5]: multi_root;
    wire SINGLE_HS_E[5]: multi_branch W;
    wire SINGLE_HS_STUB[5]: multi_root;
    wire SINGLE_HN[5]: multi_root;
    wire SINGLE_HN_E[5]: multi_branch W;
    wire SINGLE_HN_STUB[5]: multi_root;

    wire SINGLE_V[5]: multi_root;
    wire SINGLE_V_S[5]: multi_branch N;
    wire SINGLE_V_STUB[5]: multi_root;
    wire SINGLE_V_S_STUB[5]: multi_root;
    wire SINGLE_VW[5]: multi_root;
    wire SINGLE_VW_S[5]: multi_branch N;
    wire SINGLE_VW_STUB[5]: multi_root;
    wire SINGLE_VW_S_STUB[5]: multi_root;
    wire SINGLE_VE[5]: multi_root;
    wire SINGLE_VE_S[5]: multi_branch N;
    wire SINGLE_VE_STUB[5]: multi_root;
    wire SINGLE_VE_S_STUB[5]: multi_root;

    wire LONG_H[2]: regional LONG_H;
    wire LONG_IO_S[2] {
        0 => regional LONG_H_IO0,
        1 => regional LONG_H,
    }
    wire LONG_IO_N[2] {
        0 => regional LONG_H_IO0,
        1 => regional LONG_H,
    }

    wire LONG_V[2]: regional LONG_V;
    wire LONG_IO_W[2] {
        0 => regional LONG_V_IO0,
        1 => regional LONG_V_IO1,
    }
    wire LONG_IO_E[2] {
        0 => regional LONG_V_IO0,
        1 => regional LONG_V_IO1,
    }
    wire GCLK_V: regional LONG_V_IO1;
    wire ACLK_V: regional LONG_V_IO1;

    wire GCLK: regional GLOBAL;
    wire ACLK: regional GLOBAL;
    wire IOCLK_W[2]: regional GLOBAL;
    wire IOCLK_E[2]: regional GLOBAL;
    wire IOCLK_S[2]: regional GLOBAL;
    wire IOCLK_N[2]: regional GLOBAL;

    wire IMUX_CLB_A: mux;
    wire IMUX_CLB_B: mux;
    wire IMUX_CLB_C: mux;
    wire IMUX_CLB_D: mux;
    wire IMUX_CLB_E: mux;
    wire IMUX_CLB_DI: mux;
    wire IMUX_CLB_EC: mux;
    wire IMUX_CLB_RD: mux;
    wire IMUX_CLB_K: mux;
    wire IMUX_IO_W_O[2]: mux;
    wire IMUX_IO_W_T[2]: mux;
    wire IMUX_IO_W_IK[2]: mux;
    wire IMUX_IO_W_OK[2]: mux;
    wire IMUX_IO_E_O[2]: mux;
    wire IMUX_IO_E_T[2]: mux;
    wire IMUX_IO_E_IK[2]: mux;
    wire IMUX_IO_E_OK[2]: mux;
    wire IMUX_IO_S_O[2]: mux;
    wire IMUX_IO_S_T[2]: mux;
    wire IMUX_IO_S_IK[2]: mux;
    wire IMUX_IO_S_OK[2]: mux;
    wire IMUX_IO_N_O[2]: mux;
    wire IMUX_IO_N_T[2]: mux;
    wire IMUX_IO_N_IK[2]: mux;
    wire IMUX_IO_N_OK[2]: mux;
    wire IMUX_TBUF_I[4]: mux;
    wire IMUX_TBUF_T[4]: mux;
    wire IMUX_BUFG: mux;
    wire IMUX_IOCLK[2]: mux;

    wire OUT_CLB_X: bel;
    wire OUT_CLB_X_W: branch E;
    wire OUT_CLB_X_E: branch W;
    wire OUT_CLB_X_ES: branch N;
    wire OUT_CLB_Y: bel;
    wire OUT_CLB_Y_E: branch W;
    wire OUT_CLB_Y_S: branch N;
    wire OUT_IO_W_I[2]: bel;
    wire OUT_IO_W_I_S1: branch N;
    wire OUT_IO_W_Q[2]: bel;
    wire OUT_IO_W_Q_S1: branch N;
    wire OUT_IO_E_I[2]: bel;
    wire OUT_IO_E_I_S1: branch N;
    wire OUT_IO_E_Q[2]: bel;
    wire OUT_IO_E_Q_S1: branch N;
    wire OUT_IO_S_I[2]: bel;
    wire OUT_IO_S_I_E1: branch W;
    wire OUT_IO_S_Q[2]: bel;
    wire OUT_IO_S_Q_E1: branch W;
    wire OUT_IO_N_I[2]: bel;
    wire OUT_IO_N_I_E1: branch W;
    wire OUT_IO_N_Q[2]: bel;
    wire OUT_IO_N_Q_E1: branch W;
    wire OUT_CLKIOB: bel;
    wire OUT_OSC: bel;

    bitrect MAIN = vertical (rev 22, rev 8);
    bitrect MAIN_W = vertical (rev 29, rev 8);
    bitrect MAIN_E = vertical (rev 36, rev 8);
    bitrect MAIN_S = vertical (rev 22, rev 13);
    bitrect MAIN_SW = vertical (rev 29, rev 13);
    bitrect MAIN_SE = vertical (rev 36, rev 13);
    bitrect MAIN_N = vertical (rev 22, rev 10);
    bitrect MAIN_NW = vertical (rev 29, rev 10);
    bitrect MAIN_NE = vertical (rev 36, rev 10);

    bitrect LLV = vertical (rev 22, rev 1);
    bitrect LLV_W = vertical (rev 29, rev 1);
    bitrect LLV_E = vertical (rev 36, rev 1);

    tile_slot MAIN {
        bel_slot INT: routing;

        bel_slot CLB: CLB;
        bel_slot TBUF[2]: TBUF;
        bel_slot TBUF_E[2]: TBUF;
        bel_slot PULLUP_TBUF[2]: PULLUP;
        bel_slot IO_W[2]: IO;
        bel_slot IO_E[2]: IO;
        bel_slot IO_S[2]: IO;
        bel_slot IO_N[2]: IO;
        bel_slot CLKIOB: CLKIOB;
        bel_slot BUFG: routing;
        bel_slot OSC: OSC;
        bel_slot MISC_SW: MISC_SW;
        bel_slot MISC_SE: MISC_SE;
        bel_slot MISC_NW: MISC_NW;
        bel_slot MISC_NE: MISC_NE;

        tile_class
            CLB0, CLB1, CLB2,
            CLB_W0, CLB_W1, CLB_W2,
            CLB_E0, CLB_E1, CLB_E2, CLB_E3,
            CLB_S0, CLB_S1, CLB_S2,
            CLB_SW2_S,
            CLB_SW0_L, CLB_SW1_L, CLB_SW2_L,
            CLB_SE0_S,
            CLB_SE0_L,
            CLB_N0_S, CLB_N1_S, CLB_N2_S,
            CLB_N0_L, CLB_N1_L, CLB_N2_L,
            CLB_NW0_S,
            CLB_NW0_L, CLB_NW1_L, CLB_NW2_L,
            CLB_NE1_S,
            CLB_NE0_L, CLB_NE1_L, CLB_NE2_L
        {
            cell CELL;
            if tile_class [
                CLB0, CLB1, CLB2,
                CLB_W0, CLB_W1, CLB_W2,
                CLB_S0, CLB_S1, CLB_S2,
                CLB_SW2_S,
                CLB_SW0_L, CLB_SW1_L, CLB_SW2_L,
                CLB_N0_S, CLB_N1_S, CLB_N2_S,
                CLB_N0_L, CLB_N1_L, CLB_N2_L,
                CLB_NW0_S,
                CLB_NW0_L, CLB_NW1_L, CLB_NW2_L
            ] {
                cell E;
            }
            if tile_class [
                CLB0, CLB1, CLB2,
                CLB_W0, CLB_W1, CLB_W2,
                CLB_E0, CLB_E1, CLB_E2, CLB_E3,
                CLB_N0_S, CLB_N1_S, CLB_N2_S,
                CLB_N0_L, CLB_N1_L, CLB_N2_L,
                CLB_NW0_S,
                CLB_NW0_L, CLB_NW1_L, CLB_NW2_L,
                CLB_NE1_S,
                CLB_NE0_L, CLB_NE1_L, CLB_NE2_L
            ] {
                cell S;
            }
            if tile_class [
                CLB0, CLB1, CLB2,
                CLB_W0, CLB_W1, CLB_W2,
                CLB_E0, CLB_E1, CLB_E2, CLB_E3,
                CLB_S0, CLB_S1, CLB_S2,
                CLB_SW2_S,
                CLB_SW0_L, CLB_SW1_L, CLB_SW2_L,
                CLB_SE0_S,
                CLB_SE0_L
            ] {
                cell N;
            }

            if tile_class [CLB0, CLB1, CLB2] {
                bitrect MAIN: MAIN;
                // TODO: XXX this can also be MAIN_N
                bitrect MAIN_N: MAIN;
            }
            if tile_class [CLB_W0, CLB_W1, CLB_W2] {
                bitrect MAIN: MAIN_W;
                // TODO: XXX this can also be MAIN_NW
                bitrect MAIN_N: MAIN_W;
            }
            if tile_class [CLB_E0, CLB_E1, CLB_E2, CLB_E3] {
                bitrect MAIN: MAIN_E;
                // TODO: XXX this can also be MAIN_NE
                bitrect MAIN_N: MAIN_E;
            }
            if tile_class [CLB_S0, CLB_S1, CLB_S2] {
                bitrect MAIN: MAIN_S;
                bitrect MAIN_N: MAIN;
            }
            if tile_class [CLB_SW2_S, CLB_SW0_L, CLB_SW1_L, CLB_SW2_L] {
                bitrect MAIN: MAIN_SW;
                bitrect MAIN_N: MAIN_W;
            }
            if tile_class [CLB_SE0_S, CLB_SE0_L] {
                bitrect MAIN: MAIN_SE;
                bitrect MAIN_N: MAIN_E;
            }
            if tile_class [CLB_N0_S, CLB_N1_S, CLB_N2_S, CLB_N0_L, CLB_N1_L, CLB_N2_L] {
                bitrect MAIN: MAIN_N;
            }
            if tile_class [CLB_NW0_S, CLB_NW0_L, CLB_NW1_L, CLB_NW2_L] {
                bitrect MAIN: MAIN_NW;
            }
            if tile_class [CLB_NE1_S, CLB_NE0_L, CLB_NE1_L, CLB_NE2_L] {
                bitrect MAIN: MAIN_NE;
            }

            switchbox INT {
                // filled elsewhere
            }

            bel CLB {
                input A = CELL.IMUX_CLB_A;
                input B = CELL.IMUX_CLB_B;
                input C = CELL.IMUX_CLB_C;
                input D = CELL.IMUX_CLB_D;
                input E = CELL.IMUX_CLB_E;
                input DI = CELL.IMUX_CLB_DI;
                input EC = CELL.IMUX_CLB_EC;
                input RD = CELL.IMUX_CLB_RD;
                input K = CELL.IMUX_CLB_K;
                output X = CELL.OUT_CLB_X;
                output Y = CELL.OUT_CLB_Y;
            }

            for i in 0..2 {
                bel TBUF[i] {
                    input I = CELL.IMUX_TBUF_I[i];
                    input T = CELL.IMUX_TBUF_T[i];
                    bidir O = CELL.LONG_H[i];
                }
            }

            if tile_class [
                CLB_W0, CLB_W1, CLB_W2,
                CLB_SW2_S,
                CLB_SW0_L, CLB_SW1_L, CLB_SW2_L,
                CLB_NW0_S,
                CLB_NW0_L, CLB_NW1_L, CLB_NW2_L
            ] {
                for i in 0..2 {
                    bel IO_W[i] {
                        input O = CELL.IMUX_IO_W_O[i];
                        input T = CELL.IMUX_IO_W_T[i];
                        input IK = CELL.IMUX_IO_W_IK[i];
                        input OK = CELL.IMUX_IO_W_OK[i];
                        output I = CELL.OUT_IO_W_I[i];
                        output Q = CELL.OUT_IO_W_Q[i];

                        if bel_slot IO_W[0] {
                            if tile_class [CLB_W0, CLB_W1, CLB_W2] {
                                attribute READBACK_I @!MAIN[2][6];
                                attribute READBACK_IFF @!MAIN[8][6];
                            } else if tile_class [CLB_SW2_S, CLB_SW0_L, CLB_SW1_L, CLB_SW2_L] {
                                attribute READBACK_I @!MAIN[2][11];
                                attribute READBACK_IFF @!MAIN[8][11];
                            } else if tile_class [CLB_NW0_S, CLB_NW0_L, CLB_NW1_L, CLB_NW2_L] {
                                attribute READBACK_I @!MAIN[10][7];
                                attribute READBACK_IFF @!MAIN[12][5];
                            }
                        } else {
                            if tile_class [CLB_W0, CLB_W1, CLB_W2] {
                                attribute READBACK_I @!MAIN[9][3];
                                attribute READBACK_IFF @!MAIN[22][1];
                            } else if tile_class [CLB_SW2_S, CLB_SW0_L, CLB_SW1_L, CLB_SW2_L] {
                                attribute READBACK_I @!MAIN[9][8];
                                attribute READBACK_IFF @!MAIN[22][6];
                            } else if tile_class [CLB_NW0_S, CLB_NW0_L, CLB_NW1_L, CLB_NW2_L] {
                                attribute READBACK_I @!MAIN[9][3];
                                attribute READBACK_IFF @!MAIN[22][1];
                            }
                        }
                    }

                    bel PULLUP_TBUF[i] {
                        bidir O = CELL.LONG_H[i];
                    }
                }
            }

            if tile_class [
                CLB_E0, CLB_E1, CLB_E2, CLB_E3,
                CLB_SE0_S,
                CLB_SE0_L,
                CLB_NE1_S,
                CLB_NE0_L, CLB_NE1_L, CLB_NE2_L
            ] {
                for i in 0..2 {
                    bel IO_E[i] {
                        input O = CELL.IMUX_IO_E_O[i];
                        input T = CELL.IMUX_IO_E_T[i];
                        input IK = CELL.IMUX_IO_E_IK[i];
                        input OK = CELL.IMUX_IO_E_OK[i];
                        output I = CELL.OUT_IO_E_I[i];
                        output Q = CELL.OUT_IO_E_Q[i];

                        if bel_slot IO_E[0] {
                            if tile_class [CLB_E0, CLB_E1, CLB_E2, CLB_E3] {
                                attribute READBACK_I @!MAIN[13][2];
                                attribute READBACK_IFF @!MAIN[9][5];
                            } else if tile_class [CLB_SE0_S, CLB_SE0_L] {
                                attribute READBACK_I @!MAIN[13][7];
                                attribute READBACK_IFF @!MAIN[9][10];
                            } else if tile_class [CLB_NE1_S, CLB_NE0_L, CLB_NE1_L, CLB_NE2_L] {
                                attribute READBACK_I @!MAIN[13][2];
                                attribute READBACK_IFF @!MAIN[8][4];
                            }
                        } else {
                            if tile_class [CLB_E0, CLB_E1, CLB_E2, CLB_E3] {
                                attribute READBACK_I @!MAIN[5][1];
                                attribute READBACK_IFF @!MAIN[6][1];
                            } else if tile_class [CLB_SE0_S, CLB_SE0_L] {
                                attribute READBACK_I @!MAIN[5][6];
                                attribute READBACK_IFF @!MAIN[6][6];
                            } else if tile_class [CLB_NE1_S, CLB_NE0_L, CLB_NE1_L, CLB_NE2_L] {
                                attribute READBACK_I @!MAIN[5][1];
                                attribute READBACK_IFF @!MAIN[6][1];
                            }
                        }
                    }

                    bel TBUF_E[i] {
                        input I = CELL.IMUX_TBUF_I[i + 2];
                        input T = CELL.IMUX_TBUF_T[i + 2];
                        bidir O = CELL.LONG_H[i];
                    }

                    bel PULLUP_TBUF[i] {
                        bidir O = CELL.LONG_H[i];
                    }
                }
            }

            if tile_class [
                CLB_S0, CLB_S1, CLB_S2,
                CLB_SW2_S,
                CLB_SW0_L, CLB_SW1_L, CLB_SW2_L,
                CLB_SE0_S,
                CLB_SE0_L
            ] {
                for i in 0..2 {
                    bel IO_S[i] {
                        input O = CELL.IMUX_IO_S_O[i];
                        input T = CELL.IMUX_IO_S_T[i];
                        input IK = CELL.IMUX_IO_S_IK[i];
                        input OK = CELL.IMUX_IO_S_OK[i];
                        output I = CELL.OUT_IO_S_I[i];
                        output Q = CELL.OUT_IO_S_Q[i];

                        if bel_slot IO_S[0] {
                            if tile_class [CLB_S0, CLB_S1, CLB_S2] {
                                attribute READBACK_I @!MAIN[14][1];
                                attribute READBACK_IFF @!MAIN[11][0];
                            } else if tile_class [CLB_SW2_S, CLB_SW0_L, CLB_SW1_L, CLB_SW2_L] {
                                attribute READBACK_I @!MAIN[14][1];
                                attribute READBACK_IFF @!MAIN[11][0];
                            } else if tile_class [CLB_SE0_S, CLB_SE0_L] {
                                attribute READBACK_I @!MAIN[28][1];
                                attribute READBACK_IFF @!MAIN[25][0];
                            }
                        } else {
                            if tile_class [CLB_S0, CLB_S1, CLB_S2] {
                                attribute READBACK_I @!MAIN[6][1];
                                attribute READBACK_IFF @!MAIN[10][1];
                            } else if tile_class [CLB_SW2_S, CLB_SW0_L, CLB_SW1_L, CLB_SW2_L] {
                                attribute READBACK_I @!MAIN[6][1];
                                attribute READBACK_IFF @!MAIN[10][1];
                            } else if tile_class [CLB_SE0_S, CLB_SE0_L] {
                                attribute READBACK_I @!MAIN[20][1];
                                attribute READBACK_IFF @!MAIN[24][1];
                            }
                        }
                    }
                }
            }

            if tile_class [
                CLB_N0_S, CLB_N1_S, CLB_N2_S,
                CLB_N0_L, CLB_N1_L, CLB_N2_L,
                CLB_NW0_S,
                CLB_NW0_L, CLB_NW1_L, CLB_NW2_L,
                CLB_NE1_S,
                CLB_NE0_L, CLB_NE1_L, CLB_NE2_L
            ] {
                for i in 0..2 {
                    bel IO_N[i] {
                        input O = CELL.IMUX_IO_N_O[i];
                        input T = CELL.IMUX_IO_N_T[i];
                        input IK = CELL.IMUX_IO_N_IK[i];
                        input OK = CELL.IMUX_IO_N_OK[i];
                        output I = CELL.OUT_IO_N_I[i];
                        output Q = CELL.OUT_IO_N_Q[i];

                        if bel_slot IO_N[0] {
                            if tile_class [CLB_N0_S, CLB_N1_S, CLB_N2_S, CLB_N0_L, CLB_N1_L, CLB_N2_L] {
                                attribute READBACK_I @!MAIN[14][8];
                                attribute READBACK_IFF @!MAIN[11][9];
                            } else if tile_class [CLB_NW0_S, CLB_NW0_L, CLB_NW1_L, CLB_NW2_L] {
                                attribute READBACK_I @!MAIN[14][8];
                                attribute READBACK_IFF @!MAIN[11][9];
                            } else if tile_class [CLB_NE1_S, CLB_NE0_L, CLB_NE1_L, CLB_NE2_L] {
                                attribute READBACK_I @!MAIN[28][8];
                                attribute READBACK_IFF @!MAIN[25][9];
                            }
                        } else {
                            if tile_class [CLB_N0_S, CLB_N1_S, CLB_N2_S, CLB_N0_L, CLB_N1_L, CLB_N2_L] {
                                attribute READBACK_I @!MAIN[6][8];
                                attribute READBACK_IFF @!MAIN[10][8];
                            } else if tile_class [CLB_NW0_S, CLB_NW0_L, CLB_NW1_L, CLB_NW2_L] {
                                attribute READBACK_I @!MAIN[6][8];
                                attribute READBACK_IFF @!MAIN[10][8];
                            } else if tile_class [CLB_NE1_S, CLB_NE0_L, CLB_NE1_L, CLB_NE2_L] {
                                attribute READBACK_I @!MAIN[20][8];
                                attribute READBACK_IFF @!MAIN[24][8];
                            }
                        }
                    }
                }
            }

            if tile_class [CLB_SW2_S, CLB_SW0_L, CLB_SW1_L, CLB_SW2_L] {
                bel MISC_SW;
            }

            if tile_class [CLB_NW0_S, CLB_NW0_L, CLB_NW1_L, CLB_NW2_L] {
                switchbox BUFG {
                    permabuf CELL.GCLK = CELL.IMUX_BUFG;
                }

                bel CLKIOB {
                    output I = CELL.OUT_CLKIOB;
                }

                bel MISC_NW;
            }

            if tile_class [CLB_SE0_S, CLB_SE0_L] {
                switchbox BUFG {
                    permabuf CELL.ACLK = CELL.IMUX_BUFG;
                }

                bel CLKIOB {
                    output I = CELL.OUT_CLKIOB;
                }

                bel OSC {
                    output O = CELL.OUT_OSC;
                }

                bel MISC_SE {
                    attribute TLC @!MAIN[1][0];
                    attribute SLOWOSC_HALT @MAIN[5][0];
                }
            }

            if tile_class [CLB_NE1_S, CLB_NE0_L, CLB_NE1_L, CLB_NE2_L] {
                bel MISC_NE {
                    attribute TAC @!MAIN[0][5];
                    attribute POR @!MAIN[11][9];
                }
            }
        }
    }

    tile_slot LLH {
        bel_slot LLH: routing;

        tile_class LLH_S, LLH_N {
            cell W, E;

            if tile_class LLH_S {
                bitrect MAIN: MAIN_S;
            }
            if tile_class LLH_N {
                bitrect MAIN: MAIN_N;
            }

            switchbox LLH {
                // filled elsewhere
            }
        }
    }

    tile_slot LLV {
        bel_slot LLV: routing;

        tile_class LLV_W, LLV_E, LLV {
            cell S, N;

            if tile_class LLV {
                bitrect LLV: LLV;
            }
            if tile_class LLV_W {
                bitrect LLV: LLV_W;
            }
            if tile_class LLV_E {
                bitrect LLV: LLV_E;
            }

            switchbox LLV {
                // filled elsewhere
            }
        }

        tile_class LLVS_W, LLVS_E {
            cell S, N;

            if tile_class LLVS_W {
                bitrect MAIN: MAIN_W;
            }
            if tile_class LLVS_E {
                bitrect MAIN: MAIN_E;
            }

            switchbox LLV {
                // filled elsewhere
            }
        }
    }

    tile_slot MISC_E {
        bel_slot MISC_E: MISC_E;

        tile_class MISC_E {
            bitrect MAIN: MAIN_E;
            bel MISC_E {
                attribute TLC @!MAIN[0][0];
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
            pass OUT_IO_S_Q_E1 = OUT_IO_S_Q[1];
            pass OUT_IO_N_I_E1 = OUT_IO_N_I[1];
            pass OUT_IO_N_Q_E1 = OUT_IO_N_Q[1];
        }
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            pass OUT_CLB_X_W = OUT_CLB_X;
        }
    }

    connector_slot S {
        opposite N;

        connector_class PASS_S {
            // nothing
        }
    }

    connector_slot N {
        opposite S;

        connector_class PASS_N {
            pass SINGLE_V_S = SINGLE_V;
            pass SINGLE_VW_S = SINGLE_VW;
            pass SINGLE_VE_S = SINGLE_VE;

            pass OUT_CLB_X_ES = OUT_CLB_X_E;
            pass OUT_CLB_Y_S = OUT_CLB_Y;
            pass OUT_IO_W_I_S1 = OUT_IO_W_I[1];
            pass OUT_IO_W_Q_S1 = OUT_IO_W_Q[1];
            pass OUT_IO_E_I_S1 = OUT_IO_E_I[1];
            pass OUT_IO_E_Q_S1 = OUT_IO_E_Q[1];
        }
    }
}
