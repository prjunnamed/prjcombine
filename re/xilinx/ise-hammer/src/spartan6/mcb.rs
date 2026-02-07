use prjcombine_entity::EntityId;
use prjcombine_interconnect::db::BelAttributeType;
use prjcombine_re_collector::diff::{
    Diff, OcdMode, xlat_bit_bi, xlat_bitvec_sparse_u32, xlat_enum_attr, xlat_enum_attr_ocd,
};
use prjcombine_re_hammer::Session;
use prjcombine_spartan6::defs::{bcls, bslots, enums, tcls};
use prjcombine_types::bsdata::{BitRectId, TileBit};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    spartan6::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::MCB) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::MCB);
    let mode = "MCB";
    bctx.build()
        .global_mutex("MCB", "TEST")
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();
    for pin in [
        bcls::MCB::P0CMDCLK,
        bcls::MCB::P1CMDCLK,
        bcls::MCB::P2CMDCLK,
        bcls::MCB::P3CMDCLK,
        bcls::MCB::P4CMDCLK,
        bcls::MCB::P5CMDCLK,
        bcls::MCB::P0CMDEN,
        bcls::MCB::P1CMDEN,
        bcls::MCB::P2CMDEN,
        bcls::MCB::P3CMDEN,
        bcls::MCB::P4CMDEN,
        bcls::MCB::P5CMDEN,
        bcls::MCB::P0RDCLK,
        bcls::MCB::P1RDCLK,
        bcls::MCB::P0RDEN,
        bcls::MCB::P1RDEN,
        bcls::MCB::P0WRCLK,
        bcls::MCB::P1WRCLK,
        bcls::MCB::P0WREN,
        bcls::MCB::P1WREN,
        bcls::MCB::P2CLK,
        bcls::MCB::P3CLK,
        bcls::MCB::P4CLK,
        bcls::MCB::P5CLK,
        bcls::MCB::P2EN,
        bcls::MCB::P3EN,
        bcls::MCB::P4EN,
        bcls::MCB::P5EN,
    ] {
        bctx.mode(mode)
            .global_mutex("MCB", "TEST")
            .test_bel_input_inv_auto(pin);
    }
    for (aid, aname, attr) in &backend.edev.db[bcls::MCB].attributes {
        if aname.starts_with("MUI") {
            // extracted from other attributes through special handling
            continue;
        }
        match aid {
            bcls::MCB::CAL_BYPASS => {
                bctx.mode(mode)
                    .global_mutex("MCB", "TEST")
                    .test_bel_attr_bool_auto(aid, "NO", "YES");
            }
            bcls::MCB::MEM_RAS_VAL
            | bcls::MCB::MEM_RCD_VAL
            | bcls::MCB::MEM_REFI_VAL
            | bcls::MCB::MEM_RFC_VAL
            | bcls::MCB::MEM_RP_VAL
            | bcls::MCB::MEM_RTP_VAL
            | bcls::MCB::MEM_WR_VAL
            | bcls::MCB::MEM_WTR_VAL => {
                let BelAttributeType::BitVec(width) = attr.typ else {
                    unreachable!()
                };
                bctx.mode(mode)
                    .global_mutex("MCB", "TEST")
                    .test_bel_attr_bits(aid)
                    .multi_attr(aname, MultiValue::Dec(0), width);
            }
            bcls::MCB::CAL_CA | bcls::MCB::CAL_BA | bcls::MCB::CAL_RA => {
                let BelAttributeType::BitVec(width) = attr.typ else {
                    unreachable!()
                };
                bctx.mode(mode)
                    .global_mutex("MCB", "TEST")
                    .test_bel_attr_bits(aid)
                    .multi_attr(aname, MultiValue::Hex(0), width);
            }
            bcls::MCB::ARB_TIME_SLOT => {
                for i in 0..12 {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .test_bel_attr_bits_base(aid, i * 18)
                        .multi_attr(format!("ARB_TIME_SLOT_{i}"), MultiValue::Bin, 18);
                }
            }
            bcls::MCB::MR | bcls::MCB::EMR1 | bcls::MCB::EMR2 | bcls::MCB::EMR3 => {
                // hardcoded
            }
            bcls::MCB::MEM_PLL_DIV_EN => {
                bctx.mode(mode)
                    .global_mutex_here("MCB")
                    .global_mutex("DRPSDO", "NOPE")
                    .test_global_attr_bool_rename("MEM_PLL_DIV_EN", aid, "DISABLED", "ENABLED");
            }
            bcls::MCB::MEM_PLL_POL_SEL => {
                bctx.mode(mode)
                    .global_mutex_here("MCB")
                    .global_mutex("DRPSDO", "NOPE")
                    .test_global_attr_rename("MEM_PLL_POL_SEL", aid);
            }
            bcls::MCB::PORT_CONFIG => {
                bctx.mode(mode)
                    .global_mutex("MCB", "TEST")
                    .test_bel_attr_default(aid, enums::MCB_PORT_CONFIG::B32_B32_X32_X32_X32_X32);
                for mask in 0..16 {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .test_bel_attr_u32(aid, mask)
                        .attr(
                            "PORT_CONFIG",
                            format!(
                                "B32_B32_{p2}32_{p3}32_{p4}32_{p5}32",
                                p2 = if (mask & 1) != 0 { 'R' } else { 'W' },
                                p3 = if (mask & 2) != 0 { 'R' } else { 'W' },
                                p4 = if (mask & 4) != 0 { 'R' } else { 'W' },
                                p5 = if (mask & 8) != 0 { 'R' } else { 'W' },
                            ),
                        )
                        .commit();
                }
            }
            bcls::MCB::MEM_BURST_LEN => {
                for (mem_type, spec) in [
                    ("MDDR", specials::MCB_MDDR),
                    ("DDR", specials::MCB_DDR),
                    ("DDR2", specials::MCB_DDR2),
                    ("DDR3", specials::MCB_DDR3),
                ] {
                    for (val, vname) in &backend.edev.db[enums::MCB_MEM_BURST_LEN].values {
                        let vname = vname.strip_prefix('_').unwrap_or(vname);
                        if val == enums::MCB_MEM_BURST_LEN::NONE {
                            continue;
                        }
                        bctx.mode(mode)
                            .global_mutex("MCB", "TEST")
                            .attr("MEM_TYPE", mem_type)
                            .test_bel_attr_special_val(aid, spec, val)
                            .attr("MEM_BURST_LEN", vname)
                            .commit();
                    }
                }
            }
            bcls::MCB::MEM_DDR_DDR2_MDDR_BURST_LEN => {
                // derived from above
            }
            bcls::MCB::MEM_CAS_LATENCY => {
                for mt in ["DDR", "DDR2", "MDDR"] {
                    if let BelAttributeType::Enum(ecid) = attr.typ
                        && let Some(vid) = backend.edev.db[ecid].values.get("NONE")
                    {
                        bctx.mode(mode)
                            .global_mutex("MCB", "TEST")
                            .attr("MEM_TYPE", mt)
                            .test_bel_attr_default(aid, vid);
                    } else {
                        bctx.mode(mode)
                            .global_mutex("MCB", "TEST")
                            .attr("MEM_TYPE", mt)
                            .test_bel_attr(aid);
                    }
                }
            }
            // sigh. doesn't actually work for plain DDR.
            bcls::MCB::MEM_DDR1_2_ODS
            | bcls::MCB::MEM_DDR2_ADD_LATENCY
            | bcls::MCB::MEM_DDR2_DIFF_DQS_EN
            | bcls::MCB::MEM_DDR2_RTT
            | bcls::MCB::MEM_DDR2_WRT_RECOVERY => {
                let mt = "DDR2";
                if let BelAttributeType::Enum(ecid) = attr.typ
                    && let Some(vid) = backend.edev.db[ecid].values.get("NONE")
                {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .attr("MEM_TYPE", mt)
                        .test_bel_attr_default(aid, vid);
                } else {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .attr("MEM_TYPE", mt)
                        .test_bel_attr(aid);
                }
            }
            bcls::MCB::MEM_DDR2_3_HIGH_TEMP_SR | bcls::MCB::MEM_DDR2_3_PA_SR => {
                for mt in ["DDR2", "DDR3"] {
                    if let BelAttributeType::Enum(ecid) = attr.typ
                        && let Some(vid) = backend.edev.db[ecid].values.get("NONE")
                    {
                        bctx.mode(mode)
                            .global_mutex("MCB", "TEST")
                            .attr("MEM_TYPE", mt)
                            .test_bel_attr_default(aid, vid);
                    } else {
                        bctx.mode(mode)
                            .global_mutex("MCB", "TEST")
                            .attr("MEM_TYPE", mt)
                            .test_bel_attr(aid);
                    }
                }
            }
            bcls::MCB::MEM_DDR3_CAS_LATENCY
            | bcls::MCB::MEM_DDR3_WRT_RECOVERY
            | bcls::MCB::MEM_DDR3_ADD_LATENCY
            | bcls::MCB::MEM_DDR3_ODS
            | bcls::MCB::MEM_DDR3_RTT
            | bcls::MCB::MEM_DDR3_CAS_WR_LATENCY
            | bcls::MCB::MEM_DDR3_AUTO_SR
            | bcls::MCB::MEM_DDR3_DYN_WRT_ODT => {
                let mt = "DDR3";
                if let BelAttributeType::Enum(ecid) = attr.typ
                    && let Some(vid) = backend.edev.db[ecid].values.get("NONE")
                {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .attr("MEM_TYPE", mt)
                        .test_bel_attr_default(aid, vid);
                } else {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .attr("MEM_TYPE", mt)
                        .test_bel_attr(aid);
                }
            }
            bcls::MCB::MEM_MDDR_ODS | bcls::MCB::MEM_MOBILE_PA_SR | bcls::MCB::MEM_MOBILE_TC_SR => {
                let mt = "MDDR";
                if let BelAttributeType::Enum(ecid) = attr.typ
                    && let Some(vid) = backend.edev.db[ecid].values.get("NONE")
                {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .attr("MEM_TYPE", mt)
                        .test_bel_attr_default(aid, vid);
                } else {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .attr("MEM_TYPE", mt)
                        .test_bel_attr(aid);
                }
            }
            _ => {
                if let BelAttributeType::Enum(ecid) = attr.typ
                    && let Some(vid) = backend.edev.db[ecid].values.get("NONE")
                {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .test_bel_attr_default(aid, vid);
                } else {
                    bctx.mode(mode)
                        .global_mutex("MCB", "TEST")
                        .test_bel_attr(aid);
                }
            }
        }
    }
}

