use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_fpga_hammer::{
    Diff, DiffKey, FeatureId, FuzzerFeature, FuzzerProp, xlat_bit, xlat_enum,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex2::tslots;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Virtex2,
    Spartan3,
    Virtex4,
}

#[derive(Clone, Debug)]
struct RandorInit;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for RandorInit {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        if tcrd.row != edev.chip.row_n() {
            return None;
        }
        if tcrd.col == edev.chip.col_w() + 1 {
            let tcrd = tcrd.delta(-1, 0).tile(tslots::RANDOR);
            let tile = &backend.edev[tcrd];
            let tcls = backend.edev.db.tile_classes.key(tile.class);
            let DiffKey::Legacy(first_feature_id) =
                fuzzer.info.features.first().unwrap().key.clone()
            else {
                unreachable!()
            };
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: tcls.into(),
                    bel: "RANDOR_INIT".into(),
                    ..first_feature_id
                }),
                rects: backend.edev.tile_bits(tcrd),
            });
            Some((fuzzer, false))
        } else {
            Some((fuzzer, true))
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mode = match backend.edev {
        ExpandedDevice::Virtex2(edev) => {
            if edev.chip.kind.is_virtex2() {
                Mode::Virtex2
            } else {
                Mode::Spartan3
            }
        }
        ExpandedDevice::Virtex4(_) => Mode::Virtex4,
        _ => unreachable!(),
    };

    let (bk_l, bk_m) = if mode == Mode::Virtex2 {
        ("SLICE", "SLICE")
    } else {
        ("SLICEL", "SLICEM")
    };
    let mut ctx = FuzzCtx::new(session, backend, "CLB");
    let slots = match mode {
        Mode::Virtex2 | Mode::Spartan3 => prjcombine_virtex2::bels::SLICE,
        Mode::Virtex4 => prjcombine_virtex4::bels::SLICE,
    };
    for i in 0..4 {
        let mut bctx = ctx.bel(slots[i]);
        let is_m = match mode {
            Mode::Virtex2 => true,
            Mode::Spartan3 | Mode::Virtex4 => matches!(i, 0 | 2),
        };

        // inverters
        bctx.mode(bk_l).attr("FFX", "#FF").pin("XQ").test_inv("CE");
        bctx.mode(bk_l).attr("FFX", "#FF").pin("XQ").test_inv("CLK");
        bctx.mode(bk_l)
            .attr("FFX", "#FF")
            .attr("FFY", "#FF")
            .attr("SRFFMUX", if mode == Mode::Virtex2 { "0" } else { "" })
            .pin("XQ")
            .pin("YQ")
            .test_inv("SR");
        bctx.mode(bk_l)
            .attr("FFX", "#FF")
            .attr("XUSED", "0")
            .attr("DXMUX", if mode == Mode::Virtex4 { "BX" } else { "0" })
            .pin("X")
            .pin("XQ")
            .test_inv("BX");
        bctx.mode(bk_l)
            .attr("FFY", "#FF")
            .attr("YUSED", "0")
            .attr("DYMUX", if mode == Mode::Virtex4 { "BY" } else { "0" })
            .pin("Y")
            .pin("YQ")
            .test_inv("BY");

        // LUT
        for attr in ["F", "G"] {
            bctx.mode(bk_l).test_multi_attr_lut(attr, 16);
        }

        // carry chain
        if mode != Mode::Virtex4 {
            bctx.mode(bk_l)
                .attr("BXINV", "BX")
                .attr("CYSELF", "1")
                .attr("CYSELG", "1")
                .attr("COUTUSED", "0")
                .pin("CIN")
                .pin("BX")
                .pin("COUT")
                .test_enum("CYINIT", &["CIN", "BX"]);
            bctx.mode(bk_l)
                .attr("F", "#LUT:0")
                .attr("CY0F", "0")
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX")
                .attr("CYSELG", "1")
                .attr("COUTUSED", "0")
                .pin("BX")
                .pin("COUT")
                .test_enum("CYSELF", &["F", "1"]);
            bctx.mode(bk_l)
                .attr("G", "#LUT:0")
                .attr("CY0G", "0")
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX")
                .attr("CYSELF", "1")
                .attr("COUTUSED", "0")
                .pin("BX")
                .pin("COUT")
                .test_enum("CYSELG", &["G", "1"]);
            bctx.mode(bk_l)
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX")
                .attr("FXMUX", "FXOR")
                .attr("F", "#LUT:0")
                .attr("XUSED", "0")
                .attr("CYSELF", "F")
                .attr("CYSELG", "1")
                .attr("COUTUSED", "0")
                .pin("F1")
                .pin("F2")
                .pin("BX")
                .pin("X")
                .pin("COUT")
                .test_enum("CY0F", &["BX", "F2", "F1", "PROD", "0", "1"]);
            bctx.mode(bk_l)
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX")
                .attr("BYINV", "BY")
                .attr("GYMUX", "GXOR")
                .attr("G", "#LUT:0")
                .attr("YUSED", "0")
                .attr("CYSELF", "1")
                .attr("CYSELG", "G")
                .attr("COUTUSED", "0")
                .pin("G1")
                .pin("G2")
                .pin("BX")
                .pin("BY")
                .pin("Y")
                .pin("COUT")
                .test_enum("CY0G", &["BY", "G2", "G1", "PROD", "0", "1"]);
        } else {
            bctx.mode(bk_l)
                .attr("BXINV", "BX_B")
                .attr("F", "#LUT:0")
                .attr("G", "#LUT:0")
                .attr("COUTUSED", "0")
                .pin("CIN")
                .pin("BX")
                .pin("COUT")
                .test_enum("CYINIT", &["CIN", "BX"]);
            bctx.mode(bk_l)
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX_B")
                .attr("FXMUX", "FXOR")
                .attr("F", "#LUT:0")
                .attr("G", "#LUT:0")
                .attr("XMUXUSED", "0")
                .attr("COUTUSED", "0")
                .pin("F3")
                .pin("F2")
                .pin("BX")
                .pin("XMUX")
                .pin("COUT")
                .test_enum("CY0F", &["0", "1", "F3", "PROD", "F2", "BX"]);
            bctx.mode(bk_l)
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX_B")
                .attr("BYINV", "BY_B")
                .attr("GYMUX", "GXOR")
                .attr("F", "#LUT:0")
                .attr("G", "#LUT:0")
                .attr("YMUXUSED", "0")
                .attr("COUTUSED", "0")
                .pin("G3")
                .pin("G2")
                .pin("BX")
                .pin("BY")
                .pin("YMUX")
                .pin("COUT")
                .test_enum("CY0G", &["0", "1", "G3", "PROD", "G2", "BY"]);
        }

        // various muxes
        if mode != Mode::Virtex4 {
            bctx.mode(bk_l)
                .attr("F", "#LUT:0")
                .attr("CYSELF", "1")
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX")
                .attr("XUSED", "0")
                .pin("X")
                .pin("BX")
                .test_enum("FXMUX", &["F", "F5", "FXOR"]);
            if mode == Mode::Virtex2 {
                bctx.mode(bk_l)
                    .attr("G", "#LUT:0")
                    .attr("CYSELF", "1")
                    .attr("CYSELG", "1")
                    .attr("CYINIT", "BX")
                    .attr("BXINV", "BX")
                    .attr("YUSED", "0")
                    .attr("SOPEXTSEL", "SOPIN")
                    .attr("SOPOUTUSED", "0")
                    .pin("Y")
                    .pin("BX")
                    .test_enum("GYMUX", &["G", "FX", "GXOR", "SOPEXT"]);
                bctx.mode(bk_l)
                    .attr("FFX", "#FF")
                    .attr("BXINV", "BX")
                    .pin("DX")
                    .pin("XQ")
                    .pin("BX")
                    .test_enum("DXMUX", &["0", "1"]);
                bctx.mode(bk_l)
                    .attr("FFY", "#FF")
                    .attr("BYINV", "BY")
                    .pin("DY")
                    .pin("YQ")
                    .pin("BY")
                    .test_enum("DYMUX", &["0", "1"]);
                bctx.mode(bk_l)
                    .attr("SOPOUTUSED", "0")
                    .pin("SOPIN")
                    .pin("SOPOUT")
                    .test_enum("SOPEXTSEL", &["SOPIN", "0"]);
            } else {
                bctx.mode(bk_l)
                    .attr("G", "#LUT:0")
                    .attr("CYSELF", "1")
                    .attr("CYSELG", "1")
                    .attr("CYINIT", "BX")
                    .attr("BXINV", "BX")
                    .attr("YUSED", "0")
                    .pin("Y")
                    .pin("BX")
                    .test_enum("GYMUX", &["G", "FX", "GXOR"]);
                bctx.mode(bk_l)
                    .attr("F", "#LUT:0")
                    .attr("XUSED", "0")
                    .attr("FXMUX", "F")
                    .attr("FFX", "#FF")
                    .attr("BXINV", "BX")
                    .pin("X")
                    .pin("XQ")
                    .pin("BX")
                    .test_enum("DXMUX", &["0", "1"]);
                bctx.mode(bk_l)
                    .attr("G", "#LUT:0")
                    .attr("YUSED", "0")
                    .attr("GYMUX", "G")
                    .attr("FFY", "#FF")
                    .attr("BYINV", "BY")
                    .pin("Y")
                    .pin("YQ")
                    .pin("BY")
                    .test_enum("DYMUX", &["0", "1"]);
            }
        } else {
            bctx.mode(bk_l)
                .attr("F", "#LUT:0")
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX_B")
                .attr("XMUXUSED", "0")
                .pin("X")
                .pin("XMUX")
                .pin("BX")
                .test_enum("FXMUX", &["F5", "FXOR"]);
            bctx.mode(bk_l)
                .attr("F", "#LUT:0")
                .attr("G", "#LUT:0")
                .attr("CYINIT", "BX")
                .attr("BXINV", "BX_B")
                .attr("BYINV", "BY_B")
                .attr("YMUXUSED", "0")
                .pin("X")
                .pin("Y")
                .pin("FXINA")
                .pin("FXINB")
                .pin("YMUX")
                .pin("BX")
                .pin("BY")
                .test_enum("GYMUX", &["FX", "GXOR"]);
            for val in ["BX", "X", "XMUX", "XB"] {
                bctx.mode(bk_l)
                    .attr("F", "#LUT:0")
                    .attr("FFX", "#FF")
                    .attr("BXINV", "BX_B")
                    .attr("FXMUX", "F5")
                    .attr("XUSED", "0")
                    .attr("XBUSED", "0")
                    .attr("XMUXUSED", "0")
                    .pin("BX")
                    .pin("X")
                    .pin("XB")
                    .pin("XMUX")
                    .pin("XQ")
                    .test_manual("DXMUX.F5", val)
                    .attr("DXMUX", val)
                    .commit();
                bctx.mode(bk_l)
                    .attr("F", "#LUT:0")
                    .attr("FFX", "#FF")
                    .attr("BXINV", "BX_B")
                    .attr("FXMUX", "FXOR")
                    .attr("XUSED", "0")
                    .attr("XBUSED", "0")
                    .attr("XMUXUSED", "0")
                    .pin("BX")
                    .pin("X")
                    .pin("XB")
                    .pin("XMUX")
                    .pin("XQ")
                    .test_manual("DXMUX.FXOR", val)
                    .attr("DXMUX", val)
                    .commit();
            }
            for val in ["BY", "Y", "YMUX", "YB"] {
                bctx.mode(bk_l)
                    .attr("G", "#LUT:0")
                    .attr("FFY", "#FF")
                    .attr("BYINV", "BY_B")
                    .attr("GYMUX", "FX")
                    .attr("YUSED", "0")
                    .attr("YBUSED", "0")
                    .attr("YMUXUSED", "0")
                    .pin("BY")
                    .pin("Y")
                    .pin("YB")
                    .pin("YMUX")
                    .pin("YQ")
                    .test_manual("DYMUX.FX", val)
                    .attr("DYMUX", val)
                    .commit();
                bctx.mode(bk_l)
                    .attr("G", "#LUT:0")
                    .attr("FFY", "#FF")
                    .attr("BYINV", "BY_B")
                    .attr("GYMUX", "GXOR")
                    .attr("YUSED", "0")
                    .attr("YBUSED", "0")
                    .attr("YMUXUSED", "0")
                    .pin("BY")
                    .pin("Y")
                    .pin("YB")
                    .pin("YMUX")
                    .pin("YQ")
                    .test_manual("DYMUX.GXOR", val)
                    .attr("DYMUX", val)
                    .commit();
            }
        }

        // LUT: memory mode
        if is_m {
            bctx.mode(bk_m)
                .attr("F", "#RAM:0")
                .attr("FXMUX", if mode == Mode::Virtex4 { "" } else { "F" })
                .attr("XUSED", "0")
                .attr("BXINV", if mode == Mode::Virtex4 { "BX_B" } else { "BX" })
                .pin("X")
                .pin("BX")
                .pin("SHIFTIN")
                .test_enum("DIF_MUX", &["ALTDIF", "BX", "SHIFTIN"]);
            bctx.mode(bk_m)
                .attr("G", "#RAM:0")
                .attr("GYMUX", if mode == Mode::Virtex4 { "" } else { "G" })
                .attr("YUSED", "0")
                .attr("BYINV", if mode == Mode::Virtex4 { "BY_B" } else { "BY" })
                .pin("Y")
                .pin("BY")
                .pin("SHIFTIN")
                .test_enum("DIG_MUX", &["ALTDIG", "BY", "SHIFTIN"]);
            bctx.mode(bk_m)
                .attr("F", "#RAM:0")
                .pin("XB")
                .test_enum("XBMUX", &["0", "1"]);
            bctx.mode(bk_m)
                .attr("G", "#RAM:0")
                .attr("YBUSED", "0")
                .pin("YB")
                .test_enum("YBMUX", &["0", "1"]);
            bctx.mode(bk_m)
                .attr("XUSED", "0")
                .attr("FXMUX", if mode == Mode::Virtex4 { "" } else { "F" })
                .attr("G", "#LUT:0")
                .attr("F_ATTR", "DUAL_PORT")
                .pin("X")
                .test_manual("F", "#RAM:0")
                .attr_diff("F", "#LUT:0", "#RAM:0")
                .commit();
            bctx.mode(bk_m)
                .attr("YUSED", "0")
                .attr("GYMUX", if mode == Mode::Virtex4 { "" } else { "G" })
                .attr("F", "#LUT:0")
                .attr("G_ATTR", "DUAL_PORT")
                .pin("Y")
                .test_manual("G", "#RAM:0")
                .attr_diff("G", "#LUT:0", "#RAM:0")
                .commit();
            bctx.mode(bk_m)
                .attr("F", "#RAM:0")
                .attr("XUSED", "0")
                .attr("FXMUX", if mode == Mode::Virtex4 { "" } else { "F" })
                .pin("X")
                .test_enum("F_ATTR", &["DUAL_PORT", "SHIFT_REG"]);
            bctx.mode(bk_m)
                .attr("G", "#RAM:0")
                .attr("YUSED", "0")
                .attr("GYMUX", if mode == Mode::Virtex4 { "" } else { "G" })
                .pin("Y")
                .test_enum("G_ATTR", &["DUAL_PORT", "SHIFT_REG"]);
            match mode {
                Mode::Virtex2 => {
                    for (pin, pinused) in [
                        ("SLICEWE0", "SLICEWE0USED"),
                        ("SLICEWE1", "SLICEWE1USED"),
                        ("SLICEWE2", "SLICEWE2USED"),
                    ] {
                        bctx.mode(bk_m)
                            .attr("F", "#RAM:0")
                            .attr("FXMUX", "F")
                            .attr("XUSED", "0")
                            .attr("BXINV", "BX")
                            .pin("X")
                            .pin("BX")
                            .pin(pin)
                            .test_enum(pinused, &["0"]);
                    }
                    bctx.mode(bk_m)
                        .attr("BXINV", "BX")
                        .pin("BX")
                        .pin("BXOUT")
                        .test_enum("BXOUTUSED", &["0"]);
                }
                Mode::Spartan3 => {
                    for pinused in ["SLICEWE0USED", "SLICEWE1USED"] {
                        bctx.mode(bk_m)
                            .attr("F", "#RAM:0")
                            .attr("FXMUX", "F")
                            .attr("XUSED", "0")
                            .attr("BXINV", "BX")
                            .pin("X")
                            .pin("BX")
                            .pin("SLICEWE1")
                            .test_enum(pinused, &["0"]);
                    }
                }
                Mode::Virtex4 => {
                    for (pinused, pinused_f, pinused_g) in [
                        ("SLICEWE0USED", "SLICEWE0USED.F", "SLICEWE0USED.G"),
                        ("SLICEWE1USED", "SLICEWE1USED.F", "SLICEWE1USED.G"),
                    ] {
                        bctx.mode(bk_m)
                            .attr("F", "#RAM:0")
                            .attr("G", "")
                            .attr("XUSED", "0")
                            .attr("BXINV", "BX_B")
                            .pin("X")
                            .pin("BX")
                            .pin("SLICEWE1")
                            .test_manual(pinused_f, "0")
                            .attr(pinused, "0")
                            .commit();
                        bctx.mode(bk_m)
                            .attr("F", "")
                            .attr("G", "#RAM:0")
                            .attr("YUSED", "0")
                            .attr("BXINV", "BX_B")
                            .pin("Y")
                            .pin("BX")
                            .pin("SLICEWE1")
                            .test_manual(pinused_g, "0")
                            .attr(pinused, "0")
                            .commit();
                    }
                }
            }
            bctx.mode(bk_m)
                .attr("BYINV", if mode == Mode::Virtex4 { "BY_B" } else { "BY" })
                .attr("BYINVOUTUSED", "")
                .pin("BY")
                .pin("BYOUT")
                .test_enum("BYOUTUSED", &["0"]);
            bctx.mode(bk_m)
                .attr("BYINV", if mode == Mode::Virtex4 { "BY_B" } else { "BY" })
                .attr("BYOUTUSED", "")
                .pin("BY")
                .pin("BYOUT")
                .test_enum("BYINVOUTUSED", &["0"]);
        }

        // FF
        bctx.mode(bk_l)
            .pin("BX")
            .pin("XQ")
            .pin("CE")
            .attr("FFY", "")
            .attr("CEINV", "CE_B")
            .attr("FFX_INIT_ATTR", "INIT1")
            .test_enum("FFX", &["#FF", "#LATCH"]);
        bctx.mode(bk_l)
            .pin("BY")
            .pin("YQ")
            .pin("CE")
            .attr("FFX", "")
            .attr("CEINV", "CE_B")
            .attr("FFY_INIT_ATTR", "INIT1")
            .test_enum("FFY", &["#FF", "#LATCH"]);
        bctx.mode(bk_l)
            .pin("XQ")
            .attr("FFX", "#FF")
            .test_enum("SYNC_ATTR", &["SYNC", "ASYNC"]);
        bctx.mode(bk_l)
            .pin("XQ")
            .attr("FFX_INIT_ATTR", "INIT1")
            .attr("FFX", "#FF")
            .test_enum("FFX_SR_ATTR", &["SRLOW", "SRHIGH"]);
        bctx.mode(bk_l)
            .pin("YQ")
            .attr("FFY_INIT_ATTR", "INIT1")
            .attr("FFY", "#FF")
            .test_enum("FFY_SR_ATTR", &["SRLOW", "SRHIGH"]);
        bctx.mode(bk_l)
            .pin("XQ")
            .attr("FFX", "#FF")
            .test_enum("FFX_INIT_ATTR", &["INIT0", "INIT1"]);
        bctx.mode(bk_l)
            .pin("YQ")
            .attr("FFY", "#FF")
            .test_enum("FFY_INIT_ATTR", &["INIT0", "INIT1"]);
        bctx.mode(bk_l)
            .attr("FFX", "#FF")
            .attr("BYINV", if mode == Mode::Virtex4 { "BY_B" } else { "BY" })
            .pin("XQ")
            .pin("BY")
            .test_enum("REVUSED", &["0"]);
    }
    if mode == Mode::Spartan3 {
        let mut ctx = FuzzCtx::new(session, backend, "RANDOR");
        let mut bctx = ctx.bel(prjcombine_virtex2::bels::RANDOR);
        bctx.mode("RESERVED_ANDOR")
            .pin("O")
            .prop(RandorInit)
            .test_enum("ANDORMUX", &["0", "1"]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let mode = match ctx.edev {
        ExpandedDevice::Virtex2(edev) => {
            if edev.chip.kind.is_virtex2() {
                Mode::Virtex2
            } else {
                Mode::Spartan3
            }
        }
        ExpandedDevice::Virtex4(_) => Mode::Virtex4,
        _ => unreachable!(),
    };

    for (idx, bel) in ["SLICE0", "SLICE1", "SLICE2", "SLICE3"]
        .into_iter()
        .enumerate()
    {
        ctx.collect_bitvec("CLB", bel, "F", "#LUT");
        ctx.collect_bitvec("CLB", bel, "G", "#LUT");

        // carry
        ctx.collect_enum("CLB", bel, "CYINIT", &["CIN", "BX"]);
        if mode != Mode::Virtex4 {
            ctx.collect_enum("CLB", bel, "CYSELF", &["F", "1"]);
            ctx.collect_enum("CLB", bel, "CYSELG", &["G", "1"]);
            ctx.collect_enum("CLB", bel, "CY0F", &["BX", "F2", "F1", "0", "1", "PROD"]);
            ctx.collect_enum("CLB", bel, "CY0G", &["BY", "G2", "G1", "0", "1", "PROD"]);
        } else {
            ctx.collect_enum("CLB", bel, "CY0F", &["1", "0", "PROD", "F2", "F3", "BX"]);
            ctx.collect_enum("CLB", bel, "CY0G", &["1", "0", "PROD", "G2", "G3", "BY"]);
        }

        // LUT RAM
        let is_m = mode == Mode::Virtex2 || matches!(idx, 0 | 2);
        if is_m {
            ctx.state
                .get_diff("CLB", bel, "F_ATTR", "DUAL_PORT")
                .assert_empty();
            ctx.state
                .get_diff("CLB", bel, "G_ATTR", "DUAL_PORT")
                .assert_empty();
            let f_ram = ctx.state.get_diff("CLB", bel, "F", "#RAM:0");
            let g_ram = ctx.state.get_diff("CLB", bel, "G", "#RAM:0");
            let (f_ram, g_ram, ram) = Diff::split(f_ram, g_ram);
            ctx.tiledb
                .insert("CLB", bel, "FF_SR_ENABLE", xlat_bit(!ram));
            let f_shift_d = ctx.state.get_diff("CLB", bel, "F_ATTR", "SHIFT_REG");
            let g_shift_d = ctx.state.get_diff("CLB", bel, "G_ATTR", "SHIFT_REG");
            let f_shift = f_ram.combine(&f_shift_d);
            let g_shift = g_ram.combine(&g_shift_d);
            ctx.tiledb.insert("CLB", bel, "F_RAM", xlat_bit(f_ram));
            ctx.tiledb.insert("CLB", bel, "G_RAM", xlat_bit(g_ram));
            ctx.tiledb.insert("CLB", bel, "F_SHIFT", xlat_bit(f_shift));
            ctx.tiledb.insert("CLB", bel, "G_SHIFT", xlat_bit(g_shift));

            let dif_bx = ctx.state.get_diff("CLB", bel, "DIF_MUX", "BX");
            let dif_alt = ctx.state.get_diff("CLB", bel, "DIF_MUX", "ALTDIF");
            assert_eq!(
                dif_alt,
                ctx.state.get_diff("CLB", bel, "DIF_MUX", "SHIFTIN")
            );
            ctx.tiledb.insert(
                "CLB",
                bel,
                "DIF_MUX",
                xlat_enum(vec![("BX", dif_bx), ("ALT", dif_alt)]),
            );

            let dig_by = ctx.state.get_diff("CLB", bel, "DIG_MUX", "BY");
            let dig_alt = ctx.state.get_diff("CLB", bel, "DIG_MUX", "ALTDIG");
            assert_eq!(
                dig_alt,
                ctx.state.get_diff("CLB", bel, "DIG_MUX", "SHIFTIN")
            );
            ctx.tiledb.insert(
                "CLB",
                bel,
                "DIG_MUX",
                xlat_enum(vec![("BY", dig_by), ("ALT", dig_alt)]),
            );

            match mode {
                Mode::Virtex2 => {
                    ctx.state
                        .get_diff("CLB", bel, "BXOUTUSED", "0")
                        .assert_empty();
                    ctx.state
                        .get_diff("CLB", bel, "SLICEWE1USED", "0")
                        .assert_empty();
                    ctx.state
                        .get_diff("CLB", bel, "SLICEWE2USED", "0")
                        .assert_empty();
                    let slicewe0used = ctx.state.get_diff("CLB", bel, "SLICEWE0USED", "0");
                    let byoutused = ctx.state.get_diff("CLB", bel, "BYOUTUSED", "0");
                    assert_eq!(
                        byoutused,
                        ctx.state.get_diff("CLB", bel, "BYINVOUTUSED", "0")
                    );
                    // TODO should these have better names?
                    ctx.tiledb
                        .insert("CLB", bel, "SLICEWE0USED", xlat_bit(slicewe0used));
                    ctx.tiledb
                        .insert("CLB", bel, "BYOUTUSED", xlat_bit(byoutused));
                }
                Mode::Spartan3 => {
                    ctx.state
                        .get_diff("CLB", bel, "BYOUTUSED", "0")
                        .assert_empty();
                    ctx.state
                        .get_diff("CLB", bel, "BYINVOUTUSED", "0")
                        .assert_empty();
                    let slicewe0used = ctx.state.get_diff("CLB", bel, "SLICEWE0USED", "0");
                    let slicewe1used = ctx.state.get_diff("CLB", bel, "SLICEWE1USED", "0");
                    ctx.tiledb
                        .insert("CLB", bel, "SLICEWE0USED", xlat_bit(slicewe0used));
                    if idx == 0 {
                        ctx.tiledb
                            .insert("CLB", bel, "SLICEWE1USED", xlat_bit(slicewe1used));
                    } else {
                        slicewe1used.assert_empty();
                    }
                }
                Mode::Virtex4 => {
                    ctx.state
                        .get_diff("CLB", bel, "BYOUTUSED", "0")
                        .assert_empty();
                    ctx.state
                        .get_diff("CLB", bel, "BYINVOUTUSED", "0")
                        .assert_empty();
                    let f_slicewe0used = ctx.state.get_diff("CLB", bel, "SLICEWE0USED.F", "0");
                    let f_slicewe1used = ctx.state.get_diff("CLB", bel, "SLICEWE1USED.F", "0");
                    let g_slicewe0used = ctx.state.get_diff("CLB", bel, "SLICEWE0USED.G", "0");
                    let g_slicewe1used = ctx.state.get_diff("CLB", bel, "SLICEWE1USED.G", "0");
                    ctx.tiledb
                        .insert("CLB", bel, "F_SLICEWE0USED", xlat_bit(f_slicewe0used));
                    ctx.tiledb
                        .insert("CLB", bel, "F_SLICEWE1USED", xlat_bit(f_slicewe1used));
                    ctx.tiledb
                        .insert("CLB", bel, "G_SLICEWE0USED", xlat_bit(g_slicewe0used));
                    ctx.tiledb
                        .insert("CLB", bel, "G_SLICEWE1USED", xlat_bit(g_slicewe1used));
                }
            }
        }

        // muxes
        match mode {
            Mode::Virtex2 => {
                ctx.collect_enum("CLB", bel, "FXMUX", &["F", "F5", "FXOR"]);
                let gymux_g = ctx.state.get_diff("CLB", bel, "GYMUX", "G");
                let gymux_fx = ctx.state.get_diff("CLB", bel, "GYMUX", "FX");
                let gymux_gxor = ctx.state.get_diff("CLB", bel, "GYMUX", "GXOR");
                let gymux_sopout = ctx.state.get_diff("CLB", bel, "GYMUX", "SOPEXT");
                ctx.tiledb.insert(
                    "CLB",
                    bel,
                    "GYMUX",
                    xlat_enum(vec![
                        ("G", gymux_g),
                        ("FX", gymux_fx),
                        ("SOPOUT", gymux_sopout),
                        ("GXOR", gymux_gxor),
                    ]),
                );
                ctx.collect_enum("CLB", bel, "SOPEXTSEL", &["SOPIN", "0"]);
            }
            Mode::Spartan3 => {
                ctx.collect_enum("CLB", bel, "FXMUX", &["F", "F5", "FXOR"]);
                ctx.collect_enum("CLB", bel, "GYMUX", &["G", "FX", "GXOR"]);
            }
            Mode::Virtex4 => {
                ctx.collect_enum("CLB", bel, "FXMUX", &["F5", "FXOR"]);
                ctx.collect_enum("CLB", bel, "GYMUX", &["FX", "GXOR"]);
            }
        }
        if mode != Mode::Virtex4 {
            let dx_bx = ctx.state.get_diff("CLB", bel, "DXMUX", "0");
            let dx_x = ctx.state.get_diff("CLB", bel, "DXMUX", "1");
            ctx.tiledb.insert(
                "CLB",
                bel,
                "DXMUX",
                xlat_enum(vec![("BX", dx_bx), ("X", dx_x)]),
            );
            let dy_by = ctx.state.get_diff("CLB", bel, "DYMUX", "0");
            let dy_y = ctx.state.get_diff("CLB", bel, "DYMUX", "1");
            ctx.tiledb.insert(
                "CLB",
                bel,
                "DYMUX",
                xlat_enum(vec![("BY", dy_by), ("Y", dy_y)]),
            );
        } else {
            let dxmux_bx = ctx.state.get_diff("CLB", bel, "DXMUX.F5", "BX");
            let dxmux_x = ctx.state.get_diff("CLB", bel, "DXMUX.F5", "X");
            let dxmux_xb = ctx.state.get_diff("CLB", bel, "DXMUX.F5", "XB");
            let dxmux_f5 = ctx.state.get_diff("CLB", bel, "DXMUX.F5", "XMUX");
            assert_eq!(dxmux_bx, ctx.state.get_diff("CLB", bel, "DXMUX.FXOR", "BX"));
            assert_eq!(dxmux_x, ctx.state.get_diff("CLB", bel, "DXMUX.FXOR", "X"));
            assert_eq!(dxmux_xb, ctx.state.get_diff("CLB", bel, "DXMUX.FXOR", "XB"));
            let dxmux_fxor = ctx.state.get_diff("CLB", bel, "DXMUX.FXOR", "XMUX");
            ctx.tiledb.insert(
                "CLB",
                bel,
                "DXMUX",
                xlat_enum(vec![
                    ("X", dxmux_x),
                    ("F5", dxmux_f5),
                    ("XB", dxmux_xb),
                    ("FXOR", dxmux_fxor),
                    ("BX", dxmux_bx),
                ]),
            );

            let dymux_by = ctx.state.get_diff("CLB", bel, "DYMUX.FX", "BY");
            let dymux_y = ctx.state.get_diff("CLB", bel, "DYMUX.FX", "Y");
            let dymux_yb = ctx.state.get_diff("CLB", bel, "DYMUX.FX", "YB");
            let dymux_fx = ctx.state.get_diff("CLB", bel, "DYMUX.FX", "YMUX");
            assert_eq!(dymux_by, ctx.state.get_diff("CLB", bel, "DYMUX.GXOR", "BY"));
            assert_eq!(dymux_y, ctx.state.get_diff("CLB", bel, "DYMUX.GXOR", "Y"));
            assert_eq!(dymux_yb, ctx.state.get_diff("CLB", bel, "DYMUX.GXOR", "YB"));
            let dymux_gxor = ctx.state.get_diff("CLB", bel, "DYMUX.GXOR", "YMUX");
            ctx.tiledb.insert(
                "CLB",
                bel,
                "DYMUX",
                xlat_enum(vec![
                    ("Y", dymux_y),
                    ("FX", dymux_fx),
                    ("YB", dymux_yb),
                    ("GXOR", dymux_gxor),
                    ("BY", dymux_by),
                ]),
            );
        }
        if is_m {
            let xbmux_shiftout = ctx.state.get_diff("CLB", bel, "XBMUX", "0");
            let xbmux_cout = ctx.state.get_diff("CLB", bel, "XBMUX", "1");
            ctx.tiledb.insert(
                "CLB",
                bel,
                "XBMUX",
                xlat_enum(vec![("FMC15", xbmux_shiftout), ("FCY", xbmux_cout)]),
            );

            let ybmux_shiftout = ctx.state.get_diff("CLB", bel, "YBMUX", "0");
            let ybmux_cout = ctx.state.get_diff("CLB", bel, "YBMUX", "1");
            ctx.tiledb.insert(
                "CLB",
                bel,
                "YBMUX",
                xlat_enum(vec![("GMC15", ybmux_shiftout), ("GCY", ybmux_cout)]),
            );
        }

        // FFs
        let item = ctx.extract_enum_bool("CLB", bel, "SYNC_ATTR", "ASYNC", "SYNC");
        ctx.tiledb.insert("CLB", bel, "FF_SR_SYNC", item);

        let ff_latch = ctx.state.get_diff("CLB", bel, "FFX", "#LATCH");
        assert_eq!(ff_latch, ctx.state.get_diff("CLB", bel, "FFY", "#LATCH"));
        ctx.state.get_diff("CLB", bel, "FFX", "#FF").assert_empty();
        ctx.state.get_diff("CLB", bel, "FFY", "#FF").assert_empty();
        ctx.tiledb
            .insert("CLB", bel, "FF_LATCH", xlat_bit(ff_latch));

        let item = ctx.extract_bit("CLB", bel, "REVUSED", "0");
        ctx.tiledb.insert("CLB", bel, "FF_REV_ENABLE", item);

        let item = ctx.extract_enum_bool("CLB", bel, "FFX_SR_ATTR", "SRLOW", "SRHIGH");
        ctx.tiledb.insert("CLB", bel, "FFX_SRVAL", item);
        let item = ctx.extract_enum_bool("CLB", bel, "FFY_SR_ATTR", "SRLOW", "SRHIGH");
        ctx.tiledb.insert("CLB", bel, "FFY_SRVAL", item);

        let item = ctx.extract_enum_bool("CLB", bel, "FFX_INIT_ATTR", "INIT0", "INIT1");
        ctx.tiledb.insert("CLB", bel, "FFX_INIT", item);
        let item = ctx.extract_enum_bool("CLB", bel, "FFY_INIT_ATTR", "INIT0", "INIT1");
        ctx.tiledb.insert("CLB", bel, "FFY_INIT", item);

        // inverts
        let int = if mode == Mode::Virtex4 {
            "INT"
        } else {
            "INT.CLB"
        };
        ctx.collect_int_inv(&[int], "CLB", bel, "CLK", false);
        ctx.collect_int_inv(&[int], "CLB", bel, "SR", mode == Mode::Virtex2);
        ctx.collect_int_inv(&[int], "CLB", bel, "CE", mode == Mode::Virtex2);
        if mode == Mode::Virtex2 {
            ctx.collect_int_inv(&[int], "CLB", bel, "BX", false);
            ctx.collect_int_inv(&[int], "CLB", bel, "BY", false);
        } else {
            ctx.collect_inv("CLB", bel, "BX");
            ctx.collect_inv("CLB", bel, "BY");
        }
    }
    if mode == Mode::Spartan3 {
        let ExpandedDevice::Virtex2(edev) = ctx.edev else {
            unreachable!()
        };
        let tile = "RANDOR";
        let bel = "RANDOR";
        if edev.chip.kind.is_spartan3a() {
            ctx.state
                .get_diff(tile, bel, "ANDORMUX", "0")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "ANDORMUX", "1")
                .assert_empty();
        } else {
            let item = xlat_enum(vec![
                ("OR", ctx.state.get_diff(tile, bel, "ANDORMUX", "0")),
                ("AND", ctx.state.get_diff(tile, bel, "ANDORMUX", "1")),
            ]);
            ctx.tiledb.insert(tile, bel, "MODE", item);
        }
        let tile = "RANDOR_INIT";
        let bel = "RANDOR_INIT";
        let item = xlat_enum(vec![
            ("OR", ctx.state.get_diff(tile, bel, "ANDORMUX", "0")),
            ("AND", ctx.state.get_diff(tile, bel, "ANDORMUX", "1")),
        ]);
        ctx.tiledb.insert(tile, bel, "MODE", item);
    }
    for (tcid, name, _) in &ctx.edev.db.tile_classes {
        if !name.starts_with("INT.") {
            continue;
        }
        if name == "INT.CLB" {
            continue;
        }
        if ctx.edev.tile_index[tcid].is_empty() {
            continue;
        }
        for &wire in ctx.edev.db_index[tcid].pips_bwd.keys() {
            let wire_name = ctx.edev.db.wires.key(wire.wire);
            if name == "INT.GT.CLKPAD"
                && matches!(
                    &wire_name[..],
                    "IMUX.CE0" | "IMUX.CE1" | "IMUX.TS0" | "IMUX.TS1"
                )
            {
                continue;
            }
            if name == "INT.BRAM.S3A.03"
                && (wire_name.starts_with("IMUX.CLK") || wire_name.starts_with("IMUX.CE"))
            {
                continue;
            }
            let inv_name = format!("INT:INV.{wire_name}");
            let mux_name = format!("INT:MUX.{wire_name}");
            if !ctx.tiledb.tiles.contains_key(name) {
                continue;
            }
            if !ctx.tiledb.tiles[name].items.contains_key(&mux_name) {
                continue;
            }
            let int_clb = &ctx.tiledb.tiles["INT.CLB"];
            let Some(item) = int_clb.items.get(&inv_name) else {
                continue;
            };
            let item = item.clone();
            ctx.tiledb
                .insert(name, "INT", format!("INV.{wire_name}"), item);
        }
    }
}
