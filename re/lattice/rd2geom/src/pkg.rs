use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use prjcombine_ecp::{
    bond::{Bond, BondPad, CfgPad, PllSet},
    chip::{Chip, ChipKind, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    dir::{DirH, DirHV},
    grid::{EdgeIoCoord, TileIobId},
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
            let is_p = part.name.starts_with("LPTM");
            if is_p {
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
    let (func, iob) = if let Some(f) = func.strip_suffix('A') {
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
    let iob = TileIobId::from_idx(iob);
    if let Some(r) = func.strip_prefix("PL")
        && let Ok(r) = r.parse()
    {
        let row = chip.xlat_row(r);
        Some(EdgeIoCoord::W(row, iob))
    } else if let Some(r) = func.strip_prefix("PR")
        && let Ok(r) = r.parse()
    {
        let row = chip.xlat_row(r);
        Some(EdgeIoCoord::E(row, iob))
    } else if let Some(c) = func.strip_prefix("PB")
        && let Ok(c) = c.parse()
    {
        let col = chip.xlat_col(c);
        Some(EdgeIoCoord::S(col, iob))
    } else if let Some(c) = func.strip_prefix("PT")
        && let Ok(c) = c.parse()
    {
        let col = chip.xlat_col(c);
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
    let fname = get_filename(datadir, part, chip);
    let archive = read_archive(&fname);
    let pkg_data = parse_pkg(&archive);
    assert!(pkg_data.pkgs.contains(&part.package));
    let mut bond = Bond {
        pins: Default::default(),
    };
    let mut special_io = BTreeMap::new();
    // println!("{} {}", part.name, part.package);
    let mut bscan = vec![];
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
        if (bs_order == 0 && pin.starts_with("Unused"))
            || pin == "VCC"
            || pin == "GND"
            || pin == "VCCAUX"
            || pin.starts_with("VCCIO")
        {
            continue;
        }
        let pin = pin.clone();
        let pad = if let Some(io) = parse_io(chip, func) {
            let mut spec_io = io;
            let mut spec = if let Some(pclk) = cfg.strip_prefix("PCLK") {
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
                Some(SpecialIoKey::Clock(io.edge(), idx.parse().unwrap()))
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
                    _ if cfg.contains("PLL") => {
                        let (which, sig) = cfg.split_once("_PLL").unwrap();
                        let which = match which {
                            "LLM0" => DirHV::SW,
                            "LUM0" => DirHV::NW,
                            "RLM0" => DirHV::SE,
                            "RUM0" => DirHV::NE,
                            _ => unreachable!(),
                        };
                        let (tc, sig) = sig.split_once('_').unwrap();
                        match tc {
                            "T" => (),
                            "C" => {
                                assert_eq!(spec_io.iob().to_idx() % 2, 1);
                                spec_io = spec_io
                                    .with_iob(TileIobId::from_idx(spec_io.iob().to_idx() - 1))
                            }
                            _ => unreachable!(),
                        }
                        match sig {
                            "IN_A" => Some(SpecialIoKey::PllIn(which)),
                            "FB_A" => Some(SpecialIoKey::PllFb(which)),
                            _ => unreachable!(),
                        }
                    }
                    "DQS" | "Unused" => None,
                    "TSALLPAD" => Some(SpecialIoKey::TsAll),
                    "GSR_PADN" => Some(SpecialIoKey::Gsr),
                    "D0" => Some(SpecialIoKey::D(0)),
                    "D1" => Some(SpecialIoKey::D(1)),
                    "D2" => Some(SpecialIoKey::D(2)),
                    "D3" => Some(SpecialIoKey::D(3)),
                    "D4" => Some(SpecialIoKey::D(4)),
                    "D5" => Some(SpecialIoKey::D(5)),
                    "D6" => Some(SpecialIoKey::D(6)),
                    "D7" => Some(SpecialIoKey::D(7)),
                    "CSN" => Some(SpecialIoKey::CsN),
                    "CS1N" => Some(SpecialIoKey::Cs1N),
                    "WRITEN" => Some(SpecialIoKey::WriteN),
                    "DI" => Some(SpecialIoKey::Di),
                    "DOUT" => Some(SpecialIoKey::Dout),
                    "DOUT,CSON" => Some(SpecialIoKey::Dout),
                    "BUSY" => Some(SpecialIoKey::Busy),
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
                    assert_eq!(prev_io, spec_io);
                }
            }
            let io_bank = chip.get_io_bank(io);
            assert_eq!(io_bank as i32, bank);
            BondPad::Io(io)
        } else if let Some(vccio_bank) = func
            .strip_prefix("VCCIO")
            .or_else(|| func.strip_prefix("VCCO"))
        {
            BondPad::VccIo(vccio_bank.parse().unwrap())
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
                "SLEEPN/NC" if chip.kind == ChipKind::MachXo => {
                    // AAAAAAAAAAAAAAAAAAAAAAAa
                    let io = chip.special_io[&SpecialIoKey::SleepN];
                    BondPad::Io(io)
                }
                "VCC" => BondPad::VccInt,
                "VCCAUX" => BondPad::VccAux,
                "VCCJ" => BondPad::VccJtag,
                "VCCPLL" => BondPad::VccPll(PllSet::All),
                "VCCP0" if chip.kind == ChipKind::Xp => BondPad::VccPll(PllSet::Side(DirH::W)),
                "VCCP1" if chip.kind == ChipKind::Xp => BondPad::VccPll(PllSet::Side(DirH::E)),
                "GNDP0" if chip.kind == ChipKind::Xp => BondPad::VccPll(PllSet::Side(DirH::W)),
                "GNDP1" if chip.kind == ChipKind::Xp => BondPad::VccPll(PllSet::Side(DirH::E)),
                "GND" => BondPad::Gnd,
                "Unused" | "NC" => BondPad::Nc,
                "XRES" => BondPad::XRes,
                "RESERVE" => BondPad::Other,
                _ if func.starts_with("GNDIO") => BondPad::Gnd,
                _ if func.starts_with("GNDO") => BondPad::Gnd,
                // sigh. what the fuck do you expect from a vendor.
                _ if func.starts_with("GND0") => BondPad::Gnd,
                _ => {
                    println!("\tPIN {pin:5} {func:8} {cfg:20} {bank}");
                    continue;
                }
            }
        };
        if !pin.starts_with("Unused") {
            bond.pins.insert(pin, pad);
        }
        if bs_order != 0 {
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
    for (pad, bs_type) in pkg_bscan {
        let bspad = match bs_type.as_str() {
            "I:1" => {
                bit += 1;
                BScanPad::Input(bit - 1)
            }
            "BD:7:2" => {
                bit += 2;
                BScanPad::BiTristate(bit - 1, bit - 2)
            }
            _ => panic!("unk BS_TYPE {bs_type}"),
        };
        match pad {
            BondPad::Io(io) => {
                assert_eq!(bscan.io.get(&io), Some(&bspad), "bscan mismatch for {io}");
            }
            BondPad::Cfg(cfg) => {
                assert_eq!(bscan.cfg.get(&cfg), Some(&bspad), "bscan mismatch for {cfg}");
            }
            BondPad::Nc if chip.kind == ChipKind::MachXo => {
                // sigh. sigh. sigh.
                let io = chip.special_io[&SpecialIoKey::SleepN];
                assert_eq!(bscan.io[&io], bspad);
            }
            _ => panic!("weird bscan pad {pad}"),
        }
    }
    assert_eq!(bit, bscan.bits);

    BondResult { bond, special_io }
}
