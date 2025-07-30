use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use prjcombine_ecp::{
    bond::{AscPad, Bond, BondKind, BondPad, CfgPad, PfrPad, PllSet, SerdesPad},
    chip::{
        Chip, ChipKind, IoGroupKind, MachXo2Kind, PllLoc, PllPad, RowKind, SpecialIoKey,
        SpecialLocKey,
    },
};
use prjcombine_interconnect::{
    dir::{Dir, DirH, DirHV, DirV},
    grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId},
};
use prjcombine_re_lattice_naming::ChipNaming;
use prjcombine_re_lattice_rawdump::Part;
use prjcombine_types::bscan::BScanPad;
use unnamed_entity::EntityId;

use crate::{
    archive::{Archive, Reader, read_archive},
    chip::ChipExt,
};

fn get_filename(datadir: &Path, part: &Part, chip: &Chip) -> PathBuf {
    let (dir, fname) = match chip.kind {
        ChipKind::Ecp => (
            "ep5g00",
            format!("ep5g{r}x{c}", r = chip.rows.len(), c = chip.columns.len()),
        ),
        ChipKind::Xp => (
            "mg5g00",
            format!("mg5g{r}x{c}", r = chip.rows.len(), c = chip.columns.len()),
        ),
        ChipKind::MachXo => {
            if part.name.starts_with("LPTM") {
                (
                    "mj5g00p",
                    format!("mj5g{r}x{c}p", r = chip.rows.len(), c = chip.columns.len()),
                )
            } else {
                (
                    if chip.special_loc.contains_key(&SpecialLocKey::Ebr(0)) {
                        "mj5g00e"
                    } else {
                        "mj5g00"
                    },
                    format!("mj5g{r}x{c}", r = chip.rows.len(), c = chip.columns.len()),
                )
            }
        }
        ChipKind::Ecp2 => (
            "ep5a00",
            format!("ep5a{r}x{c}", r = chip.rows.len(), c = chip.columns.len()),
        ),
        ChipKind::Ecp2M => (
            "ep5m00",
            format!("ep5m{r}x{c}", r = chip.rows.len(), c = chip.columns.len()),
        ),
        ChipKind::Xp2 => (
            "mg5a00",
            format!("mg5a{r}x{c}", r = chip.rows.len(), c = chip.columns.len()),
        ),
        ChipKind::Ecp3 | ChipKind::Ecp3A => {
            let is_a = part.name.ends_with('A');
            if is_a {
                (
                    "ep5c00a",
                    format!("ec5a{r}x{c}", r = chip.rows.len(), c = chip.columns.len()),
                )
            } else {
                (
                    "ep5c00",
                    format!("ep5c{r}x{c}", r = chip.rows.len(), c = chip.columns.len()),
                )
            }
        }
        ChipKind::MachXo2(kind) => {
            let size = if kind == MachXo2Kind::MachXo2 {
                match chip.rows.len() {
                    6 => 256,
                    7 => 640,
                    11 => 1200,
                    14 => 2000,
                    21 => 4000,
                    26 => 7000,
                    30 => 10000,
                    _ => unreachable!(),
                }
            } else {
                match chip.rows.len() {
                    11 => 1300,
                    14 => 2100,
                    21 => 4300,
                    26 => 6900,
                    30 => 9400,
                    _ => unreachable!(),
                }
            };
            match part.name.as_str() {
                "LPTM21" => ("xo2c00ap", "xo2c1200p".to_string()),
                "LPTM21L" => ("xo2c00ap", "xo2c1200apl".to_string()),
                _ => match kind {
                    MachXo2Kind::MachXo2 => (
                        if chip.rows.values().any(|rd| rd.kind == RowKind::Ebr) {
                            "xo2c00a"
                        } else {
                            "xo2c00"
                        },
                        format!("xo2c{size}"),
                    ),
                    MachXo2Kind::MachXo3L => ("xo3c00a", format!("xo3c{size}")),
                    MachXo2Kind::MachXo3Lfp => ("xo3c00d", format!("xo3d{size}")),
                    MachXo2Kind::MachXo3D => ("se5c00", format!("se5c{size}")),
                    MachXo2Kind::MachNx => ("se5r00", format!("se5r{size}")),
                },
            }
        }
    };
    datadir.join(dir).join("data").join(format!("{fname}.pkg"))
}

#[derive(Debug, Clone)]
enum Value {
    String(String),
    #[allow(unused)]
    Float(f64),
    Int(i32),
}

struct PkgData {
    pkgs: BTreeSet<String>,
    pins: Vec<BTreeMap<String, Value>>,
}

fn parse_pkg(archive: &Archive) -> PkgData {
    let mut reader = Reader::new(&archive.entries["pininfo_table"].data);
    let _unk0 = reader.get_u32();
    let _ver = reader.get_nlstring();
    let num_lines = reader.get_u32();
    for _ in 0..num_lines {
        let _line = reader.get_nlstring();
    }
    let num_pkgs = reader.get_u32();
    let mut pkgs = BTreeSet::new();
    for _ in 0..num_pkgs {
        let _unk0 = reader.get_u32();
        let pkg = reader.get_zstring();
        let _ = reader.get_zstring();
        let _ = reader.get_zstring();
        let _ = reader.get_zstring();
        pkgs.insert(pkg);
    }
    let num_unk = reader.get_u32();
    for _ in 0..num_unk {
        let _ = reader.get_u32();
        let _ = reader.get_u32();
        let _ = reader.get_u32();
    }
    let num_die = reader.get_u32();
    for _ in 0..num_die {
        let _ = reader.get_u32();
        let _ = reader.get_zstring();
        let _ = reader.get_zstring();
    }
    let num_kv = reader.get_u32() as usize;
    let num_pins = reader.get_u32() as usize;
    let mut pins = vec![];
    for _ in 0..num_pins {
        pins.push(BTreeMap::new());
    }
    for _ in 0..num_kv {
        let kind = reader.get_u8();
        let key = reader.get_zstring();
        for i in 0..num_pins {
            let val = match kind {
                1 => Value::String(reader.get_zstring()),
                2 => Value::Int(reader.get_i32()),
                3 => Value::Float(reader.get_f64()),
                _ => panic!("weird kind {kind}"),
            };
            pins[i].insert(key.clone(), val);
        }
    }
    PkgData { pkgs, pins }
}

