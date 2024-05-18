use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_virtex2::grid::GridKind;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, State},
    diff::{collect_enum, xlat_bitvec, Diff},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum,
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
    if grid_kind != GridKind::Spartan3ADsp {
        if grid_kind.is_virtex2() || grid_kind == GridKind::Spartan3 {
            let f_clk = state.get_diff(tile_name, "MULT", "CLKINV", "CLK");
            let f_clk_b = state.get_diff(tile_name, "MULT", "CLKINV", "CLK_B");
            let (f_clk, f_clk_b, f_reg) = Diff::split(f_clk, f_clk_b);
            f_clk.assert_empty();
            tiledb.insert(tile_name, "MULT", "REG", xlat_bitvec(vec![f_reg]));
            tiledb.insert(tile_name, "MULT", "CLKINV", xlat_bitvec(vec![f_clk_b]));
            for (pininv, pin, pin_b, def) in [
                ("CEINV", "CE", "CE_B", false),
                ("RSTINV", "RST", "RST_B", true),
            ] {
                let f_pin = state.get_diff(tile_name, "MULT", pininv, pin);
                let f_pin_b = state.get_diff(tile_name, "MULT", pininv, pin_b);
                let inv = if !def {
                    f_pin.assert_empty();
                    f_pin_b
                } else {
                    f_pin_b.assert_empty();
                    !f_pin
                };
                tiledb.insert(tile_name, "MULT", pininv, xlat_bitvec(vec![inv]));
            }
        } else {
            for (pininv, pin, pin_b, def) in [
                ("CLKINV", "CLK", "CLK_B", false),
                ("CEAINV", "CEA", "CEA_B", false),
                ("CEBINV", "CEB", "CEB_B", false),
                ("CEPINV", "CEP", "CEP_B", false),
                ("RSTAINV", "RSTA", "RSTA_B", true),
                ("RSTBINV", "RSTB", "RSTB_B", true),
                ("RSTPINV", "RSTP", "RSTP_B", true),
            ] {
                let f_pin = state.get_diff(tile_name, "MULT", pininv, pin);
                let f_pin_b = state.get_diff(tile_name, "MULT", pininv, pin_b);
                let inv = if !def {
                    f_pin.assert_empty();
                    f_pin_b
                } else {
                    f_pin_b.assert_empty();
                    !f_pin
                };
                tiledb.insert(tile_name, "MULT", pininv, xlat_bitvec(vec![inv]));
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
