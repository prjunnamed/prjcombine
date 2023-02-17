use prjcombine_entity::EntityId;
use prjcombine_vm6::{
    BufOe, Fbnand, IBuf, InputNode, InputNodeKind, Macrocell, MacrocellId, Node, NodeId,
    NodeIoKind, NodeKind, OBuf, PTerm, Signal, Srff, Vm6,
};

use prjcombine_xilinx_cpld::types::{FbId, FbnId, PTermId};
use prjcombine_xilinx_recpld::vm6::insert_node;

pub fn insert_node_simple(vm6: &mut Vm6, name: &str, driver: &str, kind: NodeKind) -> NodeId {
    insert_node(
        vm6,
        Node {
            is_signal: false,
            name: name.into(),
            io_kind: match kind {
                NodeKind::None => NodeIoKind::Input,
                NodeKind::OiOut => NodeIoKind::Inout,
                _ => NodeIoKind::None,
            },
            flags: 0,
            module: "top".into(),
            copy_of: match kind {
                NodeKind::McQ => Some(format!("{driver}.Q")),
                NodeKind::McOe => Some(format!("{driver}.BUFOE.OUT")),
                _ => None,
            },
            driver: if kind == NodeKind::None {
                None
            } else {
                Some(driver.into())
            },
            kind,
            terms: vec![],
        },
    )
}

pub fn insert_node_signal(vm6: &mut Vm6, name: &str, driver: &str, kind: NodeKind) -> NodeId {
    insert_node(
        vm6,
        Node {
            is_signal: true,
            name: name.into(),
            io_kind: NodeIoKind::None,
            flags: 0x1000,
            module: "top".into(),
            copy_of: None,
            driver: Some(driver.into()),
            kind,
            terms: vec![],
        },
    )
}

pub fn insert_ibuf(vm6: &mut Vm6, name: &str, kind: NodeKind, flags: u32) -> NodeId {
    let node_pad = insert_node_simple(vm6, &format!("{name}_PAD"), "", NodeKind::None);
    let name_ib = format!("{name}_IBUF");
    let node = insert_node_simple(vm6, name, &name_ib, kind);
    vm6.ibufs.insert(
        name_ib,
        IBuf {
            module: "top".into(),
            flags,
            inodes: vec![InputNode {
                kind: InputNodeKind::IiIn,
                node: node_pad,
            }],
            onodes: vec![node],
        },
    );
    node
}

pub fn insert_mc(vm6: &mut Vm6, name: &str, flags: u32) -> MacrocellId {
    vm6.macrocells
        .insert(
            name.into(),
            Macrocell {
                module: "top".into(),
                flags,
                inodes: vec![],
                onodes: vec![],
                signal: Some(Signal {
                    name: format!("{name}.SI"),
                    inodes: vec![],
                    onodes: vec![],
                }),
                srff: None,
                bufoe: None,
            },
        )
        .0
}

pub fn insert_mc_si(vm6: &mut Vm6, mcid: MacrocellId, kind: NodeKind, srcs: &[NodeId]) -> NodeId {
    let n = vm6.macrocells.key(mcid).clone();
    let suf = match kind {
        NodeKind::McSiD1 => "D1",
        NodeKind::McSiD2 => "D2",
        NodeKind::McSiExport => "EXP",
        NodeKind::McSiClkf => "CLKF",
        NodeKind::McSiRstf => "RSTF",
        NodeKind::McSiSetf => "SETF",
        NodeKind::McSiTrst => "TRST",
        NodeKind::McSiCe => "CE",
        _ => unreachable!(),
    };
    let node = insert_node_signal(vm6, &format!("{n}.{suf}"), &format!("{n}.SI"), kind);
    let mc = &mut vm6.macrocells[mcid];
    for &src in srcs {
        mc.inodes.push(InputNode {
            kind: InputNodeKind::None,
            node: src,
        });
        mc.signal.as_mut().unwrap().inodes.push(InputNode {
            kind: InputNodeKind::None,
            node: src,
        });
    }
    mc.signal.as_mut().unwrap().onodes.push(node);
    let mut terms = vec![];
    for &src in srcs {
        let sn = vm6.nodes[src].name.clone();
        terms.push(PTerm {
            inputs: vec![(true, sn)],
        });
    }
    vm6.nodes[node].terms = terms;
    node
}

pub fn insert_srff(vm6: &mut Vm6, mcid: MacrocellId) {
    let n = vm6.macrocells.key(mcid).clone();
    let node_d = insert_node_simple(vm6, &format!("{n}.D"), &format!("{n}.XOR"), NodeKind::AluF);
    let node_q = insert_node_simple(vm6, &format!("{n}.Q"), &format!("{n}.REG"), NodeKind::SrffQ);
    let mc = &mut vm6.macrocells[mcid];
    mc.srff = Some(Srff {
        name: format!("{n}.REG"),
        inodes: vec![InputNode {
            kind: InputNodeKind::SrffD,
            node: node_d,
        }],
        onodes: vec![node_q],
    })
}

