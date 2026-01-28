use std::collections::HashSet;

use prjcombine_interconnect::db::{
    BelAttributeId, BelAttributeType, BelInfo, BelInput, TableFieldId,
};
use prjcombine_re_collector::diff::{extract_bitvec_val_part, xlat_bitvec_sparse};
use prjcombine_re_hammer::Session;
use prjcombine_types::bitvec::BitVec;
use prjcombine_virtex2::defs::{
    bslots, enums, tables,
    virtex2::{bcls, tcls},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex2::specials,
};

fn pma_reg_attrs() -> Vec<(BelAttributeId, Option<TableFieldId>, Vec<usize>)> {
    vec![
        (
            bcls::GT10::MASTERBIAS,
            Some(tables::GT10_PMA_SPEED::MASTERBIAS),
            (0x00..0x02).collect(),
        ),
        (
            bcls::GT10::VCODAC,
            Some(tables::GT10_PMA_SPEED::VCODAC),
            (0x02..0x08).collect(),
        ),
        (
            bcls::GT10::TXDIVRATIO,
            Some(tables::GT10_PMA_SPEED::TXDIVRATIO),
            (0x08..0x12).collect(),
        ),
        (
            bcls::GT10::TXBUSWID,
            Some(tables::GT10_PMA_SPEED::TXBUSWID),
            vec![0x12],
        ),
        (
            bcls::GT10::ENDCD,
            Some(tables::GT10_PMA_SPEED::ENDCD),
            vec![0x13],
        ),
        (
            bcls::GT10::SEL_DAC_TRAN,
            Some(tables::GT10_PMA_SPEED::SEL_DAC_TRAN),
            vec![0x46, 0x47, 0x14, 0x15],
        ),
        (
            bcls::GT10::SEL_DAC_FIX,
            Some(tables::GT10_PMA_SPEED::SEL_DAC_FIX),
            vec![0x4e, 0x4f, 0x16, 0x17],
        ),
        (
            bcls::GT10::TXLOOPFILTERC,
            Some(tables::GT10_PMA_SPEED::TXLOOPFILTERC),
            (0x18..0x1a).collect(),
        ),
        (
            bcls::GT10::TXLOOPFILTERR,
            Some(tables::GT10_PMA_SPEED::TXLOOPFILTERR),
            (0x1a..0x1c).collect(),
        ),
        (
            bcls::GT10::IBOOST,
            Some(tables::GT10_PMA_SPEED::IBOOST),
            vec![0x1f],
        ),
        (
            bcls::GT10::TXCPI,
            Some(tables::GT10_PMA_SPEED::TXCPI),
            vec![0x20],
        ),
        (
            bcls::GT10::TXVCODAC,
            Some(tables::GT10_PMA_SPEED::TXVCODAC),
            vec![0x22],
        ),
        (
            bcls::GT10::TXVCOGAIN,
            Some(tables::GT10_PMA_SPEED::TXVCOGAIN),
            vec![0x23],
        ),
        (
            bcls::GT10::TXVSEL,
            Some(tables::GT10_PMA_SPEED::TXVSEL),
            (0x24..0x26).collect(),
        ),
        (
            bcls::GT10::TXREG,
            Some(tables::GT10_PMA_SPEED::TXREG),
            (0x26..0x28).collect(),
        ),
        (
            bcls::GT10::TXDOWNLEVEL,
            Some(tables::GT10_PMA_SPEED::TXDOWNLEVEL),
            (0x28..0x2c).collect(),
        ),
        (
            bcls::GT10::PRDRVOFF,
            Some(tables::GT10_PMA_SPEED::PRDRVOFF),
            vec![0x2c],
        ),
        (
            bcls::GT10::EMPOFF,
            Some(tables::GT10_PMA_SPEED::EMPOFF),
            vec![0x2d],
        ),
        (
            bcls::GT10::SLEW,
            Some(tables::GT10_PMA_SPEED::SLEW),
            vec![0x2e],
        ),
        (
            bcls::GT10::TXEMPHLEVEL,
            Some(tables::GT10_PMA_SPEED::TXEMPHLEVEL),
            (0x30..0x34).collect(),
        ),
        (
            bcls::GT10::TXDIGSW,
            Some(tables::GT10_PMA_SPEED::TXDIGSW),
            vec![0x34],
        ),
        (
            bcls::GT10::TXANASW,
            Some(tables::GT10_PMA_SPEED::TXANASW),
            vec![0x35],
        ),
        (
            bcls::GT10::RXDIVRATIO,
            Some(tables::GT10_PMA_SPEED::RXDIVRATIO),
            (0x38..0x46).collect(),
        ),
        (
            bcls::GT10::RXLOOPFILTERC,
            Some(tables::GT10_PMA_SPEED::RXLOOPFILTERC),
            (0x48..0x4a).collect(),
        ),
        (
            bcls::GT10::RXLOOPFILTERR,
            Some(tables::GT10_PMA_SPEED::RXLOOPFILTERR),
            (0x4a..0x4d).collect(),
        ),
        (
            bcls::GT10::AFE_FLAT_ENABLE,
            Some(tables::GT10_PMA_SPEED::AFE_FLAT_ENABLE),
            vec![0x4d],
        ),
        (
            bcls::GT10::RXVCOSW,
            Some(tables::GT10_PMA_SPEED::RXVCOSW),
            vec![0x50],
        ),
        (
            bcls::GT10::RXCPI,
            Some(tables::GT10_PMA_SPEED::RXCPI),
            vec![0x51, 0x5f],
        ),
        (
            bcls::GT10::RXVCODAC,
            Some(tables::GT10_PMA_SPEED::RXVCODAC),
            vec![0x52],
        ),
        (
            bcls::GT10::RXVCOGAIN,
            Some(tables::GT10_PMA_SPEED::RXVCOGAIN),
            vec![0x53],
        ),
        (
            bcls::GT10::RXVSEL,
            Some(tables::GT10_PMA_SPEED::RXVSEL),
            (0x54..0x56).collect(),
        ),
        (
            bcls::GT10::RXREG,
            Some(tables::GT10_PMA_SPEED::RXREG),
            (0x56..0x58).collect(),
        ),
        (
            bcls::GT10::RXFLTCPT,
            Some(tables::GT10_PMA_SPEED::RXFLTCPT),
            (0x58..0x5d).collect(),
        ),
        (
            bcls::GT10::RXVSELCP,
            Some(tables::GT10_PMA_SPEED::RXVSELCP),
            (0x5d..0x5f).collect(),
        ),
        (
            bcls::GT10::VSELAFE,
            Some(tables::GT10_PMA_SPEED::VSELAFE),
            (0x60..0x62).collect(),
        ),
        (
            bcls::GT10::RXFEI,
            Some(tables::GT10_PMA_SPEED::RXFEI),
            (0x62..0x64).collect(),
        ),
        (
            bcls::GT10::RXFLCPI,
            Some(tables::GT10_PMA_SPEED::RXFLCPI),
            (0x64..0x66).collect(),
        ),
        (
            bcls::GT10::RXFER,
            Some(tables::GT10_PMA_SPEED::RXFER),
            (0x66..0x70).collect(),
        ),
        (
            bcls::GT10::PMA_REG_0E,
            Some(tables::GT10_PMA_SPEED::PMA_REG_0E),
            (0x70..0x78).collect(),
        ),
        (bcls::GT10::BIASEN, None, vec![0x78]),
        (bcls::GT10::TXANAEN, None, vec![0x79]),
        (bcls::GT10::TXDIGEN, None, vec![0x7a]),
        (bcls::GT10::RXANAEN, None, vec![0x7b]),
        (bcls::GT10::PMA_PWR_CNTRL_BIT4, None, vec![0x7c]),
        (bcls::GT10::TXEN, None, vec![0x7d]),
        (bcls::GT10::RXEN, None, vec![0x7e]),
        (bcls::GT10::TXDRVEN, None, vec![0x7f]),
    ]
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    let bcls = &intdb[bcls::GT10];
    for tcid in [tcls::GIGABIT10_S, tcls::GIGABIT10_N] {
        let mut ctx = FuzzCtx::new_id(session, backend, tcid);
        let bel_data = &intdb[ctx.tile_class.unwrap()].bels[bslots::GT10];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let mut bctx = ctx.bel(bslots::GT10);
        let mode = "GT10";
        for (pid, &inp) in &bel_data.inputs {
            let BelInput::Fixed(wire) = inp else {
                unreachable!()
            };
            if intdb.wires.key(wire.wire).starts_with("IMUX_G") {
                continue;
            }
            bctx.mode(mode).test_bel_input_inv_auto(pid);
        }
        bctx.build()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for (spec, val) in [
            (specials::GT_IOSTANDARD_XAUI, "XAUI"),
            (specials::GT_IOSTANDARD_TEST, "TEST"),
            (specials::GT_IOSTANDARD_PCI_EXPRESS, "PCI_EXPRESS"),
            (specials::GT_IOSTANDARD_OC192, "OC192"),
            (specials::GT_IOSTANDARD_OC48, "OC48"),
            (specials::GT_IOSTANDARD_INFINIBAND, "INFINIBAND"),
            (specials::GT_IOSTANDARD_CUSTOM, "CUSTOM"),
            (specials::GT_IOSTANDARD_AURORA, "AURORA"),
            (specials::GT_IOSTANDARD_10GFC, "10GFC"),
            (specials::GT_IOSTANDARD_10GE, "10GE"),
        ] {
            bctx.mode(mode)
                .null_bits()
                .test_bel_special(spec)
                .attr("IOSTANDARD", val)
                .commit();
        }
        let skip_attrs: HashSet<_> =
            HashSet::from_iter(pma_reg_attrs().into_iter().map(|(attr, _, _)| attr));
        for (aid, aname, attr) in &bcls.attributes {
            if skip_attrs.contains(&aid) {
                continue;
            }
            match aid {
                bcls::GT10::PMA_REG => {
                    // handled below
                }
                bcls::GT10::CHAN_BOND_MODE => {
                    bctx.mode(mode)
                        .test_bel_attr_default(aid, enums::GT_CHAN_BOND_MODE::NONE);
                }
                bcls::GT10::CLK_COR_REPEAT_WAIT
                | bcls::GT10::CLK_COR_ADJ_MAX
                | bcls::GT10::CLK_COR_MIN_LAT
                | bcls::GT10::CLK_COR_MAX_LAT
                | bcls::GT10::CHAN_BOND_LIMIT
                | bcls::GT10::SH_INVALID_CNT_MAX
                | bcls::GT10::SH_CNT_MAX => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                }
                bcls::GT10::CRC_START_OF_PKT | bcls::GT10::CRC_END_OF_PKT => {
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
        for (spec, val) in [
            (specials::GT10_PMA_SPEED_USE, "PMA_SPEED"),
            (specials::GT10_PMA_SPEED_USE_HEX, "PMA_SPEED_HEX"),
        ] {
            bctx.mode(mode)
                .null_bits()
                .test_bel_special(spec)
                .attr("PMA_SPEED_USE", val)
                .commit();
        }
        for (row, rname, _) in &intdb.tables[tables::GT10_PMA_SPEED].rows {
            bctx.mode(mode)
                .attr("PMA_SPEED_USE", "PMA_SPEED")
                .test_bel_special_row(specials::GT10_PMA_SPEED, row)
                .attr("PMA_SPEED", rname.strip_prefix('_').unwrap())
                .commit();
        }
        bctx.mode(mode)
            .attr("PMA_SPEED_USE", "PMA_SPEED_HEX")
            .test_bel_special_bits(specials::GT10_PMA_SPEED_HEX)
            .multi_attr("PMA_SPEED_HEX", MultiValue::Hex(0), 120);
        bctx.mode(mode)
            .test_bel_special_bits(specials::GT10_PMA_PWR_CNTRL)
            .multi_attr("PMA_PWR_CNTRL", MultiValue::Bin, 8);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [tcls::GIGABIT10_S, tcls::GIGABIT10_N] {
        let bslot = bslots::GT10;
        let bel_data = &ctx.edev.db[tcid].bels[bslot];
        let bcls = &ctx.edev.db[bcls::GT10];
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
                tcls::INT_PPC,
                tcls::INT_PPC,
                tcls::INT_PPC,
                tcls::INT_PPC,
            ];
            ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, pid);
        }

        let skip_attrs: HashSet<_> =
            HashSet::from_iter(pma_reg_attrs().into_iter().map(|(attr, _, _)| attr));
        for (aid, _, attr) in &bcls.attributes {
            if skip_attrs.contains(&aid) {
                continue;
            }
            match aid {
                bcls::GT10::PMA_REG => {
                    // handled below
                }
                bcls::GT10::CRC_START_OF_PKT => {
                    ctx.collect_bel_attr_sparse(
                        tcid,
                        bslot,
                        aid,
                        [
                            0x1c, 0x3c, 0x5c, 0x7c, 0x9c, 0xbc, 0xdc, 0xf7, 0xfb, 0xfc, 0xfd, 0xfe,
                        ],
                    );
                }
                bcls::GT10::CRC_END_OF_PKT => {
                    let mut diffs = vec![];
                    let present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
                    for val in [
                        0x1c, 0x3c, 0x5c, 0x7c, 0x9c, 0xbc, 0xdc, 0xf7, 0xfb, 0xfc, 0xfd, 0xfe,
                    ] {
                        let mut bv = BitVec::repeat(false, 8);
                        for i in 0..8 {
                            bv.set(i, (val & 1 << i) != 0);
                        }
                        let diff = ctx.get_diff_attr_bitvec(tcid, bslot, aid, bv.clone());
                        diffs.push((bv, diff.combine(&present)));
                    }
                    let bits = xlat_bitvec_sparse(diffs);
                    ctx.insert_bel_attr_bitvec(tcid, bslot, aid, bits);
                }
                bcls::GT10::CHAN_BOND_MODE => {
                    ctx.collect_bel_attr_default(tcid, bslot, aid, enums::GT_CHAN_BOND_MODE::NONE);
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

        let mut pma_reg =
            ctx.extract_bel_special_bitvec(tcid, bslot, specials::GT10_PMA_SPEED_HEX, 120);
        pma_reg.extend(ctx.extract_bel_special_bitvec(
            tcid,
            bslot,
            specials::GT10_PMA_PWR_CNTRL,
            8,
        ));
        let attrs = pma_reg_attrs();
        for &(attr, _field, ref bits) in &attrs {
            let bits = Vec::from_iter(bits.iter().map(|&idx| pma_reg[idx]));
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr, bits);
        }
        for row in ctx.edev.db.tables[tables::GT10_PMA_SPEED].rows.ids() {
            let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, specials::GT10_PMA_SPEED, row);
            for &(_attr, field, ref bits) in &attrs {
                let Some(field) = field else {
                    continue;
                };
                let bits = Vec::from_iter(bits.iter().map(|&idx| pma_reg[idx]));
                let val =
                    extract_bitvec_val_part(&bits, &BitVec::repeat(false, bits.len()), &mut diff);
                ctx.insert_table_bitvec(tables::GT10_PMA_SPEED, row, field, val);
            }
            diff.assert_empty();
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::GT10::PMA_REG, pma_reg);
    }
}
