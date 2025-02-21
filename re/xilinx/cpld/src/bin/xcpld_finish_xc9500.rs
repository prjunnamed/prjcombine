use std::{
    collections::{BTreeMap, btree_map},
    error::Error,
    path::PathBuf,
};

use bitvec::vec::BitVec;
use clap::Parser;
use prjcombine_re_xilinx_cpld::{
    bits::{BitPos, extract_bitvec, extract_bool, extract_bool_to_enum, extract_enum},
    device::{Device, DeviceKind, JtagPin, PkgPin},
    types::{
        CeMuxVal, ClkMuxVal, ExportDir, ImuxInput, OeMode, OeMuxVal, RegMode, Slew, SrMuxVal,
        TermMode, Xc9500McPt,
    },
};
use prjcombine_re_xilinx_cpld::{
    db::Database,
    fuzzdb::{FuzzDb, FuzzDbPart},
    speeddb::SpeedDb,
};
use prjcombine_types::{
    FbId, FbMcId, IoId,
    tiledb::{Tile, TileItem, TileItemKind},
};
use prjcombine_xc9500::{self as xc9500, FbBitCoord};
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

fn extract_mc_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<u32> {
    let mut tile = Tile::new();
    let neutral = device.kind == DeviceKind::Xc9500;
    let neutral = |_| neutral;
    for (fb, mc) in device.mcs() {
        let mcbits = &fpart.bits.fbs[fb].mcs[mc];
        let xlat_bit = |bit| map_mc_bit(device, fpart, fb, mc, bit);
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
            tile.insert(
                format!("PT[{i}].ALLOC"),
                extract_enum(
                    &mcbits.pt.as_ref().unwrap()[pt].alloc,
                    |alloc| match alloc {
                        prjcombine_re_xilinx_cpld::bits::PtAlloc::OrMain => "SUM".to_string(),
                        prjcombine_re_xilinx_cpld::bits::PtAlloc::OrExport => "EXPORT".to_string(),
                        prjcombine_re_xilinx_cpld::bits::PtAlloc::Special => "SPECIAL".to_string(),
                    },
                    xlat_bit,
                    "NONE",
                ),
                neutral,
            );
            tile.insert(
                format!("PT[{i}].HP"),
                extract_bool(mcbits.pt.as_ref().unwrap()[pt].hp, xlat_bit),
                neutral,
            );
        }
        for (dir, name) in [
            (ExportDir::Up, "IMPORT_UP_ALLOC"),
            (ExportDir::Down, "IMPORT_DOWN_ALLOC"),
        ] {
            tile.insert(
                name,
                extract_bool_to_enum(
                    mcbits.import.as_ref().unwrap()[dir],
                    xlat_bit,
                    "SUM",
                    "EXPORT",
                ),
                neutral,
            );
        }
        tile.insert(
            "EXPORT_CHAIN_DIR",
            extract_enum(
                mcbits.exp_dir.as_ref().unwrap(),
                |val| {
                    match val {
                        ExportDir::Up => "UP",
                        ExportDir::Down => "DOWN",
                    }
                    .into()
                },
                xlat_bit,
                "UP",
            ),
            neutral,
        );
        tile.insert(
            "SUM_HP",
            extract_bool(mcbits.hp.unwrap(), xlat_bit),
            neutral,
        );
        tile.insert("INV", extract_bool(mcbits.inv.unwrap(), xlat_bit), neutral);
        if let Some(ref enum_) = mcbits.oe_mux {
            tile.insert(
                "OE_MUX",
                extract_enum(
                    enum_,
                    |val| match val {
                        OeMuxVal::Pt => "PT".to_string(),
                        OeMuxVal::Foe(idx) => format!("FOE{}", idx.to_idx()),
                        _ => unreachable!(),
                    },
                    xlat_bit,
                    "PT",
                ),
                neutral,
            );
        }
        tile.insert(
            "OUT_MUX",
            extract_bool_to_enum(mcbits.ff_en.unwrap(), xlat_bit, "FF", "COMB"),
            neutral,
        );
        tile.insert(
            "CLK_MUX",
            extract_enum(
                &mcbits.clk_mux,
                |val| match val {
                    ClkMuxVal::Pt => "PT".to_string(),
                    ClkMuxVal::Fclk(idx) => format!("FCLK{}", idx.to_idx()),
                    _ => unreachable!(),
                },
                xlat_bit,
                "FCLK1",
            ),
            neutral,
        );
        if device.kind != DeviceKind::Xc9500 {
            tile.insert(
                "CLK_INV",
                extract_bool(mcbits.clk_inv.unwrap(), xlat_bit),
                neutral,
            );
            if let Some(bit) = mcbits.oe_inv {
                tile.insert("OE_INV".to_string(), extract_bool(bit, xlat_bit), neutral);
            }
            tile.insert(
                "CE_MUX",
                extract_enum(
                    mcbits.ce_mux.as_ref().unwrap(),
                    |val| {
                        match val {
                            CeMuxVal::PtRst => "PT2",
                            CeMuxVal::PtSet => "PT3",
                            _ => unreachable!(),
                        }
                        .into()
                    },
                    xlat_bit,
                    "NONE",
                ),
                neutral,
            );
        }
        tile.insert(
            "REG_MODE",
            extract_enum(
                &mcbits.reg_mode,
                |val| {
                    match val {
                        RegMode::Dff => "DFF",
                        RegMode::Tff => "TFF",
                        _ => unreachable!(),
                    }
                    .into()
                },
                xlat_bit,
                "DFF",
            ),
            neutral,
        );
        tile.insert(
            "RST_MUX",
            extract_enum(
                &mcbits.rst_mux,
                |val| {
                    match val {
                        SrMuxVal::Pt => "PT",
                        SrMuxVal::Fsr => "FSR",
                        _ => unreachable!(),
                    }
                    .into()
                },
                xlat_bit,
                "PT",
            ),
            neutral,
        );
        tile.insert(
            "SET_MUX",
            extract_enum(
                &mcbits.set_mux,
                |val| {
                    match val {
                        SrMuxVal::Pt => "PT",
                        SrMuxVal::Fsr => "FSR",
                        _ => unreachable!(),
                    }
                    .into()
                },
                xlat_bit,
                "PT",
            ),
            neutral,
        );
        tile.insert(
            "REG_INIT",
            extract_bool(mcbits.init.unwrap(), xlat_bit),
            neutral,
        );
        if device.kind == DeviceKind::Xc9500 {
            if let Some(ref enum_) = mcbits.obuf_oe_mode {
                tile.insert(
                    "IOB_OE_MUX",
                    extract_enum(
                        enum_,
                        |val| {
                            match val {
                                OeMode::Gnd => "GND",
                                OeMode::Vcc => "VCC",
                                OeMode::McOe => "OE_MUX",
                            }
                            .into()
                        },
                        xlat_bit,
                        "GND",
                    ),
                    neutral,
                );
            }
            tile.insert(
                "UIM_OE_MUX",
                extract_enum(
                    mcbits.uim_oe_mode.as_ref().unwrap(),
                    |val| {
                        match val {
                            OeMode::Gnd => "GND",
                            OeMode::Vcc => "VCC",
                            OeMode::McOe => "OE_MUX",
                        }
                        .into()
                    },
                    xlat_bit,
                    "GND",
                ),
                neutral,
            );
            tile.insert(
                "UIM_OUT_INV",
                extract_bool(mcbits.uim_out_inv.unwrap(), xlat_bit),
                neutral,
            );
        }
        if let Some(bit) = mcbits.is_gnd {
            tile.insert("IOB_GND", extract_bool(bit, xlat_bit), neutral);
        }

        if let Some(ref enum_) = mcbits.slew {
            tile.insert(
                "IOB_SLEW",
                extract_enum(
                    enum_,
                    |val| {
                        match val {
                            Slew::Slow => "SLOW",
                            Slew::Fast => "FAST",
                        }
                        .into()
                    },
                    xlat_bit,
                    "SLOW",
                ),
                neutral,
            );
        }
    }
    tile
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
    TileItem {
        bits: vec![res.unwrap()],
        kind: TileItemKind::BitVec {
            invert: BitVec::from_iter([device.kind == DeviceKind::Xc9500]),
        },
    }
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
    TileItem {
        bits: vec![res.unwrap()],
        kind: TileItemKind::BitVec {
            invert: BitVec::from_iter([device.kind == DeviceKind::Xc9500]),
        },
    }
}

