use prjcombine_re_fpga_hammer::diff::{Diff, xlat_bitvec_raw, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_types::{
    bits,
    bsdata::{PolTileBit, TileBit},
};
use prjcombine_xc2000::xc3000::{bcls, bslots, enums, tslots};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx, specials};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::CLB);
        for (attr, aname) in [(bcls::CLB::F, "F"), (bcls::CLB::G, "G")] {
            for (spec, i1, i2, i3, i4) in [
                (specials::CLB_LUT_ABCD, "A", "B", "C", "D"),
                (specials::CLB_LUT_ABCE, "A", "B", "C", "E"),
                (specials::CLB_LUT_AQXCD, "A", "QX", "C", "D"),
                (specials::CLB_LUT_AQYCD, "A", "QY", "C", "D"),
                (specials::CLB_LUT_ABQXD, "A", "B", "QX", "D"),
                (specials::CLB_LUT_ABQYD, "A", "B", "QY", "D"),
            ] {
                bctx.mode("FG")
                    .test_bel_attr_special(attr, spec)
                    .cfg(aname, i1)
                    .cfg(aname, i2)
                    .cfg(aname, i3)
                    .cfg(aname, i4)
                    .commit();
            }
            for i in 0..16 {
                let mut bits = bits![0; 16];
                bits.set(i, true);
                bctx.mode("FG")
                    .test_bel_attr_special_bit(attr, specials::CLB_LUT_EQ_ABCD, i)
                    .equate_fixed(aname, &["A", "B", "C", "D"], bits)
                    .commit();
            }
            for (spec, inps) in [
                (specials::CLB_LUT_EQ_ABCE, &["A", "B", "C", "E"]),
                (specials::CLB_LUT_EQ_AQXCD, &["A", "QX", "C", "D"]),
                (specials::CLB_LUT_EQ_AQYCD, &["A", "QY", "C", "D"]),
                (specials::CLB_LUT_EQ_ABQXD, &["A", "B", "QX", "D"]),
                (specials::CLB_LUT_EQ_ABQYD, &["A", "B", "QY", "D"]),
            ] {
                for i in [0, 1, 2, 4, 8] {
                    let mut bits = bits![0; 16];
                    bits.set(i, true);
                    bctx.mode("FG")
                        .test_bel_attr_special_bit(attr, spec, i)
                        .equate_fixed(aname, inps, bits)
                        .commit();
                }
            }
        }
        bctx.mode("F")
            .test_bel_attr_special(bcls::CLB::F, specials::CLB_LUT_ABCDE)
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("F", "E")
            .commit();
        for i in [0, 1, 2, 4, 8, 16] {
            let mut bits = bits![0; 32];
            bits.set(i, true);
            bctx.mode("F")
                .test_bel_attr_special_bit(bcls::CLB::F, specials::CLB_LUT_EQ_ABCDE, i)
                .equate_fixed("F", &["A", "B", "C", "D", "E"], bits)
                .commit();
        }
        bctx.mode("FG").test_bel_attr_as("X", bcls::CLB::MUX_X);
        bctx.mode("FG").test_bel_attr_as("Y", bcls::CLB::MUX_Y);
        bctx.mode("FG").test_bel_attr_as("DX", bcls::CLB::MUX_DX);
        bctx.mode("FG").test_bel_attr_as("DY", bcls::CLB::MUX_DY);
        bctx.mode("FGM")
            .mutex("FGM", "FG")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .mutex("X", "M")
            .test_bel_attr_special(bcls::CLB::MUX_X, specials::CLB_M)
            .cfg("X", "M")
            .commit();
        bctx.mode("FGM")
            .mutex("FGM", "FG")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .mutex("Y", "M")
            .test_bel_attr_special(bcls::CLB::MUX_Y, specials::CLB_M)
            .cfg("Y", "M")
            .commit();
        bctx.mode("FGM")
            .mutex("FGM", "FG")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .mutex("DX", "M")
            .test_bel_attr_special(bcls::CLB::MUX_DX, specials::CLB_M)
            .cfg("DX", "M")
            .commit();
        bctx.mode("FGM")
            .mutex("FGM", "FG")
            .cfg("F", "A")
            .cfg("F", "B")
            .cfg("F", "C")
            .cfg("F", "D")
            .cfg("G", "A")
            .cfg("G", "B")
            .cfg("G", "C")
            .cfg("G", "D")
            .mutex("DY", "M")
            .test_bel_attr_special(bcls::CLB::MUX_DY, specials::CLB_M)
            .cfg("DY", "M")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_bits(bcls::CLB::EC_ENABLE)
            .cfg("ENCLK", "EC")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_bits(bcls::CLB::RD_ENABLE)
            .cfg("RSTDIR", "RD")
            .commit();
        bctx.mode("FG")
            .null_bits()
            .test_bel_special(specials::CLB_CLK)
            .cfg("CLK", "K")
            .commit();
        bctx.mode("FG")
            .cfg("CLK", "K")
            .test_bel_input_inv(bcls::CLB::K, true)
            .cfg("CLK", "NOT")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        if !ctx.has_tile(tcid) {
            continue;
        }
        let item = xlat_enum_attr(vec![
            (
                enums::CLB_MUX_X::QX,
                ctx.get_diff_attr_val(tcid, bslots::CLB, bcls::CLB::MUX_X, enums::CLB_MUX_X::QX),
            ),
            (
                enums::CLB_MUX_X::F,
                ctx.get_diff_attr_val(tcid, bslots::CLB, bcls::CLB::MUX_X, enums::CLB_MUX_X::F),
            ),
            (
                enums::CLB_MUX_X::F,
                ctx.get_diff_attr_special(tcid, bslots::CLB, bcls::CLB::MUX_X, specials::CLB_M),
            ),
        ]);
        ctx.insert_bel_attr_raw(tcid, bslots::CLB, bcls::CLB::MUX_X, item);
        let item = xlat_enum_attr(vec![
            (
                enums::CLB_MUX_Y::QY,
                ctx.get_diff_attr_val(tcid, bslots::CLB, bcls::CLB::MUX_Y, enums::CLB_MUX_Y::QY),
            ),
            (
                enums::CLB_MUX_Y::G,
                ctx.get_diff_attr_val(tcid, bslots::CLB, bcls::CLB::MUX_Y, enums::CLB_MUX_Y::G),
            ),
            (
                enums::CLB_MUX_Y::G,
                ctx.get_diff_attr_special(tcid, bslots::CLB, bcls::CLB::MUX_Y, specials::CLB_M),
            ),
        ]);
        ctx.insert_bel_attr_raw(tcid, bslots::CLB, bcls::CLB::MUX_Y, item);
        for attr in [bcls::CLB::MUX_DX, bcls::CLB::MUX_DY] {
            let item = xlat_enum_attr(vec![
                (
                    enums::CLB_MUX_D::DI,
                    ctx.get_diff_attr_val(tcid, bslots::CLB, attr, enums::CLB_MUX_D::DI),
                ),
                (
                    enums::CLB_MUX_D::F,
                    ctx.get_diff_attr_val(tcid, bslots::CLB, attr, enums::CLB_MUX_D::F),
                ),
                (
                    enums::CLB_MUX_D::G,
                    ctx.get_diff_attr_val(tcid, bslots::CLB, attr, enums::CLB_MUX_D::G),
                ),
                (
                    enums::CLB_MUX_D::F,
                    ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_M),
                ),
            ]);
            ctx.insert_bel_attr_raw(tcid, bslots::CLB, attr, item);
        }
        ctx.collect_bel_attr(tcid, bslots::CLB, bcls::CLB::EC_ENABLE);
        ctx.collect_bel_attr(tcid, bslots::CLB, bcls::CLB::RD_ENABLE);
        ctx.collect_bel_input_inv(tcid, bslots::CLB, bcls::CLB::K);
        for (attr, attr_i2, attr_i3, attr_i4) in [
            (
                bcls::CLB::F,
                bcls::CLB::MUX_F2,
                bcls::CLB::MUX_F3,
                bcls::CLB::MUX_F4,
            ),
            (
                bcls::CLB::G,
                bcls::CLB::MUX_G2,
                bcls::CLB::MUX_G3,
                bcls::CLB::MUX_G4,
            ),
        ] {
            let diff_abcd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ABCD);
            let diff_abce =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ABCE);
            let diff_abqxd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ABQXD);
            let diff_abqyd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ABQYD);
            let diff_aqxcd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_AQXCD);
            let diff_aqycd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_AQYCD);
            let mut lut_diffs = vec![];
            for i in 0..16 {
                let diff = ctx
                    .get_diff_attr_special_bit(
                        tcid,
                        bslots::CLB,
                        attr,
                        specials::CLB_LUT_EQ_ABCD,
                        i,
                    )
                    .combine(&!&diff_abcd);
                lut_diffs.push(diff);
            }
            let diff_i4_e = diff_abce.combine(&!&diff_abcd);
            ctx.insert_bel_attr_raw(
                tcid,
                bslots::CLB,
                attr_i4,
                xlat_enum_attr(vec![
                    (enums::CLB_MUX_I4::D, Diff::default()),
                    (enums::CLB_MUX_I4::E, diff_i4_e),
                ]),
            );
            let diff_i3_qx = diff_abqxd.combine(&!&diff_abcd);
            let diff_i3_qy = diff_abqyd.combine(&!&diff_abcd);
            ctx.insert_bel_attr_raw(
                tcid,
                bslots::CLB,
                attr_i3,
                xlat_enum_attr(vec![
                    (enums::CLB_MUX_I3::C, Diff::default()),
                    (enums::CLB_MUX_I3::QX, diff_i3_qx),
                    (enums::CLB_MUX_I3::QY, diff_i3_qy),
                ]),
            );
            let diff_i2_qx = diff_aqxcd.combine(&!&diff_abcd);
            let diff_i2_qy = diff_aqycd.combine(&!&diff_abcd);
            ctx.insert_bel_attr_raw(
                tcid,
                bslots::CLB,
                attr_i2,
                xlat_enum_attr(vec![
                    (enums::CLB_MUX_I2::B, Diff::default()),
                    (enums::CLB_MUX_I2::QX, diff_i2_qx),
                    (enums::CLB_MUX_I2::QY, diff_i2_qy),
                ]),
            );
            for (spec, base) in [
                (specials::CLB_LUT_EQ_ABQXD, diff_abqxd),
                (specials::CLB_LUT_EQ_ABQYD, diff_abqyd),
                (specials::CLB_LUT_EQ_AQXCD, diff_aqxcd),
                (specials::CLB_LUT_EQ_AQYCD, diff_aqycd),
                (specials::CLB_LUT_EQ_ABCE, diff_abce),
            ] {
                for i in [0, 1, 2, 4, 8] {
                    let diff = ctx
                        .get_diff_attr_special_bit(tcid, bslots::CLB, attr, spec, i)
                        .combine(&!&base);
                    assert_eq!(lut_diffs[i], diff);
                }
            }
            ctx.insert_bel_attr_bitvec(tcid, bslots::CLB, attr, xlat_bitvec_raw(lut_diffs));
            let mut diff = diff_abcd;
            let mux_i2 = ctx.bel_attr_enum(tcid, bslots::CLB, attr_i2);
            let mux_i3 = ctx.bel_attr_enum(tcid, bslots::CLB, attr_i3);
            let mux_i4 = ctx.bel_attr_enum(tcid, bslots::CLB, attr_i4);
            diff.apply_enum_diff_attr(mux_i2, enums::CLB_MUX_I2::B, enums::CLB_MUX_I2::QY);
            diff.apply_enum_diff_attr(mux_i3, enums::CLB_MUX_I3::C, enums::CLB_MUX_I3::QY);
            diff.apply_enum_diff_attr(mux_i4, enums::CLB_MUX_I4::D, enums::CLB_MUX_I4::E);
            diff.assert_empty();
        }
        let diff_abcde =
            ctx.get_diff_attr_special(tcid, bslots::CLB, bcls::CLB::F, specials::CLB_LUT_ABCDE);
        for i in [0, 1, 2, 4, 8, 16] {
            let mut bits = bits![0; 32];
            bits.set(i, true);
            let mut diff = ctx.get_diff_attr_special_bit(
                tcid,
                bslots::CLB,
                bcls::CLB::F,
                specials::CLB_LUT_EQ_ABCDE,
                i,
            );
            diff = diff.combine(&!&diff_abcde);
            diff.apply_bitvec_diff_raw(
                ctx.bel_attr_bitvec(tcid, bslots::CLB, bcls::CLB::F),
                &bits.slice(..16),
                &bits![0; 16],
            );
            diff.apply_bitvec_diff_raw(
                ctx.bel_attr_bitvec(tcid, bslots::CLB, bcls::CLB::G),
                &bits.slice(16..),
                &bits![0; 16],
            );
            diff.assert_empty();
        }
        let mut diff = diff_abcde;
        diff.apply_enum_diff_attr(
            ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_F2),
            enums::CLB_MUX_I2::B,
            enums::CLB_MUX_I2::QY,
        );
        diff.apply_enum_diff_attr(
            ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_F3),
            enums::CLB_MUX_I3::C,
            enums::CLB_MUX_I3::QY,
        );
        diff.apply_enum_diff_attr(
            ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_F4),
            enums::CLB_MUX_I4::D,
            enums::CLB_MUX_I4::E,
        );
        diff.apply_enum_diff_attr(
            ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_G2),
            enums::CLB_MUX_I2::B,
            enums::CLB_MUX_I2::QY,
        );
        diff.apply_enum_diff_attr(
            ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_G3),
            enums::CLB_MUX_I3::C,
            enums::CLB_MUX_I3::QY,
        );
        diff.apply_enum_diff_attr(
            ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_G4),
            enums::CLB_MUX_I4::D,
            enums::CLB_MUX_I4::E,
        );
        assert_eq!(diff.bits.len(), 1);
        let bit = diff.bits.keys().copied().next().unwrap();
        ctx.insert_bel_attr_raw(
            tcid,
            bslots::CLB,
            bcls::CLB::MODE,
            xlat_enum_attr(vec![
                (enums::CLB_MODE::FG, Diff::default()),
                (enums::CLB_MODE::FGM, diff),
            ]),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslots::CLB,
            bcls::CLB::READBACK_QY,
            vec![PolTileBit {
                bit: TileBit {
                    bit: bit.bit + 1,
                    ..bit
                },
                inv: true,
            }],
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslots::CLB,
            bcls::CLB::READBACK_QX,
            vec![PolTileBit {
                bit: TileBit {
                    frame: bit.frame + 1,
                    bit: bit.bit + 1,
                    ..bit
                },
                inv: true,
            }],
        );
    }
}
