use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_virtex2::grid::GridKind;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, State},
    diff::{self, collect_bitvec, collect_enum, collect_inv, xlat_bitvec, xlat_enum, Diff},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi,
    tiledb::TileDb,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let grid_kind = match backend.edev {
        ExpandedDevice::Virtex2(ref edev) => edev.grid.kind,
        _ => unreachable!(),
    };
    let tile_name = match grid_kind {
        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
        GridKind::Spartan3 => "BRAM.S3",
        GridKind::Spartan3E => "BRAM.S3E",
        GridKind::Spartan3A => "BRAM.S3A",
        GridKind::Spartan3ADsp => "BRAM.S3ADSP",
    };
    let node_kind = backend.egrid.db.get_node(tile_name);
    let bel = BelId::from_idx(0);
    let ctx = FuzzCtx {
        session,
        node_kind,
        bits: TileBits::Bram,
        tile_name,
        bel,
        bel_name: "BRAM",
    };
    match grid_kind {
        GridKind::Spartan3A | GridKind::Spartan3ADsp => {
            let bel_kind = if grid_kind == GridKind::Spartan3ADsp {
                "RAMB16BWER"
            } else {
                "RAMB16BWE"
            };
            let mut invs = vec![
                ("CLKAINV", "CLKA", "CLKA_B"),
                ("CLKBINV", "CLKB", "CLKB_B"),
                ("ENAINV", "ENA", "ENA_B"),
                ("ENBINV", "ENB", "ENB_B"),
                ("WEA0INV", "WEA0", "WEA0_B"),
                ("WEA1INV", "WEA1", "WEA1_B"),
                ("WEA2INV", "WEA2", "WEA2_B"),
                ("WEA3INV", "WEA3", "WEA3_B"),
                ("WEB0INV", "WEB0", "WEB0_B"),
                ("WEB1INV", "WEB1", "WEB1_B"),
                ("WEB2INV", "WEB2", "WEB2_B"),
                ("WEB3INV", "WEB3", "WEB3_B"),
            ];
            if grid_kind == GridKind::Spartan3ADsp {
                invs.extend([
                    ("RSTAINV", "RSTA", "RSTA_B"),
                    ("RSTBINV", "RSTB", "RSTB_B"),
                    ("REGCEAINV", "REGCEA", "REGCEA_B"),
                    ("REGCEBINV", "REGCEB", "REGCEB_B"),
                ]);
            } else {
                invs.extend([("SSRAINV", "SSRA", "SSRA_B"), ("SSRBINV", "SSRB", "SSRB_B")]);
            }
            for (pininv, pin, pin_b) in invs {
                fuzz_enum!(ctx, pininv, [pin, pin_b], [
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36"),
                    (pin pin)
                ]);
            }
            for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
                fuzz_enum!(ctx, attr, ["0", "1", "2", "4", "9", "18", "36"], [
                    (mode bel_kind),
                    (attr "INIT_A", "0"),
                    (attr "INIT_B", "0"),
                    (attr "SRVAL_A", "0"),
                    (attr "SRVAL_B", "0")
                ]);
            }
            for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
                fuzz_enum!(ctx, attr, ["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"], [
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36")
                ]);
            }
            if grid_kind == GridKind::Spartan3ADsp {
                fuzz_enum!(ctx, "RSTTYPE", ["SYNC", "ASYNC"], [
                    (mode bel_kind)
                ]);
                fuzz_enum!(ctx, "DOA_REG", ["0", "1"], [
                    (mode bel_kind)
                ]);
                fuzz_enum!(ctx, "DOB_REG", ["0", "1"], [
                    (mode bel_kind)
                ]);
            }
            for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
                fuzz_multi!(ctx, attr, "", 36, [
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36")
                ], (attr_hex attr));
            }
            for i in 0..0x40 {
                let attr = format!("INIT_{i:02X}").leak();
                fuzz_multi!(ctx, attr, "", 256, [
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36")
                ], (attr_hex attr));
            }
            for i in 0..0x8 {
                let attr = format!("INITP_{i:02X}").leak();
                fuzz_multi!(ctx, attr, "", 256, [
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36")
                ], (attr_hex attr));
            }
        }
        _ => {
            for (pininv, pin, pin_b) in [
                ("CLKAINV", "CLKA", "CLKA_B"),
                ("CLKBINV", "CLKB", "CLKB_B"),
                ("SSRAINV", "SSRA", "SSRA_B"),
                ("SSRBINV", "SSRB", "SSRB_B"),
                ("WEAINV", "WEA", "WEA_B"),
                ("WEBINV", "WEB", "WEB_B"),
                ("ENAINV", "ENA", "ENA_B"),
                ("ENBINV", "ENB", "ENB_B"),
            ] {
                fuzz_enum!(ctx, pininv, [pin, pin_b], [
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36"),
                    (pin pin)
                ]);
            }
            for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
                fuzz_enum!(ctx, attr, ["16384X1", "8192X2", "4096X4", "2048X9", "1024X18", "512X36"], [
                    (mode "RAMB16"),
                    (attr "INIT_A", "0"),
                    (attr "INIT_B", "0"),
                    (attr "SRVAL_A", "0"),
                    (attr "SRVAL_B", "0")
                ]);
            }
            for attr in ["WRITEMODEA", "WRITEMODEB"] {
                fuzz_enum!(ctx, attr, ["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"], [
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ]);
            }
            if grid_kind.is_virtex2() {
                fuzz_enum!(ctx, "SAVEDATA", ["FALSE", "TRUE"], [
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ]);
            }
            for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
                fuzz_multi!(ctx, attr, "", 36, [
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ], (attr_hex attr));
            }
            for i in 0..0x40 {
                let attr = format!("INIT_{i:02x}").leak();
                fuzz_multi!(ctx, attr, "", 256, [
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ], (attr_hex attr));
            }
            for i in 0..0x8 {
                let attr = format!("INITP_{i:02x}").leak();
                fuzz_multi!(ctx, attr, "", 256, [
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ], (attr_hex attr));
            }
        }
    }
    if grid_kind != GridKind::Spartan3ADsp {
        // mult
        let bel = BelId::from_idx(1);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Main(4),
            tile_name,
            bel,
            bel_name: "MULT",
        };
        if !matches!(grid_kind, GridKind::Spartan3E | GridKind::Spartan3A) {
            for (pininv, pin, pin_b) in [
                ("CLKINV", "CLK", "CLK_B"),
                ("RSTINV", "RST", "RST_B"),
                ("CEINV", "CE", "CE_B"),
            ] {
                fuzz_enum!(ctx, pininv, [pin, pin_b], [
                    (mode "MULT18X18"),
                    (pin pin)
                ]);
            }
        } else {
            for (pininv, pin, pin_b) in [
                ("CLKINV", "CLK", "CLK_B"),
                ("RSTAINV", "RSTA", "RSTA_B"),
                ("RSTBINV", "RSTB", "RSTB_B"),
                ("RSTPINV", "RSTP", "RSTP_B"),
                ("CEAINV", "CEA", "CEA_B"),
                ("CEBINV", "CEB", "CEB_B"),
                ("CEPINV", "CEP", "CEP_B"),
            ] {
                fuzz_enum!(ctx, pininv, [pin, pin_b], [
                    (mode "MULT18X18SIO"),
                    (pin pin)
                ]);
            }
            fuzz_enum!(ctx, "AREG", ["0", "1"], [
                (mode "MULT18X18SIO")
            ]);
            fuzz_enum!(ctx, "BREG", ["0", "1"], [
                (mode "MULT18X18SIO")
            ]);
            fuzz_enum!(ctx, "PREG", ["0", "1"], [
                (mode "MULT18X18SIO")
            ]);
            fuzz_enum!(ctx, "PREG_CLKINVERSION", ["0", "1"], [
                (mode "MULT18X18SIO")
            ]);
            fuzz_enum!(ctx, "B_INPUT", ["DIRECT", "CASCADE"], [
                (mode "MULT18X18SIO")
            ]);
        }
    }
}