fn extract_fb_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<FbBitCoord> {
    let mut tile = Tile::new();
    let neutral = device.kind == DeviceKind::Xc9500;
    let neutral = |_| neutral;

    tile.insert(
        "PULLUP_DISABLE",
        extract_fb_pullup_disable(device, fpart),
        neutral,
    );

    for fb in device.fbs() {
        let fbbits = &fpart.bits.fbs[fb];
        let xlat_bit = |bit| map_fb_bit(device, fpart, fb, bit);
        tile.insert(
            "ENABLE",
            extract_bool(fbbits.en.unwrap(), xlat_bit),
            neutral,
        );

        tile.insert(
            "EXPORT_ENABLE",
            extract_bool(fbbits.exp_en.unwrap(), xlat_bit),
            neutral,
        );
    }

    if device.kind == DeviceKind::Xc9500 {
        let a: Vec<_> = fpart.map.rprot.chunks(2).map(|x| x[0]).collect();
        let mut b: Vec<_> = fpart.map.rprot.chunks(2).map(|x| x[1]).collect();
        // special bug workaround!
        if device.fbs == 8 {
            assert_eq!(b[1].0, 0x3043);
            b[1].0 = 0x2883;
        }
        tile.insert("READ_PROT_A", extract_fb_prot(device, &a), neutral);
        tile.insert("READ_PROT_B", extract_fb_prot(device, &b), neutral);
    } else {
        tile.insert(
            "READ_PROT",
            extract_fb_prot(device, &fpart.map.rprot),
            neutral,
        );
    }
    tile.insert(
        "WRITE_PROT",
        extract_fb_prot(device, &fpart.map.wprot),
        neutral,
    );

    tile
}

