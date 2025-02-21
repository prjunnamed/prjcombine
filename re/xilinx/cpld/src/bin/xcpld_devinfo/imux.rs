use std::collections::{hash_map::Entry, HashMap};
use std::error::Error;
use std::fmt::Write;
use std::thread::available_parallelism;

use itertools::Itertools;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_types::IoId;
use prjcombine_re_xilinx_cpld::vm6::{FbImux, NodeKind};
use prjcombine_re_xilinx_cpld::device::{Device, DeviceKind, Package, PkgPin};
use prjcombine_re_xilinx_cpld::types::ImuxInput;
use prjcombine_re_xilinx_cpld::{
    db::ImuxData,
    v2vm6::{v2vm6, FitOpts},
};
use rand::prelude::*;
use rayon::prelude::*;
use unnamed_entity::{EntityId, EntityVec};

fn gather_imux_once(
    tc: &Toolchain,
    part: &str,
    dev: &Device,
    pkg: &Package,
) -> Result<Option<ImuxData>, Box<dyn Error>> {
    let mut rng = rand::rng();
    let sz = match dev.kind {
        DeviceKind::Xc9500 => 34,
        DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv => 48,
        DeviceKind::Xpla3 | DeviceKind::Coolrunner2 => match dev.fbs {
            2 | 4 => 39,
            8 => 38,
            16 => 37,
            24 => 36,
            32 => 36,
            _ => unreachable!(),
        },
    };
    let mut imux_sets = EntityVec::new();
    let mut olocs = EntityVec::new();
    for fb in dev.fbs() {
        let mut cands = vec![];
        for &pad in dev.io.keys() {
            cands.push(ImuxInput::Ibuf(pad));
        }
        if dev.kind == DeviceKind::Xc9500 {
            if dev.has_fbk {
                for mc in dev.fb_mcs() {
                    cands.push(ImuxInput::Mc((fb, mc)));
                }
            }
        } else {
            for ofb in dev.fbs() {
                for omc in dev.fb_mcs() {
                    cands.push(ImuxInput::Mc((ofb, omc)));
                }
            }
        }
        if dev.kind == DeviceKind::Xpla3 {
            cands.push(ImuxInput::Pup);
        }
        let imux_set: Vec<_> = cands.choose_multiple(&mut rng, sz).copied().collect();
        imux_sets.push(imux_set);
        let cands: Vec<_> = dev
            .io
            .keys()
            .filter_map(|x| match x {
                IoId::Ipad(_) => None,
                IoId::Mc(mc) => {
                    if mc.0 == fb {
                        Some(mc.1)
                    } else {
                        None
                    }
                }
            })
            .collect();
        olocs.push(*cands.choose(&mut rng).unwrap());
    }
    let mut vlog = String::new();
    writeln!(vlog, "module top(")?;
    let mut io_names = HashMap::new();
    for (pin, info) in &pkg.pins {
        if let &PkgPin::Io(io) = info {
            writeln!(vlog, "    (* LOC = \"{pin}\" *)")?;
            let name = match io {
                IoId::Ipad(ip) => format!("IPAD{ip}", ip = ip.to_idx()),
                IoId::Mc(mc) => format!("IO{f}_{m}", f = mc.0.to_idx(), m = mc.1.to_idx()),
            };
            writeln!(vlog, "    inout {name},",)?;
            io_names.insert(io, name);
        }
    }
    writeln!(vlog, "    input DUMMY);")?;
    for mc in dev.io.keys() {
        let n = &io_names[mc];
        writeln!(vlog, "    wire I_{n};")?;
        writeln!(vlog, "    IBUF IB_{n}(.I({n}), .O(I_{n}));")?;
    }
    for fb in dev.fbs() {
        for mc in dev.fb_mcs() {
            writeln!(
                vlog,
                "    wire MC{f}_{m};",
                f = fb.to_idx(),
                m = mc.to_idx()
            )?;
        }
    }
    let clk = *dev.clk_pads.first().unwrap();
    for fb in dev.fbs() {
        let imux_set = &imux_sets[fb];
        let any_imux = imux_set
            .iter()
            .copied()
            .find(|&x| x != ImuxInput::Pup)
            .unwrap();
        let any_inp = match any_imux {
            ImuxInput::Ibuf(io) => {
                format!("I_{n}", n = io_names[&io])
            }
            ImuxInput::Mc(mc) => {
                format!("MC{f}_{m}", f = mc.0.to_idx(), m = mc.1.to_idx())
            }
            _ => unreachable!(),
        };
        for mc in dev.fb_mcs() {
            if olocs[fb] == mc {
                writeln!(
                    vlog,
                    "    (* LOC = \"FB{f}_{m}\" *)",
                    f = fb.to_idx() + 1,
                    m = mc.to_idx() + 1
                )?;
                let inps = imux_set
                    .iter()
                    .filter(|&&x| x != ImuxInput::Pup)
                    .map(|&inp| match inp {
                        ImuxInput::Ibuf(io) => {
                            format!("I_{n}", n = io_names[&io])
                        }
                        ImuxInput::Mc(mc) => {
                            format!("MC{f}_{m}", f = mc.0.to_idx(), m = mc.1.to_idx())
                        }
                        _ => unreachable!(),
                    })
                    .join(" & ");
                writeln!(
                    vlog,
                    "    FD #(.INIT(1'b{init})) fd{f}_{m} (.D({inps}), .C(I_{cn}), .Q(MC{f}_{m}));",
                    init = if imux_set.contains(&ImuxInput::Pup) {
                        '1'
                    } else {
                        '0'
                    },
                    f = fb.to_idx(),
                    m = mc.to_idx(),
                    cn = io_names[&clk]
                )?;
                writeln!(
                    vlog,
                    "    OBUFE ob{f}_{m}(.I(MC{f}_{m}), .O(IO{f}_{m}), .E(1));",
                    f = fb.to_idx(),
                    m = mc.to_idx()
                )?;
            } else {
                writeln!(
                    vlog,
                    "    (* LOC = \"FB{f}_{m}\" *)",
                    f = fb.to_idx() + 1,
                    m = mc.to_idx() + 1
                )?;
                writeln!(
                    vlog,
                    "    FD fd{f}_{m} (.D({any_inp}), .C(I_{cn}), .Q(MC{f}_{m}));",
                    f = fb.to_idx(),
                    m = mc.to_idx(),
                    cn = io_names[&clk]
                )?;
            }
        }
    }
    writeln!(vlog, "endmodule")?;
    let mut opts = FitOpts::default();
    match dev.kind {
        DeviceKind::Xc9500 => {
            opts.localfbk = true;
        }
        DeviceKind::Xpla3 => {
            opts.noisp = true;
            opts.inputs = Some(40);
            opts.blkfanin = Some(40);
        }
        DeviceKind::Coolrunner2 => {
            opts.inputs = Some(40);
            opts.blkfanin = Some(40);
        }
        _ => (),
    }
    opts.silent = true;
    match v2vm6(tc, part, &vlog, &opts) {
        Ok((_, vm6)) => {
            let mut res: ImuxData = dev.fb_imuxes().map(|_| HashMap::new()).collect();
            let mut mcxlat = HashMap::new();
            let mut nxlat = HashMap::new();
            for (fbid, fb) in &vm6.fbs {
                for (mcid, pin) in &fb.pins {
                    if let Some(ib) = pin.ibuf {
                        let ib = &vm6.ibufs[ib];
                        for n in &ib.inodes {
                            nxlat.insert(
                                &*vm6.nodes[n.node].name,
                                ImuxInput::Ibuf(IoId::Mc((fbid, mcid))),
                            );
                        }
                    }
                    if let Some(mc) = pin.mc {
                        let mcn = vm6.macrocells.key(mc);
                        mcxlat.insert(&**mcn, (fbid, mcid));
                    }
                }
            }
            if let Some(ref ifb) = vm6.ipad_fb {
                for (ipid, pin) in &ifb.pins {
                    if let Some(ib) = pin.ibuf {
                        let ib = &vm6.ibufs[ib];
                        for n in &ib.inodes {
                            nxlat.insert(
                                &*vm6.nodes[n.node].name,
                                ImuxInput::Ibuf(IoId::Ipad(ipid)),
                            );
                        }
                    }
                }
            }

            for n in vm6.nodes.values() {
                match n.kind {
                    NodeKind::McQ | NodeKind::McUim => {
                        let nd = n.driver.as_ref().unwrap();
                        nxlat.insert(&*n.name, ImuxInput::Mc(mcxlat[&**nd]));
                    }
                    NodeKind::McFbk => {
                        let nd = n.driver.as_ref().unwrap();
                        nxlat.insert(&*n.name, ImuxInput::Fbk(mcxlat[&**nd].1));
                    }

                    _ => (),
                }
            }

            if dev.kind == DeviceKind::Xpla3 {
                nxlat.insert("xPUP_0", ImuxInput::Pup);
            }

            for fb in vm6.fbs.values() {
                for inp in &fb.inputs {
                    let path = nxlat[&*inp.name];
                    let FbImux::Plain(imux) = fb.imux[inp.index] else {
                        assert_eq!(dev.kind, DeviceKind::Xc9500);
                        continue;
                    };
                    match res[inp.index].entry(path) {
                        Entry::Occupied(e) => assert_eq!(*e.get(), imux),
                        Entry::Vacant(e) => {
                            e.insert(imux);
                        }
                    }
                }
            }
            Ok(Some(res))
        }
        Err(_) => Ok(None),
    }
}

