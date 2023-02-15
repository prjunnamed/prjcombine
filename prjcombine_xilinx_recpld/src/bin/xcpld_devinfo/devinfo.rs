use prjcombine_entity::{EntityId, EntitySet, EntityVec};
use prjcombine_ise_dump::partgen::PartgenPkg;
use prjcombine_toolchain::Toolchain;
use prjcombine_xilinx_cpld::device::{Device, DeviceKind, Io, JtagPin, Package, PkgPin};
use prjcombine_xilinx_cpld::types::{BankId, FbGroupId, FbId, FbMcId, IoId, OePadId};
use prjcombine_xilinx_recpld::v2vm6::{v2vm6, FitOpts};

use std::collections::HashMap;
use std::error::Error;
use std::fmt::Write;

use super::DevInfo;

fn get_io_pins(dev: &PartgenPkg, pin_crd: &HashMap<String, IoId>) -> (HashMap<IoId, Io>, Package) {
    let mut io = HashMap::new();
    let mut pins = HashMap::new();

    let is_bga = dev.pins.iter().any(|x| !x.pin.starts_with('P'));
    let mut banks: Vec<_> = dev.pins.iter().filter_map(|x| x.vcco_bank).collect();
    banks.sort();
    banks.dedup();
    let mut banks: EntitySet<BankId, Option<u32>> = banks.into_iter().map(Some).collect();
    let has_banks = !banks.is_empty();
    if !has_banks {
        banks.insert(None);
    }
    for p in &dev.pins {
        if let Some(ref pad) = p.pad {
            let crd = if is_bga {
                pin_crd[&p.pin]
            } else {
                pin_crd[&p.pin[1..]]
            };
            pins.insert(p.pin.clone(), PkgPin::Io(crd));
            let pad = pad.strip_prefix("PAD").unwrap();
            let pad: u32 = pad.parse().unwrap();
            let bank = banks.get(&p.vcco_bank).unwrap();
            io.insert(
                crd,
                Io {
                    pad,
                    bank,
                    jtag: match &*p.func {
                        "IO_TCK" => Some(JtagPin::Tck),
                        "IO_TMS" => Some(JtagPin::Tms),
                        "IO_TDI" => Some(JtagPin::Tdi),
                        "IO_TDO" => Some(JtagPin::Tdo),
                        _ => None,
                    },
                },
            );
        } else {
            pins.insert(
                p.pin.clone(),
                match &*p.func {
                    "NC" => PkgPin::Nc,
                    "GND" => PkgPin::Gnd,
                    "VCC" => PkgPin::VccInt,
                    "VCCINT" => PkgPin::VccInt,
                    "VCCIO" => {
                        if !has_banks {
                            PkgPin::VccIo(BankId::from_idx(0))
                        } else {
                            let bank = match (&*dev.device, &*dev.package, &*p.pin) {
                                ("xc2c32" | "xc2c64", _, _) => 1,
                                ("xc2c32a", "qfg32", "P12") => 1,
                                ("xc2c32a", "qfg32", "P27") => 2,
                                ("xc2c32a", "di44", "P18") => 1,
                                ("xc2c32a", "di44", "P37") => 2,
                                ("xc2c32a" | "xc2c64a", "pc44", "P13") => 1,
                                ("xc2c32a" | "xc2c64a", "pc44", "P32") => 2,
                                ("xc2c32a" | "xa2c32a" | "xc2c64a" | "xa2c64a", "vq44", "P7") => 1,
                                ("xc2c32a" | "xa2c32a" | "xc2c64a" | "xa2c64a", "vq44", "P26") => 2,
                                ("xc2c32a" | "xc2c64a", "cp56", "H6") => 1,
                                ("xc2c32a" | "xc2c64a", "cp56", "C6") => 2,
                                ("xc2c32a" | "xc2c64a", "cv64", "G5") => 1,
                                ("xc2c32a" | "xc2c64a", "cv64", "B5") => 2,
                                ("xc2c64a", "qfg48", "P19") => 1,
                                ("xc2c64a", "qfg48", "P42") => 2,
                                ("xc2c64a", "di81", "P34" | "P45") => 1,
                                ("xc2c64a", "di81", "P70" | "P77") => 2,
                                (
                                    "xc2c64a" | "xa2c64a" | "xc2c128" | "xa2c128" | "xc2c256"
                                    | "xa2c256",
                                    "vq100",
                                    "P20" | "P38" | "P51",
                                ) => 1,
                                (
                                    "xc2c64a" | "xa2c64a" | "xc2c128" | "xa2c128" | "xc2c256"
                                    | "xa2c256",
                                    "vq100",
                                    "P88" | "P98",
                                ) => 2,
                                ("xc2c128", "cv100", "H1" | "H5" | "H8") => 1,
                                ("xc2c128", "cv100", "B2" | "C6") => 2,
                                ("xc2c128", "di126", "P25" | "P46" | "P63" | "P82") => 1,
                                ("xc2c128", "di126", "P96" | "P113" | "P124") => 1,
                                (
                                    "xc2c128" | "xa2c128" | "xc2c256",
                                    "cp132",
                                    "J3" | "P7" | "G14" | "P13",
                                ) => 1,
                                (
                                    "xc2c128" | "xa2c128" | "xc2c256",
                                    "cp132",
                                    "A14" | "C4" | "A7",
                                ) => 2,
                                (
                                    "xc2c128" | "xc2c256" | "xa2c256",
                                    "tq144",
                                    "P27" | "P55" | "P73" | "P93",
                                ) => 1,
                                (
                                    "xc2c128" | "xc2c256" | "xa2c256",
                                    "tq144",
                                    "P109" | "P127" | "P141",
                                ) => 2,
                                (
                                    "xc2c256",
                                    "pq208",
                                    "P33" | "P59" | "P79" | "P92" | "P105" | "P132",
                                ) => 1,
                                (
                                    "xc2c256",
                                    "pq208",
                                    "P26" | "P133" | "P157" | "P172" | "P181" | "P204",
                                ) => 2,
                                (
                                    "xc2c256",
                                    "ft256",
                                    "J6" | "K6" | "L7" | "L8" | "J11" | "K11" | "L10" | "L9",
                                ) => 1,
                                (
                                    "xc2c256",
                                    "ft256",
                                    "F7" | "F8" | "G6" | "H6" | "F10" | "F9" | "H11",
                                ) => 2,
                                (
                                    "xc2c256",
                                    "di222",
                                    "P14" | "P27" | "P36" | "P63" | "P83" | "P96" | "P111" | "P139",
                                ) => 1,
                                (
                                    "xc2c256",
                                    "di222",
                                    "P140" | "P165" | "P180" | "P193" | "P216",
                                ) => 2,
                                ("xc2c384" | "xa2c384", "tq144", "P27" | "P55") => 1,
                                ("xc2c384" | "xa2c384", "tq144", "P141") => 2,
                                ("xc2c384" | "xa2c384", "tq144", "P73" | "P93") => 3,
                                ("xc2c384" | "xa2c384", "tq144", "P109" | "P127") => 4,
                                ("xc2c384" | "xc2c512", "pq208", "P33" | "P59" | "P79") => 1,
                                ("xc2c384" | "xc2c512", "pq208", "P26" | "P204") => 2,
                                ("xc2c384" | "xc2c512", "pq208", "P92" | "P105" | "P132") => 3,
                                (
                                    "xc2c384" | "xc2c512",
                                    "pq208",
                                    "P133" | "P157" | "P172" | "P181",
                                ) => 4,
                                ("xc2c384" | "xc2c512", "ft256", "J6" | "K6" | "L7" | "L8") => 1,
                                ("xc2c384" | "xc2c512", "ft256", "F7" | "F8" | "G6" | "H6") => 2,
                                ("xc2c384" | "xc2c512", "ft256", "J11" | "K11" | "L10" | "L9") => 3,
                                ("xc2c384" | "xc2c512", "ft256", "F10" | "F9" | "H11") => 4,
                                ("xc2c384" | "xc2c512", "fg324", "M9" | "N9" | "P10" | "P11") => 1,
                                ("xc2c384" | "xc2c512", "fg324", "J10" | "J11" | "K9" | "L9") => 2,
                                ("xc2c384" | "xc2c512", "fg324", "M14" | "N14" | "P12" | "P13") => {
                                    3
                                }
                                ("xc2c384" | "xc2c512", "fg324", "J12" | "J13" | "K14" | "L14") => {
                                    4
                                }
                                ("xc2c384", "di288", "P52" | "P68" | "P81" | "P108") => 1,
                                ("xc2c384", "di288", "P21" | "P39" | "P273" | "P286") => 2,
                                ("xc2c384", "di288", "P130" | "P149" | "P172" | "P183") => 3,
                                (
                                    "xc2c384",
                                    "di288",
                                    "P184" | "P209" | "P217" | "P226" | "P243" | "P254",
                                ) => 4,
                                ("xc2c512", "di324", "P55" | "P76" | "P91" | "P120") => 1,
                                ("xc2c512", "di324", "P20" | "P42" | "P299" | "P307" | "P317") => 2,
                                (
                                    "xc2c512",
                                    "di324",
                                    "P141" | "P162" | "P182" | "P194" | "P206",
                                ) => 3,
                                (
                                    "xc2c512",
                                    "di324",
                                    "P207" | "P222" | "P233" | "P257" | "P280",
                                ) => 4,
                                _ => panic!("umm bank? {}", p.pin),
                            };
                            PkgPin::VccIo(banks.get(&Some(bank)).unwrap())
                        }
                    }
                    "VAUX" => PkgPin::VccAux,
                    "VCCIO0" => PkgPin::VccIo(BankId::from_idx(0)),
                    "VCCIO1" => PkgPin::VccIo(BankId::from_idx(1)),
                    "VCCIO2" => PkgPin::VccIo(BankId::from_idx(2)),
                    "VCCIO3" => PkgPin::VccIo(BankId::from_idx(3)),
                    "PORT_EN" => PkgPin::PortEn,
                    "TCK" => PkgPin::Jtag(JtagPin::Tck),
                    "TMS" => PkgPin::Jtag(JtagPin::Tms),
                    "TDI" => PkgPin::Jtag(JtagPin::Tdi),
                    "TDO" => PkgPin::Jtag(JtagPin::Tdo),
                    _ => panic!("unknown func {}", p.func),
                },
            );
        }
    }
    for (pin, &crd) in pin_crd {
        let apin = if !is_bga {
            format!("P{pin}")
        } else {
            pin.to_string()
        };
        let pd = pins[&apin];
        if pd != PkgPin::Io(crd) {
            println!("OOPS AT {pin}");
        }
    }
    let banks = banks.into_values().collect();
    (
        io,
        Package {
            pins,
            banks,
            spec_remap: HashMap::new(),
        },
    )
}

