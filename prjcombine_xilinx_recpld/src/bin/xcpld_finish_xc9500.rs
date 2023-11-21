use core::fmt::Debug;
use std::{
    collections::{btree_map, BTreeMap},
    error::Error,
    path::PathBuf,
};

use bitvec::vec::BitVec;
use clap::Parser;
use itertools::Itertools;
use prjcombine_xc9500::{self as xc9500, FbBitCoord, Tile, TileItem, TileItemBitVec, TileItemEnum};
use prjcombine_xilinx_cpld::{
    bits::{BitPos, EnumData, FbBits, InvBit, McBits},
    device::{Device, DeviceKind, JtagPin, PkgPin},
    types::{
        CeMuxVal, ClkMuxVal, ExportDir, FbId, FbMcId, ImuxInput, IoId, OeMode, OeMuxVal, RegMode,
        Slew, SrMuxVal, TermMode, Xc9500McPt,
    },
};
use prjcombine_xilinx_recpld::{
    db::Database,
    fuzzdb::{FuzzDb, FuzzDbPart},
    speeddb::SpeedDb,
};
use serde_json::json;
use unnamed_entity::{EntityId, EntityVec};
use xc9500::GlobalBitCoord;

#[derive(Parser)]
struct Args {
    db: PathBuf,
    fdb: PathBuf,
    sdb: PathBuf,
    out: PathBuf,
    json: PathBuf,
}

// fn print_tile<T>(prefix: &str, tile: &Tile<T>, print_coord: impl Fn(&T)) {
//     for (name, item) in &tile.items {
//         print!("{prefix}");
//         let bits = match item {
//             TileItem::Enum(it) => &it.bits,
//             TileItem::BitVec(it) => &it.bits,
//         };
//         for bit in bits {
//             print!(" ");
//             print_coord(bit);
//         }
//         print!(": {name}");
//         match item {
//             TileItem::Enum(it) => {
//                 println!(" ENUM");
//                 for (k, v) in &it.values {
//                     print!("    ");
//                     for bit in v {
//                         print!("{}", u8::from(*bit));
//                     }
//                     println!(": {k}");
//                 }
//             }
//             TileItem::BitVec(it) => {
//                 if it.invert {
//                     println!(" INVERT");
//                 } else {
//                     println!(" TRUE");
//                 }
//             }
//         }
//     }
// }

fn merge_enum<T: Copy + Eq + Ord + Debug>(
    a: &mut TileItemEnum<T>,
    b: &TileItemEnum<T>,
    neutral: bool,
) {
    if a == b {
        return;
    }
    let mut bits = a.bits.clone();
    for &bit in &b.bits {
        if !bits.contains(&bit) {
            bits.push(bit);
        }
    }
    bits.sort();
    let bit_map_a: Vec<_> = bits
        .iter()
        .map(|&x| a.bits.iter().find_position(|&&y| x == y).map(|x| x.0))
        .collect();
    let bit_map_b: Vec<_> = bits
        .iter()
        .map(|&x| b.bits.iter().find_position(|&&y| x == y).map(|x| x.0))
        .collect();
    a.bits = bits;
    for val in a.values.values_mut() {
        *val = bit_map_a
            .iter()
            .map(|&x| match x {
                Some(idx) => val[idx],
                None => neutral,
            })
            .collect();
    }
    for (key, val) in &b.values {
        let val: BitVec = bit_map_b
            .iter()
            .map(|&x| match x {
                Some(idx) => val[idx],
                None => neutral,
            })
            .collect();

        match a.values.entry(key.clone()) {
            btree_map::Entry::Vacant(e) => {
                e.insert(val);
            }
            btree_map::Entry::Occupied(e) => assert_eq!(*e.get(), val),
        }
    }
}

fn merge_tile<T: Debug + Copy + Eq + Ord>(a: &mut Tile<T>, b: &Tile<T>, neutral: bool) {
    if a == b {
        return;
    }
    for (k, v) in &b.items {
        match a.items.entry(k.clone()) {
            btree_map::Entry::Vacant(e) => {
                e.insert(v.clone());
            }
            btree_map::Entry::Occupied(mut e) => match (e.get_mut(), v) {
                (TileItem::Enum(ref mut e1), TileItem::Enum(e2)) => {
                    merge_enum(e1, e2, neutral);
                }
                (TileItem::BitVec(v1), TileItem::BitVec(v2)) => assert_eq!(v1, v2),
                _ => unreachable!(),
            },
        }
    }
}

fn map_bit_raw(device: &Device, bit: BitPos) -> GlobalBitCoord {
    let (addr, bit) = bit;
    if device.kind == DeviceKind::Xc9500 {
        let fb = addr >> 13 & 0xf;
        assert_eq!(addr & 0x1000, 0);
        let row = addr >> 5 & 0x7f;
        let col_a = addr >> 3 & 3;
        let col_b = addr & 7;
        let column = col_a * 5 + col_b;
        GlobalBitCoord {
            fb,
            row,
            column,
            bit: bit as u32,
        }
    } else {
        let fb = bit >> 3;
        let fb = fb as u32;
        let bit = bit & 7;
        let row = addr >> 5 & 0x7f;
        let col_a = addr >> 3 & 3;
        let col_b = addr & 7;
        let column = col_a * 5 + col_b;
        GlobalBitCoord {
            fb,
            row,
            column,
            bit: bit as u32,
        }
    }
}

fn map_fb_bit_raw(device: &Device, fb: FbId, bit: BitPos) -> FbBitCoord {
    let crd = map_bit_raw(device, bit);
    assert_eq!(crd.fb as usize, fb.to_idx());
    FbBitCoord {
        row: crd.row,
        column: crd.column,
        bit: crd.bit,
    }
}

