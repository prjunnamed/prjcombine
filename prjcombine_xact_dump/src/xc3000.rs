use std::collections::{BTreeMap, BTreeSet};

use enum_map::EnumMap;
use prjcombine_int::{
    db::{BelInfo, BelPin, Dir, IntDb, NodeKind, NodeTileId, PinDir, TermInfo, TermKind, WireKind},
    grid::{DieId, LayerId, EdgeIoCoord},
};
use prjcombine_xact_data::die::Die;
use prjcombine_xact_naming::db::{NamingDb, NodeNaming};
use prjcombine_xc2000::grid::{Grid, GridKind};
use prjcombine_xc2000::{
    bond::{Bond, BondPin, CfgPin},
    grid::SharedCfgPin,
};
use prjcombine_xc2000_xact::{name_device, ExpandedNamedDevice};
use unnamed_entity::{EntityId, EntityVec};

use crate::extractor::{Extractor, NetBinding};

fn bel_from_pins(db: &IntDb, pins: &[(&str, impl AsRef<str>)]) -> BelInfo {
    let mut bel = BelInfo::default();
    for &(name, ref wire) in pins {
        let wire = wire.as_ref();
        bel.pins.insert(
            name.into(),
            BelPin {
                wires: BTreeSet::from_iter([(NodeTileId::from_idx(0), db.get_wire(wire))]),
                dir: if wire.starts_with("IMUX") {
                    PinDir::Input
                } else {
                    PinDir::Output
                },
                is_intf_in: false,
            },
        );
    }
    bel
}

