use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use prjcombine_vm6::{
    Ct, Fb, FbImux, FbPin, IpadFb, IpadFbPin, Node, NodeId, NodeIoKind, NodeKind, OBuf, Pla, Vm6,
};
use prjcombine_xilinx_cpld::{
    device::{Device, DeviceKind, JtagPin, Package, PkgPin},
    types::IoId,
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::db::Part;

pub fn prep_vm6(part: &Part, device: &Device, package: &Package, speed: &str) -> Vm6 {
    let mut vm6 = Vm6 {
        nds_version: part.nds_version.clone(),
        family: part.vm6_family.clone(),
        dev: part.vm6_dev.clone(),
        devpkg: part.vm6_devpkg.clone(),
        part: format!("{d}{speed}-{p}", d = part.dev_name, p = part.pkg_name).to_ascii_uppercase(),
        network_name: "top".to_string(),
        network_flags: 0x4006,
        network_flags2: if device.kind == DeviceKind::Coolrunner2 {
            Some(0)
        } else {
            None
        },
        nodes: Default::default(),
        ibufs: Default::default(),
        macrocells: Default::default(),
        obufs: Default::default(),
        uims: Default::default(),
        fbs: Default::default(),
        ipad_fb: None,
        global_fclk: Default::default(),
        global_fsr: Default::default(),
        global_foe: Default::default(),
        dge: Default::default(),
        iostd_default: Default::default(),
        iostd: Default::default(),
        utc: Default::default(),
        cdr: Default::default(),
        prohibit_pins: Default::default(),
        vref: Default::default(),
    };
    if device.kind == DeviceKind::Coolrunner2 {
        vm6.iostd_default = Some("LVCMOS18".to_string());
    }
    let is_bga = package.pins.keys().any(|x| !x.starts_with('P'));
    let pin_map: HashMap<_, _> = package
        .pins
        .iter()
        .filter_map(|(pin, &info)| {
            if let PkgPin::Io(mc) = info {
                let pin = if is_bga {
                    &pin[..]
                } else {
                    pin.strip_prefix('P').unwrap()
                };

                Some((mc, pin))
            } else {
                None
            }
        })
        .collect();
    let oe_pads_remapped = device
        .oe_pads
        .map_values(|&io| package.spec_remap.get(&io).copied().unwrap_or(io));

    // Prep FBs
    for fbid in device.fbs() {
        let mut fb = Fb {
            module: "top".to_string(),
            pins: EntityPartVec::new(),
            inputs: vec![],
            imux: EntityVec::new(),
            ct: None,
            pla: None,
            fbnands: EntityPartVec::new(),
            global_fclk: EntityVec::new(),
        };
        for _ in 0..device.kind.imux_per_fb() {
            fb.imux.push(FbImux::None);
        }
        if !device.kind.is_xc9500() {
            fb.ct = Some(Ct {
                name: format!("FOOBAR{idx}__ctinst", idx = fbid.to_idx() + 1),
                module: "top".to_string(),
                inodes: vec![],
                onodes: vec![],
                invs: HashSet::new(),
            });
            fb.pla = Some(Pla {
                terms: EntityPartVec::new(),
            });
        }
        if device.kind == DeviceKind::Xpla3 {
            for _ in 0..2 {
                fb.global_fclk.push(None);
            }
        }
        for mc in device.fb_mcs() {
            let io = IoId::Mc((fbid, mc));
            fb.pins.insert(
                mc,
                FbPin {
                    mc: None,
                    ibuf: None,
                    obuf: None,
                    mc_used: false,
                    ibuf_used: false,
                    obuf_used: false,
                    pad: pin_map.get(&io).map(|x| {
                        let mut flags = 0xc000;
                        if device.clk_pads.values().contains(&io) {
                            flags |= 0x2000;
                        }
                        if oe_pads_remapped.values().contains(&io) {
                            flags |= 0x1000;
                        }
                        if device.sr_pad == Some(io) {
                            flags |= 0x800;
                        }
                        if device.dge_pad == Some(io) {
                            flags |= 0x400;
                        }
                        if device.cdr_pad == Some(io) {
                            flags |= 0x200;
                        }
                        match device.io[&io].jtag {
                            Some(JtagPin::Tdi) => flags |= 3,
                            Some(JtagPin::Tms) => flags |= 4,
                            Some(JtagPin::Tck) => flags |= 5,
                            Some(JtagPin::Tdo) => flags |= 6,
                            None => (),
                        }
                        (x.to_string(), flags)
                    }),
                },
            );
        }
        vm6.fbs.push(fb);
    }

    {
        let mut ipad_fb = IpadFb {
            module: "top".to_string(),
            pins: EntityPartVec::new(),
        };
        for ipad in device.ipads() {
            let io = IoId::Ipad(ipad);
            ipad_fb.pins.insert(
                ipad,
                IpadFbPin {
                    ibuf: None,
                    ibuf_used: false,
                    pad: pin_map.get(&io).map(|x| {
                        let mut flags = 0x8000;
                        if device.clk_pads.values().contains(&io) {
                            flags |= 0x2000;
                        }
                        if oe_pads_remapped.values().contains(&io) {
                            flags |= 0x1000;
                        }
                        if device.sr_pad == Some(io) {
                            flags |= 0x800;
                        }
                        if device.dge_pad == Some(io) {
                            flags |= 0x400;
                        }
                        if device.cdr_pad == Some(io) {
                            flags |= 0x200;
                        }
                        match device.io[&io].jtag {
                            Some(JtagPin::Tdi) => flags |= 3,
                            Some(JtagPin::Tms) => flags |= 4,
                            Some(JtagPin::Tck) => flags |= 5,
                            Some(JtagPin::Tdo) => flags |= 6,
                            None => (),
                        }
                        (x.to_string(), flags)
                    }),
                },
            );
        }
        vm6.ipad_fb = Some(ipad_fb);
    }

    vm6
}

pub fn insert_node(vm6: &mut Vm6, node: Node) -> NodeId {
    let idx = vm6.nodes.len() as u32;
    vm6.nodes.insert(idx, node).0
}

pub fn insert_dummy_obuf(vm6: &mut Vm6) {
    let node = insert_node(
        vm6,
        Node {
            is_signal: false,
            name: "obuf_dummy_n".to_string(),
            io_kind: NodeIoKind::Output,
            flags: 0,
            module: "meow".to_string(),
            copy_of: None,
            driver: Some("obuf_dummy".to_string()),
            kind: NodeKind::OiOut,
            terms: vec![],
        },
    );
    vm6.obufs.insert(
        "obuf_dummy".to_string(),
        OBuf {
            module: "meow".to_string(),
            flags: 0,
            inodes: vec![],
            onodes: vec![node],
        },
    );
    if let Some(ref iostd) = vm6.iostd_default {
        vm6.iostd.insert("obuf_dummy_n".to_string(), iostd.clone());
    }
}
