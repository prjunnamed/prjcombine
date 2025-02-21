use bitvec::prelude::*;
use prjcombine_re_collector::extract_bitvec_val;
use prjcombine_re_hammer::Session;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_inv,
    fuzz_multi, fuzz_multi_attr_hex, fuzz_one,
};

const PPC_INVPINS: &[&str] = &[
    "CPMC440CLK",
    "CPMC440TIMERCLOCK",
    "CPMDCRCLK",
    "CPMDMA0LLCLK",
    "CPMDMA1LLCLK",
    "CPMDMA2LLCLK",
    "CPMDMA3LLCLK",
    "CPMFCMCLK",
    "CPMINTERCONNECTCLK",
    "CPMMCCLK",
    "CPMPPCMPLBCLK",
    "CPMPPCS0PLBCLK",
    "CPMPPCS1PLBCLK",
    "JTGC440TCK",
];

const PPC_BOOL_ATTRS: &[&str] = &[
    "DCR_AUTOLOCK_ENABLE",
    "MI_CONTROL_BIT6",
    "PPCDM_ASYNCMODE",
    "PPCDS_ASYNCMODE",
    "PPCS0_WIDTH_128N64",
    "PPCS1_WIDTH_128N64",
];

const PPC_HEX_ATTRS: &[(&str, usize)] = &[
    ("APU_CONTROL", 17),
    ("APU_UDI0", 24),
    ("APU_UDI1", 24),
    ("APU_UDI2", 24),
    ("APU_UDI3", 24),
    ("APU_UDI4", 24),
    ("APU_UDI5", 24),
    ("APU_UDI6", 24),
    ("APU_UDI7", 24),
    ("APU_UDI8", 24),
    ("APU_UDI9", 24),
    ("APU_UDI10", 24),
    ("APU_UDI11", 24),
    ("APU_UDI12", 24),
    ("APU_UDI13", 24),
    ("APU_UDI14", 24),
    ("APU_UDI15", 24),
    ("DMA0_CONTROL", 8),
    ("DMA0_RXCHANNELCTRL", 32),
    ("DMA0_TXCHANNELCTRL", 32),
    ("DMA0_RXIRQTIMER", 10),
    ("DMA0_TXIRQTIMER", 10),
    ("DMA1_CONTROL", 8),
    ("DMA1_RXCHANNELCTRL", 32),
    ("DMA1_TXCHANNELCTRL", 32),
    ("DMA1_RXIRQTIMER", 10),
    ("DMA1_TXIRQTIMER", 10),
    ("DMA2_CONTROL", 8),
    ("DMA2_RXCHANNELCTRL", 32),
    ("DMA2_TXCHANNELCTRL", 32),
    ("DMA2_RXIRQTIMER", 10),
    ("DMA2_TXIRQTIMER", 10),
    ("DMA3_CONTROL", 8),
    ("DMA3_RXCHANNELCTRL", 32),
    ("DMA3_TXCHANNELCTRL", 32),
    ("DMA3_RXIRQTIMER", 10),
    ("DMA3_TXIRQTIMER", 10),
    ("INTERCONNECT_IMASK", 32),
    ("INTERCONNECT_TMPL_SEL", 32),
    ("MI_ARBCONFIG", 32),
    ("MI_BANKCONFLICT_MASK", 32),
    ("MI_CONTROL", 32),
    ("MI_ROWCONFLICT_MASK", 32),
    ("PPCM_ARBCONFIG", 32),
    ("PPCM_CONTROL", 32),
    ("PPCM_COUNTER", 32),
    ("PPCS0_CONTROL", 32),
    ("PPCS1_CONTROL", 32),
    ("PPCS0_ADDRMAP_TMPL0", 32),
    ("PPCS1_ADDRMAP_TMPL0", 32),
    ("XBAR_ADDRMAP_TMPL0", 32),
    ("PPCS0_ADDRMAP_TMPL1", 32),
    ("PPCS1_ADDRMAP_TMPL1", 32),
    ("XBAR_ADDRMAP_TMPL1", 32),
    ("PPCS0_ADDRMAP_TMPL2", 32),
    ("PPCS1_ADDRMAP_TMPL2", 32),
    ("XBAR_ADDRMAP_TMPL2", 32),
    ("PPCS0_ADDRMAP_TMPL3", 32),
    ("PPCS1_ADDRMAP_TMPL3", 32),
    ("XBAR_ADDRMAP_TMPL3", 32),
    ("APU_TEST", 3),
    ("DCR_TEST", 3),
    ("DMA_TEST", 3),
    ("MIB_TEST", 3),
    ("PLB_TEST", 4),
];

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    devdata_only: bool,
) {
    let Some(ctx) = FuzzCtx::try_new(session, backend, "PPC", "PPC", TileBits::MainAuto) else {
        return;
    };

    if !devdata_only {
        fuzz_one!(ctx, "PRESENT", "1", [
            (no_global_opt "PPCCLKDLY")
        ], [
            (mode "PPC440")
        ]);

        for &pin in PPC_INVPINS {
            fuzz_inv!(ctx, pin, [
                (no_global_opt "PPCCLKDLY"),
                (mode "PPC440")
            ]);
        }
        for &attr in PPC_BOOL_ATTRS {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (no_global_opt "PPCCLKDLY"),
                (mode "PPC440")
            ]);
        }
        for &(attr, width) in PPC_HEX_ATTRS {
            fuzz_multi_attr_hex!(ctx, attr, width, [
                (no_global_opt "PPCCLKDLY"),
                (mode "PPC440")
            ]);
        }
        fuzz_multi!(ctx, "CLOCK_DELAY", "", 5, [
            (mode "PPC440"),
            (attr "CLOCK_DELAY", "TRUE")
        ], (global_bin "PPCCLKDLY"));
        fuzz_enum!(ctx, "CLOCK_DELAY", ["FALSE", "TRUE"], [
            (no_global_opt "PPCCLKDLY"),
            (mode "PPC440")
        ]);
    } else {
        fuzz_enum!(ctx, "CLOCK_DELAY", ["FALSE"], [
            (no_global_opt "PPCCLKDLY"),
            (mode "PPC440")
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    if !ctx.has_tile("PPC") {
        return;
    }
    let tile = "PPC";
    let bel = "PPC";
    if !devdata_only {
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for &pin in PPC_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        ctx.collect_bitvec(tile, bel, "CLOCK_DELAY", "");
        for &attr in PPC_BOOL_ATTRS {
            if attr == "MI_CONTROL_BIT6" {
                ctx.state.get_diff(tile, bel, attr, "FALSE").assert_empty();
                ctx.state.get_diff(tile, bel, attr, "TRUE").assert_empty();
            } else {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            }
        }
        for &(attr, _) in PPC_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        ctx.state
            .get_diff(tile, bel, "CLOCK_DELAY", "TRUE")
            .assert_empty();
    }
    let diff = ctx.state.get_diff(tile, bel, "CLOCK_DELAY", "FALSE");
    let val = extract_bitvec_val(
        ctx.tiledb.item(tile, bel, "CLOCK_DELAY"),
        &bitvec![0; 5],
        diff,
    );
    ctx.insert_device_data("PPC:CLOCK_DELAY", val);
}
