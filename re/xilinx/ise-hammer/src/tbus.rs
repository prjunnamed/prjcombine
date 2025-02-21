use prjcombine_re_collector::xlat_bool;
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_one,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Mode {
    Virtex,
    Virtex2,
}

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let mode = match backend.edev {
        prjcombine_re_xilinx_geom::ExpandedDevice::Virtex(_) => Mode::Virtex,
        prjcombine_re_xilinx_geom::ExpandedDevice::Virtex2(_) => Mode::Virtex2,
        _ => unreachable!(),
    };
    let tiles: &[_] = match mode {
        Mode::Virtex => &["CLB", "IO.L", "IO.R"],
        Mode::Virtex2 => &["CLB"],
    };
    for &tile in tiles {
        let node = backend.egrid.db.get_node(tile);
        let node_data = &backend.egrid.db.nodes[node];
        let tbus_bel = node_data.bels.get("TBUS").unwrap().0;
        for (i, out_a, out_b) in [(0, "BUS0", "BUS2"), (1, "BUS1", "BUS3")] {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("TBUF{i}"),
                TileBits::MainAuto,
            );
            if mode == Mode::Virtex {
                fuzz_enum!(ctx, "TMUX", ["0", "1", "T", "T_B"], [
                    (mode "TBUF"),
                    (pin "T"),
                    (pin "O")
                ]);
                fuzz_enum!(ctx, "IMUX", ["0", "1", "I", "I_B"], [
                    (mode "TBUF"),
                    (pin "I"),
                    (pin "O")
                ]);
            } else {
                fuzz_inv!(ctx, "T", [
                    (mode "TBUF"),
                    (pin "O")
                ]);
                fuzz_inv!(ctx, "I", [
                    (mode "TBUF"),
                    (pin "O")
                ]);
            }
            fuzz_one!(ctx, "OUT_A", "1", [(row_mutex_site "TBUF")], [(pip (pin "O"), (bel_pin tbus_bel, out_a))]);
            fuzz_one!(ctx, "OUT_B", "1", [(row_mutex_site "TBUF")], [(pip (pin "O"), (bel_pin tbus_bel, out_b))]);
        }
        let ctx = FuzzCtx::new(session, backend, tile, "TBUS", TileBits::MainAuto);
        if tile == "IO.L" {
            let obel_tbus = BelId::from_idx(4);
            fuzz_one!(ctx, "JOINER", "1", [
                (row_mutex_site "TBUS")
                ], [
                (pip (pin "BUS3"), (pin "BUS3_E"))
            ]);
            fuzz_one!(ctx, "JOINER_R", "1", [
                (row_mutex_site "TBUS")
            ], [
                (related TileRelation::ClbTbusRight,
                    (pip (bel_pin obel_tbus, "BUS3"), (bel_pin obel_tbus, "BUS3_E"))
                )
            ]);
        } else if tile == "CLB" {
            fuzz_one!(ctx, "JOINER_R", "1", [
                (row_mutex_site "TBUS")
            ], [
                (related TileRelation::ClbTbusRight,
                    (pip (pin "BUS3"), (pin "BUS3_E"))
                )
            ]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let mode = match ctx.edev {
        prjcombine_re_xilinx_geom::ExpandedDevice::Virtex(_) => Mode::Virtex,
        prjcombine_re_xilinx_geom::ExpandedDevice::Virtex2(_) => Mode::Virtex2,
        _ => unreachable!(),
    };
    let tiles: &[_] = match mode {
        Mode::Virtex => &["CLB", "IO.L", "IO.R"],
        Mode::Virtex2 => &["CLB"],
    };
    for &tile in tiles {
        for bel in ["TBUF0", "TBUF1"] {
            if mode == Mode::Virtex {
                for (pinmux, pin, pin_b) in [("TMUX", "T", "T_B"), ("IMUX", "I", "I_B")] {
                    let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
                    assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "1"));
                    let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
                    assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "0"));
                    let item = xlat_bool(d0, d1);
                    ctx.insert_int_inv(&[tile], tile, bel, pin, item);
                }
            } else {
                ctx.collect_int_inv(&["INT.CLB"], tile, bel, "T", false);
                ctx.collect_int_inv(&["INT.CLB"], tile, bel, "I", true);
            }
            for attr in ["OUT_A", "OUT_B"] {
                ctx.collect_bit(tile, bel, attr, "1");
            }
        }
        let bel = "TBUS";
        if tile == "IO.L" {
            ctx.collect_bit(tile, bel, "JOINER", "1");
        }
        if tile != "IO.R" {
            ctx.collect_bit(tile, bel, "JOINER_R", "1");
        }
    }
}