fn map_bit(device: &Device, fpart: &FuzzDbPart, bit: usize) -> GlobalBitCoord {
    map_bit_raw(device, fpart.map.main[bit])
}

fn map_fb_bit(device: &Device, fpart: &FuzzDbPart, fb: FbId, bit: usize) -> FbBitCoord {
    map_fb_bit_raw(device, fb, fpart.map.main[bit])
}

fn map_mc_bit(device: &Device, fpart: &FuzzDbPart, fb: FbId, mc: FbMcId, bit: usize) -> u32 {
    let crd = map_fb_bit(device, fpart, fb, bit);
    assert_eq!(crd.column as usize, mc.to_idx() % 9);
    assert_eq!(crd.bit as usize, 6 + mc.to_idx() / 9);
    crd.row
}

fn extract_mc_enum<T: Clone + Debug + Eq + core::hash::Hash>(
    device: &Device,
    fpart: &FuzzDbPart,
    get_enum: impl Fn(&McBits) -> Option<&EnumData<T>>,
    xlat_val: impl Fn(&T) -> String,
    default: impl Into<String>,
) -> TileItem<u32> {
    let default = default.into();
    let mut res = None;
    for (fb, mc) in device.mcs() {
        if let Some(enum_) = get_enum(&fpart.bits.fbs[fb].mcs[mc]) {
            let bits = enum_
                .bits
                .iter()
                .map(|&bit| map_mc_bit(device, fpart, fb, mc, bit))
                .collect();
            let mut values: BTreeMap<_, _> = enum_
                .items
                .iter()
                .map(|(k, v)| (xlat_val(k), v.clone()))
                .collect();
            match values.entry(default.clone()) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(enum_.default.clone());
                }
                btree_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), enum_.default);
                }
            }
            let data = TileItemEnum { bits, values };
            if res.is_none() {
                res = Some(data);
            } else {
                assert_eq!(res, Some(data));
            }
        }
    }
    TileItem::Enum(res.unwrap())
}

fn extract_mc_bool(
    device: &Device,
    fpart: &FuzzDbPart,
    get_bit: impl Fn(&McBits) -> Option<InvBit>,
) -> TileItem<u32> {
    let mut res = None;
    for (fb, mc) in device.mcs() {
        if let Some((bit, pol)) = get_bit(&fpart.bits.fbs[fb].mcs[mc]) {
            let bits = vec![map_mc_bit(device, fpart, fb, mc, bit)];
            let data = TileItemBitVec { bits, invert: !pol };
            if res.is_none() {
                res = Some(data);
            } else {
                assert_eq!(res, Some(data));
            }
        }
    }
    TileItem::BitVec(res.unwrap())
}

fn extract_mc_bool_to_enum(
    device: &Device,
    fpart: &FuzzDbPart,
    get_bit: impl Fn(&McBits) -> Option<InvBit>,
    val_true: impl Into<String>,
    val_false: impl Into<String>,
) -> TileItem<u32> {
    let val_true = val_true.into();
    let val_false = val_false.into();
    let mut res = None;
    for (fb, mc) in device.mcs() {
        if let Some((bit, pol)) = get_bit(&fpart.bits.fbs[fb].mcs[mc]) {
            let bits = vec![map_mc_bit(device, fpart, fb, mc, bit)];
            let data = TileItemEnum {
                bits,
                values: [
                    (val_true.clone(), BitVec::repeat(pol, 1)),
                    (val_false.clone(), BitVec::repeat(!pol, 1)),
                ]
                .into_iter()
                .collect(),
            };
            if res.is_none() {
                res = Some(data);
            } else {
                assert_eq!(res, Some(data));
            }
        }
    }
    TileItem::Enum(res.unwrap())
}

