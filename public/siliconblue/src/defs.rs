use prjcombine_tablegen::target_defs;

target_defs! {
    // Connected across the whole device.
    region_slot GLOBAL;

    // Group of cells sharing the column buffer leaf.  For devices without column buffers, same as
    // `GLOBAL`.
    region_slot COLBUF;

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
    }

    connector_slot E {
        opposite W;
    }

    connector_slot S {
        opposite N;
    }

    connector_slot N {
        opposite S;
    }
}
