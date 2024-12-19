use std::collections::{BTreeMap, BTreeSet};

use enum_map::EnumMap;
use prjcombine_int::{
    db::{BelInfo, BelPin, Dir, IntDb, NodeKind, NodeTileId, PinDir, TermInfo, TermKind, WireKind},
    grid::{ColId, DieId, EdgeIoCoord, LayerId, RowId},
};
use prjcombine_xact_data::die::Die;
use prjcombine_xact_naming::db::{NamingDb, NodeNaming};
use prjcombine_xc2000::{
    bond::{Bond, BondPin, CfgPin},
    grid::{Grid, GridKind, SharedCfgPin},
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
                dir: if wire.starts_with("IMUX") || wire.starts_with("IOCLK") {
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

    for name in [
        "SINGLE.H0",
        "SINGLE.H1",
        "SINGLE.H2",
        "SINGLE.H3",
        "SINGLE.H.B0",
        "SINGLE.H.B1",
        "SINGLE.H.B2",
        "SINGLE.H.B3",
        "SINGLE.H.T0",
        "SINGLE.H.T1",
        "SINGLE.H.T2",
        "SINGLE.H.T3",
    ] {
        let w0 = db.wires.insert(name.into(), WireKind::PipOut).0;
        let w1 = db
            .wires
            .insert(format!("{name}.E"), WireKind::PipBranch(Dir::W))
            .0;
        main_terms[Dir::W].wires.insert(w1, TermInfo::PassFar(w0));
    }

    for name in [
        "SINGLE.V0",
        "SINGLE.V1",
        "SINGLE.V2",
        "SINGLE.V3",
        "SINGLE.V4",
        "SINGLE.V.L0",
        "SINGLE.V.L1",
        "SINGLE.V.L2",
        "SINGLE.V.L3",
        "SINGLE.V.R0",
        "SINGLE.V.R1",
        "SINGLE.V.R2",
        "SINGLE.V.R3",
    ] {
        let w0 = db.wires.insert(name.into(), WireKind::PipOut).0;
        let w1 = db
            .wires
            .insert(format!("{name}.S"), WireKind::PipBranch(Dir::N))
            .0;
        main_terms[Dir::N].wires.insert(w1, TermInfo::PassFar(w0));
    }

    for name in ["LONG.H", "LONG.BH", "LONG.IO.B", "LONG.IO.T"] {
        let w = db
            .wires
            .insert(name.into(), WireKind::MultiBranch(Dir::W))
            .0;
        main_terms[Dir::W].wires.insert(w, TermInfo::PassFar(w));
    }
    for name in [
        "LONG.V0",
        "LONG.V1",
        "LONG.RV0",
        "LONG.RV1",
        "LONG.IO.L",
        "LONG.IO.R",
    ] {
        let w = db
            .wires
            .insert(name.into(), WireKind::MultiBranch(Dir::S))
            .0;
        main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
    }

    for name in ["GCLK", "ACLK", "IOCLK.B", "IOCLK.T", "IOCLK.L", "IOCLK.R"] {
        db.wires.insert(name.into(), WireKind::ClkOut(0));
    }

    for name in [
        "IMUX.CLB.A",
        "IMUX.CLB.B",
        "IMUX.CLB.C",
        "IMUX.CLB.D",
        "IMUX.CLB.K",
        "IMUX.BIOB0.O",
        "IMUX.BIOB0.T",
        "IMUX.BIOB1.O",
        "IMUX.BIOB1.T",
        "IMUX.TIOB0.O",
        "IMUX.TIOB0.T",
        "IMUX.TIOB1.O",
        "IMUX.TIOB1.T",
        "IMUX.LIOB0.O",
        "IMUX.LIOB0.T",
        "IMUX.LIOB1.O",
        "IMUX.LIOB1.T",
        "IMUX.RIOB0.O",
        "IMUX.RIOB0.T",
        "IMUX.RIOB1.O",
        "IMUX.RIOB1.T",
        "IMUX.BUFG",
    ] {
        let w = db.wires.insert(name.into(), WireKind::MuxOut).0;
        if name == "IMUX.CLB.D" {
            let wn = db
                .wires
                .insert(format!("{name}.N"), WireKind::Branch(Dir::S))
                .0;
            main_terms[Dir::S].wires.insert(wn, TermInfo::PassFar(w));
        }
    }

    for (name, dirs) in [
        ("OUT.CLB.X", &[Dir::E, Dir::S, Dir::N][..]),
        ("OUT.CLB.Y", &[Dir::E][..]),
        ("OUT.BIOB0.I", &[][..]),
        ("OUT.BIOB1.I", &[Dir::E][..]),
        ("OUT.TIOB0.I", &[][..]),
        ("OUT.TIOB1.I", &[Dir::E][..]),
        ("OUT.LIOB0.I", &[][..]),
        ("OUT.LIOB1.I", &[Dir::S][..]),
        ("OUT.RIOB0.I", &[][..]),
        ("OUT.RIOB1.I", &[Dir::S][..]),
        ("OUT.OSC", &[][..]),
    ] {
        let w = db.wires.insert(name.into(), WireKind::LogicOut).0;
        for &dir in dirs {
            let wo = db
                .wires
                .insert(format!("{name}.{dir}"), WireKind::Branch(!dir))
                .0;
            main_terms[!dir].wires.insert(wo, TermInfo::PassFar(w));
        }
    }

    for (dir, term) in main_terms {
        db.terms.insert(format!("MAIN.{dir}"), term);
    }

    for name in [
        "CLB", "CLB.L", "CLB.R", "CLB.B", "CLB.BL", "CLB.BR", "CLB.T", "CLB.TL", "CLB.TR",
        "CLB.ML", "CLB.MR", "CLB.BR1", "CLB.TR1",
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
                    ("D", "IMUX.CLB.D.N"),
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
                            ("I", format!("OUT.{bt}IOB{i}.I")),
                            ("K", format!("IOCLK.{bt}")),
                        ],
                    ),
                );
            }
        }

        if name.ends_with('L') || name.ends_with('R') {
            let lr = if name.ends_with('L') { 'L' } else { 'R' };
            for i in 0..2 {
                if i == 1 && name.starts_with("CLB.B") {
                    continue;
                }
                if i == 0 && (name.starts_with("CLB.T") || name.starts_with("CLB.M")) {
                    continue;
                }
                node.bels.insert(
                    format!("{lr}IOB{i}"),
                    bel_from_pins(
                        &db,
                        &[
                            ("O", format!("IMUX.{lr}IOB{i}.O")),
                            ("T", format!("IMUX.{lr}IOB{i}.T")),
                            ("I", format!("OUT.{lr}IOB{i}.I")),
                            ("K", format!("IOCLK.{lr}")),
                        ],
                    ),
                );
            }
        }

        if name == "CLB.TL" {
            node.bels.insert(
                "BUFG".into(),
                bel_from_pins(&db, &[("O", "GCLK"), ("I", "IMUX.BUFG")]),
            );
        }
        if name == "CLB.BR" {
            node.bels.insert(
                "BUFG".into(),
                bel_from_pins(&db, &[("O", "ACLK"), ("I", "IMUX.BUFG")]),
            );
            node.bels
                .insert("OSC".into(), bel_from_pins(&db, &[("O", "OUT.OSC")]));
        }

        db.nodes.insert(name.into(), node);
    }

    for name in ["BIDIH", "BIDIV"] {
        db.nodes.insert(
            name.into(),
            NodeKind {
                tiles: Default::default(),
                muxes: Default::default(),
                iris: Default::default(),
                intfs: Default::default(),
                bels: Default::default(),
            },
        );
    }

    db
}

