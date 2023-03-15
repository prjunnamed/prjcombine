use prjcombine_entity::EntityId;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_xilinx_geom::ExpandedDevice;

use crate::{
    backend::{IseBackend, MultiValue, SimpleFeatureId},
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileKV, TileMultiFuzzKV, TileMultiFuzzerGen},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Mode {
    Virtex5,
    Virtex6,
    Spartan6,
}

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let mode = match backend.edev {
        ExpandedDevice::Virtex4(ref edev) => match edev.kind {
            prjcombine_virtex4::grid::GridKind::Virtex4 => unreachable!(),
            prjcombine_virtex4::grid::GridKind::Virtex5 => Mode::Virtex5,
            prjcombine_virtex4::grid::GridKind::Virtex6
            | prjcombine_virtex4::grid::GridKind::Virtex7 => Mode::Virtex6,
        },
        ExpandedDevice::Spartan6(_) => Mode::Spartan6,
        _ => unreachable!(),
    };

    for tile in if mode == Mode::Spartan6 {
        ["CLEXL", "CLEXM"]
    } else {
        ["CLBLL", "CLBLM"]
    } {
        let nk = backend.egrid.db.get_node(tile);
        let bk = if mode == Mode::Spartan6 {
            "SLICEX"
        } else {
            "SLICEL"
        };
        for i in 0..2 {
            let bel = BelId::from_idx(i);
            let bname = backend.egrid.db.nodes[nk].bels.key(bel);
            for v in ["SYNC", "ASYNC"] {
                session.add_fuzzer(Box::new(TileFuzzerGen {
                    node: nk,
                    bits: TileBits::Main,
                    feature: SimpleFeatureId {
                        tile,
                        bel: bname,
                        attr: "SYNC_ATTR",
                        val: v,
                    },
                    base: vec![
                        TileKV::SiteMode(bel, bk),
                        TileKV::SiteAttr(bel, "AFF", "#FF"),
                        TileKV::SitePin(bel, "AQ"),
                    ],
                    fuzz: vec![TileFuzzKV::SiteAttr(bel, "SYNC_ATTR", v)],
                }));
            }
            for attr in ["A6LUT", "B6LUT", "C6LUT", "D6LUT"] {
                session.add_fuzzer(Box::new(TileMultiFuzzerGen {
                    node: nk,
                    bits: TileBits::Main,
                    feature: SimpleFeatureId {
                        tile,
                        bel: bname,
                        attr,
                        val: "#LUT",
                    },
                    base: vec![TileKV::SiteMode(bel, bk)],
                    width: 64,
                    fuzz: TileMultiFuzzKV::SiteAttr(bel, attr, MultiValue::Lut),
                }));
            }
        }
    }
}
