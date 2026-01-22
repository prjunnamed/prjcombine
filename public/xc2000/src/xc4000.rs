use prjcombine_tablegen::target_defs;

target_defs! {
    variant xc4000;
    variant xc4000h;
    variant xc4000a;
    variant xc4000e;
    variant xc4000ex;
    variant xc4000xv;
    variant xc4000xla;
    variant spartanxl;

    enum CLB_MUX_CTRL { C1, C2, C3, C4 }
    enum CLB_MUX_X { F, H }
    enum CLB_MUX_Y { G, H }
    enum CLB_MUX_XQ { DIN, FFX }
    enum CLB_MUX_YQ { EC, FFY }
    enum CLB_MUX_D { F, G, H, DIN }
    enum CLB_MUX_CIN { COUT_S, COUT_N }
    enum CLB_CARRY_ADDSUB { ADD, SUB, ADDSUB }
    enum CLB_CARRY_PROP { CONST_0, CONST_1, XOR }
    enum CLB_CARRY_FGEN { F1, F3_INV, CONST_OP2_ENABLE }
    enum CLB_RAM_DIMS { _32X1, _16X2 }
    enum CLB_MUX_H0 { G, SR }
    enum CLB_MUX_H2 { F, DIN }
    enum CLB_FF_MODE { FF, LATCH }
    bel_class CLB {
        input F1, F2, F3, F4;
        input G1, G2, G3, G4;
        input C1, C2, C3, C4;
        input K;
        output X, XQ, Y, YQ;

        attribute F, G: bitvec[16];
        attribute H: bitvec[8];
        attribute MUX_H1, MUX_DIN, MUX_SR, MUX_EC: CLB_MUX_CTRL;
        attribute MUX_X: CLB_MUX_X;
        attribute MUX_Y: CLB_MUX_Y;
        attribute MUX_XQ: CLB_MUX_XQ;
        attribute MUX_YQ: CLB_MUX_YQ;
        attribute MUX_DX: CLB_MUX_D;
        attribute MUX_DY: CLB_MUX_D;
        attribute FFX_SRVAL, FFY_SRVAL: bitvec[1];
        attribute FFX_EC_ENABLE, FFY_EC_ENABLE: bool;
        attribute FFX_SR_ENABLE, FFY_SR_ENABLE: bool;
        attribute FFX_CLK_INV: bool;
        attribute FFY_CLK_INV: bool;
        attribute MUX_CIN: CLB_MUX_CIN; // not present on xc4000ex/spartanxl and up
        attribute CARRY_ADDSUB: CLB_CARRY_ADDSUB;
        attribute CARRY_FPROP: CLB_CARRY_PROP;
        attribute CARRY_FGEN: CLB_CARRY_FGEN;
        attribute CARRY_GPROP: CLB_CARRY_PROP; // CONST_0 only supported on xc4000ex/spartanxl and up
        attribute CARRY_OP2_ENABLE: bool;
        attribute READBACK_X, READBACK_Y: bitvec[1];
        attribute READBACK_XQ, READBACK_YQ: bitvec[1];
        attribute F_RAM_ENABLE: bool;
        attribute G_RAM_ENABLE: bool;
        attribute RAM_DIMS: CLB_RAM_DIMS;
        // the following are xc4000e and up only
        attribute RAM_DP_ENABLE: bool;
        attribute RAM_SYNC_ENABLE: bool;
        attribute RAM_CLK_INV: bool;
        attribute MUX_H0: CLB_MUX_H0; // always G on older chips
        attribute MUX_H2: CLB_MUX_H2; // always F on older chips
        // the following are xc4000ex/spartanxl and up only
        attribute FFX_MODE: CLB_FF_MODE;
        attribute FFY_MODE: CLB_FF_MODE;
    }

    bel_class TBUF {
        input I, T;
        bidir O;

        attribute DRIVE1: bool;
        // ?!? present on xc4000ex and up, in CLB and IO_E* tiles only
        attribute DRIVE1_DUP: bool;
    }

    enum IO_PULL { NONE, PULLUP, PULLDOWN }
    // MEDFAST and MEDSLOW only available on xc4000a
    enum IO_SLEW { FAST, MEDFAST, MEDSLOW, SLOW }
    enum IO_MUX_I { I, IQ, IQL }
    // MEDDELAY and SYNC only supported on xc4000ex/spartanxl and up
    enum IO_IFF_D { I, DELAY, MEDDELAY, SYNC }
    enum IO_SYNC_D { I, DELAY }
    // MUX only supported on xc4000ex/spartanxl and up
    enum IO_MUX_O { O1, O1_INV, O2, O2_INV, OQ, MUX }
    enum IO_MUX_OFF_D { O1, O2 }
    enum IO_MUX_T { T, TQ }
    enum IO_DRIVE { _12, _24 }
    bel_class IO {
        input IK, OK;
        // O1 can be used as EC on xc4000e and up
        input O1, O2;
        input T;
        output I1, I2;
        output CLKIN;
        pad PAD: inout;

        attribute SLEW: IO_SLEW;
        attribute PULL: IO_PULL;
        attribute IFF_SRVAL, OFF_SRVAL: bitvec[1];
        attribute READBACK_I1, READBACK_I2, READBACK_OQ: bitvec[1];
        attribute MUX_I1, MUX_I2: IO_MUX_I;
        attribute IFF_D: IO_IFF_D;
        attribute OFF_D_INV: bool;
        attribute MUX_OFF_D: IO_MUX_OFF_D;
        attribute MUX_O: IO_MUX_O;
        attribute OFF_USED: bool;

        // the following are only supported on xc4000e and up
        attribute IFF_CE_ENABLE: bool;
        attribute OFF_CE_ENABLE: bool;

        // the following are only supported on xc4000ex/spartanxl and up
        attribute SYNC_D: IO_SYNC_D;
        // ?!?
        attribute IFF_CE_ENABLE_NO_IQ: bool;

        // the following are only supported on xc4000xla, xc4000xv, spartanxl
        attribute MUX_T: IO_MUX_T;
        attribute DRIVE: IO_DRIVE;

        // only on spartanxl
        attribute _5V_TOLERANT: bool;
    }
    enum IO_STD { CMOS, TTL }
    enum HIO_OMODE { CAP, RES }
    bel_class HIO {
        input O, T;
        output I;
        output CLKIN;
        pad PAD: inout;

        attribute PULL: IO_PULL;
        attribute ISTD: IO_STD;
        attribute OSTD: IO_STD;
        attribute OMODE: HIO_OMODE;
        attribute I_INV: bool;
        attribute READBACK_I: bitvec[1];
    }

    bel_class DEC {
        input I;
        bidir O1, O2, O3, O4;
        attribute O1_P: bool;
        attribute O1_N: bool;
        attribute O2_P: bool;
        attribute O2_N: bool;
        attribute O3_P: bool;
        attribute O3_N: bool;
        attribute O4_P: bool;
        attribute O4_N: bool;
    }

    bel_class PULLUP {
        bidir O;
        attribute ENABLE: bool;
    }

    bel_class BUFG {
        input I;
        output O;
        output O_BUFGE;
        // ??? only on xc4000xla/spartanxl and up
        attribute CLK_EN: bool;
        attribute ALT_PAD: bool;
    }

    bel_class BUFF {
        output O;
    }

    bel_class TBUF_SPLITTER {
        bidir W, E;
        attribute BUF_W: bool;
        attribute BUF_E: bool;
        attribute PASS: bool;
        // TODO
    }

    enum CONFIG_RATE { SLOW, FAST }
    enum DONE_TIMING { Q0, Q1Q4, Q2, Q3 }
    enum GTS_GSR_TIMING { Q1Q4, Q2, Q3, DONE_IN }
    enum STARTUP_MUX_CLK { CCLK, USERCLK }
    bel_class STARTUP {
        input CLK;
        input GSR;
        input GTS;
        output DONEIN, Q1Q4, Q2, Q3;
        attribute GSR_ENABLE: bool;
        attribute GTS_ENABLE: bool;
        attribute CONFIG_RATE: CONFIG_RATE;
        attribute CRC: bool;
        attribute DONE_TIMING: DONE_TIMING;
        attribute GTS_TIMING: GTS_GSR_TIMING;
        attribute GSR_TIMING: GTS_GSR_TIMING;
        attribute SYNC_TO_DONE: bool;
        attribute MUX_CLK: STARTUP_MUX_CLK;
        // xc4000ex/spartanxl and up only
        attribute EXPRESS_MODE: bool;
    }

    bel_class READCLK {
        input I;
    }

    bel_class UPDATE {
        output O;
    }

    bel_class OSC {
        output F8M;
        output OUT0;
        output OUT1;
    }

    bel_class TDO {
        input O, T;
        pad PAD: output;
        attribute PULL: IO_PULL;
        attribute BSCAN_ENABLE: bool;
        // only on xc4000xla/spartanxl and up
        attribute BSCAN_STATUS: bool;
        attribute T_ENABLE: bool;
        attribute O_ENABLE: bool;
        // only on spartanxl
        attribute _5V_TOLERANT: bool;
    }

    bel_class MD1 {
        input O, T;
        pad PAD: inout;
        attribute PULL: IO_PULL;
        attribute T_ENABLE: bool;
        attribute O_ENABLE: bool;
        attribute _5V_TOLERANT: bool;
    }

    bel_class IBUF {
        output I;
        pad PAD: input;
        // only on xc4000e and up
        attribute PULL: IO_PULL;
        // only on spartanxl
        attribute _5V_TOLERANT: bool;
    }

    bel_class RDBK {
        input TRIG;
        output DATA;
        output RIP;
        attribute ENABLE: bool;
        attribute READ_ABORT: bool;
        attribute READ_CAPTURE: bool;
    }

    bel_class BSCAN {
        input TDO1, TDO2;
        output DRCK, IDLE, SEL1, SEL2;
        attribute ENABLE: bool;
        attribute CONFIG: bool;
    }

    bel_class CIN {
        input I;
    }

    bel_class COUT {
        output O;
    }

    bel_class MISC_SW {
        attribute TM_BOT: bool;
        // only on xc4000xla, xc4000xv
        attribute IO_5V_TOLERANT: bool;
    }

    enum OSC_MUX_OUT { F500K, F16K, F490, F15 }
    enum OSC_CLK { CCLK, EXTCLK }
    bel_class MISC_SE {
        pad PROG_B: input;
        pad DONE: inout;
        attribute DONE_PULLUP: bool;
        attribute OSC_ENABLE: bool;
        attribute OSC_MUX_OUT0: OSC_MUX_OUT;
        attribute OSC_MUX_OUT1: OSC_MUX_OUT;
        attribute TCTEST: bool;
        // xc4000xla/spartanxl and up only
        attribute TM_OSC: bool;
        attribute OSC_CLK: OSC_CLK;
        // xc4000ex and up only
        attribute FIX_DISCHARGE: bool;
        // only on spartanxl
        attribute PROG_5V_TOLERANT: bool;
        attribute DONE_5V_TOLERANT: bool;
    }

    bel_class MISC_NW {
        attribute IO_ISTD: IO_STD;
        // xc4000e and up only
        attribute IO_OSTD: IO_STD;
        attribute TM_LEFT: bool;
        attribute TM_TOP: bool;
        // xc4000ex/spartanxl and up only
        attribute _3V: bool;
    }

    enum RDBK_MUX_CLK { CCLK, RDBK }
    enum ADDRESS_LINES { _18, _22 }
    bel_class MISC_NE {
        pad CCLK: inout;
        attribute TM_RIGHT: bool;
        attribute TAC: bool;
        attribute READCLK: RDBK_MUX_CLK;
        // only on spartanxl
        attribute CCLK_5V_TOLERANT: bool;
        // only on xc4000ex/spartanxl and up
        attribute ADDRESS_LINES: ADDRESS_LINES;
    }

    enum PUMP {
        INTERNAL,
        EXTERNAL,
    }
    bel_class MISC_W {
        // xc4000ex only
        attribute PUMP: PUMP;
    }

    bel_class MISC_E {
        attribute TLC: bool;
    }

    region_slot GLOBAL;
    region_slot LONG_H;
    region_slot LONG_H_TBUF;
    region_slot DEC_H;
    region_slot LONG_V;
    region_slot DEC_V;
    region_slot GCLK;
    region_slot BUFGE_V;
    region_slot BUFGLS_H;

    wire TIE_0: tie 0;
    wire TIE_1: tie 1;
    wire SPECIAL_CLB_CIN: special;
    wire SPECIAL_CLB_COUT0: special;

    wire SINGLE_H[8]: multi_root;
    wire SINGLE_H_E[8]: multi_branch W;
    wire SINGLE_V[8]: multi_root;
    wire SINGLE_V_S[8]: multi_branch N;

    wire DOUBLE_H0[2]: multi_root;
    wire DOUBLE_H1[2]: multi_branch W;
    wire DOUBLE_H2[2]: multi_branch W;
    wire DOUBLE_V0[2]: multi_root;
    wire DOUBLE_V1[2]: multi_branch N;
    wire DOUBLE_V2[2]: multi_branch N;

    wire DOUBLE_IO_S0[4]: multi_branch W;
    wire DOUBLE_IO_S1[4]: multi_branch W;
    wire DOUBLE_IO_S2[4]: multi_branch W;
    wire DOUBLE_IO_E0[4]: multi_branch S;
    wire DOUBLE_IO_E1[4]: multi_branch S;
    wire DOUBLE_IO_E2[4]: multi_branch S;
    wire DOUBLE_IO_N0[4]: multi_branch E;
    wire DOUBLE_IO_N1[4]: multi_branch E;
    wire DOUBLE_IO_N2[4]: multi_branch E;
    wire DOUBLE_IO_W0[4]: multi_branch N;
    wire DOUBLE_IO_W1[4]: multi_branch N;
    wire DOUBLE_IO_W2[4]: multi_branch N;

    wire DBUF_IO_H[2]: mux;
    wire DBUF_IO_V[2]: mux;

    wire QUAD_H0[3]: multi_root;
    wire QUAD_H1[3]: multi_branch W;
    wire QUAD_H2[3]: multi_branch W;
    wire QUAD_H3[3]: multi_branch W;
    wire QUAD_H4[3]: multi_branch W;
    wire QUAD_V0[3]: multi_root;
    wire QUAD_V1[3]: multi_branch N;
    wire QUAD_V2[3]: multi_branch N;
    wire QUAD_V3[3]: multi_branch N;
    wire QUAD_V4[3]: multi_branch N;

    wire QBUF[3]: mux;

    wire OCTAL_H[9] {
        0 => multi_root,
        1..9 => multi_branch W,
    }
    wire OCTAL_V[9] {
        0 => multi_root,
        1..9 => multi_branch N,
    }

    wire OCTAL_IO_S[9]: multi_branch W;
    wire OCTAL_IO_E[9]: multi_branch S;
    wire OCTAL_IO_N[9]: multi_branch E;
    wire OCTAL_IO_W[9]: multi_branch N;

    wire OBUF: mux;

    wire LONG_H[6] {
        0..2 => regional LONG_H,
        2..4 => regional LONG_H_TBUF,
        4..6 => regional LONG_H,
    }
    wire LONG_H_BUF[6]: mux;
    wire LONG_V[10]: regional LONG_V;
    wire LONG_IO_H[4]: regional LONG_H;
    wire LONG_IO_V[4]: regional LONG_V;

    wire DEC_H[4]: regional DEC_H;
    wire DEC_V[4]: regional DEC_V;

    wire GCLK[8]: regional GCLK;
    wire VCLK: regional LONG_V;
    wire ECLK_H: regional DEC_H;
    wire ECLK_V: regional LONG_V;
    wire BUFGE_H: regional DEC_H;
    wire BUFGE_V[2]: regional BUFGE_V;
    wire BUFGLS[8]: regional GLOBAL;
    wire BUFGLS_H[8]: regional BUFGLS_H;

    wire IMUX_CLB_F1: mux;
    wire IMUX_CLB_F2: mux;
    wire IMUX_CLB_F3: mux;
    wire IMUX_CLB_F4: mux;
    wire IMUX_CLB_G1: mux;
    wire IMUX_CLB_G2: mux;
    wire IMUX_CLB_G3: mux;
    wire IMUX_CLB_G4: mux;
    wire IMUX_CLB_C1: mux;
    wire IMUX_CLB_C2: mux;
    wire IMUX_CLB_C3: mux;
    wire IMUX_CLB_C4: mux;
    wire IMUX_CLB_F2_N: branch S;
    wire IMUX_CLB_G2_N: branch S;
    wire IMUX_CLB_C2_N: branch S;
    wire IMUX_CLB_F3_W: branch E;
    wire IMUX_CLB_G3_W: branch E;
    wire IMUX_CLB_C3_W: branch E;
    wire IMUX_CLB_K: mux;
    wire IMUX_TBUF_I[2]: mux;
    wire IMUX_TBUF_T[2]: mux;
    wire IMUX_IO_O1[2]: mux;
    wire IMUX_IO_OK[2]: mux;
    wire IMUX_IO_IK[2]: mux;
    wire IMUX_IO_T[2]: mux;
    wire IMUX_HIO_O[4]: mux;
    wire IMUX_HIO_T[4]: mux;
    wire IMUX_CIN: mux;
    wire IMUX_STARTUP_CLK: mux;
    wire IMUX_STARTUP_GSR: mux;
    wire IMUX_STARTUP_GTS: mux;
    wire IMUX_READCLK_I: mux;
    wire IMUX_BUFG_H: mux;
    wire IMUX_BUFG_V: mux;
    wire IMUX_TDO_O: mux;
    wire IMUX_TDO_T: mux;
    wire IMUX_RDBK_TRIG: mux;
    wire IMUX_BSCAN_TDO1: mux;
    wire IMUX_BSCAN_TDO2: mux;

    wire OUT_CLB_X: bel;
    wire OUT_CLB_XQ: bel;
    wire OUT_CLB_Y: bel;
    wire OUT_CLB_YQ: bel;
    wire OUT_CLB_X_H: mux;
    wire OUT_CLB_XQ_H: mux;
    wire OUT_CLB_Y_H: mux;
    wire OUT_CLB_YQ_H: mux;
    wire OUT_CLB_X_V: mux;
    wire OUT_CLB_XQ_V: mux;
    wire OUT_CLB_Y_V: mux;
    wire OUT_CLB_YQ_V: mux;
    wire OUT_CLB_X_S: branch N;
    wire OUT_CLB_XQ_S: branch N;
    wire OUT_CLB_Y_E: branch W;
    wire OUT_CLB_YQ_E: branch W;

    wire OUT_IO_SN_I1[2]: bel;
    wire OUT_IO_SN_I2[2]: bel;
    wire OUT_IO_SN_I1_E1: branch W;
    wire OUT_IO_SN_I2_E1: branch W;
    wire OUT_IO_WE_I1[2]: bel;
    wire OUT_IO_WE_I2[2]: bel;
    wire OUT_IO_WE_I1_S1: branch N;
    wire OUT_IO_WE_I2_S1: branch N;

    wire OUT_HIO_I[4]: bel;

    wire OUT_IO_CLKIN: bel;
    wire OUT_IO_CLKIN_W: branch E;
    wire OUT_IO_CLKIN_E: branch W;
    wire OUT_IO_CLKIN_S: branch N;
    wire OUT_IO_CLKIN_N: branch S;

    wire OUT_OSC_MUX1: bel;
    wire OUT_STARTUP_DONEIN: bel;
    wire OUT_STARTUP_Q1Q4: bel;
    wire OUT_STARTUP_Q2: bel;
    wire OUT_STARTUP_Q3: bel;
    wire OUT_UPDATE_O: bel;
    wire OUT_MD0_I: bel;
    wire OUT_RDBK_DATA: bel;

    wire OUT_COUT: bel;
    wire OUT_COUT_E: branch W;

    wire OUT_BUFGE_H: mux;
    wire OUT_BUFGE_V: mux;
    wire OUT_BUFF: mux;

    if variant [xc4000, xc4000h, xc4000e, spartanxl] {
        bitrect MAIN = vertical (rev 36, rev 10);
        bitrect MAIN_W = vertical (rev 26, rev 10);
        bitrect MAIN_E = vertical (rev 41, rev 10);
        bitrect MAIN_S = vertical (rev 36, rev 13);
        bitrect MAIN_SW = vertical (rev 26, rev 13);
        bitrect MAIN_SE = vertical (rev 41, rev 13);
        bitrect MAIN_N = vertical (rev 36, rev 7);
        bitrect MAIN_NW = vertical (rev 26, rev 7);
        bitrect MAIN_NE = vertical (rev 41, rev 7);


        if variant [xc4000, xc4000h, xc4000e] {
            bitrect LLH = vertical (rev 1, rev 10);
            bitrect LLH_S = vertical (rev 1, rev 13);
            bitrect LLH_N = vertical (rev 1, rev 7);

            bitrect LLV = vertical (rev 36, rev 1);
            bitrect LLV_W = vertical (rev 26, rev 1);
            bitrect LLV_E = vertical (rev 41, rev 1);
        }
        if variant spartanxl {
            bitrect LLH = vertical (rev 2, rev 10);
            bitrect LLH_S = vertical (rev 2, rev 13);
            bitrect LLH_N = vertical (rev 2, rev 7);

            bitrect LLV = vertical (rev 36, rev 2);
            bitrect LLV_W = vertical (rev 26, rev 2);
            bitrect LLV_E = vertical (rev 41, rev 2);
        }
    }
    if variant xc4000a {
        bitrect MAIN = vertical (rev 32, rev 10);
        bitrect MAIN_W = vertical (rev 21, rev 10);
        bitrect MAIN_E = vertical (rev 32, rev 10);
        bitrect MAIN_S = vertical (rev 32, rev 10);
        bitrect MAIN_SW = vertical (rev 21, rev 10);
        bitrect MAIN_SE = vertical (rev 32, rev 10);
        bitrect MAIN_N = vertical (rev 32, rev 6);
        bitrect MAIN_NW = vertical (rev 21, rev 6);
        bitrect MAIN_NE = vertical (rev 32, rev 6);

        bitrect LLH = vertical (rev 1, rev 10);
        bitrect LLH_S = vertical (rev 1, rev 10);
        bitrect LLH_N = vertical (rev 1, rev 6);

        bitrect LLV = vertical (rev 32, rev 1);
        bitrect LLV_W = vertical (rev 21, rev 1);
        bitrect LLV_E = vertical (rev 32, rev 1);
    }
    if variant [xc4000ex, xc4000xla] {
        bitrect MAIN = vertical (rev 47, rev 12);
        bitrect MAIN_W = vertical (rev 27, rev 12);
        bitrect MAIN_E = vertical (rev 52, rev 12);
        bitrect MAIN_S = vertical (rev 47, rev 16);
        bitrect MAIN_SW = vertical (rev 27, rev 16);
        bitrect MAIN_SE = vertical (rev 52, rev 16);
        bitrect MAIN_N = vertical (rev 47, rev 8);
        bitrect MAIN_NW = vertical (rev 27, rev 8);
        bitrect MAIN_NE = vertical (rev 52, rev 8);

        bitrect LLHC = vertical (rev 2, rev 12);
        bitrect LLHC_S = vertical (rev 2, rev 16);
        bitrect LLHC_N = vertical (rev 2, rev 8);
        bitrect LLHQ = vertical (rev 1, rev 12);
        bitrect LLHQ_S = vertical (rev 1, rev 16);
        bitrect LLHQ_N = vertical (rev 1, rev 8);

        bitrect LLV = vertical (rev 47, rev 2);
        bitrect LLV_W = vertical (rev 27, rev 2);
        bitrect LLV_E = vertical (rev 52, rev 2);
    }
    if variant xc4000xv {
        bitrect MAIN = vertical (rev 47, rev 13);
        bitrect MAIN_W = vertical (rev 27, rev 13);
        bitrect MAIN_E = vertical (rev 52, rev 13);
        bitrect MAIN_S = vertical (rev 47, rev 17);
        bitrect MAIN_SW = vertical (rev 27, rev 17);
        bitrect MAIN_SE = vertical (rev 52, rev 17);
        bitrect MAIN_N = vertical (rev 47, rev 9);
        bitrect MAIN_NW = vertical (rev 27, rev 9);
        bitrect MAIN_NE = vertical (rev 52, rev 9);

        bitrect LLHC = vertical (rev 2, rev 13);
        bitrect LLHC_S = vertical (rev 2, rev 17);
        bitrect LLHC_N = vertical (rev 2, rev 9);
        bitrect LLHQ = vertical (rev 1, rev 13);
        bitrect LLHQ_S = vertical (rev 1, rev 17);
        bitrect LLHQ_N = vertical (rev 1, rev 9);

        bitrect LLV = vertical (rev 47, rev 2);
        bitrect LLV_W = vertical (rev 27, rev 2);
        bitrect LLV_E = vertical (rev 52, rev 2);
    }

    tile_slot MAIN {
        bel_slot INT: routing;
        bel_slot CLB: CLB;
        bel_slot TBUF[2]: TBUF;

        tile_class CLB, CLB_W, CLB_E, CLB_S, CLB_SW, CLB_SE, CLB_N, CLB_NW, CLB_NE {
            cell CELL, CELL_N, CELL_E;
            bitrect MAIN: MAIN;
            if tile_class [CLB_S, CLB_SW, CLB_SE] {
                bitrect MAIN_S: MAIN_S;
            } else {
                bitrect MAIN_S: MAIN;
            }
            if tile_class [CLB_W, CLB_SW, CLB_NW] {
                bitrect MAIN_W: MAIN_W;
            } else {
                bitrect MAIN_W: MAIN;
            }
            if tile_class [CLB_N, CLB_NW, CLB_NE] {
                bitrect MAIN_N: MAIN_N;
            } else {
                bitrect MAIN_N: MAIN;
            }
            if tile_class [CLB_E, CLB_SE, CLB_NE] {
                bitrect MAIN_E: MAIN_E;
            } else {
                bitrect MAIN_E: MAIN;
            }

            switchbox INT;

            bel CLB {
                input F1 = CELL.IMUX_CLB_F1;
                input G1 = CELL.IMUX_CLB_G1;
                input C1 = CELL.IMUX_CLB_C1;
                input F2 = CELL.IMUX_CLB_F2_N;
                input G2 = CELL.IMUX_CLB_G2_N;
                input C2 = CELL.IMUX_CLB_C2_N;
                input F3 = CELL.IMUX_CLB_F3_W;
                input G3 = CELL.IMUX_CLB_G3_W;
                input C3 = CELL.IMUX_CLB_C3_W;
                input F4 = CELL.IMUX_CLB_F4;
                input G4 = CELL.IMUX_CLB_G4;
                input C4 = CELL.IMUX_CLB_C4;
                input K = CELL.IMUX_CLB_K;
                output X = CELL.OUT_CLB_X;
                output XQ = CELL.OUT_CLB_XQ;
                output Y = CELL.OUT_CLB_Y;
                output YQ = CELL.OUT_CLB_YQ;
            }

            for i in 0..2 {
                bel TBUF[i] {
                    input I = CELL.IMUX_TBUF_I[i];
                    input T = CELL.IMUX_TBUF_T[i];
                    bidir O = CELL.LONG_H[i + 2];
                }
            }
        }

        bel_slot IO[2]: IO;
        bel_slot HIO[4]: HIO;
        bel_slot DEC[3]: DEC;
        bel_slot PULLUP_TBUF[2]: PULLUP;
        bel_slot CIN: CIN;
        bel_slot COUT: COUT;
        bel_slot MISC_W: MISC_W;

        tile_class IO_W0, IO_W1, IO_W0_N, IO_W1_S, IO_W0_F0, IO_W1_F0, IO_W0_F1, IO_W1_F1 {
            cell CELL, CELL_S, CELL_E, CELL_N;
            bitrect MAIN: MAIN_W;
            if tile_class IO_W1_S {
                bitrect MAIN_S: MAIN_SW;
            } else {
                bitrect MAIN_S: MAIN_W;
            }

            switchbox INT;

            if variant xc4000h {
                for i in 0..4 {
                    bel HIO[i] {
                        input O = CELL.IMUX_HIO_O[i];
                        input T = CELL.IMUX_HIO_T[i];
                        output I = CELL.OUT_HIO_I[i];
                        if tile_class IO_W1_S {
                            if bel_slot HIO[3] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                        if tile_class IO_W0_N {
                            if bel_slot HIO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                    }
                }
            } else {
                for i in 0..2 {
                    bel IO[i] {
                        input O1 = CELL.IMUX_IO_O1[i];
                        if bel_slot IO[0] {
                            input O2 = CELL.IMUX_CLB_G3_W;
                        } else {
                            input O2 = CELL.IMUX_CLB_F3_W;
                        }
                        input IK = CELL.IMUX_IO_IK[i];
                        input OK = CELL.IMUX_IO_OK[i];
                        input T = CELL.IMUX_IO_T[i];
                        if variant xc4000ex {
                            if bel_slot IO[0] {
                                if tile_class [IO_W0_F0, IO_W1_F0] {
                                    output CLKIN = CELL.OUT_IO_WE_I1[i];
                                } else {
                                    output I1 = CELL.OUT_IO_WE_I1[i];
                                }
                            } else {
                                if tile_class [IO_W0_F1, IO_W1_F1] {
                                    output CLKIN = CELL.OUT_IO_WE_I1[i];
                                } else {
                                    output I1 = CELL.OUT_IO_WE_I1[i];
                                }
                            }
                        } else {
                            output I1 = CELL.OUT_IO_WE_I1[i];
                        }
                        output I2 = CELL.OUT_IO_WE_I2[i];
                        if tile_class IO_W1_S {
                            if bel_slot IO[1] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                        if tile_class IO_W0_N {
                            if bel_slot IO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                    }
                }
            }

            for i in 0..2 {
                bel TBUF[i] {
                    input I = CELL.IMUX_TBUF_I[i];
                    input T = CELL.IMUX_TBUF_T[i];
                    bidir O = CELL.LONG_H[i + 2];
                }
            }

            for i in 0..2 {
                bel PULLUP_TBUF[i] {
                    bidir O = CELL.LONG_H[i + 2];
                }
            }

            if variant spartanxl {
            } else {
                for i in 0..3 {
                    bel DEC[i] {
                        if bel_slot DEC[0] {
                            input I = CELL.OUT_IO_WE_I1[0];
                        } else if bel_slot DEC[1] {
                            input I = CELL.IMUX_CLB_C3_W;
                        } else if bel_slot DEC[2] {
                            input I = CELL.OUT_IO_WE_I1[1];
                        }
                        bidir O1 = CELL.DEC_V[0];
                        bidir O2 = CELL.DEC_V[1];
                        if variant xc4000a {
                        } else {
                            bidir O3 = CELL.DEC_V[2];
                            bidir O4 = CELL.DEC_V[3];
                        }
                    }
                }
            }

            if variant xc4000ex {
                bel MISC_W;
            }
        }

        tile_class IO_E0, IO_E1, IO_E0_N, IO_E1_S, IO_E0_F0, IO_E1_F0, IO_E0_F1, IO_E1_F1 {
            cell CELL, CELL_S, CELL_N;
            bitrect MAIN: MAIN_E;
            if tile_class IO_E1_S {
                bitrect MAIN_S: MAIN_SE;
            } else {
                bitrect MAIN_S: MAIN_E;
            }
            bitrect MAIN_W: MAIN;

            switchbox INT;

            if variant xc4000h {
                for i in 0..4 {
                    bel HIO[i] {
                        input O = CELL.IMUX_HIO_O[i];
                        input T = CELL.IMUX_HIO_T[i];
                        output I = CELL.OUT_HIO_I[i];
                        if tile_class IO_E1_S {
                            if bel_slot HIO[2] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                        if tile_class IO_E0_N {
                            if bel_slot HIO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                    }
                }
            } else {
                for i in 0..2 {
                    bel IO[i] {
                        input O1 = CELL.IMUX_IO_O1[i];
                        if bel_slot IO[0] {
                            input O2 = CELL.IMUX_CLB_G1;
                        } else {
                            input O2 = CELL.IMUX_CLB_F1;
                        }
                        input IK = CELL.IMUX_IO_IK[i];
                        input OK = CELL.IMUX_IO_OK[i];
                        input T = CELL.IMUX_IO_T[i];
                        if variant xc4000ex {
                            if bel_slot IO[0] {
                                if tile_class [IO_E0_F0, IO_E1_F0] {
                                    output CLKIN = CELL.OUT_IO_WE_I1[i];
                                } else {
                                    output I1 = CELL.OUT_IO_WE_I1[i];
                                }
                            } else {
                                if tile_class [IO_E0_F1, IO_E1_F1] {
                                    output CLKIN = CELL.OUT_IO_WE_I1[i];
                                } else {
                                    output I1 = CELL.OUT_IO_WE_I1[i];
                                }
                            }
                        } else {
                            output I1 = CELL.OUT_IO_WE_I1[i];
                        }
                        output I2 = CELL.OUT_IO_WE_I2[i];
                        if tile_class IO_E1_S {
                            if bel_slot IO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                        if tile_class IO_E0_N {
                            if bel_slot IO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                    }
                }
            }

            for i in 0..2 {
                bel TBUF[i] {
                    input I = CELL.IMUX_TBUF_I[i];
                    input T = CELL.IMUX_TBUF_T[i];
                    bidir O = CELL.LONG_H[i + 2];
                }
            }

            for i in 0..2 {
                bel PULLUP_TBUF[i] {
                    bidir O = CELL.LONG_H[i + 2];
                }
            }

            if variant spartanxl {
            } else {
                for i in 0..3 {
                    bel DEC[i] {
                        if bel_slot DEC[0] {
                            input I = CELL.OUT_IO_WE_I1[0];
                        } else if bel_slot DEC[1] {
                            input I = CELL.IMUX_CLB_C1;
                        } else if bel_slot DEC[2] {
                            input I = CELL.OUT_IO_WE_I1[1];
                        }
                        bidir O1 = CELL.DEC_V[0];
                        bidir O2 = CELL.DEC_V[1];
                        if variant xc4000a {
                        } else {
                            bidir O3 = CELL.DEC_V[2];
                            bidir O4 = CELL.DEC_V[3];
                        }
                    }
                }
            }
        }

        tile_class IO_S0, IO_S1, IO_S0_E, IO_S1_W {
            cell CELL, CELL_N, CELL_E, CELL_W;
            bitrect MAIN: MAIN_S;
            if tile_class IO_S0_E {
                bitrect MAIN_E: MAIN_SE;
            } else {
                bitrect MAIN_E: MAIN_S;
            }

            switchbox INT;

            if variant xc4000h {
                for i in 0..4 {
                    bel HIO[i] {
                        input O = CELL.IMUX_HIO_O[i];
                        input T = CELL.IMUX_HIO_T[i];
                        output I = CELL.OUT_HIO_I[i];
                        if tile_class IO_S1_W {
                            if bel_slot HIO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                        if tile_class IO_S0_E {
                            if bel_slot HIO[3] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                    }
                }
            } else {
                for i in 0..2 {
                    bel IO[i] {
                        input O1 = CELL.IMUX_IO_O1[i];
                        if bel_slot IO[0] {
                            input O2 = CELL.IMUX_CLB_F4;
                        } else {
                            input O2 = CELL.IMUX_CLB_G4;
                        }
                        input IK = CELL.IMUX_IO_IK[i];
                        input OK = CELL.IMUX_IO_OK[i];
                        input T = CELL.IMUX_IO_T[i];
                        output I1 = CELL.OUT_IO_SN_I1[i];
                        output I2 = CELL.OUT_IO_SN_I2[i];
                        if tile_class IO_S1_W {
                            if bel_slot IO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                        if tile_class IO_S0_E {
                            if bel_slot IO[1] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                    }
                }
            }

            if variant spartanxl {
            } else {
                for i in 0..3 {
                    bel DEC[i] {
                        if bel_slot DEC[0] {
                            input I = CELL.OUT_IO_SN_I1[0];
                        } else if bel_slot DEC[1] {
                            input I = CELL.IMUX_CLB_C4;
                        } else if bel_slot DEC[2] {
                            input I = CELL.OUT_IO_SN_I1[1];
                        }
                        bidir O1 = CELL.DEC_H[0];
                        bidir O2 = CELL.DEC_H[1];
                        if variant xc4000a {
                        } else {
                            bidir O3 = CELL.DEC_H[2];
                            bidir O4 = CELL.DEC_H[3];
                        }
                    }
                }
            }

            if variant [xc4000ex, xc4000xv, xc4000xla, spartanxl] {
                bel CIN {
                    input I = CELL.IMUX_CIN;
                }
            }
        }

        tile_class IO_N0, IO_N1, IO_N0_E, IO_N1_W {
            cell CELL, CELL_E, CELL_W;
            bitrect MAIN: MAIN_N;
            bitrect MAIN_S: MAIN;
            if tile_class IO_N0_E {
                bitrect MAIN_E: MAIN_NE;
            } else {
                bitrect MAIN_E: MAIN_N;
            }
            if tile_class IO_N1_W {
                bitrect MAIN_W: MAIN_NW;
            } else {
                bitrect MAIN_W: MAIN_N;
            }

            switchbox INT;

            if variant xc4000h {
                for i in 0..4 {
                    bel HIO[i] {
                        input O = CELL.IMUX_HIO_O[i];
                        input T = CELL.IMUX_HIO_T[i];
                        output I = CELL.OUT_HIO_I[i];
                        if tile_class IO_N1_W {
                            if bel_slot HIO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                        if tile_class IO_N0_E {
                            if bel_slot HIO[2] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                    }
                }
            } else {
                for i in 0..2 {
                    bel IO[i] {
                        input O1 = CELL.IMUX_IO_O1[i];
                        if bel_slot IO[0] {
                            input O2 = CELL.IMUX_CLB_F2_N;
                        } else {
                            input O2 = CELL.IMUX_CLB_G2_N;
                        }
                        input IK = CELL.IMUX_IO_IK[i];
                        input OK = CELL.IMUX_IO_OK[i];
                        input T = CELL.IMUX_IO_T[i];
                        output I1 = CELL.OUT_IO_SN_I1[i];
                        output I2 = CELL.OUT_IO_SN_I2[i];
                        if tile_class IO_N1_W {
                            if bel_slot IO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                        if tile_class IO_N0_E {
                            if bel_slot IO[0] {
                                output CLKIN = CELL.OUT_IO_CLKIN;
                            }
                        }
                    }
                }
            }

            if variant spartanxl {
            } else {
                for i in 0..3 {
                    bel DEC[i] {
                        if bel_slot DEC[0] {
                            input I = CELL.OUT_IO_SN_I1[0];
                        } else if bel_slot DEC[1] {
                            input I = CELL.IMUX_CLB_C2_N;
                        } else if bel_slot DEC[2] {
                            input I = CELL.OUT_IO_SN_I1[1];
                        }
                        bidir O1 = CELL.DEC_H[0];
                        bidir O2 = CELL.DEC_H[1];
                        if variant xc4000a {
                        } else {
                            bidir O3 = CELL.DEC_H[2];
                            bidir O4 = CELL.DEC_H[3];
                        }
                    }
                }
            }

            if variant [xc4000ex, xc4000xv, xc4000xla, spartanxl] {
                bel COUT {
                    output O = CELL.OUT_COUT;
                }
            }
        }

        bel_slot PULLUP_DEC_H[4]: PULLUP;
        bel_slot PULLUP_DEC_V[4]: PULLUP;
        bel_slot BUFG_H: BUFG;
        bel_slot BUFG_V: BUFG;

        bel_slot STARTUP: STARTUP;
        bel_slot READCLK: READCLK;
        bel_slot UPDATE: UPDATE;
        bel_slot OSC: OSC;
        bel_slot TDO: TDO;
        bel_slot MD0: IBUF;
        bel_slot MD1: MD1;
        bel_slot MD2: IBUF;
        bel_slot RDBK: RDBK;
        bel_slot BSCAN: BSCAN;
        bel_slot MISC_SW: MISC_SW;
        bel_slot MISC_SE: MISC_SE;
        bel_slot MISC_NW: MISC_NW;
        bel_slot MISC_NE: MISC_NE;

        tile_class CNR_SW, CNR_SE, CNR_NW, CNR_NE {
            if tile_class CNR_SW {
                cell CELL, CELL_N;
                bitrect MAIN: MAIN_SW;
            } else if tile_class CNR_SE {
                cell CELL;
                bitrect MAIN: MAIN_SE;
            } else if tile_class CNR_NW {
                cell CELL, CELL_E, CELL_S, CELL_SE;
                bitrect MAIN: MAIN_NW;
            } else if tile_class CNR_NE {
                cell CELL, CELL_S;
                bitrect MAIN: MAIN_NE;
                bitrect MAIN_S: MAIN_E;
                bitrect MAIN_W: MAIN_N;
            }

            switchbox INT;

            if variant spartanxl {
            } else if variant xc4000a {
                for i in 0..2 {
                    bel PULLUP_DEC_H[i] {
                        bidir O = CELL.DEC_H[i];
                    }
                }
                for i in 0..2 {
                    bel PULLUP_DEC_V[i] {
                        bidir O = CELL.DEC_V[i];
                    }
                }
            } else {
                for i in 0..4 {
                    bel PULLUP_DEC_H[i] {
                        bidir O = CELL.DEC_H[i];
                    }
                }
                for i in 0..4 {
                    bel PULLUP_DEC_V[i] {
                        bidir O = CELL.DEC_V[i];
                    }
                }
            }

            bel BUFG_H {
                input I = CELL.IMUX_BUFG_H;
                if tile_class CNR_SW {
                    output O = CELL.BUFGLS[2];
                } else if tile_class CNR_SE {
                    output O = CELL.BUFGLS[3];
                } else if tile_class CNR_NW {
                    output O = CELL.BUFGLS[7];
                } else if tile_class CNR_NE {
                    output O = CELL.BUFGLS[6];
                }
                if variant [xc4000ex, xc4000xv, xc4000xla] {
                    output O_BUFGE = CELL.OUT_BUFGE_H;
                }
            }
            bel BUFG_V {
                input I = CELL.IMUX_BUFG_V;
                if tile_class CNR_SW {
                    output O = CELL.BUFGLS[1];
                } else if tile_class CNR_SE {
                    output O = CELL.BUFGLS[4];
                } else if tile_class CNR_NW {
                    output O = CELL.BUFGLS[0];
                } else if tile_class CNR_NE {
                    output O = CELL.BUFGLS[5];
                }
                if variant [xc4000ex, xc4000xv, xc4000xla] {
                    output O_BUFGE = CELL.OUT_BUFGE_V;
                }
            }

            if tile_class CNR_SW {
                bel MISC_SW;
                if variant spartanxl {
                    bel MD0;
                    bel MD1;
                    bel MD2;
                } else {
                    bel MD0 {
                        output I = CELL.OUT_MD0_I;
                    }
                    bel MD1 {
                        input O = CELL.IMUX_IO_O1[1];
                        input T = CELL.IMUX_IO_IK[1];
                    }
                    bel MD2 {
                        output I = CELL.OUT_IO_SN_I1[1];
                    }
                }
                bel RDBK {
                    input TRIG = CELL.IMUX_RDBK_TRIG;
                    output DATA = CELL.OUT_RDBK_DATA;
                    output RIP = CELL.OUT_IO_SN_I2[1];
                }
            } else if tile_class CNR_SE {
                bel MISC_SE;
                bel STARTUP {
                    input CLK = CELL.IMUX_STARTUP_CLK;
                    input GSR = CELL.IMUX_STARTUP_GSR;
                    input GTS = CELL.IMUX_STARTUP_GTS;
                    output DONEIN = CELL.OUT_STARTUP_DONEIN;
                    output Q1Q4 = CELL.OUT_STARTUP_Q1Q4;
                    output Q2 = CELL.OUT_STARTUP_Q2;
                    output Q3 = CELL.OUT_STARTUP_Q3;
                }
                bel READCLK {
                    input I = CELL.IMUX_READCLK_I;
                }
            } else if tile_class CNR_NW {
                bel MISC_NW;
                bel BSCAN {
                    input TDO1 = CELL.IMUX_BSCAN_TDO1;
                    input TDO2 = CELL.IMUX_BSCAN_TDO2;
                    output DRCK = CELL.OUT_IO_SN_I2[1];
                    output IDLE = CELL.OUT_IO_WE_I2[1];
                    output SEL1 = CELL.OUT_IO_WE_I1[1];
                    output SEL2 = CELL.OUT_IO_SN_I1[1];
                }
            } else if tile_class CNR_NE {
                bel MISC_NE;
                bel UPDATE {
                    output O = CELL.OUT_UPDATE_O;
                }
                bel OSC {
                    output F8M = CELL.OUT_IO_WE_I1[1];
                    output OUT0 = CELL.OUT_IO_WE_I2[1];
                    output OUT1 = CELL.OUT_OSC_MUX1;
                }
                bel TDO {
                    input O = CELL.IMUX_TDO_O;
                    input T = CELL.IMUX_TDO_T;
                }
            }
        }
    }

    tile_slot LLH {
        bel_slot LLH: routing;
        bel_slot PULLUP_TBUF_W[2]: PULLUP;
        bel_slot PULLUP_TBUF_E[2]: PULLUP;
        bel_slot TBUF_SPLITTER[2]: TBUF_SPLITTER;
        bel_slot PULLUP_DEC_W[4]: PULLUP;
        bel_slot PULLUP_DEC_E[4]: PULLUP;

        tile_class LLH_CLB, LLH_CLB_S, LLH_IO_S, LLH_IO_N {
            cell W, E;
            if variant [xc4000, xc4000a, xc4000h, xc4000e, spartanxl] {
                if tile_class LLH_CLB {
                    bitrect LLH: LLH;
                    bitrect LLH_S: LLH;
                }
                if tile_class LLH_CLB_S {
                    bitrect LLH: LLH;
                    bitrect LLH_S: LLH_S;
                    bitrect MAIN_SW: MAIN_S;
                }
                if tile_class LLH_IO_S {
                    bitrect LLH: LLH_S;
                    bitrect MAIN_W: MAIN_S;
                }
                if tile_class LLH_IO_N {
                    bitrect LLH: LLH_N;
                    bitrect LLH_S: LLH;
                    bitrect MAIN_W: MAIN_N;
                }
            }

            switchbox LLH;
            if variant spartanxl {
                if tile_class [LLH_CLB, LLH_CLB_S] {
                    for i in 0..2 {
                        bel TBUF_SPLITTER[i] {
                            bidir W = W.LONG_H[i + 2];
                            bidir E = E.LONG_H[i + 2];
                        }
                    }
                }
            }
        }

        tile_class LLHC_CLB, LLHC_CLB_S, LLHC_IO_S, LLHC_IO_N {
            cell W, E;
            if variant [xc4000ex, xc4000xla, xc4000xv] {
                if tile_class LLHC_CLB {
                    bitrect LLH: LLHC;
                    bitrect LLH_S: LLHC;
                }
                if tile_class LLHC_CLB_S {
                    bitrect LLH: LLHC;
                    bitrect LLH_S: LLHC_S;
                    bitrect MAIN_SW: MAIN_S;
                }
                if tile_class LLHC_IO_S {
                    bitrect LLH: LLHC_S;
                    bitrect MAIN_W: MAIN_S;
                }
                if tile_class LLHC_IO_N {
                    bitrect LLH: LLHC_N;
                    bitrect LLH_S: LLHC;
                    bitrect MAIN_W: MAIN_N;
                }
            }

            switchbox LLH;
            if tile_class [LLHC_CLB, LLHC_CLB_S] {
                for i in 0..2 {
                    bel TBUF_SPLITTER[i] {
                        bidir W = W.LONG_H[i + 2];
                        bidir E = E.LONG_H[i + 2];
                    }
                    bel PULLUP_TBUF_W[i] {
                        bidir O = W.LONG_H[i + 2];
                    }
                    bel PULLUP_TBUF_E[i] {
                        bidir O = E.LONG_H[i + 2];
                    }
                }
            } else {
                for i in 0..4 {
                    bel PULLUP_DEC_W[i] {
                        bidir O = W.DEC_H[i];
                    }
                    bel PULLUP_DEC_E[i] {
                        bidir O = E.DEC_H[i];
                    }
                }
            }
        }

        tile_class LLHQ_CLB, LLHQ_CLB_S, LLHQ_CLB_N, LLHQ_IO_S, LLHQ_IO_N {
            cell W, E;
            if variant [xc4000ex, xc4000xla, xc4000xv] {
                if tile_class [LLHQ_CLB, LLHQ_CLB_N] {
                    bitrect LLH: LLHQ;
                    bitrect LLH_S: LLHQ;
                }
                if tile_class LLHQ_CLB_S {
                    bitrect LLH: LLHQ;
                    bitrect LLH_S: LLHQ_S;
                    bitrect MAIN_SW: MAIN_S;
                }
                if tile_class LLHQ_IO_S {
                    bitrect LLH: LLHQ_S;
                    bitrect MAIN_W: MAIN_S;
                }
                if tile_class LLHQ_IO_N {
                    bitrect LLH: LLHQ_N;
                    bitrect LLH_S: LLHQ;
                    bitrect MAIN_W: MAIN_N;
                }
            }

            switchbox LLH;
            if tile_class [LLHQ_CLB, LLHQ_CLB_S, LLHQ_CLB_N] {
                for i in 0..2 {
                    bel PULLUP_TBUF_W[i] {
                        bidir O = W.LONG_H[i + 2];
                    }
                    bel PULLUP_TBUF_E[i] {
                        bidir O = E.LONG_H[i + 2];
                    }
                }
            }
        }
    }

    tile_slot LLV {
        bel_slot LLV: routing;
        bel_slot BUFF: BUFF;
        bel_slot PULLUP_DEC_S[4]: PULLUP;
        bel_slot PULLUP_DEC_N[4]: PULLUP;
        bel_slot MISC_E: MISC_E;

        tile_class LLV_CLB, LLV_IO_W, LLV_IO_E {
            cell S, N;
            if tile_class LLV_CLB {
                bitrect LLV: LLV;
            }
            if tile_class LLV_IO_W {
                bitrect LLV: LLV_W;
                bitrect LLV_E: LLV;
            }
            if tile_class LLV_IO_E {
                bitrect LLV: LLV_E;
            }

            switchbox LLV;
            if tile_class LLV_IO_E {
                bel MISC_E;
            }
        }

        tile_class LLVC_CLB, LLVC_IO_W, LLVC_IO_E {
            cell S, N;
            if tile_class LLVC_CLB {
                bitrect LLV: LLV;
            }
            if tile_class LLVC_IO_W {
                bitrect LLV: LLV_W;
                bitrect LLV_E: LLV;
            }
            if tile_class LLVC_IO_E {
                bitrect LLV: LLV_E;
            }

            switchbox LLV;
            if tile_class [LLVC_IO_W, LLVC_IO_E] {
                for i in 0..4 {
                    bel PULLUP_DEC_S[i] {
                        bidir O = S.DEC_V[i];
                    }
                    bel PULLUP_DEC_N[i] {
                        bidir O = N.DEC_V[i];
                    }
                }
            }
            if tile_class LLVC_IO_E {
                bel MISC_E;
            }
        }

        tile_class LLVQ_CLB, LLVQ_IO_SW, LLVQ_IO_NW, LLVQ_IO_SE, LLVQ_IO_NE {
            cell S, N;
            if tile_class LLVQ_CLB {
                bitrect LLV: LLV;
            }
            if tile_class [LLVQ_IO_SW, LLVQ_IO_NW] {
                bitrect LLV: LLV_W;
                bitrect LLV_E: LLV;
            }
            if tile_class [LLVQ_IO_SE, LLVQ_IO_NE] {
                bitrect LLV: LLV_E;
            }

            switchbox LLV;

            if tile_class [LLVQ_IO_SW, LLVQ_IO_NW, LLVQ_IO_SE, LLVQ_IO_NE] {
                bel BUFF {
                    output O = N.OUT_BUFF;
                }
            }
        }
    }

    tile_slot CLKQ {
        bel_slot CLKQC: routing;
        bel_slot CLKQ: routing;

        tile_class CLKQC {
            cell CELL;
            switchbox CLKQC;
        }

        tile_class CLKQ {
            cell W, E;
            switchbox CLKQ;
        }
    }


    connector_slot W {
        opposite E;

        connector_class PASS_W, PASS_CLB_W_W {
            pass SINGLE_H_E = SINGLE_H;
            pass DOUBLE_H1 = DOUBLE_H0;
            pass DOUBLE_H2 = DOUBLE_H1;
            pass DOUBLE_IO_S1 = DOUBLE_IO_S0;
            pass DOUBLE_IO_S2 = DOUBLE_IO_S1;
            pass QUAD_H1 = QUAD_H0;
            pass QUAD_H2 = QUAD_H1;
            pass QUAD_H3 = QUAD_H2;
            pass QUAD_H4 = QUAD_H3;
            for i in 0..8 {
                pass OCTAL_H[i + 1] = OCTAL_H[i];
            }
            for i in 0..8 {
                pass OCTAL_IO_S[i + 1] = OCTAL_IO_S[i];
            }
            if connector_class PASS_CLB_W_W {
                pass OUT_CLB_Y_E = OUT_IO_WE_I2[1];
                pass OUT_CLB_YQ_E = OUT_IO_WE_I2[0];
            } else if variant [xc4000, xc4000h, xc4000a, xc4000e] {
                pass OUT_CLB_Y_E = OUT_CLB_Y;
                pass OUT_CLB_YQ_E = OUT_CLB_YQ;
            } else {
                pass OUT_CLB_Y_E = OUT_CLB_Y_H;
                pass OUT_CLB_YQ_E = OUT_CLB_YQ_H;
            }
            pass OUT_IO_SN_I1_E1 = OUT_IO_SN_I1[1];
            pass OUT_IO_SN_I2_E1 = OUT_IO_SN_I2[1];
            pass OUT_IO_CLKIN_E = OUT_IO_CLKIN;
            pass OUT_COUT_E = OUT_COUT;
        }

        connector_class CNR_SW {
            reflect DOUBLE_IO_S1 = DOUBLE_IO_W1;
            for i in 0..8 {
                reflect OCTAL_IO_S[i] = OCTAL_IO_W[i + 1];
            }
        }
    }

    connector_slot E {
        opposite W;

        connector_class PASS_E {
            pass DOUBLE_IO_N1 = DOUBLE_IO_N0;
            pass DOUBLE_IO_N2 = DOUBLE_IO_N1;
            for i in 0..8 {
                pass OCTAL_IO_N[i + 1] = OCTAL_IO_N[i];
            }
            pass IMUX_CLB_F3_W = IMUX_CLB_F3;
            pass IMUX_CLB_G3_W = IMUX_CLB_G3;
            pass IMUX_CLB_C3_W = IMUX_CLB_C3;
            pass OUT_IO_CLKIN_W = OUT_IO_CLKIN;
        }

        connector_class CNR_NE {
            reflect DOUBLE_IO_N1 = DOUBLE_IO_E1;
            for i in 0..8 {
                reflect OCTAL_IO_N[i] = OCTAL_IO_E[i + 1];
            }
        }
    }

    connector_slot S {
        opposite N;

        connector_class PASS_S {
            pass DOUBLE_IO_E1 = DOUBLE_IO_E0;
            pass DOUBLE_IO_E2 = DOUBLE_IO_E1;
            for i in 0..8 {
                pass OCTAL_IO_E[i + 1] = OCTAL_IO_E[i];
            }
            pass IMUX_CLB_F2_N = IMUX_CLB_F2;
            pass IMUX_CLB_G2_N = IMUX_CLB_G2;
            pass IMUX_CLB_C2_N = IMUX_CLB_C2;
            pass OUT_IO_CLKIN_N = OUT_IO_CLKIN;
        }

        connector_class CNR_SE {
            for i in 0..8 {
                reflect OCTAL_IO_E[i] = OCTAL_IO_S[i + 1];
            }
        }
    }

    connector_slot N {
        opposite S;

        connector_class PASS_N, PASS_CLB_N_N {
            pass SINGLE_V_S = SINGLE_V;
            pass DOUBLE_V1 = DOUBLE_V0;
            pass DOUBLE_V2 = DOUBLE_V1;
            pass DOUBLE_IO_W1 = DOUBLE_IO_W0;
            pass DOUBLE_IO_W2 = DOUBLE_IO_W1;
            pass QUAD_V1 = QUAD_V0;
            pass QUAD_V2 = QUAD_V1;
            pass QUAD_V3 = QUAD_V2;
            pass QUAD_V4 = QUAD_V3;
            for i in 0..8 {
                pass OCTAL_V[i + 1] = OCTAL_V[i];
            }
            for i in 0..8 {
                pass OCTAL_IO_W[i + 1] = OCTAL_IO_W[i];
            }
            if connector_class PASS_CLB_N_N {
                pass OUT_CLB_X_S = OUT_IO_SN_I2[0];
                pass OUT_CLB_XQ_S = OUT_IO_SN_I2[1];
            } else if variant [xc4000, xc4000h, xc4000a, xc4000e] {
                pass OUT_CLB_X_S = OUT_CLB_X;
                pass OUT_CLB_XQ_S = OUT_CLB_XQ;
            } else {
                pass OUT_CLB_X_S = OUT_CLB_X_V;
                pass OUT_CLB_XQ_S = OUT_CLB_XQ_V;
            }
            pass OUT_IO_WE_I1_S1 = OUT_IO_WE_I1[1];
            pass OUT_IO_WE_I2_S1 = OUT_IO_WE_I2[1];
            pass OUT_IO_CLKIN_S = OUT_IO_CLKIN;
        }

        connector_class CNR_NW {
            reflect DOUBLE_IO_W0 = DOUBLE_IO_N1;
            reflect DOUBLE_IO_W1 = DOUBLE_IO_N2;
            for i in 0..8 {
                reflect OCTAL_IO_W[i] = OCTAL_IO_N[i + 1];
            }
        }
    }

}
