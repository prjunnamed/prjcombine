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

    // Main interconnect, LCs, IOIs.
    tile_slot MAIN {
        // The main interconnect switchbox.
        bel_slot INT: routing;

        bel_slot LC[8]: legacy;

        bel_slot IO[2]: legacy;
    }

    // Global wire column buffers.
    tile_slot COLBUF {
        bel_slot COLBUF: routing;
    }

    // Global wire root muxes.
    tile_slot GB_ROOT {
        bel_slot GB_ROOT: routing;
    }

    // Used for most bels.
    tile_slot BEL {
        bel_slot IO_LATCH: legacy;
        bel_slot GB_FABRIC: legacy;

        bel_slot BRAM: legacy;

        bel_slot MAC16: legacy;

        bel_slot SPRAM[2]: legacy;

        bel_slot PLL: legacy;

        bel_slot SPI: legacy;

        bel_slot I2C: legacy;

        bel_slot I2C_FIFO: legacy;

        bel_slot IOB_I3C[2]: legacy;
        bel_slot FILTER[2]: legacy;
    }

    tile_slot WARMBOOT {
        bel_slot WARMBOOT: legacy;
    }

    tile_slot OSC {
        bel_slot LSOSC: legacy;
        bel_slot HSOSC: legacy;
        bel_slot HFOSC: legacy;
        bel_slot LFOSC: legacy;
    }

    tile_slot LED_IP {
        bel_slot LEDD_IP: legacy;
        bel_slot IR_IP: legacy;
    }

    tile_slot LED_DRV {
        bel_slot RGB_DRV: legacy;
        bel_slot IR_DRV: legacy;
        bel_slot IR400_DRV: legacy;
        bel_slot BARCODE_DRV: legacy;
    }

    tile_slot LED_DRV_CUR {
        bel_slot LED_DRV_CUR: legacy;
    }

    tile_slot SMCCLK {
        bel_slot SMCCLK: legacy;
    }

    tile_slot PLL_STUB {
    }

    tile_slot TRIM {
    }

    // The I/O buffers.
    tile_slot IOB {
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
