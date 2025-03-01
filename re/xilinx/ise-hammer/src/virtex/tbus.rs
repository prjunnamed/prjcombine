use prjcombine_interconnect::grid::NodeLoc;
use prjcombine_re_fpga_hammer::xlat_bool;
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{fbuild::FuzzCtx, props::relation::NodeRelation},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Mode {
    Virtex,
    Virtex2,
}

#[derive(Copy, Clone, Debug)]
struct ClbTbusRight;

impl NodeRelation for ClbTbusRight {
    fn resolve(&self, backend: &IseBackend, mut nloc: NodeLoc) -> Option<NodeLoc> {
        loop {
            if nloc.1 == backend.egrid.die(nloc.0).cols().next_back().unwrap() {
                return None;
            }
            nloc.1 += 1;
            if let Some(layer) = backend
                .egrid
                .find_node_layer(nloc.0, (nloc.1, nloc.2), |kind| kind == "CLB")
            {
                nloc.3 = layer;
                if let ExpandedDevice::Virtex2(edev) = backend.edev {
                    if nloc.1 == edev.chip.col_e() - 1 {
                        return None;
                    }
                }
                return Some(nloc);
            }
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mode = match backend.edev {
        prjcombine_re_xilinx_geom::ExpandedDevice::Virtex(_) => Mode::Virtex,
        prjcombine_re_xilinx_geom::ExpandedDevice::Virtex2(_) => Mode::Virtex2,
        _ => unreachable!(),
    };
    let (tbus, tbuf, tiles) = match mode {
        Mode::Virtex => (
            prjcombine_virtex::bels::TBUS,
            prjcombine_virtex::bels::TBUF,
            &["CLB", "IO.L", "IO.R"][..],
        ),
        Mode::Virtex2 => (
            prjcombine_virtex2::bels::TBUS,
            prjcombine_virtex2::bels::TBUF,
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
        if tile == "IO.L" {
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual("JOINER", "1")
                .pip("BUS3_E", "BUS3")
                .commit();
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual("JOINER_R", "1")
                .related_pip(ClbTbusRight, "BUS3_E", "BUS3")
                .commit();
        } else if tile == "CLB" {
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual("JOINER_R", "1")
                .related_pip(ClbTbusRight, "BUS3_E", "BUS3")
                .commit();
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
