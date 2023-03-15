use prjcombine_entity::EntityId;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;

use crate::{
    backend::{IseBackend, MultiValue, SimpleFeatureId},
    fgen::{TileBits, TileFuzzKV, TileFuzzerGen, TileKV, TileMultiFuzzKV, TileMultiFuzzerGen},
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let nk = backend.egrid.db.get_node("CLB");
    for i in 0..2 {
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
                    TileKV::SiteMode(bel, "SLICE"),
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
                base: vec![TileKV::SiteMode(bel, "SLICE")],
                width: 16,
                fuzz: TileMultiFuzzKV::SiteAttr(bel, attr, MultiValue::Lut),
            }));
        }
    }
}
