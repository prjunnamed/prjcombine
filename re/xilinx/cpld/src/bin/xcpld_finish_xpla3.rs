use core::fmt::Debug;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    path::PathBuf,
};

use clap::Parser;
use enum_map::Enum;
use prjcombine_re_xilinx_cpld::{
    bits::{IBufOut, McOut, extract_bitvec, extract_bool, extract_bool_to_enum, extract_enum},
    db::Database,
    device::{Device, JtagPin, PkgPin},
    fuzzdb::{FuzzDb, FuzzDbPart},
    speeddb::SpeedDb,
    types::{CeMuxVal, ClkMuxVal, FbnId, ImuxId, ImuxInput, OeMuxVal, RegMode, Slew, SrMuxVal, Ut},
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{Tile, TileBit, TileItem, TileItemKind},
    cpld::{BlockId, IoCoord, MacrocellCoord, MacrocellId, ProductTermId},
};
use prjcombine_xpla3 as xpla3;
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};
use xpla3::FbColumn;

const JED_MC_BITS_IOB: &[(&str, usize)] = &[
    ("MC_IOB_MUX", 0),
    ("LUT", 0),
    ("LUT", 1),
    ("LUT", 2),
    ("LUT", 3),
    ("IOB_SLEW", 0),
    ("OE_MUX", 0),
    ("OE_MUX", 1),
    ("OE_MUX", 2),
    ("CE_MUX", 0),
    ("CLK_INV", 0),
    ("CLK_MUX", 0),
    ("CLK_MUX", 1),
    ("CLK_MUX", 2),
    ("REG_D_IREG", 0),
    ("REG_D_SHIFT_DIR", 0),
    ("REG_D_SHIFT", 0),
    ("IOB_ZIA_MUX", 0),
    ("RST_MUX", 0),
    ("RST_MUX", 1),
    ("RST_MUX", 2),
    ("SET_MUX", 0),
    ("SET_MUX", 1),
    ("SET_MUX", 2),
    ("REG_MODE", 0),
    ("REG_MODE", 1),
    ("MC_ZIA_MUX", 0),
];

const JED_MC_BITS_BURIED: &[(&str, usize)] = &[
    ("LUT", 0),
    ("LUT", 1),
    ("LUT", 2),
    ("LUT", 3),
    ("CE_MUX", 0),
    ("CLK_INV", 0),
    ("CLK_MUX", 0),
    ("CLK_MUX", 1),
    ("CLK_MUX", 2),
    ("REG_D_IREG", 0),
    ("REG_D_SHIFT_DIR", 0),
    ("REG_D_SHIFT", 0),
    ("RST_MUX", 0),
    ("RST_MUX", 1),
    ("RST_MUX", 2),
    ("SET_MUX", 0),
    ("SET_MUX", 1),
    ("SET_MUX", 2),
    ("REG_MODE", 0),
    ("REG_MODE", 1),
    ("MC_ZIA_MUX", 0),
];

const JED_FB_BITS: &[(&str, usize)] = &[
    ("FCLK_MUX", 0),
    ("FCLK_MUX", 1),
    ("FCLK_MUX", 2),
    ("FCLK_MUX", 3),
    ("LCT0_INV", 0),
    ("LCT1_INV", 0),
    ("LCT2_INV", 0),
    ("LCT3_INV", 0),
    ("LCT4_INV", 0),
    ("LCT5_INV", 0),
    ("LCT6_INV", 0),
    ("LCT7_INV", 0),
];