fn extract_mc_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<u32> {
    let mut res = Tile {
        items: BTreeMap::new(),
    };
    for (i, pt) in [
        Xc9500McPt::Clk,
        Xc9500McPt::Oe,
        Xc9500McPt::Rst,
        Xc9500McPt::Set,
        Xc9500McPt::Xor,
    ]
    .into_iter()
    .enumerate()
    {
        res.items.insert(
            format!("PT.{i}.ALLOC"),
            extract_mc_enum(
                device,
                fpart,
                |mcbits| Some(&mcbits.pt.as_ref().unwrap()[pt].alloc),
                |alloc| match alloc {
                    prjcombine_xilinx_cpld::bits::PtAlloc::OrMain => "SUM".to_string(),
                    prjcombine_xilinx_cpld::bits::PtAlloc::OrExport => "EXPORT".to_string(),
                    prjcombine_xilinx_cpld::bits::PtAlloc::Special => "SPECIAL".to_string(),
                },
                "NONE",
            ),
        );
        res.items.insert(
            format!("PT.{i}.HP"),
            extract_mc_bool(device, fpart, |mcbits| {
                Some(mcbits.pt.as_ref().unwrap()[pt].hp)
            }),
        );
    }
    for (dir, name) in [
        (ExportDir::Up, "IMPORT_UP_ALLOC"),
        (ExportDir::Down, "IMPORT_DOWN_ALLOC"),
    ] {
        res.items.insert(
            name.to_string(),
            extract_mc_bool_to_enum(
                device,
                fpart,
                |mcbits| Some(mcbits.import.as_ref().unwrap()[dir]),
                "SUM",
                "EXPORT",
            ),
        );
    }
    res.items.insert(
        "EXPORT_DIR".to_string(),
        extract_mc_enum(
            device,
            fpart,
            |mcbits| mcbits.exp_dir.as_ref(),
            |val| {
                match val {
                    ExportDir::Up => "UP",
                    ExportDir::Down => "DOWN",
                }
                .into()
            },
            "UP",
        ),
    );
    res.items.insert(
        "SUM_HP".to_string(),
        extract_mc_bool(device, fpart, |mcbits| mcbits.hp),
    );
    res.items.insert(
        "INV".to_string(),
        extract_mc_bool(device, fpart, |mcbits| mcbits.inv),
    );
    res.items.insert(
        "OE_MUX".to_string(),
        extract_mc_enum(
            device,
            fpart,
            |mcbits| mcbits.oe_mux.as_ref(),
            |val| match val {
                OeMuxVal::Pt => "PT".to_string(),
                OeMuxVal::Foe(idx) => format!("FOE{}", idx.to_idx()),
                _ => unreachable!(),
            },
            "PT",
        ),
    );
    res.items.insert(
        "OUT_MUX".to_string(),
        extract_mc_bool_to_enum(device, fpart, |mcbits| mcbits.ff_en, "FF", "COMB"),
    );
    res.items.insert(
        "CLK_MUX".to_string(),
        extract_mc_enum(
            device,
            fpart,
            |mcbits| Some(&mcbits.clk_mux),
            |val| match val {
                ClkMuxVal::Pt => "PT".to_string(),
                ClkMuxVal::Fclk(idx) => format!("FCLK{}", idx.to_idx()),
                _ => unreachable!(),
            },
            "FCLK1",
        ),
    );
    if device.kind != DeviceKind::Xc9500 {
        res.items.insert(
            "CLK_INV".to_string(),
            extract_mc_bool(device, fpart, |mcbits| mcbits.clk_inv),
        );
        res.items.insert(
            "OE_INV".to_string(),
            extract_mc_bool(device, fpart, |mcbits| mcbits.oe_inv),
        );
        res.items.insert(
            "CE_MUX".to_string(),
            extract_mc_enum(
                device,
                fpart,
                |mcbits| mcbits.ce_mux.as_ref(),
                |val| {
                    match val {
                        CeMuxVal::PtRst => "PT2",
                        CeMuxVal::PtSet => "PT3",
                        _ => unreachable!(),
                    }
                    .into()
                },
                "NONE",
            ),
        );
    }
    res.items.insert(
        "REG_MODE".to_string(),
        extract_mc_enum(
            device,
            fpart,
            |mcbits| Some(&mcbits.reg_mode),
            |val| {
                match val {
                    RegMode::Dff => "DFF",
                    RegMode::Tff => "TFF",
                    _ => unreachable!(),
                }
                .into()
            },
            "DFF",
        ),
    );
    res.items.insert(
        "RST_MUX".to_string(),
        extract_mc_enum(
            device,
            fpart,
            |mcbits| Some(&mcbits.rst_mux),
            |val| {
                match val {
                    SrMuxVal::Pt => "PT",
                    SrMuxVal::Fsr => "FSR",
                    _ => unreachable!(),
                }
                .into()
            },
            "PT",
        ),
    );
    res.items.insert(
        "SET_MUX".to_string(),
        extract_mc_enum(
            device,
            fpart,
            |mcbits| Some(&mcbits.set_mux),
            |val| {
                match val {
                    SrMuxVal::Pt => "PT",
                    SrMuxVal::Fsr => "FSR",
                    _ => unreachable!(),
                }
                .into()
            },
            "PT",
        ),
    );
    res.items.insert(
        "REG_INIT".to_string(),
        extract_mc_bool(device, fpart, |mcbits| mcbits.init),
    );
    if device.kind == DeviceKind::Xc9500 {
        res.items.insert(
            "IOB_OE_MUX".to_string(),
            extract_mc_enum(
                device,
                fpart,
                |mcbits| mcbits.obuf_oe_mode.as_ref(),
                |val| {
                    match val {
                        OeMode::Gnd => "GND",
                        OeMode::Vcc => "VCC",
                        OeMode::McOe => "OE_MUX",
                    }
                    .into()
                },
                "GND",
            ),
        );
        res.items.insert(
            "UIM_OE_MUX".to_string(),
            extract_mc_enum(
                device,
                fpart,
                |mcbits| mcbits.uim_oe_mode.as_ref(),
                |val| {
                    match val {
                        OeMode::Gnd => "GND",
                        OeMode::Vcc => "VCC",
                        OeMode::McOe => "OE_MUX",
                    }
                    .into()
                },
                "GND",
            ),
        );
        res.items.insert(
            "UIM_OUT_INV".to_string(),
            extract_mc_bool(device, fpart, |mcbits| mcbits.uim_out_inv),
        );
    }
    res.items.insert(
        "IOB_GND".to_string(),
        extract_mc_bool(device, fpart, |mcbits| mcbits.is_gnd),
    );

    res.items.insert(
        "IOB_SLEW".to_string(),
        extract_mc_enum(
            device,
            fpart,
            |mcbits| mcbits.slew.as_ref(),
            |val| {
                match val {
                    Slew::Slow => "SLOW",
                    Slew::Fast => "FAST",
                }
                .into()
            },
            "SLOW",
        ),
    );

    res
}