fn parse_io(chip: &Chip, func: &str) -> Option<EdgeIoCoord> {
    let (func, iob_idx) = if let Some(f) = func.strip_suffix("E_A") {
        (f, 4)
    } else if let Some(f) = func.strip_suffix("E_B") {
        (f, 5)
    } else if let Some(f) = func.strip_suffix("E_C") {
        (f, 6)
    } else if let Some(f) = func.strip_suffix("E_D") {
        (f, 7)
    } else if let Some(f) = func.strip_suffix('A') {
        (f, 0)
    } else if let Some(f) = func.strip_suffix('B') {
        (f, 1)
    } else if let Some(f) = func.strip_suffix('C') {
        (f, 2)
    } else if let Some(f) = func.strip_suffix('D') {
        (f, 3)
    } else if let Some(f) = func.strip_suffix('E') {
        (f, 4)
    } else if let Some(f) = func.strip_suffix('F') {
        (f, 5)
    } else {
        (func, 0)
    };
    let mut iob = TileIobId::from_idx(iob_idx);
    if let Some(r) = func.strip_prefix("PL")
        && let Ok(r) = r.parse()
    {
        let mut row = chip.xlat_row(r);
        #[allow(clippy::single_match)]
        match chip.kind {
            ChipKind::Ecp3 | ChipKind::Ecp3A if iob_idx < 4 => {
                if chip.rows[row].io_w == IoGroupKind::None {
                    row -= 2;
                } else {
                    iob = TileIobId::from_idx(iob_idx + 2);
                }
            }
            _ => (),
        }
        Some(EdgeIoCoord::W(row, iob))
    } else if let Some(r) = func.strip_prefix("PR")
        && let Ok(r) = r.parse()
    {
        let mut row = chip.xlat_row(r);
        #[allow(clippy::single_match)]
        match chip.kind {
            ChipKind::Ecp3 | ChipKind::Ecp3A if iob_idx < 4 => {
                if chip.rows[row].io_e == IoGroupKind::None {
                    row -= 2;
                } else {
                    iob = TileIobId::from_idx(iob_idx + 2);
                }
            }
            _ => (),
        }
        Some(EdgeIoCoord::E(row, iob))
    } else if let Some(c) = func.strip_prefix("PB")
        && let Ok(c) = c.parse()
    {
        let mut col = chip.xlat_col(c);
        #[allow(clippy::single_match)]
        match chip.kind {
            ChipKind::Ecp3 | ChipKind::Ecp3A if iob_idx < 4 => {
                if chip.columns[col].io_s == IoGroupKind::None {
                    col -= 2;
                    iob = TileIobId::from_idx(iob_idx + 2);
                }
            }
            _ => (),
        }
        Some(EdgeIoCoord::S(col, iob))
    } else if let Some(c) = func.strip_prefix("PT")
        && let Ok(c) = c.parse()
    {
        let mut col = chip.xlat_col(c);
        #[allow(clippy::single_match)]
        match chip.kind {
            ChipKind::Ecp3 | ChipKind::Ecp3A if iob_idx < 4 => {
                if chip.columns[col].io_n == IoGroupKind::None {
                    col -= 2;
                    iob = TileIobId::from_idx(iob_idx + 2);
                }
            }
            _ => (),
        }
        Some(EdgeIoCoord::N(col, iob))
    } else {
        None
    }
}

fn parse_pfr_io(func: &str) -> Option<EdgeIoCoord> {
    let (func, iob_idx) = if let Some(f) = func.strip_suffix('A') {
        (f, 0)
    } else if let Some(f) = func.strip_suffix('B') {
        (f, 1)
    } else {
        (func, 0)
    };
    let iob = TileIobId::from_idx(iob_idx);
    if let Some(r) = func.strip_prefix("PL")
        && let Ok(r) = r.parse::<usize>()
    {
        let row = RowId::from_idx(55 - r);
        Some(EdgeIoCoord::W(row, iob))
    } else if let Some(r) = func.strip_prefix("PR")
        && let Ok(r) = r.parse::<usize>()
    {
        let row = RowId::from_idx(55 - r);
        Some(EdgeIoCoord::E(row, iob))
    } else if let Some(c) = func.strip_prefix("PB")
        && let Ok(c) = c.parse::<usize>()
    {
        let col = ColId::from_idx(c - 1);
        Some(EdgeIoCoord::S(col, iob))
    } else if let Some(c) = func.strip_prefix("PT")
        && let Ok(c) = c.parse::<usize>()
    {
        let col = ColId::from_idx(c - 1);
        Some(EdgeIoCoord::N(col, iob))
    } else {
        None
    }
}

pub struct BondResult {
    pub bond: Bond,
    pub special_io: BTreeMap<SpecialIoKey, EdgeIoCoord>,
}

