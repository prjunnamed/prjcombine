use prjcombine_re_collector::diff::{Diff, xlat_bit_raw, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_types::bits;
use prjcombine_xc2000::xc2000::{bcls, bslots, enums, tslots};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx, specials};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(bslots::CLB);
        for (val, spec) in [
            ("F", specials::CLB_MODE_F),
            ("FG", specials::CLB_MODE_FG),
            ("FGM", specials::CLB_MODE_FGM),
        ] {
            bctx.build()
                .null_bits()
                .test_bel_special(spec)
                .mode(val)
                .commit();
        }
        for (attr, aname) in [(bcls::CLB::F, "F"), (bcls::CLB::G, "G")] {
            bctx.mode("FG")
                .test_bel_attr_equate(attr, aname, &["B", "C", "Q"]);
            for (spec, i1, i2, i3) in [
                (specials::CLB_LUT_ABC, "A", "B", "C"),
                (specials::CLB_LUT_ABD, "A", "B", "D"),
                (specials::CLB_LUT_ACD, "A", "C", "D"),
                (specials::CLB_LUT_BCD, "B", "C", "D"),
                (specials::CLB_LUT_ABQ, "A", "B", "Q"),
                (specials::CLB_LUT_ACQ, "A", "C", "Q"),
                (specials::CLB_LUT_BCQ, "B", "C", "Q"),
            ] {
                bctx.mode("FG")
                    .test_bel_attr_special(attr, spec)
                    .cfg(aname, i1)
                    .cfg(aname, i2)
                    .cfg(aname, i3)
                    .commit();
            }
            bctx.mode("FGM")
                .mutex("FGM", aname)
                .test_bel_attr_special(attr, specials::CLB_LUT_MACD)
                .cfg(aname, "A")
                .cfg(aname, "C")
                .cfg(aname, "D")
                .commit();
            for (spec, inps) in [
                (specials::CLB_LUT_EQ_ABC, &["A", "B", "C"]),
                (specials::CLB_LUT_EQ_BCD, &["B", "C", "D"]),
                (specials::CLB_LUT_EQ_BCQ, &["B", "C", "Q"]),
            ] {
                for bidx in [0, 1, 2, 4] {
                    let mut val = bits![0; 8];
                    val.set(bidx, true);
                    bctx.mode("FG")
                        .test_bel_attr_special_bit(attr, spec, bidx)
                        .equate_fixed(aname, inps, val)
                        .commit();
                }
            }
        }
        for bidx in [0, 1, 2, 4, 8] {
            let mut val = bits![0; 16];
            val.set(bidx, true);
            bctx.mode("F")
                .test_bel_attr_special_bit(bcls::CLB::F, specials::CLB_LUT_EQ_ABCD, bidx)
                .equate_fixed("F", &["A", "B", "C", "D"], val)
                .commit();
        }
        bctx.mode("FG").test_bel_attr_default_as(
            "SET",
            bcls::CLB::MUX_SET,
            enums::CLB_MUX_SET::TIE_0,
        );
        bctx.mode("FG").test_bel_attr_default_as(
            "RES",
            bcls::CLB::MUX_RES,
            enums::CLB_MUX_RES::TIE_0,
        );
        bctx.mode("FG")
            .mutex("CLK", "C")
            .test_bel_special(specials::CLB_CLK_C)
            .cfg("CLK", "C")
            .commit();
        bctx.mode("FG")
            .mutex("CLK", "G")
            .test_bel_special(specials::CLB_CLK_G)
            .cfg("CLK", "G")
            .commit();
        bctx.mode("FG")
            .mutex("Q", "FF")
            .cfg("Q", "FF")
            .mutex("CLK", "C")
            .cfg("CLK", "C")
            .test_bel_input_inv(bcls::CLB::K, true)
            .cfg("CLK", "NOT")
            .commit();
        bctx.mode("FG")
            .mutex("CLK", "C")
            .cfg("CLK", "C")
            .test_bel_attr_as("Q", bcls::CLB::FF_MODE);
        bctx.mode("FG").test_bel_attr_as("X", bcls::CLB::MUX_X);
        bctx.mode("FG").test_bel_attr_as("Y", bcls::CLB::MUX_Y);
        bctx.mode("FGM")
            .mutex("FGM", "F")
            .cfg("F", "A")
            .cfg("F", "C")
            .cfg("F", "D")
            .test_bel_attr_special(bcls::CLB::MUX_X, specials::CLB_MUX_XY_M)
            .cfg("X", "M")
            .commit();
        bctx.mode("FGM")
            .mutex("FGM", "F")
            .cfg("F", "A")
            .cfg("F", "C")
            .cfg("F", "D")
            .test_bel_attr_special(bcls::CLB::MUX_Y, specials::CLB_MUX_XY_M)
            .cfg("Y", "M")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        for attr in [bcls::CLB::MUX_X, bcls::CLB::MUX_Y] {
            let mut diffs = vec![(
                enums::CLB_MUX_XY::F,
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_MUX_XY_M),
            )];
            for val in [
                enums::CLB_MUX_XY::Q,
                enums::CLB_MUX_XY::F,
                enums::CLB_MUX_XY::G,
            ] {
                diffs.push((val, ctx.get_diff_attr_val(tcid, bslots::CLB, attr, val)));
            }
            let item = xlat_enum_attr(diffs);
            ctx.insert_bel_attr_raw(tcid, bslots::CLB, attr, item);
        }
        ctx.collect_bel_attr_default(
            tcid,
            bslots::CLB,
            bcls::CLB::MUX_RES,
            enums::CLB_MUX_RES::TIE_0,
        );
        ctx.collect_bel_attr_default(
            tcid,
            bslots::CLB,
            bcls::CLB::MUX_SET,
            enums::CLB_MUX_SET::TIE_0,
        );
        ctx.collect_bel_input_inv(tcid, bslots::CLB, bcls::CLB::K);
        let bit_inv = ctx.bel_input_inv(tcid, bslots::CLB, bcls::CLB::K);
        let diff_latch =
            ctx.get_diff_attr_val(tcid, bslots::CLB, bcls::CLB::FF_MODE, enums::FF_MODE::LATCH);
        assert_eq!(xlat_bit_raw(diff_latch), bit_inv);
        ctx.collect_bel_attr_default(tcid, bslots::CLB, bcls::CLB::FF_MODE, enums::FF_MODE::LATCH);
        ctx.collect_bel_attr(tcid, bslots::CLB, bcls::CLB::F);
        ctx.collect_bel_attr(tcid, bslots::CLB, bcls::CLB::G);
        for (attr, attr_i1, attr_i2, attr_i3) in [
            (
                bcls::CLB::F,
                bcls::CLB::MUX_F1,
                bcls::CLB::MUX_F2,
                bcls::CLB::MUX_F3,
            ),
            (
                bcls::CLB::G,
                bcls::CLB::MUX_G1,
                bcls::CLB::MUX_G2,
                bcls::CLB::MUX_G3,
            ),
        ] {
            let diff_abc =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ABC);
            let diff_abd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ABD);
            let diff_abq =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ABQ);
            let diff_acd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ACD);
            let diff_acq =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_ACQ);
            let diff_bcd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_BCD);
            let diff_bcq =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_BCQ);
            let diff_macd =
                ctx.get_diff_attr_special(tcid, bslots::CLB, attr, specials::CLB_LUT_MACD);
            let diff_m = diff_macd.combine(&!&diff_acd);
            ctx.insert_bel_attr_raw(
                tcid,
                bslots::CLB,
                bcls::CLB::MODE,
                xlat_enum_attr(vec![
                    (enums::CLB_MODE::FG, Diff::default()),
                    (enums::CLB_MODE::FGM, diff_m),
                ]),
            );
            let diff_i3_c = diff_abc.combine(&!&diff_abq);
            let diff_i3_d = diff_abd.combine(&!&diff_abq);
            assert_eq!(diff_i3_d, diff_acd.combine(&!&diff_acq));
            assert_eq!(diff_i3_d, diff_bcd.combine(&!&diff_bcq));
            ctx.insert_bel_attr_raw(
                tcid,
                bslots::CLB,
                attr_i3,
                xlat_enum_attr(vec![
                    (enums::CLB_MUX_I3::C, diff_i3_c),
                    (enums::CLB_MUX_I3::D, diff_i3_d),
                    (enums::CLB_MUX_I3::Q, Diff::default()),
                ]),
            );
            let diff_i1_a = diff_acq.combine(&!&diff_bcq);
            let diff_i2_b = diff_abq.combine(&!&diff_acq);
            diff_bcq.assert_empty();
            ctx.insert_bel_attr_raw(
                tcid,
                bslots::CLB,
                attr_i1,
                xlat_enum_attr(vec![
                    (enums::CLB_MUX_I1::A, diff_i1_a),
                    (enums::CLB_MUX_I1::B, Diff::default()),
                ]),
            );
            ctx.insert_bel_attr_raw(
                tcid,
                bslots::CLB,
                attr_i2,
                xlat_enum_attr(vec![
                    (enums::CLB_MUX_I2::B, diff_i2_b),
                    (enums::CLB_MUX_I2::C, Diff::default()),
                ]),
            );

            let lut_bits = ctx.bel_attr_bitvec(tcid, bslots::CLB, attr).to_vec();
            let mux_i1 = ctx.bel_attr_enum(tcid, bslots::CLB, attr_i1).clone();
            let mux_i2 = ctx.bel_attr_enum(tcid, bslots::CLB, attr_i2).clone();
            let mux_i3 = ctx.bel_attr_enum(tcid, bslots::CLB, attr_i3).clone();

            for spec in [
                specials::CLB_LUT_EQ_ABC,
                specials::CLB_LUT_EQ_BCD,
                specials::CLB_LUT_EQ_BCQ,
            ] {
                for bit in [0, 1, 2, 4] {
                    let mut diff =
                        ctx.get_diff_attr_special_bit(tcid, bslots::CLB, attr, spec, bit);
                    let mut bits = bits![0; 8];
                    bits.set(bit, true);
                    diff.apply_bitvec_diff_raw(&lut_bits, &bits, &bits![0; 8]);
                    if spec == specials::CLB_LUT_EQ_ABC {
                        diff.apply_enum_diff_attr(
                            &mux_i1,
                            enums::CLB_MUX_I1::A,
                            enums::CLB_MUX_I1::B,
                        );
                        diff.apply_enum_diff_attr(
                            &mux_i2,
                            enums::CLB_MUX_I2::B,
                            enums::CLB_MUX_I2::C,
                        );
                        diff.apply_enum_diff_attr(
                            &mux_i3,
                            enums::CLB_MUX_I3::C,
                            enums::CLB_MUX_I3::Q,
                        );
                    }
                    if spec == specials::CLB_LUT_EQ_BCD {
                        diff.apply_enum_diff_attr(
                            &mux_i3,
                            enums::CLB_MUX_I3::D,
                            enums::CLB_MUX_I3::Q,
                        );
                    }
                    diff.assert_empty();
                }
            }
        }
        for (diff_bit, real_bit) in [(0, 0), (1, 1), (4, 2), (8, 4), (2, 8)] {
            let mut bits = bits![0; 16];
            bits.set(real_bit, true);
            let mut diff = ctx.get_diff_attr_special_bit(
                tcid,
                bslots::CLB,
                bcls::CLB::F,
                specials::CLB_LUT_EQ_ABCD,
                diff_bit,
            );
            let f_bits = ctx.bel_attr_bitvec(tcid, bslots::CLB, bcls::CLB::F);
            let g_bits = ctx.bel_attr_bitvec(tcid, bslots::CLB, bcls::CLB::G);
            let mux_f1 = ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_F1);
            let mux_g1 = ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_G1);
            let mux_f3 = ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_F3);
            let mux_g3 = ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MUX_G3);
            let mode = ctx.bel_attr_enum(tcid, bslots::CLB, bcls::CLB::MODE);
            diff.apply_bitvec_diff_raw(g_bits, &bits.slice(..8), &bits![0; 8]);
            diff.apply_bitvec_diff_raw(f_bits, &bits.slice(8..), &bits![0; 8]);
            diff.apply_enum_diff_attr(mux_f1, enums::CLB_MUX_I1::A, enums::CLB_MUX_I1::B);
            diff.apply_enum_diff_attr(mux_g1, enums::CLB_MUX_I1::A, enums::CLB_MUX_I1::B);
            diff.apply_enum_diff_attr(mux_f3, enums::CLB_MUX_I3::D, enums::CLB_MUX_I3::Q);
            diff.apply_enum_diff_attr(mux_g3, enums::CLB_MUX_I3::D, enums::CLB_MUX_I3::Q);
            diff.apply_enum_diff_attr(mode, enums::CLB_MODE::FGM, enums::CLB_MODE::FG);
            diff.assert_empty();
        }
    }
}