fn extract_fb_pullup_disable(device: &Device, fpart: &FuzzDbPart) -> TileItem<FbBitCoord> {
    let mut blank_expected: BitVec =
        BitVec::repeat(device.kind == DeviceKind::Xc9500, fpart.blank.len());
    for (bit, pol) in fpart.bits.usercode.unwrap() {
        blank_expected.set(bit, pol);
    }
    if device.kind != DeviceKind::Xc9500 {
        let data = fpart.bits.term_mode.as_ref().unwrap();
        for (bit, val) in data.bits.iter().copied().zip(data.default.iter()) {
            blank_expected.set(bit, *val);
        }
    }
    for (fb, mc) in device.mcs() {
        if device.kind == DeviceKind::Xc9500 {
            let (bit, pol) = fpart.bits.fbs[fb].mcs[mc].ff_en.unwrap();
            blank_expected.set(bit, !pol);
            let data = fpart.bits.fbs[fb].mcs[mc].uim_oe_mode.as_ref().unwrap();
            for (bit, val) in data
                .bits
                .iter()
                .copied()
                .zip(data.items[&OeMode::Gnd].iter())
            {
                blank_expected.set(bit, *val);
            }
        }
    }
    let mut pullup_disable_bits = vec![];
    for (i, val) in fpart.blank.iter().enumerate() {
        if val != blank_expected[i] {
            assert_eq!(!val, device.kind == DeviceKind::Xc9500);
            pullup_disable_bits.push(i);
        }
    }
    assert_eq!(pullup_disable_bits.len(), device.fbs);
    let mut res = None;
    for (fb, bit) in device.fbs().zip(pullup_disable_bits.iter().copied()) {
        let crd = map_fb_bit(device, fpart, fb, bit);
        if res.is_none() {
            res = Some(crd);
        } else {
            assert_eq!(res, Some(crd));
        }
    }
    TileItem::BitVec(TileItemBitVec {
        bits: vec![res.unwrap()],
        invert: device.kind == DeviceKind::Xc9500,
    })
}

fn extract_fb_bool(
    device: &Device,
    fpart: &FuzzDbPart,
    get_bit: impl Fn(&FbBits) -> Option<InvBit>,
) -> TileItem<FbBitCoord> {
    let mut res = None;
    for fb in device.fbs() {
        if let Some((bit, pol)) = get_bit(&fpart.bits.fbs[fb]) {
            let bits = vec![map_fb_bit(device, fpart, fb, bit)];
            let data = TileItemBitVec { bits, invert: !pol };
            if res.is_none() {
                res = Some(data);
            } else {
                assert_eq!(res, Some(data));
            }
        }
    }
    TileItem::BitVec(res.unwrap())
}

fn extract_fb_enum<T: Clone + Debug + Eq + core::hash::Hash>(
    device: &Device,
    fpart: &FuzzDbPart,
    get_enum: impl Fn(&FbBits) -> Option<&EnumData<T>>,
    xlat_val: impl Fn(&T) -> String,
    default: impl Into<String>,
) -> TileItem<FbBitCoord> {
    let default = default.into();
    let mut res = None;
    for fb in device.fbs() {
        if let Some(enum_) = get_enum(&fpart.bits.fbs[fb]) {
            let bits = enum_
                .bits
                .iter()
                .map(|&bit| map_fb_bit(device, fpart, fb, bit))
                .collect();
            let mut values: BTreeMap<_, _> = enum_
                .items
                .iter()
                .map(|(k, v)| (xlat_val(k), v.clone()))
                .collect();
            match values.entry(default.clone()) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(enum_.default.clone());
                }
                btree_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), enum_.default);
                }
            }
            let data = TileItemEnum { bits, values };
            if res.is_none() {
                res = Some(data);
            } else {
                assert_eq!(res, Some(data));
            }
        }
    }
    TileItem::Enum(res.unwrap())
}

fn extract_fb_prot(device: &Device, bits: &[BitPos]) -> TileItem<FbBitCoord> {
    assert_eq!(device.fbs, bits.len());
    let mut res = None;
    for (fb, &bit) in device.fbs().zip(bits.iter()) {
        let crd = map_fb_bit_raw(device, fb, bit);
        if res.is_none() {
            res = Some(crd);
        } else {
            assert_eq!(res, Some(crd));
        }
    }
    TileItem::BitVec(TileItemBitVec {
        bits: vec![res.unwrap()],
        invert: device.kind == DeviceKind::Xc9500,
    })
}

fn extract_fb_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<FbBitCoord> {
    let mut res = Tile {
        items: BTreeMap::new(),
    };

    res.items.insert(
        "PULLUP_DISABLE".to_string(),
        extract_fb_pullup_disable(device, fpart),
    );

    res.items.insert(
        "ENABLE".to_string(),
        extract_fb_bool(device, fpart, |fbbits| fbbits.en),
    );
    res.items.insert(
        "EXPORT_ENABLE".to_string(),
        extract_fb_bool(device, fpart, |fbbits| fbbits.exp_en),
    );

    if device.kind == DeviceKind::Xc9500 {
        let a: Vec<_> = fpart.map.rprot.chunks(2).map(|x| x[0]).collect();
        let mut b: Vec<_> = fpart.map.rprot.chunks(2).map(|x| x[1]).collect();
        // special bug workaround!
        if device.fbs == 8 {
            assert_eq!(b[1].0, 0x3043);
            b[1].0 = 0x2883;
        }
        res.items
            .insert("READ_PROT_A".to_string(), extract_fb_prot(device, &a));
        res.items
            .insert("READ_PROT_B".to_string(), extract_fb_prot(device, &b));
    } else {
        res.items.insert(
            "READ_PROT".to_string(),
            extract_fb_prot(device, &fpart.map.rprot),
        );
    }
    res.items.insert(
        "WRITE_PROT".to_string(),
        extract_fb_prot(device, &fpart.map.wprot),
    );

    res
}

fn extract_global_bool(
    device: &Device,
    fpart: &FuzzDbPart,
    bit: InvBit,
) -> TileItem<GlobalBitCoord> {
    let (bit, pol) = bit;
    let bits = vec![map_bit(device, fpart, bit)];
    TileItem::BitVec(TileItemBitVec { bits, invert: !pol })
}