pub fn make_grid(die: &Die) -> Grid {
    let pd_clb = die
        .primdefs
        .iter()
        .find(|(_, pd)| pd.name == "cle")
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
    let clb_x = Vec::from_iter(clb_x);
    let clb_y = Vec::from_iter(clb_y);
    let mut cols_bidi_raw = BTreeSet::new();
    let mut rows_bidi_raw = BTreeSet::new();
    let matrix = die.matrix.as_ref().unwrap();
    for x in 0..matrix.dim().0 {
        for y in 0..matrix.dim().1 {
            let cv = matrix[(x, y)] & 0xff;
            if cv == 0x11 {
                cols_bidi_raw.insert(x);
            }
            if cv == 0x15 {
                rows_bidi_raw.insert(y);
            }
        }
    }
    let cols_bidi = cols_bidi_raw
        .into_iter()
        .map(|x| ColId::from_idx(clb_x.binary_search(&x).unwrap_err()))
        .collect();
    let rows_bidi = rows_bidi_raw
        .into_iter()
        .map(|y| RowId::from_idx(clb_y.binary_search(&y).unwrap_err()))
        .collect();
    Grid {
        kind: GridKind::Xc2000,
        columns: clb_x.len(),
        rows: clb_y.len(),
        cols_bidi,
        rows_bidi,
        is_small: false,
        is_buff_large: false,
        cfg_io: Default::default(),
        unbonded_io: BTreeSet::new(),
    }
}