pub fn gather_imux(
    tc: &Toolchain,
    part: &str,
    dev: &Device,
    pkg: &Package,
) -> Result<ImuxData, Box<dyn Error>> {
    let mut data = None;
    let tgt = match (dev.kind, dev.fbs) {
        (DeviceKind::Xc9500, 2) => 34,
        (DeviceKind::Xc9500, 4) => 270,
        (DeviceKind::Xc9500, 6) => 378,
        (DeviceKind::Xc9500, 8) => 453,
        (DeviceKind::Xc9500, 12) => 552, // XXX two never-bonded pads
        (DeviceKind::Xc9500, 16) => 630,
        (DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv, 2) => 216,
        (DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv, 4) => 432,
        (DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv, 8) => 813, // XXX never-bonded pads, many of them
        (DeviceKind::Xc9500Xl | DeviceKind::Xc9500Xv, 16) => 1458,
        (DeviceKind::Xpla3, 2) => 240,
        (DeviceKind::Xpla3, 4) => 520,
        (DeviceKind::Xpla3, 8) => 880,
        (DeviceKind::Xpla3, 16) => 1560,
        (DeviceKind::Xpla3, 24) => 2120,
        (DeviceKind::Xpla3, 32) => 2840,
        (DeviceKind::Coolrunner2, 2) => 240,
        (DeviceKind::Coolrunner2, 4) => 480,
        (DeviceKind::Coolrunner2, 8) => 880,
        (DeviceKind::Coolrunner2, 16) => 1600,
        (DeviceKind::Coolrunner2, 24) => 2480,
        (DeviceKind::Coolrunner2, 32) => 3120,
        _ => unreachable!(),
    };
    let mut cnt = 0;
    loop {
        let results: Vec<_> = (0..available_parallelism().unwrap().into())
            .into_par_iter()
            .map(|_| gather_imux_once(tc, part, dev, pkg).unwrap())
            .collect();
        for res in results.into_iter().flatten() {
            match data {
                None => data = Some(res),
                Some(ref mut data) => {
                    for imid in dev.fb_imuxes() {
                        for (&k, &v) in &res[imid] {
                            match data[imid].entry(k) {
                                Entry::Occupied(e) => assert_eq!(*e.get(), v),
                                Entry::Vacant(e) => {
                                    e.insert(v);
                                }
                            }
                        }
                    }
                }
            }
            cnt += 1;
        }
        let mut sz = 0;
        for im in data.as_ref().unwrap().values() {
            sz += im.len();
        }
        if sz == tgt {
            break;
        }
        assert!(sz < tgt);
        println!("at {cnt}: {sz}")
    }
    println!("DONE {part} in {cnt}");
    Ok(data.unwrap())
}