pub fn insert_srff_ireg(vm6: &mut Vm6, mcid: MacrocellId, node_d: NodeId) {
    let n = vm6.macrocells.key(mcid).clone();
    let node_q = insert_node_simple(vm6, &format!("{n}.Q"), &format!("{n}.REG"), NodeKind::SrffQ);
    let mc = &mut vm6.macrocells[mcid];
    mc.inodes.push(InputNode {
        kind: InputNodeKind::None,
        node: node_d,
    });
    mc.srff = Some(Srff {
        name: format!("{n}.REG"),
        inodes: vec![InputNode {
            kind: InputNodeKind::SrffD,
            node: node_d,
        }],
        onodes: vec![node_q],
    })
}

pub fn insert_srff_inp(vm6: &mut Vm6, mcid: MacrocellId, kind: InputNodeKind, node: NodeId) {
    let mc = &mut vm6.macrocells[mcid];
    if !mc.signal.as_ref().unwrap().onodes.contains(&node) {
        mc.inodes.push(InputNode {
            kind: InputNodeKind::None,
            node,
        });
    }
    mc.srff
        .as_mut()
        .unwrap()
        .inodes
        .push(InputNode { kind, node });
}

pub fn insert_bufoe(vm6: &mut Vm6, mcid: MacrocellId, node: NodeId) {
    let n = vm6.macrocells.key(mcid).clone();
    let name = format!("{n}.BUFOE");
    let node_buf = insert_node_simple(vm6, &format!("{name}.OUT"), &name, NodeKind::BufOut);
    let mc = &mut vm6.macrocells[mcid];
    if !mc.signal.as_ref().unwrap().onodes.contains(&node) {
        mc.inodes.push(InputNode {
            kind: InputNodeKind::None,
            node,
        });
    }
    mc.bufoe = Some(BufOe {
        name,
        inodes: vec![InputNode {
            kind: InputNodeKind::CtorUnknown,
            node,
        }],
        onodes: vec![node_buf],
    });
}

pub fn insert_mc_out(vm6: &mut Vm6, mcid: MacrocellId, kind: NodeKind) -> NodeId {
    let n = vm6.macrocells.key(mcid).clone();
    let suf = match kind {
        NodeKind::McQ => "Q",
        NodeKind::McExport => "EXP",
        NodeKind::McFbk => "FBK",
        NodeKind::McUim => "UIM",
        NodeKind::McOe => "OE",
        NodeKind::McGlb => "GLB",
        _ => unreachable!(),
    };
    let node = insert_node_simple(vm6, &format!("{n}_{suf}"), &n, kind);
    vm6.macrocells[mcid].onodes.push(node);
    node
}

pub fn insert_obuf(vm6: &mut Vm6, mcid: MacrocellId, flags: u32) {
    let n = vm6.macrocells.key(mcid).clone();
    let mut inodes = vec![];
    let q = insert_mc_out(vm6, mcid, NodeKind::McQ);
    inodes.push(InputNode {
        kind: InputNodeKind::OiIn,
        node: q,
    });
    if vm6.macrocells[mcid].bufoe.is_some() {
        let oe = insert_mc_out(vm6, mcid, NodeKind::McOe);
        inodes.push(InputNode {
            kind: InputNodeKind::OiOe,
            node: oe,
        });
    }
    let pad = insert_node_simple(
        vm6,
        &format!("{n}_PAD"),
        &format!("{n}_OBUF"),
        NodeKind::OiOut,
    );
    vm6.obufs.insert(
        format!("{n}_OBUF"),
        OBuf {
            module: "top".into(),
            flags,
            inodes,
            onodes: vec![pad],
        },
    );
}

pub fn insert_fbn(vm6: &mut Vm6, name: &str, inps: &[NodeId]) -> NodeId {
    let node = insert_node_simple(vm6, &format!("{name}_OUT"), name, NodeKind::FbnOut);
    let inputs = inps
        .iter()
        .map(|&node| (true, vm6.nodes[node].name.clone()))
        .collect();
    vm6.fbs.first_mut().unwrap().fbnands.insert(
        FbnId::from_idx(0),
        Fbnand {
            name: name.into(),
            module: "top".into(),
            inodes: inps
                .iter()
                .map(|&node| InputNode {
                    kind: InputNodeKind::None,
                    node,
                })
                .collect(),
            onodes: vec![node],
            term: PTerm { inputs },
        },
    );
    node
}

pub fn insert_ct(vm6: &mut Vm6, fb: FbId, pt: PTermId, inps: &[NodeId]) -> NodeId {
    let ctn = vm6.fbs[fb].ct.as_ref().unwrap().name.clone();
    let kind = [
        NodeKind::CtSi0,
        NodeKind::CtSi1,
        NodeKind::CtSi2,
        NodeKind::CtSi3,
        NodeKind::CtSi4,
        NodeKind::CtSi5,
        NodeKind::CtSi6,
        NodeKind::CtSi7,
    ][pt.to_idx()];
    let node = insert_node_signal(vm6, &format!("{ctn}/{pti}", pti = pt.to_idx()), &ctn, kind);
    let inputs = inps
        .iter()
        .map(|&node| (true, vm6.nodes[node].name.clone()))
        .collect();
    vm6.nodes[node].terms.push(PTerm { inputs });
    let ct = vm6.fbs[fb].ct.as_mut().unwrap();
    ct.onodes.push(node);
    for &inp in inps {
        ct.inodes.push(InputNode {
            kind: InputNodeKind::None,
            node: inp,
        });
    }
    node
}
