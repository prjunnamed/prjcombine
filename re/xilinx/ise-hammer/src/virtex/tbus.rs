use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_fpga_hammer::diff::xlat_bool;
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{fbuild::FuzzCtx, props::relation::TileRelation},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Mode {
    Virtex,
    Virtex2,
}

#[derive(Copy, Clone, Debug)]
struct ClbTbusRight;

impl TileRelation for ClbTbusRight {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let mut cell = tcrd.cell;
        loop {
            if cell.col == backend.edev.cols(cell.die).last().unwrap() {
                return None;
            }
            cell.col += 1;
            match backend.edev {
                ExpandedDevice::Virtex(_) => {
                    if backend
                        .edev
                        .has_bel(cell.bel(prjcombine_virtex::defs::bslots::SLICE[0]))
                    {
                        return Some(cell.tile(prjcombine_virtex::defs::tslots::MAIN));
                    }
                }
                ExpandedDevice::Virtex2(edev) => {
                    if cell.col == edev.chip.col_e() - 1 {
                        return None;
                    }
                    if backend
                        .edev
                        .has_bel(cell.bel(prjcombine_virtex2::defs::bslots::SLICE[0]))
                    {
                        return Some(cell.tile(prjcombine_virtex2::defs::tslots::BEL));
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mode = match backend.edev {
        ExpandedDevice::Virtex(_) => Mode::Virtex,
        ExpandedDevice::Virtex2(_) => Mode::Virtex2,
        _ => unreachable!(),
    };
    let (tbus, tbuf, tiles) = match mode {
        Mode::Virtex => (
            prjcombine_virtex::defs::bslots::TBUS,
            prjcombine_virtex::defs::bslots::TBUF,
            &["CLB", "IO_W", "IO_E"][..],
        ),
        Mode::Virtex2 => (
            prjcombine_virtex2::defs::bslots::TBUS,
            prjcombine_virtex2::defs::bslots::TBUF,
            &["CLB"][..],
        ),
    };
    for &tile in tiles {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for (i, out_a, out_b) in [(0, "BUS0", "BUS2"), (1, "BUS1", "BUS3")] {
            let mut bctx = ctx.bel(tbuf[i]);
            if mode == Mode::Virtex {
                bctx.mode("TBUF")
                    .pin("T")
                    .pin("O")
                    .test_enum("TMUX", &["0", "1", "T", "T_B"]);
                bctx.mode("TBUF")
                    .pin("I")
                    .pin("O")
                    .test_enum("IMUX", &["0", "1", "I", "I_B"]);
            } else {
                bctx.mode("TBUF").pin("O").test_inv("T");
                bctx.mode("TBUF").pin("O").test_inv("I");
            }
            bctx.build()
                .row_mutex_here("TBUF")
                .test_manual("OUT_A", "1")
                .pip((tbus, out_a), "O")
                .commit();
            bctx.build()
                .row_mutex_here("TBUF")
                .test_manual("OUT_B", "1")
                .pip((tbus, out_b), "O")
                .commit();
        }
        let mut bctx = ctx.bel(tbus);
        if tile == "IO_W" {
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual("JOINER", "1")
                .pip("BUS3_E", "BUS3")
                .commit();
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual("JOINER_E", "1")
                .related_pip(ClbTbusRight, "BUS3_E", "BUS3")
                .commit();
        } else if tile == "CLB" {
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual("JOINER_E", "1")
                .related_pip(ClbTbusRight, "BUS3_E", "BUS3")
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let mode = match ctx.edev {
        ExpandedDevice::Virtex(_) => Mode::Virtex,
        ExpandedDevice::Virtex2(_) => Mode::Virtex2,
        _ => unreachable!(),
    };
    let tiles: &[_] = match mode {
        Mode::Virtex => &["CLB", "IO_W", "IO_E"],
        Mode::Virtex2 => &["CLB"],
    };
    for &tile in tiles {
        for bel in ["TBUF[0]", "TBUF[1]"] {
            if mode == Mode::Virtex {
                for (pinmux, pin, pin_b) in [("TMUX", "T", "T_B"), ("IMUX", "I", "I_B")] {
                    let d0 = ctx.get_diff(tile, bel, pinmux, pin);
                    assert_eq!(d0, ctx.get_diff(tile, bel, pinmux, "1"));
                    let d1 = ctx.get_diff(tile, bel, pinmux, pin_b);
                    assert_eq!(d1, ctx.get_diff(tile, bel, pinmux, "0"));
                    let item = xlat_bool(d0, d1);
                    ctx.insert_int_inv(&[tile], tile, bel, pin, item);
                }
            } else {
                ctx.collect_int_inv(&["INT_CLB"], tile, bel, "T", false);
                ctx.collect_int_inv(&["INT_CLB"], tile, bel, "I", true);
            }
            for attr in ["OUT_A", "OUT_B"] {
                ctx.collect_bit(tile, bel, attr, "1");
            }
        }
        let bel = "TBUS";
        if tile == "IO_W" {
            ctx.collect_bit(tile, bel, "JOINER", "1");
        }
        if tile != "IO_E" {
            ctx.collect_bit(tile, bel, "JOINER_E", "1");
        }
    }
}
