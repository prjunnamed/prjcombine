use prjcombine_entity::EntityId;
use prjcombine_re_collector::diff::{
    Diff, SpecialId, extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_wide_bi,
    xlat_bitvec, xlat_enum_attr,
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bits;
use prjcombine_virtex2::{
    chip::ChipKind,
    defs::{
        self, bcls, bslots, devdata, enums, spartan3::tcls as tcls_s3, virtex2::tcls as tcls_v2,
    },
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzBuilderBel, FuzzCtx},
    virtex2::specials,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let chip_kind = match backend.edev {
        ExpandedDevice::Virtex2(edev) => edev.chip.kind,
        _ => unreachable!(),
    };
    let tcid = match chip_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => tcls_v2::BRAM,
        ChipKind::Spartan3 => tcls_s3::BRAM_S3,
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => tcls_s3::BRAM_S3E,
        ChipKind::Spartan3A => tcls_s3::BRAM_S3A,
        ChipKind::Spartan3ADsp => tcls_s3::BRAM_S3ADSP,
    };
    let mut ctx = FuzzCtx::new(session, backend, tcid);
    let mut bctx = ctx.bel(defs::bslots::BRAM);
    let mode = match chip_kind {
        ChipKind::Spartan3ADsp => "RAMB16BWER",
        ChipKind::Spartan3A => "RAMB16BWE",
        _ => "RAMB16",
    };
    let test_present = |builder: FuzzBuilderBel, spec: SpecialId| match chip_kind {
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
            builder
                .global_mutex("BRAM_OPTS", spec)
                .test_bel_special(spec)
                .mode(mode)
                .attr("DATA_WIDTH_A", "36")
                .attr("DATA_WIDTH_B", "36")
                .attr("SRVAL_A", "fffffffff")
                .attr("SRVAL_B", "fffffffff")
                .attr("INIT_A", "fffffffff")
                .attr("INIT_B", "fffffffff")
                .commit();
        }
        _ => {
            builder
                .global_mutex("BRAM_OPTS", spec)
                .test_bel_special(spec)
                .mode(mode)
                .attr("PORTA_ATTR", "512X36")
                .attr("PORTB_ATTR", "512X36")
                .attr("SRVAL_A", "fffffffff")
                .attr("SRVAL_B", "fffffffff")
                .attr("INIT_A", "fffffffff")
                .attr("INIT_B", "fffffffff")
                .commit();
        }
    };
    if devdata_only {
        if !chip_kind.is_virtex2() {
            test_present(bctx.build(), specials::PRESENT);
            test_present(
                bctx.build()
                    .global("Ibram_ddel0", "0")
                    .global("Ibram_ddel1", "0")
                    .global("Ibram_wdel0", "0")
                    .global("Ibram_wdel1", "0")
                    .global("Ibram_wdel2", "0"),
                specials::BRAM_PRESENT_ALL_0,
            );
        }
        return;
    }
    match chip_kind {
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
            for pin in [
                bcls::BRAM::CLKA,
                bcls::BRAM::CLKB,
                bcls::BRAM::ENA,
                bcls::BRAM::ENB,
            ]
            .into_iter()
            .chain(bcls::BRAM::WEA)
            .chain(bcls::BRAM::WEB)
            {
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_bel_input_inv_auto(pin);
            }
            if chip_kind == ChipKind::Spartan3ADsp {
                for pin in [
                    bcls::BRAM::RSTA,
                    bcls::BRAM::RSTB,
                    bcls::BRAM::REGCEA,
                    bcls::BRAM::REGCEB,
                ] {
                    bctx.mode(mode)
                        .attr("DATA_WIDTH_A", "36")
                        .attr("DATA_WIDTH_B", "36")
                        .test_bel_input_inv_auto(pin);
                }
            } else {
                for (pin, pname) in [(bcls::BRAM::RSTA, "SSRA"), (bcls::BRAM::RSTB, "SSRB")] {
                    bctx.mode(mode)
                        .attr("DATA_WIDTH_A", "36")
                        .attr("DATA_WIDTH_B", "36")
                        .pin(pname)
                        .test_bel_input_inv_enum(
                            format!("{pname}INV"),
                            pin,
                            pname,
                            format!("{pname}_B"),
                        );
                }
            }
            for attr in [bcls::BRAM::DATA_WIDTH_A, bcls::BRAM::DATA_WIDTH_B] {
                bctx.mode(mode)
                    .attr("INIT_A", "0")
                    .attr("INIT_B", "0")
                    .attr("SRVAL_A", "0")
                    .attr("SRVAL_B", "0")
                    .test_bel_attr(attr);
                let aname = backend.edev.db.bel_classes[bcls::BRAM].attributes.key(attr);
                bctx.mode(mode)
                    .null_bits()
                    .attr("INIT_A", "0")
                    .attr("INIT_B", "0")
                    .attr("SRVAL_A", "0")
                    .attr("SRVAL_B", "0")
                    .test_bel_attr_special(attr, specials::BRAM_DATA_WIDTH_0)
                    .attr(aname, "0")
                    .commit();
            }
            for attr in [bcls::BRAM::WRITE_MODE_A, bcls::BRAM::WRITE_MODE_B] {
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_bel_attr(attr);
            }
            if chip_kind == ChipKind::Spartan3ADsp {
                bctx.mode(mode)
                    .test_bel_attr_rename("RSTTYPE", bcls::BRAM::RSTTYPE_A);
                bctx.mode(mode)
                    .test_bel_attr_bool_auto(bcls::BRAM::DOA_REG, "0", "1");
                bctx.mode(mode)
                    .test_bel_attr_bool_auto(bcls::BRAM::DOB_REG, "0", "1");
            }
            for attr in [
                bcls::BRAM::INIT_A,
                bcls::BRAM::INIT_B,
                bcls::BRAM::SRVAL_A,
                bcls::BRAM::SRVAL_B,
            ] {
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_bel_attr_multi(attr, MultiValue::Hex(0));
            }
            for i in 0..0x40 {
                let attr = format!("INIT_{i:02X}");
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_bel_attr_bits_base(bcls::BRAM::DATA, i * 0x100)
                    .multi_attr(attr, MultiValue::Hex(0), 0x100);
            }
            for i in 0..0x8 {
                let attr = format!("INITP_{i:02X}");
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_bel_attr_bits_base(bcls::BRAM::DATAP, i * 0x100)
                    .multi_attr(attr, MultiValue::Hex(0), 0x100);
            }
        }
        _ => {
            for (pin, pname) in [
                (bcls::BRAM::CLKA, "CLKA"),
                (bcls::BRAM::CLKB, "CLKB"),
                (bcls::BRAM::RSTA, "SSRA"),
                (bcls::BRAM::RSTB, "SSRB"),
                (bcls::BRAM::WEA[0], "WEA"),
                (bcls::BRAM::WEB[0], "WEB"),
                (bcls::BRAM::ENA, "ENA"),
                (bcls::BRAM::ENB, "ENB"),
            ] {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .pin(pname)
                    .test_bel_input_inv_enum(
                        format!("{pname}INV"),
                        pin,
                        pname,
                        format!("{pname}_B"),
                    );
            }
            for (attr, aname) in [
                (bcls::BRAM::DATA_WIDTH_A, "PORTA_ATTR"),
                (bcls::BRAM::DATA_WIDTH_B, "PORTB_ATTR"),
            ] {
                for (val, vname) in [
                    (enums::BRAM_DATA_WIDTH::_1, "16384X1"),
                    (enums::BRAM_DATA_WIDTH::_2, "8192X2"),
                    (enums::BRAM_DATA_WIDTH::_4, "4096X4"),
                    (enums::BRAM_DATA_WIDTH::_9, "2048X9"),
                    (enums::BRAM_DATA_WIDTH::_18, "1024X18"),
                    (enums::BRAM_DATA_WIDTH::_36, "512X36"),
                ] {
                    bctx.mode(mode)
                        .attr("INIT_A", "0")
                        .attr("INIT_B", "0")
                        .attr("SRVAL_A", "0")
                        .attr("SRVAL_B", "0")
                        .test_bel_attr_val(attr, val)
                        .attr(aname, vname)
                        .commit();
                }
            }
            for (attr, aname) in [
                (bcls::BRAM::WRITE_MODE_A, "WRITEMODEA"),
                (bcls::BRAM::WRITE_MODE_B, "WRITEMODEB"),
            ] {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_bel_attr_rename(aname, attr);
            }
            if chip_kind.is_virtex2() {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_bel_attr_bool_auto(bcls::BRAM::SAVEDATA, "FALSE", "TRUE");
            }
            for attr in [
                bcls::BRAM::INIT_A,
                bcls::BRAM::INIT_B,
                bcls::BRAM::SRVAL_A,
                bcls::BRAM::SRVAL_B,
            ] {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_bel_attr_multi(attr, MultiValue::Hex(0));
            }
            for i in 0..0x40 {
                let attr = format!("INIT_{i:02x}");
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_bel_attr_bits_base(bcls::BRAM::DATA, i * 0x100)
                    .multi_attr(attr, MultiValue::Hex(0), 0x100);
            }
            for i in 0..0x8 {
                let attr = format!("INITP_{i:02x}");
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_bel_attr_bits_base(bcls::BRAM::DATAP, i * 0x100)
                    .multi_attr(attr, MultiValue::Hex(0), 0x100);
            }
        }
    }
    test_present(bctx.build(), specials::PRESENT);
    if !chip_kind.is_virtex2() {
        test_present(
            bctx.build()
                .global("Ibram_ddel0", "0")
                .global("Ibram_ddel1", "0"),
            specials::BRAM_DDEL_00,
        );
        test_present(
            bctx.build()
                .global("Ibram_ddel0", "1")
                .global("Ibram_ddel1", "0"),
            specials::BRAM_DDEL_01,
        );
        test_present(
            bctx.build()
                .global("Ibram_ddel0", "0")
                .global("Ibram_ddel1", "1"),
            specials::BRAM_DDEL_10,
        );
        test_present(
            bctx.build()
                .global("Ibram_wdel0", "0")
                .global("Ibram_wdel1", "0")
                .global("Ibram_wdel2", "0"),
            specials::BRAM_WDEL_000,
        );
        test_present(
            bctx.build()
                .global("Ibram_wdel0", "1")
                .global("Ibram_wdel1", "0")
                .global("Ibram_wdel2", "0"),
            specials::BRAM_WDEL_001,
        );
        test_present(
            bctx.build()
                .global("Ibram_wdel0", "0")
                .global("Ibram_wdel1", "1")
                .global("Ibram_wdel2", "0"),
            specials::BRAM_WDEL_010,
        );
        test_present(
            bctx.build()
                .global("Ibram_wdel0", "0")
                .global("Ibram_wdel1", "0")
                .global("Ibram_wdel2", "1"),
            specials::BRAM_WDEL_100,
        );
        test_present(
            bctx.build().global("Ibram_ww_value", "0"),
            specials::BRAM_WW_VALUE_0,
        );
        test_present(
            bctx.build().global("Ibram_ww_value", "1"),
            specials::BRAM_WW_VALUE_1,
        );
    }
    if chip_kind != ChipKind::Spartan3ADsp {
        // mult
        let mut bctx = ctx.bel(defs::bslots::MULT);
        let mode = if matches!(chip_kind, ChipKind::Spartan3E | ChipKind::Spartan3A) {
            "MULT18X18SIO"
        } else {
            "MULT18X18"
        };
        bctx.build()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        if !matches!(chip_kind, ChipKind::Spartan3E | ChipKind::Spartan3A) {
            bctx.mode(mode).test_bel_input_inv_auto(bcls::MULT::CLK);
            for (pin, pname) in [(bcls::MULT::RSTP, "RST"), (bcls::MULT::CEP, "CE")] {
                bctx.mode(mode).pin(pname).test_bel_input_inv_enum(
                    format!("{pname}INV"),
                    pin,
                    pname,
                    format!("{pname}_B"),
                );
            }
        } else {
            for pin in [
                bcls::MULT::CLK,
                bcls::MULT::RSTP,
                bcls::MULT::CEP,
                bcls::MULT::RSTA,
                bcls::MULT::CEA,
                bcls::MULT::RSTB,
                bcls::MULT::CEB,
            ] {
                bctx.mode(mode).test_bel_input_inv_auto(pin);
            }
            for attr in [
                bcls::MULT::AREG,
                bcls::MULT::BREG,
                bcls::MULT::PREG,
                bcls::MULT::PREG_CLKINVERSION,
            ] {
                bctx.mode(mode).test_bel_attr_bool_auto(attr, "0", "1");
            }
            bctx.mode(mode).test_bel_attr(bcls::MULT::B_INPUT);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let chip_kind = match ctx.edev {
        ExpandedDevice::Virtex2(edev) => edev.chip.kind,
        _ => unreachable!(),
    };
    let int_tiles = match chip_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => &[tcls_v2::INT_BRAM; 4],
        ChipKind::Spartan3 => &[tcls_s3::INT_BRAM_S3; 4],
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => &[tcls_s3::INT_BRAM_S3E; 4],
        ChipKind::Spartan3A => &[
            tcls_s3::INT_BRAM_S3A_03,
            tcls_s3::INT_BRAM_S3A_12,
            tcls_s3::INT_BRAM_S3A_12,
            tcls_s3::INT_BRAM_S3A_03,
        ],
        ChipKind::Spartan3ADsp => &[tcls_s3::INT_BRAM_S3ADSP; 4],
    };
    let tcid = match chip_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => tcls_v2::BRAM,
        ChipKind::Spartan3 => tcls_s3::BRAM_S3,
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => tcls_s3::BRAM_S3E,
        ChipKind::Spartan3A => tcls_s3::BRAM_S3A,
        ChipKind::Spartan3ADsp => tcls_s3::BRAM_S3ADSP,
    };
    let bslot = bslots::BRAM;
    fn filter_ab(diff: Diff) -> (Diff, Diff) {
        let mut a = diff;
        let b = a.split_bits_by(|bit| bit.rect.to_idx() >= 2);
        (a, b)
    }
    if devdata_only {
        if !chip_kind.is_virtex2() {
            let present_base = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
            let all_0 = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_PRESENT_ALL_0);
            let mut diff = present_base.combine(&!all_0);
            let adef = extract_bitvec_val_part(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::BRAM::DDEL_A),
                &bits![0, 0],
                &mut diff,
            );
            ctx.insert_devdata_bitvec(devdata::BRAM_DDEL_A, adef);
            if chip_kind != ChipKind::Spartan3 {
                let bdef = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::BRAM::DDEL_B),
                    &bits![0, 0],
                    &mut diff,
                );
                ctx.insert_devdata_bitvec(devdata::BRAM_DDEL_B, bdef);
            }

            let adef = extract_bitvec_val_part(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::BRAM::WDEL_A),
                &bits![0, 0, 0],
                &mut diff,
            );
            ctx.insert_devdata_bitvec(devdata::BRAM_WDEL_A, adef);
            if chip_kind != ChipKind::Spartan3 {
                let bdef = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::BRAM::WDEL_B),
                    &bits![0, 0, 0],
                    &mut diff,
                );
                ctx.insert_devdata_bitvec(devdata::BRAM_WDEL_B, bdef);
            }
            diff.assert_empty();
        }
        return;
    }
    let present_base = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
    let mut present = present_base.clone();
    if !chip_kind.is_virtex2() {
        let diff_base = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_DDEL_00);
        let diff0 = ctx
            .get_diff_bel_special(tcid, bslot, specials::BRAM_DDEL_01)
            .combine(&!&diff_base);
        let diff1 = ctx
            .get_diff_bel_special(tcid, bslot, specials::BRAM_DDEL_10)
            .combine(&!&diff_base);
        let diff_def = present_base.combine(&!diff_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        let (adef, bdef) = filter_ab(diff_def);
        let ddel_a = xlat_bitvec(vec![a0, a1]);
        let adef = extract_bitvec_val(&ddel_a, &bits![0, 0], adef);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::BRAM::DDEL_A, ddel_a);
        ctx.insert_devdata_bitvec(devdata::BRAM_DDEL_A, adef);
        present.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, bcls::BRAM::DDEL_A));
        if chip_kind == ChipKind::Spartan3 {
            b0.assert_empty();
            b1.assert_empty();
            bdef.assert_empty();
        } else {
            let ddel_b = xlat_bitvec(vec![b0, b1]);
            let bdef = extract_bitvec_val(&ddel_b, &bits![0, 0], bdef);
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::BRAM::DDEL_B, ddel_b);
            ctx.insert_devdata_bitvec(devdata::BRAM_DDEL_B, bdef);
            present.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, bcls::BRAM::DDEL_B));
        }

        let diff_base = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_WDEL_000);
        let diff0 = ctx
            .get_diff_bel_special(tcid, bslot, specials::BRAM_WDEL_001)
            .combine(&!&diff_base);
        let diff1 = ctx
            .get_diff_bel_special(tcid, bslot, specials::BRAM_WDEL_010)
            .combine(&!&diff_base);
        let diff2 = ctx
            .get_diff_bel_special(tcid, bslot, specials::BRAM_WDEL_100)
            .combine(&!&diff_base);
        let diff_def = present_base.combine(&!diff_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        let (a2, b2) = filter_ab(diff2);
        let (adef, bdef) = filter_ab(diff_def);
        let wdel_a = xlat_bitvec(vec![a0, a1, a2]);
        let adef = extract_bitvec_val(&wdel_a, &bits![0, 0, 0], adef);
        ctx.insert_devdata_bitvec(devdata::BRAM_WDEL_A, adef);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::BRAM::WDEL_A, wdel_a);
        present.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, bcls::BRAM::WDEL_A));
        if chip_kind == ChipKind::Spartan3 {
            b0.assert_empty();
            b1.assert_empty();
            b2.assert_empty();
            bdef.assert_empty();
        } else {
            let wdel_b = xlat_bitvec(vec![b0, b1, b2]);
            let bdef = extract_bitvec_val(&wdel_b, &bits![0, 0, 0], bdef);
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::BRAM::WDEL_B, wdel_b);
            ctx.insert_devdata_bitvec(devdata::BRAM_WDEL_B, bdef);
            present.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, bcls::BRAM::WDEL_B));
        }

        let diff0 = ctx
            .get_diff_bel_special(tcid, bslot, specials::BRAM_WW_VALUE_0)
            .combine(&!&present_base);
        let diff1 = ctx
            .get_diff_bel_special(tcid, bslot, specials::BRAM_WW_VALUE_1)
            .combine(&!&present_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::BRAM::WW_VALUE_A,
            xlat_enum_attr(vec![
                (enums::BRAM_WW_VALUE::NONE, Diff::default()),
                (enums::BRAM_WW_VALUE::_0, a0),
                (enums::BRAM_WW_VALUE::_1, a1),
            ]),
        );
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::BRAM::WW_VALUE_B,
            xlat_enum_attr(vec![
                (enums::BRAM_WW_VALUE::NONE, Diff::default()),
                (enums::BRAM_WW_VALUE::_0, b0),
                (enums::BRAM_WW_VALUE::_1, b1),
            ]),
        );
    }

    ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::BRAM::CLKA);
    ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::BRAM::CLKB);
    ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::BRAM::ENA);
    ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::BRAM::ENB);
    ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::BRAM::RSTA);
    ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::BRAM::RSTB);
    for pin in [
        bcls::BRAM::ENA,
        bcls::BRAM::ENB,
        bcls::BRAM::RSTA,
        bcls::BRAM::RSTB,
    ] {
        present.discard_bits(&[ctx.item_int_inv(int_tiles, tcid, bslot, pin).bit]);
    }
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::DATA_WIDTH_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::DATA_WIDTH_B);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::INIT_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::INIT_B);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::SRVAL_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::SRVAL_B);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::WRITE_MODE_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::WRITE_MODE_B);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::DATA);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM::DATAP);

    match chip_kind {
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
            for pin in bcls::BRAM::WEA.into_iter().chain(bcls::BRAM::WEB) {
                ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, pin);
                present.discard_bits(&[ctx.item_int_inv(int_tiles, tcid, bslot, pin).bit]);
            }
            if chip_kind == ChipKind::Spartan3ADsp {
                for pin in [bcls::BRAM::REGCEA, bcls::BRAM::REGCEB] {
                    ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, pin);
                    present.discard_bits(&[ctx.item_int_inv(int_tiles, tcid, bslot, pin).bit]);
                }

                ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM::DOA_REG);
                ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM::DOB_REG);
                let mut diffs_a = vec![];
                let mut diffs_b = vec![];
                for val in ctx.edev.db.enum_classes[enums::BRAM_RSTTYPE].values.ids() {
                    let diff = ctx.get_diff_attr_val(tcid, bslot, bcls::BRAM::RSTTYPE_A, val);
                    let (diff_a, diff_b) = filter_ab(diff);
                    diffs_a.push((val, diff_a));
                    diffs_b.push((val, diff_b));
                }
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::BRAM::RSTTYPE_A,
                    xlat_enum_attr(diffs_a),
                );
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::BRAM::RSTTYPE_B,
                    xlat_enum_attr(diffs_b),
                );
            }
        }
        _ => {
            for pin in [bcls::BRAM::WEA[0], bcls::BRAM::WEB[0]] {
                ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, pin);
                present.discard_bits(&[ctx.item_int_inv(int_tiles, tcid, bslot, pin).bit]);
            }
            if chip_kind.is_virtex2() {
                let diff0 = ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::BRAM::SAVEDATA, false);
                let diff1 = ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::BRAM::SAVEDATA, true);
                let bits = xlat_bit_wide_bi(diff0, diff1);
                ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::BRAM::SAVEDATA, bits);
            }
        }
    }
    present.discard_bits(
        &ctx.bel_attr_enum(tcid, bslot, bcls::BRAM::DATA_WIDTH_A)
            .bits,
    );
    present.discard_bits(
        &ctx.bel_attr_enum(tcid, bslot, bcls::BRAM::DATA_WIDTH_B)
            .bits,
    );
    if chip_kind.is_spartan3a() {
        let (diff_a, diff_b) = filter_ab(present);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::BRAM::ENABLE_A, xlat_bit(diff_a));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::BRAM::ENABLE_B, xlat_bit(diff_b));
    } else {
        present.assert_empty();
    }

    if chip_kind != ChipKind::Spartan3ADsp {
        let bslot = bslots::MULT;
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        if chip_kind.is_virtex2() || chip_kind == ChipKind::Spartan3 {
            let f_clk = ctx.get_diff_bel_input_inv(tcid, bslot, bcls::MULT::CLK, false);
            let f_clk_b = ctx.get_diff_bel_input_inv(tcid, bslot, bcls::MULT::CLK, true);
            let (f_clk, f_clk_b, f_reg) = Diff::split(f_clk, f_clk_b);
            f_clk.assert_empty();
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::MULT::PREG, xlat_bit(f_reg));
            ctx.insert_bel_input_inv_int(
                int_tiles,
                tcid,
                bslot,
                bcls::MULT::CLK,
                xlat_bit(f_clk_b),
            );
            ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::MULT::CEP);
            ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::MULT::RSTP);
            present.discard_bits(&[ctx
                .item_int_inv(int_tiles, tcid, bslot, bcls::MULT::CEP)
                .bit]);
        } else {
            for pin in [
                bcls::MULT::CLK,
                bcls::MULT::RSTP,
                bcls::MULT::CEP,
                bcls::MULT::RSTA,
                bcls::MULT::CEA,
                bcls::MULT::RSTB,
                bcls::MULT::CEB,
            ] {
                ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, pin);
            }
            for attr in [
                bcls::MULT::AREG,
                bcls::MULT::BREG,
                bcls::MULT::PREG,
                bcls::MULT::PREG_CLKINVERSION,
            ] {
                ctx.collect_bel_attr_bi(tcid, bslot, attr);
            }
            ctx.collect_bel_attr(tcid, bslot, bcls::MULT::B_INPUT);
            present.discard_bits(&[ctx
                .bel_attr_bit(tcid, bslot, bcls::MULT::PREG_CLKINVERSION)
                .bit]);
            present.discard_bits(&[ctx
                .item_int_inv(int_tiles, tcid, bslot, bcls::MULT::CEA)
                .bit]);
            present.discard_bits(&[ctx
                .item_int_inv(int_tiles, tcid, bslot, bcls::MULT::CEB)
                .bit]);
            present.discard_bits(&[ctx
                .item_int_inv(int_tiles, tcid, bslot, bcls::MULT::CEP)
                .bit]);
        }
        present.assert_empty();
    }
}
