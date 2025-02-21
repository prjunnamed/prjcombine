use prjcombine_re_hammer::Session;
use prjcombine_xc2000::chip::ChipKind;

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let grid = backend.edev.chip;
    for tile in ["LLV.CLB", "LLV.IO.L", "LLV.IO.R"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel("CLKH");
        for (out, val, buf) in [
            ("O0", "I.LL.V", "bufgs_bl"),
            ("O0", "I.LR.H", "bufgs_br"),
            ("O0", "I.UL.H", "bufgs_tl"),
            ("O0", "I.UR.V", "bufgs_tr"),
            ("O0", "I.UL.V", "bufgp_tl"),
            ("O1", "I.LL.V", "bufgs_bl"),
            ("O1", "I.LR.H", "bufgs_br"),
            ("O1", "I.UL.H", "bufgs_tl"),
            ("O1", "I.UR.V", "bufgs_tr"),
            ("O1", "I.LL.H", "bufgp_bl"),
            ("O2", "I.LL.V", "bufgs_bl"),
            ("O2", "I.LR.H", "bufgs_br"),
            ("O2", "I.UL.H", "bufgs_tl"),
            ("O2", "I.UR.V", "bufgs_tr"),
            ("O2", "I.LR.V", "bufgp_br"),
            ("O3", "I.LL.V", "bufgs_bl"),
            ("O3", "I.LR.H", "bufgs_br"),
            ("O3", "I.UL.H", "bufgs_tl"),
            ("O3", "I.UR.V", "bufgs_tr"),
            ("O3", "I.UR.H", "bufgp_tr"),
        ] {
            bctx.build()
                .mutex(out, val)
                .test_manual(format!("MUX.{out}"), val)
                .pip_bufg(format!("{out}.{val}"), buf)
                .commit();
        }
        if grid.kind == ChipKind::Xc4000H {
            for out in ["O0", "O1", "O2", "O3"] {
                bctx.build()
                    .mutex(out, "GND")
                    .test_manual(format!("MUX.{out}"), "GND")
                    .pip_pin(format!("{out}.GND"), "O")
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let grid = ctx.edev.chip;
    for tile in ["LLV.CLB", "LLV.IO.L", "LLV.IO.R"] {
        let bel = "CLKH";
        if grid.kind != ChipKind::Xc4000H {
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.O0",
                &["I.UL.V", "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"],
                "NONE",
            );
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.O1",
                &["I.LL.H", "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"],
                "NONE",
            );
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.O2",
                &["I.LR.V", "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"],
                "NONE",
            );
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.O3",
                &["I.UR.H", "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"],
                "NONE",
            );
        } else {
            ctx.collect_enum(
                tile,
                bel,
                "MUX.O0",
                &["I.UL.V", "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V", "GND"],
            );
            ctx.collect_enum(
                tile,
                bel,
                "MUX.O1",
                &["I.LL.H", "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V", "GND"],
            );
            ctx.collect_enum(
                tile,
                bel,
                "MUX.O2",
                &["I.LR.V", "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V", "GND"],
            );
            ctx.collect_enum(
                tile,
                bel,
                "MUX.O3",
                &["I.UR.H", "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V", "GND"],
            );
        }
    }
}
