use prjcombine_re_collector::diff::{Diff, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::{
    chip::ChipKind,
    xc4000::{bslots, enums, xc4000::bcls},
};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx, specials};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let chip = backend.edev.chip;
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for bslot in bslots::IO {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            let mut bctx = ctx.bel(bslot);
            bctx.mode("IO")
                .mutex("CLK", "O")
                .cfg("OUT", "OK")
                .test_bel_attr_enum_bool_as("OUT", bcls::IO::OFF_SRVAL, "RESET", "SET");
            bctx.mode("IO")
                .mutex("CLK", "O")
                .test_bel_special(specials::IO_OUT_OK)
                .cfg("OUT", "OK")
                .commit();
            bctx.mode("IO")
                .mutex("CLK", "O")
                .cfg("OUT", "OK")
                .test_bel_input_inv(bcls::IO::OK, true)
                .cfg("OUT", "OKNOT")
                .commit();
            bctx.mode("IO")
                .mutex("CLK", "I")
                .cfg("INFF", "IK")
                .test_bel_attr_enum_bool_as("INFF", bcls::IO::IFF_SRVAL, "RESET", "SET");
            bctx.mode("IO")
                .mutex("CLK", "I")
                .test_bel_special(specials::IO_INFF_IK)
                .cfg("INFF", "IK")
                .commit();
            bctx.mode("IO")
                .mutex("CLK", "I")
                .cfg("INFF", "IK")
                .test_bel_input_inv(bcls::IO::IK, true)
                .cfg("INFF", "IKNOT")
                .commit();
            bctx.mode("IO")
                .mutex("CLK", "I")
                .cfg("INFF", "IK")
                .test_bel_attr_val(bcls::IO::IFF_D, enums::IO_IFF_D::DELAY)
                .cfg("INFF", "DELAY")
                .commit();
            bctx.mode("IO")
                .mutex("CLK", "I")
                .cfg("INFF", "IK")
                .test_bel_attr_default_as("PAD", bcls::IO::PULL, enums::IO_PULL::NONE);

            if chip.kind != ChipKind::Xc4000A {
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_bel_attr_val(bcls::IO::SLEW, enums::IO_SLEW::FAST)
                    .cfg("PAD", "FAST")
                    .commit();
            } else {
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_bel_attr_as("OSPEED", bcls::IO::SLEW);
            }
            bctx.mode("IO")
                .mutex("CLK", "O")
                .cfg("OUT", "OK")
                .mutex("OUT", "O")
                .cfg("OUT", "O")
                .cfg("TRI", "T")
                .test_bel_input_inv(bcls::IO::T, true)
                .cfg("TRI", "NOT")
                .commit();
            bctx.mode("IO")
                .mutex("CLK", "I")
                .cfg("INFF", "IK")
                .test_bel_attr_as("I1", bcls::IO::MUX_I1);
            bctx.mode("IO")
                .mutex("CLK", "I")
                .cfg("INFF", "IK")
                .test_bel_attr_as("I2", bcls::IO::MUX_I2);
            for (attr, vname) in [
                (bcls::IO::READBACK_I1, "I1"),
                (bcls::IO::READBACK_I2, "I2"),
                (bcls::IO::READBACK_OQ, "OQ"),
            ] {
                bctx.mode("IO")
                    .mutex("CLK", "I")
                    .cfg("INFF", "IK")
                    .test_bel_attr_bits(attr)
                    .cfg_excl("RDBK", vname)
                    .commit();
            }
        }
        for bslot in bslots::HIO {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            let mut bctx = ctx.bel(bslot);
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .test_bel_attr_default_as("PAD", bcls::HIO::PULL, enums::IO_PULL::NONE);
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .cfg("OUT", "O")
                .test_bel_input_inv(bcls::HIO::T, true)
                .cfg("TRI", "NOT")
                .commit();
            bctx.mode("IO")
                .bonded_io()
                .test_bel_special(specials::HIO_IN_I)
                .cfg("IN", "I")
                .commit();
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .test_bel_attr_bits(bcls::HIO::I_INV)
                .cfg("IN", "NOT")
                .commit();
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .test_bel_attr_as("IN", bcls::HIO::ISTD);
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .test_bel_special(specials::HIO_OUT_O)
                .cfg("OUT", "O")
                .commit();
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .cfg("OUT", "O")
                .test_bel_input_inv(bcls::HIO::O, true)
                .cfg("OUT", "NOT")
                .commit();
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .cfg("OUT", "O")
                .test_bel_attr_as("OUT", bcls::HIO::OSTD);
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .cfg("OUT", "O")
                .test_bel_attr_as("OUT", bcls::HIO::OMODE);
            bctx.mode("IO")
                .bonded_io()
                .cfg("IN", "I")
                .test_bel_attr_bits(bcls::HIO::READBACK_I)
                .cfg("RDBK", "I")
                .commit();
        }
        for bslot in bslots::DEC {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            let mut bctx = ctx.bel(bslot);
            for (pid, pin, attr_p, attr_n) in [
                (bcls::DEC::O1, "O1", bcls::DEC::O1_P, bcls::DEC::O1_N),
                (bcls::DEC::O2, "O2", bcls::DEC::O2_P, bcls::DEC::O2_N),
                (bcls::DEC::O3, "O3", bcls::DEC::O3_P, bcls::DEC::O3_N),
                (bcls::DEC::O4, "O4", bcls::DEC::O4_P, bcls::DEC::O4_N),
            ] {
                if chip.kind == ChipKind::Xc4000A
                    && matches!(attr_p, bcls::DEC::O3_P | bcls::DEC::O4_P)
                {
                    continue;
                }
                bctx.mode("DECODER")
                    .bidir_mutex_exclusive(pid)
                    .test_bel_attr_bits(attr_p)
                    .pip_pin(pin, pin)
                    .commit();
                bctx.mode("DECODER")
                    .bidir_mutex_exclusive(pid)
                    .test_bel_attr_bits(attr_n)
                    .pip_pin(pin, pin)
                    .cfg(pin, "NOT")
                    .commit();
            }
        }
        for bslot in bslots::PULLUP_TBUF {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            let mut bctx = ctx.bel(bslot);
            bctx.build()
                .bidir_mutex_exclusive(bcls::PULLUP::O)
                .test_bel_attr_bits(bcls::PULLUP::ENABLE)
                .pip_pin("O", "O")
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let chip = ctx.edev.chip;
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        if !ctx.has_tile(tcid) {
            continue;
        }
        for bslot in bslots::IO {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            ctx.collect_bel_attr_default(tcid, bslot, bcls::IO::PULL, enums::IO_PULL::NONE);
            ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::IO::OFF_SRVAL);
            ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::IO::IFF_SRVAL);
            ctx.collect_bel_input_inv(tcid, bslot, bcls::IO::IK);
            ctx.collect_bel_input_inv(tcid, bslot, bcls::IO::OK);
            ctx.collect_bel_input_inv(tcid, bslot, bcls::IO::T);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::MUX_I1);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::MUX_I2);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::READBACK_I1);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::READBACK_I2);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::READBACK_OQ);
            if chip.kind != ChipKind::Xc4000A {
                let diff = ctx.get_diff_attr_val(tcid, bslot, bcls::IO::SLEW, enums::IO_SLEW::FAST);
                let item = xlat_enum_attr(vec![
                    (enums::IO_SLEW::FAST, diff),
                    (enums::IO_SLEW::SLOW, Diff::default()),
                ]);
                ctx.insert_bel_attr_raw(tcid, bslot, bcls::IO::SLEW, item);
            } else {
                ctx.collect_bel_attr(tcid, bslot, bcls::IO::SLEW);
            }
            let diff = ctx.get_diff_attr_val(tcid, bslot, bcls::IO::IFF_D, enums::IO_IFF_D::DELAY);
            let item = xlat_enum_attr(vec![
                (enums::IO_IFF_D::DELAY, diff),
                (enums::IO_IFF_D::I, Diff::default()),
            ]);
            ctx.insert_bel_attr_raw(tcid, bslot, bcls::IO::IFF_D, item);

            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IO_INFF_IK);
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, bcls::IO::PULL),
                enums::IO_PULL::NONE,
                enums::IO_PULL::PULLUP,
            );
            diff.assert_empty();
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUT_OK);
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, bcls::IO::PULL),
                enums::IO_PULL::NONE,
                enums::IO_PULL::PULLUP,
            );
            diff.assert_empty();
        }
        for bslot in bslots::HIO {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            ctx.collect_bel_attr_default(tcid, bslot, bcls::HIO::PULL, enums::IO_PULL::NONE);
            ctx.collect_bel_attr(tcid, bslot, bcls::HIO::ISTD);
            ctx.collect_bel_attr(tcid, bslot, bcls::HIO::OSTD);
            ctx.collect_bel_attr(tcid, bslot, bcls::HIO::OMODE);
            ctx.collect_bel_attr(tcid, bslot, bcls::HIO::I_INV);
            ctx.collect_bel_attr(tcid, bslot, bcls::HIO::READBACK_I);
            ctx.collect_bel_input_inv(tcid, bslot, bcls::HIO::T);
            ctx.collect_bel_input_inv(tcid, bslot, bcls::HIO::O);

            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::HIO_IN_I);
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, bcls::HIO::PULL),
                enums::IO_PULL::NONE,
                enums::IO_PULL::PULLUP,
            );
            diff.assert_empty();
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::HIO_OUT_O);
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, bcls::HIO::OSTD),
                enums::IO_STD::TTL,
                enums::IO_STD::CMOS,
            );
            diff.apply_bit_diff(ctx.bel_input_inv(tcid, bslot, bcls::HIO::T), false, true);
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, bcls::HIO::OMODE),
                enums::HIO_OMODE::RES,
                enums::HIO_OMODE::CAP,
            );
            diff.assert_empty();
        }
        for bslot in bslots::DEC {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            for (attr_p, attr_n) in [
                (bcls::DEC::O1_P, bcls::DEC::O1_N),
                (bcls::DEC::O2_P, bcls::DEC::O2_N),
                (bcls::DEC::O3_P, bcls::DEC::O3_N),
                (bcls::DEC::O4_P, bcls::DEC::O4_N),
            ] {
                if chip.kind == ChipKind::Xc4000A
                    && matches!(attr_p, bcls::DEC::O3_P | bcls::DEC::O4_P)
                {
                    continue;
                }
                ctx.collect_bel_attr(tcid, bslot, attr_p);
                ctx.collect_bel_attr(tcid, bslot, attr_n);
            }
        }
        for bslot in bslots::PULLUP_TBUF {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            ctx.collect_bel_attr(tcid, bslot, bcls::PULLUP::ENABLE);
        }
    }
}
