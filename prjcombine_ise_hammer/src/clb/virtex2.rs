use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, MultiValue, SimpleFeatureId, State},
    diff::{collect_bitvec, collect_enum, xlat_bitvec, xlat_enum, Diff},
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileKV, TileMultiFuzzKV, TileMultiFuzzerGen},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_one,
    tiledb::TileDb,
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
            bits: TileBits::Main,
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
            fuzz_enum!(ctx, "CY0F", ["0", "1", "F1", "PROD", "F2", "BX"], [
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
            fuzz_enum!(ctx, "CY0G", ["0", "1", "G1", "PROD", "G2", "BY"], [
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
}

pub fn collect_fuzzers(state: &mut State, tiledb: &mut TileDb, mode: Mode) {
    for (idx, bel) in ["SLICE0", "SLICE1", "SLICE2", "SLICE3"]
        .into_iter()
        .enumerate()
    {
        collect_bitvec(state, tiledb, "CLB", bel, "F", "#LUT");
        collect_bitvec(state, tiledb, "CLB", bel, "G", "#LUT");

        // carry
        collect_enum(state, tiledb, "CLB", bel, "CYINIT", &["CIN", "BX"]);
        if mode != Mode::Virtex4 {
            collect_enum(state, tiledb, "CLB", bel, "CYSELF", &["F", "1"]);
            collect_enum(state, tiledb, "CLB", bel, "CYSELG", &["G", "1"]);
            collect_enum(
                state,
                tiledb,
                "CLB",
                bel,
                "CY0F",
                &["0", "1", "F1", "PROD", "F2", "BX"],
            );
            collect_enum(
                state,
                tiledb,
                "CLB",
                bel,
                "CY0G",
                &["0", "1", "G1", "PROD", "G2", "BY"],
            );
        } else {
            collect_enum(
                state,
                tiledb,
                "CLB",
                bel,
                "CY0F",
                &["0", "1", "F3", "PROD", "F2", "BX"],
            );
            collect_enum(
                state,
                tiledb,
                "CLB",
                bel,
                "CY0G",
                &["0", "1", "G3", "PROD", "G2", "BY"],
            );
        }

        // LUT RAM
        let is_m = mode == Mode::Virtex2 || matches!(idx, 0 | 2);
        if is_m {
            state.get_diff("CLB", bel, "F", "#LUT:0").assert_empty();
            state.get_diff("CLB", bel, "G", "#LUT:0").assert_empty();
            state
                .get_diff("CLB", bel, "F_ATTR", "DUAL_PORT")
                .assert_empty();
            state
                .get_diff("CLB", bel, "G_ATTR", "DUAL_PORT")
                .assert_empty();
            let f_ram = state.get_diff("CLB", bel, "F", "#RAM:0");
            let g_ram = state.get_diff("CLB", bel, "G", "#RAM:0");
            let (f_ram, g_ram, ram) = Diff::split(f_ram, g_ram);
            tiledb.insert("CLB", format!("{bel}.FF_SR_EN"), xlat_bitvec(vec![!ram]));
            let f_shift_d = state.get_diff("CLB", bel, "F_ATTR", "SHIFT_REG");
            let g_shift_d = state.get_diff("CLB", bel, "G_ATTR", "SHIFT_REG");
            let f_shift = f_ram.combine(&f_shift_d);
            let g_shift = g_ram.combine(&g_shift_d);
            tiledb.insert("CLB", format!("{bel}.F_RAM"), xlat_bitvec(vec![f_ram]));
            tiledb.insert("CLB", format!("{bel}.G_RAM"), xlat_bitvec(vec![g_ram]));
            tiledb.insert("CLB", format!("{bel}.F_SHIFT"), xlat_bitvec(vec![f_shift]));
            tiledb.insert("CLB", format!("{bel}.G_SHIFT"), xlat_bitvec(vec![g_shift]));

            let dif_bx = state.get_diff("CLB", bel, "DIF_MUX", "BX");
            let dif_alt = state.get_diff("CLB", bel, "DIF_MUX", "ALTDIF");
            assert_eq!(dif_alt, state.get_diff("CLB", bel, "DIF_MUX", "SHIFTIN"));
            tiledb.insert(
                "CLB",
                format!("{bel}.DIF_MUX"),
                xlat_enum(vec![
                    ("BX".to_string(), dif_bx),
                    ("ALT".to_string(), dif_alt),
                ]),
            );

            let dig_by = state.get_diff("CLB", bel, "DIG_MUX", "BY");
            let dig_alt = state.get_diff("CLB", bel, "DIG_MUX", "ALTDIG");
            assert_eq!(dig_alt, state.get_diff("CLB", bel, "DIG_MUX", "SHIFTIN"));
            tiledb.insert(
                "CLB",
                format!("{bel}.DIG_MUX"),
                xlat_enum(vec![
                    ("BY".to_string(), dig_by),
                    ("ALT".to_string(), dig_alt),
                ]),
            );

            match mode {
                Mode::Virtex2 => {
                    state.get_diff("CLB", bel, "BXOUTUSED", "0").assert_empty();
                    state
                        .get_diff("CLB", bel, "SLICEWE1USED", "0")
                        .assert_empty();
                    state
                        .get_diff("CLB", bel, "SLICEWE2USED", "0")
                        .assert_empty();
                    let slicewe0used = state.get_diff("CLB", bel, "SLICEWE0USED", "0");
                    let byoutused = state.get_diff("CLB", bel, "BYOUTUSED", "0");
                    assert_eq!(byoutused, state.get_diff("CLB", bel, "BYINVOUTUSED", "0"));
                    // TODO should these have better names?
                    tiledb.insert(
                        "CLB",
                        format!("{bel}.SLICEWE0USED"),
                        xlat_bitvec(vec![slicewe0used]),
                    );
                    tiledb.insert(
                        "CLB",
                        format!("{bel}.BYOUTUSED"),
                        xlat_bitvec(vec![byoutused]),
                    );
                }
                Mode::Spartan3 => {
                    state.get_diff("CLB", bel, "BYOUTUSED", "0").assert_empty();
                    state
                        .get_diff("CLB", bel, "BYINVOUTUSED", "0")
                        .assert_empty();
                    let slicewe0used = state.get_diff("CLB", bel, "SLICEWE0USED", "0");
                    let slicewe1used = state.get_diff("CLB", bel, "SLICEWE1USED", "0");
                    tiledb.insert(
                        "CLB",
                        format!("{bel}.SLICEWE0USED"),
                        xlat_bitvec(vec![slicewe0used]),
                    );
                    if idx == 0 {
                        tiledb.insert(
                            "CLB",
                            format!("{bel}.SLICEWE1USED"),
                            xlat_bitvec(vec![slicewe1used]),
                        );
                    } else {
                        slicewe1used.assert_empty();
                    }
                }
                Mode::Virtex4 => {
                    state.get_diff("CLB", bel, "BYOUTUSED", "0").assert_empty();
                    state
                        .get_diff("CLB", bel, "BYINVOUTUSED", "0")
                        .assert_empty();
                    let f_slicewe0used = state.get_diff("CLB", bel, "SLICEWE0USED.F", "0");
                    let f_slicewe1used = state.get_diff("CLB", bel, "SLICEWE1USED.F", "0");
                    let g_slicewe0used = state.get_diff("CLB", bel, "SLICEWE0USED.G", "0");
                    let g_slicewe1used = state.get_diff("CLB", bel, "SLICEWE1USED.G", "0");
                    tiledb.insert(
                        "CLB",
                        format!("{bel}.F_SLICEWE0USED"),
                        xlat_bitvec(vec![f_slicewe0used]),
                    );
                    tiledb.insert(
                        "CLB",
                        format!("{bel}.F_SLICEWE1USED"),
                        xlat_bitvec(vec![f_slicewe1used]),
                    );
                    tiledb.insert(
                        "CLB",
                        format!("{bel}.G_SLICEWE0USED"),
                        xlat_bitvec(vec![g_slicewe0used]),
                    );
                    tiledb.insert(
                        "CLB",
                        format!("{bel}.G_SLICEWE1USED"),
                        xlat_bitvec(vec![g_slicewe1used]),
                    );
                }
            }
        }

        // muxes
        match mode {
            Mode::Virtex2 => {
                collect_enum(state, tiledb, "CLB", bel, "FXMUX", &["F", "F5", "FXOR"]);
                collect_enum(
                    state,
                    tiledb,
                    "CLB",
                    bel,
                    "GYMUX",
                    &["G", "FX", "GXOR", "SOPEXT"],
                );
                collect_enum(state, tiledb, "CLB", bel, "SOPEXTSEL", &["SOPIN", "0"]);
            }
            Mode::Spartan3 => {
                collect_enum(state, tiledb, "CLB", bel, "FXMUX", &["F", "F5", "FXOR"]);
                collect_enum(state, tiledb, "CLB", bel, "GYMUX", &["G", "FX", "GXOR"]);
            }
            Mode::Virtex4 => {
                collect_enum(state, tiledb, "CLB", bel, "FXMUX", &["F5", "FXOR"]);
                collect_enum(state, tiledb, "CLB", bel, "GYMUX", &["FX", "GXOR"]);
            }
        }
        if mode != Mode::Virtex4 {
            let dx_bx = state.get_diff("CLB", bel, "DXMUX", "0");
            let dx_x = state.get_diff("CLB", bel, "DXMUX", "1");
            tiledb.insert(
                "CLB",
                format!("{bel}.DXMUX"),
                xlat_enum(vec![("BX".to_string(), dx_bx), ("X".to_string(), dx_x)]),
            );
            let dy_by = state.get_diff("CLB", bel, "DYMUX", "0");
            let dy_y = state.get_diff("CLB", bel, "DYMUX", "1");
            tiledb.insert(
                "CLB",
                format!("{bel}.DYMUX"),
                xlat_enum(vec![("BY".to_string(), dy_by), ("Y".to_string(), dy_y)]),
            );
        } else {
            let dxmux_bx = state.get_diff("CLB", bel, "DXMUX.F5", "BX");
            let dxmux_x = state.get_diff("CLB", bel, "DXMUX.F5", "X");
            let dxmux_xb = state.get_diff("CLB", bel, "DXMUX.F5", "XB");
            let dxmux_f5 = state.get_diff("CLB", bel, "DXMUX.F5", "XMUX");
            assert_eq!(dxmux_bx, state.get_diff("CLB", bel, "DXMUX.FXOR", "BX"));
            assert_eq!(dxmux_x, state.get_diff("CLB", bel, "DXMUX.FXOR", "X"));
            assert_eq!(dxmux_xb, state.get_diff("CLB", bel, "DXMUX.FXOR", "XB"));
            let dxmux_fxor = state.get_diff("CLB", bel, "DXMUX.FXOR", "XMUX");
            tiledb.insert(
                "CLB",
                format!("{bel}.DXMUX"),
                xlat_enum(vec![
                    ("BX".to_string(), dxmux_bx),
                    ("X".to_string(), dxmux_x),
                    ("XB".to_string(), dxmux_xb),
                    ("F5".to_string(), dxmux_f5),
                    ("FXOR".to_string(), dxmux_fxor),
                ]),
            );

            let dymux_by = state.get_diff("CLB", bel, "DYMUX.FX", "BY");
            let dymux_y = state.get_diff("CLB", bel, "DYMUX.FX", "Y");
            let dymux_yb = state.get_diff("CLB", bel, "DYMUX.FX", "YB");
            let dymux_fx = state.get_diff("CLB", bel, "DYMUX.FX", "YMUX");
            assert_eq!(dymux_by, state.get_diff("CLB", bel, "DYMUX.GXOR", "BY"));
            assert_eq!(dymux_y, state.get_diff("CLB", bel, "DYMUX.GXOR", "Y"));
            assert_eq!(dymux_yb, state.get_diff("CLB", bel, "DYMUX.GXOR", "YB"));
            let dymux_gxor = state.get_diff("CLB", bel, "DYMUX.GXOR", "YMUX");
            tiledb.insert(
                "CLB",
                format!("{bel}.DYMUX"),
                xlat_enum(vec![
                    ("BY".to_string(), dymux_by),
                    ("Y".to_string(), dymux_y),
                    ("YB".to_string(), dymux_yb),
                    ("FX".to_string(), dymux_fx),
                    ("GXOR".to_string(), dymux_gxor),
                ]),
            );
        }
        if is_m {
            let xbmux_shiftout = state.get_diff("CLB", bel, "XBMUX", "0");
            let xbmux_cout = state.get_diff("CLB", bel, "XBMUX", "1");
            tiledb.insert(
                "CLB",
                format!("{bel}.XBMUX"),
                xlat_enum(vec![
                    ("SHIFTOUT".to_string(), xbmux_shiftout),
                    ("COUT".to_string(), xbmux_cout),
                ]),
            );

            let ybmux_shiftout = state.get_diff("CLB", bel, "YBMUX", "0");
            let ybmux_cout = state.get_diff("CLB", bel, "YBMUX", "1");
            tiledb.insert(
                "CLB",
                format!("{bel}.YBMUX"),
                xlat_enum(vec![
                    ("SHIFTOUT".to_string(), ybmux_shiftout),
                    ("COUT".to_string(), ybmux_cout),
                ]),
            );
        }

        // FFs
        let ff_sync = state.get_diff("CLB", bel, "SYNC_ATTR", "SYNC");
        state
            .get_diff("CLB", bel, "SYNC_ATTR", "ASYNC")
            .assert_empty();
        tiledb.insert("CLB", format!("{bel}.FF_SYNC"), xlat_bitvec(vec![ff_sync]));

        let ff_latch = state.get_diff("CLB", bel, "FFX", "#LATCH");
        assert_eq!(ff_latch, state.get_diff("CLB", bel, "FFY", "#LATCH"));
        state.get_diff("CLB", bel, "FFX", "#FF").assert_empty();
        state.get_diff("CLB", bel, "FFY", "#FF").assert_empty();
        tiledb.insert(
            "CLB",
            format!("{bel}.FF_LATCH"),
            xlat_bitvec(vec![ff_latch]),
        );

        let revused = state.get_diff("CLB", bel, "REVUSED", "0");
        tiledb.insert(
            "CLB",
            format!("{bel}.FF_REV_EN"),
            xlat_bitvec(vec![revused]),
        );

        let ffx_srval = !state.get_diff("CLB", bel, "FFX_SR_ATTR", "SRLOW");
        let ffy_srval = !state.get_diff("CLB", bel, "FFY_SR_ATTR", "SRLOW");
        state
            .get_diff("CLB", bel, "FFX_SR_ATTR", "SRHIGH")
            .assert_empty();
        state
            .get_diff("CLB", bel, "FFY_SR_ATTR", "SRHIGH")
            .assert_empty();
        tiledb.insert(
            "CLB",
            format!("{bel}.FFX_SRVAL"),
            xlat_bitvec(vec![ffx_srval]),
        );
        tiledb.insert(
            "CLB",
            format!("{bel}.FFY_SRVAL"),
            xlat_bitvec(vec![ffy_srval]),
        );

        let ffx_init = state.get_diff("CLB", bel, "FFX_INIT_ATTR", "INIT1");
        let ffy_init = state.get_diff("CLB", bel, "FFY_INIT_ATTR", "INIT1");
        state
            .get_diff("CLB", bel, "FFX_INIT_ATTR", "INIT0")
            .assert_empty();
        state
            .get_diff("CLB", bel, "FFY_INIT_ATTR", "INIT0")
            .assert_empty();
        tiledb.insert(
            "CLB",
            format!("{bel}.FFX_INIT"),
            xlat_bitvec(vec![ffx_init]),
        );
        tiledb.insert(
            "CLB",
            format!("{bel}.FFY_INIT"),
            xlat_bitvec(vec![ffy_init]),
        );

        // inverts
        for (pininv, pin, pin_b, def) in [
            ("CLKINV", "CLK", "CLK_B", mode == Mode::Virtex4),
            ("CEINV", "CE", "CE_B", false),
            ("SRINV", "SR", "SR_B", mode != Mode::Virtex2),
            ("BXINV", "BX", "BX_B", false),
            ("BYINV", "BY", "BY_B", false),
        ] {
            let f_pin = state.get_diff("CLB", bel, pininv, pin);
            let f_pin_b = state.get_diff("CLB", bel, pininv, pin_b);
            let inv = if !def {
                f_pin.assert_empty();
                f_pin_b
            } else {
                f_pin_b.assert_empty();
                !f_pin
            };
            tiledb.insert("CLB", format!("{bel}.{pininv}"), xlat_bitvec(vec![inv]));
        }
    }
}
