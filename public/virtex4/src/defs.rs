use prjcombine_tablegen::target_defs;

target_defs! {
    variant virtex4;
    variant virtex5;
    variant virtex6;
    variant virtex7;

    enum SLICE_V4_CYINIT { BX, CIN }
    enum SLICE_V4_CY0F { CONST_0, CONST_1, BX, F3, F2, PROD }
    enum SLICE_V4_CY0G { CONST_0, CONST_1, BY, G3, G2, PROD }
    enum SLICE_V4_DIF_MUX { ALT, BX }
    enum SLICE_V4_DIG_MUX { ALT, BY }
    enum SLICE_V4_DXMUX { X, BX, F5, FXOR, XB }
    enum SLICE_V4_DYMUX { Y, BY, FX, GXOR, YB }
    enum SLICE_V4_FXMUX { F5, FXOR }
    enum SLICE_V4_GYMUX { FX, GXOR }
    enum SLICE_V4_XBMUX { FCY, FMC15 }
    enum SLICE_V4_YBMUX { GCY, GMC15 }
    bel_class SLICE_V4 {
        input F1, F2, F3, F4;
        input G1, G2, G3, G4;
        input BX, BY;
        input CLK, SR, CE;
        output X, Y;
        output XQ, YQ;
        output XB, YB;
        output XMUX, YMUX;

        attribute F, G: bitvec[16];

        // SLICEM only
        attribute DIF_MUX: SLICE_V4_DIF_MUX;
        attribute DIG_MUX: SLICE_V4_DIG_MUX;
        attribute F_RAM_ENABLE, G_RAM_ENABLE: bool;
        attribute F_SHIFT_ENABLE, G_SHIFT_ENABLE: bool;
        // SLICEM only
        attribute F_SLICEWE0USED, G_SLICEWE0USED: bool;
        attribute F_SLICEWE1USED, G_SLICEWE1USED: bool;

        attribute CYINIT: SLICE_V4_CYINIT;
        attribute CY0F: SLICE_V4_CY0F;
        attribute CY0G: SLICE_V4_CY0G;

        attribute FFX_INIT, FFY_INIT: bitvec[1];
        attribute FFX_SRVAL, FFY_SRVAL: bitvec[1];
        attribute FF_LATCH: bool;
        attribute FF_REV_ENABLE: bool;
        attribute FF_SR_SYNC: bool;
        // SLICEM only (effectively always enabled on SLICEL)
        attribute FF_SR_ENABLE: bool;

        attribute FXMUX: SLICE_V4_FXMUX;
        attribute GYMUX: SLICE_V4_GYMUX;
        attribute DXMUX: SLICE_V4_DXMUX;
        attribute DYMUX: SLICE_V4_DYMUX;

        // SLICEM only (effectively *CY on SLICEL)
        attribute XBMUX: SLICE_V4_XBMUX;
        attribute YBMUX: SLICE_V4_YBMUX;
    }

    // TODO: enums, bel slots

    region_slot HCLK;
    region_slot LEAF;

    if variant virtex4 {
        wire PULLUP: pullup;
        wire TIE_0: tie 0;
        wire TIE_1: tie 1;

        wire HCLK[8]: regional LEAF;
        wire RCLK[2]: regional LEAF;

        wire OMUX[16]: mux;
        wire OMUX_S0: branch N;
        wire OMUX_S0_ALT: branch N;
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

        wire LH[25] {
            0..12 => multi_branch W,
            12 => multi_root,
            13..25 => multi_branch E,
        }
        wire LV[25] {
            0..12 => multi_branch S,
            12 => multi_root,
            13..25 => multi_branch N,
        }

        wire IMUX_SR[4]: mux;
        wire IMUX_SR_OPTINV[4]: mux;
        wire IMUX_BOUNCE[4]: mux;
        wire IMUX_CLK[4]: mux;
        wire IMUX_CLK_OPTINV[4]: mux;
        wire IMUX_CE[4]: mux;
        wire IMUX_CE_OPTINV[4]: mux;
        wire IMUX_BYP[8]: mux;
        wire IMUX_BYP_BOUNCE[8]: mux;
        wire IMUX_IMUX[32]: mux;

        wire OUT_BEST[8]: bel;
        wire OUT_BEST_TMIN[8]: bel;
        wire OUT_SEC[8]: bel;
        wire OUT_SEC_TMIN[8]: bel;
        wire OUT_HALF0[8]: bel;
        wire OUT_HALF0_BEL[8]: bel;
        wire OUT_HALF0_TEST[8]: test;
        wire OUT_HALF1[8]: bel;
        wire OUT_HALF1_BEL[8]: bel;
        wire OUT_HALF1_TEST[8]: test;

        wire TEST[4]: test;
    }

    if variant virtex5 {
        wire PULLUP: pullup;
        wire TIE_0: tie 0;
        wire TIE_1: tie 1;

        wire HCLK[10]: regional LEAF;
        wire RCLK[4]: regional LEAF;

        wire DBL_WW0_S0: mux;
        wire DBL_WW0_N5: mux;
        wire DBL_WW0[6] {
            0 => branch S,
            1..5 => mux,
            5 => branch N,
        }
        wire DBL_WW1[6]: branch E;
        wire DBL_WW2[6]: branch E;
        wire DBL_WS0[3]: mux;
        wire DBL_WS1[3]: branch E;
        wire DBL_WS2[3]: branch N;
        wire DBL_WS1_BUF0: mux;
        wire DBL_WS1_S0: branch N;
        wire DBL_WN0[3]: mux;
        wire DBL_WN1[3]: branch E;
        wire DBL_WN2[3]: branch S;
        wire DBL_WN2_S0: branch N;
        wire DBL_EE0_S3: mux;
        wire DBL_EE0[6] {
            0..3 => mux,
            3 => branch S,
            4..6 => mux,
        }
        wire DBL_EE1[6]: branch W;
        wire DBL_EE2[6]: branch W;
        wire DBL_ES0[3]: mux;
        wire DBL_ES1[3]: branch W;
        wire DBL_ES2[3]: branch N;
        wire DBL_EN0[3]: mux;
        wire DBL_EN1[3]: branch W;
        wire DBL_EN2[3]: branch S;
        wire DBL_NN0_S0: mux;
        wire DBL_NN0_N5: mux;
        wire DBL_NN0[6] {
            0 => branch S,
            1..5 => mux,
            5 => branch N,
        }
        wire DBL_NN1[6]: branch S;
        wire DBL_NN2[6]: branch S;
        wire DBL_NW0[3]: mux;
        wire DBL_NW1[3]: branch S;
        wire DBL_NW2[3]: branch E;
        wire DBL_NW2_N2: branch S;
        wire DBL_NE0[3]: mux;
        wire DBL_NE1[3]: branch S;
        wire DBL_NE2[3]: branch W;
        wire DBL_NE1_BUF2: mux;
        wire DBL_NE1_N2: branch S;
        wire DBL_SS0_N2: mux;
        wire DBL_SS0[6] {
            0..2 => mux,
            2 => branch N,
            3..6 => mux,
        }
        wire DBL_SS1[6]: branch N;
        wire DBL_SS2[6]: branch N;
        wire DBL_SW0[3]: mux;
        wire DBL_SW1[3]: branch N;
        wire DBL_SW2[3]: branch E;
        wire DBL_SE0[3]: mux;
        wire DBL_SE1[3]: branch N;
        wire DBL_SE2[3]: branch W;

        wire PENT_WW0_S0: mux;
        wire PENT_WW0[6] {
            0 => branch S,
            1..6 => mux,
        }
        wire PENT_WW1[6]: branch E;
        wire PENT_WW2[6]: branch E;
        wire PENT_WW3[6]: branch E;
        wire PENT_WW4[6]: branch E;
        wire PENT_WW5[6]: branch E;
        wire PENT_WS0[3]: mux;
        wire PENT_WS1[3]: branch E;
        wire PENT_WS2[3]: branch E;
        wire PENT_WS3[3]: branch E;
        wire PENT_WS4[3]: branch N;
        wire PENT_WS5[3]: branch N;
        wire PENT_WS3_BUF0: mux;
        wire PENT_WS3_S0: branch N;
        wire PENT_WN0[3]: mux;
        wire PENT_WN1[3]: branch E;
        wire PENT_WN2[3]: branch E;
        wire PENT_WN3[3]: branch E;
        wire PENT_WN4[3]: branch S;
        wire PENT_WN5[3]: branch S;
        wire PENT_WN5_S0: branch N;
        wire PENT_EE0[6]: mux;
        wire PENT_EE1[6]: branch W;
        wire PENT_EE2[6]: branch W;
        wire PENT_EE3[6]: branch W;
        wire PENT_EE4[6]: branch W;
        wire PENT_EE5[6]: branch W;
        wire PENT_ES0[3]: mux;
        wire PENT_ES1[3]: branch W;
        wire PENT_ES2[3]: branch W;
        wire PENT_ES3[3]: branch W;
        wire PENT_ES4[3]: branch N;
        wire PENT_ES5[3]: branch N;
        wire PENT_EN0[3]: mux;
        wire PENT_EN1[3]: branch W;
        wire PENT_EN2[3]: branch W;
        wire PENT_EN3[3]: branch W;
        wire PENT_EN4[3]: branch S;
        wire PENT_EN5[3]: branch S;
        wire PENT_SS0[6]: mux;
        wire PENT_SS1[6]: branch N;
        wire PENT_SS2[6]: branch N;
        wire PENT_SS3[6]: branch N;
        wire PENT_SS4[6]: branch N;
        wire PENT_SS5[6]: branch N;
        wire PENT_SW0[3]: mux;
        wire PENT_SW1[3]: branch N;
        wire PENT_SW2[3]: branch N;
        wire PENT_SW3[3]: branch N;
        wire PENT_SW4[3]: branch E;
        wire PENT_SW5[3]: branch E;
        wire PENT_SE0[3]: mux;
        wire PENT_SE1[3]: branch N;
        wire PENT_SE2[3]: branch N;
        wire PENT_SE3[3]: branch N;
        wire PENT_SE4[3]: branch W;
        wire PENT_SE5[3]: branch W;
        wire PENT_NN0_N5: mux;
        wire PENT_NN0[6] {
            0..5 => mux,
            5 => branch N,
        }
        wire PENT_NN1[6]: branch S;
        wire PENT_NN2[6]: branch S;
        wire PENT_NN3[6]: branch S;
        wire PENT_NN4[6]: branch S;
        wire PENT_NN5[6]: branch S;
        wire PENT_NW0[3]: mux;
        wire PENT_NW1[3]: branch S;
        wire PENT_NW2[3]: branch S;
        wire PENT_NW3[3]: branch S;
        wire PENT_NW4[3]: branch E;
        wire PENT_NW5[3]: branch E;
        wire PENT_NW5_N2: branch S;
        wire PENT_NE0[3]: mux;
        wire PENT_NE1[3]: branch S;
        wire PENT_NE2[3]: branch S;
        wire PENT_NE3[3]: branch S;
        wire PENT_NE4[3]: branch W;
        wire PENT_NE5[3]: branch W;
        wire PENT_NE3_BUF2: mux;
        wire PENT_NE3_N2: branch S;

        wire LH[19] {
            0..9 => multi_branch W,
            9 => multi_root,
            10..19 => multi_branch E,
        }
        wire LV[19] {
            0..9 => multi_branch N,
            9 => multi_root,
            10..19 => multi_branch S,
        }

        wire IMUX_GFAN[2]: mux;
        wire IMUX_CLK[2]: mux;

        wire IMUX_CTRL[4]: mux;
        wire IMUX_CTRL_SITE[4]: mux;
        wire IMUX_CTRL_BOUNCE[4]: mux;
        wire IMUX_CTRL_BOUNCE_S0: branch N;
        wire IMUX_CTRL_BOUNCE_N3: branch S;

        wire IMUX_BYP[8]: mux;
        wire IMUX_BYP_SITE[8]: mux;
        wire IMUX_BYP_BOUNCE[8]: mux;
        wire IMUX_BYP_BOUNCE_S0: branch N;
        wire IMUX_BYP_BOUNCE_N3: branch S;
        wire IMUX_BYP_BOUNCE_S4: branch N;
        wire IMUX_BYP_BOUNCE_N7: branch S;

        wire IMUX_FAN[8]: mux;
        wire IMUX_FAN_SITE[8]: mux;
        wire IMUX_FAN_BOUNCE[8]: mux;
        wire IMUX_FAN_BOUNCE_S0: branch N;
        wire IMUX_FAN_BOUNCE_N7: branch S;

        wire IMUX_IMUX[48]: mux;
        wire IMUX_IMUX_DELAY[48]: mux;

        wire OUT[24]: bel;
        wire OUT_BEL[24]: bel;
        wire OUT_TEST[24]: test;
        wire OUT_S12_DBL: branch N;
        wire OUT_N15_DBL: branch S;
        wire OUT_N17_DBL: branch S;
        wire OUT_S18_DBL: branch N;
        wire OUT_S12_PENT: branch N;
        wire OUT_N15_PENT: branch S;
        wire OUT_N17_PENT: branch S;
        wire OUT_S18_PENT: branch N;

        wire TEST[4]: test;
    }

    if variant virtex6 {
        wire TIE_0: tie 0;
        wire TIE_1: tie 1;

        wire LCLK[8]: regional LEAF;

        wire SNG_W0_N3: mux;
        wire SNG_W0_S4: mux;
        wire SNG_W0[8] {
            0..3 => mux,
            3 => branch N,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_W1[8]: branch E;
        wire SNG_W1_S4: branch N;
        wire SNG_W1_N3: branch S;
        wire SNG_E0_N3: mux;
        wire SNG_E0_S4: mux;
        wire SNG_E0[8] {
            0..3 => mux,
            3 => branch N,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_E1[8]: branch W;
        wire SNG_E1_S0: branch N;
        wire SNG_E1_N7: branch S;
        wire SNG_S0_S4: mux;
        wire SNG_S0[8] {
            0..4 => mux,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_S1[8]: branch N;
        wire SNG_S1_N7: branch S;
        wire SNG_N0_N3: mux;
        wire SNG_N0[8] {
            0..3 => mux,
            3 => branch N,
            4..8 => mux,
        }
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
        wire QUAD_SW1[4]: branch E;
        wire QUAD_SW2[4]: branch N;
        wire QUAD_SW3[4]: branch N;
        wire QUAD_SW4[4]: branch E;
        wire QUAD_SW4_N3: branch S;
        wire QUAD_SE0[4]: mux;
        wire QUAD_SE1[4]: branch W;
        wire QUAD_SE2[4]: branch N;
        wire QUAD_SE3[4]: branch N;
        wire QUAD_SE4[4]: branch W;
        wire QUAD_NN0[4]: mux;
        wire QUAD_NN1[4]: branch S;
        wire QUAD_NN2[4]: branch S;
        wire QUAD_NN3[4]: branch S;
        wire QUAD_NN4[4]: branch S;
        wire QUAD_NN4_S0: branch N;
        wire QUAD_NW0[4]: mux;
        wire QUAD_NW1[4]: branch E;
        wire QUAD_NW2[4]: branch S;
        wire QUAD_NW3[4]: branch S;
        wire QUAD_NW4[4]: branch E;
        wire QUAD_NW4_S0: branch N;
        wire QUAD_NE0[4]: mux;
        wire QUAD_NE1[4]: branch W;
        wire QUAD_NE2[4]: branch S;
        wire QUAD_NE3[4]: branch S;
        wire QUAD_NE4[4]: branch W;

        wire LH[17] {
            0..8 => multi_branch W,
            8 => multi_root,
            9..17 => multi_branch E,
        }
        wire LV[17] {
            0..8 => multi_branch N,
            8 => multi_root,
            9..17 => multi_branch S,
        }

        wire IMUX_GFAN[2]: mux;
        wire IMUX_CLK[2]: mux;
        wire IMUX_CTRL[2]: mux;

        wire IMUX_BYP[8]: mux;
        wire IMUX_BYP_SITE[8]: mux;
        wire IMUX_BYP_BOUNCE[8]: mux;
        wire IMUX_BYP_BOUNCE_N[8]: branch S;

        wire IMUX_FAN[8]: mux;
        wire IMUX_FAN_SITE[8]: mux;
        wire IMUX_FAN_BOUNCE[8]: mux;
        wire IMUX_FAN_BOUNCE_S[8]: branch N;

        wire IMUX_IMUX[48]: mux;
        wire IMUX_IMUX_DELAY[48]: mux;

        wire OUT[24]: bel;
        wire OUT_BEL[24]: bel;
        wire OUT_TEST[24]: test;

        wire TEST[4]: test;
    }

    if variant virtex7 {
        wire TIE_0: tie 0;
        wire TIE_1: tie 1;

        wire LCLK[12]: bel;

        wire SNG_W0_N3: mux;
        wire SNG_W0_S4: mux;
        wire SNG_W0[8] {
            0..3 => mux,
            3 => branch N,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_W1[8]: branch E;
        wire SNG_W1_S4: branch N;
        wire SNG_W1_N3: branch S;
        wire SNG_E0_N3: mux;
        wire SNG_E0_S4: mux;
        wire SNG_E0[8] {
            0..3 => mux,
            3 => branch N,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_E1[8]: branch W;
        wire SNG_E1_S0: branch N;
        wire SNG_E1_N7: branch S;
        wire SNG_S0_S4: mux;
        wire SNG_S0[8] {
            0..4 => mux,
            4 => branch S,
            5..8 => mux,
        }
        wire SNG_S1[8]: branch N;
        wire SNG_S1_N7: branch S;
        wire SNG_N0_N3: mux;
        wire SNG_N0[8] {
            0..3 => mux,
            3 => branch N,
            4..8 => mux,
        }
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

        wire HEX_SS0[4]: mux;
        wire HEX_SS1[4]: branch N;
        wire HEX_SS2[4]: branch N;
        wire HEX_SS3[4]: branch N;
        wire HEX_SS4[4]: branch N;
        wire HEX_SS5[4]: branch N;
        wire HEX_SS6[4]: branch N;
        wire HEX_SS6_N3: branch S;
        wire HEX_SW0[4]: mux;
        wire HEX_SW1[4]: branch E;
        wire HEX_SW2[4]: branch N;
        wire HEX_SW3[4]: branch N;
        wire HEX_SW4[4]: branch N;
        wire HEX_SW5[4]: branch N;
        wire HEX_SW6[4]: branch E;
        wire HEX_SW6_N3: branch S;
        wire HEX_SE0[4]: mux;
        wire HEX_SE1[4]: branch W;
        wire HEX_SE2[4]: branch N;
        wire HEX_SE3[4]: branch N;
        wire HEX_SE4[4]: branch N;
        wire HEX_SE5[4]: branch N;
        wire HEX_SE6[4]: branch W;
        wire HEX_NN0[4]: mux;
        wire HEX_NN1[4]: branch S;
        wire HEX_NN2[4]: branch S;
        wire HEX_NN3[4]: branch S;
        wire HEX_NN4[4]: branch S;
        wire HEX_NN5[4]: branch S;
        wire HEX_NN6[4]: branch S;
        wire HEX_NN6_S0: branch N;
        wire HEX_NW0[4]: mux;
        wire HEX_NW1[4]: branch E;
        wire HEX_NW2[4]: branch S;
        wire HEX_NW3[4]: branch S;
        wire HEX_NW4[4]: branch S;
        wire HEX_NW5[4]: branch S;
        wire HEX_NW6[4]: branch E;
        wire HEX_NW6_S0: branch N;
        wire HEX_NE0[4]: mux;
        wire HEX_NE1[4]: branch W;
        wire HEX_NE2[4]: branch S;
        wire HEX_NE3[4]: branch S;
        wire HEX_NE4[4]: branch S;
        wire HEX_NE5[4]: branch S;
        wire HEX_NE6[4]: branch W;

        wire LH[13] {
            0..6 => multi_branch W,
            6 => multi_root,
            7..13 => multi_branch E,
        }
        wire LV[19] {
            0..9 => multi_branch N,
            9 => multi_root,
            10..19 => multi_branch S,
        }
        wire LVB[13] {
            0..6 => multi_branch N,
            6 => multi_root,
            7..13 => multi_branch S,
        }

        wire IMUX_GFAN[2]: mux;
        wire IMUX_CLK[2]: mux;
        wire IMUX_CTRL[2]: mux;

        wire IMUX_BYP[8]: mux;
        wire IMUX_BYP_SITE[8]: mux;
        wire IMUX_BYP_BOUNCE[8]: mux;
        wire IMUX_BYP_BOUNCE_N[8]: branch S;

        wire IMUX_FAN[8]: mux;
        wire IMUX_FAN_SITE[8]: mux;
        wire IMUX_FAN_BOUNCE[8]: mux;
        wire IMUX_FAN_BOUNCE_S[8]: branch N;

        wire IMUX_IMUX[48]: mux;
        wire IMUX_IMUX_DELAY[48]: mux;
        wire IMUX_BRAM[48]: test;

        wire OUT[24]: bel;
        wire OUT_BEL[24]: bel;
        wire OUT_TEST[24]: test;

        wire TEST[4]: test;
    }

    if variant virtex4 {
        bitrect INT = vertical (19, rev 80);
        bitrect CLB = vertical (22, rev 80);
        bitrect BRAM = vertical (20, rev 80);
        bitrect DSP = vertical (21, rev 80);
        bitrect IO = vertical (30, rev 80);
        bitrect GT = vertical (20, rev 80);
        bitrect CLK = vertical (3, rev 80);
        bitrect HCLK = vertical (20, rev 32);
        bitrect HCLK_IO = vertical (30, rev 32);
        bitrect HCLK_CLK = vertical (3, rev 32);
        bitrect BRAM_DATA = vertical (64, rev 320);
    }
    if variant virtex5 {
        bitrect INT = vertical (28, rev 64);
        bitrect CLB = vertical (36, rev 64);
        bitrect BRAM = vertical (30, rev 64);
        bitrect DSP = vertical (28, rev 64);
        bitrect IO = vertical (54, rev 64);
        bitrect GT = vertical (32, rev 64);
        bitrect CLK = vertical (4, rev 64);
        bitrect HCLK = vertical (28, rev 32);
        bitrect HCLK_GT = vertical (32, rev 32);
        bitrect HCLK_IO = vertical (54, rev 32);
        bitrect HCLK_CLK = vertical (4, rev 32);
        bitrect BRAM_DATA = vertical (128, rev 320);
    }
    if variant virtex6 {
        bitrect INT = vertical (28, rev 64);
        bitrect CLB = vertical (36, rev 64);
        bitrect BRAM = vertical (28, rev 64);
        bitrect DSP = vertical (28, rev 64);
        bitrect IO = vertical (44, rev 64);
        bitrect CMT = vertical (38, rev 64);
        bitrect GT = vertical (30, rev 64);
        bitrect HCLK = vertical (28, rev 32);
        bitrect HCLK_CMT = vertical (38, rev 32);
        bitrect HCLK_IO = vertical (44, rev 32);
        bitrect BRAM_DATA = vertical (128, rev 320);
    }
    if variant virtex7 {
        bitrect INT = vertical (28, rev 64);
        bitrect CLB = vertical (36, rev 64);
        bitrect BRAM = vertical (28, rev 64);
        bitrect DSP = vertical (28, rev 64);
        bitrect IO = vertical (42, rev 64);
        bitrect CMT = vertical (30, rev 64);
        bitrect CFG = vertical (30, rev 64);
        bitrect CLK = vertical (30, rev 64);
        bitrect GT = vertical (32, rev 64);
        bitrect HCLK = vertical (28, rev 32);
        bitrect HCLK_CMT = vertical (30, rev 32);
        bitrect HCLK_CLK = vertical (30, rev 32);
        bitrect HCLK_IO = vertical (42, rev 32);
        bitrect BRAM_DATA = vertical (128, rev 320);
    }

    tile_slot INT {
        bel_slot INT: routing;
        tile_class INT {
            cell CELL;
            bitrect MAIN: INT;
        }
    }

    tile_slot INTF {
        bel_slot INTF_INT: routing;
        bel_slot INTF_TESTMUX: routing;
        tile_class INTF {
            cell CELL;
            bitrect MAIN: INT;
        }
        if variant [virtex5, virtex6, virtex7] {
            tile_class INTF_DELAY {
                cell CELL;
                bitrect MAIN: INT;
            }
        }
        if variant virtex7 {
            tile_class INTF_BRAM {
                cell CELL;
                bitrect MAIN: INT;
            }
        }
    }

    tile_slot BEL {
        if variant virtex4 {
            bel_slot SLICE[4]: SLICE_V4;
        } else {
            bel_slot SLICE[4]: legacy;
        }
        if variant virtex4 {
            tile_class CLB {
                cell CELL;
                bitrect MAIN: CLB;
            }
        } else {
            tile_class CLBLL, CLBLM {
                cell CELL;
                bitrect MAIN: CLB;
            }
        }

        bel_slot BRAM: legacy;
        bel_slot FIFO: legacy;

        bel_slot BRAM_F: legacy;
        bel_slot BRAM_H[2]: legacy;
        bel_slot BRAM_ADDR: legacy;

        if variant virtex4 {
            tile_class BRAM {
                cell CELL[4];
                bitrect MAIN[4]: BRAM;
                bitrect DATA: BRAM_DATA;
            }
        } else {
            tile_class BRAM {
                cell CELL[5];
                bitrect MAIN[5]: BRAM;
                bitrect DATA: BRAM_DATA;
            }
        }

        bel_slot DSP[2]: legacy;
        bel_slot TIEOFF_DSP: legacy;
        if variant virtex4 {
            tile_class DSP {
                cell CELL[4];
                bitrect MAIN[4]: DSP;
            }
        } else {
            tile_class DSP {
                cell CELL[5];
                bitrect MAIN[5]: DSP;
            }
        }

        bel_slot ILOGIC[2]: legacy;
        bel_slot OLOGIC[2]: legacy;
        bel_slot IODELAY[2]: legacy;
        bel_slot IDELAY[2]: legacy;
        bel_slot ODELAY[2]: legacy;
        bel_slot IOB[2]: legacy;
        bel_slot IOI: legacy;
        if variant virtex4 {
            tile_class IO {
                cell CELL;
                bitrect MAIN: IO;
            }
        }
        if variant virtex5 {
            tile_class IO {
                cell CELL;
                bitrect MAIN: IO;
            }
        }
        if variant virtex6 {
            tile_class IO {
                cell CELL[2];
                bitrect MAIN[2]: IO;
            }
        }
        if variant virtex7 {
            tile_class IO_HP_S, IO_HP_N, IO_HR_S, IO_HR_N {
                cell CELL;
                bitrect MAIN: IO;
            }
            tile_class IO_HP_PAIR, IO_HR_PAIR {
                cell CELL[2];
                bitrect MAIN[2]: IO;
            }
        }

        bel_slot DCM[2]: legacy;
        bel_slot PLL: legacy;
        bel_slot MMCM[2]: legacy;
        bel_slot CMT: legacy;
        bel_slot CMT_A: legacy;
        bel_slot CMT_B: legacy;
        bel_slot CMT_C: legacy;
        bel_slot CMT_D: legacy;
        bel_slot HCLK_CMT: legacy;
        bel_slot PPR_FRAME: legacy;
        bel_slot PHASER_IN[4]: legacy;
        bel_slot PHASER_OUT[4]: legacy;
        bel_slot PHASER_REF: legacy;
        bel_slot PHY_CONTROL: legacy;
        bel_slot BUFMRCE[2]: legacy;
        if variant virtex4 {
            tile_class DCM {
                cell CELL[4];
                bitrect MAIN[4]: IO;
            }
        }
        if variant virtex5 {
            tile_class CMT {
                cell CELL[10];
                bitrect MAIN[10]: IO;
            }
        }
        if variant virtex6 {
            tile_class CMT {
                cell CELL[40];
                bitrect MAIN[40]: CMT;
                bitrect HCLK: HCLK_CMT;
            }
        }
        if variant virtex7 {
            tile_class CMT {
                cell CELL[50];
                bitrect MAIN[50]: CMT;
                bitrect HCLK: HCLK_CMT;
            }
        }

        bel_slot CCM: legacy;
        bel_slot PMCD[2]: legacy;
        bel_slot DPM: legacy;
        if variant virtex4 {
            tile_class CCM {
                cell CELL[4];
                bitrect MAIN[4]: IO;
            }
        }

        bel_slot BUFHCE_W[12]: legacy;
        bel_slot BUFHCE_E[12]: legacy;
        bel_slot CLK_HROW_V7: legacy;
        bel_slot GCLK_TEST_BUF_HROW_GCLK[32]: legacy;
        bel_slot GCLK_TEST_BUF_HROW_BUFH_W: legacy;
        bel_slot GCLK_TEST_BUF_HROW_BUFH_E: legacy;
        if variant virtex7 {
            tile_class CLK_HROW {
                cell CELL[2];
                bitrect MAIN[8]: CLK;
                bitrect HCLK: HCLK_CLK;
            }
        }

        bel_slot PMV_CLK: legacy;
        bel_slot PMVIOB_CLK: legacy;
        bel_slot PMV2: legacy;
        bel_slot PMV2_SVT: legacy;
        bel_slot MTBF2: legacy;
        if variant virtex6 {
            tile_class PMVIOB {
                cell CELL[2];
                bitrect MAIN[2]: CMT;
            }
        }
        if variant virtex7 {
            tile_class CLK_PMV {
                cell CELL;
            }
            tile_class CLK_PMVIOB {
                cell CELL;
            }
            tile_class CLK_PMV2_SVT {
                cell CELL;
            }
            tile_class CLK_PMV2 {
                cell CELL;
            }
            tile_class CLK_MTBF2 {
                cell CELL;
            }
        }

        bel_slot PPC: legacy;
        if variant virtex4 {
            tile_class PPC {
                cell CELL_W[24];
                cell CELL_E[24];
                cell CELL_S[7];
                cell CELL_N[7];
            }
        }
        if variant virtex5 {
            tile_class PPC {
                cell CELL_W[40];
                cell CELL_E[40];
                bitrect MAIN_W[40]: CLB;
                bitrect MAIN_E[40]: CLB;
            }
        }

        bel_slot EMAC: legacy;
        if variant [virtex5, virtex6] {
            tile_class EMAC {
                cell CELL[10];
                bitrect MAIN[10]: BRAM;
            }
        }

        bel_slot PCIE: legacy;
        if variant virtex5 {
            tile_class PCIE {
                cell CELL[40];
                bitrect MAIN[40]: BRAM;
            }
        }
        if variant virtex6 {
            tile_class PCIE {
                cell CELL_W[20];
                cell CELL_E[20];
                bitrect MAIN[20]: BRAM;
            }
        }
        if variant virtex7 {
            tile_class PCIE {
                cell CELL_A[25];
                cell CELL_B[25];
                bitrect MAIN[25]: CLB;
            }
        }

        bel_slot PCIE3: legacy;
        if variant virtex7 {
            tile_class PCIE3 {
                cell CELL_W[50];
                cell CELL_E[50];
                bitrect MAIN[50]: CLB;
            }
        }

        bel_slot GT11[2]: legacy;
        bel_slot GT11CLK: legacy;
        if variant virtex4 {
            tile_class MGT {
                cell CELL[32];
                bitrect MAIN[32]: GT;
            }
        }

        bel_slot GTP_DUAL: legacy;
        if variant virtex5 {
            tile_class GTP {
                cell CELL[20];
                bitrect MAIN[20]: GT;
                bitrect HCLK: HCLK_GT;
            }
        }

        bel_slot GTX_DUAL: legacy;
        if variant virtex5 {
            tile_class GTX {
                cell CELL[20];
                bitrect MAIN[20]: GT;
                bitrect HCLK: HCLK_GT;
            }
        }

        bel_slot GTX[4]: legacy;
        if variant virtex6 {
            tile_class GTX {
                cell CELL[40];
                bitrect MAIN[40]: GT;
            }
        }

        bel_slot GTH_QUAD: legacy;
        if variant virtex6 {
            tile_class GTH {
                cell CELL[40];
                bitrect MAIN[40]: GT;
            }
        }

        bel_slot GTP_COMMON: legacy;
        bel_slot GTX_COMMON: legacy;
        bel_slot GTH_COMMON: legacy;

        if variant virtex7 {
            tile_class GTP_COMMON {
                cell CELL[6];
                bitrect MAIN[6]: GT;
            }
            tile_class GTP_COMMON_MID {
                cell CELL[6];
                bitrect MAIN[6]: INT;
                bitrect HCLK: HCLK;
            }
            tile_class GTX_COMMON {
                cell CELL[6];
                bitrect MAIN[6]: GT;
            }
            tile_class GTH_COMMON {
                cell CELL[6];
                bitrect MAIN[6]: GT;
            }
        }

        bel_slot GTP_CHANNEL: legacy;
        bel_slot GTX_CHANNEL: legacy;
        bel_slot GTH_CHANNEL: legacy;

        if variant virtex7 {
            tile_class GTP_CHANNEL {
                cell CELL[11];
                bitrect MAIN[11]: GT;
            }
            tile_class GTP_CHANNEL_MID {
                cell CELL[11];
                bitrect MAIN[11]: INT;
            }
            tile_class GTX_CHANNEL {
                cell CELL[11];
                bitrect MAIN[11]: GT;
            }
            tile_class GTH_CHANNEL {
                cell CELL[11];
                bitrect MAIN[11]: GT;
            }
        }

        bel_slot BUFDS[2]: legacy;
        bel_slot CRC32[4]: legacy;
        bel_slot CRC64[2]: legacy;

        bel_slot IPAD_CLKP[2]: legacy;
        bel_slot IPAD_CLKN[2]: legacy;
        bel_slot IPAD_RXP[4]: legacy;
        bel_slot IPAD_RXN[4]: legacy;
        bel_slot OPAD_TXP[4]: legacy;
        bel_slot OPAD_TXN[4]: legacy;

        bel_slot BUFGCTRL[32]: legacy;
        bel_slot GIO_S: legacy;
        bel_slot GIO_N: legacy;
        bel_slot BUFG_MGTCLK_S: legacy;
        bel_slot BUFG_MGTCLK_N: legacy;
        bel_slot BUFG_MGTCLK_S_HROW: legacy;
        bel_slot BUFG_MGTCLK_N_HROW: legacy;
        bel_slot BUFG_MGTCLK_S_HCLK: legacy;
        bel_slot BUFG_MGTCLK_N_HCLK: legacy;
        if variant virtex4 {
            tile_class CLK_BUFG {
                cell CELL[16];
                bitrect MAIN[16]: CLK;
            }
        }
        if variant virtex5 {
            tile_class CLK_BUFG {
                cell CELL[20];
                bitrect MAIN[20]: CLK;
            }
        }
        if variant virtex6 {
            tile_class CMT_BUFG_S, CMT_BUFG_N {
                cell CELL[3];
                bitrect MAIN[2]: CMT;
            }
        }
        if variant virtex7 {
            tile_class CLK_BUFG {
                cell CELL[4];
                bitrect MAIN[4]: CLK;
            }
        }

        bel_slot GCLK_BUF: legacy;
        if variant virtex6 {
            tile_class GCLK_BUF {
            }
        }

        bel_slot HCLK_GTX: legacy;
        bel_slot HCLK_GTH: legacy;

        bel_slot CLK_REBUF: legacy;
        bel_slot GCLK_TEST_BUF_REBUF_S[16]: legacy;
        bel_slot GCLK_TEST_BUF_REBUF_N[16]: legacy;
        if variant virtex7 {
            tile_class CLK_BUFG_REBUF {
                cell CELL[2];
                bitrect MAIN[2]: CLK;
            }
            tile_class CLK_BALI_REBUF {
                cell CELL[16];
                bitrect MAIN[16]: CLK;
            }
        }

        bel_slot PS: legacy;
        bel_slot HCLK_PS_S: legacy;
        bel_slot HCLK_PS_N: legacy;
        bel_slot IOPAD_DDRWEB: legacy;
        bel_slot IOPAD_DDRVRN: legacy;
        bel_slot IOPAD_DDRVRP: legacy;
        bel_slot IOPAD_DDRA[15]: legacy;
        bel_slot IOPAD_DDRBA[3]: legacy;
        bel_slot IOPAD_DDRCASB: legacy;
        bel_slot IOPAD_DDRCKE: legacy;
        bel_slot IOPAD_DDRCKN: legacy;
        bel_slot IOPAD_DDRCKP: legacy;
        bel_slot IOPAD_PSCLK: legacy;
        bel_slot IOPAD_DDRCSB: legacy;
        bel_slot IOPAD_DDRDM[4]: legacy;
        bel_slot IOPAD_DDRDQ[32]: legacy;
        bel_slot IOPAD_DDRDQSN[4]: legacy;
        bel_slot IOPAD_DDRDQSP[4]: legacy;
        bel_slot IOPAD_DDRDRSTB: legacy;
        bel_slot IOPAD_MIO[54]: legacy;
        bel_slot IOPAD_DDRODT: legacy;
        bel_slot IOPAD_PSPORB: legacy;
        bel_slot IOPAD_DDRRASB: legacy;
        bel_slot IOPAD_PSSRSTB: legacy;
        if variant virtex7 {
            tile_class PS {
                cell CELL[100];
            }
        }
    }

    tile_slot CMT_FIFO {
        bel_slot IN_FIFO: legacy;
        bel_slot OUT_FIFO: legacy;
        if variant virtex7 {
            tile_class CMT_FIFO {
                cell CELL[12];
                bitrect MAIN[12]: CMT;
            }
        }
    }

    tile_slot CFG {
        bel_slot BSCAN[4]: legacy;
        bel_slot ICAP[2]: legacy;
        bel_slot STARTUP: legacy;
        bel_slot CAPTURE: legacy;
        bel_slot JTAGPPC: legacy;
        bel_slot PMV_CFG[2]: legacy;
        bel_slot DCIRESET: legacy;
        bel_slot FRAME_ECC: legacy;
        bel_slot USR_ACCESS: legacy;
        bel_slot DNA_PORT: legacy;
        bel_slot KEY_CLEAR: legacy;
        bel_slot EFUSE_USR: legacy;
        bel_slot CFG_IO_ACCESS: legacy;
        bel_slot PMVIOB_CFG: legacy;
        bel_slot SYSMON: legacy;
        bel_slot IPAD_VP: legacy;
        bel_slot IPAD_VN: legacy;

        if variant virtex4 {
            tile_class CFG {
                cell CELL[16];
                bitrect MAIN[16]: IO;
            }
            tile_class SYSMON {
                cell CELL[8];
                bitrect MAIN[8]: IO;
            }
        }
        if variant virtex5 {
            tile_class CFG {
                cell CELL[20];
                bitrect MAIN[20]: IO;
            }
        }
        if variant virtex6 {
            tile_class CFG {
                cell CELL[80];
                bitrect MAIN[80]: CMT;
            }
        }
        if variant virtex7 {
            tile_class CFG {
                cell CELL[50];
                bitrect MAIN[50]: CFG;
            }
            tile_class SYSMON {
                cell CELL[25];
                bitrect MAIN[25]: CFG;
            }
        }
    }

    tile_slot CLK {
        bel_slot CLK_IOB: legacy;
        bel_slot CLK_DCM: legacy;
        bel_slot CLK_CMT: legacy;
        bel_slot CLK_MGT: legacy;
        if variant virtex4 {
            tile_class CLK_DCM_S {
                bitrect MAIN[8]: CLK;
            }
            tile_class CLK_DCM_N {
                bitrect MAIN[8]: CLK;
            }
            tile_class CLK_IOB_S {
                bitrect MAIN[16]: CLK;
            }
            tile_class CLK_IOB_N {
                bitrect MAIN[16]: CLK;
            }
        }
        if variant virtex5 {
            tile_class CLK_CMT_S {
                bitrect MAIN[10]: CLK;
            }
            tile_class CLK_CMT_N {
                bitrect MAIN[10]: CLK;
            }
            tile_class CLK_IOB_S {
                bitrect MAIN[10]: CLK;
            }
            tile_class CLK_IOB_N {
                bitrect MAIN[10]: CLK;
            }
            tile_class CLK_MGT_S {
                bitrect MAIN[10]: CLK;
            }
            tile_class CLK_MGT_N {
                bitrect MAIN[10]: CLK;
            }
        }

        bel_slot HCLK_MGT_BUF: legacy;
        if variant [virtex4, virtex5, virtex6] {
            tile_class HCLK_MGT_BUF {
                bitrect MAIN: HCLK;
            }
        }

        bel_slot INT_LCLK_W: legacy;
        bel_slot INT_LCLK_E: legacy;
        if variant virtex7 {
            tile_class INT_LCLK {
                cell W, E;
            }
        }
    }

    tile_slot HROW {
        bel_slot CLK_HROW: legacy;
        if variant virtex4 {
            tile_class CLK_HROW {
                bitrect MAIN[2]: CLK;
                bitrect HCLK: HCLK_CLK;
            }
        }
            if variant virtex5 {
            tile_class CLK_HROW {
                bitrect MAIN[2]: CLK;
                bitrect HCLK: HCLK_CLK;
            }
        }

        bel_slot HCLK_QBUF: legacy;
        if variant virtex6 {
            tile_class HCLK_QBUF {
            }
        }

        if variant virtex4 {
            tile_class CLK_TERM {
                bitrect MAIN: CLK;
            }

            tile_class HCLK_TERM {
                bitrect MAIN: HCLK;
            }
        }
    }

    tile_slot HCLK {
        bel_slot HCLK: legacy;
        bel_slot HCLK_W: legacy;
        bel_slot HCLK_E: legacy;
        bel_slot GLOBALSIG: legacy;

        if variant [virtex4, virtex5] {
            tile_class HCLK {
                cell CELL;
                bitrect MAIN: HCLK;
            }
        }
        if variant virtex6 {
            tile_class HCLK {
                cell S, N;
                bitrect MAIN: HCLK;
            }
        }
        if variant virtex7 {
            tile_class HCLK {
                bitrect MAIN[2]: HCLK;
            }
        }
    }

    tile_slot HCLK_BEL {
        bel_slot PMVBRAM: legacy;
        bel_slot PMVBRAM_NC: legacy;

        if variant [virtex5] {
            tile_class PMVBRAM {
                cell CELL[5];
            }
        }
        if variant [virtex6, virtex7] {
            tile_class PMVBRAM {
                cell CELL[15];
            }
        }
        if variant virtex7 {
            tile_class PMVBRAM_NC {
            }
        }

        bel_slot HCLK_IO: legacy;
        bel_slot IOCLK: legacy;
        bel_slot RCLK: legacy;
        bel_slot HCLK_DCM_HROW: legacy;
        bel_slot HCLK_DCM: legacy;
        bel_slot HCLK_DCM_S: legacy;
        bel_slot HCLK_DCM_N: legacy;
        bel_slot BUFR[4]: legacy;
        bel_slot BUFIO[4]: legacy;
        bel_slot BUFO[2]: legacy;
        bel_slot IDELAYCTRL: legacy;
        bel_slot DCI: legacy;

        if variant virtex4 {
            tile_class HCLK_IO_DCI, HCLK_IO_LVDS {
                cell CELL[3];
                bitrect MAIN: HCLK_IO;
            }
            tile_class HCLK_IO_CENTER, HCLK_IO_CFG_N, HCLK_IO_DCM_S, HCLK_IO_DCM_N {
                cell CELL[2];
                bitrect MAIN: HCLK_IO;
            }
            tile_class HCLK_DCM {
                bitrect MAIN: HCLK_IO;
            }
        }
        if variant virtex5 {
            tile_class HCLK_IO {
                cell CELL[4];
                bitrect MAIN: HCLK_IO;
            }
            tile_class HCLK_IO_CENTER, HCLK_IO_CFG_S, HCLK_IO_CMT_S, HCLK_IO_CFG_N, HCLK_IO_CMT_N {
                cell CELL[2];
                bitrect MAIN: HCLK_IO;
            }
        }
        if variant virtex6 {
            tile_class HCLK_IO {
                cell CELL[2];
                bitrect MAIN: HCLK_IO;
            }
        }
        if variant virtex7 {
            tile_class HCLK_IO_HP, HCLK_IO_HR {
                cell CELL[8];
                bitrect MAIN: HCLK_IO;
            }
        }

        bel_slot BRKH_GTX: legacy;

        if variant virtex7 {
            tile_class BRKH_GTX {
            }
        }

        if variant virtex4 {
            tile_class HCLK_MGT {
                bitrect MAIN: HCLK;
            }
        }
    }

    tile_slot HCLK_CMT {
        bel_slot HCLK_CMT_HCLK: legacy;
        bel_slot HCLK_CMT_GIOB: legacy;
        if variant virtex5 {
            tile_class HCLK_CMT {
                bitrect MAIN: HCLK_IO;
            }
        }
    }

    connector_slot W {
        opposite E;

        if variant virtex4 {
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
                for i in 0..12 {
                    pass LH[i] = LH[i + 1];
                }
            }
            connector_class TERM_W;
            connector_class CLB_BUFFER_W;
            connector_class PPC_W;
        }
        if variant virtex5 {
            connector_class PASS_W {
                pass DBL_EE1 = DBL_EE0;
                pass DBL_EE2 = DBL_EE1;
                pass DBL_ES1 = DBL_ES0;
                pass DBL_EN1 = DBL_EN0;
                pass DBL_SE2 = DBL_SE1;
                pass DBL_NE2 = DBL_NE1;

                pass PENT_EE1 = PENT_EE0;
                pass PENT_EE2 = PENT_EE1;
                pass PENT_EE3 = PENT_EE2;
                pass PENT_EE4 = PENT_EE3;
                pass PENT_EE5 = PENT_EE4;
                pass PENT_ES1 = PENT_ES0;
                pass PENT_ES2 = PENT_ES1;
                pass PENT_ES3 = PENT_ES2;
                pass PENT_EN1 = PENT_EN0;
                pass PENT_EN2 = PENT_EN1;
                pass PENT_EN3 = PENT_EN2;
                pass PENT_SE4 = PENT_SE3;
                pass PENT_SE5 = PENT_SE4;
                pass PENT_NE4 = PENT_NE3;
                pass PENT_NE5 = PENT_NE4;

                for i in 0..9 {
                    pass LH[i] = LH[i + 1];
                }
            }
            connector_class TERM_W;
            connector_class INT_BUFS_W;
            connector_class PPC_W;
        }
        if variant virtex6 {
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
                pass QUAD_SE1 = QUAD_SE0;
                pass QUAD_SE4 = QUAD_SE3;
                pass QUAD_NE1 = QUAD_NE0;
                pass QUAD_NE4 = QUAD_NE3;

                for i in 0..8 {
                    pass LH[i] = LH[i + 1];
                }
            }
            connector_class TERM_W;
        }
        if variant virtex7 {
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
                pass HEX_SE1 = HEX_SE0;
                pass HEX_SE6 = HEX_SE5;
                pass HEX_NE1 = HEX_NE0;
                pass HEX_NE6 = HEX_NE5;

                for i in 0..6 {
                    pass LH[i] = LH[i + 1];
                }
            }
            connector_class TERM_W;
        }
    }

    connector_slot E {
        opposite W;

        if variant virtex4 {
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
                for i in 13..25 {
                    pass LH[i] = LH[i - 1];
                }
            }
            connector_class TERM_E;
            connector_class CLB_BUFFER_E;
            connector_class PPC_E;
        }
        if variant virtex5 {
            connector_class PASS_E {
                pass DBL_WW1 = DBL_WW0;
                pass DBL_WW2 = DBL_WW1;
                pass DBL_WS1 = DBL_WS0;
                pass DBL_WN1 = DBL_WN0;
                pass DBL_SW2 = DBL_SW1;
                pass DBL_NW2 = DBL_NW1;

                pass PENT_WW1 = PENT_WW0;
                pass PENT_WW2 = PENT_WW1;
                pass PENT_WW3 = PENT_WW2;
                pass PENT_WW4 = PENT_WW3;
                pass PENT_WW5 = PENT_WW4;
                pass PENT_WS1 = PENT_WS0;
                pass PENT_WS2 = PENT_WS1;
                pass PENT_WS3 = PENT_WS2;
                pass PENT_WN1 = PENT_WN0;
                pass PENT_WN2 = PENT_WN1;
                pass PENT_WN3 = PENT_WN2;
                pass PENT_SW4 = PENT_SW3;
                pass PENT_SW5 = PENT_SW4;
                pass PENT_NW4 = PENT_NW3;
                pass PENT_NW5 = PENT_NW4;

                for i in 10..19 {
                    pass LH[i] = LH[i - 1];
                }
            }
            connector_class TERM_E;
            connector_class TERM_E_HOLE {
                for i in 10..19 {
                    blackhole LH[i];
                }
            }
            connector_class INT_BUFS_E;
            connector_class PPC_E;
        }
        if variant virtex6 {
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
                pass QUAD_SW1 = QUAD_SW0;
                pass QUAD_SW4 = QUAD_SW3;
                pass QUAD_NW1 = QUAD_NW0;
                pass QUAD_NW4 = QUAD_NW3;

                for i in 9..17 {
                    pass LH[i] = LH[i - 1];
                }
            }
            connector_class TERM_E;
        }
        if variant virtex7 {
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
                pass HEX_SW1 = HEX_SW0;
                pass HEX_SW6 = HEX_SW5;
                pass HEX_NW1 = HEX_NW0;
                pass HEX_NW6 = HEX_NW5;

                for i in 7..13 {
                    pass LH[i] = LH[i - 1];
                }
            }
            connector_class TERM_E;
        }
    }

    connector_slot S {
        opposite N;

        if variant virtex4 {
            connector_class PASS_S {
                pass OMUX_EN8 = OMUX_E8;
                pass OMUX_N10 = OMUX[10];
                pass OMUX_N11 = OMUX[11];
                pass OMUX_N12 = OMUX[12];
                pass OMUX_N13 = OMUX[13];
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
                for i in 0..12 {
                    pass LV[i] = LV[i + 1];
                }
            }
            connector_class TERM_S;
            connector_class BRKH_S;
            connector_class PPC_A_S;
            connector_class PPC_B_S;
        }
        if variant virtex5 {
            connector_class PASS_S {
                pass DBL_NN1 = DBL_NN0;
                pass DBL_NN2 = DBL_NN1;
                pass DBL_NW1 = DBL_NW0;
                pass DBL_NE1 = DBL_NE0;
                pass DBL_WN2 = DBL_WN1;
                pass DBL_EN2 = DBL_EN1;
                pass DBL_WW0[0] = DBL_WW0_S0;
                pass DBL_EE0[3] = DBL_EE0_S3;
                pass DBL_NN0[0] = DBL_NN0_S0;
                pass DBL_NW2_N2 = DBL_NW2[2];
                pass DBL_NE1_N2 = DBL_NE1_BUF2;

                pass PENT_NN1 = PENT_NN0;
                pass PENT_NN2 = PENT_NN1;
                pass PENT_NN3 = PENT_NN2;
                pass PENT_NN4 = PENT_NN3;
                pass PENT_NN5 = PENT_NN4;
                pass PENT_NW1 = PENT_NW0;
                pass PENT_NW2 = PENT_NW1;
                pass PENT_NW3 = PENT_NW2;
                pass PENT_NE1 = PENT_NE0;
                pass PENT_NE2 = PENT_NE1;
                pass PENT_NE3 = PENT_NE2;
                pass PENT_WN4 = PENT_WN3;
                pass PENT_WN5 = PENT_WN4;
                pass PENT_EN4 = PENT_EN3;
                pass PENT_EN5 = PENT_EN4;
                pass PENT_WW0[0] = PENT_WW0_S0;
                pass PENT_NW5_N2 = PENT_NW5[2];
                pass PENT_NE3_N2 = PENT_NE3_BUF2;

                for i in 10..19 {
                    pass LV[i] = LV[i - 1];
                }

                pass IMUX_CTRL_BOUNCE_N3 = IMUX_CTRL_BOUNCE[3];
                pass IMUX_BYP_BOUNCE_N3 = IMUX_BYP_BOUNCE[3];
                pass IMUX_BYP_BOUNCE_N7 = IMUX_BYP_BOUNCE[7];
                pass IMUX_FAN_BOUNCE_N7 = IMUX_FAN_BOUNCE[7];
                pass OUT_N15_DBL = OUT[15];
                pass OUT_N17_DBL = OUT[17];
                pass OUT_N15_PENT = OUT[15];
                pass OUT_N17_PENT = OUT[17];
            }
            connector_class TERM_S_HOLE {
                for i in 10..19 {
                    blackhole LV[i];
                }
            }
            connector_class TERM_S_PPC;
        }
        if variant virtex6 {
            connector_class PASS_S {
                pass SNG_N1 = SNG_N0;
                pass SNG_W0[4] = SNG_W0_S4;
                pass SNG_E0[4] = SNG_E0_S4;
                pass SNG_S0[4] = SNG_S0_S4;
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
                pass QUAD_NW2 = QUAD_NW1;
                pass QUAD_NW3 = QUAD_NW2;
                pass QUAD_NE2 = QUAD_NE1;
                pass QUAD_NE3 = QUAD_NE2;
                pass QUAD_SS4_N3 = QUAD_SS4[3];
                pass QUAD_SW4_N3 = QUAD_SW4[3];

                for i in 9..17 {
                    pass LV[i] = LV[i - 1];
                }

                pass IMUX_BYP_BOUNCE_N = IMUX_BYP_BOUNCE;
            }
            connector_class TERM_S;
            connector_class TERM_S_HOLE {
                for i in 9..17 {
                    blackhole LV[i];
                }
            }
        }
        if variant virtex7 {
            connector_class PASS_S {
                pass SNG_N1 = SNG_N0;
                pass SNG_W0[4] = SNG_W0_S4;
                pass SNG_E0[4] = SNG_E0_S4;
                pass SNG_S0[4] = SNG_S0_S4;
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

                pass HEX_NN1 = HEX_NN0;
                pass HEX_NN2 = HEX_NN1;
                pass HEX_NN3 = HEX_NN2;
                pass HEX_NN4 = HEX_NN3;
                pass HEX_NN5 = HEX_NN4;
                pass HEX_NN6 = HEX_NN5;
                pass HEX_NW2 = HEX_NW1;
                pass HEX_NW3 = HEX_NW2;
                pass HEX_NW4 = HEX_NW3;
                pass HEX_NW5 = HEX_NW4;
                pass HEX_NE2 = HEX_NE1;
                pass HEX_NE3 = HEX_NE2;
                pass HEX_NE4 = HEX_NE3;
                pass HEX_NE5 = HEX_NE4;
                pass HEX_SS6_N3 = HEX_SS6[3];
                pass HEX_SW6_N3 = HEX_SW6[3];

                for i in 10..19 {
                    pass LV[i] = LV[i - 1];
                }
                for i in 7..13 {
                    pass LVB[i] = LVB[i - 1];
                }

                pass IMUX_BYP_BOUNCE_N = IMUX_BYP_BOUNCE;
            }
            connector_class TERM_S;
            connector_class TERM_S_HOLE {
                for i in 10..19 {
                    blackhole LV[i];
                }
                for i in 7..13 {
                    blackhole LVB[i];
                }
            }
            connector_class BRKH_S;
        }
    }

    connector_slot N {
        opposite S;

        if variant virtex4 {
            connector_class PASS_N {
                pass OMUX_S0 = OMUX[0];
                pass OMUX_S0_ALT = OMUX[0];
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
                for i in 13..25 {
                    pass LV[i] = LV[i - 1];
                }
            }
            connector_class TERM_N;
            connector_class BRKH_N;
            connector_class PPC_A_N;
            connector_class PPC_B_N;
        }
        if variant virtex5 {
            connector_class PASS_N, PASS_NHOLE_N {
                pass DBL_SS1 = DBL_SS0;
                pass DBL_SS2 = DBL_SS1;
                pass DBL_SW1 = DBL_SW0;
                pass DBL_SE1 = DBL_SE0;
                pass DBL_WS2 = DBL_WS1;
                pass DBL_ES2 = DBL_ES1;
                pass DBL_WW0[5] = DBL_WW0_N5;
                pass DBL_NN0[5] = DBL_NN0_N5;
                pass DBL_SS0[2] = DBL_SS0_N2;
                pass DBL_WS1_S0 = DBL_WS1_BUF0;
                pass DBL_WN2_S0 = DBL_WN2[0];

                pass PENT_SS1 = PENT_SS0;
                pass PENT_SS2 = PENT_SS1;
                pass PENT_SS3 = PENT_SS2;
                pass PENT_SS4 = PENT_SS3;
                pass PENT_SS5 = PENT_SS4;
                pass PENT_SW1 = PENT_SW0;
                pass PENT_SW2 = PENT_SW1;
                pass PENT_SW3 = PENT_SW2;
                pass PENT_SE1 = PENT_SE0;
                pass PENT_SE2 = PENT_SE1;
                pass PENT_SE3 = PENT_SE2;
                pass PENT_WS4 = PENT_WS3;
                pass PENT_WS5 = PENT_WS4;
                pass PENT_ES4 = PENT_ES3;
                pass PENT_ES5 = PENT_ES4;
                pass PENT_NN0[5] = PENT_NN0_N5;
                pass PENT_WS3_S0 = PENT_WS3_BUF0;
                pass PENT_WN5_S0 = PENT_WN5[0];

                if connector_class PASS_N {
                    for i in 0..9 {
                        pass LV[i] = LV[i + 1];
                    }
                } else {
                    for i in 0..9 {
                        blackhole LV[i];
                    }
                }

                pass IMUX_CTRL_BOUNCE_S0 = IMUX_CTRL_BOUNCE[0];
                pass IMUX_BYP_BOUNCE_S0 = IMUX_BYP_BOUNCE[0];
                pass IMUX_BYP_BOUNCE_S4 = IMUX_BYP_BOUNCE[4];
                pass IMUX_FAN_BOUNCE_S0 = IMUX_FAN_BOUNCE[0];
                pass OUT_S12_DBL = OUT[12];
                pass OUT_S18_DBL = OUT[18];
                pass OUT_S12_PENT = OUT[12];
                pass OUT_S18_PENT = OUT[18];
            }
            connector_class TERM_N_HOLE {
                for i in 0..9 {
                    blackhole LV[i];
                }
            }
            connector_class TERM_N_PPC;
        }
        if variant virtex6 {
            connector_class PASS_N {
                pass SNG_S1 = SNG_S0;
                pass SNG_W0[3] = SNG_W0_N3;
                pass SNG_E0[3] = SNG_E0_N3;
                pass SNG_N0[3] = SNG_N0_N3;
                pass SNG_W1_S4 = SNG_W1[4];
                pass SNG_E1_S0 = SNG_E1[0];
                pass SNG_N1_S0 = SNG_N1[0];

                pass DBL_SS1 = DBL_SS0;
                pass DBL_SS2 = DBL_SS1;
                pass DBL_SW1 = DBL_SW0;
                pass DBL_SE1 = DBL_SE0;
                pass DBL_NN2_S0 = DBL_NN2[0];
                pass DBL_NE2_S0 = DBL_NE2[0];
                pass DBL_NW2_S0 = DBL_NW2[0];

                pass QUAD_SS1 = QUAD_SS0;
                pass QUAD_SS2 = QUAD_SS1;
                pass QUAD_SS3 = QUAD_SS2;
                pass QUAD_SS4 = QUAD_SS3;
                pass QUAD_SW2 = QUAD_SW1;
                pass QUAD_SW3 = QUAD_SW2;
                pass QUAD_SE2 = QUAD_SE1;
                pass QUAD_SE3 = QUAD_SE2;
                pass QUAD_WW4_S0 = QUAD_WW4[0];
                pass QUAD_NN4_S0 = QUAD_NN4[0];
                pass QUAD_NW4_S0 = QUAD_NW4[0];

                for i in 0..8 {
                    pass LV[i] = LV[i + 1];
                }

                pass IMUX_FAN_BOUNCE_S = IMUX_FAN_BOUNCE;
            }
            connector_class TERM_N;
            connector_class TERM_N_HOLE {
                for i in 0..8 {
                    blackhole LV[i];
                }
            }
        }
        if variant virtex7 {
            connector_class PASS_N {
                pass SNG_S1 = SNG_S0;
                pass SNG_W0[3] = SNG_W0_N3;
                pass SNG_E0[3] = SNG_E0_N3;
                pass SNG_N0[3] = SNG_N0_N3;
                pass SNG_W1_S4 = SNG_W1[4];
                pass SNG_E1_S0 = SNG_E1[0];
                pass SNG_N1_S0 = SNG_N1[0];

                pass DBL_SS1 = DBL_SS0;
                pass DBL_SS2 = DBL_SS1;
                pass DBL_SW1 = DBL_SW0;
                pass DBL_SE1 = DBL_SE0;
                pass DBL_NN2_S0 = DBL_NN2[0];
                pass DBL_NE2_S0 = DBL_NE2[0];
                pass DBL_NW2_S0 = DBL_NW2[0];

                pass QUAD_WW4_S0 = QUAD_WW4[0];

                pass HEX_SS1 = HEX_SS0;
                pass HEX_SS2 = HEX_SS1;
                pass HEX_SS3 = HEX_SS2;
                pass HEX_SS4 = HEX_SS3;
                pass HEX_SS5 = HEX_SS4;
                pass HEX_SS6 = HEX_SS5;
                pass HEX_SW2 = HEX_SW1;
                pass HEX_SW3 = HEX_SW2;
                pass HEX_SW4 = HEX_SW3;
                pass HEX_SW5 = HEX_SW4;
                pass HEX_SE2 = HEX_SE1;
                pass HEX_SE3 = HEX_SE2;
                pass HEX_SE4 = HEX_SE3;
                pass HEX_SE5 = HEX_SE4;
                pass HEX_NN6_S0 = HEX_NN6[0];
                pass HEX_NW6_S0 = HEX_NW6[0];

                for i in 0..9 {
                    pass LV[i] = LV[i + 1];
                }
                for i in 0..6 {
                    pass LVB[i] = LVB[i + 1];
                }

                pass IMUX_FAN_BOUNCE_S = IMUX_FAN_BOUNCE;
            }
            connector_class TERM_N;
            connector_class TERM_N_HOLE {
                for i in 0..9 {
                    blackhole LV[i];
                }
                for i in 0..6 {
                    blackhole LVB[i];
                }
            }
            connector_class BRKH_N;
        }
    }
}