pub fn collect_fuzzers(state: &mut State, tiledb: &mut TileDb, grid_kind: GridKind) {
    let tile_name = match grid_kind {
        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
        GridKind::Spartan3 => "BRAM.S3",
        GridKind::Spartan3E => "BRAM.S3E",
        GridKind::Spartan3A => "BRAM.S3A",
        GridKind::Spartan3ADsp => "BRAM.S3ADSP",
    };
    let mut diffs_data = vec![];
    let mut diffs_datap = vec![];
    for pin in ["CLKA", "CLKB", "ENA", "ENB"] {
        collect_inv(state, tiledb, tile_name, "BRAM", pin);
    }
    match grid_kind {
        GridKind::Spartan3A | GridKind::Spartan3ADsp => {
            for pin in [
                "WEA0", "WEB0", "WEA1", "WEB1", "WEA2", "WEB2", "WEA3", "WEB3",
            ] {
                collect_inv(state, tiledb, tile_name, "BRAM", pin);
            }
            for i in 0..0x40 {
                diffs_data.extend(state.get_diffs(
                    tile_name,
                    "BRAM",
                    format!("INIT_{i:02X}").leak(),
                    "",
                ));
            }
            for i in 0..0x08 {
                diffs_datap.extend(state.get_diffs(
                    tile_name,
                    "BRAM",
                    format!("INITP_{i:02X}").leak(),
                    "",
                ));
            }
            for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
                collect_enum(
                    state,
                    tiledb,
                    tile_name,
                    "BRAM",
                    attr,
                    &["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"],
                );
            }
            for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
                state.get_diff(tile_name, "BRAM", attr, "0").assert_empty();
                collect_enum(
                    state,
                    tiledb,
                    tile_name,
                    "BRAM",
                    attr,
                    &["1", "2", "4", "9", "18", "36"],
                );
            }
            if grid_kind == GridKind::Spartan3ADsp {
                for pin in ["RSTA", "RSTB", "REGCEA", "REGCEB"] {
                    collect_inv(state, tiledb, tile_name, "BRAM", pin);
                }

                collect_enum(state, tiledb, tile_name, "BRAM", "DOA_REG", &["0", "1"]);
                collect_enum(state, tiledb, tile_name, "BRAM", "DOB_REG", &["0", "1"]);
                collect_enum(
                    state,
                    tiledb,
                    tile_name,
                    "BRAM",
                    "RSTTYPE",
                    &["ASYNC", "SYNC"],
                );
            } else {
                for pin in ["SSRA", "SSRB"] {
                    collect_inv(state, tiledb, tile_name, "BRAM", pin);
                }
            }
        }
        _ => {
            for pin in ["WEA", "WEB", "SSRA", "SSRB"] {
                collect_inv(state, tiledb, tile_name, "BRAM", pin);
            }
            for i in 0..0x40 {
                diffs_data.extend(state.get_diffs(
                    tile_name,
                    "BRAM",
                    format!("INIT_{i:02x}").leak(),
                    "",
                ));
            }
            for i in 0..0x08 {
                diffs_datap.extend(state.get_diffs(
                    tile_name,
                    "BRAM",
                    format!("INITP_{i:02x}").leak(),
                    "",
                ));
            }
            for (dattr, sattr) in [
                ("WRITE_MODE_A", "WRITEMODEA"),
                ("WRITE_MODE_B", "WRITEMODEB"),
            ] {
                let diffs = ["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"]
                    .iter()
                    .map(|val| {
                        (
                            val.to_string(),
                            state.get_diff(tile_name, "BRAM", sattr, val),
                        )
                    })
                    .collect();
                tiledb.insert(tile_name, "BRAM", dattr, xlat_enum(diffs));
            }
            for (dattr, sattr) in [
                ("DATA_WIDTH_A", "PORTA_ATTR"),
                ("DATA_WIDTH_B", "PORTB_ATTR"),
            ] {
                let diffs = [
                    ("1", "16384X1"),
                    ("2", "8192X2"),
                    ("4", "4096X4"),
                    ("9", "2048X9"),
                    ("18", "1024X18"),
                    ("36", "512X36"),
                ]
                .iter()
                .map(|(dval, sval)| {
                    (
                        dval.to_string(),
                        state.get_diff(tile_name, "BRAM", sattr, sval),
                    )
                })
                .collect();
                tiledb.insert(tile_name, "BRAM", dattr, xlat_enum(diffs));
            }
            if grid_kind.is_virtex2() {
                state
                    .get_diff(tile_name, "BRAM", "SAVEDATA", "FALSE")
                    .assert_empty();
                let diff = state.get_diff(tile_name, "BRAM", "SAVEDATA", "TRUE");
                let mut bits: Vec<_> = diff.bits.into_iter().collect();
                bits.sort();
                tiledb.insert(
                    tile_name,
                    "BRAM",
                    "SAVEDATA",
                    xlat_bitvec(
                        bits.into_iter()
                            .map(|(k, v)| Diff {
                                bits: [(k, v)].into_iter().collect(),
                            })
                            .collect(),
                    ),
                )
            }
        }
    }
    tiledb.insert(tile_name, "BRAM", "DATA", xlat_bitvec(diffs_data));
    tiledb.insert(tile_name, "BRAM", "DATAP", xlat_bitvec(diffs_datap));
    collect_bitvec(state, tiledb, tile_name, "BRAM", "INIT_A", "");
    collect_bitvec(state, tiledb, tile_name, "BRAM", "INIT_B", "");
    collect_bitvec(state, tiledb, tile_name, "BRAM", "SRVAL_A", "");
    collect_bitvec(state, tiledb, tile_name, "BRAM", "SRVAL_B", "");

    if grid_kind != GridKind::Spartan3ADsp {
        if grid_kind.is_virtex2() || grid_kind == GridKind::Spartan3 {
            let f_clk = state.get_diff(tile_name, "MULT", "CLKINV", "CLK");
            let f_clk_b = state.get_diff(tile_name, "MULT", "CLKINV", "CLK_B");
            let (f_clk, f_clk_b, f_reg) = Diff::split(f_clk, f_clk_b);
            f_clk.assert_empty();
            tiledb.insert(tile_name, "MULT", "REG", xlat_bitvec(vec![f_reg]));
            tiledb.insert(tile_name, "MULT", "CLKINV", xlat_bitvec(vec![f_clk_b]));
            collect_inv(state, tiledb, tile_name, "MULT", "CE");
            collect_inv(state, tiledb, tile_name, "MULT", "RST");
        } else {
            for pin in ["CLK", "CEA", "CEB", "CEP", "RSTA", "RSTB", "RSTP"] {
                collect_inv(state, tiledb, tile_name, "MULT", pin);
            }
            collect_enum(state, tiledb, tile_name, "MULT", "AREG", &["0", "1"]);
            collect_enum(state, tiledb, tile_name, "MULT", "BREG", &["0", "1"]);
            collect_enum(state, tiledb, tile_name, "MULT", "PREG", &["0", "1"]);
            collect_enum(
                state,
                tiledb,
                tile_name,
                "MULT",
                "B_INPUT",
                &["DIRECT", "CASCADE"],
            );
            state
                .get_diff(tile_name, "MULT", "PREG_CLKINVERSION", "0")
                .assert_empty();
            tiledb.insert(
                tile_name,
                "MULT",
                "PREG_CLKINVERSION",
                xlat_bitvec(vec![state.get_diff(
                    tile_name,
                    "MULT",
                    "PREG_CLKINVERSION",
                    "1",
                )]),
            );
        }
    }
}