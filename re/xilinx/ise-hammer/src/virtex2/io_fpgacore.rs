use prjcombine_interconnect::{dir::Dir, grid::TileCoord};
use prjcombine_re_collector::diff::{Diff, DiffKey, FeatureId, xlat_bit, xlat_bit_wide, xlat_bool};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_virtex2::defs;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
};

#[derive(Clone, Debug)]
struct IobExtra {
    edge: Dir,
}

impl IobExtra {
    pub fn new(edge: Dir) -> Self {
        Self { edge }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for IobExtra {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tcrd = tcrd.tile(defs::tslots::IOB);
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        let edge_match = match self.edge {
            Dir::W => tcrd.col == edev.chip.col_w(),
            Dir::E => tcrd.col == edev.chip.col_e(),
            Dir::S => tcrd.row == edev.chip.row_s(),
            Dir::N => tcrd.row == edev.chip.row_n(),
        };
        if edge_match {
            let tile = &backend.edev[tcrd];
            let DiffKey::Legacy(fuzzer_id) = fuzzer.info.features[0].key.clone() else {
                unreachable!()
            };
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: backend.edev.db.tile_classes.key(tile.class).clone(),
                    ..fuzzer_id
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
    let tile = "IOI_FC";
    let mut ctx = FuzzCtx::new(session, backend, tile);
    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::IBUF[i]);
        let mode = "IBUF";
        bctx.test_manual("ENABLE", "1")
            .mode(mode)
            .prop(IobExtra::new(Dir::W))
            .prop(IobExtra::new(Dir::E))
            .prop(IobExtra::new(Dir::S))
            .prop(IobExtra::new(Dir::N))
            .commit();
        bctx.mode(mode)
            .prop(IobExtra::new(Dir::W))
            .prop(IobExtra::new(Dir::E))
            .prop(IobExtra::new(Dir::S))
            .prop(IobExtra::new(Dir::N))
            .test_manual("ENABLE_O2IPADPATH", "1")
            .attr("ENABLE_O2IPADPATH", "ENABLE_O2IPADPATH")
            .commit();
        bctx.mode(mode)
            .attr("ENABLE_O2IQPATH", "")
            .test_manual("ENABLE_O2IPATH", "1")
            .attr("ENABLE_O2IPATH", "ENABLE_O2IPATH")
            .commit();
        bctx.mode(mode)
            .attr("ENABLE_O2IPATH", "")
            .test_manual("ENABLE_O2IQPATH", "1")
            .attr("ENABLE_O2IQPATH", "ENABLE_O2IQPATH")
            .commit();
        bctx.mode(mode)
            .attr("IFFDMUX", "1")
            .attr("IFF", "#FF")
            .pin("I")
            .pin("IQ")
            .test_enum("IMUX", &["0", "1"]);
        bctx.mode(mode)
            .attr("IMUX", "1")
            .attr("IFF", "#FF")
            .pin("I")
            .pin("IQ")
            .test_enum("IFFDMUX", &["0", "1"]);
        bctx.mode(mode)
            .attr("IFFDMUX", "1")
            .attr("IFF_INIT_ATTR", "INIT1")
            .attr("CEINV", "CE_B")
            .pin("IQ")
            .pin("CE")
            .test_enum("IFF", &["#FF", "#LATCH"]);
        bctx.mode(mode)
            .attr("IFF", "#FF")
            .attr("IFFDMUX", "1")
            .pin("IQ")
            .test_enum("IFFATTRBOX", &["SYNC", "ASYNC"]);
        bctx.mode(mode)
            .attr("IFF", "#FF")
            .attr("IFFDMUX", "1")
            .attr("IFF_SR_ATTR", "SRLOW")
            .pin("IQ")
            .test_enum("IFF_INIT_ATTR", &["INIT0", "INIT1"]);
        bctx.mode(mode)
            .attr("IFF", "#FF")
            .attr("IFFDMUX", "1")
            .attr("IFF_INIT_ATTR", "INIT0")
            .pin("IQ")
            .test_enum("IFF_SR_ATTR", &["SRLOW", "SRHIGH"]);

        for pin in ["CLK", "CE", "SR", "REV"] {
            bctx.mode(mode).pin("IQ").attr("IFF", "#FF").test_inv(pin);
        }
    }
    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::OBUF[i]);
        let mode = "OBUF";
        bctx.test_manual("ENABLE", "1")
            .mode(mode)
            .attr("ENABLE_MISR", "FALSE")
            .prop(IobExtra::new(Dir::W))
            .prop(IobExtra::new(Dir::E))
            .prop(IobExtra::new(Dir::S))
            .prop(IobExtra::new(Dir::N))
            .commit();
        bctx.mode(mode)
            .prop(IobExtra::new(Dir::W))
            .prop(IobExtra::new(Dir::E))
            .prop(IobExtra::new(Dir::S))
            .prop(IobExtra::new(Dir::N))
            .test_manual("ENABLE_MISR", "TRUE")
            .attr_diff("ENABLE_MISR", "FALSE", "TRUE")
            .commit();
        for pin in ["CLK", "CE", "SR", "REV", "O"] {
            bctx.mode(mode)
                .attr("OMUX", "OFF")
                .attr("OFF", "#FF")
                .test_inv(pin);
        }
        bctx.mode(mode)
            .attr("OINV", "O")
            .attr("OFF_INIT_ATTR", "INIT1")
            .attr("CEINV", "CE_B")
            .pin("O")
            .pin("CE")
            .test_enum("OFF", &["#FF", "#LATCH"]);
        bctx.mode(mode)
            .attr("OFF", "#FF")
            .attr("OINV", "O")
            .pin("O")
            .test_enum("OFFATTRBOX", &["SYNC", "ASYNC"]);
        bctx.mode(mode)
            .attr("OFF", "#FF")
            .attr("OINV", "O")
            .attr("OFF_SR_ATTR", "SRLOW")
            .pin("O")
            .test_enum("OFF_INIT_ATTR", &["INIT0", "INIT1"]);
        bctx.mode(mode)
            .attr("OFF", "#FF")
            .attr("OINV", "O")
            .attr("OFF_INIT_ATTR", "INIT0")
            .pin("O")
            .test_enum("OFF_SR_ATTR", &["SRLOW", "SRHIGH"]);
        bctx.mode(mode)
            .attr("OFF", "#FF")
            .attr("OINV", "O")
            .pin("O")
            .test_enum("OMUX", &["O", "OFF"]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for i in 0..4 {
        let tile = "IOI_FC";
        let bel = &format!("IBUF[{i}]");
        ctx.get_diff(tile, bel, "ENABLE", "1").assert_empty();
        ctx.get_diff(tile, bel, "ENABLE_O2IPADPATH", "1")
            .assert_empty();
        let diff_i = ctx.get_diff(tile, bel, "ENABLE_O2IPATH", "1");
        let diff_iq = ctx.get_diff(tile, bel, "ENABLE_O2IQPATH", "1");
        let (diff_i, diff_iq, diff_common) = Diff::split(diff_i, diff_iq);
        ctx.insert(tile, bel, "ENABLE_O2IPATH", xlat_bit(diff_i));
        ctx.insert(tile, bel, "ENABLE_O2IQPATH", xlat_bit(diff_iq));
        ctx.insert(tile, bel, "ENABLE_O2I_O2IQ_PATH", xlat_bit(diff_common));
        for pin in ["CLK", "CE"] {
            ctx.collect_inv(tile, bel, pin);
        }
        for pin in ["REV", "SR"] {
            let d0 = ctx.get_diff(tile, bel, format!("{pin}INV"), pin);
            let d1 = ctx.get_diff(tile, bel, format!("{pin}INV"), format!("{pin}_B"));
            let (d0, d1, de) = Diff::split(d0, d1);
            ctx.insert(tile, bel, format!("INV.{pin}"), xlat_bool(d0, d1));
            ctx.insert(tile, bel, format!("FF_{pin}_ENABLE"), xlat_bit(de));
        }
        ctx.get_diff(tile, bel, "IMUX", "1").assert_empty();
        ctx.get_diff(tile, bel, "IFFDMUX", "1").assert_empty();
        let diff_i = ctx.get_diff(tile, bel, "IMUX", "0");
        let diff_iff = ctx.get_diff(tile, bel, "IFFDMUX", "0");
        let (diff_i, diff_iff, diff_common) = Diff::split(diff_i, diff_iff);
        ctx.insert(tile, bel, "I_DELAY_ENABLE", xlat_bit(diff_i));
        ctx.insert(tile, bel, "IFF_DELAY_ENABLE", xlat_bit(diff_iff));
        ctx.insert(tile, bel, "DELAY_ENABLE", xlat_bit_wide(diff_common));
        let item = ctx.extract_enum_bool(tile, bel, "IFF", "#FF", "#LATCH");
        ctx.insert(tile, bel, "FF_LATCH", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFFATTRBOX", "ASYNC", "SYNC");
        ctx.insert(tile, bel, "FF_SR_SYNC", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFF_INIT_ATTR", "INIT0", "INIT1");
        ctx.insert(tile, bel, "FF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFF_SR_ATTR", "SRLOW", "SRHIGH");
        ctx.insert(tile, bel, "FF_SRVAL", item);
        ctx.insert(
            tile,
            bel,
            "READBACK_I",
            TileItem::from_bit(TileBit::new(0, 3, [0, 31, 32, 63][i]), false),
        );
        for tile in ["IOB_FC_S", "IOB_FC_N", "IOB_FC_W", "IOB_FC_E"] {
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE_O2IPADPATH", "1");
        }
    }
    for i in 0..4 {
        let tile = "IOI_FC";
        let bel = &format!("OBUF[{i}]");
        ctx.get_diff(tile, bel, "ENABLE", "1").assert_empty();
        ctx.get_diff(tile, bel, "ENABLE_MISR", "TRUE")
            .assert_empty();
        for pin in ["CLK", "O"] {
            ctx.collect_inv(tile, bel, pin);
        }
        ctx.collect_int_inv(&["INT_IOI_FC"], tile, bel, "CE", false);
        for pin in ["REV", "SR"] {
            let d0 = ctx.get_diff(tile, bel, format!("{pin}INV"), pin);
            let d1 = ctx.get_diff(tile, bel, format!("{pin}INV"), format!("{pin}_B"));
            let (d0, d1, de) = Diff::split(d0, d1);
            if pin == "REV" {
                ctx.insert(tile, bel, format!("INV.{pin}"), xlat_bool(d0, d1));
            } else {
                ctx.insert_int_inv(&["INT_IOI_FC"], tile, bel, pin, xlat_bool(d0, d1));
            }
            ctx.insert(tile, bel, format!("FF_{pin}_ENABLE"), xlat_bit(de));
        }
        let item = ctx.extract_enum_bool(tile, bel, "OFF", "#FF", "#LATCH");
        ctx.insert(tile, bel, "FF_LATCH", item);
        let item = ctx.extract_enum_bool(tile, bel, "OFFATTRBOX", "ASYNC", "SYNC");
        ctx.insert(tile, bel, "FF_SR_SYNC", item);
        let item = ctx.extract_enum_bool(tile, bel, "OFF_INIT_ATTR", "INIT0", "INIT1");
        ctx.insert(tile, bel, "FF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "OFF_SR_ATTR", "SRLOW", "SRHIGH");
        ctx.insert(tile, bel, "FF_SRVAL", item);
        ctx.collect_enum_default(tile, bel, "OMUX", &["O", "OFF"], "NONE");
        for tile in ["IOB_FC_S", "IOB_FC_N", "IOB_FC_W", "IOB_FC_E"] {
            ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE_MISR", "TRUE");
        }
    }
}