fn extract_global_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<GlobalBitCoord> {
    let mut tile = Tile::new();
    let neutral = device.kind == DeviceKind::Xc9500;
    let neutral = |_| neutral;
    let xlat_bit = |bit| map_bit(device, fpart, bit);

    tile.items.insert(
        "FSR_INV".to_string(),
        extract_bool(fpart.bits.fsr_inv.unwrap(), xlat_bit),
    );
    if device.kind == DeviceKind::Xc9500 {
        for (i, &bit) in &fpart.bits.fclk_inv {
            tile.insert(format!("FCLK{i}_INV"), extract_bool(bit, xlat_bit), neutral);
        }
        for (i, &bit) in &fpart.bits.foe_inv {
            tile.insert(format!("FOE{i}_INV"), extract_bool(bit, xlat_bit), neutral);
        }
        for (i, enum_) in &fpart.bits.fclk_mux {
            tile.insert(
                format!("FCLK{i}_MUX"),
                extract_enum(enum_, |val| format!("GCLK{val}"), xlat_bit, "NONE"),
                neutral,
            );
        }
        for (i, enum_) in &fpart.bits.foe_mux {
            let kind = match fpart.bits.foe_mux.len() {
                2 => "SMALL",
                4 => "LARGE",
                _ => unreachable!(),
            };
            tile.insert(
                format!("FOE{i}_MUX.{kind}"),
                extract_enum(enum_, |val| format!("GOE{val}"), xlat_bit, "NONE"),
                neutral,
            );
        }
    } else {
        for (i, &bit) in &fpart.bits.fclk_en {
            tile.insert(
                format!("FCLK{i}_ENABLE"),
                extract_bool(bit, xlat_bit),
                neutral,
            );
        }
        for (i, &bit) in &fpart.bits.foe_en {
            tile.insert(
                format!("FOE{i}_ENABLE"),
                extract_bool(bit, xlat_bit),
                neutral,
            );
        }

        tile.insert(
            "TERM_MODE",
            extract_enum(
                fpart.bits.term_mode.as_ref().unwrap(),
                |val| match val {
                    TermMode::Pullup => unreachable!(),
                    TermMode::Keeper => "KEEPER".to_string(),
                },
                xlat_bit,
                "FLOAT",
            ),
            neutral,
        );
    }
    tile.insert(
        "USERCODE",
        extract_bitvec(&fpart.bits.usercode.unwrap(), xlat_bit),
        neutral,
    );
    if device.kind == DeviceKind::Xc9500Xv {
        tile.insert(
            "DONE",
            TileItem {
                bits: vec![map_bit_raw(device, fpart.map.done.unwrap())],
                kind: TileItemKind::BitVec {
                    invert: BitVec::from_iter([false]),
                },
            },
            neutral,
        );
    }
    tile
}