fn extract_global_enum<T: Clone + Debug + Eq + core::hash::Hash>(
    device: &Device,
    fpart: &FuzzDbPart,
    enum_: &EnumData<T>,
    xlat_val: impl Fn(&T) -> String,
    default: impl Into<String>,
) -> TileItem<GlobalBitCoord> {
    let default = default.into();
    let bits = enum_
        .bits
        .iter()
        .map(|&bit| map_bit(device, fpart, bit))
        .collect();
    let mut values: BTreeMap<_, _> = enum_
        .items
        .iter()
        .map(|(k, v)| (xlat_val(k), v.clone()))
        .collect();
    match values.entry(default.clone()) {
        btree_map::Entry::Vacant(e) => {
            e.insert(enum_.default.clone());
        }
        btree_map::Entry::Occupied(e) => {
            assert_eq!(*e.get(), enum_.default);
        }
    }
    TileItem::Enum(TileItemEnum { bits, values })
}

fn extract_global_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<GlobalBitCoord> {
    let mut res = Tile {
        items: BTreeMap::new(),
    };

    res.items.insert(
        "FSR_INV".to_string(),
        extract_global_bool(device, fpart, fpart.bits.fsr_inv.unwrap()),
    );
    if device.kind == DeviceKind::Xc9500 {
        for (i, &bit) in &fpart.bits.fclk_inv {
            res.items.insert(
                format!("FCLK{i}_INV"),
                extract_global_bool(device, fpart, bit),
            );
        }
        for (i, &bit) in &fpart.bits.foe_inv {
            res.items.insert(
                format!("FOE{i}_INV"),
                extract_global_bool(device, fpart, bit),
            );
        }
        for (i, enum_) in &fpart.bits.fclk_mux {
            res.items.insert(
                format!("FCLK{i}_MUX"),
                extract_global_enum(device, fpart, enum_, |val| format!("GCLK{val}"), "NONE"),
            );
        }
        for (i, enum_) in &fpart.bits.foe_mux {
            let kind = match fpart.bits.foe_mux.len() {
                2 => "SMALL",
                4 => "LARGE",
                _ => unreachable!(),
            };
            res.items.insert(
                format!("FOE{i}_MUX.{kind}"),
                extract_global_enum(device, fpart, enum_, |val| format!("GOE{val}"), "NONE"),
            );
        }
    } else {
        for (i, &bit) in &fpart.bits.fclk_en {
            res.items.insert(
                format!("FCLK{i}_ENABLE"),
                extract_global_bool(device, fpart, bit),
            );
        }
        for (i, &bit) in &fpart.bits.foe_en {
            res.items.insert(
                format!("FOE{i}_ENABLE"),
                extract_global_bool(device, fpart, bit),
            );
        }

        res.items.insert(
            "TERM_MODE".to_string(),
            extract_global_enum(
                device,
                fpart,
                fpart.bits.term_mode.as_ref().unwrap(),
                |val| match val {
                    TermMode::Pullup => unreachable!(),
                    TermMode::Keeper => "KEEPER".to_string(),
                },
                "FLOAT",
            ),
        );
    }
    let usercode = fpart.bits.usercode.unwrap();
    for (_, pol) in usercode {
        assert_eq!(pol, usercode[0].1);
    }
    res.items.insert(
        "USERCODE".to_string(),
        TileItem::BitVec(TileItemBitVec {
            bits: usercode
                .into_iter()
                .map(|(bit, _)| map_bit(device, fpart, bit))
                .collect(),
            invert: !usercode[1].1,
        }),
    );
    if device.kind == DeviceKind::Xc9500Xv {
        res.items.insert(
            "DONE".to_string(),
            TileItem::BitVec(TileItemBitVec {
                bits: vec![map_bit_raw(device, fpart.map.done.unwrap())],
                invert: false,
            }),
        );
    }
    res
}

fn extract_imux_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<FbBitCoord> {
    let mut tile = Tile {
        items: BTreeMap::new(),
    };
    for im in device.fb_imuxes() {
        let item = extract_fb_enum(
            device,
            fpart,
            |fbbits| Some(&fbbits.imux[im]),
            |val| match val {
                ImuxInput::Fbk(mc) => format!("FBK.{mc}"),
                ImuxInput::Mc((fb, mc)) => format!("MC.{fb}.{mc}"),
                ImuxInput::Ibuf(IoId::Mc((fb, mc))) => format!("IOB.{fb}.{mc}"),
                ImuxInput::Uim => "UIM".to_string(),
                _ => unreachable!(),
            },
            "NONE",
        );
        tile.items.insert(format!("IMUX.{im}"), item);
    }
    tile
}

fn extract_ibuf_uim_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<GlobalBitCoord> {
    let mut res = Tile {
        items: BTreeMap::new(),
    };
    for (fb, mc) in device.mcs() {
        let bits = &fpart.bits.fbs[fb].mcs[mc].ibuf_uim_en;
        if bits.is_empty() {
            continue;
        }
        assert_eq!(bits.len(), 2);
        for (i, bit) in bits.iter().copied().enumerate() {
            res.items.insert(
                format!("FB.{fb}.MC.{mc}.IBUF_UIM_ENABLE.{i}"),
                extract_global_bool(device, fpart, bit),
            );
        }
    }
    res
}

