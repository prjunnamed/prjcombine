use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, State},
    diff::{collect_bitvec, collect_enum, collect_enum_bool, collect_inv, xlat_bitvec, xlat_enum},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi,
    tiledb::TileDb,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Virtex5,
    Virtex6,
    Virtex7,
    Spartan6,
}

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let mode = match backend.edev {
        ExpandedDevice::Virtex4(ref edev) => match edev.kind {
            prjcombine_virtex4::grid::GridKind::Virtex4 => unreachable!(),
            prjcombine_virtex4::grid::GridKind::Virtex5 => Mode::Virtex5,
            prjcombine_virtex4::grid::GridKind::Virtex6 => Mode::Virtex6,
            prjcombine_virtex4::grid::GridKind::Virtex7 => Mode::Virtex7,
        },
        ExpandedDevice::Spartan6(_) => Mode::Spartan6,
        _ => unreachable!(),
    };

    for tile_name in if mode == Mode::Spartan6 {
        ["CLEXL", "CLEXM"]
    } else {
        ["CLBLL", "CLBLM"]
    } {
        let node_kind = backend.egrid.db.get_node(tile_name);
        let bk_x = if mode == Mode::Spartan6 {
            "SLICEX"
        } else {
            "SLICEL"
        };
        for i in 0..2 {
            let bel = BelId::from_idx(i);
            let bel_name = backend.egrid.db.nodes[node_kind].bels.key(bel);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Main(1),
                tile_name,
                bel,
                bel_name,
            };
            let is_x = i == 1 && mode == Mode::Spartan6;
            let is_m = i == 0 && tile_name.ends_with('M');

            // LUTs
            for attr in ["A6LUT", "B6LUT", "C6LUT", "D6LUT"] {
                fuzz_multi!(ctx, attr, "#LUT", 64, [(mode bk_x)], (attr_lut attr));
            }

            if is_m {
                // LUT RAM
                fuzz_enum!(ctx, "WEMUX", ["WE", "CE"], [
                    (mode "SLICEM"),
                    (attr "A6LUT", "#RAM:0"),
                    (attr "A6RAMMODE", "SPRAM64"),
                    (pin "WE"),
                    (pin "CE")
                ]);
                for attr in ["WA7USED", "WA8USED"] {
                    fuzz_enum!(ctx, attr, ["0"], [
                        (mode "SLICEM"),
                        (attr "A6LUT", "#RAM:0"),
                        (attr "A6RAMMODE", "SPRAM64"),
                        (pin "AX"),
                        (pin "BX"),
                        (pin "CX"),
                        (pin "DX")
                    ]);
                }
                if matches!(mode, Mode::Virtex5 | Mode::Spartan6) {
                    fuzz_enum!(ctx, "ADI1MUX", ["AX", "BMC31", "BDI1"], [
                        (mode "SLICEM"),
                        (attr "A6LUT", "#RAM:0"),
                        (attr "A6RAMMODE", "SPRAM64"),
                        (pin "AX")
                    ]);
                    fuzz_enum!(ctx, "BDI1MUX", ["BX", "CMC31", "DX"], [
                        (mode "SLICEM"),
                        (attr "B6LUT", "#RAM:0"),
                        (attr "B6RAMMODE", "SPRAM64"),
                        (pin "BX"),
                        (pin "DX")
                    ]);
                    fuzz_enum!(ctx, "CDI1MUX", ["CX", "DMC31", "DX"], [
                        (mode "SLICEM"),
                        (attr "C6LUT", "#RAM:0"),
                        (attr "C6RAMMODE", "SPRAM64"),
                        (pin "CX"),
                        (pin "DX")
                    ]);
                } else {
                    fuzz_enum!(ctx, "ADI1MUX", ["AI", "BMC31", "BDI1"], [
                        (mode "SLICEM"),
                        (attr "A6LUT", "#RAM:0"),
                        (attr "A6RAMMODE", "SPRAM64"),
                        (pin "AI")
                    ]);
                    fuzz_enum!(ctx, "BDI1MUX", ["BI", "CMC31", "DI"], [
                        (mode "SLICEM"),
                        (attr "B6LUT", "#RAM:0"),
                        (attr "B6RAMMODE", "SPRAM64"),
                        (pin "BI"),
                        (pin "DI")
                    ]);
                    fuzz_enum!(ctx, "CDI1MUX", ["CI", "DMC31", "DI"], [
                        (mode "SLICEM"),
                        (attr "C6LUT", "#RAM:0"),
                        (attr "C6RAMMODE", "SPRAM64"),
                        (pin "CI"),
                        (pin "DI")
                    ]);
                }
                fuzz_enum!(ctx, "A6RAMMODE", ["SPRAM32", "SPRAM64", "DPRAM32", "DPRAM64", "SRL16", "SRL32"], [
                    (mode "SLICEM"),
                    (attr "A6LUT", "#RAM:0")
                ]);
                fuzz_enum!(ctx, "B6RAMMODE", ["SPRAM32", "SPRAM64", "DPRAM32", "DPRAM64", "SRL16", "SRL32"], [
                    (mode "SLICEM"),
                    (attr "B6LUT", "#RAM:0")
                ]);
                fuzz_enum!(ctx, "C6RAMMODE", ["SPRAM32", "SPRAM64", "DPRAM32", "DPRAM64", "SRL16", "SRL32"], [
                    (mode "SLICEM"),
                    (attr "C6LUT", "#RAM:0")
                ]);
                fuzz_enum!(ctx, "D6RAMMODE", ["SPRAM32", "SPRAM64", "DPRAM32", "DPRAM64", "SRL16", "SRL32"], [
                    (mode "SLICEM"),
                    (attr "D6LUT", "#RAM:0")
                ]);
            }

            if !is_x {
                // carry chain
                fuzz_enum!(ctx, "ACY0", ["AX", "O5"], [
                    (mode "SLICEL"),
                    (attr "A5LUT", "#LUT:0"),
                    (attr "A6LUT", "#LUT:0"),
                    (attr "COUTUSED", "0"),
                    (pin "AX"),
                    (pin "COUT")
                ]);
                fuzz_enum!(ctx, "BCY0", ["BX", "O5"], [
                    (mode "SLICEL"),
                    (attr "B5LUT", "#LUT:0"),
                    (attr "B6LUT", "#LUT:0"),
                    (attr "COUTUSED", "0"),
                    (pin "BX"),
                    (pin "COUT")
                ]);
                fuzz_enum!(ctx, "CCY0", ["CX", "O5"], [
                    (mode "SLICEL"),
                    (attr "C5LUT", "#LUT:0"),
                    (attr "C6LUT", "#LUT:0"),
                    (attr "COUTUSED", "0"),
                    (pin "CX"),
                    (pin "COUT")
                ]);
                fuzz_enum!(ctx, "DCY0", ["DX", "O5"], [
                    (mode "SLICEL"),
                    (attr "D5LUT", "#LUT:0"),
                    (attr "D6LUT", "#LUT:0"),
                    (attr "COUTUSED", "0"),
                    (pin "DX"),
                    (pin "COUT")
                ]);
                fuzz_enum!(ctx, "PRECYINIT", ["AX", "1", "0"], [
                    (mode "SLICEL"),
                    (attr "COUTUSED", "0"),
                    (pin "AX"),
                    (pin "COUT")
                ]);

                // TODO: CIN special
            }

            // misc muxes
            if is_x {
                fuzz_enum!(ctx, "AOUTMUX", ["A5Q", "O5"], [
                    (mode "SLICEX"),
                    (attr "A6LUT", "#LUT:0"),
                    (attr "A5LUT", "#LUT:0"),
                    (pin "AMUX")
                ]);
                fuzz_enum!(ctx, "BOUTMUX", ["B5Q", "O5"], [
                    (mode "SLICEX"),
                    (attr "B6LUT", "#LUT:0"),
                    (attr "B5LUT", "#LUT:0"),
                    (pin "BMUX")
                ]);
                fuzz_enum!(ctx, "COUTMUX", ["C5Q", "O5"], [
                    (mode "SLICEX"),
                    (attr "C6LUT", "#LUT:0"),
                    (attr "C5LUT", "#LUT:0"),
                    (pin "CMUX")
                ]);
                fuzz_enum!(ctx, "DOUTMUX", ["D5Q", "O5"], [
                    (mode "SLICEX"),
                    (attr "D6LUT", "#LUT:0"),
                    (attr "D5LUT", "#LUT:0"),
                    (pin "DMUX")
                ]);
                fuzz_enum!(ctx, "AFFMUX", ["AX", "O6"], [
                    (mode "SLICEX"),
                    (attr "A6LUT", "#LUT:0"),
                    (attr "AFF", "#FF"),
                    (pin "AX"),
                    (pin "AQ"),
                    (pin "CLK")
                ]);
                fuzz_enum!(ctx, "BFFMUX", ["BX", "O6"], [
                    (mode "SLICEX"),
                    (attr "B6LUT", "#LUT:0"),
                    (attr "BFF", "#FF"),
                    (pin "BX"),
                    (pin "BQ"),
                    (pin "CLK")
                ]);
                fuzz_enum!(ctx, "CFFMUX", ["CX", "O6"], [
                    (mode "SLICEX"),
                    (attr "C6LUT", "#LUT:0"),
                    (attr "CFF", "#FF"),
                    (pin "CX"),
                    (pin "CQ"),
                    (pin "CLK")
                ]);
                fuzz_enum!(ctx, "DFFMUX", ["DX", "O6"], [
                    (mode "SLICEX"),
                    (attr "D6LUT", "#LUT:0"),
                    (attr "DFF", "#FF"),
                    (pin "DX"),
                    (pin "DQ"),
                    (pin "CLK")
                ]);
            } else {
                // [ABCD]MUX
                if mode == Mode::Virtex5 {
                    fuzz_enum!(ctx, "AOUTMUX", ["O5", "O6", "XOR", "CY", "F7"], [
                        (mode "SLICEL"),
                        (attr "A6LUT", "#LUT:0"),
                        (attr "A5LUT", "#LUT:0"),
                        (pin "AMUX")
                    ]);
                    fuzz_enum!(ctx, "BOUTMUX", ["O5", "O6", "XOR", "CY", "F8"], [
                        (mode "SLICEL"),
                        (attr "B6LUT", "#LUT:0"),
                        (attr "B5LUT", "#LUT:0"),
                        (pin "BMUX")
                    ]);
                    fuzz_enum!(ctx, "COUTMUX", ["O5", "O6", "XOR", "CY", "F7"], [
                        (mode "SLICEL"),
                        (attr "C6LUT", "#LUT:0"),
                        (attr "C5LUT", "#LUT:0"),
                        (pin "CMUX")
                    ]);
                    if is_m {
                        fuzz_enum!(ctx, "DOUTMUX", ["O5", "O6", "XOR", "CY", "MC31"], [
                            (mode "SLICEM"),
                            (attr "A6LUT", "#LUT:0"),
                            (attr "D6LUT", "#LUT:0"),
                            (attr "D5LUT", "#LUT:0"),
                            (pin "DMUX")
                        ]);
                    } else {
                        fuzz_enum!(ctx, "DOUTMUX", ["O5", "O6", "XOR", "CY"], [
                            (mode "SLICEL"),
                            (attr "D6LUT", "#LUT:0"),
                            (attr "D5LUT", "#LUT:0"),
                            (pin "DMUX")
                        ]);
                    }
                } else {
                    fuzz_enum!(ctx, "AOUTMUX", ["O5", "O6", "XOR", "CY", "A5Q", "F7"], [
                        (mode "SLICEL"),
                        (attr "A6LUT", "#LUT:0"),
                        (attr "A5LUT", "#LUT:0"),
                        (attr "A5FFMUX", ""),
                        (attr "CLKINV", "CLK"),
                        (pin "AMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "BOUTMUX", ["O5", "O6", "XOR", "CY", "B5Q", "F8"], [
                        (mode "SLICEL"),
                        (attr "B6LUT", "#LUT:0"),
                        (attr "B5LUT", "#LUT:0"),
                        (attr "B5FFMUX", ""),
                        (attr "CLKINV", "CLK"),
                        (pin "BMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "COUTMUX", ["O5", "O6", "XOR", "CY", "C5Q", "F7"], [
                        (mode "SLICEL"),
                        (attr "C6LUT", "#LUT:0"),
                        (attr "C5LUT", "#LUT:0"),
                        (attr "C5FFMUX", ""),
                        (attr "CLKINV", "CLK"),
                        (pin "CMUX"),
                        (pin "CLK")
                    ]);
                    if is_m {
                        fuzz_enum!(ctx, "DOUTMUX", ["O5", "O6", "XOR", "CY", "D5Q", "MC31"], [
                            (mode "SLICEM"),
                            (attr "A6LUT", "#LUT:0"),
                            (attr "D6LUT", "#LUT:0"),
                            (attr "D5LUT", "#LUT:0"),
                            (attr "D5FFMUX", ""),
                            (attr "CLKINV", "CLK"),
                            (pin "DMUX"),
                            (pin "CLK")
                        ]);
                    } else {
                        fuzz_enum!(ctx, "DOUTMUX", ["O5", "O6", "XOR", "CY", "D5Q"], [
                            (mode "SLICEL"),
                            (attr "D6LUT", "#LUT:0"),
                            (attr "D5LUT", "#LUT:0"),
                            (attr "D5FFMUX", ""),
                            (attr "CLKINV", "CLK"),
                            (pin "DMUX"),
                            (pin "CLK")
                        ]);
                    }
                }

                // [ABCD]FF input
                fuzz_enum!(ctx, "AFFMUX", ["O5", "O6", "XOR", "CY", "AX", "F7"], [
                    (mode "SLICEL"),
                    (attr "A6LUT", "#LUT:0"),
                    (attr "A5LUT", "#LUT:0"),
                    (attr "AFF", "#FF"),
                    (attr "CLKINV", "CLK"),
                    (pin "AX"),
                    (pin "AQ"),
                    (pin "CLK")
                ]);
                fuzz_enum!(ctx, "BFFMUX", ["O5", "O6", "XOR", "CY", "BX", "F8"], [
                    (mode "SLICEL"),
                    (attr "B6LUT", "#LUT:0"),
                    (attr "B5LUT", "#LUT:0"),
                    (attr "BFF", "#FF"),
                    (attr "CLKINV", "CLK"),
                    (pin "BX"),
                    (pin "BQ"),
                    (pin "CLK")
                ]);
                fuzz_enum!(ctx, "CFFMUX", ["O5", "O6", "XOR", "CY", "CX", "F7"], [
                    (mode "SLICEL"),
                    (attr "C6LUT", "#LUT:0"),
                    (attr "C5LUT", "#LUT:0"),
                    (attr "CFF", "#FF"),
                    (attr "CLKINV", "CLK"),
                    (pin "CX"),
                    (pin "CQ"),
                    (pin "CLK")
                ]);
                if is_m {
                    fuzz_enum!(ctx, "DFFMUX", ["O5", "O6", "XOR", "CY", "DX", "MC31"], [
                        (mode "SLICEM"),
                        (attr "A6LUT", "#LUT:0"),
                        (attr "D6LUT", "#LUT:0"),
                        (attr "D5LUT", "#LUT:0"),
                        (attr "DFF", "#FF"),
                        (attr "CLKINV", "CLK"),
                        (pin "DX"),
                        (pin "DQ"),
                        (pin "CLK")
                    ]);
                } else {
                    fuzz_enum!(ctx, "DFFMUX", ["O5", "O6", "XOR", "CY", "DX"], [
                        (mode "SLICEL"),
                        (attr "D6LUT", "#LUT:0"),
                        (attr "D5LUT", "#LUT:0"),
                        (attr "DFF", "#FF"),
                        (attr "CLKINV", "CLK"),
                        (pin "DX"),
                        (pin "DQ"),
                        (pin "CLK")
                    ]);
                }
                if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                    fuzz_enum!(ctx, "A5FFMUX", ["IN_A", "IN_B"], [
                        (mode "SLICEL"),
                        (attr "A6LUT", "#LUT:0"),
                        (attr "A5LUT", "#LUT:0"),
                        (attr "AOUTMUX", "A5Q"),
                        (attr "CLKINV", "CLK"),
                        (pin "AX"),
                        (pin "AMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "B5FFMUX", ["IN_A", "IN_B"], [
                        (mode "SLICEL"),
                        (attr "B6LUT", "#LUT:0"),
                        (attr "B5LUT", "#LUT:0"),
                        (attr "BOUTMUX", "B5Q"),
                        (attr "CLKINV", "CLK"),
                        (pin "BX"),
                        (pin "BMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "C5FFMUX", ["IN_A", "IN_B"], [
                        (mode "SLICEL"),
                        (attr "C6LUT", "#LUT:0"),
                        (attr "C5LUT", "#LUT:0"),
                        (attr "COUTMUX", "C5Q"),
                        (attr "CLKINV", "CLK"),
                        (pin "CX"),
                        (pin "CMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "D5FFMUX", ["IN_A", "IN_B"], [
                        (mode "SLICEL"),
                        (attr "D6LUT", "#LUT:0"),
                        (attr "D5LUT", "#LUT:0"),
                        (attr "DOUTMUX", "D5Q"),
                        (attr "CLKINV", "CLK"),
                        (pin "DX"),
                        (pin "DMUX"),
                        (pin "CLK")
                    ]);
                }
            }

            // FFs
            fuzz_enum!(ctx, "SYNC_ATTR", ["SYNC", "ASYNC"], [
                (mode bk_x),
                (attr "AFF", "#FF"),
                (pin "AQ")
            ]);
            fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [
                (mode bk_x),
                (attr "AFF", "#FF"),
                (pin "AQ"),
                (pin "CLK")
            ]);
            match mode {
                Mode::Virtex5 => {
                    fuzz_enum!(ctx, "REVUSED", ["0"], [
                        (mode bk_x),
                        (attr "AFF", "#FF"),
                        (pin "AQ"),
                        (pin "DX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "AFF", ["#LATCH", "#FF"], [
                        (mode bk_x),
                        (attr "AFFINIT", "INIT1"),
                        (attr "BFF", ""),
                        (attr "CFF", ""),
                        (attr "DFF", ""),
                        (pin "AQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "BFF", ["#LATCH", "#FF"], [
                        (mode bk_x),
                        (attr "BFFINIT", "INIT1"),
                        (attr "AFF", ""),
                        (attr "CFF", ""),
                        (attr "DFF", ""),
                        (pin "BQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "CFF", ["#LATCH", "#FF"], [
                        (mode bk_x),
                        (attr "CFFINIT", "INIT1"),
                        (attr "AFF", ""),
                        (attr "BFF", ""),
                        (attr "DFF", ""),
                        (pin "CQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "DFF", ["#LATCH", "#FF"], [
                        (mode bk_x),
                        (attr "DFFINIT", "INIT1"),
                        (attr "AFF", ""),
                        (attr "BFF", ""),
                        (attr "CFF", ""),
                        (pin "DQ"),
                        (pin "CLK")
                    ]);
                    for attr in ["AFFSR", "BFFSR", "CFFSR", "DFFSR"] {
                        fuzz_enum!(ctx, attr, ["SRHIGH", "SRLOW"], [
                            (mode bk_x),
                            (attr "AFF", "#FF"),
                            (attr "BFF", "#FF"),
                            (attr "CFF", "#FF"),
                            (attr "DFF", "#FF"),
                            (attr "AFFINIT", "INIT0"),
                            (attr "BFFINIT", "INIT0"),
                            (attr "CFFINIT", "INIT0"),
                            (attr "DFFINIT", "INIT0"),
                            (pin "AQ"),
                            (pin "BQ"),
                            (pin "CQ"),
                            (pin "DQ"),
                            (pin "CLK")
                        ]);
                    }
                    for attr in ["AFFINIT", "BFFINIT", "CFFINIT", "DFFINIT"] {
                        fuzz_enum!(ctx, attr, ["INIT0", "INIT1"], [
                            (mode bk_x),
                            (attr "AFF", "#FF"),
                            (attr "BFF", "#FF"),
                            (attr "CFF", "#FF"),
                            (attr "DFF", "#FF"),
                            (attr "AFFSR", "SRLOW"),
                            (attr "BFFSR", "SRLOW"),
                            (attr "CFFSR", "SRLOW"),
                            (attr "DFFSR", "SRLOW"),
                            (pin "AQ"),
                            (pin "BQ"),
                            (pin "CQ"),
                            (pin "DQ"),
                            (pin "CLK")
                        ]);
                    }
                }
                Mode::Spartan6 => {
                    fuzz_enum!(ctx, "AFF", ["#LATCH", "#FF", "AND2L", "OR2L"], [
                        (mode bk_x),
                        (attr "BFF", ""),
                        (attr "CFF", ""),
                        (attr "DFF", ""),
                        (pin "AQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "BFF", ["#LATCH", "#FF", "AND2L", "OR2L"], [
                        (mode bk_x),
                        (attr "AFF", ""),
                        (attr "CFF", ""),
                        (attr "DFF", ""),
                        (pin "BQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "CFF", ["#LATCH", "#FF", "AND2L", "OR2L"], [
                        (mode bk_x),
                        (attr "AFF", ""),
                        (attr "BFF", ""),
                        (attr "DFF", ""),
                        (pin "CQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "DFF", ["#LATCH", "#FF", "AND2L", "OR2L"], [
                        (mode bk_x),
                        (attr "AFF", ""),
                        (attr "BFF", ""),
                        (attr "CFF", ""),
                        (pin "DQ"),
                        (pin "CLK")
                    ]);
                    for attr in ["AFFSRINIT", "BFFSRINIT", "CFFSRINIT", "DFFSRINIT"] {
                        fuzz_enum!(ctx, attr, ["SRINIT0", "SRINIT1"], [
                            (mode bk_x),
                            (attr "AFF", "#FF"),
                            (attr "BFF", "#FF"),
                            (attr "CFF", "#FF"),
                            (attr "DFF", "#FF"),
                            (pin "AQ"),
                            (pin "BQ"),
                            (pin "CQ"),
                            (pin "DQ"),
                            (pin "CLK")
                        ]);
                    }
                    fuzz_enum!(ctx, "A5FFSRINIT", ["SRINIT0", "SRINIT1"], [
                        (mode bk_x),
                        (attr "AOUTMUX", "A5Q"),
                        (attr "A5LUT", "#LUT:0"),
                        (attr "A6LUT", "#LUT:0"),
                        (pin "AMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "B5FFSRINIT", ["SRINIT0", "SRINIT1"], [
                        (mode bk_x),
                        (attr "BOUTMUX", "B5Q"),
                        (attr "B5LUT", "#LUT:0"),
                        (attr "B6LUT", "#LUT:0"),
                        (pin "BMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "C5FFSRINIT", ["SRINIT0", "SRINIT1"], [
                        (mode bk_x),
                        (attr "COUTMUX", "C5Q"),
                        (attr "C5LUT", "#LUT:0"),
                        (attr "C6LUT", "#LUT:0"),
                        (pin "CMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "D5FFSRINIT", ["SRINIT0", "SRINIT1"], [
                        (mode bk_x),
                        (attr "DOUTMUX", "D5Q"),
                        (attr "D5LUT", "#LUT:0"),
                        (attr "D6LUT", "#LUT:0"),
                        (pin "DMUX"),
                        (pin "CLK")
                    ]);
                }
                Mode::Virtex6 | Mode::Virtex7 => {
                    fuzz_enum!(ctx, "AFF", ["#LATCH", "#FF", "AND2L", "OR2L"], [
                        (mode bk_x),
                        (attr "AFFINIT", "INIT1"),
                        (attr "BFF", ""),
                        (attr "CFF", ""),
                        (attr "DFF", ""),
                        (pin "AQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "BFF", ["#LATCH", "#FF", "AND2L", "OR2L"], [
                        (mode bk_x),
                        (attr "BFFINIT", "INIT1"),
                        (attr "AFF", ""),
                        (attr "CFF", ""),
                        (attr "DFF", ""),
                        (pin "BQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "CFF", ["#LATCH", "#FF", "AND2L", "OR2L"], [
                        (mode bk_x),
                        (attr "CFFINIT", "INIT1"),
                        (attr "AFF", ""),
                        (attr "BFF", ""),
                        (attr "DFF", ""),
                        (pin "CQ"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "DFF", ["#LATCH", "#FF", "AND2L", "OR2L"], [
                        (mode bk_x),
                        (attr "DFFINIT", "INIT1"),
                        (attr "AFF", ""),
                        (attr "BFF", ""),
                        (attr "CFF", ""),
                        (pin "DQ"),
                        (pin "CLK")
                    ]);

                    for attr in ["AFFSR", "BFFSR", "CFFSR", "DFFSR"] {
                        fuzz_enum!(ctx, attr, ["SRHIGH", "SRLOW"], [
                            (mode bk_x),
                            (attr "AFF", "#FF"),
                            (attr "BFF", "#FF"),
                            (attr "CFF", "#FF"),
                            (attr "DFF", "#FF"),
                            (attr "AFFINIT", "INIT0"),
                            (attr "BFFINIT", "INIT0"),
                            (attr "CFFINIT", "INIT0"),
                            (attr "DFFINIT", "INIT0"),
                            (pin "AQ"),
                            (pin "BQ"),
                            (pin "CQ"),
                            (pin "DQ"),
                            (pin "CLK")
                        ]);
                    }
                    for attr in ["AFFINIT", "BFFINIT", "CFFINIT", "DFFINIT"] {
                        fuzz_enum!(ctx, attr, ["INIT0", "INIT1"], [
                            (mode bk_x),
                            (attr "AFF", "#FF"),
                            (attr "BFF", "#FF"),
                            (attr "CFF", "#FF"),
                            (attr "DFF", "#FF"),
                            (attr "AFFSR", "SRLOW"),
                            (attr "BFFSR", "SRLOW"),
                            (attr "CFFSR", "SRLOW"),
                            (attr "DFFSR", "SRLOW"),
                            (pin "AQ"),
                            (pin "BQ"),
                            (pin "CQ"),
                            (pin "DQ"),
                            (pin "CLK")
                        ]);
                    }
                    fuzz_enum!(ctx, "A5FFSR", ["SRLOW", "SRHIGH"], [
                        (mode bk_x),
                        (attr "AOUTMUX", "A5Q"),
                        (attr "A5LUT", "#LUT:0"),
                        (attr "A6LUT", "#LUT:0"),
                        (attr "A5FFMUX", "IN_A"),
                        (attr "A5FFINIT", "INIT0"),
                        (pin "AMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "B5FFSR", ["SRLOW", "SRHIGH"], [
                        (mode bk_x),
                        (attr "BOUTMUX", "B5Q"),
                        (attr "B5LUT", "#LUT:0"),
                        (attr "B6LUT", "#LUT:0"),
                        (attr "B5FFMUX", "IN_A"),
                        (attr "B5FFINIT", "INIT0"),
                        (pin "BMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "C5FFSR", ["SRLOW", "SRHIGH"], [
                        (mode bk_x),
                        (attr "COUTMUX", "C5Q"),
                        (attr "C5LUT", "#LUT:0"),
                        (attr "C6LUT", "#LUT:0"),
                        (attr "C5FFMUX", "IN_A"),
                        (attr "C5FFINIT", "INIT0"),
                        (pin "CMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "D5FFSR", ["SRLOW", "SRHIGH"], [
                        (mode bk_x),
                        (attr "DOUTMUX", "D5Q"),
                        (attr "D5LUT", "#LUT:0"),
                        (attr "D6LUT", "#LUT:0"),
                        (attr "D5FFMUX", "IN_A"),
                        (attr "D5FFINIT", "INIT0"),
                        (pin "DMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "A5FFINIT", ["INIT0", "INIT1"], [
                        (mode bk_x),
                        (attr "AOUTMUX", "A5Q"),
                        (attr "A5LUT", "#LUT:0"),
                        (attr "A6LUT", "#LUT:0"),
                        (attr "A5FFMUX", "IN_A"),
                        (attr "A5FFSR", "SRLOW"),
                        (pin "AMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "B5FFINIT", ["INIT0", "INIT1"], [
                        (mode bk_x),
                        (attr "BOUTMUX", "B5Q"),
                        (attr "B5LUT", "#LUT:0"),
                        (attr "B6LUT", "#LUT:0"),
                        (attr "B5FFMUX", "IN_A"),
                        (attr "B5FFSR", "SRLOW"),
                        (pin "BMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "C5FFINIT", ["INIT0", "INIT1"], [
                        (mode bk_x),
                        (attr "COUTMUX", "C5Q"),
                        (attr "C5LUT", "#LUT:0"),
                        (attr "C6LUT", "#LUT:0"),
                        (attr "C5FFMUX", "IN_A"),
                        (attr "C5FFSR", "SRLOW"),
                        (pin "CMUX"),
                        (pin "CLK")
                    ]);
                    fuzz_enum!(ctx, "D5FFINIT", ["INIT0", "INIT1"], [
                        (mode bk_x),
                        (attr "DOUTMUX", "D5Q"),
                        (attr "D5LUT", "#LUT:0"),
                        (attr "D6LUT", "#LUT:0"),
                        (attr "D5FFMUX", "IN_A"),
                        (attr "D5FFSR", "SRLOW"),
                        (pin "DMUX"),
                        (pin "CLK")
                    ]);
                }
            }
            if matches!(mode, Mode::Virtex5 | Mode::Spartan6) {
                fuzz_enum!(ctx, "CEUSED", ["0"], [
                    (mode bk_x),
                    (attr "AFF", "#FF"),
                    (pin "AQ"),
                    (pin "CE"),
                    (pin "CLK")
                ]);
                fuzz_enum!(ctx, "SRUSED", ["0"], [
                    (mode bk_x),
                    (attr "AFF", "#FF"),
                    (pin "AQ"),
                    (pin "SR"),
                    (pin "CLK")
                ]);
            } else {
                fuzz_enum!(ctx, "CEUSEDMUX", ["1", "IN"], [
                    (mode bk_x),
                    (attr "AFF", "#FF"),
                    (pin "AQ"),
                    (pin "CE"),
                    (pin "CLK")
                ]);
                fuzz_enum!(ctx, "SRUSEDMUX", ["0", "IN"], [
                    (mode bk_x),
                    (attr "AFF", "#FF"),
                    (pin "AQ"),
                    (pin "SR"),
                    (pin "CLK")
                ]);
            }
        }
    }
}

pub fn collect_fuzzers(state: &mut State, tiledb: &mut TileDb, mode: Mode) {
    for tile_name in if mode == Mode::Spartan6 {
        ["CLEXL", "CLEXM"]
    } else {
        ["CLBLL", "CLBLM"]
    } {
        for (idx, bel) in ["SLICE0", "SLICE1"].into_iter().enumerate() {
            let is_x = idx == 1 && mode == Mode::Spartan6;
            let is_m = idx == 0 && tile_name.ends_with('M');

            // LUTs
            collect_bitvec(state, tiledb, tile_name, bel, "A6LUT", "#LUT");
            collect_bitvec(state, tiledb, tile_name, bel, "B6LUT", "#LUT");
            collect_bitvec(state, tiledb, tile_name, bel, "C6LUT", "#LUT");
            collect_bitvec(state, tiledb, tile_name, bel, "D6LUT", "#LUT");

            // LUT RAM
            if is_m {
                collect_enum(state, tiledb, tile_name, bel, "WEMUX", &["WE", "CE"]);
                for attr in ["WA7USED", "WA8USED"] {
                    let diff = state.get_diff(tile_name, bel, attr, "0");
                    tiledb.insert(tile_name, bel, attr, xlat_bitvec(vec![diff]));
                }
                let di_muxes = match mode {
                    Mode::Virtex5 | Mode::Spartan6 => [
                        ("ADI1MUX", "AX", "BMC31", "BDI1"),
                        ("BDI1MUX", "BX", "CMC31", "DX"),
                        ("CDI1MUX", "CX", "DMC31", "DX"),
                    ],
                    Mode::Virtex6 | Mode::Virtex7 => [
                        ("ADI1MUX", "AI", "BMC31", "BDI1"),
                        ("BDI1MUX", "BI", "CMC31", "DI"),
                        ("CDI1MUX", "CI", "DMC31", "DI"),
                    ],
                };
                for (attr, byp, alt_shift, alt_ram) in di_muxes {
                    let d_byp = state.get_diff(tile_name, bel, attr, byp);
                    let d_alt = state.get_diff(tile_name, bel, attr, alt_shift);
                    assert_eq!(d_alt, state.get_diff(tile_name, bel, attr, alt_ram));
                    tiledb.insert(
                        tile_name,
                        bel,
                        attr,
                        xlat_enum(vec![(byp.to_string(), d_byp), ("ALT".to_string(), d_alt)]),
                    );
                }
                for (dattr, sattr) in [
                    ("ARAMMODE", "A6RAMMODE"),
                    ("BRAMMODE", "B6RAMMODE"),
                    ("CRAMMODE", "C6RAMMODE"),
                    ("DRAMMODE", "D6RAMMODE"),
                ] {
                    let d_ram32 = state.get_diff(tile_name, bel, sattr, "SPRAM32");
                    let d_ram64 = state.get_diff(tile_name, bel, sattr, "SPRAM64");
                    let d_srl16 = state.get_diff(tile_name, bel, sattr, "SRL16");
                    let d_srl32 = state.get_diff(tile_name, bel, sattr, "SRL32");
                    assert_eq!(d_ram32, state.get_diff(tile_name, bel, sattr, "DPRAM32"));
                    assert_eq!(d_ram64, state.get_diff(tile_name, bel, sattr, "DPRAM64"));
                    tiledb.insert(
                        tile_name,
                        bel,
                        dattr,
                        xlat_enum(vec![
                            ("RAM32".to_string(), d_ram32),
                            ("RAM64".to_string(), d_ram64),
                            ("SRL16".to_string(), d_srl16),
                            ("SRL32".to_string(), d_srl32),
                        ]),
                    );
                }
            }

            // carry chain
            if !is_x {
                collect_enum(state, tiledb, tile_name, bel, "ACY0", &["O5", "AX"]);
                collect_enum(state, tiledb, tile_name, bel, "BCY0", &["O5", "BX"]);
                collect_enum(state, tiledb, tile_name, bel, "CCY0", &["O5", "CX"]);
                collect_enum(state, tiledb, tile_name, bel, "DCY0", &["O5", "DX"]);
                collect_enum(
                    state,
                    tiledb,
                    tile_name,
                    bel,
                    "PRECYINIT",
                    &["AX", "1", "0"],
                );
            }

            // misc muxes
            if is_x {
                collect_enum(state, tiledb, tile_name, bel, "AOUTMUX", &["O5", "A5Q"]);
                collect_enum(state, tiledb, tile_name, bel, "BOUTMUX", &["O5", "B5Q"]);
                collect_enum(state, tiledb, tile_name, bel, "COUTMUX", &["O5", "C5Q"]);
                collect_enum(state, tiledb, tile_name, bel, "DOUTMUX", &["O5", "D5Q"]);
                collect_enum(state, tiledb, tile_name, bel, "AFFMUX", &["O6", "AX"]);
                collect_enum(state, tiledb, tile_name, bel, "BFFMUX", &["O6", "BX"]);
                collect_enum(state, tiledb, tile_name, bel, "CFFMUX", &["O6", "CX"]);
                collect_enum(state, tiledb, tile_name, bel, "DFFMUX", &["O6", "DX"]);
            } else {
                if mode == Mode::Virtex5 {
                    collect_enum(
                        state,
                        tiledb,
                        tile_name,
                        bel,
                        "AOUTMUX",
                        &["O5", "O6", "XOR", "CY", "F7"],
                    );
                    collect_enum(
                        state,
                        tiledb,
                        tile_name,
                        bel,
                        "BOUTMUX",
                        &["O5", "O6", "XOR", "CY", "F8"],
                    );
                    collect_enum(
                        state,
                        tiledb,
                        tile_name,
                        bel,
                        "COUTMUX",
                        &["O5", "O6", "XOR", "CY", "F7"],
                    );
                    if is_m {
                        collect_enum(
                            state,
                            tiledb,
                            tile_name,
                            bel,
                            "DOUTMUX",
                            &["O5", "O6", "XOR", "CY", "MC31"],
                        );
                    } else {
                        collect_enum(
                            state,
                            tiledb,
                            tile_name,
                            bel,
                            "DOUTMUX",
                            &["O5", "O6", "XOR", "CY"],
                        );
                    }
                } else {
                    collect_enum(
                        state,
                        tiledb,
                        tile_name,
                        bel,
                        "AOUTMUX",
                        &["O5", "O6", "XOR", "CY", "A5Q", "F7"],
                    );
                    collect_enum(
                        state,
                        tiledb,
                        tile_name,
                        bel,
                        "BOUTMUX",
                        &["O5", "O6", "XOR", "CY", "B5Q", "F8"],
                    );
                    collect_enum(
                        state,
                        tiledb,
                        tile_name,
                        bel,
                        "COUTMUX",
                        &["O5", "O6", "XOR", "CY", "C5Q", "F7"],
                    );
                    if is_m {
                        collect_enum(
                            state,
                            tiledb,
                            tile_name,
                            bel,
                            "DOUTMUX",
                            &["O5", "O6", "XOR", "CY", "D5Q", "MC31"],
                        );
                    } else {
                        collect_enum(
                            state,
                            tiledb,
                            tile_name,
                            bel,
                            "DOUTMUX",
                            &["O5", "O6", "XOR", "CY", "D5Q"],
                        );
                    }
                }
                collect_enum(
                    state,
                    tiledb,
                    tile_name,
                    bel,
                    "AFFMUX",
                    &["O5", "O6", "XOR", "CY", "AX", "F7"],
                );
                collect_enum(
                    state,
                    tiledb,
                    tile_name,
                    bel,
                    "BFFMUX",
                    &["O5", "O6", "XOR", "CY", "BX", "F8"],
                );
                collect_enum(
                    state,
                    tiledb,
                    tile_name,
                    bel,
                    "CFFMUX",
                    &["O5", "O6", "XOR", "CY", "CX", "F7"],
                );
                if is_m {
                    collect_enum(
                        state,
                        tiledb,
                        tile_name,
                        bel,
                        "DFFMUX",
                        &["O5", "O6", "XOR", "CY", "DX", "MC31"],
                    );
                } else {
                    collect_enum(
                        state,
                        tiledb,
                        tile_name,
                        bel,
                        "DFFMUX",
                        &["O5", "O6", "XOR", "CY", "DX"],
                    );
                }
                if matches!(mode, Mode::Virtex6 | Mode::Virtex7) {
                    for (attr, byp) in [
                        ("A5FFMUX", "AX"),
                        ("B5FFMUX", "BX"),
                        ("C5FFMUX", "CX"),
                        ("D5FFMUX", "DX"),
                    ] {
                        let d_o5 = state.get_diff(tile_name, bel, attr, "IN_A");
                        let d_byp = state.get_diff(tile_name, bel, attr, "IN_B");
                        tiledb.insert(
                            tile_name,
                            bel,
                            attr,
                            xlat_enum(vec![("O5".to_string(), d_o5), (byp.to_string(), d_byp)]),
                        );
                    }
                }
            }

            // FFs
            let ff_sync = state.get_diff(tile_name, bel, "SYNC_ATTR", "SYNC");
            state
                .get_diff(tile_name, bel, "SYNC_ATTR", "ASYNC")
                .assert_empty();
            tiledb.insert(tile_name, bel, "FF_SYNC", xlat_bitvec(vec![ff_sync]));
            collect_inv(state, tiledb, tile_name, bel, "CLK");
            if mode == Mode::Virtex5 {
                let revused = state.get_diff(tile_name, bel, "REVUSED", "0");
                tiledb.insert(tile_name, bel, "FF_REV_EN", xlat_bitvec(vec![revused]));
            }
            if matches!(mode, Mode::Virtex5 | Mode::Spartan6) {
                let ceused = state.get_diff(tile_name, bel, "CEUSED", "0");
                tiledb.insert(tile_name, bel, "FF_CE_EN", xlat_bitvec(vec![ceused]));
                let srused = state.get_diff(tile_name, bel, "SRUSED", "0");
                tiledb.insert(tile_name, bel, "FF_SR_EN", xlat_bitvec(vec![srused]));
            } else {
                state
                    .get_diff(tile_name, bel, "CEUSEDMUX", "1")
                    .assert_empty();
                state
                    .get_diff(tile_name, bel, "SRUSEDMUX", "0")
                    .assert_empty();
                let ceused = state.get_diff(tile_name, bel, "CEUSEDMUX", "IN");
                tiledb.insert(tile_name, bel, "FF_CE_EN", xlat_bitvec(vec![ceused]));
                let srused = state.get_diff(tile_name, bel, "SRUSEDMUX", "IN");
                tiledb.insert(tile_name, bel, "FF_SR_EN", xlat_bitvec(vec![srused]));
            }
            if mode != Mode::Virtex6 {
                let ff_latch = state.get_diff(tile_name, bel, "AFF", "#LATCH");
                for attr in ["AFF", "BFF", "CFF", "DFF"] {
                    state.get_diff(tile_name, bel, attr, "#FF").assert_empty();
                    if attr != "AFF" {
                        assert_eq!(ff_latch, state.get_diff(tile_name, bel, attr, "#LATCH"));
                    }
                    if mode != Mode::Virtex5 {
                        assert_eq!(ff_latch, state.get_diff(tile_name, bel, attr, "AND2L"));
                        assert_eq!(ff_latch, state.get_diff(tile_name, bel, attr, "OR2L"));
                    }
                }
                tiledb.insert(tile_name, bel, "FF_LATCH", xlat_bitvec(vec![ff_latch]));
            } else {
                for attr in ["AFF", "BFF", "CFF", "DFF"] {
                    state.get_diff(tile_name, bel, attr, "#FF").assert_empty();
                    let ff_latch = state.get_diff(tile_name, bel, attr, "#LATCH");
                    assert_eq!(ff_latch, state.get_diff(tile_name, bel, attr, "AND2L"));
                    assert_eq!(ff_latch, state.get_diff(tile_name, bel, attr, "OR2L"));
                    tiledb.insert(
                        tile_name,
                        bel,
                        &format!("{attr}_LATCH"),
                        xlat_bitvec(vec![ff_latch]),
                    );
                }
            }
            match mode {
                Mode::Virtex5 => {
                    for attr in ["AFFINIT", "BFFINIT", "CFFINIT", "DFFINIT"] {
                        collect_enum_bool(state, tiledb, tile_name, bel, attr, "INIT0", "INIT1");
                    }
                    for attr in ["AFFSR", "BFFSR", "CFFSR", "DFFSR"] {
                        collect_enum_bool(state, tiledb, tile_name, bel, attr, "SRLOW", "SRHIGH");
                    }
                }
                Mode::Virtex6 | Mode::Virtex7 => {
                    for attr in [
                        "AFFINIT", "BFFINIT", "CFFINIT", "DFFINIT", "A5FFINIT", "B5FFINIT",
                        "C5FFINIT", "D5FFINIT",
                    ] {
                        collect_enum_bool(state, tiledb, tile_name, bel, attr, "INIT0", "INIT1");
                    }
                    for attr in [
                        "AFFSR", "BFFSR", "CFFSR", "DFFSR", "A5FFSR", "B5FFSR", "C5FFSR", "D5FFSR",
                    ] {
                        collect_enum_bool(state, tiledb, tile_name, bel, attr, "SRLOW", "SRHIGH");
                    }
                }
                Mode::Spartan6 => {
                    for attr in [
                        "AFFSRINIT",
                        "BFFSRINIT",
                        "CFFSRINIT",
                        "DFFSRINIT",
                        "A5FFSRINIT",
                        "B5FFSRINIT",
                        "C5FFSRINIT",
                        "D5FFSRINIT",
                    ] {
                        collect_enum_bool(
                            state, tiledb, tile_name, bel, attr, "SRINIT0", "SRINIT1",
                        );
                    }
                }
            }
        }
    }
}