#[derive(Parser)]
struct Args {
    db: PathBuf,
    fdb: PathBuf,
    sdb: PathBuf,
    out: PathBuf,
    json: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DevData {
    bs_cols: usize,
    fb_rows: usize,
    fb_cols: Vec<FbColumn>,
    imux_width: usize,
    io_mcs: BTreeSet<MacrocellId>,
    io_special: BTreeMap<String, MacrocellCoord>,
}

fn extract_mc_bits(device: &Device, fpart: &FuzzDbPart, dd: &DevData) -> Tile {
    let (plane_bit, row_mask) = match dd.fb_rows {
        1 => (6, 0x3f),
        2 => (7, 0x7f),
        4 => (8, 0xff),
        _ => unreachable!(),
    };
    let mut tile = Tile::new();
    for fbmc in device.mcs() {
        let fb = fbmc.block;
        let mc = fbmc.macrocell;
        let mcb = &fpart.bits.blocks[fb].mcs[mc];
        let fbc = fb.to_idx() / (dd.fb_rows * 2);
        let fbr = fb.to_idx() / 2 % dd.fb_rows;
        let xlat_bit = |bit| {
            let (row, column) = fpart.map.main[bit];
            let row = row as usize;
            let plane = row >> plane_bit & 1;
            let row = row & row_mask;
            let row = row - fbr * 52;
            let row = if mc.to_idx() >= 8 {
                row - mc.to_idx() * 3 - 4
            } else {
                row - mc.to_idx() * 3
            };
            let column = dd.bs_cols - 1 - column;
            let column = if fb.to_idx() % 2 == 0 {
                column - dd.fb_cols[fbc].mc_col
            } else {
                9 - (column - dd.fb_cols[fbc].mc_col)
            };
            TileBit {
                tile: plane,
                frame: row,
                bit: column,
            }
        };
        tile.insert(
            "CLK_MUX",
            extract_enum(
                &mcb.clk_mux,
                |val| match val {
                    ClkMuxVal::Pt => "PT".to_string(),
                    ClkMuxVal::Fclk(which) => format!("FCLK{which:#}"),
                    ClkMuxVal::Ct(which) => format!("LCT{which:#}"),
                    ClkMuxVal::Ut => "UCT3".to_string(),
                },
                xlat_bit,
                "FCLK0",
            ),
            |_| true,
        );
        tile.insert(
            "RST_MUX",
            extract_enum(
                &mcb.rst_mux,
                |val| match val {
                    SrMuxVal::Ct(which) => format!("LCT{which:#}"),
                    SrMuxVal::Ut => "UCT1".to_string(),
                    SrMuxVal::Gnd => "GND".to_string(),
                    _ => unreachable!(),
                },
                xlat_bit,
                "GND",
            ),
            |_| true,
        );
        tile.insert(
            "SET_MUX",
            extract_enum(
                &mcb.set_mux,
                |val| match val {
                    SrMuxVal::Ct(which) => format!("LCT{which:#}"),
                    SrMuxVal::Ut => "UCT2".to_string(),
                    SrMuxVal::Gnd => "GND".to_string(),
                    _ => unreachable!(),
                },
                xlat_bit,
                "GND",
            ),
            |_| true,
        );
        tile.insert(
            "CE_MUX",
            extract_enum(
                mcb.ce_mux.as_ref().unwrap(),
                |val| match val {
                    CeMuxVal::Pt => "PT".to_string(),
                    CeMuxVal::Ct(which) => format!("LCT{which:#}"),
                    _ => unreachable!(),
                },
                xlat_bit,
                "LCT4",
            ),
            |_| true,
        );
        tile.insert(
            "CLK_INV",
            extract_bool(mcb.clk_inv.unwrap(), xlat_bit),
            |_| true,
        );
        tile.insert("LUT", extract_bitvec(&mcb.lut.unwrap(), xlat_bit), |_| true);

        tile.insert(
            "REG_MODE",
            extract_enum(
                &mcb.reg_mode,
                |val| {
                    match val {
                        RegMode::Dff => "DFF",
                        RegMode::Tff => "TFF",
                        RegMode::Latch => "LATCH",
                        RegMode::DffCe => "DFFCE",
                    }
                    .to_string()
                },
                xlat_bit,
                "DFF",
            ),
            |_| true,
        );
        tile.insert(
            "MC_ZIA_MUX",
            extract_enum(
                mcb.mc_uim_out.as_ref().unwrap(),
                |val| {
                    match val {
                        McOut::Comb => "LUT",
                        McOut::Reg => "REG",
                    }
                    .to_string()
                },
                xlat_bit,
                "REG",
            ),
            |_| true,
        );

        if mcb.slew.is_none() {
            continue;
        }
        tile.insert(
            "REG_D_IREG",
            extract_bool(mcb.use_ireg.unwrap(), xlat_bit),
            |_| true,
        );
        tile.insert(
            "REG_D_SHIFT_DIR",
            extract_bool_to_enum((mcb.use_ireg.unwrap().0 + 1, true), xlat_bit, "DOWN", "UP"),
            |_| true,
        );
        tile.insert(
            "REG_D_SHIFT",
            extract_bool((mcb.use_ireg.unwrap().0 + 2, true), xlat_bit),
            |_| true,
        );
        tile.insert(
            "OE_MUX",
            extract_enum(
                mcb.oe_mux.as_ref().unwrap(),
                |val| match val {
                    OeMuxVal::Ct(which) => format!("LCT{which:#}"),
                    OeMuxVal::Ut => "UCT0".to_string(),
                    OeMuxVal::Gnd => "GND".to_string(),
                    OeMuxVal::Pullup => "PULLUP".to_string(),
                    OeMuxVal::Vcc => "VCC".to_string(),
                    _ => unreachable!(),
                },
                xlat_bit,
                "GND",
            ),
            |_| true,
        );
        tile.insert(
            "MC_IOB_MUX",
            extract_enum(
                mcb.mc_obuf_out.as_ref().unwrap(),
                |val| {
                    match val {
                        McOut::Comb => "LUT",
                        McOut::Reg => "REG",
                    }
                    .to_string()
                },
                xlat_bit,
                "LUT",
            ),
            |_| true,
        );
        tile.insert(
            "IOB_ZIA_MUX",
            extract_enum(
                mcb.ibuf_uim_out.as_ref().unwrap(),
                |val| {
                    match val {
                        IBufOut::Pad => "IBUF",
                        IBufOut::Reg => "REG",
                    }
                    .to_string()
                },
                xlat_bit,
                "IBUF",
            ),
            |_| true,
        );
        tile.insert(
            "IOB_SLEW",
            extract_enum(
                mcb.slew.as_ref().unwrap(),
                |val| {
                    match val {
                        Slew::Slow => "SLOW",
                        Slew::Fast => "FAST",
                    }
                    .to_string()
                },
                xlat_bit,
                "SLOW",
            ),
            |_| true,
        );
    }
    tile
}

fn extract_fb_bits(fpart: &FuzzDbPart, dd: &DevData) -> Tile {
    let (plane_bit, row_mask) = match dd.fb_rows {
        1 => (6, 0x3f),
        2 => (7, 0x7f),
        4 => (8, 0xff),
        _ => unreachable!(),
    };
    let mut tile = Tile::new();
    for fb in fpart.bits.blocks.ids() {
        let fbb = &fpart.bits.blocks[fb];
        let fbc = fb.to_idx() / (dd.fb_rows * 2);
        let fbr = fb.to_idx() / 2 % dd.fb_rows;
        let xlat_bit = |bit| {
            let (row, column) = fpart.map.main[bit];
            let row = row as usize;
            let plane = row >> plane_bit & 1;
            let row = row & row_mask;
            let row = row - fbr * 52 - 24;
            let column = dd.bs_cols - 1 - column;
            let column = if fb.to_idx() % 2 == 0 {
                column - dd.fb_cols[fbc].mc_col
            } else {
                9 - (column - dd.fb_cols[fbc].mc_col)
            };
            TileBit {
                tile: plane,
                frame: row,
                bit: column,
            }
        };
        tile.insert(
            "FCLK_MUX",
            extract_enum(
                fbb.fbclk.as_ref().unwrap(),
                |val| match val {
                    (Some(a), Some(b)) => format!("GCLK{a:#}_GCLK{b:#}"),
                    (Some(a), None) => format!("GCLK{a:#}_NONE"),
                    (None, Some(b)) => format!("NONE_GCLK{b:#}"),
                    (None, None) => "NONE".to_string(),
                },
                xlat_bit,
                "NONE",
            ),
            |_| true,
        );
        for i in 0..8 {
            tile.insert(
                format!("LCT{i}_INV"),
                extract_bool(fbb.ct_invert[ProductTermId::from_idx(i)], xlat_bit),
                |_| true,
            );
        }
    }
    tile
}

fn extract_global_bits(device: &Device, fpart: &FuzzDbPart, dd: &DevData) -> Tile {
    let (plane_bit, row_mask) = match dd.fb_rows {
        1 => (6, 0x3f),
        2 => (7, 0x7f),
        4 => (8, 0xff),
        _ => unreachable!(),
    };
    let xlat_bit = |bit| {
        let (row, column) = fpart.map.main[bit];
        let row = row as usize;
        let plane = row >> plane_bit & 1;
        let row = row & row_mask;
        let column = dd.bs_cols - 1 - column;
        TileBit {
            tile: plane,
            frame: row,
            bit: column,
        }
    };
    let xlat_bit_raw = |(row, column)| {
        let row = row as usize;
        let plane = row >> plane_bit & 1;
        let row = row & row_mask;
        let column = dd.bs_cols - 1 - column;
        TileBit {
            tile: plane,
            frame: row,
            bit: column,
        }
    };
    let mut tile = Tile::new();
    for (gclk, &io) in &device.clk_pads {
        let IoCoord::Ipad(ipad) = io else {
            unreachable!()
        };
        for (fbg, &bit) in &fpart.bits.ipads[ipad].uim_out_en {
            tile.insert(
                format!("FB_COL[{fbg:#}].ZIA_GCLK{gclk:#}_ENABLE"),
                extract_bool(bit, xlat_bit),
                |_| true,
            );
        }
    }
    for (idx, ut) in [(0, Ut::Oe), (1, Ut::Rst), (2, Ut::Set), (3, Ut::Clk)] {
        let item = extract_enum(
            &fpart.bits.ut.as_ref().unwrap()[ut.into_usize()],
            |(fb, pt)| format!("FB{fb:#}_LCT{pt:#}"),
            xlat_bit,
            "NONE",
        );
        if device.fbs == 32 {
            let TileItemKind::Enum { values } = &item.kind else {
                unreachable!()
            };
            let split = item.bits.len() / 2;
            for i in 0..2 {
                let subitem = TileItem {
                    bits: item.bits[split * i..split * (i + 1)].to_vec(),
                    kind: TileItemKind::Enum {
                        values: values
                            .iter()
                            .map(|(k, v)| (k.clone(), v.slice(split * i..split * (i + 1))))
                            .collect(),
                    },
                };
                tile.insert(format!("FB_GROUP[{i}].UCT{idx}"), subitem, |_| true);
            }
        } else {
            tile.insert(format!("FB_GROUP[0].UCT{idx}"), item, |_| true);
        }
    }
    tile.insert(
        "ISP_DISABLE",
        extract_bool(fpart.bits.no_isp.unwrap(), xlat_bit),
        |_| true,
    );
    tile.insert(
        "READ_PROT",
        TileItem {
            bits: vec![xlat_bit_raw(fpart.map.rprot[0])],
            kind: TileItemKind::BitVec {
                invert: BitVec::from_iter([true]),
            },
        },
        |_| true,
    );
    tile.insert(
        "UES",
        TileItem {
            bits: fpart
                .map
                .ues
                .as_ref()
                .unwrap()
                .iter()
                .map(|&bit| xlat_bit_raw(bit))
                .collect(),
            kind: TileItemKind::BitVec {
                invert: BitVec::repeat(false, fpart.map.ues.as_ref().unwrap().len()),
            },
        },
        |_| true,
    );
    tile
}

fn extract_jed_global_bits(device: &Device, fpart: &FuzzDbPart) -> Vec<(String, usize)> {
    let mut res = vec![];
    if device.fbs == 32 {
        let bits = fpart.bits.ut.as_ref().unwrap()[Ut::Clk.into_usize()]
            .bits
            .len()
            / 2;
        for i in 0..2 {
            for j in 0..4 {
                for k in 0..bits {
                    res.push((format!("FB_GROUP[{i}].UCT{j}"), k));
                }
            }
        }
    } else {
        let bits = fpart.bits.ut.as_ref().unwrap()[Ut::Clk.into_usize()]
            .bits
            .len();
        for j in 0..4 {
            for k in 0..bits {
                res.push((format!("FB_GROUP[0].UCT{j}"), k));
            }
        }
    }
    for i in 0..device.fb_groups {
        for j in 0..4 {
            res.push((format!("FB_COL[{i}].ZIA_GCLK{j}_ENABLE"), 0));
        }
    }
    res.push(("ISP_DISABLE".to_string(), 0));
    res
}

fn extract_imux_bits(
    data: &mut [BTreeMap<String, BitVec>; 40],
    device: &Device,
    fpart: &FuzzDbPart,
    dd: &DevData,
) {
    let mut ipad_to_gclk = EntityPartVec::new();
    for (gclk, &io) in &device.clk_pads {
        let IoCoord::Ipad(ipad) = io else {
            unreachable!()
        };
        ipad_to_gclk.insert(ipad, gclk);
    }
    for fb in device.fbs() {
        let fbc = fb.to_idx() / 2 / dd.fb_rows;
        for i in 0..40 {
            let imux = ImuxId::from_idx(i);
            let enum_ = &fpart.bits.blocks[fb].imux[imux];
            let xlat: Vec<_> = enum_
                .bits
                .iter()
                .map(|&x| {
                    let col = dd.bs_cols - 1 - fpart.map.main[x].1;
                    let col = col - dd.fb_cols[fbc].imux_col;
                    let col = dd.imux_width - 1 - col;
                    assert!(col < dd.imux_width);
                    col
                })
                .collect();
            for (&val, bits) in &enum_.items {
                let vname = match val {
                    ImuxInput::Ibuf(IoCoord::Ipad(ipad)) => format!("GCLK{:#}", ipad_to_gclk[ipad]),
                    ImuxInput::Ibuf(io) => io.to_string(),
                    ImuxInput::Mc(mc) => format!("MC_{mc}"),
                    ImuxInput::Pup => "STARTUP".to_string(),
                    _ => unreachable!(),
                };
                let mut vbits = BitVec::repeat(true, dd.imux_width);
                for (i, bit) in bits.iter().enumerate() {
                    vbits.set(xlat[i], bit);
                }
                data[i].insert(vname, vbits);
            }
        }
    }
}

fn prep_imux_bits(imux_bits: &[BTreeMap<String, BitVec>; 40], dd: &DevData) -> Tile {
    let mut tile = Tile::new();
    for i in 0..40 {
        let mut values = imux_bits[i].clone();
        values.insert("VCC".to_string(), BitVec::repeat(true, dd.imux_width));
        let item = TileItem {
            bits: (0..dd.imux_width)
                .map(|j| TileBit {
                    tile: 0,
                    frame: if i < 20 { 2 + i } else { 10 + i },
                    bit: dd.imux_width - 1 - j,
                })
                .collect(),
            kind: TileItemKind::Enum { values },
        };
        tile.insert(format!("IM[{i}].MUX"), item, |_| true);
    }
    tile
}

fn verify_jed(
    device: &Device,
    fpart: &FuzzDbPart,
    dd: &DevData,
    mc_bits: &Tile,
    fb_bits: &Tile,
    global_bits: &Tile,
    jed_global_bits: &Vec<(String, usize)>,
) {
    let plane_bit = match dd.fb_rows {
        1 => 6,
        2 => 7,
        4 => 8,
        _ => unreachable!(),
    };
    let check_bit = |pos: &mut usize, col: usize, row: usize, plane: usize| {
        let exp_coord = ((row | plane << plane_bit) as u32, dd.bs_cols - 1 - col);
        assert_eq!(fpart.map.main[*pos], exp_coord);
        *pos += 1;
    };

    let mut pos = 0;
    for fb in device.fbs() {
        let fbc = fb.to_idx() / (dd.fb_rows * 2);
        let fbr = fb.to_idx() / 2 % dd.fb_rows;
        let fb_odd = fb.to_idx() % 2 == 1;
        for i in 0..40 {
            for j in 0..dd.imux_width {
                let exp_col = dd.fb_cols[fbc].imux_col + (dd.imux_width - 1 - j);
                let exp_row = fbr * 52 + if i < 20 { 2 + i } else { 2 + i + 8 };
                let exp_plane = if fb_odd { 0 } else { 1 };
                check_bit(&mut pos, exp_col, exp_row, exp_plane);
            }
        }
        for pt in 0..48 {
            let exp_col = dd.fb_cols[fbc].pt_col + if fb_odd { 95 - pt } else { pt };
            for imux in 0..40 {
                let exp_row = fbr * 52 + if imux < 20 { 2 + imux } else { 2 + imux + 8 };
                assert_eq!(
                    fpart.bits.blocks[fb].pla_and[ProductTermId::from_idx(pt)].imux
                        [ImuxId::from_idx(imux)]
                    .0,
                    (pos, false)
                );
                check_bit(&mut pos, exp_col, exp_row, 0);
                assert_eq!(
                    fpart.bits.blocks[fb].pla_and[ProductTermId::from_idx(pt)].imux
                        [ImuxId::from_idx(imux)]
                    .1,
                    (pos, false)
                );
                check_bit(&mut pos, exp_col, exp_row, 1);
            }
            for fbn in 0..8 {
                let (row, exp_plane) = [
                    (0, 1),
                    (0, 0),
                    (1, 1),
                    (1, 0),
                    (50, 0),
                    (50, 1),
                    (51, 0),
                    (51, 1),
                ][fbn];
                let exp_row = fbr * 52 + row;
                assert_eq!(
                    fpart.bits.blocks[fb].pla_and[ProductTermId::from_idx(pt)].fbn
                        [FbnId::from_idx(fbn)],
                    (pos, false)
                );
                check_bit(&mut pos, exp_col, exp_row, exp_plane);
            }
        }
        for pt in 0..48 {
            let exp_col = dd.fb_cols[fbc].pt_col + if fb_odd { 95 - pt } else { pt };
            for mc in device.fb_mcs() {
                let exp_row = fbr * 52 + 22 + mc.to_idx() / 2;
                let exp_plane = 1 - mc.to_idx() % 2;
                assert_eq!(
                    fpart.bits.blocks[fb].mcs[mc].pla_or[ProductTermId::from_idx(pt)],
                    (pos, false)
                );
                check_bit(&mut pos, exp_col, exp_row, exp_plane);
            }
        }
        for &(name, bit) in JED_FB_BITS {
            let item = &fb_bits.items[name];
            let coord = item.bits[bit];
            let exp_col = dd.fb_cols[fbc].mc_col + if fb_odd { 9 - coord.bit } else { coord.bit };
            let exp_row = fbr * 52 + 24 + coord.frame;
            let exp_plane = coord.tile;
            check_bit(&mut pos, exp_col, exp_row, exp_plane);
        }
        for mc in device.fb_mcs() {
            if !dd.io_mcs.contains(&MacrocellId::from_idx(mc.to_idx())) {
                continue;
            }
            for &(name, bit) in JED_MC_BITS_IOB {
                let item = &mc_bits.items[name];
                let coord = item.bits[bit];
                let exp_col =
                    dd.fb_cols[fbc].mc_col + if fb_odd { 9 - coord.bit } else { coord.bit };
                let exp_row = fbr * 52
                    + if mc.to_idx() < 8 {
                        mc.to_idx() * 3
                    } else {
                        4 + mc.to_idx() * 3
                    }
                    + coord.frame;
                let exp_plane = coord.tile;
                check_bit(&mut pos, exp_col, exp_row, exp_plane);
            }
        }
        for mc in device.fb_mcs() {
            if dd.io_mcs.contains(&MacrocellId::from_idx(mc.to_idx())) {
                continue;
            }
            for &(name, bit) in JED_MC_BITS_BURIED {
                let item = &mc_bits.items[name];
                let coord = item.bits[bit];
                let exp_col =
                    dd.fb_cols[fbc].mc_col + if fb_odd { 9 - coord.bit } else { coord.bit };
                let exp_row = fbr * 52
                    + if mc.to_idx() < 8 {
                        mc.to_idx() * 3
                    } else {
                        4 + mc.to_idx() * 3
                    }
                    + coord.frame;
                let exp_plane = coord.tile;
                check_bit(&mut pos, exp_col, exp_row, exp_plane);
            }
        }
    }
    for (name, bit) in jed_global_bits {
        let item = &global_bits.items[name];
        let coord = item.bits[*bit];
        check_bit(&mut pos, coord.bit, coord.frame, coord.tile);
    }
    assert_eq!(pos, fpart.map.main.len());
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = Database::from_file(args.db)?;
    let fdb = FuzzDb::from_file(args.fdb)?;
    let sdb = SpeedDb::from_file(args.sdb)?;

    let mut dev_data = EntityPartVec::new();
    let mut mc_bits = None;
    let mut fb_bits = None;
    let mut global_bits = EntityPartVec::new();
    let mut imux_bits = db
        .devices
        .map_values(|_| core::array::from_fn(|_| BTreeMap::new()));
    let mut bond_idcode = EntityPartVec::new();

    for fpart in &fdb.parts {
        let part = db
            .parts
            .iter()
            .find(|p| p.dev_name == fpart.dev_name && p.pkg_name == fpart.pkg_name)
            .unwrap();
        let device = &db.devices[part.device].device;
        let imux_width = fpart.bits.blocks[BlockId::from_idx(0)].pla_and
            [ProductTermId::from_idx(0)]
        .imux[ImuxId::from_idx(0)]
        .0
        .0 / 40;
        let bs_cols = fpart.map.dims.unwrap().0;
        let bs_rows = fpart.map.dims.unwrap().1;
        let fb_rows = device.fbs / device.fb_groups / 2;
        assert_eq!(bs_rows, (fb_rows * 52 + 2) * 2);
        let fb_cols: Vec<_> = (0..device.fb_groups)
            .map(|i| {
                let fb = BlockId::from_idx(i * fb_rows * 2);
                let pt_bit = fpart.bits.blocks[fb].pla_and[ProductTermId::from_idx(0)].imux
                    [ImuxId::from_idx(0)]
                .0
                .0;
                let pt_col = bs_cols - 1 - fpart.map.main[pt_bit].1;
                let imux_bit = fpart.bits.blocks[fb].imux[ImuxId::from_idx(0)].bits[0];
                let imux_col = bs_cols - imux_width - fpart.map.main[imux_bit].1;
                let mc_bit = fpart.bits.blocks[fb].ct_invert[ProductTermId::from_idx(0)].0;
                let mc_col = bs_cols - 1 - fpart.map.main[mc_bit].1;
                FbColumn {
                    pt_col,
                    imux_col,
                    mc_col,
                }
            })
            .collect();
        let mut io_special = BTreeMap::new();
        let mut io_mcs = BTreeSet::new();
        for (k, v) in &device.io {
            let IoCoord::Macrocell(mc) = *k else { continue };
            io_mcs.insert(mc.macrocell);
            if let Some(jtag) = v.jtag {
                let spec = match jtag {
                    JtagPin::Tdi => "TDI",
                    JtagPin::Tdo => "TDO",
                    JtagPin::Tck => "TCK",
                    JtagPin::Tms => "TMS",
                }
                .to_string();
                io_special.insert(spec, mc);
            }
        }
        let dd = DevData {
            bs_cols,
            fb_rows,
            fb_cols,
            imux_width,
            io_special,
            io_mcs,
        };
        let mcb = extract_mc_bits(device, fpart, &dd);
        if mc_bits.is_some() {
            assert_eq!(mc_bits, Some(mcb));
        } else {
            mc_bits = Some(mcb);
        }
        let fbb = extract_fb_bits(fpart, &dd);
        if fb_bits.is_some() {
            assert_eq!(fb_bits, Some(fbb));
        } else {
            fb_bits = Some(fbb);
        }
        let gb = extract_global_bits(device, fpart, &dd);
        let jgb = extract_jed_global_bits(device, fpart);
        if global_bits.contains_id(part.device) {
            assert_eq!(global_bits[part.device], (gb, jgb));
        } else {
            global_bits.insert(part.device, (gb, jgb));
        }

        extract_imux_bits(&mut imux_bits[part.device], device, fpart, &dd);

        verify_jed(
            device,
            fpart,
            &dd,
            mc_bits.as_ref().unwrap(),
            fb_bits.as_ref().unwrap(),
            &global_bits[part.device].0,
            &global_bits[part.device].1,
        );

        if dev_data.contains_id(part.device) {
            assert_eq!(dev_data[part.device], dd)
        } else {
            dev_data.insert(part.device, dd);
        }

        let idcode = match (&part.dev_name[..], &part.pkg_name[..]) {
            ("xcr3032xl", "cs48") => 0x480c,
            ("xcr3032xl", "pc44") => 0x480d,
            ("xcr3032xl", "vq44") => 0x480e,
            ("xcr3064xl", "cp56") => 0x4848,
            ("xcr3064xl", "vq100") => 0x4849,
            ("xcr3064xl", "cs48") => 0x484c,
            ("xcr3064xl", "pc44") => 0x484d,
            ("xcr3064xl", "vq44") => 0x484e,
            ("xcr3128xl", "vq100") => 0x4889,
            ("xcr3128xl", "tq144") => 0x488b,
            ("xcr3128xl", "cs144") => 0x488c,
            ("xcr3256xl", "tq144") => 0x494b,
            ("xcr3256xl", "pq208") => 0x494c,
            ("xcr3256xl", "cs280") => 0x494d,
            ("xcr3256xl", "ft256") => 0x494e,
            ("xcr3384xl", "fg324") => 0x495a,
            ("xcr3384xl", "tq144") => 0x495b,
            ("xcr3384xl", "pq208") => 0x495c,
            ("xcr3384xl", "ft256") => 0x495e,
            ("xcr3512xl", "pq208") => 0x497c,
            ("xcr3512xl", "fg324") => 0x497d,
            ("xcr3512xl", "ft256") => 0x497e,
            _ => panic!("unknown {} {}", part.dev_name, part.pkg_name),
        };
        bond_idcode.insert(part.package, idcode);
    }

    let mc_bits = mc_bits.unwrap();
    let fb_bits = fb_bits.unwrap();

    let chips: EntityVec<_, _> = db
        .devices
        .ids()
        .map(|did| {
            let dd = &dev_data[did];
            let imux_bits = prep_imux_bits(&imux_bits[did], dd);
            xpla3::Chip {
                idcode_part: match db.devices[did].device.fbs {
                    2 => 0x4808,
                    4 => 0x4848,
                    8 => 0x4888,
                    16 => 0x4948,
                    24 => 0x4958,
                    32 => 0x4978,
                    _ => unreachable!(),
                },
                io_mcs: dd.io_mcs.clone(),
                io_special: dd.io_special.clone(),
                bs_cols: dd.bs_cols,
                imux_width: dd.imux_width,
                block_rows: dd.fb_rows,
                block_cols: dd.fb_cols.clone(),
                global_bits: global_bits[did].0.clone(),
                jed_global_bits: global_bits[did].1.clone(),
                imux_bits,
            }
        })
        .collect();

    let mut bonds = EntityVec::new();
    let mut speeds = EntityVec::new();
    let mut parts: Vec<xpla3::Part> = vec![];
    'parts: for spart in &db.parts {
        let package = &db.packages[spart.package];
        let chip = spart.device;
        let bond = xpla3::Bond {
            idcode_part: bond_idcode[spart.package],
            pins: package
                .pins
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        match *v {
                            PkgPin::Nc => xpla3::BondPin::Nc,
                            PkgPin::Gnd => xpla3::BondPin::Gnd,
                            PkgPin::VccInt => xpla3::BondPin::Vcc,
                            PkgPin::PortEn => xpla3::BondPin::PortEn,
                            PkgPin::Io(IoCoord::Macrocell(mc)) => xpla3::BondPin::Iob(mc),
                            PkgPin::Io(IoCoord::Ipad(pad)) => {
                                xpla3::BondPin::Gclk(xpla3::GclkId::from_idx(3 - pad.to_idx()))
                            }
                            _ => unreachable!(),
                        },
                    )
                })
                .collect(),
        };
        let bond = 'bid: {
            for (i, x) in &bonds {
                if *x == bond {
                    break 'bid i;
                }
            }
            bonds.push(bond)
        };

        for dpart in &mut parts {
            if dpart.name == spart.dev_name {
                assert_eq!(dpart.chip, chip);
                dpart.packages.insert(spart.pkg_name.clone(), bond);
                continue 'parts;
            }
        }
        parts.push(xpla3::Part {
            name: spart.dev_name.clone(),
            chip,
            packages: [(spart.pkg_name.clone(), bond)].into_iter().collect(),
            speeds: spart
                .speeds
                .iter()
                .map(|sn| {
                    let speed = sdb
                        .parts
                        .iter()
                        .find(|x| x.dev_name == spart.dev_name && &x.speed_name == sn)
                        .unwrap()
                        .speed
                        .clone();
                    let speed = 'sid: {
                        for (i, x) in &speeds {
                            if *x == speed {
                                break 'sid i;
                            }
                        }
                        speeds.push(speed)
                    };
                    (sn.clone(), speed)
                })
                .collect(),
        })
    }

    let database = xpla3::Database {
        chips,
        bonds,
        speeds,
        parts,
        mc_bits,
        block_bits: fb_bits,
        jed_mc_bits_iob: JED_MC_BITS_IOB
            .iter()
            .map(|&(item, bit)| (item.to_string(), bit))
            .collect(),
        jed_mc_bits_buried: JED_MC_BITS_BURIED
            .iter()
            .map(|&(item, bit)| (item.to_string(), bit))
            .collect(),
        jed_block_bits: JED_FB_BITS
            .iter()
            .map(|&(item, bit)| (item.to_string(), bit))
            .collect(),
    };
    database.to_file(args.out)?;

    let json = database.to_json();
    std::fs::write(args.json, json.to_string())?;

    Ok(())
}