fn mui_split(diff: Diff) -> [Diff; 9] {
    let mut res = core::array::from_fn(|_| Diff::default());
    for (k, v) in diff.bits {
        let r = k.rect.to_idx();
        let slot = if r < 12 { 0 } else { 1 + (r - 12) / 2 };
        res[slot].bits.insert(k, v);
    }
    res
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::MCB;
    if !ctx.has_tcls(tcid) {
        return;
    }
    let bslot = bslots::MCB;

    let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);

    for pin in [
        bcls::MCB::P0CMDCLK,
        bcls::MCB::P1CMDCLK,
        bcls::MCB::P2CMDCLK,
        bcls::MCB::P3CMDCLK,
        bcls::MCB::P4CMDCLK,
        bcls::MCB::P5CMDCLK,
        bcls::MCB::P0CMDEN,
        bcls::MCB::P1CMDEN,
        bcls::MCB::P2CMDEN,
        bcls::MCB::P3CMDEN,
        bcls::MCB::P4CMDEN,
        bcls::MCB::P5CMDEN,
        bcls::MCB::P0RDCLK,
        bcls::MCB::P1RDCLK,
        bcls::MCB::P0RDEN,
        bcls::MCB::P1RDEN,
        bcls::MCB::P0WRCLK,
        bcls::MCB::P1WRCLK,
        bcls::MCB::P0WREN,
        bcls::MCB::P1WREN,
        bcls::MCB::P2CLK,
        bcls::MCB::P3CLK,
        bcls::MCB::P4CLK,
        bcls::MCB::P5CLK,
        bcls::MCB::P2EN,
        bcls::MCB::P3EN,
        bcls::MCB::P4EN,
        bcls::MCB::P5EN,
    ] {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }
    for (aid, aname, attr) in &ctx.edev.db[bcls::MCB].attributes {
        if aname.starts_with("MUI") {
            // extracted from other attributes through special handling
            continue;
        }
        match aid {
            bcls::MCB::CAL_BYPASS => {
                ctx.collect_bel_attr_bi(tcid, bslot, aid);
            }
            bcls::MCB::MR | bcls::MCB::EMR1 | bcls::MCB::EMR2 | bcls::MCB::EMR3 => {
                // hardcoded below
            }
            bcls::MCB::MEM_PLL_DIV_EN => {
                let diff0 = ctx.get_diff_attr_bool_bi(tcid, bslot, aid, false);
                let diff1 = ctx.get_diff_attr_bool_bi(tcid, bslot, aid, true);
                present = present.combine(&diff0);
                for ((attr, diff0), diff1) in [
                    bcls::MCB::MEM_PLL_DIV_EN,
                    bcls::MCB::MUI0R_MEM_PLL_DIV_EN,
                    bcls::MCB::MUI0W_MEM_PLL_DIV_EN,
                    bcls::MCB::MUI1R_MEM_PLL_DIV_EN,
                    bcls::MCB::MUI1W_MEM_PLL_DIV_EN,
                    bcls::MCB::MUI2_MEM_PLL_DIV_EN,
                    bcls::MCB::MUI3_MEM_PLL_DIV_EN,
                    bcls::MCB::MUI4_MEM_PLL_DIV_EN,
                    bcls::MCB::MUI5_MEM_PLL_DIV_EN,
                ]
                .into_iter()
                .zip(mui_split(diff0))
                .zip(mui_split(diff1))
                {
                    ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit_bi(diff0, diff1));
                }
            }
            bcls::MCB::MEM_PLL_POL_SEL => {
                let mut diffs = [const { vec![] }; 9];
                for val in ctx.edev.db[enums::MCB_MEM_PLL_POL_SEL].values.ids() {
                    let diff = ctx.get_diff_attr_val(tcid, bslot, aid, val);
                    if val == enums::MCB_MEM_PLL_POL_SEL::INVERTED {
                        present = present.combine(&diff);
                    }
                    for (i, diff) in mui_split(diff).into_iter().enumerate() {
                        diffs[i].push((val, diff));
                    }
                }
                for (attr, diffs) in [
                    bcls::MCB::MEM_PLL_POL_SEL,
                    bcls::MCB::MUI0R_MEM_PLL_POL_SEL,
                    bcls::MCB::MUI0W_MEM_PLL_POL_SEL,
                    bcls::MCB::MUI1R_MEM_PLL_POL_SEL,
                    bcls::MCB::MUI1W_MEM_PLL_POL_SEL,
                    bcls::MCB::MUI2_MEM_PLL_POL_SEL,
                    bcls::MCB::MUI3_MEM_PLL_POL_SEL,
                    bcls::MCB::MUI4_MEM_PLL_POL_SEL,
                    bcls::MCB::MUI5_MEM_PLL_POL_SEL,
                ]
                .into_iter()
                .zip(diffs)
                {
                    ctx.insert_bel_attr_enum(tcid, bslot, attr, xlat_enum_attr(diffs));
                }
            }
            bcls::MCB::MEM_WIDTH => {
                let mut diffs = [const { vec![] }; 9];
                for val in ctx.edev.db[enums::MCB_MEM_WIDTH].values.ids() {
                    let diff = if val == enums::MCB_MEM_WIDTH::NONE {
                        Diff::default()
                    } else {
                        ctx.get_diff_attr_val(tcid, bslot, aid, val)
                    };
                    for (i, diff) in mui_split(diff).into_iter().enumerate() {
                        diffs[i].push((val, diff));
                    }
                }
                for (attr, diffs) in [
                    bcls::MCB::MEM_WIDTH,
                    bcls::MCB::MUI0R_MEM_WIDTH,
                    bcls::MCB::MUI0W_MEM_WIDTH,
                    bcls::MCB::MUI1R_MEM_WIDTH,
                    bcls::MCB::MUI1W_MEM_WIDTH,
                    bcls::MCB::MUI2_MEM_WIDTH,
                    bcls::MCB::MUI3_MEM_WIDTH,
                    bcls::MCB::MUI4_MEM_WIDTH,
                    bcls::MCB::MUI5_MEM_WIDTH,
                ]
                .into_iter()
                .zip(diffs)
                {
                    ctx.insert_bel_attr_enum(tcid, bslot, attr, xlat_enum_attr(diffs));
                }
            }
            bcls::MCB::PORT_CONFIG => {
                let bits = xlat_bitvec_sparse_u32(
                    (0..16)
                        .map(|val| (val, ctx.get_diff_attr_u32(tcid, bslot, aid, val)))
                        .collect(),
                );
                assert_eq!(bits.len(), 4);
                for (attr, bit) in [
                    bcls::MCB::MUI2_PORT_CONFIG,
                    bcls::MCB::MUI3_PORT_CONFIG,
                    bcls::MCB::MUI4_PORT_CONFIG,
                    bcls::MCB::MUI5_PORT_CONFIG,
                ]
                .into_iter()
                .zip(bits.iter().copied())
                {
                    ctx.insert_bel_attr_enum(
                        tcid,
                        bslot,
                        attr,
                        xlat_enum_attr(vec![
                            (enums::MCB_MUI_PORT_CONFIG::WRITE, Diff::default()),
                            (enums::MCB_MUI_PORT_CONFIG::READ, Diff::from_bit(bit)),
                        ]),
                    );
                }

                let mut diffs = vec![(
                    enums::MCB_PORT_CONFIG::B32_B32_X32_X32_X32_X32,
                    Diff::default(),
                )];
                for val in [
                    enums::MCB_PORT_CONFIG::B32_B32_B32_B32,
                    enums::MCB_PORT_CONFIG::B64_B32_B32,
                    enums::MCB_PORT_CONFIG::B64_B64,
                    enums::MCB_PORT_CONFIG::B128,
                ] {
                    let mut diff = ctx.get_diff_attr_val(tcid, bslot, aid, val);
                    diff.apply_bitvec_diff_int(&bits, 5, 0);
                    diffs.push((val, diff));
                }
                ctx.insert_bel_attr_enum(tcid, bslot, aid, xlat_enum_attr(diffs));

                for (i, (attr, def)) in [
                    (bcls::MCB::MUI0R_PORT_CONFIG, true),
                    (bcls::MCB::MUI0W_PORT_CONFIG, false),
                    (bcls::MCB::MUI1R_PORT_CONFIG, true),
                    (bcls::MCB::MUI1W_PORT_CONFIG, false),
                ]
                .into_iter()
                .enumerate()
                {
                    let mut item = ctx
                        .bel_attr_enum(tcid, bslot, bcls::MCB::MUI2_PORT_CONFIG)
                        .clone();
                    for bit in &mut item.bits {
                        bit.rect = BitRectId::from_idx(bit.rect.to_idx() - 4 * 2 + i * 2);
                    }
                    if def {
                        present.apply_enum_diff(
                            &item,
                            enums::MCB_MUI_PORT_CONFIG::READ,
                            enums::MCB_MUI_PORT_CONFIG::WRITE,
                        );
                    }
                    ctx.insert_bel_attr_enum(tcid, bslot, attr, item);
                }
            }
            bcls::MCB::MEM_BURST_LEN => {
                let mut diffs = vec![];
                let mut diffs_mr = vec![];
                for val in ctx.edev.db[enums::MCB_MEM_BURST_LEN].values.ids() {
                    if val == enums::MCB_MEM_BURST_LEN::NONE {
                        diffs.push((val, Diff::default()));
                        diffs_mr.push((val, Diff::default()));
                        continue;
                    }
                    let diff_ddr =
                        ctx.get_diff_attr_special_val(tcid, bslot, aid, specials::MCB_DDR, val);
                    let diff_mddr =
                        ctx.get_diff_attr_special_val(tcid, bslot, aid, specials::MCB_MDDR, val);
                    let diff_ddr2 =
                        ctx.get_diff_attr_special_val(tcid, bslot, aid, specials::MCB_DDR2, val);
                    let diff_ddr3 =
                        ctx.get_diff_attr_special_val(tcid, bslot, aid, specials::MCB_DDR3, val);
                    assert_eq!(diff_ddr, diff_mddr);
                    assert_eq!(diff_ddr, diff_ddr2);
                    let diff_mr = diff_ddr.combine(&!&diff_ddr3);
                    diffs.push((val, diff_ddr3));
                    diffs_mr.push((val, diff_mr));
                }
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    aid,
                    xlat_enum_attr_ocd(diffs, OcdMode::BitOrder),
                );
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::MCB::MEM_DDR_DDR2_MDDR_BURST_LEN,
                    xlat_enum_attr_ocd(diffs_mr, OcdMode::BitOrder),
                );
            }
            bcls::MCB::MEM_DDR_DDR2_MDDR_BURST_LEN => {
                // derived above
            }
            _ => {
                if let BelAttributeType::Enum(ecid) = attr.typ {
                    if let Some(vid) = ctx.edev.db[ecid].values.get("NONE") {
                        ctx.collect_bel_attr_default_ocd(tcid, bslot, aid, vid, OcdMode::BitOrder);
                    } else {
                        ctx.collect_bel_attr_ocd(tcid, bslot, aid, OcdMode::BitOrder);
                    }
                } else {
                    ctx.collect_bel_attr(tcid, bslot, aid);
                }
            }
        }
    }

    present.assert_empty();

    for (reg, bittile) in [
        (bcls::MCB::MR, 7),
        (bcls::MCB::EMR1, 6),
        (bcls::MCB::EMR2, 5),
        (bcls::MCB::EMR3, 4),
    ] {
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            reg,
            (0..14)
                .map(|i| TileBit::new(bittile, 22, 18 + i).pos())
                .collect(),
        );
    }
}
