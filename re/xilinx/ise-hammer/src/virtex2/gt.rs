use prjcombine_interconnect::db::{BelAttributeType, BelInfo, BelInput};
use prjcombine_re_hammer::Session;
use prjcombine_types::{bits, bitvec::BitVec};
use prjcombine_virtex2::defs::{bcls, bslots, enums, virtex2::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex2::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    let bcls = &intdb[bcls::GT];
    for tcid in [tcls::GIGABIT_S, tcls::GIGABIT_N] {
        let mut ctx = FuzzCtx::new_id(session, backend, tcid);
        let bel_data = &intdb[ctx.tile_class.unwrap()].bels[bslots::GT];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let mut bctx = ctx.bel(bslots::GT);
        let mode = "GT";
        for (pid, &inp) in &bel_data.inputs {
            let BelInput::Fixed(wire) = inp else {
                unreachable!()
            };
            if intdb.wires.key(wire.wire).starts_with("IMUX_G") {
                continue;
            }
            bctx.mode(mode).test_bel_input_inv_auto(pid);
        }
        for (spec, val) in [
            (specials::GT_IOSTANDARD_FIBRE_CHAN, "FIBRE_CHAN"),
            (specials::GT_IOSTANDARD_ETHERNET, "ETHERNET"),
            (specials::GT_IOSTANDARD_XAUI, "XAUI"),
            (specials::GT_IOSTANDARD_INFINIBAND, "INFINIBAND"),
            (specials::GT_IOSTANDARD_AURORA, "AURORA"),
        ] {
            bctx.mode(mode)
                .null_bits()
                .test_bel_special(spec)
                .attr("IOSTANDARD", val)
                .commit();
        }
        for (aid, aname, attr) in &bcls.attributes {
            match aid {
                bcls::GT::ENABLE => {
                    bctx.build()
                        .test_bel_attr_bits(bcls::GT::ENABLE)
                        .mode(mode)
                        .commit();
                }
                bcls::GT::CHAN_BOND_MODE => {
                    bctx.mode(mode)
                        .test_bel_attr_default(aid, enums::GT_CHAN_BOND_MODE::NONE);
                }
                bcls::GT::TX_PREEMPHASIS
                | bcls::GT::CLK_COR_REPEAT_WAIT
                | bcls::GT::CHAN_BOND_OFFSET
                | bcls::GT::REF_CLK_V_SEL => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                }
                bcls::GT::CHAN_BOND_WAIT => {
                    for val in 1..=15 {
                        bctx.mode(mode)
                            .test_bel_attr_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                bcls::GT::CHAN_BOND_LIMIT => {
                    for val in 1..=31 {
                        bctx.mode(mode)
                            .test_bel_attr_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                bcls::GT::CRC_START_OF_PKT | bcls::GT::CRC_END_OF_PKT => {
                    for (val, vname) in [
                        (0x1c, "K28_0"),
                        (0x3c, "K28_1"),
                        (0x5c, "K28_2"),
                        (0x7c, "K28_3"),
                        (0x9c, "K28_4"),
                        (0xbc, "K28_5"),
                        (0xdc, "K28_6"),
                        (0xf7, "K23_7"),
                        (0xfb, "K27_7"),
                        (0xfc, "K28_7"),
                        (0xfd, "K29_7"),
                        (0xfe, "K30_7"),
                    ] {
                        bctx.mode(mode)
                            .test_bel_attr_u32(aid, val)
                            .attr(aname, vname)
                            .commit();
                    }
                }
                bcls::GT::RX_BUFFER_LIMIT => {
                    bctx.mode(mode)
                        .attr("CHAN_BOND_MODE", "")
                        .test_bel_attr_multi(aid, MultiValue::Dec(0));
                    bctx.mode(mode)
                        .attr("CHAN_BOND_MODE", "MASTER")
                        .test_bel_attr_special(aid, specials::GT_CHAN_BOND_MODE_MASTER)
                        .attr("RX_BUFFER_LIMIT", "15")
                        .commit();
                    bctx.mode(mode)
                        .attr("CHAN_BOND_MODE", "SLAVE_1_HOP")
                        .test_bel_attr_special(aid, specials::GT_CHAN_BOND_MODE_SLAVE_1_HOP)
                        .attr("RX_BUFFER_LIMIT", "15")
                        .commit();
                    bctx.mode(mode)
                        .attr("CHAN_BOND_MODE", "SLAVE_2_HOPS")
                        .test_bel_attr_special(aid, specials::GT_CHAN_BOND_MODE_SLAVE_2_HOPS)
                        .attr("RX_BUFFER_LIMIT", "15")
                        .commit();
                }
                _ => match attr.typ {
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode).test_bel_attr(aid);
                    }
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .test_bel_attr_bool_rename(aname, aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::BitVec(_) => {
                        bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Bin);
                    }
                    _ => unreachable!(),
                },
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let bcls = &ctx.edev.db[bcls::GT];
    for tcid in [tcls::GIGABIT_S, tcls::GIGABIT_N] {
        let bslot = bslots::GT;
        let bel_data = &ctx.edev.db[tcid].bels[bslot];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        for (pid, &inp) in &bel_data.inputs {
            let BelInput::Fixed(wire) = inp else {
                unreachable!()
            };
            if ctx.edev.db.wires.key(wire.wire).starts_with("IMUX_G") {
                continue;
            }
            let int_tiles = &[
                tcls::INT_GT_CLKPAD,
                tcls::INT_PPC,
                tcls::INT_PPC,
                tcls::INT_PPC,
                tcls::INT_PPC,
            ];
            ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, pid);
        }
        for (aid, _, attr) in &bcls.attributes {
            match aid {
                bcls::GT::ENABLE => {
                    ctx.collect_bel_attr(tcid, bslot, aid);
                }
                bcls::GT::CHAN_BOND_MODE => {
                    ctx.collect_bel_attr_default(tcid, bslot, aid, enums::GT_CHAN_BOND_MODE::NONE);
                }
                bcls::GT::CHAN_BOND_WAIT => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..=15);
                }
                bcls::GT::CHAN_BOND_LIMIT => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..=31);
                }
                bcls::GT::CRC_START_OF_PKT | bcls::GT::CRC_END_OF_PKT => {
                    ctx.collect_bel_attr_sparse(
                        tcid,
                        bslot,
                        aid,
                        [
                            0x1c, 0x3c, 0x5c, 0x7c, 0x9c, 0xbc, 0xdc, 0xf7, 0xfb, 0xfc, 0xfd, 0xfe,
                        ],
                    );
                }
                _ => {
                    if attr.typ == BelAttributeType::Bool {
                        ctx.collect_bel_attr_bool_bi(tcid, bslot, aid);
                    } else {
                        ctx.collect_bel_attr(tcid, bslot, aid);
                    }
                }
            }
        }
        let item = ctx
            .bel_attr_bitvec(tcid, bslot, bcls::GT::RX_BUFFER_LIMIT)
            .to_vec();
        for (spec, val) in [
            (specials::GT_CHAN_BOND_MODE_MASTER, bits![0, 0, 1, 1]),
            (specials::GT_CHAN_BOND_MODE_SLAVE_1_HOP, bits![0, 0, 1, 0]),
            (specials::GT_CHAN_BOND_MODE_SLAVE_2_HOPS, bits![0, 0, 1, 0]),
        ] {
            let mut diff =
                ctx.get_diff_bel_attr_special(tcid, bslot, bcls::GT::RX_BUFFER_LIMIT, spec);
            diff.apply_bitvec_diff(&item, &val, &BitVec::repeat(false, 4));
            diff.assert_empty();
        }
    }
}
