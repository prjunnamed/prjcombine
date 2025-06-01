#![allow(clippy::type_complexity)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Stdio;

use prjcombine_interconnect::db::PinDir;
use prjcombine_re_sdf::Sdf;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_siliconblue::bitstream::Bitstream;
use prjcombine_siliconblue::chip::ChipKind;
use prjcombine_types::bitvec::BitVec;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec, entity_id};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::parts::Part;
use crate::prims::{Primitive, PropKind, get_prims};

entity_id! {
    pub id InstId u32;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub kind: String,
    pub pins: BTreeMap<InstPin, InstPinSource>,
    pub props: BTreeMap<String, String>,
    pub loc: Option<RawLoc>,
    pub io: BTreeMap<InstPin, String>,
}

impl Instance {
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            pins: Default::default(),
            props: Default::default(),
            loc: None,
            io: BTreeMap::new(),
        }
    }

    pub fn top_port(&mut self, name: &str) {
        self.pins
            .insert(InstPin::Simple(name.into()), InstPinSource::TopPort);
    }

    pub fn connect(&mut self, pin: &str, src_site: InstId, src_pin: InstPin) {
        self.pins.insert(
            InstPin::Simple(pin.into()),
            InstPinSource::FromInst(src_site, src_pin),
        );
    }

    pub fn connect_idx(&mut self, pin: &str, idx: usize, src_site: InstId, src_pin: InstPin) {
        self.pins.insert(
            InstPin::Indexed(pin.into(), idx),
            InstPinSource::FromInst(src_site, src_pin),
        );
    }

    pub fn prop(&mut self, prop: &str, value: &str) {
        self.props.insert(prop.into(), value.into());
    }

    pub fn prop_bin(&mut self, prop: &str, val: &BitVec) {
        self.props.insert(prop.into(), val.to_string());
    }

    pub fn prop_bin_str(&mut self, prop: &str, val: &BitVec) {
        let mut value = "0b".to_string();
        write!(value, "{val}").unwrap();
        self.props.insert(prop.into(), value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstPinSource {
    Gnd,
    Vcc,
    FromInst(InstId, InstPin),
    TopPort,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum InstPin {
    Simple(String),
    Indexed(String, usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Design {
    pub kind: ChipKind,
    pub device: String,
    pub package: String,
    pub speed: String,
    pub temp: String,
    pub insts: EntityVec<InstId, Instance>,
    pub opts: Vec<String>,
    pub props: BTreeMap<String, String>,
}

impl Design {
    pub fn new(part: &Part, pkg: &str, speed: &str, temp: &str) -> Self {
        Self {
            kind: part.kind,
            device: part.name.to_string(),
            package: pkg.to_string(),
            speed: speed.to_string(),
            temp: temp.to_string(),
            insts: Default::default(),
            opts: vec![],
            props: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct RunResult {
    pub pin_table: BTreeMap<String, PinTableEntry>,
    pub loc_map: EntityPartVec<InstId, LocInfo>,
    pub io_map: BTreeMap<(InstId, InstPin), IoLocInfo>,
    pub routes: BTreeMap<(InstId, InstPin), Vec<Vec<(u32, u32, String)>>>,
    pub bitstream: Bitstream,
    pub dedio: BTreeSet<(InstId, InstPin)>,
    #[allow(dead_code)]
    pub sdf: Sdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RawLoc {
    pub x: u32,
    pub y: u32,
    pub bel: u32,
}

#[derive(Debug, Clone)]
pub struct LocInfo {
    pub loc: RawLoc,
    pub ds_rep0: Option<RawLoc>,
    pub ds_rep1: Option<RawLoc>,
    pub is_io: bool,
}

#[derive(Debug, Clone)]
pub struct IoLocInfo {
    pub loc: RawLoc,
    pub pin: String,
}

#[derive(Clone, Debug)]
pub struct PinTableEntry {
    pub typ: String,
    #[allow(dead_code)]
    pub bank: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct RunError {
    pub stdout: String,
    pub stderr: String,
}

fn top_port_name(inst: InstId, pin: &InstPin) -> String {
    match pin {
        InstPin::Simple(pin) => format!("port_i{inst}_{pin}_noidx"),
        InstPin::Indexed(pin, idx) => format!("port_i{inst}_{pin}_{idx}"),
    }
}

fn net_name(inst: InstId, pin: &InstPin) -> String {
    match pin {
        InstPin::Simple(pin) => format!("net_i{inst}_{pin}_noidx"),
        InstPin::Indexed(pin, idx) => format!("net_i{inst}_{pin}_{idx}"),
    }
}

fn emit_edif(mut f: impl Write, design: &Design) -> std::io::Result<()> {
    write!(
        f,
        r#"
        (edif root
            (edifVersion 2 0 0)
            (edifLevel 0)
            (keywordMap (keywordLevel 0))
            (status (written (timeStamp 1995 1 1 1 1 1) (program "xxx" (version "v1"))))
            (library PrimitivesExt
                (edifLevel 0)
                (technology (numberDefinition))
        "#
    )?;
    let prims = get_prims(design.kind);
    for (name, prim) in &prims {
        write!(
            f,
            "
            (cell {name}
                (cellType generic)
                (view meow
                    (viewType netlist)
                    (interface
            "
        )?;
        for (pname, pin) in &prim.pins {
            let dir = match pin.dir {
                PinDir::Input => "INPUT",
                PinDir::Output => "OUTPUT",
                PinDir::Inout => "INOUT",
            };
            match pin.len {
                None => {
                    writeln!(f, "(port {pname} (direction {dir}))")?;
                }
                Some(len) => {
                    writeln!(
                        f,
                        "(port (array (rename {pname} \"{pname}[{lm1}:0]\") {len}) (direction {dir}))",
                        lm1 = len - 1
                    )?;
                }
            }
        }
        write!(
            f,
            "
                    )
            "
        )?;
        for (pname, &pval) in &prim.props {
            let pval_s = match pval {
                PropKind::String(vals) => vals[0].to_string(),
                PropKind::BitvecHex(l) => {
                    assert_eq!(l % 4, 0);
                    let mut v = format!("{l}'h");
                    for _ in 0..(l / 4) {
                        write!(v, "0").unwrap();
                    }
                    v
                }
                PropKind::BitvecBin(l) => {
                    let mut v = format!("{l}'b");
                    for _ in 0..l {
                        write!(v, "0").unwrap();
                    }
                    v
                }
                PropKind::BitvecBinStr(l) => {
                    let mut v = "0b".to_string();
                    for _ in 0..l {
                        write!(v, "0").unwrap();
                    }
                    v
                }
            };
            writeln!(f, "(property {pname} (string \"{pval_s}\"))")?;
        }
        write!(
            f,
            "
                )
            )
            "
        )?;
    }

    let mut top_ports = vec![];
    let mut internal_nets: BTreeMap<_, Vec<_>> = BTreeMap::new();
    let mut gnd_pins = vec![];
    let mut vcc_pins = vec![];
    for (iid, inst) in &design.insts {
        for (pin, source) in &inst.pins {
            match source {
                InstPinSource::Gnd => {
                    gnd_pins.push((iid, pin.clone()));
                }
                InstPinSource::Vcc => {
                    vcc_pins.push((iid, pin.clone()));
                }
                InstPinSource::FromInst(inst_id, inst_pin) => {
                    internal_nets
                        .entry((*inst_id, inst_pin.clone()))
                        .or_default()
                        .push((iid, pin.clone()));
                }
                InstPinSource::TopPort => top_ports.push((iid, pin.clone())),
            }
        }
    }
    write!(
        f,
        r#"
            )
            (library work
                (edifLevel 0)
                (technology (numberDefinition))
                (cell top
                    (cellType generic)
        "#
    )?;
    for (pname, pval) in &design.props {
        writeln!(f, "(property {pname} (string \"{pval}\"))")?;
    }
    write!(
        f,
        r#"
                    (view TECH
                        (viewType netlist)
                        (interface
        "#
    )?;
    for &(iid, ref pin) in &top_ports {
        let name = top_port_name(iid, pin);
        writeln!(f, "(port {name} (direction INOUT))")?;
    }
    if top_ports.is_empty() {
        writeln!(f, "(port dummy (direction INPUT))")?;
    }
    write!(
        f,
        r#"
                        )
                        (contents
                            (instance i_gnd
                                (viewRef meow (cellRef GND (libraryref PrimitivesExt)))
                            )
                            (instance i_vcc
                                (viewRef meow (cellRef VCC (libraryref PrimitivesExt)))
                            )
        "#
    )?;
    for (iid, inst) in &design.insts {
        writeln!(f, "(instance i{iid}")?;
        writeln!(
            f,
            "(viewRef meow (cellRef {} (libraryref PrimitivesExt)))",
            inst.kind
        )?;
        for (pname, pval) in &inst.props {
            writeln!(f, "(property {pname} (string \"{pval}\"))")?;
        }
        writeln!(f, ")")?;
    }

    fn fmt_port(
        prims: &BTreeMap<&'static str, Primitive>,
        design: &Design,
        inst: InstId,
        pin: &InstPin,
    ) -> String {
        match pin {
            InstPin::Simple(pin) => {
                format!("(portRef {pin} (instanceRef i{inst}))")
            }
            InstPin::Indexed(pin, idx) => {
                let kind = &design.insts[inst].kind;
                let len = &prims[kind.as_str()].pins[pin.as_str()].len.unwrap();
                let ridx = len - 1 - idx;
                format!("(portRef (member {pin} {ridx}) (instanceRef i{inst}))")
            }
        }
    }

    writeln!(f, "(net n_gnd (joined (portRef Y (instanceRef i_gnd))")?;
    for &(iid, ref pin) in &gnd_pins {
        writeln!(f, "{}", fmt_port(&prims, design, iid, pin))?;
    }
    writeln!(f, "))")?;

    writeln!(f, "(net n_vcc (joined (portRef Y (instanceRef i_vcc))")?;
    for &(iid, ref pin) in &vcc_pins {
        writeln!(f, "{}", fmt_port(&prims, design, iid, pin))?;
    }
    writeln!(f, "))")?;

    for (&(siid, ref spin), dsts) in &internal_nets {
        let name = net_name(siid, spin);
        writeln!(f, "(net {name} (joined")?;
        writeln!(f, "{}", fmt_port(&prims, design, siid, spin))?;
        for &(diid, ref dpin) in dsts {
            writeln!(f, "{}", fmt_port(&prims, design, diid, dpin))?;
        }
        writeln!(f, "))")?;
    }

    for &(iid, ref pin) in &top_ports {
        let name = net_name(iid, pin);
        let pname = top_port_name(iid, pin);
        writeln!(f, "(net {name} (joined (portRef {pname})")?;
        writeln!(f, "{}", fmt_port(&prims, design, iid, pin))?;
        writeln!(f, "))")?;
    }

    write!(
        f,
        r#"

                        )
                    )
                )
            )
            (design TECH
                (cellRef top (libraryref work))
            )
        )"#
    )?;
    Ok(())
}

fn parse_pin_table(pin_table: &str) -> BTreeMap<String, PinTableEntry> {
    let mut res = BTreeMap::new();
    let mut lines = pin_table.lines();
    for line in &mut lines {
        if line.starts_with("----") {
            break;
        }
    }
    for line in lines {
        if line.starts_with("Total Number") {
            break;
        }
        let line = line.trim().split(", ").collect::<Vec<_>>();
        let (pin, typ, bank) = match line.len() {
            9 => (
                line[0].to_ascii_uppercase(),
                line[1].to_string(),
                line[8].to_string(),
            ),
            10 => (
                line[0].to_ascii_uppercase(),
                line[1].to_string(),
                line[9].to_string(),
            ),
            11 | 12 => (
                line[0].to_ascii_uppercase(),
                line[1].to_string(),
                line[10].to_string(),
            ),
            _ => panic!("ummmmm {line:?}"),
        };
        res.insert(pin, PinTableEntry { typ, bank });
    }
    res
}

fn parse_placer_pcf(placer_pcf: &str) -> EntityPartVec<InstId, LocInfo> {
    let mut res = EntityPartVec::new();
    let mut ds_rep_0 = EntityPartVec::new();
    let mut ds_rep_1 = EntityPartVec::new();
    for line in placer_pcf.lines() {
        // println!("{line}");
        let line = line.trim();
        let (line, comment) = line.split_once(" # ").unwrap();
        let line = Vec::from_iter(line.split_ascii_whitespace());
        let is_io = match line[0] {
            "set_location" => false,
            "set_io" => true,
            _ => panic!("ummm {line:?}?"),
        };
        assert_eq!(line.len(), 5);
        if line[1] == "GND" {
            assert_eq!(line[0], "set_location");
            assert_eq!(line[2], "-1");
            assert_eq!(line[3], "-1");
            assert_eq!(line[4], "-1");
            assert_eq!(comment, "GND");
            continue;
        }
        if line[1].starts_with("INV_") {
            assert_eq!(line[0], "set_location");
            assert_eq!(line[2], "-1");
            assert_eq!(line[3], "-1");
            assert_eq!(line[4], "-1");
            assert_eq!(comment, "INV");
            continue;
        }
        let loc = LocInfo {
            loc: RawLoc {
                x: line[2].parse().unwrap(),
                y: line[3].parse().unwrap(),
                bel: line[4].parse().unwrap(),
            },
            ds_rep0: None,
            ds_rep1: None,
            is_io,
        };
        if let Some(base) = line[1].strip_suffix("_REP_DRIVESTRENGTH_IO_0") {
            let inst = InstId::from_idx(
                base.strip_prefix('i')
                    .unwrap_or_else(|| panic!("umm {}", line[1]))
                    .parse()
                    .unwrap_or_else(|_| panic!("umm {}", line[1])),
            );
            ds_rep_0.insert(inst, loc.loc);
        } else if let Some(base) = line[1].strip_suffix("_REP_DRIVESTRENGTH_IO_1") {
            let inst = InstId::from_idx(
                base.strip_prefix('i')
                    .unwrap_or_else(|| panic!("umm {}", line[1]))
                    .parse()
                    .unwrap_or_else(|_| panic!("umm {}", line[1])),
            );
            ds_rep_1.insert(inst, loc.loc);
        } else {
            let inst = InstId::from_idx(
                line[1]
                    .strip_prefix('i')
                    .unwrap_or_else(|| panic!("umm {}", line[1]))
                    .parse()
                    .unwrap_or_else(|_| panic!("umm {}", line[1])),
            );
            res.insert(inst, loc);
        }
    }
    for (k, v) in ds_rep_0 {
        res[k].ds_rep0 = Some(v);
    }
    for (k, v) in ds_rep_1 {
        res[k].ds_rep1 = Some(v);
    }
    res
}

fn parse_io_pcf(io_pcf: &str) -> BTreeMap<(InstId, InstPin), IoLocInfo> {
    let mut res: BTreeMap<(InstId, InstPin), IoLocInfo> = BTreeMap::new();
    for line in io_pcf.lines() {
        let line = Vec::from_iter(line.trim().split_ascii_whitespace());
        if line.is_empty() {
            continue;
        }
        assert_eq!(line[0], "INOUT");
        let pname = line[1].strip_prefix("port_i").unwrap();
        let (inst, pname) = pname.split_once('_').unwrap();
        let iid = InstId::from_idx(inst.parse().unwrap());
        let pin = if let Some(pin) = pname.strip_suffix("_noidx") {
            InstPin::Simple(pin.into())
        } else {
            let (pin, idx) = pname.rsplit_once('_').unwrap();
            InstPin::Indexed(pin.into(), idx.parse().unwrap())
        };
        assert_eq!(line[2], "location");
        let loc = IoLocInfo {
            loc: RawLoc {
                x: line[3]
                    .strip_prefix('(')
                    .unwrap()
                    .strip_suffix(',')
                    .unwrap()
                    .parse()
                    .unwrap(),
                y: line[4].strip_suffix(',').unwrap().parse().unwrap(),
                bel: line[5].strip_suffix(')').unwrap().parse().unwrap(),
            },
            pin: line[6].into(),
        };
        res.insert((iid, pin), loc);
    }
    res
}

fn parse_routes(routes: String) -> BTreeMap<(InstId, InstPin), Vec<Vec<(u32, u32, String)>>> {
    let mut res = BTreeMap::new();
    let mut net = None;
    let mut skip = false;
    let mut paths = vec![];
    let mut path = vec![];
    for line in routes.lines() {
        let line = line.trim();
        if line.starts_with('*') {
            assert!(net.is_none());
        } else if let Some(name) = line.strip_prefix("Net : ") {
            if net.is_some() {
                assert!(path.is_empty());
                assert!(paths.is_empty());
            }
            if name.starts_with("bfn_") {
                skip = true;
                net = None;
                continue;
            }
            skip = false;
            let name = name.strip_prefix("net_i").unwrap();
            let (inst, name) = name.split_once('_').unwrap();
            let inst = InstId::from_idx(inst.parse().unwrap());
            let pin = if name == "O_noidx_cascade_" {
                InstPin::Simple("O__CASCADE".into())
            } else if let Some(pin) = name.strip_suffix("_noidx") {
                InstPin::Simple(pin.into())
            } else {
                let (pin, idx) = name.rsplit_once('_').unwrap();
                InstPin::Indexed(pin.into(), idx.parse().unwrap())
            };
            net = Some((inst, pin));
            continue;
        } else if line == "End" {
            if skip {
                net = None;
            } else {
                assert!(path.is_empty());
                res.insert(net.take().unwrap(), paths);
            }
            paths = vec![];
        } else if let Some(wire) = line.strip_prefix("T_") {
            if !skip {
                assert!(net.is_some());
                let (x, rest) = wire.split_once('_').unwrap();
                let (y, rest) = rest.split_once('_').unwrap();
                let x = x.parse().unwrap();
                let y = y.parse().unwrap();
                path.push((x, y, rest.to_string()));
            }
        } else if line.is_empty() {
            if net.is_some() {
                assert!(!path.is_empty());
                paths.push(path);
                path = vec![];
            }
        } else {
            panic!("umm {line}");
        }
    }
    res
}

fn parse_dedio(placer_log: String) -> BTreeSet<(InstId, InstPin)> {
    let mut res = BTreeSet::new();
    for line in placer_log.lines() {
        let line = line.trim();
        let Some(line) = line.strip_prefix("I2784: ") else {
            continue;
        };
        let Some(line) = line.strip_suffix(" is using dedicated routing") else {
            continue;
        };
        let (_inst, name) = line.split_once(" signal ").unwrap();
        let name = name.strip_prefix("net_i").unwrap();
        let (inst, name) = name.split_once('_').unwrap();
        let inst = InstId::from_idx(inst.parse().unwrap());
        let pin = if name == "O_noidx_cascade_" {
            InstPin::Simple("O__CASCADE".into())
        } else if let Some(pin) = name.strip_suffix("_noidx") {
            InstPin::Simple(pin.into())
        } else {
            let (pin, idx) = name.rsplit_once('_').unwrap();
            InstPin::Indexed(pin.into(), idx.parse().unwrap())
        };
        let net = (inst, pin);
        res.insert(net);
    }
    res
}

/*

  cache/icecube/<kind>/...

  - work
  - ok
  - fail

  normal run:
  - start in work, mkdir
  - serialize design into file
  - do the usual stuff
  - dump stdout and stderr into dir
  - failure: move to fail dir
  - ok: move to ok dir; overwrite whatever dir currently there

  cached run:
  - go to ok dir, grab serialized design
    - if found and match: use the dir
  - go to fail dir, grab serialized design
    - if found and match: use the dir
  - actually do the run

*/

fn get_result<R: std::io::Read + std::io::Seek>(zip: &mut ZipArchive<R>) -> RunResult {
    let mut read_to_string = |name| {
        let mut res = String::new();
        let mut f = zip.by_name(name).unwrap();
        f.read_to_string(&mut res).unwrap();
        res
    };

    let pin_table = read_to_string("meow_Implmnt/sbt/outputs/packer/top_pin_table.CSV");
    let pin_table = parse_pin_table(&pin_table);
    let placer_pcf = read_to_string("meow_Implmnt/sbt/outputs/placer/top_sbt.pcf");
    let loc_map = parse_placer_pcf(&placer_pcf);
    let io_pcf = read_to_string("meow_Implmnt/sbt/outputs/packer/top_io_pcf.log");
    let io_map = parse_io_pcf(&io_pcf);
    let routes = read_to_string("meow_Implmnt/sbt/outputs/router/top.route");
    let routes = parse_routes(routes);
    let sdf = read_to_string("meow_Implmnt/top_sbt.sdf");
    let sdf = Sdf::parse(&sdf);
    let placer_log = read_to_string("meow_Implmnt/sbt/outputs/placer/placer.log");
    let dedio = parse_dedio(placer_log);
    let mut bsdata = vec![];
    zip.by_name("meow_Implmnt/sbt/outputs/bitmap/top_bitmap.bin")
        .unwrap()
        .read_to_end(&mut bsdata)
        .unwrap();
    let bitstream = Bitstream::parse(&bsdata);

    RunResult {
        pin_table,
        loc_map,
        io_map,
        routes,
        bitstream,
        dedio,
        sdf,
    }
}

pub fn run(toolchain: &Toolchain, design: &Design, key: &str) -> Result<RunResult, RunError> {
    let cache_dir = PathBuf::from("cache")
        .join("icecube")
        .join(design.kind.to_string());
    let ok_path = cache_dir.join("ok").join(format!("{key}.zip"));
    if let Ok(ok_zip) = File::open(&ok_path) {
        if let Ok(mut ok_zip) = ZipArchive::new(ok_zip) {
            let mut design_file = ok_zip.by_name("design").unwrap();
            let config = bincode::config::legacy();
            let cur_design: Design =
                bincode::serde::decode_from_std_read(&mut design_file, config).unwrap();
            core::mem::drop(design_file);
            if cur_design == *design {
                return Ok(get_result(&mut ok_zip));
            }
        }
    }
    let fail_path = cache_dir.join("fail").join(format!("{key}.zip"));
    if let Ok(fail_zip) = File::open(&fail_path) {
        if let Ok(mut fail_zip) = ZipArchive::new(fail_zip) {
            let mut design_file = fail_zip.by_name("design").unwrap();
            let config = bincode::config::legacy();
            let cur_design: Design =
                bincode::serde::decode_from_std_read(&mut design_file, config).unwrap();
            core::mem::drop(design_file);
            if cur_design == *design {
                let mut stdout = String::new();
                let mut stderr = String::new();
                fail_zip
                    .by_name("stdout")
                    .unwrap()
                    .read_to_string(&mut stdout)
                    .unwrap();
                fail_zip
                    .by_name("stderr")
                    .unwrap()
                    .read_to_string(&mut stderr)
                    .unwrap();
                return Err(RunError { stdout, stderr });
            }
        }
    }

    let work_dir = cache_dir.join("work").join(key);
    let _ = std::fs::remove_dir_all(&work_dir);
    std::fs::create_dir_all(&work_dir).unwrap();

    let mut f_tcl = File::create(work_dir.join("top.tcl")).unwrap();
    writeln!(f_tcl, "set sbt_root $::env(SBT_DIR)").unwrap();
    writeln!(
        f_tcl,
        "append sbt_tcl $sbt_root \"/tcl/sbt_backend_synpl.tcl\""
    )
    .unwrap();
    writeln!(f_tcl, "source $sbt_tcl").unwrap();
    let opts = design.opts.join(" ");
    writeln!(f_tcl, "set res [run_sbt_backend_auto {dev}-{speed}{pkg}{temp} top . meow_Implmnt \":edifparser -y meow_Implmnt/top.pcf :bitmap --noheader {opts}\" top]", dev = design.device, speed = design.speed, pkg = design.package, temp = design.temp).unwrap();
    writeln!(f_tcl, "exit [expr {{1 - $res}}]").unwrap();
    std::mem::drop(f_tcl);

    let impl_dir = work_dir.join("meow_Implmnt");
    std::fs::create_dir_all(&impl_dir).unwrap();

    let mut f_pcf = File::create(impl_dir.join("top.pcf")).unwrap();
    writeln!(f_pcf, "# hi,,,").unwrap();
    for (iid, inst) in &design.insts {
        if let Some(loc) = inst.loc {
            writeln!(
                f_pcf,
                "set_location i{iid} {x} {y} {bel}",
                x = loc.x,
                y = loc.y,
                bel = loc.bel
            )
            .unwrap();
        }
        for (pin, pad) in &inst.io {
            let port = top_port_name(iid, pin);
            writeln!(f_pcf, "set_io {port} {pad}").unwrap();
        }
    }
    std::mem::drop(f_pcf);

    let mut f_scf = File::create(impl_dir.join("top.scf")).unwrap();
    writeln!(f_scf, "# hi,,,").unwrap();
    std::mem::drop(f_scf);

    let mut f_edf = File::create(impl_dir.join("top.edf")).unwrap();
    emit_edif(&mut f_edf, design).unwrap();
    std::mem::drop(f_edf);

    // HORRIBLE HACK ALERT
    //
    // In SDF, icecube emits the base delay (from the ground-truth timing database)
    // multiplied by the three derating factors (min, typ, max).  The derating factors
    // are, in turn, computed from voltage, temperature, and device-specific coefficients.
    //
    // We are interested in obtaining the base delay, and so would need to compute it backwards
    // through dividing by the derate factor.  This is perfectly feasible, but results in some
    // loss of precision.
    //
    // Instead, we use a hack: we provide nonsensical temperature and voltage data.
    // This causes the derating factor computation function to error out, and an effective factor
    // of exactly 1.0 is used, resulting in no loss of precision.
    let mut f_proj = File::create(work_dir.join("meow_sbt.project")).unwrap();
    writeln!(f_proj, "[Project]").unwrap();
    writeln!(f_proj, "CurImplementation=top_Implmnt").unwrap();
    writeln!(f_proj, "Implementations=top_Implmnt").unwrap();
    writeln!(f_proj, "[top_Implmnt]").unwrap();
    if let Some(dev) = design.device.strip_prefix("iCE65") {
        writeln!(f_proj, "DeviceFamily=iCE65").unwrap();
        writeln!(f_proj, "Device={dev}").unwrap();
    } else {
        let family = &design.device[..7];
        let dev = &design.device[7..];
        writeln!(f_proj, "DeviceFamily={family}").unwrap();
        writeln!(f_proj, "Device={dev}").unwrap();
    };
    writeln!(f_proj, "DevicePackage={pkg}", pkg = design.package).unwrap();
    writeln!(f_proj, "DevicePower={grade}", grade = design.speed).unwrap();
    writeln!(f_proj, "Devicevoltage=1337").unwrap();
    writeln!(f_proj, "DevicevoltagePerformance=+/-5%(datasheet default)").unwrap();
    writeln!(f_proj, "DeviceTemperature=-1337").unwrap();
    writeln!(f_proj, "TimingAnalysisBasedOn=Worst").unwrap();
    writeln!(f_proj, "OperationRange=Custom").unwrap();
    writeln!(f_proj, "TypicalCustomTemperature=25").unwrap();
    writeln!(f_proj, "WorstCustomTemperature=25").unwrap();
    writeln!(f_proj, "BestCustomTemperature=25").unwrap();
    writeln!(
        f_proj,
        "IOBankVoltages=topBank,3.3 bottomBank,3.3 leftBank,3.3 rightBank,3.3"
    )
    .unwrap();
    writeln!(f_proj, "derValue=0.85").unwrap();
    std::mem::drop(f_proj);

    let mut cmd = toolchain.command("timeout");
    cmd.arg("5m");
    cmd.arg("tclsh");
    cmd.env("UNBLOCK_LEDDIP_THUNDER", "1");
    cmd.env("DISABLE_IOGLITCHFIX_THUNDER", "1");
    cmd.current_dir(&work_dir);
    cmd.arg("top.tcl");
    cmd.stdin(Stdio::null());
    let status = cmd.output().unwrap();
    {
        let mut design_file = File::create(work_dir.join("design")).unwrap();
        let config = bincode::config::legacy();
        bincode::serde::encode_into_std_write(design, &mut design_file, config).unwrap();
    }
    std::fs::write(work_dir.join("stdout"), &status.stdout).unwrap();
    std::fs::write(work_dir.join("stderr"), &status.stderr).unwrap();
    let work_path = cache_dir.join("work").join(format!("{key}.zip"));
    let mut zip = ZipWriter::new(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&work_path)
            .unwrap(),
    );
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Zstd)
        .unix_permissions(0o755);
    for entry in WalkDir::new(&work_dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.strip_prefix(&work_dir).unwrap();
        if entry.metadata().unwrap().is_dir() {
            if path != work_dir {
                zip.add_directory_from_path(name, options).unwrap();
            }
        } else {
            zip.start_file_from_path(name, options).unwrap();
            zip.write_all(&std::fs::read(path).unwrap()).unwrap();
        }
    }
    let mut zip = zip.finish_into_readable().unwrap();
    let _ = std::fs::remove_dir_all(&work_dir);
    if !status.status.success() {
        std::fs::create_dir_all(cache_dir.join("fail")).unwrap();
        let _ = std::fs::remove_file(&fail_path);
        std::fs::rename(&work_path, &fail_path).unwrap();
        Err(RunError {
            stdout: String::from_utf8_lossy(&status.stdout).to_string(),
            stderr: String::from_utf8_lossy(&status.stderr).to_string(),
        })
    } else {
        let result = get_result(&mut zip);
        std::fs::create_dir_all(cache_dir.join("ok")).unwrap();
        let _ = std::fs::remove_file(&ok_path);
        std::fs::rename(&work_path, &ok_path).unwrap();
        Ok(result)
    }
}

pub fn get_cached_designs(
    kind: ChipKind,
    prefix: &str,
) -> impl ParallelIterator<Item = (String, Design, RunResult)> {
    let ok_dir = PathBuf::from("cache")
        .join("icecube")
        .join(kind.to_string())
        .join("ok");
    let mut keys = vec![];
    if let Ok(dirs) = std::fs::read_dir(&ok_dir) {
        for dir in dirs {
            let key = dir.unwrap().file_name().into_string().unwrap();
            let Some(key) = key.strip_suffix(".zip") else {
                continue;
            };
            if key.starts_with(prefix) {
                keys.push(key.to_string());
            }
        }
    }
    keys.into_par_iter().map(move |key| {
        let zip = ok_dir.join(format!("{key}.zip"));
        let mut zip = ZipArchive::new(File::open(zip).unwrap()).unwrap();
        let mut design_file = zip.by_name("design").unwrap();
        let config = bincode::config::legacy();
        let design: Design =
            bincode::serde::decode_from_std_read(&mut design_file, config).unwrap();
        core::mem::drop(design_file);
        (key, design, get_result(&mut zip))
    })
}

pub fn remove_cache_key(kind: ChipKind, key: &str) {
    let ok_dir = PathBuf::from("cache")
        .join("icecube")
        .join(kind.to_string())
        .join("ok")
        .join(format!("{key}.zip"));
    let _ = std::fs::remove_file(&ok_dir);
    let fail_dir = PathBuf::from("cache")
        .join("icecube")
        .join(kind.to_string())
        .join("fail")
        .join(format!("{key}.zip"));
    let _ = std::fs::remove_file(&fail_dir);
}