fn extract_imux_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<FbBitCoord> {
    let mut tile = Tile::new();
    let neutral = device.kind == DeviceKind::Xc9500;
    let neutral = |_| neutral;

    for fb in device.fbs() {
        let fbbits = &fpart.bits.fbs[fb];
        let xlat_bit = |bit| map_fb_bit(device, fpart, fb, bit);
        for im in device.fb_imuxes() {
            tile.insert(
                format!("IM[{im}].MUX"),
                extract_enum(
                    &fbbits.imux[im],
                    |val| match val {
                        ImuxInput::Fbk(mc) => format!("FBK_{mc}"),
                        ImuxInput::Mc((fb, mc)) => format!("MC_{fb}_{mc}"),
                        ImuxInput::Ibuf(IoId::Mc((fb, mc))) => format!("IOB_{fb}_{mc}"),
                        ImuxInput::Uim => "UIM".to_string(),
                        _ => unreachable!(),
                    },
                    xlat_bit,
                    "NONE",
                ),
                neutral,
            );
        }
    }
    tile
}

fn extract_ibuf_uim_bits(device: &Device, fpart: &FuzzDbPart) -> Tile<GlobalBitCoord> {
    let mut tile = Tile::new();
    let neutral = device.kind == DeviceKind::Xc9500;
    let neutral = |_| neutral;
    for (fb, mc) in device.mcs() {
        let bits = &fpart.bits.fbs[fb].mcs[mc].ibuf_uim_en;
        if bits.is_empty() {
            continue;
        }
        assert_eq!(bits.len(), 2);
        for (i, bit) in bits.iter().copied().enumerate() {
            tile.insert(
                format!("FB[{fb}].MC[{mc}].IBUF_UIM_ENABLE.{i}"),
                extract_bool(bit, |bit| map_bit(device, fpart, bit)),
                neutral,
            );
        }
    }
    tile
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
                    let (fuse, pol) = fpart.bits.fbs[fb].uim_mc[im][sfb][smc];
                    assert!(pol);
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
                    let ((fuse_t, pol_t), (fuse_f, pol_f)) =
                        fpart.bits.fbs[fb].mcs[mc].pt.as_ref().unwrap()[pt].and[im];
                    assert!(pol_t);
                    assert!(pol_f);
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

fn convert_io(io: IoId) -> (FbId, FbMcId) {
    let IoId::Mc((fb, mc)) = io else {
        unreachable!();
    };
    (fb, mc)
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = Database::from_file(args.db)?;
    let fdb = FuzzDb::from_file(args.fdb)?;
    let sdb = SpeedDb::from_file(args.sdb)?;

    let mut mc_bits: Option<Tile<_>> = None;
    let mut fb_bits: Option<Tile<_>> = None;
    let mut global_bits: Option<Tile<_>> = None;
    let mut imux_bits = BTreeMap::new();
    let mut ibuf_uim_bits: Option<Tile<_>> = None;

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
            bits.merge(&extract_mc_bits(device, fpart), |_| {
                device.kind == DeviceKind::Xc9500
            });
        } else {
            mc_bits = Some(extract_mc_bits(device, fpart));
        }
        if let Some(ref mut bits) = fb_bits {
            bits.merge(&extract_fb_bits(device, fpart), |_| {
                device.kind == DeviceKind::Xc9500
            });
        } else {
            fb_bits = Some(extract_fb_bits(device, fpart));
        }
        if let Some(ref mut bits) = global_bits {
            bits.merge(&extract_global_bits(device, fpart), |_| {
                device.kind == DeviceKind::Xc9500
            });
        } else {
            global_bits = Some(extract_global_bits(device, fpart));
        }
        let cur_imux_bits = extract_imux_bits(device, fpart);
        match imux_bits.entry(device.fbs) {
            btree_map::Entry::Vacant(e) => {
                e.insert(cur_imux_bits);
            }
            btree_map::Entry::Occupied(mut e) => e
                .get_mut()
                .merge(&cur_imux_bits, |_| device.kind == DeviceKind::Xc9500),
        }

        if device.kind == DeviceKind::Xc9500 && device.fbs == 16 {
            let cur_bits = extract_ibuf_uim_bits(device, fpart);
            if let Some(ref mut bits) = ibuf_uim_bits {
                bits.merge(&cur_bits, |_| true);
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

    let chips: EntityVec<_, _> = db
        .devices
        .values()
        .map(|dev| {
            let device = &dev.device;
            let idcode = match device.kind {
                DeviceKind::Xc9500 => 0x9500093,
                DeviceKind::Xc9500Xl => 0x9600093,
                DeviceKind::Xc9500Xv => 0x9700093,
                _ => unreachable!(),
            } | match device.fbs {
                2 => 0x02000,
                4 => 0x04000,
                6 => 0x06000,
                8 => 0x08000,
                12 => 0x12000,
                16 => 0x16000,
                _ => unreachable!(),
            };
            let mut io_special = BTreeMap::new();
            io_special.insert("GSR".to_string(), convert_io(device.sr_pad.unwrap()));
            for (i, &io) in &device.clk_pads {
                io_special.insert(format!("GCLK{i}"), convert_io(io));
            }
            for (i, &io) in &device.oe_pads {
                io_special.insert(format!("GOE{i}"), convert_io(io));
            }
            xc9500::Chip {
                kind: match device.kind {
                    DeviceKind::Xc9500 => xc9500::ChipKind::Xc9500,
                    DeviceKind::Xc9500Xl => xc9500::ChipKind::Xc9500Xl,
                    DeviceKind::Xc9500Xv => xc9500::ChipKind::Xc9500Xv,
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
                program_time: match (device.kind, device.fbs) {
                    (DeviceKind::Xc9500, 2) => 640,
                    (DeviceKind::Xc9500, 4) => 320,
                    (DeviceKind::Xc9500, _) => 160,
                    _ => 20000,
                },
                erase_time: if device.kind == DeviceKind::Xc9500 {
                    1300000
                } else {
                    200000
                },
            }
        })
        .collect();

    let mut bonds = EntityVec::new();
    let mut speeds = EntityVec::new();
    let mut parts: Vec<xc9500::Part> = vec![];
    'parts: for spart in &db.parts {
        let package = &db.packages[spart.package];
        let chip = xc9500::ChipId::from_idx(spart.device.to_idx());
        let mut io_special_override = BTreeMap::new();
        for (func, &pad) in &chips[chip].io_special {
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
                        match *v {
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
                            PkgPin::Io(IoId::Mc((fb, mc))) => xc9500::Pad::Iob(fb, mc),
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
        parts.push(xc9500::Part {
            name: spart.dev_name.clone(),
            chip,
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
        chips,
        bonds,
        speeds,
        parts,
        mc_bits,
        fb_bits,
        global_bits,
    };
    database.to_file(args.out)?;

    let json = database.to_json();
    std::fs::write(args.json, json.to_string())?;

    Ok(())
}
