use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, SimpleFeatureId},
    diff::{xlat_bitvec, xlat_enum, CollectorCtx, Diff},
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileRelation, TileWire},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_one,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Virtex2,
    Spartan3,
    Virtex4,
}

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let node_kind = backend.egrid.db.get_node("CLB");
    let mode = match backend.edev {
        ExpandedDevice::Virtex2(ref edev) => {
            if edev.grid.kind.is_virtex2() {
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
    for i in 0..4 {
        let bel = BelId::from_idx(i);
        let bel_name = backend.egrid.db.nodes[node_kind].bels.key(bel);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Main(1),
            tile_name: "CLB",
            bel,
            bel_name,
        };
        let is_m = match mode {
            Mode::Virtex2 => true,
            Mode::Spartan3 | Mode::Virtex4 => matches!(i, 0 | 2),
        };

        // inverters
        fuzz_enum!(ctx, "CEINV", ["CE", "CE_B"], [
            (mode bk_l),
            (attr "FFX", "#FF"),
            (pin "CE"),
            (pin "XQ")
        ]);
        fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [
            (mode bk_l),
            (attr "FFX", "#FF"),
            (pin "CLK"),
            (pin "XQ")
        ]);
        fuzz_enum!(ctx, "SRINV", ["SR", "SR_B"], [
            (mode bk_l),
            (attr "FFX", "#FF"),
            (attr "FFY", "#FF"),
            (attr "SRFFMUX", if mode == Mode::Virtex2 {"0"} else {""}),
            (pin "SR"),
            (pin "XQ"),
            (pin "YQ")
        ]);
        fuzz_enum!(ctx, "BXINV", ["BX", "BX_B"], [
            (mode bk_l),
            (attr "FFX", "#FF"),
            (attr "XUSED", "0"),
            (attr "DXMUX", if mode == Mode::Virtex4 {"BX"} else {"0"}),
            (pin "X"),
            (pin "BX"),
            (pin "XQ")
        ]);
        fuzz_enum!(ctx, "BYINV", ["BY", "BY_B"], [
            (mode bk_l),
            (attr "FFY", "#FF"),
            (attr "YUSED", "0"),
            (attr "DYMUX", if mode == Mode::Virtex4 {"BY"} else {"0"}),
            (pin "Y"),
            (pin "BY"),
            (pin "YQ")
        ]);

        // LUT
        for attr in ["F", "G"] {
            fuzz_multi!(ctx, attr, "#LUT", 16, [(mode bk_l)], (attr_lut attr));
        }

        // carry chain
        if mode != Mode::Virtex4 {
            fuzz_enum!(ctx, "CYINIT", ["CIN", "BX"], [
                (mode bk_l),
                (attr "BXINV", "BX"),
                (attr "CYSELF", "1"),
                (attr "CYSELG", "1"),
                (attr "COUTUSED", "0"),
                (pin "CIN"),
                (pin "BX"),
                (pin "COUT")
            ]);
            fuzz_enum!(ctx, "CYSELF", ["F", "1"], [
                (mode bk_l),
                (attr "F", "#LUT:0"),
                (attr "CY0F", "0"),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX"),
                (attr "CYSELG", "1"),
                (attr "COUTUSED", "0"),
                (pin "BX"),
                (pin "COUT")
            ]);
            fuzz_enum!(ctx, "CYSELG", ["G", "1"], [
                (mode bk_l),
                (attr "G", "#LUT:0"),
                (attr "CY0G", "0"),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX"),
                (attr "CYSELF", "1"),
                (attr "COUTUSED", "0"),
                (pin "BX"),
                (pin "COUT")
            ]);
            fuzz_enum!(ctx, "CY0F", ["BX", "F2", "F1", "PROD", "0", "1"], [
                (mode bk_l),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX"),
                (attr "FXMUX", "FXOR"),
                (attr "F", "#LUT:0"),
                (attr "XUSED", "0"),
                (attr "CYSELF", "F"),
                (attr "CYSELG", "1"),
                (attr "COUTUSED", "0"),
                (pin "F1"),
                (pin "F2"),
                (pin "BX"),
                (pin "X"),
                (pin "COUT")
            ]);
            fuzz_enum!(ctx, "CY0G", ["BY", "G2", "G1", "PROD", "0", "1"], [
                (mode bk_l),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX"),
                (attr "BYINV", "BY"),
                (attr "GYMUX", "GXOR"),
                (attr "G", "#LUT:0"),
                (attr "YUSED", "0"),
                (attr "CYSELF", "1"),
                (attr "CYSELG", "G"),
                (attr "COUTUSED", "0"),
                (pin "G1"),
                (pin "G2"),
                (pin "BX"),
                (pin "BY"),
                (pin "Y"),
                (pin "COUT")
            ]);
        } else {
            fuzz_enum!(ctx, "CYINIT", ["CIN", "BX"], [
                (mode bk_l),
                (attr "BXINV", "BX_B"),
                (attr "F", "#LUT:0"),
                (attr "G", "#LUT:0"),
                (attr "COUTUSED", "0"),
                (pin "CIN"),
                (pin "BX"),
                (pin "COUT")
            ]);
            fuzz_enum!(ctx, "CY0F", ["0", "1", "F3", "PROD", "F2", "BX"], [
                (mode bk_l),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX_B"),
                (attr "FXMUX", "FXOR"),
                (attr "F", "#LUT:0"),
                (attr "G", "#LUT:0"),
                (attr "XMUXUSED", "0"),
                (attr "COUTUSED", "0"),
                (pin "F3"),
                (pin "F2"),
                (pin "BX"),
                (pin "XMUX"),
                (pin "COUT")
            ]);
            fuzz_enum!(ctx, "CY0G", ["0", "1", "G3", "PROD", "G2", "BY"], [
                (mode bk_l),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX_B"),
                (attr "BYINV", "BY_B"),
                (attr "GYMUX", "GXOR"),
                (attr "F", "#LUT:0"),
                (attr "G", "#LUT:0"),
                (attr "YMUXUSED", "0"),
                (attr "COUTUSED", "0"),
                (pin "G3"),
                (pin "G2"),
                (pin "BX"),
                (pin "BY"),
                (pin "YMUX"),
                (pin "COUT")
            ]);
        }

        // various muxes
        if mode != Mode::Virtex4 {
            fuzz_enum!(ctx, "FXMUX", ["F", "F5", "FXOR"], [
                (mode bk_l),
                (attr "F", "#LUT:0"),
                (attr "CYSELF", "1"),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX"),
                (attr "XUSED", "0"),
                (pin "X"),
                (pin "BX")
            ]);
            if mode == Mode::Virtex2 {
                fuzz_enum!(ctx, "GYMUX", ["G", "FX", "GXOR", "SOPEXT"], [
                    (mode bk_l),
                    (attr "G", "#LUT:0"),
                    (attr "CYSELF", "1"),
                    (attr "CYSELG", "1"),
                    (attr "CYINIT", "BX"),
                    (attr "BXINV", "BX"),
                    (attr "YUSED", "0"),
                    (attr "SOPEXTSEL", "SOPIN"),
                    (attr "SOPOUTUSED", "0"),
                    (pin "Y"),
                    (pin "BX")
                ]);
                fuzz_enum!(ctx, "DXMUX", ["0", "1"], [
                    (mode bk_l),
                    (attr "FFX", "#FF"),
                    (attr "BXINV", "BX"),
                    (pin "DX"),
                    (pin "XQ"),
                    (pin "BX")
                ]);
                fuzz_enum!(ctx, "DYMUX", ["0", "1"], [
                    (mode bk_l),
                    (attr "FFY", "#FF"),
                    (attr "BYINV", "BY"),
                    (pin "DY"),
                    (pin "YQ"),
                    (pin "BY")
                ]);
                fuzz_enum!(ctx, "SOPEXTSEL", ["SOPIN", "0"], [
                    (mode bk_l),
                    (attr "SOPOUTUSED", "0"),
                    (pin "SOPIN"),
                    (pin "SOPOUT")
                ]);
            } else {
                fuzz_enum!(ctx, "GYMUX", ["G", "FX", "GXOR"], [
                    (mode bk_l),
                    (attr "G", "#LUT:0"),
                    (attr "CYSELF", "1"),
                    (attr "CYSELG", "1"),
                    (attr "CYINIT", "BX"),
                    (attr "BXINV", "BX"),
                    (attr "YUSED", "0"),
                    (pin "Y"),
                    (pin "BX")
                ]);
                fuzz_enum!(ctx, "DXMUX", ["0", "1"], [
                    (mode bk_l),
                    (attr "F", "#LUT:0"),
                    (attr "XUSED", "0"),
                    (attr "FXMUX", "F"),
                    (attr "FFX", "#FF"),
                    (attr "BXINV", "BX"),
                    (pin "X"),
                    (pin "XQ"),
                    (pin "BX")
                ]);
                fuzz_enum!(ctx, "DYMUX", ["0", "1"], [
                    (mode bk_l),
                    (attr "G", "#LUT:0"),
                    (attr "YUSED", "0"),
                    (attr "GYMUX", "G"),
                    (attr "FFY", "#FF"),
                    (attr "BYINV", "BY"),
                    (pin "Y"),
                    (pin "YQ"),
                    (pin "BY")
                ]);
            }
        } else {
            fuzz_enum!(ctx, "FXMUX", ["F5", "FXOR"], [
                (mode bk_l),
                (attr "F", "#LUT:0"),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX_B"),
                (attr "XMUXUSED", "0"),
                (pin "X"),
                (pin "XMUX"),
                (pin "BX")
            ]);
            fuzz_enum!(ctx, "GYMUX", ["FX", "GXOR"], [
                (mode bk_l),
                (attr "F", "#LUT:0"),
                (attr "G", "#LUT:0"),
                (attr "CYINIT", "BX"),
                (attr "BXINV", "BX_B"),
                (attr "BYINV", "BY_B"),
                (attr "YMUXUSED", "0"),
                (pin "X"),
                (pin "Y"),
                (pin "FXINA"),
                (pin "FXINB"),
                (pin "YMUX"),
                (pin "BX"),
                (pin "BY")
            ]);
            for val in ["BX", "X", "XMUX", "XB"] {
                fuzz_one!(ctx, "DXMUX.F5", val, [
                    (mode bk_l),
                    (attr "F", "#LUT:0"),
                    (attr "FFX", "#FF"),
                    (attr "BXINV", "BX_B"),
                    (attr "FXMUX", "F5"),
                    (attr "XUSED", "0"),
                    (attr "XBUSED", "0"),
                    (attr "XMUXUSED", "0"),
                    (pin "BX"),
                    (pin "X"),
                    (pin "XB"),
                    (pin "XMUX"),
                    (pin "XQ")
                ], [
                    (attr "DXMUX", val)
                ]);
                fuzz_one!(ctx, "DXMUX.FXOR", val, [
                    (mode bk_l),
                    (attr "F", "#LUT:0"),
                    (attr "FFX", "#FF"),
                    (attr "BXINV", "BX_B"),
                    (attr "FXMUX", "FXOR"),
                    (attr "XUSED", "0"),
                    (attr "XBUSED", "0"),
                    (attr "XMUXUSED", "0"),
                    (pin "BX"),
                    (pin "X"),
                    (pin "XB"),
                    (pin "XMUX"),
                    (pin "XQ")
                ], [
                    (attr "DXMUX", val)
                ]);
            }
            for val in ["BY", "Y", "YMUX", "YB"] {
                fuzz_one!(ctx, "DYMUX.FX", val, [
                    (mode bk_l),
                    (attr "G", "#LUT:0"),
                    (attr "FFY", "#FF"),
                    (attr "BYINV", "BY_B"),
                    (attr "GYMUX", "FX"),
                    (attr "YUSED", "0"),
                    (attr "YBUSED", "0"),
                    (attr "YMUXUSED", "0"),
                    (pin "BY"),
                    (pin "Y"),
                    (pin "YB"),
                    (pin "YMUX"),
                    (pin "YQ")
                ], [
                    (attr "DYMUX", val)
                ]);
                fuzz_one!(ctx, "DYMUX.GXOR", val, [
                    (mode bk_l),
                    (attr "G", "#LUT:0"),
                    (attr "FFY", "#FF"),
                    (attr "BYINV", "BY_B"),
                    (attr "GYMUX", "GXOR"),
                    (attr "YUSED", "0"),
                    (attr "YBUSED", "0"),
                    (attr "YMUXUSED", "0"),
                    (pin "BY"),
                    (pin "Y"),
                    (pin "YB"),
                    (pin "YMUX"),
                    (pin "YQ")
                ], [
                    (attr "DYMUX", val)
                ]);
            }
        }

        // LUT: memory mode
        if is_m {
            fuzz_enum!(ctx, "DIF_MUX", ["ALTDIF", "BX", "SHIFTIN"], [
                (mode bk_m),
                (attr "F", "#RAM:0"),
                (attr "FXMUX", if mode == Mode::Virtex4 {""} else {"F"}),
                (attr "XUSED", "0"),
                (attr "BXINV", if mode == Mode::Virtex4 {"BX_B"} else {"BX"}),
                (pin "X"),
                (pin "BX"),
                (pin "SHIFTIN")
            ]);
            fuzz_enum!(ctx, "DIG_MUX", ["ALTDIG", "BY", "SHIFTIN"], [
                (mode bk_m),
                (attr "G", "#RAM:0"),
                (attr "GYMUX", if mode == Mode::Virtex4 {""} else {"G"}),
                (attr "YUSED", "0"),
                (attr "BYINV", if mode == Mode::Virtex4 {"BY_B"} else {"BY"}),
                (pin "Y"),
                (pin "BY"),
                (pin "SHIFTIN")
            ]);
            fuzz_enum!(ctx, "XBMUX", ["0", "1"], [
                (mode bk_m),
                (attr "F", "#RAM:0"),
                (pin "XB")
            ]);
            fuzz_enum!(ctx, "YBMUX", ["0", "1"], [
                (mode bk_m),
                (attr "G", "#RAM:0"),
                (attr "YBUSED", "0"),
                (pin "YB")
            ]);
            fuzz_enum!(ctx, "F", ["#LUT:0", "#RAM:0"], [
                (mode bk_m),
                (attr "XUSED", "0"),
                (attr "FXMUX", if mode == Mode::Virtex4 {""} else {"F"}),
                (attr "G", "#LUT:0"),
                (attr "F_ATTR", "DUAL_PORT"),
                (pin "X")
            ]);
            fuzz_enum!(ctx, "G", ["#LUT:0", "#RAM:0"], [
                (mode bk_m),
                (attr "YUSED", "0"),
                (attr "GYMUX", if mode == Mode::Virtex4 {""} else {"G"}),
                (attr "F", "#LUT:0"),
                (attr "G_ATTR", "DUAL_PORT"),
                (pin "Y")
            ]);
            fuzz_enum!(ctx, "F_ATTR", ["DUAL_PORT", "SHIFT_REG"], [
                (mode bk_m),
                (attr "F", "#RAM:0"),
                (attr "XUSED", "0"),
                (attr "FXMUX", if mode == Mode::Virtex4 {""} else {"F"}),
                (pin "X")
            ]);
            fuzz_enum!(ctx, "G_ATTR", ["DUAL_PORT", "SHIFT_REG"], [
                (mode bk_m),
                (attr "G", "#RAM:0"),
                (attr "YUSED", "0"),
                (attr "GYMUX", if mode == Mode::Virtex4 {""} else {"G"}),
                (pin "Y")
            ]);
            match mode {
                Mode::Virtex2 => {
                    for (pin, pinused) in [
                        ("SLICEWE0", "SLICEWE0USED"),
                        ("SLICEWE1", "SLICEWE1USED"),
                        ("SLICEWE2", "SLICEWE2USED"),
                    ] {
                        fuzz_enum!(ctx, pinused, ["0"], [
                            (mode bk_m),
                            (attr "F", "#RAM:0"),
                            (attr "FXMUX", "F"),
                            (attr "XUSED", "0"),
                            (attr "BXINV", "BX"),
                            (pin "X"),
                            (pin "BX"),
                            (pin pin)
                        ]);
                    }
                    fuzz_enum!(ctx, "BXOUTUSED", ["0"], [
                        (mode bk_m),
                        (attr "BXINV", "BX"),
                        (pin "BX"),
                        (pin "BXOUT")
                    ]);
                }
                Mode::Spartan3 => {
                    for pinused in ["SLICEWE0USED", "SLICEWE1USED"] {
                        fuzz_enum!(ctx, pinused, ["0"], [
                            (mode bk_m),
                            (attr "F", "#RAM:0"),
                            (attr "FXMUX", "F"),
                            (attr "XUSED", "0"),
                            (attr "BXINV", "BX"),
                            (pin "X"),
                            (pin "BX"),
                            (pin "SLICEWE1")
                        ]);
                    }
                }
                Mode::Virtex4 => {
                    for (pinused, pinused_f, pinused_g) in [
                        ("SLICEWE0USED", "SLICEWE0USED.F", "SLICEWE0USED.G"),
                        ("SLICEWE1USED", "SLICEWE1USED.F", "SLICEWE1USED.G"),
                    ] {
                        fuzz_one!(ctx, pinused_f, "0", [
                            (mode bk_m),
                            (attr "F", "#RAM:0"),
                            (attr "G", ""),
                            (attr "XUSED", "0"),
                            (attr "BXINV", "BX_B"),
                            (pin "X"),
                            (pin "BX"),
                            (pin "SLICEWE1")
                        ], [
                            (attr pinused, "0")
                        ]);
                        fuzz_one!(ctx, pinused_g, "0", [
                            (mode bk_m),
                            (attr "F", ""),
                            (attr "G", "#RAM:0"),
                            (attr "YUSED", "0"),
                            (attr "BXINV", "BX_B"),
                            (pin "Y"),
                            (pin "BX"),
                            (pin "SLICEWE1")
                        ], [
                            (attr pinused, "0")
                        ]);
                    }
                }
            }
            fuzz_enum!(ctx, "BYOUTUSED", ["0"], [
                (mode bk_m),
                (attr "BYINV", if mode == Mode::Virtex4 {"BY_B"} else {"BY"}),
                (attr "BYINVOUTUSED", ""),
                (pin "BY"),
                (pin "BYOUT")
            ]);
            fuzz_enum!(ctx, "BYINVOUTUSED", ["0"], [
                (mode bk_m),
                (attr "BYINV", if mode == Mode::Virtex4 {"BY_B"} else {"BY"}),
                (attr "BYOUTUSED", ""),
                (pin "BY"),
                (pin "BYOUT")
            ]);
        }

        // FF
        fuzz_enum!(ctx, "FFX", ["#FF", "#LATCH"], [
            (mode bk_l),
            (pin "BX"),
            (pin "XQ"),
            (pin "CE"),
            (attr "FFY", ""),
            (attr "CEINV", "CE_B"),
            (attr "FFX_INIT_ATTR", "INIT1")
        ]);
        fuzz_enum!(ctx, "FFY", ["#FF", "#LATCH"], [
            (mode bk_l),
            (pin "BY"),
            (pin "YQ"),
            (pin "CE"),
            (attr "FFX", ""),
            (attr "CEINV", "CE_B"),
            (attr "FFY_INIT_ATTR", "INIT1")
        ]);
        fuzz_enum!(ctx, "SYNC_ATTR", ["SYNC", "ASYNC"], [
            (mode bk_l),
            (pin "XQ"),
            (attr "FFX", "#FF")
        ]);
        fuzz_enum!(ctx, "FFX_SR_ATTR", ["SRLOW", "SRHIGH"], [
            (mode bk_l),
            (pin "XQ"),
            (attr "FFX_INIT_ATTR", "INIT1"),
            (attr "FFX", "#FF")
        ]);
        fuzz_enum!(ctx, "FFY_SR_ATTR", ["SRLOW", "SRHIGH"], [
            (mode bk_l),
            (pin "YQ"),
            (attr "FFY_INIT_ATTR", "INIT1"),
            (attr "FFY", "#FF")
        ]);
        fuzz_enum!(ctx, "FFX_INIT_ATTR", ["INIT0", "INIT1"], [
            (mode bk_l),
            (pin "XQ"),
            (attr "FFX", "#FF")
        ]);
        fuzz_enum!(ctx, "FFY_INIT_ATTR", ["INIT0", "INIT1"], [
            (mode bk_l),
            (pin "YQ"),
            (attr "FFY", "#FF")
        ]);
        fuzz_enum!(ctx, "REVUSED", ["0"], [
            (mode bk_l),
            (attr "FFX", "#FF"),
            (attr "BYINV", if mode == Mode::Virtex4 {"BY_B"} else {"BY"}),
            (pin "XQ"),
            (pin "BY")
        ]);
    }
    if mode == Mode::Virtex2 {
        let tbus_bel = BelId::from_idx(6);
        for (i, out_a, out_b) in [(0, "BUS0", "BUS2"), (1, "BUS1", "BUS3")] {
            let bel = BelId::from_idx(4 + i);
            let bel_name = backend.egrid.db.nodes[node_kind].bels.key(bel);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Main(1),
                tile_name: "CLB",
                bel,
                bel_name,
            };
            fuzz_enum!(ctx, "TINV", ["T", "T_B"], [
                (mode "TBUF"),
                (pin "T"),
                (pin "O")
            ]);
            fuzz_enum!(ctx, "IINV", ["I", "I_B"], [
                (mode "TBUF"),
                (pin "I"),
                (pin "O")
            ]);
            fuzz_one!(ctx, "OUT_A", "1", [(row_mutex_site "TBUF")], [(pip (pin "O"), (bel_pin tbus_bel, out_a))]);
            fuzz_one!(ctx, "OUT_B", "1", [(row_mutex_site "TBUF")], [(pip (pin "O"), (bel_pin tbus_bel, out_b))]);
        }
        let bel = BelId::from_idx(6);
        session.add_fuzzer(Box::new(TileFuzzerGen {
            node: node_kind,
            bits: TileBits::Main(1),
            feature: SimpleFeatureId {
                tile: "CLB",
                bel: "TBUS",
                attr: "JOINER",
                val: "1",
            },
            base: vec![],
            fuzz: vec![TileFuzzKV::TileRelated(
                TileRelation::ClbTbusRight,
                Box::new(TileFuzzKV::Pip(
                    TileWire::BelPinNear(bel, "BUS3"),
                    TileWire::BelPinNear(bel, "BUS3_E"),
                )),
            )],
        }));
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let mode = match ctx.edev {
        ExpandedDevice::Virtex2(ref edev) => {
            if edev.grid.kind.is_virtex2() {
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
            ctx.state.get_diff("CLB", bel, "F", "#LUT:0").assert_empty();
            ctx.state.get_diff("CLB", bel, "G", "#LUT:0").assert_empty();
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
                .insert("CLB", bel, "FF_SR_EN", xlat_bitvec(vec![!ram]));
            let f_shift_d = ctx.state.get_diff("CLB", bel, "F_ATTR", "SHIFT_REG");
            let g_shift_d = ctx.state.get_diff("CLB", bel, "G_ATTR", "SHIFT_REG");
            let f_shift = f_ram.combine(&f_shift_d);
            let g_shift = g_ram.combine(&g_shift_d);
            ctx.tiledb
                .insert("CLB", bel, "F_RAM", xlat_bitvec(vec![f_ram]));
            ctx.tiledb
                .insert("CLB", bel, "G_RAM", xlat_bitvec(vec![g_ram]));
            ctx.tiledb
                .insert("CLB", bel, "F_SHIFT", xlat_bitvec(vec![f_shift]));
            ctx.tiledb
                .insert("CLB", bel, "G_SHIFT", xlat_bitvec(vec![g_shift]));

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
                xlat_enum(vec![
                    ("BX".to_string(), dif_bx),
                    ("ALT".to_string(), dif_alt),
                ]),
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
                xlat_enum(vec![
                    ("BY".to_string(), dig_by),
                    ("ALT".to_string(), dig_alt),
                ]),
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
                        .insert("CLB", bel, "SLICEWE0USED", xlat_bitvec(vec![slicewe0used]));
                    ctx.tiledb
                        .insert("CLB", bel, "BYOUTUSED", xlat_bitvec(vec![byoutused]));
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
                        .insert("CLB", bel, "SLICEWE0USED", xlat_bitvec(vec![slicewe0used]));
                    if idx == 0 {
                        ctx.tiledb.insert(
                            "CLB",
                            bel,
                            "SLICEWE1USED",
                            xlat_bitvec(vec![slicewe1used]),
                        );
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
                    ctx.tiledb.insert(
                        "CLB",
                        bel,
                        "F_SLICEWE0USED",
                        xlat_bitvec(vec![f_slicewe0used]),
                    );
                    ctx.tiledb.insert(
                        "CLB",
                        bel,
                        "F_SLICEWE1USED",
                        xlat_bitvec(vec![f_slicewe1used]),
                    );
                    ctx.tiledb.insert(
                        "CLB",
                        bel,
                        "G_SLICEWE0USED",
                        xlat_bitvec(vec![g_slicewe0used]),
                    );
                    ctx.tiledb.insert(
                        "CLB",
                        bel,
                        "G_SLICEWE1USED",
                        xlat_bitvec(vec![g_slicewe1used]),
                    );
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
                        ("G".to_string(), gymux_g),
                        ("FX".to_string(), gymux_fx),
                        ("SOPOUT".to_string(), gymux_sopout),
                        ("GXOR".to_string(), gymux_gxor),
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
                xlat_enum(vec![("BX".to_string(), dx_bx), ("X".to_string(), dx_x)]),
            );
            let dy_by = ctx.state.get_diff("CLB", bel, "DYMUX", "0");
            let dy_y = ctx.state.get_diff("CLB", bel, "DYMUX", "1");
            ctx.tiledb.insert(
                "CLB",
                bel,
                "DYMUX",
                xlat_enum(vec![("BY".to_string(), dy_by), ("Y".to_string(), dy_y)]),
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
                    ("X".to_string(), dxmux_x),
                    ("F5".to_string(), dxmux_f5),
                    ("XB".to_string(), dxmux_xb),
                    ("FXOR".to_string(), dxmux_fxor),
                    ("BX".to_string(), dxmux_bx),
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
                    ("Y".to_string(), dymux_y),
                    ("FX".to_string(), dymux_fx),
                    ("YB".to_string(), dymux_yb),
                    ("GXOR".to_string(), dymux_gxor),
                    ("BY".to_string(), dymux_by),
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
                xlat_enum(vec![
                    ("FMC15".to_string(), xbmux_shiftout),
                    ("FCY".to_string(), xbmux_cout),
                ]),
            );

            let ybmux_shiftout = ctx.state.get_diff("CLB", bel, "YBMUX", "0");
            let ybmux_cout = ctx.state.get_diff("CLB", bel, "YBMUX", "1");
            ctx.tiledb.insert(
                "CLB",
                bel,
                "YBMUX",
                xlat_enum(vec![
                    ("GMC15".to_string(), ybmux_shiftout),
                    ("GCY".to_string(), ybmux_cout),
                ]),
            );
        }

        // FFs
        let ff_sync = ctx.state.get_diff("CLB", bel, "SYNC_ATTR", "SYNC");
        ctx.state
            .get_diff("CLB", bel, "SYNC_ATTR", "ASYNC")
            .assert_empty();
        ctx.tiledb
            .insert("CLB", bel, "FF_SYNC", xlat_bitvec(vec![ff_sync]));

        let ff_latch = ctx.state.get_diff("CLB", bel, "FFX", "#LATCH");
        assert_eq!(ff_latch, ctx.state.get_diff("CLB", bel, "FFY", "#LATCH"));
        ctx.state.get_diff("CLB", bel, "FFX", "#FF").assert_empty();
        ctx.state.get_diff("CLB", bel, "FFY", "#FF").assert_empty();
        ctx.tiledb
            .insert("CLB", bel, "FF_LATCH", xlat_bitvec(vec![ff_latch]));

        let revused = ctx.state.get_diff("CLB", bel, "REVUSED", "0");
        ctx.tiledb
            .insert("CLB", bel, "FF_REV_EN", xlat_bitvec(vec![revused]));

        let ffx_srval = !ctx.state.get_diff("CLB", bel, "FFX_SR_ATTR", "SRLOW");
        let ffy_srval = !ctx.state.get_diff("CLB", bel, "FFY_SR_ATTR", "SRLOW");
        ctx.state
            .get_diff("CLB", bel, "FFX_SR_ATTR", "SRHIGH")
            .assert_empty();
        ctx.state
            .get_diff("CLB", bel, "FFY_SR_ATTR", "SRHIGH")
            .assert_empty();
        ctx.tiledb
            .insert("CLB", bel, "FFX_SRVAL", xlat_bitvec(vec![ffx_srval]));
        ctx.tiledb
            .insert("CLB", bel, "FFY_SRVAL", xlat_bitvec(vec![ffy_srval]));

        let ffx_init = ctx.state.get_diff("CLB", bel, "FFX_INIT_ATTR", "INIT1");
        let ffy_init = ctx.state.get_diff("CLB", bel, "FFY_INIT_ATTR", "INIT1");
        ctx.state
            .get_diff("CLB", bel, "FFX_INIT_ATTR", "INIT0")
            .assert_empty();
        ctx.state
            .get_diff("CLB", bel, "FFY_INIT_ATTR", "INIT0")
            .assert_empty();
        ctx.tiledb
            .insert("CLB", bel, "FFX_INIT", xlat_bitvec(vec![ffx_init]));
        ctx.tiledb
            .insert("CLB", bel, "FFY_INIT", xlat_bitvec(vec![ffy_init]));

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
    if mode == Mode::Virtex2 {
        for bel in ["TBUF0", "TBUF1"] {
            ctx.collect_int_inv(&["INT.CLB"], "CLB", bel, "T", false);
            ctx.collect_int_inv(&["INT.CLB"], "CLB", bel, "I", true);
            for attr in ["OUT_A", "OUT_B"] {
                ctx.tiledb.insert(
                    "CLB",
                    bel,
                    attr,
                    xlat_bitvec(vec![ctx.state.get_diff("CLB", bel, attr, "1")]),
                );
            }
        }
        ctx.tiledb.insert(
            "CLB",
            "TBUS",
            "JOINER",
            xlat_bitvec(vec![ctx.state.get_diff("CLB", "TBUS", "JOINER", "1")]),
        );
    }
    let egrid = ctx.edev.egrid();
    for (node_kind, name, node) in &egrid.db.nodes {
        if !name.starts_with("INT.") {
            continue;
        }
        if name == "INT.CLB" {
            continue;
        }
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }
        for &wire in node.muxes.keys() {
            let wire_name = egrid.db.wires.key(wire.1);
            let inv_name = format!("INT:INV.{wire_name}");
            let mux_name = format!("INT:MUX.{wire_name}");
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
