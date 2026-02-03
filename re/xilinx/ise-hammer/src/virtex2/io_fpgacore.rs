use prjcombine_interconnect::{db::BelSlotId, dir::Dir, grid::TileCoord};
use prjcombine_re_collector::diff::{Diff, DiffKey, xlat_bit, xlat_bit_bi, xlat_bit_wide};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex2::defs::{self, bcls, bslots, enums, spartan3::tcls};

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

fn ioi_to_iob(slot: BelSlotId) -> BelSlotId {
    if let Some(idx) = bslots::IREG.index_of(slot) {
        bslots::IBUF[idx]
    } else if let Some(idx) = bslots::OREG.index_of(slot) {
        bslots::OBUF[idx]
    } else {
        unreachable!()
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
            let key = match fuzzer.info.features[0].key {
                DiffKey::BelAttrBit(_tcid, bslot, attr, bit, val) => {
                    DiffKey::BelAttrBit(tile.class, ioi_to_iob(bslot), attr, bit, val)
                }
                _ => unreachable!(),
            };
            fuzzer.info.features.push(FuzzerFeature {
                key,
                rects: backend.edev.tile_bits(tcrd),
            });
            Some((fuzzer, false))
        } else {
            Some((fuzzer, true))
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::IOI_FC);
    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::IREG[i]);
        let mode = "IBUF";
        bctx.build()
            .null_bits()
            .prop(IobExtra::new(Dir::W))
            .prop(IobExtra::new(Dir::E))
            .prop(IobExtra::new(Dir::S))
            .prop(IobExtra::new(Dir::N))
            .test_bel_attr_bits(bcls::IBUF::ENABLE)
            .mode(mode)
            .commit();
        bctx.mode(mode)
            .null_bits()
            .prop(IobExtra::new(Dir::W))
            .prop(IobExtra::new(Dir::E))
            .prop(IobExtra::new(Dir::S))
            .prop(IobExtra::new(Dir::N))
            .test_bel_attr_bits(bcls::IBUF::O2IPAD_ENABLE)
            .attr("ENABLE_O2IPADPATH", "ENABLE_O2IPADPATH")
            .commit();
        bctx.mode(mode)
            .attr("ENABLE_O2IQPATH", "")
            .test_bel_attr_bits(bcls::IREG::O2I_ENABLE)
            .attr("ENABLE_O2IPATH", "ENABLE_O2IPATH")
            .commit();
        bctx.mode(mode)
            .attr("ENABLE_O2IPATH", "")
            .test_bel_attr_bits(bcls::IREG::O2IQ_ENABLE)
            .attr("ENABLE_O2IQPATH", "ENABLE_O2IQPATH")
            .commit();
        bctx.mode(mode)
            .attr("IFFDMUX", "1")
            .attr("IFF", "#FF")
            .pin("I")
            .pin("IQ")
            .test_bel_attr_bool_rename("IMUX", bcls::IREG::I_DELAY_ENABLE, "1", "0");
        bctx.mode(mode)
            .attr("IMUX", "1")
            .attr("IFF", "#FF")
            .pin("I")
            .pin("IQ")
            .test_bel_attr_bool_rename("IFFDMUX", bcls::IREG::IQ_DELAY_ENABLE, "1", "0");
        bctx.mode(mode)
            .attr("IFFDMUX", "1")
            .attr("IFF_INIT_ATTR", "INIT1")
            .attr("CEINV", "CE_B")
            .pin("IQ")
            .pin("CE")
            .test_bel_attr_bool_rename("IFF", bcls::IREG::FF_LATCH, "#FF", "#LATCH");
        bctx.mode(mode)
            .attr("IFF", "#FF")
            .attr("IFFDMUX", "1")
            .pin("IQ")
            .test_bel_attr_bool_rename("IFFATTRBOX", bcls::IREG::FF_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode(mode)
            .attr("IFF", "#FF")
            .attr("IFFDMUX", "1")
            .attr("IFF_SR_ATTR", "SRLOW")
            .pin("IQ")
            .test_bel_attr_bool_rename("IFF_INIT_ATTR", bcls::IREG::FF_INIT, "INIT0", "INIT1");
        bctx.mode(mode)
            .attr("IFF", "#FF")
            .attr("IFFDMUX", "1")
            .attr("IFF_INIT_ATTR", "INIT0")
            .pin("IQ")
            .test_bel_attr_bool_rename("IFF_SR_ATTR", bcls::IREG::FF_SRVAL, "SRLOW", "SRHIGH");

        for pin in [
            bcls::IREG::CLK,
            bcls::IREG::CE,
            bcls::IREG::SR,
            bcls::IREG::REV,
        ] {
            bctx.mode(mode)
                .pin("IQ")
                .attr("IFF", "#FF")
                .test_bel_input_inv_auto(pin);
        }
    }
    for i in 0..4 {
        let mut bctx = ctx.bel(defs::bslots::OREG[i]);
        let mode = "OBUF";
        bctx.build()
            .null_bits()
            .prop(IobExtra::new(Dir::W))
            .prop(IobExtra::new(Dir::E))
            .prop(IobExtra::new(Dir::S))
            .prop(IobExtra::new(Dir::N))
            .test_bel_attr_bits(bcls::OBUF::ENABLE)
            .mode(mode)
            .attr("ENABLE_MISR", "FALSE")
            .commit();
        bctx.mode(mode)
            .null_bits()
            .prop(IobExtra::new(Dir::W))
            .prop(IobExtra::new(Dir::E))
            .prop(IobExtra::new(Dir::S))
            .prop(IobExtra::new(Dir::N))
            .test_bel_attr_bits(bcls::OBUF::MISR_ENABLE)
            .attr_diff("ENABLE_MISR", "FALSE", "TRUE")
            .commit();
        for pin in [
            bcls::OREG::CLK,
            bcls::OREG::CE,
            bcls::OREG::SR,
            bcls::OREG::REV,
            bcls::OREG::O,
        ] {
            bctx.mode(mode)
                .attr("OMUX", "OFF")
                .attr("OFF", "#FF")
                .test_bel_input_inv_auto(pin);
        }
        bctx.mode(mode)
            .attr("OINV", "O")
            .attr("OFF_INIT_ATTR", "INIT1")
            .attr("CEINV", "CE_B")
            .pin("O")
            .pin("CE")
            .test_bel_attr_bool_rename("OFF", bcls::OREG::FF_LATCH, "#FF", "#LATCH");
        bctx.mode(mode)
            .attr("OFF", "#FF")
            .attr("OINV", "O")
            .pin("O")
            .test_bel_attr_bool_rename("OFFATTRBOX", bcls::OREG::FF_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode(mode)
            .attr("OFF", "#FF")
            .attr("OINV", "O")
            .attr("OFF_SR_ATTR", "SRLOW")
            .pin("O")
            .test_bel_attr_bool_rename("OFF_INIT_ATTR", bcls::OREG::FF_INIT, "INIT0", "INIT1");
        bctx.mode(mode)
            .attr("OFF", "#FF")
            .attr("OINV", "O")
            .attr("OFF_INIT_ATTR", "INIT0")
            .pin("O")
            .test_bel_attr_bool_rename("OFF_SR_ATTR", bcls::OREG::FF_SRVAL, "SRLOW", "SRHIGH");
        for (val, vname) in [(enums::OREG_MUX_O::O, "O"), (enums::OREG_MUX_O::OQ, "OFF")] {
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .attr("OINV", "O")
                .pin("O")
                .test_bel_attr_val(bcls::OREG::MUX_O, val)
                .attr("OMUX", vname)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::IOI_FC;
    for i in 0..4 {
        let bslot = bslots::IREG[i];
        let diff_i = ctx.get_diff_attr_bit(tcid, bslot, bcls::IREG::O2I_ENABLE, 0);
        let diff_iq = ctx.get_diff_attr_bit(tcid, bslot, bcls::IREG::O2IQ_ENABLE, 0);
        let (diff_i, diff_iq, diff_common) = Diff::split(diff_i, diff_iq);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::IREG::O2I_ENABLE, xlat_bit(diff_i));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::IREG::O2IQ_ENABLE, xlat_bit(diff_iq));
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::IREG::O2I_O2IQ_ENABLE,
            xlat_bit(diff_common),
        );
        for pin in [bcls::IREG::CLK, bcls::IREG::CE] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        for (pin, attr) in [
            (bcls::IREG::SR, bcls::IREG::FF_SR_ENABLE),
            (bcls::IREG::REV, bcls::IREG::FF_REV_ENABLE),
        ] {
            let d0 = ctx.get_diff_bel_input_inv(tcid, bslot, pin, false);
            let d1 = ctx.get_diff_bel_input_inv(tcid, bslot, pin, true);
            let (d0, d1, de) = Diff::split(d0, d1);
            ctx.insert_bel_input_inv(tcid, bslot, pin, xlat_bit_bi(d0, d1));
            ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(de));
        }
        ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::IREG::I_DELAY_ENABLE, false)
            .assert_empty();
        ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::IREG::IQ_DELAY_ENABLE, false)
            .assert_empty();
        let diff_i = ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::IREG::I_DELAY_ENABLE, true);
        let diff_iff = ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::IREG::IQ_DELAY_ENABLE, true);
        let (diff_i, diff_iff, diff_common) = Diff::split(diff_i, diff_iff);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::IREG::I_DELAY_ENABLE, xlat_bit(diff_i));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::IREG::IQ_DELAY_ENABLE, xlat_bit(diff_iff));
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::IREG::DELAY_ENABLE,
            xlat_bit_wide(diff_common),
        );
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::IREG::FF_LATCH);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::IREG::FF_SR_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::IREG::FF_INIT);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::IREG::FF_SRVAL);
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::IREG::READBACK_I,
            TileBit::new(0, 3, [0, 31, 32, 63][i]).pos(),
        );
        for tcid in [
            tcls::IOB_FC_S,
            tcls::IOB_FC_N,
            tcls::IOB_FC_W,
            tcls::IOB_FC_E,
        ] {
            let bslot = bslots::IBUF[i];
            ctx.collect_bel_attr(tcid, bslot, bcls::IBUF::ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IBUF::O2IPAD_ENABLE);
        }
    }
    for i in 0..4 {
        let tcid = tcls::IOI_FC;
        let bslot = bslots::OREG[i];
        for pin in [bcls::OREG::CLK, bcls::OREG::O] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT_IOI_FC], tcid, bslot, bcls::OREG::CE);
        for (pin, attr) in [
            (bcls::OREG::SR, bcls::OREG::FF_SR_ENABLE),
            (bcls::OREG::REV, bcls::OREG::FF_REV_ENABLE),
        ] {
            let d0 = ctx.get_diff_bel_input_inv(tcid, bslot, pin, false);
            let d1 = ctx.get_diff_bel_input_inv(tcid, bslot, pin, true);
            let (d0, d1, de) = Diff::split(d0, d1);
            if pin == bcls::OREG::REV {
                ctx.insert_bel_input_inv(tcid, bslot, pin, xlat_bit_bi(d0, d1));
            } else {
                ctx.insert_bel_input_inv_int(
                    &[tcls::INT_IOI_FC],
                    tcid,
                    bslot,
                    pin,
                    xlat_bit_bi(d0, d1),
                );
            }
            ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(de));
        }

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OREG::FF_LATCH);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OREG::FF_SR_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OREG::FF_INIT);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OREG::FF_SRVAL);
        ctx.collect_bel_attr_default(tcid, bslot, bcls::OREG::MUX_O, enums::OREG_MUX_O::NONE);

        for tcid in [
            tcls::IOB_FC_S,
            tcls::IOB_FC_N,
            tcls::IOB_FC_W,
            tcls::IOB_FC_E,
        ] {
            let bslot = bslots::OBUF[i];
            let diff = ctx.get_diff_attr_bit(tcid, bslot, bcls::OBUF::ENABLE, 0);
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::OBUF::ENABLE, xlat_bit_wide(diff));
            ctx.collect_bel_attr(tcid, bslot, bcls::OBUF::MISR_ENABLE);
        }
    }
}