pub fn get_devinfo(
    tc: &Toolchain,
    kind: DeviceKind,
    dev: &PartgenPkg,
    pname: &str,
) -> Result<DevInfo, Box<dyn Error>> {
    let nio = dev.pins.iter().filter(|x| x.pad.is_some()).count();
    let mut vlog = String::new();
    write!(vlog, "module top (input ")?;
    for i in 0..(nio - 1) {
        write!(vlog, "I{i}, ")?;
    }
    writeln!(vlog, "output O);")?;
    write!(vlog, "assign O = 1")?;
    for i in 0..(nio - 1) {
        write!(vlog, " & I{i}")?;
    }
    writeln!(vlog, ";")?;
    writeln!(vlog, "endmodule")?;
    let mut fopts = FitOpts::default();
    if kind == DeviceKind::Xpla3 {
        fopts.noisp = true;
    }
    if kind == DeviceKind::Xc9500 {
        fopts.localfbk = true;
    }
    let vm6 = v2vm6(tc, pname, &vlog, &fopts)?.1;
    let mut pin_crd = HashMap::new();
    for (fbid, fb) in &vm6.fbs {
        for (mcid, pin) in &fb.pins {
            if let Some(ref pad) = pin.pad {
                pin_crd.insert(pad.0.clone(), IoId::Mc((fbid, mcid)));
            }
        }
    }
    if let Some(ref ifb) = vm6.ipad_fb {
        for (ipid, pin) in &ifb.pins {
            if let Some(ref pad) = pin.pad {
                pin_crd.insert(pad.0.clone(), IoId::Ipad(ipid));
            }
        }
    }

    let fbs = vm6.fbs.len();

    let (io, mut pkg) = get_io_pins(dev, &pin_crd);

    let mut nfclk = 0;
    let mut nfoe = 0;
    let mut nfsr = 0;
    let mut has_dge = false;
    let mut cdr_pad = None;
    for (fbid, fb) in &vm6.fbs {
        for (mcid, pin) in &fb.pins {
            if let Some((_, flags)) = pin.pad {
                if (flags & 0x2000) != 0 {
                    nfclk += 1;
                }
                if (flags & 0x1000) != 0 {
                    nfoe += 1;
                }
                if (flags & 0x800) != 0 {
                    nfsr += 1;
                }
                if (flags & 0x400) != 0 {
                    has_dge = true;
                }
                if (flags & 0x200) != 0 {
                    cdr_pad = Some(IoId::Mc((fbid, mcid)));
                }
            }
        }
    }
    if let Some(ref ifb) = vm6.ipad_fb {
        for (ipid, pin) in &ifb.pins {
            if let Some((_, flags)) = pin.pad {
                if (flags & 0x2000) != 0 {
                    nfclk += 1;
                }
                if (flags & 0x1000) != 0 {
                    nfoe += 1;
                }
                if (flags & 0x800) != 0 {
                    nfsr += 1;
                }
                if (flags & 0x400) != 0 {
                    has_dge = true;
                }
                if (flags & 0x200) != 0 {
                    cdr_pad = Some(IoId::Ipad(ipid));
                }
            }
        }
    }

    let mut vlog = String::new();
    writeln!(vlog, "module top(")?;
    writeln!(vlog, "    input C, D,")?;
    for i in 0..nfclk {
        writeln!(vlog, "    input CC{i},")?;
        writeln!(vlog, "    output reg CQ{i},")?;
    }
    for i in 0..nfsr {
        writeln!(vlog, "    input RR{i},")?;
        writeln!(vlog, "    output reg RQ{i},")?;
    }
    for i in 0..nfoe {
        writeln!(vlog, "    input TT{i},")?;
        writeln!(vlog, "    output TO{i},")?;
    }
    if has_dge {
        writeln!(vlog, "    input GG,")?;
        writeln!(vlog, "    input GD,")?;
        writeln!(vlog, "    output GQ,")?;
    }
    writeln!(vlog, "    input dummy);")?;
    for i in 0..nfclk {
        writeln!(vlog, "    always @(posedge CC{i})",)?;
        writeln!(vlog, "        CQ{i} <= D;")?;
    }
    for i in 0..nfsr {
        writeln!(vlog, "    always @(posedge C, posedge RR{i})",)?;
        writeln!(vlog, "        if (RR{i})",)?;
        writeln!(vlog, "            RQ{i} <= 0;")?;
        writeln!(vlog, "        else")?;
        writeln!(vlog, "            RQ{i} <= D;")?;
    }
    for i in 0..nfoe {
        writeln!(vlog, "    assign TO{i} = TT{i} ? D : 1'bz;")?;
    }
    if has_dge {
        writeln!(vlog, "    LDG ldg(.G(GG), .D(GD), .Q(GQ));")?;
    }
    writeln!(vlog, "endmodule")?;
    let t_fclk = v2vm6(tc, pname, &vlog, &fopts)?.1;
    let mut nxlat = HashMap::new();
    for (fbid, fb) in &t_fclk.fbs {
        for (mcid, pin) in &fb.pins {
            if let Some(ib) = pin.ibuf {
                let ibuf = &t_fclk.ibufs[ib];
                for n in &ibuf.inodes {
                    let node = &t_fclk.nodes[n.node];
                    nxlat.insert(&*node.name, IoId::Mc((fbid, mcid)));
                }
                for &n in &ibuf.onodes {
                    let node = &t_fclk.nodes[n];
                    nxlat.insert(&*node.name, IoId::Mc((fbid, mcid)));
                }
            }
        }
    }
    if let Some(ref ifb) = t_fclk.ipad_fb {
        for (ipid, pin) in &ifb.pins {
            if let Some(ib) = pin.ibuf {
                let ibuf = &t_fclk.ibufs[ib];
                for n in &ibuf.inodes {
                    let node = &t_fclk.nodes[n.node];
                    nxlat.insert(&*node.name, IoId::Ipad(ipid));
                }
                for &n in &ibuf.onodes {
                    let node = &t_fclk.nodes[n];
                    nxlat.insert(&*node.name, IoId::Ipad(ipid));
                }
            }
        }
    }
    let clk_pads = t_fclk
        .global_fclk
        .into_full()
        .into_values()
        .map(|x| nxlat[&*x.name])
        .collect();
    let mut oe_pads: EntityVec<_, _> = t_fclk
        .global_foe
        .into_full()
        .into_values()
        .map(|x| nxlat[&*x.name])
        .collect();
    let sr_pad = t_fclk.global_fsr.map(|x| nxlat[&*x]);
    let dge_pad = t_fclk.dge.map(|x| nxlat[&*x]);

    let ipads = io.keys().filter(|x| matches!(x, IoId::Ipad(_))).count();

    let mut fb_groups = 0;
    let mut fb_group = EntityVec::new();
    if kind == DeviceKind::Xpla3 {
        let gs = if fbs <= 8 { 4 } else { 8 };
        fb_groups = (fbs + gs - 1) / gs;
        for i in 0..fbs {
            fb_group.push(FbGroupId::from_idx(i / gs));
        }
    }

    let oe0_9572 = IoId::Mc((FbId::from_idx(1), FbMcId::from_idx(6)));
    let oe0 = OePadId::from_idx(0);
    if kind.is_xc9500() && fbs == 4 && oe_pads[oe0] != oe0_9572 {
        pkg.spec_remap.insert(oe0_9572, oe_pads[oe0]);
        oe_pads[oe0] = oe0_9572;
    }

    Ok(DevInfo {
        dev: Device {
            kind,
            fbs,
            ipads,
            banks: pkg.banks.len(),
            fb_groups,
            fb_group,
            has_fbk: kind == DeviceKind::Xc9500 && fbs != 2,
            has_vref: kind == DeviceKind::Coolrunner2 && fbs >= 8,
            io,
            clk_pads,
            oe_pads,
            sr_pad,
            dge_pad,
            cdr_pad,
        },
        pkg,
        nds_version: vm6.nds_version,
        vm6_family: vm6.family,
        vm6_dev: vm6.dev,
        vm6_devpkg: vm6.devpkg,
        vm6_part: vm6.part,
    })
}