fn validate_jed_map_xc9500(device: &Device, fpart: &FuzzDbPart) {
    let main_row_bits = 8 * 9 + 6 * 6;
    let uim_row_bits = 8 + 7 * 4;
    let main_area_bits = main_row_bits * 72;
    let uim_subarea_bits = uim_row_bits * 18;
    let uim_area_bits = uim_subarea_bits * device.fbs;
    let fb_bits = main_area_bits + uim_area_bits;
    let total_bits = fb_bits * device.fbs;
    assert_eq!(fpart.map.main.len(), total_bits);
    for (mut fuse, &(r_addr, r_bit)) in fpart.map.main.iter().enumerate() {
        let r_addr = r_addr as usize;
        let addr;
        let bit;
        let fb = fuse / fb_bits;
        fuse %= fb_bits;
        if fuse < main_area_bits {
            let row = fuse / main_row_bits;
            fuse %= main_row_bits;
            let column;
            if fuse < 8 * 9 {
                column = fuse / 8;
                bit = fuse % 8;
            } else {
                fuse -= 8 * 9;
                column = 9 + fuse / 6;
                bit = fuse % 6;
            }
            addr = fb << 13 | row << 5 | (column / 5) << 3 | (column % 5);
        } else {
            fuse -= main_area_bits;
            let subarea = fuse / uim_subarea_bits;
            fuse %= uim_subarea_bits;
            let row = fuse / uim_row_bits;
            fuse %= uim_row_bits;
            let column;
            if fuse < 8 {
                column = 0;
                bit = fuse;
            } else {
                fuse -= 8;
                column = 1 + fuse / 7;
                bit = fuse % 7;
            }
            addr = fb << 13 | 1 << 12 | subarea << 8 | row << 3 | column;
        }
        assert_eq!((addr, bit), (r_addr, r_bit));
    }
}

fn validate_jed_map_xc9500xl(device: &Device, fpart: &FuzzDbPart) {
    let row_bits = (8 * 9 + 6 * 6) * device.fbs;
    let total_bits = row_bits * 108;
    assert_eq!(fpart.map.main.len(), total_bits);
    for (mut fuse, &(r_addr, r_bit)) in fpart.map.main.iter().enumerate() {
        let r_addr = r_addr as usize;
        let bit;
        let row = fuse / row_bits;
        let fb;
        fuse %= row_bits;
        let column;
        if fuse < 8 * 9 * device.fbs {
            column = fuse / (8 * device.fbs);
            fuse %= 8 * device.fbs;
            fb = fuse / 8;
            bit = fuse % 8;
        } else {
            fuse -= 8 * 9 * device.fbs;
            column = 9 + fuse / (6 * device.fbs);
            fuse %= 6 * device.fbs;
            fb = fuse / 6;
            bit = fuse % 6;
        }
        let addr = row << 5 | (column / 5) << 3 | (column % 5);
        assert_eq!((addr, fb * 8 + bit), (r_addr, r_bit));
    }
}

fn validate_imux_uim(device: &Device, fpart: &FuzzDbPart) {
    for fb in device.fbs() {
        for im in device.fb_imuxes() {
            for sfb in device.fbs() {
                for smc in device.fb_mcs() {
                    let (fuse, inv) = fpart.bits.fbs[fb].uim_mc[im][sfb][smc];
                    assert!(inv);
                    let (r_addr, r_bit) = fpart.map.main[fuse];
                    let r_addr = r_addr as usize;
                    let bit = im.to_idx() / 5;
                    let column = im.to_idx() % 5;
                    let row = smc.to_idx();
                    let subarea = sfb.to_idx();
                    let addr = fb.to_idx() << 13 | 1 << 12 | subarea << 8 | row << 3 | column;
                    assert_eq!((addr, bit), (r_addr, r_bit));
                }
            }
        }
    }
}

fn validate_pterm(device: &Device, fpart: &FuzzDbPart) {
    for fb in device.fbs() {
        for mc in device.fb_mcs() {
            for (pti, pt) in [
                Xc9500McPt::Clk,
                Xc9500McPt::Oe,
                Xc9500McPt::Rst,
                Xc9500McPt::Set,
                Xc9500McPt::Xor,
            ]
            .into_iter()
            .enumerate()
            {
                for im in device.fb_imuxes() {
                    let ((fuse_t, inv_t), (fuse_f, inv_f)) =
                        fpart.bits.fbs[fb].mcs[mc].pt.as_ref().unwrap()[pt].and[im];
                    assert!(inv_t);
                    assert!(inv_f);
                    for (neg, fuse) in [(0, fuse_t), (1, fuse_f)] {
                        let (r_addr, r_bit) = fpart.map.main[fuse];
                        let r_addr = r_addr as usize;
                        let row = im.to_idx() * 2 + (1 - neg);
                        let column = pti + (mc.to_idx() % 3) * 5;
                        let bit = mc.to_idx() / 3;
                        if device.kind == DeviceKind::Xc9500 {
                            let addr =
                                fb.to_idx() << 13 | row << 5 | (column / 5) << 3 | (column % 5);
                            assert_eq!((addr, bit), (r_addr, r_bit));
                        } else {
                            let addr = row << 5 | (column / 5) << 3 | (column % 5);
                            let bit = fb.to_idx() * 8 + bit;
                            assert_eq!((addr, bit), (r_addr, r_bit));
                        }
                    }
                }
            }
        }
    }
}

fn tile_to_json<T: Copy>(
    tile: &Tile<T>,
    bit_to_json: impl Fn(T) -> serde_json::Value,
) -> serde_json::Value {
    serde_json::Map::from_iter(tile.items.iter().map(|(k, v)| {
        (
            k.clone(),
            match v {
                TileItem::Enum(it) => json!({
                    "bits": Vec::from_iter(it.bits.iter().copied().map(&bit_to_json)),
                    "values": serde_json::Map::from_iter(
                        it.values.iter().map(|(vk, vv)| {
                            (vk.clone(), Vec::from_iter(vv.iter().map(|x| *x)).into())
                        })
                    ),
                }),
                TileItem::BitVec(it) => json!({
                    "bits": Vec::from_iter(it.bits.iter().copied().map(&bit_to_json)),
                    "invert": it.invert,
                }),
            },
        )
    }))
    .into()
}