pub fn make_intdb() -> IntDb {
    let mut db = IntDb::default();
    let mut main_terms = EnumMap::from_fn(|dir| TermKind {
        dir,
        wires: Default::default(),
    });

    for (name, stub) in [
        ("SINGLE.H0", true),
        ("SINGLE.H1", false),
        ("SINGLE.H2", true),
        ("SINGLE.H3", false),
        ("SINGLE.H4", true),
        ("SINGLE.H.B0", true),
        ("SINGLE.H.B1", false),
        ("SINGLE.H.B2", true),
        ("SINGLE.H.B3", false),
        ("SINGLE.H.B4", true),
        ("SINGLE.H.T0", true),
        ("SINGLE.H.T1", false),
        ("SINGLE.H.T2", true),
        ("SINGLE.H.T3", false),
        ("SINGLE.H.T4", true),
    ] {
        let w0 = db.wires.insert(name.into(), WireKind::PipOut).0;
        let w1 = db
            .wires
            .insert(format!("{name}.E"), WireKind::PipBranch(Dir::W))
            .0;
        main_terms[Dir::W].wires.insert(w1, TermInfo::PassFar(w0));
        if stub {
            db.wires.insert(format!("{name}.STUB"), WireKind::PipOut);
        }
    }

    for (name, stub) in [
        ("SINGLE.V0", true),
        ("SINGLE.V1", false),
        ("SINGLE.V2", true),
        ("SINGLE.V3", false),
        ("SINGLE.V4", false),
        ("SINGLE.V.L0", false),
        ("SINGLE.V.L1", false),
        ("SINGLE.V.L2", true),
        ("SINGLE.V.L3", false),
        ("SINGLE.V.L4", true),
        ("SINGLE.V.R0", false),
        ("SINGLE.V.R1", false),
        ("SINGLE.V.R2", true),
        ("SINGLE.V.R3", false),
        ("SINGLE.V.R4", true),
    ] {
        let w0 = db.wires.insert(name.into(), WireKind::PipOut).0;
        let w1 = db
            .wires
            .insert(format!("{name}.S"), WireKind::PipBranch(Dir::N))
            .0;
        main_terms[Dir::N].wires.insert(w1, TermInfo::PassFar(w0));
        if stub {
            db.wires.insert(format!("{name}.STUB"), WireKind::PipOut);
            db.wires.insert(format!("{name}.S.STUB"), WireKind::PipOut);
        }
    }

    for name in [
        "LONG.H0",
        "LONG.H1",
        "LONG.IO.B0",
        "LONG.IO.B1",
        "LONG.IO.T0",
        "LONG.IO.T1",
    ] {
        let w = db
            .wires
            .insert(name.into(), WireKind::MultiBranch(Dir::W))
            .0;
        main_terms[Dir::W].wires.insert(w, TermInfo::PassFar(w));
    }
    for name in [
        "LONG.V0",
        "LONG.V1",
        "LONG.IO.L0",
        "LONG.IO.L1",
        "LONG.IO.R0",
        "LONG.IO.R1",
        "GCLK.V",
        "ACLK.V",
    ] {
        let w = db
            .wires
            .insert(name.into(), WireKind::MultiBranch(Dir::S))
            .0;
        main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
    }

    for name in [
        "GCLK", "ACLK", "IOCLK.B0", "IOCLK.B1", "IOCLK.T0", "IOCLK.T1", "IOCLK.L0", "IOCLK.L1",
        "IOCLK.R0", "IOCLK.R1",
    ] {
        db.wires.insert(name.into(), WireKind::ClkOut(0));
    }

    for name in [
        "IMUX.CLB.A",
        "IMUX.CLB.B",
        "IMUX.CLB.C",
        "IMUX.CLB.D",
        "IMUX.CLB.E",
        "IMUX.CLB.DI",
        "IMUX.CLB.EC",
        "IMUX.CLB.RD",
        "IMUX.CLB.K",
        "IMUX.BIOB0.O",
        "IMUX.BIOB0.T",
        "IMUX.BIOB0.IK",
        "IMUX.BIOB0.OK",
        "IMUX.BIOB1.O",
        "IMUX.BIOB1.T",
        "IMUX.BIOB1.IK",
        "IMUX.BIOB1.OK",
        "IMUX.TIOB0.O",
        "IMUX.TIOB0.T",
        "IMUX.TIOB0.IK",
        "IMUX.TIOB0.OK",
        "IMUX.TIOB1.O",
        "IMUX.TIOB1.T",
        "IMUX.TIOB1.IK",
        "IMUX.TIOB1.OK",
        "IMUX.LIOB0.O",
        "IMUX.LIOB0.T",
        "IMUX.LIOB0.IK",
        "IMUX.LIOB0.OK",
        "IMUX.LIOB1.O",
        "IMUX.LIOB1.T",
        "IMUX.LIOB1.IK",
        "IMUX.LIOB1.OK",
        "IMUX.RIOB0.O",
        "IMUX.RIOB0.T",
        "IMUX.RIOB0.IK",
        "IMUX.RIOB0.OK",
        "IMUX.RIOB1.O",
        "IMUX.RIOB1.T",
        "IMUX.RIOB1.IK",
        "IMUX.RIOB1.OK",
        "IMUX.TBUF0.I",
        "IMUX.TBUF0.T",
        "IMUX.TBUF1.I",
        "IMUX.TBUF1.T",
        "IMUX.TBUF2.I",
        "IMUX.TBUF2.T",
        "IMUX.TBUF3.I",
        "IMUX.TBUF3.T",
        "IMUX.BUFG",
        "IMUX.IOCLK0",
        "IMUX.IOCLK1",
    ] {
        db.wires.insert(name.into(), WireKind::MuxOut);
    }

    for (name, dirs) in [
        ("OUT.CLB.X", &[Dir::W, Dir::E][..]),
        ("OUT.CLB.Y", &[Dir::E, Dir::S][..]),
        ("OUT.BIOB0.I", &[][..]),
        ("OUT.BIOB0.Q", &[][..]),
        ("OUT.BIOB1.I", &[Dir::E][..]),
        ("OUT.BIOB1.Q", &[Dir::E][..]),
        ("OUT.TIOB0.I", &[][..]),
        ("OUT.TIOB0.Q", &[][..]),
        ("OUT.TIOB1.I", &[Dir::E][..]),
        ("OUT.TIOB1.Q", &[Dir::E][..]),
        ("OUT.LIOB0.I", &[][..]),
        ("OUT.LIOB0.Q", &[][..]),
        ("OUT.LIOB1.I", &[Dir::S][..]),
        ("OUT.LIOB1.Q", &[Dir::S][..]),
        ("OUT.RIOB0.I", &[][..]),
        ("OUT.RIOB0.Q", &[][..]),
        ("OUT.RIOB1.I", &[Dir::S][..]),
        ("OUT.RIOB1.Q", &[Dir::S][..]),
        ("OUT.CLKIOB", &[][..]),
        ("OUT.OSC", &[][..]),
    ] {
        let w = db.wires.insert(name.into(), WireKind::LogicOut).0;
        for &dir in dirs {
            let wo = db
                .wires
                .insert(format!("{name}.{dir}"), WireKind::Branch(!dir))
                .0;
            main_terms[!dir].wires.insert(wo, TermInfo::PassFar(w));

            if name == "OUT.CLB.X" && dir == Dir::E {
                let wos = db
                    .wires
                    .insert(format!("{name}.{dir}S"), WireKind::Branch(Dir::N))
                    .0;
                main_terms[Dir::N].wires.insert(wos, TermInfo::PassFar(wo));
            }
        }
    }

    let mut llh_w = main_terms[Dir::W].clone();
    let mut llh_e = main_terms[Dir::E].clone();
    let mut llv_s = main_terms[Dir::S].clone();
    let mut llv_n = main_terms[Dir::N].clone();
    let mut llvs_s = main_terms[Dir::S].clone();
    let mut llvs_n = main_terms[Dir::N].clone();

    for term in [&mut llh_w, &mut llh_e] {
        term.wires.remove(db.get_wire("LONG.IO.B0"));
        term.wires.remove(db.get_wire("LONG.IO.T0"));
    }
    for term in [&mut llv_s, &mut llv_n, &mut llvs_n, &mut llvs_s] {
        term.wires.remove(db.get_wire("LONG.IO.L0"));
        term.wires.remove(db.get_wire("LONG.IO.R0"));
    }
    for term in [&mut llv_s, &mut llv_n] {
        term.wires.remove(db.get_wire("LONG.V0"));
        term.wires.remove(db.get_wire("LONG.V1"));
    }

    for (dir, term) in main_terms {
        db.terms.insert(format!("MAIN.{dir}"), term);
    }
    db.terms.insert("LLH.W".into(), llh_w);
    db.terms.insert("LLH.E".into(), llh_e);
    db.terms.insert("LLV.S".into(), llv_s);
    db.terms.insert("LLV.N".into(), llv_n);
    db.terms.insert("LLV.S.S".into(), llvs_s);
    db.terms.insert("LLV.S.N".into(), llvs_n);

    for name in [
        "CLB", "CLB.L", "CLB.R", "CLB.B", "CLB.BL", "CLB.BR", "CLB.T", "CLB.TL", "CLB.TR",
    ] {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        node.bels.insert(
            "CLB".into(),
            bel_from_pins(
                &db,
                &[
                    ("A", "IMUX.CLB.A"),
                    ("B", "IMUX.CLB.B"),
                    ("C", "IMUX.CLB.C"),
                    ("D", "IMUX.CLB.D"),
                    ("E", "IMUX.CLB.E"),
                    ("DI", "IMUX.CLB.DI"),
                    ("EC", "IMUX.CLB.EC"),
                    ("RD", "IMUX.CLB.RD"),
                    ("K", "IMUX.CLB.K"),
                    ("X", "OUT.CLB.X"),
                    ("Y", "OUT.CLB.Y"),
                ],
            ),
        );

        if name.starts_with("CLB.B") || name.starts_with("CLB.T") {
            let bt = if name.starts_with("CLB.B") { 'B' } else { 'T' };
            for i in 0..2 {
                node.bels.insert(
                    format!("{bt}IOB{i}"),
                    bel_from_pins(
                        &db,
                        &[
                            ("O", format!("IMUX.{bt}IOB{i}.O")),
                            ("T", format!("IMUX.{bt}IOB{i}.T")),
                            ("IK", format!("IMUX.{bt}IOB{i}.IK")),
                            ("OK", format!("IMUX.{bt}IOB{i}.OK")),
                            ("I", format!("OUT.{bt}IOB{i}.I")),
                            ("Q", format!("OUT.{bt}IOB{i}.Q")),
                        ],
                    ),
                );
            }
        }

        if name.ends_with('L') || name.ends_with('R') {
            let lr = if name.ends_with('L') { 'L' } else { 'R' };
            for i in 0..2 {
                node.bels.insert(
                    format!("{lr}IOB{i}"),
                    bel_from_pins(
                        &db,
                        &[
                            ("O", format!("IMUX.{lr}IOB{i}.O")),
                            ("T", format!("IMUX.{lr}IOB{i}.T")),
                            ("IK", format!("IMUX.{lr}IOB{i}.IK")),
                            ("OK", format!("IMUX.{lr}IOB{i}.OK")),
                            ("I", format!("OUT.{lr}IOB{i}.I")),
                            ("Q", format!("OUT.{lr}IOB{i}.Q")),
                        ],
                    ),
                );
            }
        }

        for i in 0..4 {
            if i >= 2 && !name.ends_with('R') {
                continue;
            }
            node.bels.insert(
                format!("TBUF{i}"),
                bel_from_pins(
                    &db,
                    &[
                        ("I", format!("IMUX.TBUF{i}.I")),
                        ("T", format!("IMUX.TBUF{i}.T")),
                        ("O", format!("LONG.H{}", i % 2)),
                    ],
                ),
            );
        }
        if name.ends_with('L') || name.ends_with('R') {
            for i in 0..2 {
                node.bels.insert(
                    format!("PULLUP.TBUF{i}"),
                    bel_from_pins(&db, &[("O", format!("LONG.H{}", i % 2))]),
                );
            }
        }

        if name == "CLB.TL" || name == "CLB.BR" {
            node.bels
                .insert("CLKIOB".into(), bel_from_pins(&db, &[("I", "OUT.CLKIOB")]));
            node.bels.insert(
                "BUFG".into(),
                bel_from_pins(
                    &db,
                    &[
                        ("I", "IMUX.BUFG"),
                        ("O", if name == "CLB.TL" { "GCLK" } else { "ACLK" }),
                    ],
                ),
            );
        }
        if name == "CLB.BR" {
            node.bels
                .insert("OSC".into(), bel_from_pins(&db, &[("O", "OUT.OSC")]));
        }

        for subkind in 0..4 {
            if subkind == 3 && name != "CLB.R" {
                continue;
            }
            db.nodes.insert(format!("{name}.{subkind}"), node.clone());
            if matches!(name, "CLB.BL" | "CLB.BR" | "CLB.TL" | "CLB.TR" | "CLB.T") {
                db.nodes.insert(format!("{name}S.{subkind}"), node.clone());
            }
        }
    }
    for name in [
        "LLH.B", "LLH.T", "LLV.LS", "LLV.RS", "LLV.L", "LLV.R", "LLV",
    ] {
        let node = NodeKind {
            tiles: EntityVec::from_iter([(); 2]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        db.nodes.insert(name.into(), node);
    }

    db
}

pub fn make_grid(die: &Die, kind: GridKind) -> Grid {
    let pd_clb = die
        .primdefs
        .iter()
        .find(|(_, pd)| pd.name == "xcle")
        .unwrap()
        .0;
    let mut clb_x = BTreeSet::new();
    let mut clb_y = BTreeSet::new();
    for prim in die.prims.values() {
        if prim.primdef != pd_clb {
            continue;
        }
        clb_x.insert(prim.pins.first().unwrap().x);
        clb_y.insert(prim.pins.first().unwrap().y);
    }
    Grid {
        kind,
        columns: clb_x.len(),
        rows: clb_y.len(),
        cols_bidi: Default::default(),
        rows_bidi: Default::default(),
        is_small: clb_x.len() == 8,
        is_buff_large: false,
        cfg_io: Default::default(),
    }
}

pub fn dump_grid(die: &Die, kind: GridKind) -> (Grid, IntDb, NamingDb) {
    let grid = make_grid(die, kind);
    let mut intdb = make_intdb();
    let mut ndb = NamingDb::default();
    for name in intdb.nodes.keys() {
        ndb.node_namings.insert(name.clone(), NodeNaming::default());
        if name.starts_with("CLB") && !name.contains("L.") && !name.contains("R.") {
            ndb.node_namings
                .insert(format!("{name}.L1"), NodeNaming::default());
            if !name.starts_with("CLB.B") && !name.starts_with("CLB.T") {
                ndb.node_namings
                    .insert(format!("{name}.L1.B1"), NodeNaming::default());
            }
        }
        if name.starts_with("CLB") && !name.starts_with("CLB.B") && !name.starts_with("CLB.T") {
            ndb.node_namings
                .insert(format!("{name}.B1"), NodeNaming::default());
        }
    }
    let bd_c20 = die
        .boxdefs
        .iter()
        .find(|(_, bd)| bd.name == "cross20")
        .unwrap()
        .0;
    let mut c20_x = BTreeSet::new();
    let mut c20_y = BTreeSet::new();
    for boxx in die.boxes.values() {
        if boxx.boxdef != bd_c20 {
            continue;
        }
        c20_x.insert(usize::from(boxx.bx));
        c20_y.insert(usize::from(boxx.by));
    }
    let c20_x = Vec::from_iter(c20_x);
    let c20_y = Vec::from_iter(c20_y);
    assert_eq!(c20_x.len(), grid.columns - 1);
    assert_eq!(c20_y.len(), grid.rows - 1);
    ndb.tile_widths.insert("L".into(), c20_x[0] - 1);
    ndb.tile_widths.insert("C".into(), c20_x[1] - c20_x[0]);
    ndb.tile_widths.insert(
        "R".into(),
        die.matrix.as_ref().unwrap().dim().0 - (c20_x[grid.columns - 2] - 1),
    );
    ndb.tile_heights.insert("B".into(), c20_y[0] + 2);
    ndb.tile_heights.insert("C".into(), c20_y[1] - c20_y[0]);
    ndb.tile_heights.insert(
        "T".into(),
        die.matrix.as_ref().unwrap().dim().1 - (c20_y[grid.rows - 2] + 2),
    );
    let edev = grid.expand_grid(&intdb);
    let endev = name_device(&edev, &ndb);

    let mut extractor = Extractor::new(die, &edev.egrid, &endev.ngrid);

    let die = edev.egrid.die(DieId::from_idx(0));
    for col in die.cols() {
        for row in die.rows() {
            for (layer, node) in &die[(col, row)].nodes {
                let nloc = (die.die, col, row, layer);
                let node_kind = &intdb.nodes[node.kind];
                let nnode = &endev.ngrid.nodes[&nloc];
                for (bel, key, bel_info) in &node_kind.bels {
                    match &key[..] {
                        "CLB" | "OSC" | "CLKIOB" | "BUFG" => {
                            let mut prim = extractor.grab_prim_a(&nnode.bels[bel][0]);
                            for pin in bel_info.pins.keys() {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                        }
                        "BIOB0" | "BIOB1" | "TIOB0" | "TIOB1" | "LIOB0" | "LIOB1" | "RIOB0"
                        | "RIOB1" => {
                            let mut prim = extractor.grab_prim_i(&nnode.bels[bel][0]);
                            for pin in bel_info.pins.keys() {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                        }
                        "TBUF0" | "TBUF1" | "TBUF2" | "TBUF3" => {
                            let mut prim = extractor.grab_prim_a(&nnode.bels[bel][0]);
                            for pin in ["I", "T"] {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "O");
                            let (line, pip) = extractor.consume_one_fwd(o, nloc);
                            extractor.net_bel_int(line, nloc, bel, "O");
                            extractor.bel_pip(nnode.naming, bel, "O", pip);

                            let net_i = extractor.get_bel_int_net(nloc, bel, "I");
                            let net_o = extractor.get_bel_int_net(nloc, bel, "O");
                            let src_nets =
                                Vec::from_iter(extractor.nets[net_i].pips_bwd.keys().copied());
                            for net in src_nets {
                                extractor.mark_tbuf_pseudo(net_o, net);
                            }
                        }
                        "PULLUP.TBUF0" | "PULLUP.TBUF1" => {
                            let mut prim = extractor.grab_prim_a(&nnode.bels[bel][0]);
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "O");
                            let (line, pip) = extractor.consume_one_fwd(o, nloc);
                            extractor.net_bel_int(line, nloc, bel, "O");
                            extractor.bel_pip(nnode.naming, bel, "O", pip);
                        }
                        _ => panic!("umm bel {key}?"),
                    }
                }
            }
        }
    }
    extractor.junk_prim_names.extend(
        ["VCC", "GND", "M0RT", "M1RD", "DPGM", "RST", "PWRDN", "CCLK"]
            .into_iter()
            .map(|x| x.to_string()),
    );

    // long verticals + GCLK
    for col in die.cols() {
        let mut queue = vec![];
        for row in [grid.row_bio() + 1, grid.row_tio() - 1] {
            let by = endev.row_y[row].start;
            let ty = endev.row_y[row].end;
            let mut nets = vec![];
            for x in endev.col_x[col].clone() {
                let net_b = extractor.matrix_nets[(x, by)].net_b;
                let net_t = extractor.matrix_nets[(x, ty)].net_b;
                if net_b != net_t {
                    continue;
                }
                let Some(net) = net_b else { continue };
                if extractor.nets[net].binding != NetBinding::None {
                    continue;
                }
                nets.push(net);
            }
            let wires = if col == grid.col_lio() {
                &[
                    "IOCLK.L0",
                    "IOCLK.L1",
                    "LONG.IO.L0",
                    "LONG.IO.L1",
                    "LONG.V0",
                    "LONG.V1",
                    "ACLK.V",
                    "GCLK.V",
                ][..]
            } else if col == grid.col_rio() {
                &[
                    "GCLK.V",
                    "LONG.V0",
                    "LONG.V1",
                    "ACLK.V",
                    "LONG.IO.R1",
                    "LONG.IO.R0",
                    "IOCLK.R1",
                    "IOCLK.R0",
                ][..]
            } else {
                &["GCLK.V", "LONG.V0", "LONG.V1", "ACLK.V"][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }
    // long horizontals
    for row in die.rows() {
        let mut queue = vec![];
        for col in [grid.col_lio() + 1, grid.col_rio() - 1] {
            let lx = endev.col_x[col].start;
            let rx = endev.col_x[col].end;
            let mut nets = vec![];
            for y in endev.row_y[row].clone() {
                let net_l = extractor.matrix_nets[(lx, y)].net_l;
                let net_r = extractor.matrix_nets[(rx, y)].net_l;
                if net_l != net_r {
                    continue;
                }
                let Some(net) = net_l else { continue };
                if extractor.nets[net].binding != NetBinding::None {
                    continue;
                }
                nets.push(net);
            }
            let wires = if row == grid.row_bio() {
                &["IOCLK.B0", "IOCLK.B1", "LONG.IO.B0", "LONG.IO.B1"][..]
            } else if row == grid.row_tio() {
                &["LONG.IO.T1", "LONG.IO.T0", "IOCLK.T1", "IOCLK.T0"][..]
            } else {
                &[][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }

    // assign LL splitters to their proper tiles
    let mut queue = vec![];
    for (net_t, net_info) in &extractor.nets {
        let NetBinding::Int(rwt) = net_info.binding else {
            continue;
        };
        for &net_f in net_info.pips_bwd.keys() {
            let NetBinding::Int(rwf) = extractor.nets[net_f].binding else {
                continue;
            };
            if rwt.2 != rwf.2 {
                continue;
            }
            if rwt.1 .0 == rwf.1 .0 {
                assert_ne!(rwt.1 .1, rwf.1 .1);
                // LLV
                let col = rwt.1 .0;
                let row = grid.row_mid();
                let layer = edev
                    .egrid
                    .find_node_layer(die.die, (col, row), |node| node.starts_with("LLV"))
                    .unwrap();
                queue.push((net_t, net_f, (die.die, col, row, layer)))
            } else {
                assert_eq!(rwt.1 .1, rwf.1 .1);
                // LLH
                let col = grid.col_mid();
                let row = rwt.1 .1;
                let layer = edev
                    .egrid
                    .find_node_layer(die.die, (col, row), |node| node.starts_with("LLH"))
                    .unwrap();
                queue.push((net_t, net_f, (die.die, col, row, layer)))
            }
        }
    }
    for (net_t, net_f, nloc) in queue {
        extractor.own_pip(net_t, net_f, nloc);
    }

    // horizontal singles
    let mut queue = vec![];
    for col in die.cols() {
        let mut x = endev.col_x[col].end;
        if col == grid.col_rio() {
            x = endev.col_x[col].start + 8;
        }
        for row in die.rows() {
            let mut nets = vec![];
            for y in endev.row_y[row].clone() {
                let Some(net) = extractor.matrix_nets[(x, y)].net_l else {
                    continue;
                };
                if extractor.nets[net].binding != NetBinding::None {
                    continue;
                }
                nets.push(net);
            }
            let wires = if row == grid.row_bio() {
                &[
                    "SINGLE.H.B4",
                    "SINGLE.H.B3",
                    "SINGLE.H.B2",
                    "SINGLE.H.B1",
                    "SINGLE.H.B0",
                    "SINGLE.H4",
                    "SINGLE.H3",
                    "SINGLE.H2",
                    "SINGLE.H1",
                    "SINGLE.H0",
                ][..]
            } else if row == grid.row_tio() {
                &[
                    "SINGLE.H.T4",
                    "SINGLE.H.T3",
                    "SINGLE.H.T2",
                    "SINGLE.H.T1",
                    "SINGLE.H.T0",
                ][..]
            } else {
                &[
                    "SINGLE.H4",
                    "SINGLE.H3",
                    "SINGLE.H2",
                    "SINGLE.H1",
                    "SINGLE.H0",
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
    }
    // vertical singles
    for row in die.rows() {
        let mut y = endev.row_y[row].start;
        if row == grid.row_bio() {
            y = endev.row_y[row + 1].start - 8;
        }
        for col in die.cols() {
            let mut nets = vec![];
            for x in endev.col_x[col].clone() {
                let Some(net) = extractor.matrix_nets[(x, y)].net_b else {
                    continue;
                };
                if extractor.nets[net].binding != NetBinding::None {
                    continue;
                }
                nets.push(net);
            }
            let wires = if col == grid.col_lio() {
                &[
                    "SINGLE.V.L0",
                    "SINGLE.V.L1",
                    "SINGLE.V.L2",
                    "SINGLE.V.L3",
                    "SINGLE.V.L4",
                ][..]
            } else if col == grid.col_rio() {
                &[
                    "SINGLE.V0",
                    "SINGLE.V1",
                    "SINGLE.V2",
                    "SINGLE.V3",
                    "SINGLE.V4",
                    "SINGLE.V.R0",
                    "SINGLE.V.R1",
                    "SINGLE.V.R2",
                    "SINGLE.V.R3",
                    "SINGLE.V.R4",
                ][..]
            } else {
                &[
                    "SINGLE.V0",
                    "SINGLE.V1",
                    "SINGLE.V2",
                    "SINGLE.V3",
                    "SINGLE.V4",
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
    }
    for (net, wire) in queue {
        extractor.net_int(net, wire);
    }

    // find single stubs
    for x in 0..extractor.matrix.dim().0 {
        for y in 0..extractor.matrix.dim().1 {
            let cv = extractor.matrix[(x, y)] & 0xff;
            if cv == 0x28 {
                // vertical joiner
                let net_u = extractor.matrix_nets[(x, y + 1)].net_b.unwrap();
                let net_d = extractor.matrix_nets[(x, y)].net_b.unwrap();
                if let NetBinding::Int(rw) = extractor.nets[net_u].binding {
                    if rw.1 .1 == grid.row_bio() {
                        if extractor.nets[net_d].binding == NetBinding::None {
                            let sw =
                                intdb.get_wire(&format!("{wn}.STUB", wn = intdb.wires.key(rw.2)));
                            extractor.net_int(net_d, (rw.0, rw.1, sw));
                        }
                    } else {
                        if extractor.nets[net_d].binding == NetBinding::None {
                            let sw =
                                intdb.get_wire(&format!("{wn}.S.STUB", wn = intdb.wires.key(rw.2)));
                            extractor.net_int(net_d, (rw.0, (rw.1 .0, rw.1 .1 - 1), sw));
                        }
                    }
                }
            } else if cv == 0x68 {
                // horizontal joiner
                let net_r = extractor.matrix_nets[(x + 1, y)].net_l.unwrap();
                let net_l = extractor.matrix_nets[(x, y)].net_l.unwrap();
                if let NetBinding::Int(rw) = extractor.nets[net_r].binding {
                    if extractor.nets[net_l].binding == NetBinding::None {
                        let sw = intdb.get_wire(&format!("{wn}.STUB", wn = intdb.wires.key(rw.2)));
                        extractor.net_int(net_l, (rw.0, rw.1, sw));
                    }
                }
            }
        }
    }

    let xlut = endev.col_x.map_values(|x| x.end);
    let ylut = endev.row_y.map_values(|y| y.end);
    for (box_id, boxx) in &extractor.die.boxes {
        let col = xlut.binary_search(&usize::from(boxx.bx)).unwrap_err();
        let row = ylut.binary_search(&usize::from(boxx.by)).unwrap_err();
        extractor.own_box(box_id, (die.die, col, row, LayerId::from_idx(0)));
    }

    // find IMUX.IOCLK
    let mut queue = vec![];
    for (net, net_info) in &extractor.nets {
        if net_info.binding != NetBinding::None {
            continue;
        }
        assert_eq!(net_info.pips_fwd.len(), 1);
        let (&net_t, &pip) = net_info.pips_fwd.iter().next().unwrap();
        let NetBinding::Int(rw) = extractor.nets[net_t].binding else {
            unreachable!()
        };
        let wn = intdb.wires.key(rw.2);
        assert!(wn.starts_with("IOCLK"));
        let nwn = if wn.ends_with('1') {
            "IMUX.IOCLK1"
        } else {
            "IMUX.IOCLK0"
        };
        let col = xlut.binary_search(&pip.0).unwrap_err();
        let row = ylut.binary_search(&pip.1).unwrap_err();
        assert!(col == grid.col_lio() || col == grid.col_rio());
        assert!(row == grid.row_bio() || row == grid.row_tio());
        queue.push((net, (die.die, (col, row), intdb.get_wire(nwn))));
    }
    for (net, wire) in queue {
        extractor.net_int(net, wire);
    }

    for (wire, name, &kind) in &intdb.wires {
        if !name.starts_with("IMUX") {
            continue;
        }
        if kind != WireKind::MuxOut {
            continue;
        }
        for col in die.cols() {
            for row in die.rows() {
                let rw = edev
                    .egrid
                    .resolve_wire((die.die, (col, row), wire))
                    .unwrap();
                if extractor.int_nets.contains_key(&rw) {
                    extractor.own_mux(rw, (die.die, col, row, LayerId::from_idx(0)));
                }
            }
        }
    }

    let finisher = extractor.finish();
    finisher.finish(&mut intdb, &mut ndb);
    (grid, intdb, ndb)
}

pub fn make_bond(
    endev: &ExpandedNamedDevice,
    name: &str,
    pkg: &BTreeMap<String, String>,
) -> (Bond, BTreeMap<SharedCfgPin, EdgeIoCoord>) {
    let io_lookup: BTreeMap<_, _> = endev
        .grid
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io), io))
        .collect();
    let mut bond = Bond {
        pins: Default::default(),
    };
    for (pin, pad) in pkg {
        let io = io_lookup[&**pad];
        bond.pins.insert(pin.to_ascii_uppercase(), BondPin::Io(io));
    }

    let (pwrdwn, m1, m0, prog, done, cclk, gnd, vcc) = match name {
        "pc44" => (
            "P7",
            "P16",
            "P17",
            "P27",
            "P28",
            "P40",
            &["P1", "P23"][..],
            &["P12", "P34"][..],
        ),
        "pc68" => (
            "P10",
            "P25",
            "P26",
            "P44",
            "P45",
            "P60",
            &["P1", "P35"][..],
            &["P18", "P52"][..],
        ),
        "pc84" if endev.grid.columns < 14 => (
            "P12",
            "P31",
            "P32",
            "P54",
            "P55",
            "P74",
            &["P1", "P43"][..],
            &["P22", "P64"][..],
        ),
        "pc84" if endev.grid.columns >= 14 => (
            "P12",
            "P31",
            "P32",
            "P54",
            "P55",
            "P74",
            &["P1", "P21", "P43", "P65"][..],
            &["P2", "P22", "P42", "P64"][..],
        ),

        "pq100" => (
            "P29",
            "P52",
            "P54",
            "P78",
            "P80",
            "P2",
            &["P4", "P16", "P28", "P53", "P66", "P77"][..],
            &["P3", "P27", "P41", "P55", "P79", "P91"][..],
        ),
        "pq160" => (
            "P159",
            "P40",
            "P42",
            "P78",
            "P80",
            "P121",
            &["P19", "P41", "P61", "P77", "P101", "P123", "P139", "P158"][..],
            &["P20", "P43", "P60", "P79", "P100", "P122", "P140", "P157"][..],
        ),
        "pq208" if endev.grid.columns == 16 => (
            "P3",
            "P48",
            "P50",
            "P102",
            "P107",
            "P153",
            &["P2", "P25", "P49", "P79", "P101", "P131", "P160", "P182"][..],
            &["P26", "P55", "P78", "P106", "P130", "P154", "P183", "P205"][..],
        ),
        "pq208" if endev.grid.columns == 22 => (
            "P1",
            "P50",
            "P52",
            "P105",
            "P107",
            "P156",
            &["P26", "P51", "P79", "P104", "P131", "P158", "P182", "P208"][..],
            &["P27", "P53", "P78", "P106", "P130", "P157", "P183", "P207"][..],
        ),

        "vq64" => (
            "P17",
            "P31",
            "P32",
            "P48",
            "P49",
            "P64",
            &["P8", "P41"][..],
            &["P24", "P56"][..],
        ),
        "tq100" | "vq100" => (
            "P26",
            "P49",
            "P51",
            "P75",
            "P77",
            "P99",
            &["P1", "P13", "P25", "P50", "P63", "P74"][..],
            &["P24", "P38", "P52", "P76", "P88", "P100"][..],
        ),
        "tq144" => (
            "P1",
            "P36",
            "P38",
            "P71",
            "P73",
            "P108",
            &["P18", "P37", "P55", "P70", "P91", "P110", "P126", "P144"][..],
            &["P19", "P39", "P54", "P72", "P90", "P109", "P127", "P143"][..],
        ),
        "tq176" => (
            "P1",
            "P45",
            "P47",
            "P87",
            "P89",
            "P132",
            &["P22", "P46", "P67", "P86", "P111", "P134", "P154", "P176"][..],
            &["P23", "P48", "P66", "P88", "P110", "P133", "P155", "P175"][..],
        ),

        "cb100" | "cq100" => (
            "P14",
            "P37",
            "P39",
            "P63",
            "P65",
            "P87",
            &["P1", "P13", "P38", "P51", "P62", "P89"][..],
            &["P12", "P26", "P40", "P64", "P76", "P88"][..],
        ),
        "cb164" | "cq164" => (
            "P20",
            "P62",
            "P64",
            "P101",
            "P103",
            "P145",
            &["P19", "P41", "P63", "P83", "P100", "P124", "P147", "P164"][..],
            &["P1", "P18", "P42", "P65", "P82", "P102", "P123", "P146"][..],
        ),

        "pg84" => (
            "B2",
            "J2",
            "L1",
            "K10",
            "J10",
            "A11",
            &["C6", "J6"][..],
            &["F3", "F9"][..],
        ),
        "pg132" | "pp132" => (
            "A1",
            "B13",
            "A14",
            "P14",
            "N13",
            "P1",
            &["C4", "C7", "C11", "H12", "L12", "M7", "L3", "H3"][..],
            &["C8", "D12", "G12", "M11", "M8", "M4", "G3", "D3"][..],
        ),
        "pp175" | "pg175" => (
            "B2",
            "B14",
            "B15",
            "R15",
            "R14",
            "R2",
            &["D8", "C14", "J14", "N14", "N8", "N3", "J3", "C3"][..],
            &["D9", "D14", "H14", "P14", "N9", "P3", "H3", "D3"][..],
        ),
        "pg223" => (
            "B2",
            "C16",
            "B17",
            "U17",
            "V17",
            "U2",
            &["D9", "D15", "K15", "R16", "R9", "R3", "K4", "D4"][..],
            &["D10", "D16", "J15", "R15", "R10", "R4", "J4", "D3"][..],
        ),

        _ => panic!("ummm {name}?"),
    };
    assert_eq!(
        bond.pins
            .insert(pwrdwn.into(), BondPin::Cfg(CfgPin::PwrdwnB)),
        None
    );
    assert_eq!(bond.pins.insert(m0.into(), BondPin::Cfg(CfgPin::M0)), None);
    assert_eq!(bond.pins.insert(m1.into(), BondPin::Cfg(CfgPin::M1)), None);
    assert_eq!(
        bond.pins.insert(prog.into(), BondPin::Cfg(CfgPin::ProgB)),
        None
    );
    assert_eq!(
        bond.pins.insert(done.into(), BondPin::Cfg(CfgPin::Done)),
        None
    );
    assert_eq!(
        bond.pins.insert(cclk.into(), BondPin::Cfg(CfgPin::Cclk)),
        None
    );
    for &pin in gnd {
        assert_eq!(bond.pins.insert(pin.into(), BondPin::Gnd), None);
    }
    for &pin in vcc {
        assert_eq!(bond.pins.insert(pin.into(), BondPin::Vcc), None);
    }

    let len1d = match name {
        "pc44" => Some(44),
        "pc68" => Some(68),
        "pc84" => Some(84),
        "vq64" => Some(64),
        "vq100" | "tq100" => Some(100),
        "tq144" => Some(144),
        "tq176" => Some(176),
        "pq100" => Some(100),
        "pq160" => Some(160),
        "pq208" => Some(208),
        "cb100" | "cq100" => Some(100),
        "cb164" | "cq164" => Some(164),
        _ => None,
    };
    if let Some(len1d) = len1d {
        for i in 1..=len1d {
            bond.pins.entry(format!("P{i}")).or_insert(BondPin::Nc);
        }
        assert_eq!(bond.pins.len(), len1d);
    }

    match name {
        "pg84" => {
            for a in ["A", "B", "K", "L"] {
                for i in 1..=11 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["C", "D", "E", "F", "G", "H", "J"] {
                for i in (1..=2).chain(10..=11) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["C", "J"] {
                for i in 5..=7 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["E", "F", "G"] {
                for i in [3, 9] {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 84);
        }
        "pg132" | "pp132" => {
            for a in ["A", "B", "C", "M", "N", "P"] {
                for i in 1..=14 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K", "L"] {
                for i in (1..=3).chain(12..=14) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }

            assert_eq!(bond.pins.len(), 132);
        }
        "pp175" | "pg175" => {
            for i in 2..=16 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPin::Nc);
            }
            for a in ["B", "C", "D", "N", "P", "R", "T"] {
                for i in 1..=16 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["E", "F", "G", "H", "J", "K", "L", "M"] {
                for i in (1..=3).chain(14..=16) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 175);
        }
        "pg223" => {
            for i in 2..=18 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPin::Nc);
            }
            for a in ["B", "C", "D", "R", "T", "U", "V"] {
                for i in 1..=18 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["E", "F", "G", "H", "J", "K", "L", "M", "N", "P"] {
                for i in (1..=4).chain(15..=18) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 223);
        }
        _ => (),
    }

    let mut cfg_io = BTreeMap::new();
    if name == "pc68" {
        for (pin, fun) in [
            ("P2", SharedCfgPin::Addr(13)),
            ("P3", SharedCfgPin::Addr(6)),
            ("P4", SharedCfgPin::Addr(12)),
            ("P5", SharedCfgPin::Addr(7)),
            ("P6", SharedCfgPin::Addr(11)),
            ("P7", SharedCfgPin::Addr(8)),
            ("P8", SharedCfgPin::Addr(10)),
            ("P9", SharedCfgPin::Addr(9)),
            ("P27", SharedCfgPin::M2),
            ("P28", SharedCfgPin::Hdc),
            ("P30", SharedCfgPin::Ldc),
            ("P34", SharedCfgPin::InitB),
            ("P46", SharedCfgPin::Data(7)),
            ("P48", SharedCfgPin::Data(6)),
            ("P49", SharedCfgPin::Data(5)),
            ("P50", SharedCfgPin::Cs0B),
            ("P51", SharedCfgPin::Data(4)),
            ("P53", SharedCfgPin::Data(3)),
            ("P54", SharedCfgPin::Cs1B),
            ("P55", SharedCfgPin::Data(2)),
            ("P56", SharedCfgPin::Data(1)),
            ("P57", SharedCfgPin::RclkB),
            ("P58", SharedCfgPin::Data(0)),
            ("P59", SharedCfgPin::Dout),
            ("P61", SharedCfgPin::Addr(0)),
            ("P62", SharedCfgPin::Addr(1)),
            ("P63", SharedCfgPin::Addr(2)),
            ("P64", SharedCfgPin::Addr(3)),
            ("P65", SharedCfgPin::Addr(15)),
            ("P66", SharedCfgPin::Addr(4)),
            ("P67", SharedCfgPin::Addr(14)),
            ("P68", SharedCfgPin::Addr(5)),
        ] {
            let BondPin::Io(io) = bond.pins[pin] else {
                unreachable!()
            };
            cfg_io.insert(fun, io);
        }
    } else if name == "pc84" && endev.grid.columns < 14 {
        for (pin, fun) in [
            ("P2", SharedCfgPin::Addr(13)),
            ("P3", SharedCfgPin::Addr(6)),
            ("P4", SharedCfgPin::Addr(12)),
            ("P5", SharedCfgPin::Addr(7)),
            ("P8", SharedCfgPin::Addr(11)),
            ("P9", SharedCfgPin::Addr(8)),
            ("P10", SharedCfgPin::Addr(10)),
            ("P11", SharedCfgPin::Addr(9)),
            ("P33", SharedCfgPin::M2),
            ("P34", SharedCfgPin::Hdc),
            ("P36", SharedCfgPin::Ldc),
            ("P42", SharedCfgPin::InitB),
            ("P56", SharedCfgPin::Data(7)),
            ("P58", SharedCfgPin::Data(6)),
            ("P60", SharedCfgPin::Data(5)),
            ("P61", SharedCfgPin::Cs0B),
            ("P62", SharedCfgPin::Data(4)),
            ("P65", SharedCfgPin::Data(3)),
            ("P66", SharedCfgPin::Cs1B),
            ("P67", SharedCfgPin::Data(2)),
            ("P70", SharedCfgPin::Data(1)),
            ("P71", SharedCfgPin::RclkB),
            ("P72", SharedCfgPin::Data(0)),
            ("P73", SharedCfgPin::Dout),
            ("P75", SharedCfgPin::Addr(0)),
            ("P76", SharedCfgPin::Addr(1)),
            ("P77", SharedCfgPin::Addr(2)),
            ("P78", SharedCfgPin::Addr(3)),
            ("P81", SharedCfgPin::Addr(15)),
            ("P82", SharedCfgPin::Addr(4)),
            ("P83", SharedCfgPin::Addr(14)),
            ("P84", SharedCfgPin::Addr(5)),
        ] {
            let BondPin::Io(io) = bond.pins[pin] else {
                unreachable!()
            };
            cfg_io.insert(fun, io);
        }
    } else if name == "pg132" {
        for (pin, fun) in [
            ("G2", SharedCfgPin::Addr(13)),
            ("G1", SharedCfgPin::Addr(6)),
            ("F2", SharedCfgPin::Addr(12)),
            ("E1", SharedCfgPin::Addr(7)),
            ("D1", SharedCfgPin::Addr(11)),
            ("D2", SharedCfgPin::Addr(8)),
            ("B1", SharedCfgPin::Addr(10)),
            ("C2", SharedCfgPin::Addr(9)),
            ("C13", SharedCfgPin::M2),
            ("B14", SharedCfgPin::Hdc),
            ("D14", SharedCfgPin::Ldc),
            ("G14", SharedCfgPin::InitB),
            ("M12", SharedCfgPin::Data(7)),
            ("N11", SharedCfgPin::Data(6)),
            ("M9", SharedCfgPin::Data(5)),
            ("N9", SharedCfgPin::Cs0B),
            ("N8", SharedCfgPin::Data(4)),
            ("N7", SharedCfgPin::Data(3)),
            ("P6", SharedCfgPin::Cs1B),
            ("M6", SharedCfgPin::Data(2)),
            ("M5", SharedCfgPin::Data(1)),
            ("N4", SharedCfgPin::RclkB),
            ("N2", SharedCfgPin::Data(0)),
            ("M3", SharedCfgPin::Dout),
            ("M2", SharedCfgPin::Addr(0)),
            ("N1", SharedCfgPin::Addr(1)),
            ("L2", SharedCfgPin::Addr(2)),
            ("L1", SharedCfgPin::Addr(3)),
            ("K1", SharedCfgPin::Addr(15)),
            ("J2", SharedCfgPin::Addr(4)),
            ("H1", SharedCfgPin::Addr(14)),
            ("H2", SharedCfgPin::Addr(5)),
        ] {
            let BondPin::Io(io) = bond.pins[pin] else {
                unreachable!()
            };
            cfg_io.insert(fun, io);
        }
    } else if name == "pq160" {
        for (pin, fun) in [
            ("P141", SharedCfgPin::Addr(13)),
            ("P142", SharedCfgPin::Addr(6)),
            ("P147", SharedCfgPin::Addr(12)),
            ("P148", SharedCfgPin::Addr(7)),
            ("P151", SharedCfgPin::Addr(11)),
            ("P152", SharedCfgPin::Addr(8)),
            ("P155", SharedCfgPin::Addr(10)),
            ("P156", SharedCfgPin::Addr(9)),
            ("P44", SharedCfgPin::M2),
            ("P45", SharedCfgPin::Hdc),
            ("P49", SharedCfgPin::Ldc),
            ("P59", SharedCfgPin::InitB),
            ("P81", SharedCfgPin::Data(7)),
            ("P86", SharedCfgPin::Data(6)),
            ("P92", SharedCfgPin::Data(5)),
            ("P93", SharedCfgPin::Cs0B),
            ("P98", SharedCfgPin::Data(4)),
            ("P102", SharedCfgPin::Data(3)),
            ("P103", SharedCfgPin::Cs1B),
            ("P108", SharedCfgPin::Data(2)),
            ("P114", SharedCfgPin::Data(1)),
            ("P115", SharedCfgPin::RclkB),
            ("P119", SharedCfgPin::Data(0)),
            ("P120", SharedCfgPin::Dout),
            ("P124", SharedCfgPin::Addr(0)),
            ("P125", SharedCfgPin::Addr(1)),
            ("P128", SharedCfgPin::Addr(2)),
            ("P129", SharedCfgPin::Addr(3)),
            ("P132", SharedCfgPin::Addr(15)),
            ("P133", SharedCfgPin::Addr(4)),
            ("P136", SharedCfgPin::Addr(14)),
            ("P137", SharedCfgPin::Addr(5)),
        ] {
            let BondPin::Io(io) = bond.pins[pin] else {
                unreachable!()
            };
            cfg_io.insert(fun, io);
        }
    }

    (bond, cfg_io)
}
