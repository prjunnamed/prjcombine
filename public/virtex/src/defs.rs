use prjcombine_tablegen::target_defs;

target_defs! {
    region_slot GLOBAL;
    region_slot LEAF;
    region_slot PCI_CE;

    wire GCLK[4]: regional LEAF;
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

    wire LH[12]: multi_branch W;
    wire LH_FAKE0: mux;
    wire LH_FAKE6: mux;

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

    tile_slot MAIN {
        bel_slot INT: routing;
        bel_slot SLICE[2]: legacy;
        bel_slot TBUF[2]: legacy;
        bel_slot TBUS: legacy;

        tile_class CLB {
            cell CELL;
            bitrect MAIN: MAIN;
        }

        bel_slot IO[4]: legacy;

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

        bel_slot BRAM: legacy;
        bel_slot CLKV_BRAM: legacy;
        tile_class BRAM_W {
            cell CELL[4];
            cell CELL_W[4];
            cell CELL_E[4];
            bitrect MAIN[4]: BRAM;
            bitrect DATA: BRAM_DATA;
        }
        tile_class BRAM_E {
            cell CELL[4];
            cell CELL_W[4];
            cell CELL_E[4];
            bitrect MAIN[4]: BRAM;
            bitrect DATA: BRAM_DATA;
        }
        tile_class BRAM_M {
            cell CELL[4];
            cell CELL_W[4];
            cell CELL_E[4];
            bitrect MAIN[4]: BRAM;
            bitrect DATA: BRAM_DATA;
        }

        bel_slot CAPTURE: legacy;
        bel_slot STARTUP: legacy;
        bel_slot BSCAN: legacy;
        tile_class CNR_SW {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
        tile_class CNR_SE {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
        tile_class CNR_NW {
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
        bel_slot DLL: legacy;

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
        tile_class IOB_W_V, IOB_W_VE {
            bitrect MAIN: IO_WE;
        }
        tile_class IOB_E_V, IOB_E_VE {
            bitrect MAIN: IO_WE;
        }
        tile_class IOB_S_V, IOB_S_VE {
            bitrect MAIN: MAIN;
        }
        tile_class IOB_N_V, IOB_N_VE {
            bitrect MAIN: MAIN;
        }
    }

    tile_slot PCILOGIC {
        bel_slot PCI_INT: routing;
        bel_slot PCILOGIC: legacy;

        tile_class PCI_W, PCI_E {
            cell CELL;
            bitrect MAIN: IO_WE;
        }
    }

    tile_slot CLK_SN {
        bel_slot GCLK_INT: routing;
        bel_slot GCLK_IO[2]: legacy;
        bel_slot BUFG[2]: legacy;
        bel_slot IOFB[2]: legacy;

        tile_class CLK_S_V, CLK_N_V {
            cell CELL, DLL_W, DLL_E;
            bitrect CLK[2]: CLK;
        }
        tile_class CLK_S_VE_4DLL, CLK_S_VE_2DLL, CLK_N_VE_4DLL, CLK_N_VE_2DLL {
            cell CELL, DLLP_W, DLLP_E, DLLS_W, DLLS_E;
            bitrect CLK[2]: CLK;
        }
    }

    tile_slot CLKC {
        bel_slot CLKC: legacy;
        bel_slot GCLKC: legacy;
        bel_slot CLKH: legacy;
        bel_slot BRAM_CLKH: legacy;
        tile_class CLKC {
        }
        tile_class GCLKC {
        }
        tile_class BRAM_CLKH {
            cell CELL;
        }
    }

    tile_slot CLKV {
        bel_slot CLKV: legacy;
        bel_slot CLKV_BRAM_S: legacy;
        bel_slot CLKV_BRAM_N: legacy;
        tile_class CLKV_CLKV, CLKV_GCLKV {
            cell W, E;
            bitrect CLKV: CLKV;
        }
        tile_class CLKV_NULL {
            cell W, E;
        }
        tile_class CLKV_BRAM_S, CLKV_BRAM_N {
            cell CELL, W, BRAM;
            bitrect MAIN: BRAM;
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
