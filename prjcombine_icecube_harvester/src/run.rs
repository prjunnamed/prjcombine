#![allow(clippy::type_complexity)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write;
use std::mem::ManuallyDrop;
use std::process::{Command, ExitStatus, Stdio};
use std::{fs::File, path::Path};

use bitvec::prelude::*;
use prjcombine_int::db::PinDir;
use prjcombine_siliconblue::bitstream::Bitstream;
use prjcombine_siliconblue::grid::GridKind;
use tempfile::TempDir;
use unnamed_entity::{entity_id, EntityId, EntityPartVec, EntityVec};

use crate::prims::{get_prims, Primitive, PropKind};

entity_id! {
    pub id InstId u32;
}

#[derive(Debug, Clone)]
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

    pub fn prop_bin(&mut self, prop: &str, val: &BitSlice) {
        let mut value = format!("{}'b", val.len());
        for bit in val.iter().rev() {
            if *bit {
                write!(value, "1").unwrap();
            } else {
                write!(value, "0").unwrap();
            }
        }
        self.props.insert(prop.into(), value);
    }

    pub fn prop_bin_str(&mut self, prop: &str, val: &BitSlice) {
        let mut value = "0b".to_string();
        for bit in val.iter().rev() {
            if *bit {
                write!(value, "1").unwrap();
            } else {
                write!(value, "0").unwrap();
            }
        }
        self.props.insert(prop.into(), value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstPinSource {
    Gnd,
    Vcc,
    FromInst(InstId, InstPin),
    TopPort,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstPin {
    Simple(String),
    Indexed(String, usize),
}

#[derive(Debug, Clone)]
pub struct Design {
    pub kind: GridKind,
    pub device: &'static str,
    pub package: &'static str,
    pub speed: &'static str,
    pub temp: &'static str,
    pub insts: EntityVec<InstId, Instance>,
    pub keep_tmp: bool,
    pub opts: Vec<String>,
}

#[derive(Debug)]
pub struct RunResult {
    pub pin_table: BTreeMap<String, PinTableEntry>,
    pub loc_map: EntityPartVec<InstId, LocInfo>,
    pub io_map: BTreeMap<(InstId, InstPin), IoLocInfo>,
    pub routes: BTreeMap<(InstId, InstPin), Vec<Vec<(u32, u32, String)>>>,
    pub bitstream: Bitstream,
    #[allow(dead_code)]
    pub dir: Option<TempDir>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawLoc {
    pub x: u32,
    pub y: u32,
    pub bel: u32,
}

#[derive(Debug, Clone)]
pub struct LocInfo {
    pub loc: RawLoc,
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
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub dir: Option<TempDir>,
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
                    writeln!(f, "(port (array (rename {pname} \"{pname}[{lm1}:0]\") {len}) (direction {dir}))", lm1 = len - 1)?;
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
        let inst = InstId::from_idx(
            line[1]
                .strip_prefix('i')
                .unwrap_or_else(|| panic!("umm {}", line[1]))
                .parse()
                .unwrap_or_else(|_| panic!("umm {}", line[1])),
        );
        let loc = LocInfo {
            loc: RawLoc {
                x: line[2].parse().unwrap(),
                y: line[3].parse().unwrap(),
                bel: line[4].parse().unwrap(),
            },
            is_io,
        };
        res.insert(inst, loc);
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

pub fn run(sbt: &Path, design: &Design) -> Result<RunResult, RunError> {
    let dir = ManuallyDrop::new(TempDir::with_prefix("icecube").unwrap());

    let mut f_tcl = File::create(dir.path().join("top.tcl")).unwrap();
    writeln!(f_tcl, "set sbt_root $::env(SBT_DIR)").unwrap();
    writeln!(
        f_tcl,
        "append sbt_tcl $sbt_root \"/tcl/sbt_backend_synpl.tcl\""
    )
    .unwrap();
    writeln!(f_tcl, "source $sbt_tcl").unwrap();
    let opts = design.opts.join(" ");
    writeln!(f_tcl, "set res [run_sbt_backend_auto {dev}-{speed}{pkg}{temp} top . . \":edifparser -y top.pcf :bitmap --noheader {opts}\" top]", dev = design.device, speed = design.speed, pkg = design.package, temp = design.temp).unwrap();
    writeln!(f_tcl, "exit [expr {{1 - $res}}]").unwrap();
    std::mem::drop(f_tcl);

    let mut f_pcf = File::create(dir.path().join("top.pcf")).unwrap();
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

    let mut f_scf = File::create(dir.path().join("top.scf")).unwrap();
    writeln!(f_scf, "# hi,,,").unwrap();
    std::mem::drop(f_scf);

    let mut f_edf = File::create(dir.path().join("top.edf")).unwrap();
    emit_edif(&mut f_edf, design).unwrap();
    std::mem::drop(f_edf);

    let mut cmd = Command::new("tclsh");
    cmd.env("SBT_DIR", sbt);
    cmd.env("UNBLOCK_LEDDIP_THUNDER", "1");
    cmd.env("DISABLE_IOGLITCHFIX_THUNDER", "1");
    cmd.current_dir(dir.path());
    cmd.arg("top.tcl");
    cmd.stdin(Stdio::null());
    let status = cmd.output().unwrap();
    if !status.status.success() {
        let dir = ManuallyDrop::into_inner(dir);
        Err(RunError {
            status: status.status,
            stdout: String::from_utf8_lossy(&status.stdout).to_string(),
            stderr: String::from_utf8_lossy(&status.stderr).to_string(),
            dir: if design.keep_tmp { Some(dir) } else { None },
        })
    } else {
        let pin_table =
            std::fs::read_to_string(dir.path().join("sbt/outputs/packer/top_pin_table.CSV"))
                .unwrap();
        let pin_table = parse_pin_table(&pin_table);
        let placer_pcf =
            std::fs::read_to_string(dir.path().join("sbt/outputs/placer/top_sbt.pcf")).unwrap();
        let loc_map = parse_placer_pcf(&placer_pcf);
        let io_pcf =
            std::fs::read_to_string(dir.path().join("sbt/outputs/packer/top_io_pcf.log")).unwrap();
        let io_map = parse_io_pcf(&io_pcf);
        let routes =
            std::fs::read_to_string(dir.path().join("sbt/outputs/router/top.route")).unwrap();
        let routes = parse_routes(routes);
        let bsdata = std::fs::read(dir.path().join("sbt/outputs/bitmap/top_bitmap.bin")).unwrap();
        let bitstream = Bitstream::parse(&bsdata);

        let dir = ManuallyDrop::into_inner(dir);
        Ok(RunResult {
            pin_table,
            loc_map,
            io_map,
            routes,
            bitstream,
            dir: if design.keep_tmp { Some(dir) } else { None },
        })
    }
}
