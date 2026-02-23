use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use prjcombine_ecp::{
    bond::{AscPad, Bond, BondKind, BondPad, CfgPad, MipiPad, PfrPad, PllSet, SerdesPad},
    chip::{
        Chip, ChipKind, IoGroupKind, MachXo2Kind, PllLoc, PllPad, RowKind, SpecialIoKey,
        SpecialLocKey,
    },
};
use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    dir::{DirH, DirHV, DirV},
    grid::{ColId, DieIdExt, EdgeIoCoord, RowId, TileIobId},
};
use prjcombine_re_lattice_naming::ChipNaming;
use prjcombine_re_lattice_rawdump::Part;
use prjcombine_types::bscan::BScanPad;

use crate::{
    archive::{Archive, Reader, read_archive},
    chip::ChipExt,
};

fn get_filename(datadir: &Path, part: &Part, chip: &Chip) -> PathBuf {
    let (dir, fname) = match chip.kind {
        ChipKind::Scm => {
            let size = match chip.rows.len() {
                46 => 15,
                58 => 25,
                72 => 40,
                96 => 80,
                118 => 115,
                _ => unreachable!(),
            };
            ("or5s00", format!("or5s{size}"))
        }
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
        ChipKind::Ecp4 => {
            let size = match chip.rows.len() {
                78 => 50,
                128 => 130,
                130 => 190,
                _ => unreachable!(),
            };
            ("ep5d00", format!("ep5d{size}"))
        }
        ChipKind::Ecp5 => match part.name.as_str() {
            "LFE5U-12F" | "LAE5U-12F" => ("sa5p00b", "sa5p25b".to_string()),
            "LFE5U-25F" => ("sa5p00", "sa5p25".to_string()),
            "LFE5U-45F" => ("sa5p00", "sa5p45".to_string()),
            "LFE5U-85F" => ("sa5p00", "sa5p85".to_string()),
            "LFE5UM-25F" | "LAE5UM-25F" => ("sa5p00m", "sa5p25m".to_string()),
            "LFE5UM-45F" | "LAE5UM-45F" => ("sa5p00m", "sa5p45m".to_string()),
            "LFE5UM-85F" | "LAE5UM-85F" => ("sa5p00m", "sa5p85m".to_string()),
            "LFE5UM5G-25F" => ("sa5p00a", "sa5p25a".to_string()),
            "LFE5UM5G-45F" => ("sa5p00a", "sa5p45a".to_string()),
            "LFE5UM5G-85F" => ("sa5p00a", "sa5p85a".to_string()),
            _ => unreachable!(),
        },
        ChipKind::Crosslink => {
            if part.name.contains("MDF6000") {
                ("wi5s00", "wi5s6000".to_string())
            } else {
                ("sn5w00", "sn5w6000".to_string())
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
    let version = reader.get_u32();
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
        let _ = reader.get_zstring();
        let _ = reader.get_zstring();
        let _ = reader.get_zstring();
    }
    if version != 0x3e8 {
        let num_die = reader.get_u32();
        for _ in 0..num_die {
            let _ = reader.get_u32();
            let _ = reader.get_zstring();
            let _ = reader.get_zstring();
        }
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

fn parse_io_scm(chip: &Chip, func: &str) -> Option<EdgeIoCoord> {
    let (func, mut iob_idx) = if let Some(f) = func.strip_suffix('A') {
        (f, 0)
    } else if let Some(f) = func.strip_suffix('B') {
        (f, 1)
    } else if let Some(f) = func.strip_suffix('C') {
        (f, 2)
    } else if let Some(f) = func.strip_suffix('D') {
        (f, 3)
    } else {
        (func, 0)
    };
    if let Some(r) = func.strip_prefix("PL")
        && let Ok(r) = r.parse()
    {
        let mut row = chip.xlat_row(r);
        let mut ridx = 0;
        while chip.rows[row].io_w == IoGroupKind::None {
            row -= 1;
            ridx += 1;
        }
        match chip.rows[row].io_w {
            IoGroupKind::Quad => (),
            IoGroupKind::Octal => iob_idx += (1 - ridx) * 4,
            IoGroupKind::Dozen => iob_idx += (2 - ridx) * 4,
            _ => unreachable!(),
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::W(row, iob))
    } else if let Some(r) = func.strip_prefix("PR")
        && let Ok(r) = r.parse()
    {
        let mut row = chip.xlat_row(r);
        let mut ridx = 0;
        while chip.rows[row].io_e == IoGroupKind::None {
            row -= 1;
            ridx += 1;
        }
        match chip.rows[row].io_e {
            IoGroupKind::Quad => (),
            IoGroupKind::Octal => iob_idx += (1 - ridx) * 4,
            IoGroupKind::Dozen => iob_idx += (2 - ridx) * 4,
            _ => unreachable!(),
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::E(row, iob))
    } else if let Some(c) = func.strip_prefix("PB")
        && let Ok(c) = c.parse()
    {
        let mut col = chip.xlat_col(c);
        while chip.columns[col].io_s == IoGroupKind::None {
            col -= 1;
            iob_idx += 4;
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::S(col, iob))
    } else if let Some(c) = func.strip_prefix("PT")
        && let Ok(c) = c.parse()
    {
        let mut col = chip.xlat_col(c);
        while chip.columns[col].io_n == IoGroupKind::None {
            col -= 1;
            iob_idx += 4;
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::N(col, iob))
    } else {
        None
    }
}

fn parse_io_ecp(chip: &Chip, func: &str) -> Option<EdgeIoCoord> {
    let (func, iob_idx) = if let Some(f) = func.strip_suffix('A') {
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
    let iob = TileIobId::from_idx(iob_idx);
    if let Some(r) = func.strip_prefix("PL")
        && let Ok(r) = r.parse()
    {
        Some(EdgeIoCoord::W(chip.xlat_row(r), iob))
    } else if let Some(r) = func.strip_prefix("PR")
        && let Ok(r) = r.parse()
    {
        Some(EdgeIoCoord::E(chip.xlat_row(r), iob))
    } else if let Some(c) = func.strip_prefix("PB")
        && let Ok(c) = c.parse()
    {
        Some(EdgeIoCoord::S(chip.xlat_col(c), iob))
    } else if let Some(c) = func.strip_prefix("PT")
        && let Ok(c) = c.parse()
    {
        Some(EdgeIoCoord::N(chip.xlat_col(c), iob))
    } else {
        None
    }
}

fn parse_io_ecp3(chip: &Chip, func: &str) -> Option<EdgeIoCoord> {
    let (func, mut iob_idx) = if let Some(f) = func.strip_suffix("E_A") {
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
    } else {
        (func, 0)
    };
    if let Some(r) = func.strip_prefix("PL")
        && let Ok(r) = r.parse()
    {
        let mut row = chip.xlat_row(r);
        if iob_idx < 4 {
            if chip.rows[row].io_w == IoGroupKind::None {
                row -= 2;
            } else {
                iob_idx += 2;
            }
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::W(row, iob))
    } else if let Some(r) = func.strip_prefix("PR")
        && let Ok(r) = r.parse()
    {
        let mut row = chip.xlat_row(r);
        if iob_idx < 4 {
            if chip.rows[row].io_e == IoGroupKind::None {
                row -= 2;
            } else {
                iob_idx += 2;
            }
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::E(row, iob))
    } else if let Some(c) = func.strip_prefix("PB")
        && let Ok(c) = c.parse()
    {
        let mut col = chip.xlat_col(c);
        if iob_idx < 4 && chip.columns[col].io_s == IoGroupKind::None {
            col -= 2;
            iob_idx += 2;
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::S(col, iob))
    } else if let Some(c) = func.strip_prefix("PT")
        && let Ok(c) = c.parse()
    {
        let mut col = chip.xlat_col(c);
        if iob_idx < 4 && chip.columns[col].io_n == IoGroupKind::None {
            col -= 2;
            iob_idx += 2;
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::N(col, iob))
    } else {
        None
    }
}

fn parse_io_ecp4(chip: &Chip, func: &str) -> Option<EdgeIoCoord> {
    let (func, mut iob_idx) = if let Some(f) = func.strip_suffix("EA") {
        (f, 4)
    } else if let Some(f) = func.strip_suffix("EB") {
        (f, 5)
    } else if let Some(f) = func.strip_suffix("EC") {
        (f, 6)
    } else if let Some(f) = func.strip_suffix("ED") {
        (f, 7)
    } else {
        (func, 0)
    };
    if let Some(r) = func.strip_prefix("PL")
        && let Ok(r) = r.parse()
    {
        let mut row = chip.xlat_row(r);
        if iob_idx >= 4 {
            match (chip.rows[row].kind, row >= chip.row_clk) {
                (RowKind::Ebr, _) => (),
                (RowKind::Dsp, false) => {
                    row -= 1;
                    iob_idx -= 4;
                }
                (RowKind::Dsp, true) => {
                    iob_idx -= 4;
                    iob_idx += 2;
                }
                _ => unreachable!(),
            }
        } else {
            iob_idx += 3;
            while !matches!(
                chip.rows[row].io_w,
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadEbrDqs
            ) {
                iob_idx -= 1;
                row -= 1;
            }
            if chip.rows[row].kind == RowKind::Dsp {
                iob_idx -= 2;
            }
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::W(row, iob))
    } else if let Some(r) = func.strip_prefix("PR")
        && let Ok(r) = r.parse()
    {
        let mut row = chip.xlat_row(r);
        if iob_idx >= 4 {
            match (chip.rows[row].kind, row >= chip.row_clk) {
                (RowKind::Ebr, _) => (),
                (RowKind::Dsp, false) => {
                    row -= 1;
                    iob_idx -= 4;
                }
                (RowKind::Dsp, true) => {
                    iob_idx -= 4;
                    iob_idx += 2;
                }
                _ => unreachable!(),
            }
        } else {
            iob_idx += 3;
            while !matches!(
                chip.rows[row].io_e,
                IoGroupKind::Quad | IoGroupKind::QuadDqs | IoGroupKind::QuadEbrDqs
            ) {
                iob_idx -= 1;
                row -= 1;
            }
            if chip.rows[row].kind == RowKind::Dsp {
                iob_idx -= 2;
            }
        }
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::E(row, iob))
    } else if let Some(c) = func.strip_prefix("PT")
        && let Ok(c) = c.parse()
    {
        assert_eq!(iob_idx, 0);
        let mut col = chip.xlat_col(c);
        while chip.columns[col].io_n == IoGroupKind::None {
            iob_idx += 1;
            col -= 1;
        }
        assert!(iob_idx < 4);
        let iob = TileIobId::from_idx(iob_idx);
        Some(EdgeIoCoord::N(col, iob))
    } else {
        None
    }
}

fn parse_io_ecp5(chip: &Chip, func: &str) -> Option<EdgeIoCoord> {
    let (func, iob_idx) = if let Some(f) = func.strip_suffix('A') {
        (f, 0)
    } else if let Some(f) = func.strip_suffix('B') {
        (f, 1)
    } else if let Some(f) = func.strip_suffix('C') {
        (f, 2)
    } else if let Some(f) = func.strip_suffix('D') {
        (f, 3)
    } else {
        (func, 0)
    };
    let iob = TileIobId::from_idx(iob_idx);
    if let Some(r) = func.strip_prefix("PL")
        && let Ok(r) = r.parse()
    {
        Some(EdgeIoCoord::W(chip.xlat_row(r) - 2, iob))
    } else if let Some(r) = func.strip_prefix("PR")
        && let Ok(r) = r.parse()
    {
        Some(EdgeIoCoord::E(chip.xlat_row(r) - 2, iob))
    } else if let Some(c) = func.strip_prefix("PB")
        && let Ok(c) = c.parse()
    {
        Some(EdgeIoCoord::S(chip.xlat_col(c), iob))
    } else if let Some(c) = func.strip_prefix("PT")
        && let Ok(c) = c.parse()
    {
        Some(EdgeIoCoord::N(chip.xlat_col(c), iob))
    } else {
        None
    }
}

fn parse_io(chip: &Chip, func: &str) -> Option<EdgeIoCoord> {
    match chip.kind {
        ChipKind::Scm => parse_io_scm(chip, func),
        ChipKind::Ecp3 | ChipKind::Ecp3A => parse_io_ecp3(chip, func),
        ChipKind::Ecp4 => parse_io_ecp4(chip, func),
        ChipKind::Ecp5 => parse_io_ecp5(chip, func),
        _ => parse_io_ecp(chip, func),
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

fn uncompl_io(_chip: &Chip, io: EdgeIoCoord) -> EdgeIoCoord {
    assert_eq!(io.iob().to_idx() % 2, 1);
    io.with_iob(TileIobId::from_idx(io.iob().to_idx() - 1))
}

fn parse_io_special(
    part: &Part,
    pin: &str,
    io: EdgeIoCoord,
    bank: u32,
    chip: &Chip,
    func: &str,
) -> Option<(SpecialIoKey, bool)> {
    if let Some(pclk) = func.strip_prefix("PCLK") {
        let (which, idx) = pclk.split_once('_').unwrap();
        let (mut pclk_bank, compl): (u32, bool) = if let Some(pclk_bank) = which.strip_prefix('T') {
            (pclk_bank.parse().unwrap(), false)
        } else if let Some(pclk_bank) = which.strip_prefix('C') {
            (pclk_bank.parse().unwrap(), true)
        } else {
            unreachable!();
        };
        let mut idx = idx.parse().unwrap();
        if chip.kind == ChipKind::Scm {
            match pclk_bank {
                1 => (),
                5 => (),
                4 => idx += 8,
                6 | 3 => idx += 4,
                7 | 2 => (),
                _ => unreachable!(),
            }
        }
        if chip.kind == ChipKind::Ecp4 {
            match pclk_bank {
                0 => {
                    pclk_bank = 1;
                }
                1 => {
                    idx += 2;
                }
                2 => {
                    idx += 4;
                }
                3 => {
                    pclk_bank = 2;
                    idx += 6;
                }
                4 | 7 => {
                    idx += 2;
                }
                5 | 6 => (),
                _ => unreachable!(),
            }
        }
        if chip.kind == ChipKind::Ecp5 {
            match pclk_bank {
                0 => (),
                1 => {
                    idx += 2;
                }
                2 | 7 => {
                    idx += 2;
                }
                3 | 6 => (),
                _ => unreachable!(),
            }
        }
        if chip.kind == ChipKind::Crosslink {
            match pclk_bank {
                2 => (),
                1 => {
                    idx += 2;
                }
                0 => {
                    idx += 4;
                }
                _ => unreachable!(),
            }
        }
        if chip.kind != ChipKind::MachXo {
            assert_eq!(bank, pclk_bank);
        }
        if matches!(
            chip.kind,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 | ChipKind::Ecp3 | ChipKind::Ecp3A
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
        Some((SpecialIoKey::Clock(io.edge(), idx), compl))
    } else if let Some(vref_bank) = func.strip_prefix("VREF1_") {
        let vref_bank = vref_bank.parse().unwrap();
        assert_eq!(bank, vref_bank);
        Some((SpecialIoKey::Vref1(vref_bank), false))
    } else if let Some(vref_bank) = func.strip_prefix("VREF2_") {
        let vref_bank = vref_bank.parse().unwrap();
        assert_eq!(bank, vref_bank);
        Some((SpecialIoKey::Vref2(vref_bank), false))
    } else if (func.contains("PLL") || func.contains("DLL")) && chip.kind == ChipKind::Scm {
        let (loc, sig) = func.split_once('_').unwrap();
        let hv = match loc {
            "LLC" => DirHV::SW,
            "ULC" => DirHV::NW,
            "LRC" => DirHV::SE,
            "URC" => DirHV::NE,
            _ => panic!("weird PLL loc {loc}"),
        };
        let (pad, is_c) = match sig {
            "PLLT_IN_A" | "PLLT_FB_B" => (PllPad::PllIn0, false),
            "PLLC_IN_A" | "PLLC_FB_B" => (PllPad::PllIn0, true),
            "PLLT_IN_B" | "PLLT_FB_A" => (PllPad::PllIn1, false),
            "PLLC_IN_B" | "PLLC_FB_A" => (PllPad::PllIn1, true),
            "DLLT_IN_C" | "DLLT_FB_D" => (PllPad::DllIn0, false),
            "DLLC_IN_C" | "DLLC_FB_D" => (PllPad::DllIn0, true),
            "DLLT_IN_D" | "DLLT_FB_C" => (PllPad::DllIn1, false),
            "DLLC_IN_D" | "DLLC_FB_C" => (PllPad::DllIn1, true),
            "DLLT_IN_E" | "DLLT_FB_F" => (PllPad::DllIn2, false),
            "DLLC_IN_E" | "DLLC_FB_F" => (PllPad::DllIn2, true),
            "DLLT_IN_F" | "DLLT_FB_E" => (PllPad::DllIn3, false),
            "DLLC_IN_F" | "DLLC_FB_E" => (PllPad::DllIn3, true),
            _ => {
                println!("weird PLL pin {func}");
                return None;
            }
        };
        Some((SpecialIoKey::Pll(pad, PllLoc::new(hv, 0)), is_c))
    } else if (func.contains("PLL") || func.contains("DLL"))
        && matches!(chip.kind, ChipKind::Ecp4 | ChipKind::Ecp5)
    {
        if func.contains("MFGOUT") {
            return None;
        }
        let (loc, sig) = func.split_once('_').unwrap();
        let hv = match loc {
            "LLC" => DirHV::SW,
            "ULC" => DirHV::NW,
            "LRC" => DirHV::SE,
            "URC" => DirHV::NE,
            _ => panic!("weird PLL loc {loc}"),
        };
        let (idx, pad, is_c) = match sig {
            "GPLL0T_IN" => (0, PllPad::PllIn0, false),
            "GPLL0C_IN" => (0, PllPad::PllIn0, true),
            "GPLL0T_FB" => (0, PllPad::PllFb, false),
            "GPLL0C_FB" => (0, PllPad::PllFb, true),
            "GPLL1T_IN" if chip.kind == ChipKind::Ecp4 => (1, PllPad::PllIn0, false),
            "GPLL1C_IN" if chip.kind == ChipKind::Ecp4 => (1, PllPad::PllIn0, true),
            "GPLL1T_IN" if chip.kind == ChipKind::Ecp5 => (0, PllPad::PllIn1, false),
            "GPLL1C_IN" if chip.kind == ChipKind::Ecp5 => (0, PllPad::PllIn1, true),
            "GPLL1T_FB" => (1, PllPad::PllFb, false),
            "GPLL1C_FB" => (1, PllPad::PllFb, true),
            _ => {
                println!("weird PLL pin {func}");
                return None;
            }
        };
        Some((SpecialIoKey::Pll(pad, PllLoc::new(hv, idx)), is_c))
    } else if (func.contains("PLL") || func.contains("DLL")) && chip.kind != ChipKind::Crosslink {
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
        let mut pll_xlat = BTreeMap::new();
        for (&loc, &cell) in &chip.special_loc {
            if let SpecialLocKey::Pll(loc) = loc {
                pll_xlat.insert(cell, loc);
            }
        }

        let (loc, sig) = func.split_once('_').unwrap();
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
            ChipKind::Scm
            | ChipKind::Ecp
            | ChipKind::Xp
            | ChipKind::MachXo
            | ChipKind::Xp2
            | ChipKind::MachXo2(_)
            | ChipKind::Ecp4
            | ChipKind::Ecp5
            | ChipKind::Crosslink => PllLoc::new(hv, idx),
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Ecp3 | ChipKind::Ecp3A => {
                let cell = Chip::DIE.cell(
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
            "GPLLT_MFGOUT1" | "GPLLC_MFGOUT1" | "GPLLT_MFGOUT2" | "GPLLC_MFGOUT2" => {
                return None;
            }
            _ => panic!("weird PLL pin {sig}"),
        };
        Some((SpecialIoKey::Pll(pad, loc), is_c))
    } else if let Some(idx) = func.strip_prefix('D')
        && let Ok(idx) = idx.parse()
    {
        if chip.kind == ChipKind::MachXo {
            None
        } else {
            Some((SpecialIoKey::D(idx), false))
        }
    } else if let Some(idx) = func.strip_prefix("IO")
        && let Ok(idx) = idx.parse()
    {
        Some((SpecialIoKey::D(idx), false))
    } else if let Some(idx) = func.strip_prefix("DP")
        && let Ok(idx) = idx.parse()
    {
        Some((SpecialIoKey::DP(idx), false))
    } else if let Some(idx) = func.strip_prefix('A')
        && let Ok(idx) = idx.parse()
    {
        Some((SpecialIoKey::A(idx), false))
    } else if func.starts_with("DEBUG_BUS") || func.starts_with("GR_PCLK") {
        None
    } else {
        match func {
            "DQS" | "Unused" => None,
            "TSALLPAD" => Some((SpecialIoKey::TsAll, false)),
            "GSR_PADN" => Some((SpecialIoKey::Gsr, false)),
            "CSN" => Some((SpecialIoKey::CsN, false)),
            "CS1N" => Some((SpecialIoKey::Cs1N, false)),
            "WRITEN" => Some((SpecialIoKey::WriteN, false)),
            "DI" => Some((SpecialIoKey::Di, false)),
            "BUSY" if !matches!(chip.kind, ChipKind::Ecp4 | ChipKind::Ecp5) => {
                Some((SpecialIoKey::Busy, false))
            }
            "DOUT" | "DOUT,CSON" | "DOUT_CSON" | "CSON" => Some((SpecialIoKey::Dout, false)),
            "SPIFASTN" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::D(0), false))
            }
            "SI" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::D(3), false))
            }
            "SO" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::D(4), false))
            }
            "SPID1" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::D(6), false))
            }
            "SPID0" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::D(7), false))
            }
            "SN" | "CONT1N" | "OEN" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::CsN, false))
            }
            "HOLDN" | "CONT2N" | "RDY" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::Cs1N, false))
            }
            "CSSPI1N" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::Dout, false))
            }
            "CSSPI0N" | "CEN" | "CSSPIN"
                if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) =>
            {
                Some((SpecialIoKey::Di, false))
            }
            "SISPI" | "AVDN" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::Busy, false))
            }
            "MCLK" if matches!(chip.kind, ChipKind::Ecp3 | ChipKind::Ecp3A) => {
                Some((SpecialIoKey::MClk, false))
            }
            "MCLK" if matches!(chip.kind, ChipKind::MachXo2(_)) => {
                Some((SpecialIoKey::Cclk, false))
            }
            // SCM
            "RDN" => Some((SpecialIoKey::ReadN, false)),
            "WRN" => Some((SpecialIoKey::WriteN, false)),
            "MPICLK" => Some((SpecialIoKey::MpiClk, false)),
            "QOUT" => Some((SpecialIoKey::Qout, false)),
            "CS1" => Some((SpecialIoKey::Cs1, false)),
            "CS0N" => Some((SpecialIoKey::CsN, false)),
            "RDY" => Some((SpecialIoKey::Busy, false)),
            "MPIACKN" => Some((SpecialIoKey::MpiAckN, false)),
            "MPIRTRYN" => Some((SpecialIoKey::MpiRetryN, false)),
            "MPITEAN" => Some((SpecialIoKey::MpiTeaN, false)),
            "LDCN" => Some((SpecialIoKey::Ldc, false)),
            "HDC" => Some((SpecialIoKey::Hdc, false)),
            "EXTDONEI" => Some((SpecialIoKey::ExtDoneI, false)),
            "EXTDONEO" => Some((SpecialIoKey::ExtDoneO, false)),
            "EXTCLKP1I" => Some((SpecialIoKey::ExtClkI(1), false)),
            "EXTCLKP2I" => Some((SpecialIoKey::ExtClkI(2), false)),
            "EXTCLKP1O" => Some((SpecialIoKey::ExtClkO(1), false)),
            "EXTCLKP2O" => Some((SpecialIoKey::ExtClkO(2), false)),
            "DIFFR_2" => Some((SpecialIoKey::DiffR(2), false)),
            "DIFFR_3" => Some((SpecialIoKey::DiffR(3), false)),
            "DIFFR_6" => Some((SpecialIoKey::DiffR(6), false)),
            "DIFFR_7" => Some((SpecialIoKey::DiffR(7), false)),
            "TESTCFGN" => None,
            // XP2 stuff
            "INITN" => Some((SpecialIoKey::InitB, false)),
            "SISPI" | "SI" if chip.kind != ChipKind::Ecp3 => Some((SpecialIoKey::D(0), false)),
            "SPISO" | "SO" if chip.kind != ChipKind::Ecp3 => Some((SpecialIoKey::D(1), false)),
            "CCLK" => Some((SpecialIoKey::Cclk, false)),
            "CSSPIN" if matches!(chip.kind, ChipKind::Xp2 | ChipKind::MachXo2(_)) => {
                Some((SpecialIoKey::SpiCCsB, false))
            }
            "CSSPISN" => Some((SpecialIoKey::SpiPCsB, false)),
            "SN" if matches!(chip.kind, ChipKind::MachXo2(_)) => {
                Some((SpecialIoKey::SpiPCsB, false))
            }
            "CFG1" => Some((SpecialIoKey::M1, false)),
            "DONE" => Some((SpecialIoKey::Done, false)),
            "PROGRAMN" => Some((SpecialIoKey::ProgB, false)),
            "MFG_EXT_CLK" | "FL_EXT_PULSE_D" | "FL_EXT_PULSE_G" => None,
            // MachXO2
            "MD0" if matches!(chip.kind, ChipKind::MachXo2(_)) => Some((SpecialIoKey::D(0), false)),
            "MD1" if matches!(chip.kind, ChipKind::MachXo2(_)) => Some((SpecialIoKey::D(1), false)),
            "MD2" | "SCL" if matches!(chip.kind, ChipKind::MachXo2(_)) => {
                Some((SpecialIoKey::D(2), false))
            }
            "MD3" | "SDA" if matches!(chip.kind, ChipKind::MachXo2(_)) => {
                Some((SpecialIoKey::D(3), false))
            }
            "MD4" if matches!(chip.kind, ChipKind::MachXo2(_)) => None,
            "MD5" if matches!(chip.kind, ChipKind::MachXo2(_)) => None,
            "MD6" if matches!(chip.kind, ChipKind::MachXo2(_)) => None,
            "MD7" if matches!(chip.kind, ChipKind::MachXo2(_)) => None,
            "TEST_CLK" if matches!(chip.kind, ChipKind::MachXo2(_)) => None,
            "TCK" => Some((SpecialIoKey::Tck, false)),
            "TMS" => Some((SpecialIoKey::Tms, false)),
            "TDI" => Some((SpecialIoKey::Tdi, false)),
            "TDO" => Some((SpecialIoKey::Tdo, false)),
            "JTAGENB" => Some((SpecialIoKey::JtagEn, false)),
            "INTEST_OVER" => None,
            // ECP4, ECP5
            "SISPI1" | "MOSI" => Some((SpecialIoKey::D(0), false)),
            "SPISO1" | "MISO" => Some((SpecialIoKey::D(1), false)),
            "SISPI2" | "MOSI2" => Some((SpecialIoKey::D(4), false)),
            "SPISO2" | "MISO2" => Some((SpecialIoKey::D(5), false)),
            "SDA" if chip.kind == ChipKind::Ecp4 => Some((SpecialIoKey::Cs1N, false)),
            "SCL" if chip.kind == ChipKind::Ecp4 => Some((SpecialIoKey::WriteN, false)),
            "SN" if matches!(chip.kind, ChipKind::Ecp4 | ChipKind::Ecp5) => {
                Some((SpecialIoKey::CsN, false))
            }
            "HOLDN" | "BUSY" | "CSSPIN" | "CEN"
                if matches!(chip.kind, ChipKind::Ecp4 | ChipKind::Ecp5) =>
            {
                Some((SpecialIoKey::Di, false))
            }
            "TDI0" | "TDI1" | "TDI2" | "TDI3" | "TDI4" | "TDI5" | "TDI6" | "TDI7" | "TDO0"
            | "TDO1" | "TDO2" | "TDO3" | "TDO4" | "TDO5" | "TDO6" | "TDO7"
                if chip.kind == ChipKind::Ecp4 =>
            {
                None
            }
            "ATB_FORCE" | "ATB_SENSE" | "SCAN_SHFT_ENB" | "SCAN_SHFT_EN" | "ATB_SENSE_1"
            | "ATB_SENSE_2" | "ATB_FORCE_1" | "ATB_FORCE_2" => None,
            "TDIB" | "TDOB" | "TDOT" | "TDIL" | "TDIR" | "TDOR" => None,
            "S0_IN" | "S1_IN" | "S2_IN" | "S3_IN" | "S4_IN" | "S5_IN" | "S6_IN" | "S7_IN"
            | "S0_OUT" | "S1_OUT" | "S2_OUT" | "S3_OUT" | "S4_OUT" | "S5_OUT" | "S6_OUT"
            | "S7_OUT"
                if chip.kind == ChipKind::Ecp5 =>
            {
                None
            }
            // Crosslink
            "MOSI_D0" => Some((SpecialIoKey::D(0), false)),
            "MISO_D1" => Some((SpecialIoKey::D(1), false)),
            "PMU_WKUPN" => Some((SpecialIoKey::PmuWakeupN, false)),
            "SPI_SS" | "SCL" if chip.kind == ChipKind::Crosslink => {
                Some((SpecialIoKey::CsN, false))
            }
            "SPI_SCK" | "MCK" | "SDA" if chip.kind == ChipKind::Crosslink => {
                Some((SpecialIoKey::Cclk, false))
            }
            "MIPI_CLKT2_0" => Some((SpecialIoKey::MipiClk(DirH::W), false)),
            "MIPI_CLKT1_0" => Some((SpecialIoKey::MipiClk(DirH::E), false)),
            "MIPI_CLKC2_0" => Some((SpecialIoKey::MipiClk(DirH::W), true)),
            "MIPI_CLKC1_0" => Some((SpecialIoKey::MipiClk(DirH::E), true)),
            "GPLL_MFGOUT_1" | "GPLL_MFGOUT_2" if chip.kind == ChipKind::Crosslink => None,
            "GPLLT2_0" if chip.kind == ChipKind::Crosslink => Some((
                SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::SE, 0)),
                false,
            )),
            "GPLLC2_0" if chip.kind == ChipKind::Crosslink => Some((
                SpecialIoKey::Pll(PllPad::PllIn0, PllLoc::new(DirHV::SE, 0)),
                true,
            )),
            "USER_SDA" => Some((SpecialIoKey::D(2), false)),
            "USER_SCL" => Some((SpecialIoKey::D(3), false)),
            _ => {
                println!(
                    "\t{name}: unknown spec {pin:5} {func} {bank} {io}",
                    name = part.name
                );
                None
            }
        }
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
    let mut virt_io: BTreeMap<String, Vec<BondPad>> = BTreeMap::new();
    let mut serdes_xlat = BTreeMap::new();
    for (col, cd) in &chip.columns {
        if cd.io_s == IoGroupKind::Serdes {
            serdes_xlat.insert(cd.bank_s.unwrap(), col);
        }
        if cd.io_n == IoGroupKind::Serdes {
            serdes_xlat.insert(cd.bank_n.unwrap(), col);
        }
    }
    let mut mipi_xlat = BTreeMap::new();
    for (col, cd) in &chip.columns {
        if cd.io_n == IoGroupKind::Mipi {
            mipi_xlat.insert(cd.bank_n.unwrap(), col);
        }
    }
    let mut cdone_shared = None;
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
        let mut func = func;
        if func == "UNUSED" && cfg.contains("RXREFCLK") && chip.kind == ChipKind::Scm {
            func = cfg;
        }
        if (bs_order == 0 || bs_type == "L")
            && (pin.starts_with("Unused")
                || pin == "UNUSED"
                || pin == "GND"
                || pin == "GNDA"
                || pin == "DB"
                || pin.starts_with("VCC")
                || pin.starts_with("VDD")
                || pin.starts_with("VTT")
                || pin.starts_with("VSS"))
        {
            continue;
        }
        if pin.ends_with("PAD") && func.starts_with("DU_") {
            continue;
        }
        if let Some(suffix) = func.strip_prefix("NC_")
            && pin == suffix
        {
            continue;
        }
        if func == "CDONE"
            && let Some(pin) = pin.strip_suffix("_CDONE")
            && let Some(io) = parse_io(chip, pin)
        {
            cdone_shared = Some(io);
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
        let pin = pin.strip_suffix("_DB").unwrap_or(pin);
        let pin = pin.to_string();
        if let Some(n) = func.strip_suffix("_DB")
            && n == pin
        {
            continue;
        }
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
            for func in cfg.split('/') {
                let Some((spec, uncompl)) =
                    parse_io_special(part, &pin, io, bank as u32, chip, func)
                else {
                    continue;
                };
                let spec_io = if uncompl { uncompl_io(chip, io) } else { io };
                // well.
                if let Some(prev_io) = special_io.insert(spec, spec_io) {
                    assert_eq!(prev_io, spec_io, "fail on {spec}");
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
        } else if chip.kind == ChipKind::Scm
            && (func.starts_with("A_")
                || func.starts_with("B_")
                || func.starts_with("C_")
                || func.starts_with("D_"))
            && (func.ends_with("_L") || func.ends_with("_R"))
        {
            let (which, func) = func.split_once('_').unwrap();
            let (pad, lr) = func.rsplit_once('_').unwrap();
            let exp_bank = match (which, lr) {
                ("A", "L") => 9,
                ("B", "L") => 11,
                ("C", "L") => 13,
                ("D", "L") => 15,
                ("A", "R") => 10,
                ("B", "R") => 12,
                ("C", "R") => 14,
                ("D", "R") => 16,
                _ => unreachable!(),
            };
            let col = serdes_xlat[&exp_bank];
            let pad = if let Ok(channel) = pad[pad.len() - 1..].parse()
                && pad != "VDDAX25"
                && pad != "VDDRX01"
                && pad != "VDDRX23"
            {
                match &pad[..pad.len() - 1] {
                    "HDINP" => SerdesPad::InP(channel),
                    "HDINN" => SerdesPad::InN(channel),
                    "HDOUTP" => SerdesPad::OutP(channel),
                    "HDOUTN" => SerdesPad::OutN(channel),
                    "VDDRX" => SerdesPad::VccRx(channel),
                    "VDDIB" => SerdesPad::VccIB(channel),
                    "VDDOB" => SerdesPad::VccOB(channel),
                    _ => panic!("umm {pad}"),
                }
            } else {
                match pad {
                    "REFCLKP" => SerdesPad::ClkP,
                    "REFCLKN" => SerdesPad::ClkN,
                    "RXREFCLKP" => SerdesPad::RxClkP,
                    "RXREFCLKN" => SerdesPad::RxClkN,
                    "VDDAX25" => SerdesPad::VccAux25,
                    "VDDRX01" => SerdesPad::VccRxPair(0),
                    "VDDRX23" => SerdesPad::VccRxPair(2),
                    _ => panic!("umm {pad}"),
                }
            };
            if !matches!(pad, SerdesPad::VccRxPair(_)) {
                assert_eq!(bank, exp_bank as i32);
            }
            BondPad::Serdes(DirV::N, col, pad)
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
        } else if func.starts_with("HD") && chip.kind == ChipKind::Ecp4 {
            let (sig, idx) = func.split_once("_Q").unwrap();
            let (quad, ch) = idx.split_once("CH").unwrap();
            let quad: usize = quad.parse().unwrap();
            let ch = ch.parse().unwrap();
            let pad = match sig {
                "HDTXP" => SerdesPad::OutP(ch),
                "HDTXN" => SerdesPad::OutN(ch),
                "HDRXP" => SerdesPad::InP(ch),
                "HDRXN" => SerdesPad::InN(ch),
                _ => panic!("umm {sig}"),
            };
            let cell = chip
                .special_loc
                .get(&SpecialLocKey::SerdesSingle)
                .or_else(|| chip.special_loc.get(&SpecialLocKey::SerdesDouble))
                .or_else(|| chip.special_loc.get(&SpecialLocKey::SerdesTriple))
                .copied()
                .unwrap();
            BondPad::Serdes(DirV::S, cell.col + quad, pad)
        } else if func.starts_with("REFCLK") && func.contains('Q') && chip.kind == ChipKind::Ecp4 {
            let (sig, quad) = func.split_once("_Q").unwrap();
            let quad: usize = quad.parse().unwrap();
            let pad = match sig {
                "REFCLKP" => SerdesPad::ClkP,
                "REFCLKN" => SerdesPad::ClkN,
                _ => panic!("umm {sig}"),
            };
            let cell = chip
                .special_loc
                .get(&SpecialLocKey::SerdesSingle)
                .or_else(|| chip.special_loc.get(&SpecialLocKey::SerdesDouble))
                .or_else(|| chip.special_loc.get(&SpecialLocKey::SerdesTriple))
                .copied()
                .unwrap();
            BondPad::Serdes(DirV::S, cell.col + quad, pad)
        } else if (func.starts_with("HD") || func.starts_with("VCCH"))
            && func.contains("_D")
            && chip.kind == ChipKind::Ecp5
        {
            let (sig, idx) = func.split_once("_D").unwrap();
            let (dual, ch) = idx.split_once("CH").unwrap();
            let dual: usize = dual.parse().unwrap();
            let exp_bank = match dual {
                0 => 50,
                1 => 51,
                _ => unreachable!(),
            };
            let col = serdes_xlat[&exp_bank];
            if sig.starts_with("HD") {
                assert_eq!(bank, exp_bank as i32);
            }
            let ch = ch.parse().unwrap();
            let pad = match sig {
                "HDTXP0" => SerdesPad::OutP(ch),
                "HDTXN0" => SerdesPad::OutN(ch),
                "HDRXP0" => SerdesPad::InP(ch),
                "HDRXN0" => SerdesPad::InN(ch),
                "VCCHRX0" if ch == 0 => SerdesPad::VccRx(0),
                "VCCHRX1" if ch == 1 => SerdesPad::VccRx(1),
                "VCCHTX0" if ch == 0 => SerdesPad::VccTx(0),
                "VCCHTX1" if ch == 1 => SerdesPad::VccTx(1),
                _ => panic!("umm {sig}"),
            };
            BondPad::Serdes(DirV::S, col, pad)
        } else if (func.ends_with("_D0") || func.ends_with("_D1")) && chip.kind == ChipKind::Ecp5 {
            let (sig, dual) = func.split_once("_D").unwrap();
            let dual: usize = dual.parse().unwrap();
            let exp_bank = match dual {
                0 => 50,
                1 => 51,
                _ => unreachable!(),
            };
            let col = serdes_xlat[&exp_bank];
            if sig.starts_with("REFCLK") {
                assert_eq!(bank, exp_bank as i32);
            }
            let pad = match sig {
                "REFCLKP" => SerdesPad::ClkP,
                "REFCLKN" => SerdesPad::ClkN,
                "ATSTP" => SerdesPad::AuxTstPadOutP,
                "ATSTN" => SerdesPad::AuxTstPadOutN,
                _ => panic!("umm {sig}"),
            };
            BondPad::Serdes(DirV::S, col, pad)
        } else if let Some(suffix) = func.strip_prefix("DPHY") {
            let (idx, sig) = suffix.split_once('_').unwrap();
            let idx: u32 = idx.parse().unwrap();
            let exp_bank = 60 + idx;
            let col = mipi_xlat[&exp_bank];
            assert_eq!(bank, exp_bank as i32);
            let pad = match sig {
                "DP0" => MipiPad::DataP(0),
                "DN0" => MipiPad::DataN(0),
                "DP1" => MipiPad::DataP(1),
                "DN1" => MipiPad::DataN(1),
                "DP2" => MipiPad::DataP(2),
                "DN2" => MipiPad::DataN(2),
                "DP3" => MipiPad::DataP(3),
                "DN3" => MipiPad::DataN(3),
                "CKP" => MipiPad::ClkP,
                "CKN" => MipiPad::ClkN,
                _ => panic!("umm {sig}"),
            };
            BondPad::Mipi(col, pad)
        } else if let Some((sig, which)) = func.split_once("_DPHY") {
            let pad = match sig {
                "VCCA" => MipiPad::Vcc,
                "VSSA" => MipiPad::Gnd,
                "VCCPLL" => MipiPad::VccPll,
                "VSSPLL" => MipiPad::GndPll,
                "VCCMU" => MipiPad::VccMu,
                "VSSMU" => MipiPad::GndMu,
                _ => panic!("umm {sig}"),
            };
            if which == "X" {
                BondPad::MipiCommon(pad)
            } else {
                let idx: u32 = which.parse().unwrap();
                let exp_bank = 60 + idx;
                let col = mipi_xlat[&exp_bank];
                BondPad::Mipi(col, pad)
            }
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
        } else if let Some(vtt_bank) = func.strip_prefix("VTT_") {
            BondPad::Vtt(vtt_bank.parse().unwrap())
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
                "M0" | "CFG0" | "CFGMDN0" | "CFG_0" => BondPad::Cfg(CfgPad::M0),
                "M1" | "CFG1" | "CFGMDN1" | "CFG_1" => BondPad::Cfg(CfgPad::M1),
                "M2" | "CFG2" | "CFGMDN2" | "CFG_2" => BondPad::Cfg(CfgPad::M2),
                "M3" => BondPad::Cfg(CfgPad::M3),
                "SLEEPN/TOE" => BondPad::Cfg(CfgPad::SleepB),
                "TOE" => BondPad::Cfg(CfgPad::Toe),
                // SCM special.
                "RESETN" => BondPad::Cfg(CfgPad::ResetB),
                "RDCFGN" => BondPad::Cfg(CfgPad::RdCfgB),
                "MPIIRQN" => BondPad::Cfg(CfgPad::MpiIrqB),
                "VDDAX25_L" if chip.kind == ChipKind::Scm => {
                    BondPad::Serdes(DirV::N, serdes_xlat[&9], SerdesPad::VccAux25)
                }
                "VDDAX25_R" if chip.kind == ChipKind::Scm => {
                    BondPad::Serdes(DirV::N, serdes_xlat[&10], SerdesPad::VccAux25)
                }
                "RESP_ULC" if chip.kind == ChipKind::Scm => {
                    BondPad::Serdes(DirV::N, serdes_xlat[&9], SerdesPad::ResP)
                }
                "RESP_URC" if chip.kind == ChipKind::Scm => {
                    BondPad::Serdes(DirV::N, serdes_xlat[&10], SerdesPad::ResP)
                }
                "RESPN_ULC" if chip.kind == ChipKind::Scm => {
                    BondPad::Serdes(DirV::N, serdes_xlat[&9], SerdesPad::ResPN)
                }
                "RESPN_URC" if chip.kind == ChipKind::Scm => {
                    BondPad::Serdes(DirV::N, serdes_xlat[&10], SerdesPad::ResPN)
                }
                "TEMP" => BondPad::TempSense,
                "PROBE_VCC" => BondPad::ProbeVcc,
                "PROBE_GND" => BondPad::ProbeGnd,
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
                // Crosslink
                "CDONE" => BondPad::Cfg(CfgPad::Done),
                "CRESET_B" => BondPad::Cfg(CfgPad::ProgB),
                "VCC" => BondPad::VccInt,
                "VCCAUX" | "VCCAUX1" | "VCCAUX2" | "VCCAUX3" | "VCCAUX4" | "VCCAUX5"
                | "VCCAUX6" | "VCCAUX7" | "VCCAUX23" | "VCCAUX67" => BondPad::VccAux,
                "VCCAUXA" => BondPad::VccAuxA,
                "VCCJ" => BondPad::VccJtag,
                "VCC12" => BondPad::Vcc12,
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
                "VCCGPLL" => BondPad::VccPll(PllSet::All),
                "VSSGPLL" => BondPad::GndPll(PllSet::All),
                "VCCA" => BondPad::VccA,
                "GND" | "GNDAUX" => BondPad::Gnd,
                "GNDA" => BondPad::GndA,
                "VSS" => BondPad::Gnd,
                "Unused" | "UNUSED" | "NC" => BondPad::Nc,
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
                "REFCLKP_CORNER" => BondPad::SerdesCorner(SerdesPad::ClkP),
                "REFCLKN_CORNER" => BondPad::SerdesCorner(SerdesPad::ClkN),
                "RXANTOUTP" => BondPad::SerdesCorner(SerdesPad::RxantOutP),
                "RXANTOUTN" => BondPad::SerdesCorner(SerdesPad::RxantOutN),
                "AUXTSTPADOUTP" => BondPad::SerdesCorner(SerdesPad::AuxTstPadOutP),
                "AUXTSTPADOUTN" => BondPad::SerdesCorner(SerdesPad::AuxTstPadOutN),
                "VCCA0" if chip.kind == ChipKind::Ecp5 => {
                    BondPad::Serdes(DirV::S, serdes_xlat[&50], SerdesPad::VccA)
                }
                "VCCA1" if chip.kind == ChipKind::Ecp5 => {
                    BondPad::Serdes(DirV::S, serdes_xlat[&51], SerdesPad::VccA)
                }
                "VCCAUXA0" if chip.kind == ChipKind::Ecp5 => {
                    BondPad::Serdes(DirV::S, serdes_xlat[&50], SerdesPad::VccAuxA)
                }
                "VCCAUXA1" if chip.kind == ChipKind::Ecp5 => {
                    BondPad::Serdes(DirV::S, serdes_xlat[&51], SerdesPad::VccAuxA)
                }
                "VCCHRX" if chip.kind == ChipKind::Ecp5 => {
                    BondPad::Serdes(DirV::S, serdes_xlat[&50], SerdesPad::VccTxCommon)
                }
                "VCCAUX25VPP" => BondPad::Vpp,
                _ if func.starts_with("GNDIO") => BondPad::Gnd,
                _ if func.starts_with("VSSIO") => BondPad::Gnd,
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
        if !pin.starts_with("Unused")
            && pin != "GND"
            && pin != "GNDA"
            && pin != "VCCA"
            && pin != "VSS"
        {
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
    // for (i, x) in bscan.iter().enumerate() {
    //     if let &Some((x, kind)) = x {
    //         println!("{i}: {x} {kind}");
    //     } else {
    //         println!("{i}: ----");
    //     }
    // }
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
            "BD:7:1" | "BD:7:2" | "BD:7-0:1" => {
                bit += 2;
                BScanPad::BiTristate(bit - 1, bit - 2)
            }
            "O3:1:1" => {
                bit += 2;
                BScanPad::OutputTristate(bit - 1, bit - 2)
            }
            "BDC:1" => {
                bit += 1;
                if matches!(pad, BondPad::Serdes(_, _, SerdesPad::OutP(_))) {
                    BScanPad::Output(bit - 1)
                } else {
                    BScanPad::Input(bit - 1)
                }
            }
            _ => panic!("unk BS_TYPE {bs_type}"),
        };
        if matches!(pad, BondPad::Other | BondPad::Nc) {
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

    if let Some(cdone_shared) = cdone_shared {
        for pad in bond.pins.values_mut() {
            if *pad == BondPad::Io(cdone_shared) {
                *pad = BondPad::IoCdone(cdone_shared);
            }
        }
    }

    BondResult { bond, special_io }
}