pub fn process_bond(datadir: &Path, part: &Part, chip: &Chip, _naming: &ChipNaming) -> BondResult {
    // println!("{} {}", part.name, part.package);
    let fname = get_filename(datadir, part, chip);
    let archive = read_archive(&fname);
    let pkg_data = parse_pkg(&archive);
    assert!(pkg_data.pkgs.contains(&part.package));
    let kind = if part.name.starts_with("LPTM21") {
        BondKind::Asc
    } else if part.name == "LFMNX-50" {
        BondKind::MachNx
    } else {
        BondKind::Single
    };
    let mut bond = Bond {
        kind,
        pins: Default::default(),
        pfr_io: Default::default(),
    };
    let mut special_io = BTreeMap::new();
    let mut bscan = vec![];
    let mut pll_xlat = BTreeMap::new();
    let mut virt_io: BTreeMap<String, Vec<BondPad>> = BTreeMap::new();
    for (&loc, &cell) in &chip.special_loc {
        if let SpecialLocKey::Pll(loc) = loc {
            pll_xlat.insert(cell, loc);
        }
    }
    let pll_rows_s = if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) {
        Vec::from_iter(chip.rows.ids().rev().filter(|&row| {
            row < chip.row_clk && matches!(chip.rows[row].kind, RowKind::Ebr | RowKind::Dsp)
        }))
    } else {
        Vec::from_iter(chip.rows.ids().filter(|&row| {
            row < chip.row_clk && matches!(chip.rows[row].kind, RowKind::Ebr | RowKind::Dsp)
        }))
    };
    let pll_rows_n = if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) {
        Vec::from_iter(chip.rows.ids().filter(|&row| {
            row >= chip.row_clk && matches!(chip.rows[row].kind, RowKind::Ebr | RowKind::Dsp)
        }))
    } else {
        Vec::from_iter(chip.rows.ids().rev().filter(|&row| {
            row >= chip.row_clk && matches!(chip.rows[row].kind, RowKind::Ebr | RowKind::Dsp)
        }))
    };
    let mut serdes_xlat = BTreeMap::new();
    for (col, cd) in &chip.columns {
        if cd.io_s == IoGroupKind::Serdes {
            serdes_xlat.insert(cd.bank_s.unwrap(), col);
        }
    }
    for pin_info in &pkg_data.pins {
        let Value::String(ref func) = pin_info["FNC"] else {
            unreachable!()
        };
        let Value::String(ref cfg) = pin_info["CFG"] else {
            unreachable!()
        };
        let Value::Int(bank) = pin_info["BANK"] else {
            unreachable!()
        };
        let Value::String(ref pin) = pin_info[&part.package] else {
            unreachable!()
        };
        let Value::String(ref bs_type) = pin_info["BS_TYPE"] else {
            unreachable!()
        };
        let Value::Int(bs_order) = pin_info["BS_ORDER"] else {
            unreachable!()
        };
        if ((bs_order == 0 || bs_type == "L") && (pin.starts_with("Unused") || pin == "UNUSED"))
            || pin == "VCC"
            || pin == "GND"
            || pin == "VCCAUX"
            || pin == "VCCA"
            || pin == "VDDD"
            || pin == "VSSD_GND"
            || pin == "VSSA"
            || pin == "VSS"
            || pin.starts_with("VCCNX")
            || pin.starts_with("VCCXD")
            || pin.starts_with("VCCIO")
            || pin.starts_with("VCCPLL")
            || pin.starts_with("VTT")
        {
            continue;
        }
        // I hate you. I hate you so fucking much.
        let pin = match pin.as_str() {
            "NXBOOT_MCLK" => "NXBOOTMCLK",
            "NXBOOT_MOSI" => "NXBOOTMOSI",
            "NXBOOT_MISO" => "NXBOOTMISO",
            "NXBOOT_MCSN" => "NXBOOTMCSN",
            "NXPROGRAMN" => "NX_PROGRAMN",
            "NXJTAGEN" => "NX_JTAGEN",
            _ => pin,
        };
        let pin = pin.to_string();
        let pad = if let Some(pads) = virt_io.get_mut(func) {
            pads.sort();
            match *pads.as_slice() {
                [BondPad::Io(io), BondPad::Asc(asc)] => BondPad::IoAsc(io, asc),
                [BondPad::Io(io), BondPad::Pfr(pfr)] => BondPad::IoPfr(io, pfr),
                [BondPad::Pfr(pad)] => BondPad::Pfr(pad),
                [BondPad::Io(pad)] if func == "FLASHCSN" => BondPad::Io(pad),
                _ => {
                    print!("WEIRD VIRT {func}:");
                    for pad in pads {
                        print!(" {pad}");
                    }
                    println!();
                    continue;
                }
            }
        } else if let Some(io) = parse_io(chip, func) {
            let mut spec_io = io;
            let mut spec = if let Some(pclk) = cfg.strip_prefix("PCLK") {
                let pclk = pclk.strip_suffix("/INTEST_OVER").unwrap_or(pclk);
                let (which, idx) = pclk.split_once('_').unwrap();
                if let Some(pclk_bank) = which.strip_prefix('T') {
                    let pclk_bank: i32 = pclk_bank.parse().unwrap();
                    if chip.kind != ChipKind::MachXo {
                        assert_eq!(bank, pclk_bank);
                    }
                } else if let Some(pclk_bank) = which.strip_prefix('C') {
                    assert_eq!(spec_io.iob().to_idx() % 2, 1);
                    spec_io = spec_io.with_iob(TileIobId::from_idx(spec_io.iob().to_idx() - 1));
                    let pclk_bank: i32 = pclk_bank.parse().unwrap();
                    assert_eq!(bank, pclk_bank);
                } else {
                    unreachable!();
                }
                let mut idx = idx.parse().unwrap();
                if matches!(
                    chip.kind,
                    ChipKind::Ecp2
                        | ChipKind::Ecp2M
                        | ChipKind::Xp2
                        | ChipKind::Ecp3
                        | ChipKind::Ecp3A
                ) {
                    assert_eq!(idx, 0);
                    idx = match bank {
                        0 | 2 | 5 | 7 => 0,
                        1 | 3 | 4 | 6 => 1,
                        _ => unreachable!(),
                    };
                }
                if matches!(chip.kind, ChipKind::MachXo2(_)) && bank >= 3 && chip.rows.len() >= 14 {
                    idx = (bank - 3) as u8;
                }
                Some(SpecialIoKey::Clock(io.edge(), idx))
            } else if let Some(vref_bank) = cfg.strip_prefix("VREF1_") {
                let vref_bank = vref_bank.parse().unwrap();
                assert_eq!(bank as u32, vref_bank);
                Some(SpecialIoKey::Vref1(vref_bank))
            } else if let Some(vref_bank) = cfg.strip_prefix("VREF2_") {
                let vref_bank = vref_bank.parse().unwrap();
                assert_eq!(bank as u32, vref_bank);
                Some(SpecialIoKey::Vref2(vref_bank))
            } else {
                match cfg.as_str() {
                    _ if cfg.contains("PLL") || cfg.contains("DLL") => 'pll: {
                        let (loc, sig) = cfg.split_once('_').unwrap();
                        let (hv, idx) = if loc.len() < 4 {
                            let hv = match loc {
                                "LLC" => DirHV::SW,
                                "ULC" => DirHV::NW,
                                "LRC" => DirHV::SE,
                                "URC" => DirHV::NE,
                                "L" if matches!(chip.kind, ChipKind::MachXo2(_)) => DirHV::NW,
                                "R" if matches!(chip.kind, ChipKind::MachXo2(_)) => DirHV::NE,
                                _ => panic!("weird PLL loc {loc}"),
                            };
                            (hv, 0)
                        } else {
                            let idx: u8 = loc[3..].parse().unwrap();
                            let loc = &loc[..3];
                            let hv = match loc {
                                "LLM" => DirHV::SW,
                                "LUM" => DirHV::NW,
                                "RLM" => DirHV::SE,
                                "RUM" => DirHV::NE,
                                _ => panic!("weird PLL loc {loc}"),
                            };
                            (hv, idx)
                        };
                        let loc = match chip.kind {
                            ChipKind::Ecp
                            | ChipKind::Xp
                            | ChipKind::MachXo
                            | ChipKind::Xp2
                            | ChipKind::MachXo2(_) => PllLoc::new(hv, idx),
                            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                                let cell = CellCoord::new(
                                    DieId::from_idx(0),
                                    if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) {
                                        match hv.h {
                                            DirH::W => chip.col_w() + 1,
                                            DirH::E => chip.col_e() - 1,
                                        }
                                    } else {
                                        chip.col_edge(hv.h)
                                    },
                                    match hv.v {
                                        DirV::S => pll_rows_s[idx as usize],
                                        DirV::N => pll_rows_n[idx as usize],
                                    },
                                );
                                pll_xlat[&cell]
                            }
                        };
                        let (pad, is_c) = match sig {
                            "PLLT_IN_A" => (PllPad::PllIn0, false),
                            "PLLC_IN_A" => (PllPad::PllIn0, true),
                            "PLLT_FB_A" => (PllPad::PllFb, false),
                            "PLLC_FB_A" => (PllPad::PllFb, true),
                            "GPLLT_IN_A" => (PllPad::PllIn0, false),
                            "GPLLC_IN_A" | "GPLLT_IN_B" => (PllPad::PllIn0, true),
                            "GPLLT_FB_A" => (PllPad::PllFb, false),
                            "GPLLC_FB_A" | "GPLLT_FB_B" => (PllPad::PllFb, true),
                            "SPLLT_IN_A" => (PllPad::PllIn0, false),
                            "SPLLC_IN_A" => (PllPad::PllIn0, true),
                            "SPLLT_FB_A" => (PllPad::PllFb, false),
                            "SPLLC_FB_A" => (PllPad::PllFb, true),
                            "GDLLT_IN_A" => (PllPad::DllIn0, false),
                            "GDLLC_IN_A" | "GDLLT_IN_B" => (PllPad::DllIn0, true),
                            "GDLLT_FB_A" => (PllPad::DllFb, false),
                            "GDLLC_FB_A" | "GDLLT_FB_B" => (PllPad::DllFb, true),
                            // bug? bug.
                            "GDLLC_FB_D" => (PllPad::DllFb, true),
                            "GPLLT_IN" => (PllPad::PllIn0, false),
                            "GPLLC_IN" => (PllPad::PllIn0, true),
                            "GPLLT_FB" => (PllPad::PllFb, false),
                            "GPLLC_FB" => (PllPad::PllFb, true),
                            "GPLLT_MFGOUT1" | "GPLLC_MFGOUT1" | "GPLLT_MFGOUT2"
                            | "GPLLC_MFGOUT2" => break 'pll None,
                            _ => panic!("weird PLL pin {sig}"),
                        };
                        if is_c {
                            assert_eq!(spec_io.iob().to_idx() % 2, 1);
                            spec_io =
                                spec_io.with_iob(TileIobId::from_idx(spec_io.iob().to_idx() - 1))
                        }
                        Some(SpecialIoKey::Pll(pad, loc))
                    }
                    "DQS" | "Unused" => None,
                    "TSALLPAD" => Some(SpecialIoKey::TsAll),
                    "GSR_PADN" => Some(SpecialIoKey::Gsr),
                    "D0" | "D0/SPIFASTN" => Some(SpecialIoKey::D(0)),
                    "D1" => Some(SpecialIoKey::D(1)),
                    "D2" => Some(SpecialIoKey::D(2)),
                    "D3" | "D3/SI" => Some(SpecialIoKey::D(3)),
                    "D4" | "D4/SO" => Some(SpecialIoKey::D(4)),
                    "D5" => Some(SpecialIoKey::D(5)),
                    "D6" | "D6/SPID1" => Some(SpecialIoKey::D(6)),
                    "D7" | "D7/SPID0" => Some(SpecialIoKey::D(7)),
                    "CSN" | "CSN/SN/CONT1N/OEN" => Some(SpecialIoKey::CsN),
                    "CS1N" | "CS1N/HOLDN/CONT2N/RDY" => Some(SpecialIoKey::Cs1N),
                    "WRITEN" => Some(SpecialIoKey::WriteN),
                    "DI" | "DI/CSSPI0N/CEN/CSSPIN" => Some(SpecialIoKey::Di),
                    "DOUT" | "DOUT,CSON" | "DOUT_CSON" | "DOUT/CSON/CSSPI1N" => {
                        Some(SpecialIoKey::Dout)
                    }
                    "BUSY" | "BUSY/SISPI/AVDN" => Some(SpecialIoKey::Busy),
                    "MCLK" => Some(SpecialIoKey::MClk),
                    // XP2 stuff
                    "INITN" => Some(SpecialIoKey::InitB),
                    "SI" => Some(SpecialIoKey::SpiSdi),
                    "SO" => Some(SpecialIoKey::SpiSdo),
                    "CCLK" => Some(SpecialIoKey::Cclk),
                    "CSSPIN" => Some(SpecialIoKey::SpiCCsB),
                    "CSSPISN" => Some(SpecialIoKey::SpiPCsB),
                    "CFG1" => Some(SpecialIoKey::M1),
                    "DONE" => Some(SpecialIoKey::Done),
                    "PROGRAMN" => Some(SpecialIoKey::ProgB),
                    "MFG_EXT_CLK" | "FL_EXT_PULSE_D" | "FL_EXT_PULSE_G" => None,
                    // MachXO2
                    "MCLK/CCLK" => Some(SpecialIoKey::Cclk),
                    "CSSPIN/MD4/TDOB" => Some(SpecialIoKey::SpiCCsB),
                    "SN/MD5/SCAN_SHFT_ENB/TDIB" | "SN/MD5/SCAN_SHFT_ENB/TDIB/ATB_SENSE_1" => {
                        Some(SpecialIoKey::SpiPCsB)
                    }
                    "SO/SPISO/IO1/MD1/TDIL" => Some(SpecialIoKey::SpiCipo),
                    "SI/SISPI/IO0/MD0/TDOR" | "SI/SISPI/IO0/MD0/TDOR/ATB_FORCE_1" => {
                        Some(SpecialIoKey::SpiCopi)
                    }
                    "SCL/IO2/MD2/ATB_SENSE/PCLKT0_0" => Some(SpecialIoKey::I2cScl),
                    "SDA/IO3/MD3/ATB_FORCE/PCLKC0_0/TDOT" => Some(SpecialIoKey::I2cSda),
                    "TCK/TEST_CLK" => Some(SpecialIoKey::Tck),
                    "TMS" => Some(SpecialIoKey::Tms),
                    "TDI/MD7" => Some(SpecialIoKey::Tdi),
                    "TDO" => Some(SpecialIoKey::Tdo),
                    "JTAGENB/MD6/TDIR" | "JTAGENB/MD6/TDIR/ATB_SENSE_2" => {
                        Some(SpecialIoKey::JtagEn)
                    }
                    "PROGRAMN/ATB_FORCE_2" => Some(SpecialIoKey::ProgB),
                    _ => {
                        println!("\tPIN {pin:5} {func:8} {cfg:20} {bank} {io}");
                        None
                    }
                }
            };
            if matches!(spec, Some(SpecialIoKey::D(_))) && chip.kind == ChipKind::MachXo {
                spec = None;
            }
            if let Some(spec) = spec {
                // well.
                if let Some(prev_io) = special_io.insert(spec, spec_io) {
                    assert_eq!(prev_io, spec_io, "fail on {spec}");
                }
                if matches!(chip.kind, ChipKind::MachXo2(_)) && spec == SpecialIoKey::I2cScl {
                    let spec = SpecialIoKey::Clock(Dir::N, 0);
                    if let Some(prev_io) = special_io.insert(spec, spec_io) {
                        assert_eq!(prev_io, spec_io, "fail on {spec}");
                    }
                }
            }
            let io_bank = chip.get_io_bank(io);
            assert_eq!(io_bank as i32, bank);
            BondPad::Io(io)
        } else if bond.kind == BondKind::MachNx
            && let Some(pfr_func) = func.strip_prefix("PFR_")
            && let Some(io) = parse_pfr_io(pfr_func)
        {
            BondPad::Pfr(PfrPad::Io(io))
        } else if let Some((corner, pad)) = func.split_once("_SQ_")
            && chip.kind == ChipKind::Ecp2M
        {
            let (edge, col) = match corner {
                "LLC" => (DirV::S, chip.col_w() + 1),
                "LRC" => (DirV::S, chip.col_e() - 27),
                "ULC" => (DirV::N, chip.col_w() + 1),
                "URC" => (DirV::N, chip.col_e() - 27),
                _ => unreachable!(),
            };
            let pad = if let Ok(channel) = pad[pad.len() - 1..].parse()
                && pad != "VCCAUX33"
            {
                match &pad[..pad.len() - 1] {
                    "HDINP" => SerdesPad::InP(channel),
                    "HDINN" => SerdesPad::InN(channel),
                    "HDOUTP" => SerdesPad::OutP(channel),
                    "HDOUTN" => SerdesPad::OutN(channel),
                    "VCCTX" => SerdesPad::VccTx(channel),
                    "VCCRX" => SerdesPad::VccRx(channel),
                    "VCCIB" => SerdesPad::VccIB(channel),
                    "VCCOB" => SerdesPad::VccOB(channel),
                    _ => panic!("umm {pad}"),
                }
            } else {
                match pad {
                    "REFCLKP" => SerdesPad::ClkP,
                    "REFCLKN" => SerdesPad::ClkN,
                    "VCCAUX33" => SerdesPad::VccAux33,
                    "VCCP" => SerdesPad::VccP,
                    _ => panic!("umm {pad}"),
                }
            };
            let exp_bank = match edge {
                DirV::S => chip.columns[col].bank_s.unwrap(),
                DirV::N => chip.columns[col].bank_n.unwrap(),
            };
            assert_eq!(bank, exp_bank as i32);
            BondPad::Serdes(edge, col, pad)
        } else if func.starts_with("PCS") && matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) {
            let (loc, pad) = func.split_once('_').unwrap();
            let exp_bank = match loc {
                "PCSA" => 50,
                "PCSB" => 51,
                "PCSC" => 52,
                "PCSD" => 53,
                _ => unreachable!(),
            };
            let col = serdes_xlat[&exp_bank];
            assert_eq!(bank, exp_bank as i32);
            let pad = if let Ok(channel) = pad[pad.len() - 1..].parse()
                && !pad.starts_with("VCCTX")
            {
                match &pad[..pad.len() - 1] {
                    "HDINP" => SerdesPad::InP(channel),
                    "HDINN" => SerdesPad::InN(channel),
                    "HDOUTP" => SerdesPad::OutP(channel),
                    "HDOUTN" => SerdesPad::OutN(channel),
                    "VCCRX" => SerdesPad::VccRx(channel),
                    "VCCIB" => SerdesPad::VccIB(channel),
                    "VCCOB" => SerdesPad::VccOB(channel),
                    _ => panic!("umm {pad}"),
                }
            } else {
                match pad {
                    "REFCLKP" => SerdesPad::ClkP,
                    "REFCLKN" => SerdesPad::ClkN,
                    "VCCTX01" => SerdesPad::VccTx(0),
                    "VCCTX23" => SerdesPad::VccTx(2),
                    "VCCP" => SerdesPad::VccP,
                    _ => panic!("umm {pad}"),
                }
            };
            BondPad::Serdes(DirV::S, col, pad)
        } else if let Some(vccio_bank) = func
            .strip_prefix("VCCIO")
            .or_else(|| func.strip_prefix("VCCO"))
        {
            BondPad::VccIo(vccio_bank.parse().unwrap())
        } else if bond.kind == BondKind::MachNx
            && let Some(vccio_bank) = func.strip_prefix("VCCXDIO")
            && let Ok(vccio_bank) = vccio_bank.parse()
        {
            BondPad::VccIo(vccio_bank)
        } else if bond.kind == BondKind::MachNx
            && let Some(vccio_bank) = func.strip_prefix("VCCNXIO")
        {
            BondPad::Pfr(PfrPad::VccIo(vccio_bank.parse().unwrap()))
        } else if let Some(vtt_bank) = func.strip_prefix("VTT") {
            BondPad::Vtt(vtt_bank.parse().unwrap())
        } else if let Some(idx) = func.strip_prefix("GPIO")
            && bond.kind == BondKind::Asc
        {
            BondPad::Asc(AscPad::Gpio(idx.parse().unwrap()))
        } else if let Some(idx) = func.strip_prefix("TRIM")
            && bond.kind == BondKind::Asc
        {
            BondPad::Asc(AscPad::Trim(idx.parse().unwrap()))
        } else if let Some(idx) = func.strip_prefix("HVOUT")
            && bond.kind == BondKind::Asc
        {
            BondPad::Asc(AscPad::HvOut(idx.parse().unwrap()))
        } else {
            match func.as_str() {
                "TCK" => BondPad::Cfg(CfgPad::Tck),
                "TMS" => BondPad::Cfg(CfgPad::Tms),
                "TDI" => BondPad::Cfg(CfgPad::Tdi),
                "TDO" => BondPad::Cfg(CfgPad::Tdo),
                "PROGRAMN" => BondPad::Cfg(CfgPad::ProgB),
                "CCLK" => BondPad::Cfg(CfgPad::Cclk),
                "INITN" => BondPad::Cfg(CfgPad::InitB),
                "DONE" => BondPad::Cfg(CfgPad::Done),
                "CFG0" => BondPad::Cfg(CfgPad::M0),
                "CFG1" => BondPad::Cfg(CfgPad::M1),
                "CFG2" => BondPad::Cfg(CfgPad::M2),
                "SLEEPN/TOE" => BondPad::Cfg(CfgPad::SleepB),
                "TOE" => BondPad::Cfg(CfgPad::Toe),
                // ECP2M special.
                "WRITEN" => BondPad::Cfg(CfgPad::WriteN),
                "CS1N" => BondPad::Cfg(CfgPad::Cs1N),
                "CSN" => BondPad::Cfg(CfgPad::CsN),
                "D0" => BondPad::Cfg(CfgPad::D(0)),
                "D1" => BondPad::Cfg(CfgPad::D(1)),
                "D2" => BondPad::Cfg(CfgPad::D(2)),
                "D3" => BondPad::Cfg(CfgPad::D(3)),
                "D4" => BondPad::Cfg(CfgPad::D(4)),
                "D5" => BondPad::Cfg(CfgPad::D(5)),
                "D6" => BondPad::Cfg(CfgPad::D(6)),
                "D7" => BondPad::Cfg(CfgPad::D(7)),
                "DI" => BondPad::Cfg(CfgPad::Di),
                "DOUT_CSON" => BondPad::Cfg(CfgPad::Dout),
                "BUSY" => BondPad::Cfg(CfgPad::Busy),
                // ???
                "HFP" => BondPad::Cfg(CfgPad::Hfp),
                "SLEEPN/NC" if chip.kind == ChipKind::MachXo => {
                    // AAAAAAAAAAAAAAAAAAAAAAAa
                    let io = chip.special_io[&SpecialIoKey::SleepN];
                    BondPad::Io(io)
                }
                "VCC" => BondPad::VccInt,
                "VCCAUX" => BondPad::VccAux,
                "VCCJ" => BondPad::VccJtag,
                "VCCPLL" => BondPad::VccPll(PllSet::All),
                "L_VCCPLL" | "VCCPLL_L" => BondPad::VccPll(PllSet::Side(DirH::W)),
                "R_VCCPLL" | "VCCPLL_R" => BondPad::VccPll(PllSet::Side(DirH::E)),
                "LLM0_VCCPLL" if chip.kind == ChipKind::Ecp2 => {
                    BondPad::VccPll(PllSet::Quad(DirHV::SW))
                }
                "RLM0_VCCPLL" if chip.kind == ChipKind::Ecp2 => {
                    BondPad::VccPll(PllSet::Quad(DirHV::SE))
                }
                "LUM0_VCCPLL" if chip.kind == ChipKind::Ecp2 => {
                    BondPad::VccPll(PllSet::Quad(DirHV::NW))
                }
                "RUM0_VCCPLL" if chip.kind == ChipKind::Ecp2 => {
                    BondPad::VccPll(PllSet::Quad(DirHV::NE))
                }
                "LLC_VCCPLL" if chip.kind == ChipKind::Xp2 => {
                    BondPad::VccPll(PllSet::Quad(DirHV::SW))
                }
                "LRC_VCCPLL" if chip.kind == ChipKind::Xp2 => {
                    BondPad::VccPll(PllSet::Quad(DirHV::SE))
                }
                "ULC_VCCPLL" if chip.kind == ChipKind::Xp2 => {
                    BondPad::VccPll(PllSet::Quad(DirHV::NW))
                }
                "URC_VCCPLL" if chip.kind == ChipKind::Xp2 => {
                    BondPad::VccPll(PllSet::Quad(DirHV::NE))
                }
                "LLC_GNDPLL" if chip.kind == ChipKind::Xp2 => {
                    BondPad::GndPll(PllSet::Quad(DirHV::SW))
                }
                "LRC_GNDPLL" if chip.kind == ChipKind::Xp2 => {
                    BondPad::GndPll(PllSet::Quad(DirHV::SE))
                }
                "ULC_GNDPLL" if chip.kind == ChipKind::Xp2 => {
                    BondPad::GndPll(PllSet::Quad(DirHV::NW))
                }
                "URC_GNDPLL" if chip.kind == ChipKind::Xp2 => {
                    BondPad::GndPll(PllSet::Quad(DirHV::NE))
                }
                "LUM0_VCCPLL" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                    BondPad::VccPll(PllSet::Side(DirH::W))
                }
                "RUM0_VCCPLL" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                    BondPad::VccPll(PllSet::Side(DirH::E))
                }
                "VCCP0" if chip.kind == ChipKind::Xp => BondPad::VccPll(PllSet::Side(DirH::W)),
                "VCCP1" if chip.kind == ChipKind::Xp => BondPad::VccPll(PllSet::Side(DirH::E)),
                "GNDP0" if chip.kind == ChipKind::Xp => BondPad::VccPll(PllSet::Side(DirH::W)),
                "GNDP1" if chip.kind == ChipKind::Xp => BondPad::VccPll(PllSet::Side(DirH::E)),
                "LLM0_PLLCAP" if matches!(chip.kind, ChipKind::Ecp2 | ChipKind::Ecp2M) => {
                    BondPad::PllCap(PllSet::Side(DirH::W))
                }
                "RLM0_PLLCAP" if matches!(chip.kind, ChipKind::Ecp2 | ChipKind::Ecp2M) => {
                    BondPad::PllCap(PllSet::Side(DirH::E))
                }
                "VCCA" => BondPad::VccA,
                "GND" | "GNDAUX" => BondPad::Gnd,
                "VSS" if bond.kind == BondKind::MachNx => BondPad::Gnd,
                "Unused" | "NC" => BondPad::Nc,
                "XRES" => BondPad::XRes,
                "TEMPVSS" => BondPad::TempVss,
                "TEMPSENSE" => BondPad::TempSense,
                "RESERVE" => BondPad::Other,
                "WDAT" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::WDat),
                "RDAT" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::RDat),
                "WRCLK" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::WrClk),
                "ASCCLK" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::AscClk),
                "SCL" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::Scl),
                "SDA" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::Sda),
                "I2C_ADDR" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::I2cAddr),
                "RESETb" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::ResetB),
                "HVIMONP" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::HviMonP),
                "HIMONN" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::HiMonN),
                "IMON1P" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::IMonP(1)),
                "IMON1N" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::IMonN(1)),
                "TMON1P" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::TMonP(1)),
                "TMON1N" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::TMonN(1)),
                "TMON2P" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::TMonP(2)),
                "TMON2N" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::TMonN(2)),
                "VMON1" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(1)),
                "VMON2" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(2)),
                "VMON3" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(3)),
                "VMON4" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(4)),
                "VMON5" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(5)),
                "VMON6" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(6)),
                "VMON7" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(7)),
                "VMON8" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(8)),
                "VMON9" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMon(9)),
                "VMON1GS" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMonGs(1)),
                "VMON2GS" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMonGs(2)),
                "VMON3GS" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMonGs(3)),
                "VMON4GS" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VMonGs(4)),
                "LDRV" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::Ldrv),
                "HDRV" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::Hdrv),
                "VDC" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::Vdc),
                "VSSA" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VssA),
                "VDDA" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VddA),
                "VDDD" if bond.kind == BondKind::Asc => BondPad::Asc(AscPad::VddD),
                "VSSD_GND" if bond.kind == BondKind::Asc => BondPad::Gnd,
                "JTAG_EN" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::JtagEn),
                "ADC_REFP0" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::AdcRefP0),
                "ADC_REFP1" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::AdcRefP1),
                "ADC_DP0" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::AdcDp0),
                "ADC_DP1" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::AdcDp1),
                "VSSADC" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VssAdc),
                "VCCXDNXIO0XDIO4" | "VCCXDXDIO4" if bond.kind == BondKind::MachNx => {
                    BondPad::VccInt
                }
                "VCCXDIO0NXIO0NXIO1NXIO2" if bond.kind == BondKind::MachNx => BondPad::VccIo(0),
                "VCCNX" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VccInt),
                "VCCNXECLK" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VccEclk),
                "VCCNXAUX" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VccAux),
                "VCCNXAUXA" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VccAuxA),
                "VCCNXAUXH3" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VccAuxH(3)),
                "VCCNXAUXH4" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VccAuxH(4)),
                "VCCNXAUXH5" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VccAuxH(5)),
                "VCCNXADC18" if bond.kind == BondKind::MachNx => BondPad::Pfr(PfrPad::VccAdc18),
                _ if func.starts_with("GNDIO") => BondPad::Gnd,
                _ if func.starts_with("GNDO") => BondPad::Gnd,
                // sigh. what the fuck do you expect from a vendor.
                _ if func.starts_with("GND0") => BondPad::Gnd,
                _ if func.starts_with("J_UNUSED") && chip.kind == ChipKind::Xp2 => {
                    assert!(pin.starts_with("Unused"));
                    BondPad::Other
                }
                _ => {
                    println!("\tUNK SPEC {pin:5} {func:8} {cfg:20} {bank}");
                    continue;
                }
            }
        };
        if !pin.starts_with("Unused") {
            if pin.starts_with("ICC")
                || pin.starts_with("NX")
                || pin == "FLASHCSN"
                || pin.starts_with("WDAT")
                || pin.starts_with("RDAT")
                || pin.starts_with("WRCLK")
                || pin.starts_with("ASCCLK")
                || pin.starts_with("SCL")
                || pin.starts_with("SDA")
                || pin.starts_with("RESET")
            {
                virt_io.entry(pin).or_default().push(pad);
            } else {
                bond.pins.insert(pin, pad);
            }
        }
        if bs_order != 0 && !matches!(pad, BondPad::Pfr(_)) {
            let idx: usize = (bs_order - 1).try_into().unwrap();
            while idx >= bscan.len() {
                bscan.push(None);
            }
            assert_eq!(bscan[idx], None);
            bscan[idx] = Some((pad, bs_type));
        }
    }
    let pkg_bscan = Vec::from_iter(bscan.into_iter().rev().map(Option::unwrap));
    let bscan = chip.get_bscan();
    let mut bit = 0;
    for (mut pad, bs_type) in pkg_bscan {
        let bspad = match bs_type.as_str() {
            "I:1" | "OO:4" => {
                bit += 1;
                BScanPad::Input(bit - 1)
            }
            "O2:1" => {
                bit += 1;
                BScanPad::Output(bit - 1)
            }
            "BD:7:2" => {
                bit += 2;
                BScanPad::BiTristate(bit - 1, bit - 2)
            }
            _ => panic!("unk BS_TYPE {bs_type}"),
        };
        if pad == BondPad::Other {
            continue;
        }
        if pad == BondPad::Nc && chip.kind == ChipKind::MachXo {
            // sigh. sigh. sigh.
            let io = chip.special_io[&SpecialIoKey::SleepN];
            pad = BondPad::Io(io);
        }
        if part.name == "LFE2M70E" {
            // toolchain bug I guess? I don't believe the bscan register actually does this shit
            if let BondPad::Serdes(DirV::S, col, spad) = pad {
                let ncol = if col == chip.col_w() + 1 {
                    chip.col_e() - 27
                } else {
                    chip.col_w() + 1
                };
                pad = BondPad::Serdes(DirV::S, ncol, spad);
            }
        }
        assert_eq!(
            bscan.pads.get(&pad),
            Some(&bspad),
            "bscan mismatch for {pad}"
        );
    }
    assert_eq!(bit, bscan.bits);

    for (_key, mut pads) in virt_io {
        pads.sort();
        if let [BondPad::Io(io), BondPad::Pfr(pfr)] = *pads.as_slice() {
            bond.pfr_io.insert(pfr, io);
        }
    }

    BondResult { bond, special_io }
}
