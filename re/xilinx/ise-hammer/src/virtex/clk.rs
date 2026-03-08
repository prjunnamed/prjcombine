use prjcombine_interconnect::dir::DirV;
use prjcombine_re_collector::diff::OcdMode;
use prjcombine_re_hammer::Session;
use prjcombine_virtex::defs::{
    bcls::{BUFGCE, GCLK_IOB},
    bslots, enums, tcls,
};

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

use super::io::VirtexOtherIobInput;

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let package = backend.ebonds.keys().next().unwrap();
    for (tcid, side, is_ve) in [
        (tcls::CLK_S_V, DirV::S, false),
        (tcls::CLK_N_V, DirV::N, false),
        (tcls::CLK_S_VE_4DLL, DirV::S, true),
        (tcls::CLK_N_VE_4DLL, DirV::N, true),
        (tcls::CLK_S_VE_2DLL, DirV::S, true),
        (tcls::CLK_N_VE_2DLL, DirV::N, true),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for i in 0..2 {
            let mut bctx = ctx.bel(bslots::GCLK_IOB[i]);
            let iostds = if !is_ve {
                &[
                    ("LVTTL", enums::IOB_IBUF_MODE::CMOS),
                    ("LVCMOS2", enums::IOB_IBUF_MODE::CMOS),
                    ("PCI33_3", enums::IOB_IBUF_MODE::CMOS),
                    ("PCI33_5", enums::IOB_IBUF_MODE::CMOS),
                    ("PCI66_3", enums::IOB_IBUF_MODE::CMOS),
                    ("GTL", enums::IOB_IBUF_MODE::VREF_LV),
                    ("GTLP", enums::IOB_IBUF_MODE::VREF_HV),
                    ("HSTL_I", enums::IOB_IBUF_MODE::VREF_LV),
                    ("HSTL_III", enums::IOB_IBUF_MODE::VREF_LV),
                    ("HSTL_IV", enums::IOB_IBUF_MODE::VREF_LV),
                    ("SSTL3_I", enums::IOB_IBUF_MODE::VREF_HV),
                    ("SSTL3_II", enums::IOB_IBUF_MODE::VREF_HV),
                    ("SSTL2_I", enums::IOB_IBUF_MODE::VREF_HV),
                    ("SSTL2_II", enums::IOB_IBUF_MODE::VREF_HV),
                    ("CTT", enums::IOB_IBUF_MODE::VREF_HV),
                    ("AGP", enums::IOB_IBUF_MODE::VREF_HV),
                ][..]
            } else {
                &[
                    ("LVTTL", enums::IOB_IBUF_MODE::CMOS),
                    ("LVCMOS2", enums::IOB_IBUF_MODE::CMOS),
                    ("LVCMOS18", enums::IOB_IBUF_MODE::CMOS),
                    ("PCI33_3", enums::IOB_IBUF_MODE::CMOS),
                    ("PCI66_3", enums::IOB_IBUF_MODE::CMOS),
                    ("PCIX66_3", enums::IOB_IBUF_MODE::CMOS),
                    ("GTL", enums::IOB_IBUF_MODE::VREF),
                    ("GTLP", enums::IOB_IBUF_MODE::VREF),
                    ("HSTL_I", enums::IOB_IBUF_MODE::VREF),
                    ("HSTL_III", enums::IOB_IBUF_MODE::VREF),
                    ("HSTL_IV", enums::IOB_IBUF_MODE::VREF),
                    ("SSTL3_I", enums::IOB_IBUF_MODE::VREF),
                    ("SSTL3_II", enums::IOB_IBUF_MODE::VREF),
                    ("SSTL2_I", enums::IOB_IBUF_MODE::VREF),
                    ("SSTL2_II", enums::IOB_IBUF_MODE::VREF),
                    ("CTT", enums::IOB_IBUF_MODE::VREF),
                    ("AGP", enums::IOB_IBUF_MODE::VREF),
                    ("LVDS", enums::IOB_IBUF_MODE::DIFF),
                    ("LVPECL", enums::IOB_IBUF_MODE::DIFF),
                ][..]
            };
            for &(iostd, val) in iostds {
                bctx.build()
                    .global_mutex("GCLKIOB", "YES")
                    .raw(Key::Package, package)
                    .global_mutex("VREF", "YES")
                    .prop(VirtexOtherIobInput(bslots::GCLK_IOB[i], "GTL".into()))
                    .global("UNUSEDPIN", "PULLNONE")
                    .test_bel_attr_val(GCLK_IOB::IBUF_MODE, val)
                    .mode("GCLKIOB")
                    .attr("IOATTRBOX", iostd)
                    .commit();
            }
            let idx = if side == DirV::S { i } else { 2 + i };
            for (i, val) in ["11110", "11101", "11011", "10111", "01111"]
                .into_iter()
                .enumerate()
            {
                bctx.mode("GCLKIOB")
                    .test_bel_attr_bits_base(GCLK_IOB::DELAY, i)
                    .global_diff_none(format!("GCLKDEL{idx}"), val)
                    .commit();
            }
        }
        // TODO: IOFB
        for i in 0..2 {
            let mut bctx = ctx.bel(bslots::BUFGCE[i]);
            for (val, vname) in [(false, "1"), (false, "CE"), (true, "0"), (true, "CE_B")] {
                bctx.mode("GCLK")
                    .pin("CE")
                    .test_bel_input_inv(BUFGCE::CE, val)
                    .attr("CEMUX", vname)
                    .commit();
            }
            bctx.mode("GCLK").test_bel_attr_bool_rename(
                "DISABLE_ATTR",
                BUFGCE::INIT_OUT,
                "LOW",
                "HIGH",
            );
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [
        tcls::CLK_S_V,
        tcls::CLK_S_VE_4DLL,
        tcls::CLK_S_VE_2DLL,
        tcls::CLK_N_V,
        tcls::CLK_N_VE_4DLL,
        tcls::CLK_N_VE_2DLL,
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for i in 0..2 {
            let bslot = bslots::GCLK_IOB[i];
            ctx.collect_bel_attr(tcid, bslot, GCLK_IOB::DELAY);
            if matches!(tcid, tcls::CLK_S_V | tcls::CLK_N_V) {
                ctx.collect_bel_attr_subset_default_ocd(
                    tcid,
                    bslot,
                    GCLK_IOB::IBUF_MODE,
                    &[
                        enums::IOB_IBUF_MODE::CMOS,
                        enums::IOB_IBUF_MODE::VREF_LV,
                        enums::IOB_IBUF_MODE::VREF_HV,
                    ],
                    enums::IOB_IBUF_MODE::NONE,
                    OcdMode::ValueOrder,
                );
            } else {
                ctx.collect_bel_attr_subset_default_ocd(
                    tcid,
                    bslot,
                    GCLK_IOB::IBUF_MODE,
                    &[
                        enums::IOB_IBUF_MODE::CMOS,
                        enums::IOB_IBUF_MODE::VREF,
                        enums::IOB_IBUF_MODE::DIFF,
                    ],
                    enums::IOB_IBUF_MODE::NONE,
                    OcdMode::ValueOrder,
                );
            }
        }
        for i in 0..2 {
            let bslot = bslots::BUFGCE[i];
            ctx.collect_bel_input_inv_bi(tcid, bslot, BUFGCE::CE);
            ctx.collect_bel_attr_bi(tcid, bslot, BUFGCE::INIT_OUT);
        }
    }
}
