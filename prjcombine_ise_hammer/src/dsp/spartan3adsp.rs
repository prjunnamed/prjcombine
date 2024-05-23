use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, State},
    diff::{collect_enum, collect_inv},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one,
    tiledb::TileDb,
};

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

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    mode: Mode,
) {
    let node_kind = backend.egrid.db.get_node("DSP");
    let bel = BelId::from_idx(0);
    let ctx = FuzzCtx {
        session,
        node_kind,
        bits: TileBits::Main(4),
        tile_name: "DSP",
        bel,
        bel_name: "DSP",
    };
    let bel_kind = match mode {
        Mode::Spartan3ADsp => "DSP48A",
        Mode::Spartan6 => "DSP48A1",
    };
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode bel_kind)]);
    for &pin in DSP48A_INVPINS {
        let pininv = format!("{pin}INV").leak();
        let pin_b = format!("{pin}_B").leak();
        fuzz_enum!(ctx, pininv, [pin, pin_b], [(mode bel_kind), (pin pin)]);
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
        fuzz_enum!(ctx, attr, ["0", "1"], [(mode bel_kind)]);
    }
    if mode == Mode::Spartan6 {
        fuzz_enum!(ctx, "CARRYOUTREG", ["0", "1"], [(mode bel_kind)]);
    }
    fuzz_enum!(ctx, "B_INPUT", ["DIRECT", "CASCADE"], [(mode bel_kind)]);
    fuzz_enum!(ctx, "CARRYINSEL", ["OPMODE5", "CARRYIN"], [(mode bel_kind)]);
    fuzz_enum!(ctx, "RSTTYPE", ["SYNC", "ASYNC"], [(mode bel_kind)]);
}

pub fn collect_fuzzers(state: &mut State, tiledb: &mut TileDb, mode: Mode) {
    for &pin in DSP48A_INVPINS {
        collect_inv(state, tiledb, "DSP", "DSP", pin);
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
        collect_enum(state, tiledb, "DSP", "DSP", attr, &["0", "1"]);
    }
    if mode == Mode::Spartan6 {
        collect_enum(state, tiledb, "DSP", "DSP", "CARRYOUTREG", &["0", "1"]);
    }
    collect_enum(
        state,
        tiledb,
        "DSP",
        "DSP",
        "B_INPUT",
        &["DIRECT", "CASCADE"],
    );
    collect_enum(
        state,
        tiledb,
        "DSP",
        "DSP",
        "CARRYINSEL",
        &["OPMODE5", "CARRYIN"],
    );
    collect_enum(state, tiledb, "DSP", "DSP", "RSTTYPE", &["SYNC", "ASYNC"]);
    state.get_diff("DSP", "DSP", "PRESENT", "1").assert_empty();
}
