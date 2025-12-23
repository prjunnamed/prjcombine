use prjcombine_entity::id::EntityStaticRange;
use prjcombine_interconnect::db::WireSlotId;
use prjcombine_tablegen::target_defs;

target_defs! {
    // Connected across the whole device.
    region_slot GLOBAL;

    // Group of cells sharing the column buffer leaf.  For devices without column buffers, same as
    // `GLOBAL`.
    region_slot COLBUF;

    wire TIE_0: tie 0;
    wire TIE_1: tie 1;

    // The global wires.
    wire GLOBAL[8]: regional GLOBAL;

    // Helper wires used to route `GLOBAL` wires towards `LOCAL` wires.
    wire GLOBAL_OUT[4]: mux;

    // Length-4 interconnect.
    wire QUAD_H0[12]: multi_root;
    for i in 1..=4 {
        wire "QUAD_H{i}"[12]: multi_branch W;
    }
    wire QUAD_V0[12]: multi_root;
    for i in 1..=4 {
        wire "QUAD_V{i}"[12]: multi_branch S;
        wire "QUAD_V{i}_W"[12]: multi_branch E;
    }

    // Length-12 interconnect.
    wire LONG_H0[2]: multi_root;
    for i in 1..=12 {
        wire "LONG_H{i}"[2]: multi_branch W;
    }
    wire LONG_V0[2]: multi_root;
    for i in 1..=12 {
        wire "LONG_V{i}"[2]: multi_branch S;
    }

    // Local interconnect.  All signals going to `IMUX_*` must go through these wires, except
    // for direct `GLOBAL` → `IMUX_CLK`/`IMUX_CE`/`IMUX_RST` paths.
    for i in 0..4 {
        wire "LOCAL_{i}"[8]: mux;
    }

    // General interconnect inputs to LUTs, BRAMs, and iCE40T04/T01/T05 hard IP.
    wire IMUX_LC_I0[8]: mux;
    wire IMUX_LC_I1[8]: mux;
    wire IMUX_LC_I2[8]: mux;
    wire IMUX_LC_I3[8]: mux;

    // Control inputs to LUTs, BRAMs, and iCE40T04/T01/T05 hard IP.
    // `IMUX_CE` is also used for IOIs.
    wire IMUX_CLK: mux;
    wire IMUX_CLK_OPTINV: mux;
    wire IMUX_RST: mux;
    wire IMUX_CE: mux;

    // General interconnect inputs to IOIs.
    wire IMUX_IO_DOUT0[2]: mux;
    wire IMUX_IO_DOUT1[2]: mux;
    wire IMUX_IO_OE[2]: mux;

    // Control inputs to IOIs.  `IMUX_CE` is also used.
    wire IMUX_IO_ICLK: mux;
    wire IMUX_IO_ICLK_OPTINV: mux;
    wire IMUX_IO_OCLK: mux;
    wire IMUX_IO_OCLK_OPTINV: mux;

    // General interconnect input for misc stuff.  Located in IOI tiles.
    wire IMUX_IO_EXTRA: mux;

    // Bel outputs.  For IOI tiles, `OUT_LC[4..8]` are the same as `OUT_LC[0..4]`.  For special
    // PLL outputs in corner cells, all 8 `OUT_LC` wires are the same.
    wire OUT_LC[8]: bel;
    wire OUT_LC_N[8]: branch S;
    wire OUT_LC_S[8]: branch N;
    wire OUT_LC_E[8]: branch W;
    wire OUT_LC_EN[8]: branch S;
    wire OUT_LC_ES[8]: branch N;
    wire OUT_LC_W[8]: branch E;
    wire OUT_LC_WN[8]: branch S;
    wire OUT_LC_WS[8]: branch N;

    bitrect PLB = horizontal (16, 54);
    bitrect BRAM = horizontal (16, 42);
    bitrect IOI_WE = horizontal (16, 18);
    bitrect CLK = horizontal (16, 2);
    bitrect BRAM_DATA = horizontal (256, 16);

    // Main interconnect, LCs, IOIs.
    tile_slot MAIN {
        // The main interconnect switchbox.
        bel_slot INT: routing;

        bel_slot LC[8]: legacy;
        tile_class PLB_L04, PLB_L08, PLB_P01 {
            cell CELL;
            bitrect MAIN: PLB;

            switchbox INT {
                // filled by harvester
            }
        }

        // Two `INT_BRAM` tiles for every `BRAM` tile.
        tile_class INT_BRAM {
            cell CELL;
            bitrect MAIN: BRAM;

            switchbox INT {
                // filled by harvester
            }
        }

        bel_slot IOI[2]: legacy;
        tile_class IOI_W_L04, IOI_E_L04, IOI_W_L08, IOI_E_L08, IOI_S_L04, IOI_N_L04, IOI_S_L08, IOI_N_L08, IOI_S_T04, IOI_N_T04 {
            cell CELL;
            if tile_class [IOI_W_L04, IOI_E_L04, IOI_W_L08, IOI_E_L08] {
                bitrect MAIN: IOI_WE;
            } else {
                bitrect MAIN: BRAM;
            }

            switchbox INT {
                // filled by harvester
            }
        }
    }

    // Global wire column buffers.
    tile_slot COLBUF {
        bel_slot COLBUF: routing;
        tile_class COLBUF_L01, COLBUF_P08, COLBUF_IO_W, COLBUF_IO_E {
            cell CELL;
            if tile_class [COLBUF_IO_W, COLBUF_IO_E] {
                bitrect MAIN: IOI_WE;
            } else {
                bitrect MAIN: BRAM;
            }
        }
    }

    // Global wire root muxes.
    tile_slot GB_ROOT {
        bel_slot GB_ROOT: routing;
        tile_class GB_ROOT_L04, GB_ROOT_L08 {
            cell CELL;
            bitrect CLK[2]: CLK;
        }
    }

    // Used for most bels.
    tile_slot BEL {
        bel_slot IO_LATCH: routing;
        tile_class IO_LATCH {
            cell CELL;
        }

        bel_slot GB_FABRIC: legacy;
        tile_class GB_FABRIC {
            cell CELL;
        }

        bel_slot BRAM: legacy;
        tile_class BRAM_L04, BRAM_P01, BRAM_P08 {
            cell CELL[2];
            bitrect MAIN[2]: BRAM;
            bitrect DATA: BRAM_DATA;
        }

        bel_slot MAC16: legacy;
        tile_class MAC16, MAC16_TRIM {
            cell CELL[5];
            bitrect MAIN[5]: PLB;
        }

        bel_slot SPRAM[2]: legacy;
        tile_class SPRAM {
            cell CELL[4];
            bitrect MAIN[4]: PLB;
        }

        bel_slot PLL: legacy;
        tile_class PLL_S_P04 {
            // cells filled by harvester
            bitrect CLK[2]: CLK;
        }
        tile_class
            PLL_S_P01,
            PLL_S_P08, PLL_N_P08,
            PLL_S_R04, PLL_N_R04,
            PLL_S_T01
        {
            // filled by harvester
        }

        bel_slot SPI: legacy;
        tile_class SPI_R04, SPI_T04, SPI_T05 {
            // filled by harvester
        }

        bel_slot I2C: legacy;
        tile_class I2C_R04, I2C_T04 {
            // filled by harvester
        }

        bel_slot I2C_FIFO: legacy;
        tile_class I2C_FIFO {
            // filled by harvester
        }

        bel_slot IOB_I3C[2]: legacy;
        bel_slot FILTER[2]: legacy;
        tile_class I3C {
            // fileld by harvester
        }
    }

    tile_slot WARMBOOT {
        bel_slot WARMBOOT: legacy;
        tile_class WARMBOOT, WARMBOOT_T01;
    }

    tile_slot OSC {
        bel_slot LSOSC: legacy;
        tile_class LSOSC;

        bel_slot HSOSC: legacy;
        tile_class HSOSC;

        bel_slot HFOSC: legacy;
        tile_class HFOSC_T04, HFOSC_T01;

        bel_slot LFOSC: legacy;
        tile_class LFOSC_T04, LFOSC_T01;
    }

    tile_slot LED_IP {
        bel_slot LEDD_IP: legacy;
        tile_class LEDD_IP_T04, LEDD_IP_T01, LEDD_IP_T05;

        bel_slot IR_IP: legacy;
        tile_class IR_IP;
    }

    tile_slot LED_DRV {
        bel_slot RGB_DRV: legacy;
        tile_class RGB_DRV_T04, RGB_DRV_T01, RGB_DRV_T05;

        bel_slot IR_DRV: legacy;
        tile_class IR_DRV;

        bel_slot IR400_DRV: legacy;
        bel_slot BARCODE_DRV: legacy;
        tile_class IR500_DRV;
    }

    tile_slot LED_DRV_CUR {
        bel_slot LED_DRV_CUR: legacy;
        tile_class LED_DRV_CUR_T04, LED_DRV_CUR_T05, LED_DRV_CUR_T01;
    }

    tile_slot SMCCLK {
        bel_slot SMCCLK: legacy;
        tile_class SMCCLK_T04, SMCCLK_T05, SMCCLK_T01;
    }

    tile_slot PLL_STUB {
        tile_class PLL_STUB_S;
    }

    tile_slot TRIM {
        tile_class TRIM_T04, TRIM_T01 {
            // filled by harvester
        }
    }

    // The I/O buffers.
    tile_slot IOB {
        tile_class
            IOB_W_L04, IOB_E_L04, IOB_S_L04, IOB_N_L04,
            IOB_W_P04, IOB_E_P04, IOB_S_P04, IOB_N_P04,
            IOB_W_L08, IOB_E_L08, IOB_S_L08, IOB_N_L08,
            IOB_W_L01, IOB_E_L01, IOB_S_L01, IOB_N_L01,
            IOB_W_P01, IOB_E_P01, IOB_S_P01, IOB_N_P01,
            IOB_W_P08, IOB_E_P08, IOB_S_P08, IOB_N_P08,
            IOB_W_P03, IOB_E_P03, IOB_S_P03, IOB_N_P03,
            IOB_S_R04, IOB_N_R04,
            IOB_S_T04, IOB_N_T04,
            IOB_S_T05, IOB_N_T05,
            IOB_S_T01, IOB_N_T01 {
            cell CELL;
            if tile_class ["IOB_W_*", "IOB_E_*"] {
                bitrect MAIN: IOI_WE;
            } else {
                bitrect MAIN: BRAM;
            }
        }
    }

    connector_slot W {
        opposite E;
        connector_class PASS_W {
            for i in 0..4 {
                pass "QUAD_H{i + 1}" = "QUAD_H{i}";
            }
            for i in 0..12 {
                pass "LONG_H{i + 1}" = "LONG_H{i}";
            }
            pass OUT_LC_E = OUT_LC;
        }
    }

    connector_slot E {
        opposite W;
        connector_class PASS_E {
            for i in 1..=4 {
                pass "QUAD_V{i}_W" = "QUAD_V{i}";
            }
            pass OUT_LC_W = OUT_LC;
        }
    }

    connector_slot S {
        opposite N;
        connector_class PASS_S {
            for i in 0..4 {
                pass "QUAD_V{i + 1}" = "QUAD_V{i}";
            }
            for i in 0..12 {
                pass "LONG_V{i + 1}" = "LONG_V{i}";
            }
            pass OUT_LC_N = OUT_LC;
            pass OUT_LC_WN = OUT_LC_W;
            pass OUT_LC_EN = OUT_LC_E;
        }
    }

    connector_slot N {
        opposite S;
        connector_class PASS_N {
            pass OUT_LC_S = OUT_LC;
            pass OUT_LC_WS = OUT_LC_W;
            pass OUT_LC_ES = OUT_LC_E;
        }
    }
}

pub const QUAD_H: &[EntityStaticRange<WireSlotId, 12>; 5] = &[
    wires::QUAD_H0,
    wires::QUAD_H1,
    wires::QUAD_H2,
    wires::QUAD_H3,
    wires::QUAD_H4,
];

pub const QUAD_V: &[EntityStaticRange<WireSlotId, 12>; 5] = &[
    wires::QUAD_V0,
    wires::QUAD_V1,
    wires::QUAD_V2,
    wires::QUAD_V3,
    wires::QUAD_V4,
];