pub fn dump_grid(die: &Die) -> (Grid, IntDb, NamingDb) {
    let grid = make_grid(die);
    let mut intdb = make_intdb();
    let mut ndb = NamingDb::default();
    for name in intdb.nodes.keys() {
        ndb.node_namings.insert(name.clone(), NodeNaming::default());
    }
    for name in ["CLB.B1L", "CLB.B1R"] {
        ndb.node_namings.insert(name.into(), NodeNaming::default());
    }
    let bd_c8 = die
        .boxdefs
        .iter()
        .find(|(_, bd)| bd.name == "cross8")
        .unwrap()
        .0;
    let mut c8_x = BTreeSet::new();
    let mut c8_y = BTreeSet::new();
    for boxx in die.boxes.values() {
        if boxx.boxdef != bd_c8 {
            continue;
        }
        c8_x.insert(usize::from(boxx.bx));
        c8_y.insert(usize::from(boxx.by));
    }
    let c8_x = Vec::from_iter(c8_x);
    let c8_y = Vec::from_iter(c8_y);
    assert_eq!(c8_x.len(), grid.columns * 2 - 2);
    assert_eq!(c8_y.len(), grid.rows * 2 - 2);
    ndb.tile_widths.insert("L".into(), c8_x[0] - 2);
    ndb.tile_widths.insert("C".into(), c8_x[2] - c8_x[0]);
    ndb.tile_widths.insert(
        "R".into(),
        die.matrix.as_ref().unwrap().dim().0 - (c8_x[grid.columns * 2 - 4] - 2),
    );
    ndb.tile_heights.insert("B".into(), c8_y[1] + 2);
    ndb.tile_heights.insert("C".into(), c8_y[3] - c8_y[1]);
    ndb.tile_heights.insert(
        "T".into(),
        die.matrix.as_ref().unwrap().dim().1 - (c8_y[grid.rows * 2 - 3] + 2),
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
                        "CLB" | "BUFG" | "OSC" => {
                            let mut prim = extractor.grab_prim_a(&nnode.bels[bel][0]);
                            for pin in bel_info.pins.keys() {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                        }
                        "BIOB0" | "BIOB1" | "TIOB0" | "TIOB1" | "LIOB0" | "LIOB1" | "RIOB0"
                        | "RIOB1" => {
                            let mut prim = extractor.grab_prim_i(&nnode.bels[bel][0]);
                            for pin in ["I", "O", "T"] {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                            let k = prim.get_pin("K");
                            extractor.net_bel(k, nloc, bel, "K");
                            let (line, pip) = extractor.consume_one_bwd(k, nloc);
                            extractor.net_bel_int(line, nloc, bel, "K");
                            extractor.bel_pip(nnode.naming, bel, "K", pip);
                        }
                        _ => panic!("umm bel {key}?"),
                    }
                }
            }
        }
    }
    extractor.junk_prim_names.extend(
        ["VCC", "GND", "M0RT", "M1RD", "DP", "RST", "PWRDN", "CCLK"]
            .into_iter()
            .map(|x| x.to_string()),
    );

    // long verticals
    for col in die.cols() {
        let row = grid.row_bio() + 1;
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
            &["LONG.IO.L", "LONG.V0", "LONG.V1"][..]
        } else if col == grid.col_rio() {
            &["LONG.V0", "LONG.V1", "LONG.RV0", "LONG.RV1", "LONG.IO.R"][..]
        } else {
            &["LONG.V0", "LONG.V1"][..]
        };
        assert_eq!(nets.len(), wires.len());
        for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
            let wire = intdb.get_wire(wire);
            extractor.net_int(net, (die.die, (col, row), wire));
        }
    }
    // long horizontals
    for row in die.rows() {
        let col = grid.col_lio() + 1;
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
            &["LONG.IO.B", "LONG.BH", "LONG.H"][..]
        } else if row == grid.row_tio() {
            &["LONG.H", "LONG.IO.T"][..]
        } else {
            &["LONG.H"][..]
        };
        assert_eq!(nets.len(), wires.len());
        for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
            let wire = intdb.get_wire(wire);
            extractor.net_int(net, (die.die, (col, row), wire));
        }
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
                    "SINGLE.H.B3",
                    "SINGLE.H.B2",
                    "SINGLE.H.B1",
                    "SINGLE.H.B0",
                    "SINGLE.H3",
                    "SINGLE.H2",
                    "SINGLE.H1",
                    "SINGLE.H0",
                ][..]
            } else if row == grid.row_tio() {
                &["SINGLE.H.T3", "SINGLE.H.T2", "SINGLE.H.T1", "SINGLE.H.T0"][..]
            } else {
                &["SINGLE.H3", "SINGLE.H2", "SINGLE.H1", "SINGLE.H0"][..]
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
                &["SINGLE.V.L0", "SINGLE.V.L1", "SINGLE.V.L2", "SINGLE.V.L3"][..]
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

    let xlut = endev.col_x.map_values(|x| x.end);
    let ylut = endev.row_y.map_values(|y| y.end);
    for (box_id, boxx) in &extractor.die.boxes {
        let col = xlut.binary_search(&usize::from(boxx.bx)).unwrap_err();
        let row = ylut.binary_search(&usize::from(boxx.by)).unwrap_err();
        extractor.own_box(box_id, (die.die, col, row, LayerId::from_idx(0)));
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
        "pd48" => (
            "P7",
            "P17",
            "P18",
            "P31",
            "P32",
            "P42",
            &["P24"][..],
            &["P12"][..],
        ),
        "pc44" => (
            "P8",
            "P16",
            "P17",
            "P27",
            "P28",
            "P38",
            &["P1", "P23"][..],
            &["P12", "P33"][..],
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
        "pc84" => (
            "P12",
            "P31",
            "P32",
            "P54",
            "P55",
            "P74",
            &["P1", "P43"][..],
            &["P22", "P64"][..],
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
            &["P13", "P63"][..],
            &["P38", "P88"][..],
        ),
        "pg68" => (
            "B2",
            "J1",
            "K1",
            "K10",
            "K11",
            "B11",
            &["B6", "K6"][..],
            &["F2", "F10"][..],
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
    match name {
        "pd48" => assert_eq!(bond.pins.len(), 48),
        "pc44" => assert_eq!(bond.pins.len(), 44),
        "pc68" => assert_eq!(bond.pins.len(), 68),
        "pc84" => assert_eq!(bond.pins.len(), 84),
        "vq64" => assert_eq!(bond.pins.len(), 64),
        "vq100" | "tq100" => {
            for i in 1..=100 {
                bond.pins.entry(format!("P{i}")).or_insert(BondPin::Nc);
            }
        }
        "pg68" => assert_eq!(bond.pins.len(), 68),
        "pg84" => assert_eq!(bond.pins.len(), 84),
        _ => panic!("ummm {name}?"),
    };

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
            ("P41", SharedCfgPin::Data(7)),
            ("P42", SharedCfgPin::Data(6)),
            ("P48", SharedCfgPin::Data(5)),
            ("P50", SharedCfgPin::Data(4)),
            ("P51", SharedCfgPin::Data(3)),
            ("P54", SharedCfgPin::Data(2)),
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
    }

    (bond, cfg_io)
}
