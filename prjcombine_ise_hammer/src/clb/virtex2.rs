use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, MultiValue, SimpleFeatureId},
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileKV, TileMultiFuzzKV, TileMultiFuzzerGen},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Mode {
    Virtex2,
    Spartan3,
    Virtex4,
}

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let nk = backend.egrid.db.get_node("CLB");
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

    let bk = if mode == Mode::Virtex2 {
        "SLICE"
    } else {
        "SLICEL"
    };
    for i in 0..4 {
        let bel = BelId::from_idx(i);
        let bname = backend.egrid.db.nodes[nk].bels.key(bel);
        for v in ["SYNC", "ASYNC"] {
            session.add_fuzzer(Box::new(TileFuzzerGen {
                node: nk,
                bits: TileBits::Main,
                feature: SimpleFeatureId {
                    tile: "CLB",
                    bel: bname,
                    attr: "SYNC_ATTR",
                    val: v,
                },
                base: vec![
                    TileKV::SiteMode(bel, bk),
                    TileKV::SiteAttr(bel, "FFX", "#FF"),
                    TileKV::SitePin(bel, "XQ"),
                ],
                fuzz: vec![TileFuzzKV::SiteAttr(bel, "SYNC_ATTR", v)],
            }));
        }
        for attr in ["F", "G"] {
            session.add_fuzzer(Box::new(TileMultiFuzzerGen {
                node: nk,
                bits: TileBits::Main,
                feature: SimpleFeatureId {
                    tile: "CLB",
                    bel: bname,
                    attr,
                    val: "#LUT",
                },
                base: vec![TileKV::SiteMode(bel, bk)],
                width: 16,
                fuzz: TileMultiFuzzKV::SiteAttr(bel, attr, MultiValue::Lut),
            }));
        }
    }
}
