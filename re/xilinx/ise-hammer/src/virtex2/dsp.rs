use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Spartan3ADsp,
    Spartan6,
}

const DSP48A_INVPINS: &[&str] = &[
    "CLK",
    "CEA",
    "CEB",
    "CEC",
    "CED",
    "CEM",
    "CEP",
    "CEOPMODE",
    "CECARRYIN",
    "RSTA",
    "RSTB",
    "RSTC",
    "RSTD",
    "RSTM",
    "RSTP",
    "RSTOPMODE",
    "RSTCARRYIN",
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mode = match backend.edev {
        ExpandedDevice::Virtex2(_) => Mode::Spartan3ADsp,
        ExpandedDevice::Spartan6(_) => Mode::Spartan6,
        _ => unreachable!(),
    };
    let mut ctx = FuzzCtx::new(session, backend, "DSP");
    let (bel_kind, slot) = match mode {
        Mode::Spartan3ADsp => ("DSP48A", prjcombine_virtex2::defs::bslots::DSP),
        Mode::Spartan6 => ("DSP48A1", prjcombine_spartan6::defs::bslots::DSP),
    };
    let mut bctx = ctx.bel(slot);
    bctx.test_manual("PRESENT", "1").mode(bel_kind).commit();
    for &pin in DSP48A_INVPINS {
        bctx.mode(bel_kind).test_inv(pin);
    }
    for attr in [
        "A0REG",
        "A1REG",
        "B0REG",
        "B1REG",
        "CREG",
        "DREG",
        "MREG",
        "PREG",
        "OPMODEREG",
        "CARRYINREG",
    ] {
        bctx.mode(bel_kind).test_enum(attr, &["0", "1"]);
    }
    if mode == Mode::Spartan6 {
        bctx.mode(bel_kind).test_enum("CARRYOUTREG", &["0", "1"]);
    }
    bctx.mode(bel_kind)
        .test_enum("B_INPUT", &["DIRECT", "CASCADE"]);
    bctx.mode(bel_kind)
        .test_enum("CARRYINSEL", &["OPMODE5", "CARRYIN"]);
    bctx.mode(bel_kind).test_enum("RSTTYPE", &["SYNC", "ASYNC"]);
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let mode = match ctx.edev {
        ExpandedDevice::Virtex2(_) => Mode::Spartan3ADsp,
        ExpandedDevice::Spartan6(_) => Mode::Spartan6,
        _ => unreachable!(),
    };

    for &pin in DSP48A_INVPINS {
        match mode {
            Mode::Spartan3ADsp => {
                ctx.collect_int_inv(&["INT_BRAM_S3ADSP"; 4], "DSP", "DSP", pin, false)
            }
            Mode::Spartan6 => ctx.collect_inv("DSP", "DSP", pin),
        }
    }
    for attr in [
        "A0REG",
        "A1REG",
        "B0REG",
        "B1REG",
        "CREG",
        "DREG",
        "MREG",
        "PREG",
        "OPMODEREG",
        "CARRYINREG",
    ] {
        ctx.collect_enum("DSP", "DSP", attr, &["0", "1"]);
    }
    if mode == Mode::Spartan6 {
        ctx.collect_enum("DSP", "DSP", "CARRYOUTREG", &["0", "1"]);
    }
    ctx.collect_enum("DSP", "DSP", "B_INPUT", &["DIRECT", "CASCADE"]);
    ctx.collect_enum("DSP", "DSP", "CARRYINSEL", &["OPMODE5", "CARRYIN"]);
    ctx.collect_enum("DSP", "DSP", "RSTTYPE", &["SYNC", "ASYNC"]);
    ctx.state
        .get_diff("DSP", "DSP", "PRESENT", "1")
        .assert_empty();
}