fn fb_bit_to_json(crd: FbBitCoord) -> serde_json::Value {
    json!([crd.row, crd.column, crd.bit])
}

fn global_bit_to_json(crd: GlobalBitCoord) -> serde_json::Value {
    json!([crd.fb, crd.row, crd.column, crd.bit])
}

fn convert_io(io: IoId) -> (xc9500::FbId, xc9500::FbMcId) {
    let IoId::Mc((fb, mc)) = io else {
        unreachable!();
    };
    (
        xc9500::FbId::from_idx(fb.to_idx()),
        xc9500::FbMcId::from_idx(mc.to_idx()),
    )
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = Database::from_file(args.db)?;
    let fdb = FuzzDb::from_file(args.fdb)?;
    let sdb = SpeedDb::from_file(args.sdb)?;

    let mut mc_bits = None;
    let mut fb_bits = None;
    let mut global_bits = None;
    let mut imux_bits = BTreeMap::new();
    let mut ibuf_uim_bits = None;

    for fpart in &fdb.parts {
        let part = db
            .parts
            .iter()
            .find(|p| p.dev_name == fpart.dev_name && p.pkg_name == fpart.pkg_name)
            .unwrap();
        let device = &db.devices[part.device].device;
        if device.kind == DeviceKind::Xc9500 {
            validate_jed_map_xc9500(device, fpart);
            validate_imux_uim(device, fpart);
        } else {
            validate_jed_map_xc9500xl(device, fpart);
        }
        validate_pterm(device, fpart);

        if let Some(ref mut bits) = mc_bits {
            merge_tile(
                bits,
                &extract_mc_bits(device, fpart),
                device.kind == DeviceKind::Xc9500,
            );
        } else {
            mc_bits = Some(extract_mc_bits(device, fpart));
        }
        if let Some(ref mut bits) = fb_bits {
            merge_tile(
                bits,
                &extract_fb_bits(device, fpart),
                device.kind == DeviceKind::Xc9500,
            );
        } else {
            fb_bits = Some(extract_fb_bits(device, fpart));
        }
        if let Some(ref mut bits) = global_bits {
            merge_tile(
                bits,
                &extract_global_bits(device, fpart),
                device.kind == DeviceKind::Xc9500,
            );
        } else {
            global_bits = Some(extract_global_bits(device, fpart));
        }
        let cur_imux_bits = extract_imux_bits(device, fpart);
        match imux_bits.entry(device.fbs) {
            btree_map::Entry::Vacant(e) => {
                e.insert(cur_imux_bits);
            }
            btree_map::Entry::Occupied(mut e) => merge_tile(
                e.get_mut(),
                &cur_imux_bits,
                device.kind == DeviceKind::Xc9500,
            ),
        }

        if device.kind == DeviceKind::Xc9500 && device.fbs == 16 {
            let cur_bits = extract_ibuf_uim_bits(device, fpart);
            if let Some(ref mut bits) = ibuf_uim_bits {
                merge_tile(bits, &cur_bits, true);
            } else {
                ibuf_uim_bits = Some(cur_bits);
            }
        } else {
            for (fb, mc) in device.mcs() {
                assert!(fpart.bits.fbs[fb].mcs[mc].ibuf_uim_en.is_empty());
            }
        }
    }
    let mc_bits = mc_bits.unwrap();
    let fb_bits = fb_bits.unwrap();
    let global_bits = global_bits.unwrap();
    // print_tile("MC", &mc_bits, |x| print!("{x}"));
    // print_tile("FB", &fb_bits, |x| {
    //     print!("{}.{}.{}", x.row, x.column, x.bit)
    // });
    // print_tile("GLOBAL", &global_bits, |x| {
    //     print!("{}.{}.{}.{}", x.fb, x.row, x.column, x.bit)
    // });
    // for (k, v) in &imux_bits {
    //     print_tile(&format!("IMUX {k}"), v, |x| {
    //         print!("{}.{}.{}", x.row, x.column, x.bit)
    //     })
    // }
    // if let Some(ref bits) = ibuf_uim_bits {
    //     print_tile("IBUF_UIM.16", bits, |x| {
    //         print!("{}.{}.{}.{}", x.fb, x.row, x.column, x.bit)
    //     });
    // }

    let devices: EntityVec<_, _> = db
        .devices
        .values()
        .map(|dev| {
            let device = &dev.device;
            let idcode = match device.kind {
                DeviceKind::Xc9500 => 0x9500093,
                DeviceKind::Xc9500Xl => 0x9600093,
                DeviceKind::Xc9500Xv => 0x9700093,
                _ => unreachable!(),
            } | (device.fbs as u32) << 12;
            let mut io_special = BTreeMap::new();
            io_special.insert("GSR".to_string(), convert_io(device.sr_pad.unwrap()));
            for (i, &io) in &device.clk_pads {
                io_special.insert(format!("GCLK{i}"), convert_io(io));
            }
            for (i, &io) in &device.oe_pads {
                io_special.insert(format!("GOE{i}"), convert_io(io));
            }
            xc9500::Device {
                kind: match device.kind {
                    DeviceKind::Xc9500 => xc9500::DeviceKind::Xc9500,
                    DeviceKind::Xc9500Xl => xc9500::DeviceKind::Xc9500Xl,
                    DeviceKind::Xc9500Xv => xc9500::DeviceKind::Xc9500Xv,
                    _ => unreachable!(),
                },
                idcode,
                fbs: device.fbs,
                io: device
                    .io
                    .iter()
                    .map(|(k, v)| (convert_io(*k), xc9500::BankId::from_idx(v.bank.to_idx())))
                    .collect(),
                banks: device.banks,
                tdo_bank: xc9500::BankId::from_idx(device.banks - 1),
                io_special,
                imux_bits: imux_bits[&device.fbs].clone(),
                uim_ibuf_bits: if device.kind == DeviceKind::Xc9500 && device.fbs == 16 {
                    Some(ibuf_uim_bits.clone().unwrap())
                } else {
                    None
                },
            }
        })
        .collect();

    let mut bonds = EntityVec::new();
    let mut speeds = EntityVec::new();
    let mut parts: Vec<xc9500::Part> = vec![];
    'parts: for spart in &db.parts {
        let package = &db.packages[spart.package];
        let device = xc9500::DeviceId::from_idx(spart.device.to_idx());
        let mut io_special_override = BTreeMap::new();
        for (func, &pad) in &devices[device].io_special {
            for (&from, &to) in &package.spec_remap {
                if convert_io(from) == pad {
                    io_special_override.insert(func.clone(), convert_io(to));
                }
            }
        }
        let bond = xc9500::Bond {
            io_special_override,
            pins: package
                .pins
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        match v {
                            PkgPin::Nc => xc9500::Pad::Nc,
                            PkgPin::Gnd => xc9500::Pad::Gnd,
                            PkgPin::VccInt => xc9500::Pad::VccInt,
                            PkgPin::VccIo(bank) => {
                                xc9500::Pad::VccIo(xc9500::BankId::from_idx(bank.to_idx()))
                            }
                            PkgPin::Jtag(JtagPin::Tck) => xc9500::Pad::Tck,
                            PkgPin::Jtag(JtagPin::Tms) => xc9500::Pad::Tms,
                            PkgPin::Jtag(JtagPin::Tdi) => xc9500::Pad::Tdi,
                            PkgPin::Jtag(JtagPin::Tdo) => xc9500::Pad::Tdo,
                            PkgPin::Io(io) => {
                                let (fb, mc) = convert_io(*io);
                                xc9500::Pad::Iob(fb, mc)
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
                assert_eq!(dpart.device, device);
                dpart.packages.insert(spart.pkg_name.clone(), bond);
                continue 'parts;
            }
        }
        parts.push(xc9500::Part {
            name: spart.dev_name.clone(),
            device,
            packages: [(spart.pkg_name.clone(), bond)].into_iter().collect(),
            speeds: spart
                .speeds
                .iter()
                .map(|sn| {
                    let speed = xc9500::Speed {
                        timing: sdb
                            .parts
                            .iter()
                            .find(|x| x.dev_name == spart.dev_name && &x.speed_name == sn)
                            .unwrap()
                            .timing
                            .clone(),
                    };
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

    let database = xc9500::Database {
        devices,
        bonds,
        speeds,
        parts,
        mc_bits,
        fb_bits,
        global_bits,
    };
    database.to_file(args.out)?;

    let json = json! ({
        "devices": Vec::from_iter(database.devices.values().map(|device| json! ({
            "kind": match device.kind {
                xc9500::DeviceKind::Xc9500 => "xc9500",
                xc9500::DeviceKind::Xc9500Xl => "xc9500xl",
                xc9500::DeviceKind::Xc9500Xv => "xc9500xv",
            },
            "idcode": device.idcode,
            "fbs": device.fbs,
            "ios": serde_json::Map::from_iter(
                device.io.iter().map(|(&(fb, mc), bank)| (format!("{fb}.{mc}"), json!(bank)))
            ),
            "banks": device.banks,
            "tdo_bank": device.tdo_bank,
            "io_special": device.io_special,
            "imux_bits": tile_to_json(&device.imux_bits, fb_bit_to_json),
            "uim_ibuf_bits": if let Some(ref bits) = device.uim_ibuf_bits {
                tile_to_json(bits, global_bit_to_json)
            } else {
                serde_json::Value::Null
            },
        }))),
        "bonds": Vec::from_iter(
            database.bonds.values().map(|bond| json!({
                "io_special_override": &bond.io_special_override,
                "pins": serde_json::Map::from_iter(
                    bond.pins.iter().map(|(k, v)| {
                        (k.clone(), match v {
                            xc9500::Pad::Nc => "NC".to_string(),
                            xc9500::Pad::Gnd => "GND".to_string(),
                            xc9500::Pad::VccInt => "VCCINT".to_string(),
                            xc9500::Pad::VccIo(bank) => format!("VCCIO{bank}"),
                            xc9500::Pad::Iob(fb, mc) => format!("MC_{fb}_{mc}"),
                            xc9500::Pad::Tms => "TMS".to_string(),
                            xc9500::Pad::Tck => "TCK".to_string(),
                            xc9500::Pad::Tdi => "TDI".to_string(),
                            xc9500::Pad::Tdo => "TDO".to_string(),
                        }.into())
                    })
                ),
            }))
        ),
        "speeds": &database.speeds,
        "parts": &database.parts,
        "mc_bits": tile_to_json(&database.mc_bits, |bit| bit.into()),
        "fb_bits": tile_to_json(&database.fb_bits, fb_bit_to_json),
        "global_bits": tile_to_json(&database.global_bits, global_bit_to_json),
    });
    std::fs::write(args.json, json.to_string())?;

    Ok(())
}
