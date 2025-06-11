use std::collections::HashSet;

use prjcombine_interconnect::{
    db::{TileCellId, TileClassWire, WireKind},
    grid::{ColId, NodeLoc, RowId},
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, OcdMode, xlat_bit, xlat_enum_ocd};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex::{bels, tslots};
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::FuzzCtx,
        int::{BaseIntPip, FuzzIntPip, resolve_int_pip},
        props::{
            BaseRaw, DynProp,
            bel::{BaseBelMode, BaseBelPin, FuzzBelMode},
            mutex::NodeMutexExclusive,
            relation::{Delta, Related},
        },
    },
};

#[derive(Clone, Debug)]
struct VirtexPinBramLv(TileClassWire);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinBramLv {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let wire = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.0.0], self.0.1))?;
        let mut nloc = nloc;
        nloc.2 = RowId::from_idx(1);
        nloc.3 = tslots::MAIN;
        for i in 0..12 {
            let wire_pin = (
                TileCellId::from_idx(0),
                backend.egrid.db.get_wire(&format!("LV.{i}")),
            );

            let resolved_pin = backend
                .egrid
                .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_pin.1))
                .unwrap();
            let wire_clk = (
                TileCellId::from_idx(0),
                backend.egrid.db.get_wire("IMUX.BRAM.CLKA"),
            );
            let resolved_clk = backend
                .egrid
                .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_clk.1))
                .unwrap();
            if resolved_pin == wire {
                let (tile, wt, wf) = resolve_int_pip(backend, nloc, wire_clk, wire_pin).unwrap();
                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                fuzzer = fuzzer.fuzz(Key::NodeMutex(resolved_clk), None, "EXCLUSIVE");
                return Some((fuzzer, false));
            }
        }
        panic!("UMM FAILED TO PIN BRAM LV");
    }
}

#[derive(Clone, Debug)]
struct VirtexPinLh(TileClassWire);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinLh {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let resolved_wire = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.0.0], self.0.1))?;
        let mut nloc = (nloc.0, ColId::from_idx(0), node.cells[self.0.0].1, nloc.3);
        let (layer, node) = backend
            .egrid
            .find_tile_loc(nloc.0, (nloc.1, nloc.2), |n| {
                backend.egrid.db.tile_classes.key(n.class) == "IO.L"
            })
            .unwrap();
        nloc.3 = layer;
        let node_data = &backend.egrid.db.tile_classes[node.class];
        for i in 0..12 {
            let wire_pin = (
                TileCellId::from_idx(0),
                backend.egrid.db.get_wire(&format!("LH.{i}")),
            );
            let resolved_pin = backend
                .egrid
                .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_pin.1))
                .unwrap();
            if resolved_pin != resolved_wire {
                continue;
            }
            for (&wire_out, mux_data) in &node_data.muxes {
                if mux_data.ins.contains(&wire_pin) {
                    // FOUND
                    let resolved_out = backend
                        .egrid
                        .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_out.1))
                        .unwrap();
                    let (tile, wt, wf) =
                        resolve_int_pip(backend, nloc, wire_out, wire_pin).unwrap();
                    fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                    fuzzer = fuzzer.fuzz(Key::NodeMutex(resolved_out), None, "EXCLUSIVE");
                    return Some((fuzzer, false));
                }
            }
        }
        unreachable!()
    }
}

#[derive(Clone, Debug)]
struct VirtexPinIoLh(TileClassWire);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinIoLh {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let resolved_wire = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.0.0], self.0.1))?;
        let mut nloc = (nloc.0, ColId::from_idx(0), node.cells[self.0.0].1, nloc.3);
        loop {
            if let Some((layer, _)) = backend.egrid.find_tile_loc(nloc.0, (nloc.1, nloc.2), |n| {
                matches!(
                    &backend.egrid.db.tile_classes.key(n.class)[..],
                    "IO.B" | "IO.T"
                )
            }) {
                nloc.3 = layer;
                for i in [0, 6] {
                    let wire_pin = (
                        TileCellId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("LH.{i}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_pin.1))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    // FOUND
                    let wire_buf = (
                        TileCellId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("LH.{i}.FAKE")),
                    );
                    let resolved_buf = backend
                        .egrid
                        .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_buf.1))
                        .unwrap();
                    let (tile, wt, wf) =
                        resolve_int_pip(backend, nloc, wire_buf, wire_pin).unwrap();
                    fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                    fuzzer = fuzzer.fuzz(Key::NodeMutex(resolved_buf), None, "EXCLUSIVE");
                    return Some((fuzzer, false));
                }
            }
            nloc.1 += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct VirtexPinHexH(TileClassWire);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinHexH {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let resolved_wire = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.0.0], self.0.1))?;
        let wire_name = backend.egrid.db.wires.key(self.0.1);
        let h = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[5..6].parse().unwrap();
        let mut nloc = (
            nloc.0,
            node.cells[self.0.0].0,
            node.cells[self.0.0].1,
            nloc.3,
        );
        if nloc.1.to_idx() >= 8 {
            nloc.1 -= 8;
        } else {
            nloc.1 = ColId::from_idx(0)
        };
        loop {
            if let Some((layer, node)) =
                backend.egrid.find_tile_loc(nloc.0, (nloc.1, nloc.2), |n| {
                    matches!(
                        &backend.egrid.db.tile_classes.key(n.class)[..],
                        "IO.L" | "IO.R" | "IO.B" | "IO.T" | "CLB" | "CNR.BR" | "CNR.TR"
                    )
                })
            {
                nloc.3 = layer;
                let node_data = &backend.egrid.db.tile_classes[node.class];
                for j in 0..=6 {
                    let wire_pin = (
                        TileCellId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("HEX.{h}{i}.{j}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_pin.1))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for (&wire_out, mux_data) in &node_data.muxes {
                        if mux_data.ins.contains(&wire_pin) {
                            let out_name = backend.egrid.db.wires.key(wire_out.1);
                            if out_name.starts_with("SINGLE")
                                || (out_name.starts_with("LV") && i >= 4)
                                || (out_name.starts_with("HEX.E")
                                    && backend.egrid.db.tile_classes.key(node.class) == "IO.L")
                                || (out_name.starts_with("HEX.W")
                                    && backend.egrid.db.tile_classes.key(node.class) == "IO.R")
                            {
                                // FOUND
                                let resolved_out = backend
                                    .egrid
                                    .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_out.1))
                                    .unwrap();
                                let (tile, wt, wf) =
                                    resolve_int_pip(backend, nloc, wire_out, wire_pin).unwrap();
                                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                                fuzzer =
                                    fuzzer.fuzz(Key::NodeMutex(resolved_out), None, "EXCLUSIVE");
                                return Some((fuzzer, false));
                            }
                        }
                    }
                }
            }
            nloc.1 += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct VirtexPinHexV(TileClassWire);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinHexV {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let resolved_wire = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.0.0], self.0.1))?;
        let wire_name = backend.egrid.db.wires.key(self.0.1);
        let v = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[5..6].parse().unwrap();
        let mut nloc = (
            nloc.0,
            node.cells[self.0.0].0,
            node.cells[self.0.0].1,
            nloc.3,
        );
        if nloc.2.to_idx() >= 6 {
            nloc.2 -= 6;
        } else {
            nloc.2 = RowId::from_idx(0)
        };
        loop {
            if let Some((layer, node)) =
                backend.egrid.find_tile_loc(nloc.0, (nloc.1, nloc.2), |n| {
                    matches!(
                        &backend.egrid.db.tile_classes.key(n.class)[..],
                        "IO.L" | "IO.R" | "CLB" | "IO.B" | "IO.T"
                    )
                })
            {
                nloc.3 = layer;
                let node_data = &backend.egrid.db.tile_classes[node.class];
                for j in 0..=6 {
                    let wire_pin = (
                        TileCellId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("HEX.{v}{i}.{j}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_pin.1))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for (&wire_out, mux_data) in &node_data.muxes {
                        if mux_data.ins.contains(&wire_pin) {
                            let out_name = backend.egrid.db.wires.key(wire_out.1);
                            if out_name.starts_with("SINGLE")
                                || (out_name.starts_with("HEX.N")
                                    && backend.egrid.db.tile_classes.key(node.class) == "IO.B")
                                || (out_name.starts_with("HEX.S")
                                    && backend.egrid.db.tile_classes.key(node.class) == "IO.T")
                            {
                                // FOUND
                                let resolved_out = backend
                                    .egrid
                                    .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_out.1))
                                    .unwrap();
                                let (tile, wt, wf) =
                                    resolve_int_pip(backend, nloc, wire_out, wire_pin).unwrap();
                                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                                fuzzer =
                                    fuzzer.fuzz(Key::NodeMutex(resolved_out), None, "EXCLUSIVE");
                                return Some((fuzzer, false));
                            }
                        }
                    }
                }
            }
            nloc.2 += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct VirtexDriveHexH(TileClassWire);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexDriveHexH {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let resolved_wire = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.0.0], self.0.1))?;
        let wire_name = backend.egrid.db.wires.key(self.0.1);
        let h = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[5..6].parse().unwrap();
        let mut nloc = (
            nloc.0,
            node.cells[self.0.0].0,
            node.cells[self.0.0].1,
            nloc.3,
        );
        if nloc.1.to_idx() >= 8 {
            nloc.1 -= 8;
        } else {
            nloc.1 = ColId::from_idx(0)
        };
        loop {
            if let Some((layer, node)) =
                backend.egrid.find_tile_loc(nloc.0, (nloc.1, nloc.2), |n| {
                    matches!(
                        &backend.egrid.db.tile_classes.key(n.class)[..],
                        "IO.L" | "IO.R" | "IO.B" | "IO.T" | "CLB"
                    )
                })
            {
                nloc.3 = layer;
                let node_data = &backend.egrid.db.tile_classes[node.class];
                for j in 0..=6 {
                    let wire_pin = (
                        TileCellId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("HEX.{h}{i}.{j}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_pin.1))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    if let Some(mux_data) = node_data.muxes.get(&wire_pin) {
                        for &inp in &mux_data.ins {
                            let inp_name = backend.egrid.db.wires.key(inp.1);
                            if inp_name.starts_with("OMUX")
                                || inp_name.starts_with("OUT")
                                || (h == 'E'
                                    && backend.egrid.db.tile_classes.key(node.class) == "IO.L"
                                    && inp_name.starts_with("HEX"))
                                || (h == 'W'
                                    && backend.egrid.db.tile_classes.key(node.class) == "IO.R"
                                    && inp_name.starts_with("HEX"))
                            {
                                // FOUND
                                let resolved_inp = backend
                                    .egrid
                                    .resolve_wire((nloc.0, (nloc.1, nloc.2), inp.1))
                                    .unwrap();
                                let (tile, wt, wf) =
                                    resolve_int_pip(backend, nloc, wire_pin, inp).unwrap();
                                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                                fuzzer =
                                    fuzzer.fuzz(Key::NodeMutex(resolved_inp), None, "EXCLUSIVE");
                                fuzzer =
                                    fuzzer.fuzz(Key::NodeMutex(resolved_pin), None, "EXCLUSIVE");
                                return Some((fuzzer, false));
                            }
                        }
                    }
                }
            }
            nloc.1 += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct VirtexDriveHexV(TileClassWire);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexDriveHexV {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let resolved_wire = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.0.0], self.0.1))?;
        let wire_name = backend.egrid.db.wires.key(self.0.1);
        let v = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[5..6].parse().unwrap();
        let mut nloc = (
            nloc.0,
            node.cells[self.0.0].0,
            node.cells[self.0.0].1,
            nloc.3,
        );
        if nloc.2.to_idx() >= 6 {
            nloc.2 -= 6;
        } else {
            nloc.2 = RowId::from_idx(0)
        };
        loop {
            if let Some((layer, node)) =
                backend.egrid.find_tile_loc(nloc.0, (nloc.1, nloc.2), |n| {
                    matches!(
                        &backend.egrid.db.tile_classes.key(n.class)[..],
                        "IO.L" | "IO.R" | "CLB" | "IO.B" | "IO.T"
                    )
                })
            {
                nloc.3 = layer;
                let node_data = &backend.egrid.db.tile_classes[node.class];
                for j in 0..=6 {
                    let wire_pin = (
                        TileCellId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("HEX.{v}{i}.{j}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire((nloc.0, (nloc.1, nloc.2), wire_pin.1))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    if let Some(mux_data) = node_data.muxes.get(&wire_pin) {
                        for &inp in &mux_data.ins {
                            let inp_name = backend.egrid.db.wires.key(inp.1);
                            if inp_name.starts_with("OMUX")
                                || inp_name.starts_with("OUT")
                                || (v == 'N'
                                    && backend.egrid.db.tile_classes.key(node.class) == "IO.B"
                                    && inp_name.starts_with("HEX"))
                                || (v == 'S'
                                    && backend.egrid.db.tile_classes.key(node.class) == "IO.T"
                                    && inp_name.starts_with("HEX"))
                            {
                                // FOUND
                                let resolved_inp = backend
                                    .egrid
                                    .resolve_wire((nloc.0, (nloc.1, nloc.2), inp.1))
                                    .unwrap();
                                let (tile, wt, wf) =
                                    resolve_int_pip(backend, nloc, wire_pin, inp).unwrap();
                                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                                fuzzer =
                                    fuzzer.fuzz(Key::NodeMutex(resolved_inp), None, "EXCLUSIVE");
                                fuzzer =
                                    fuzzer.fuzz(Key::NodeMutex(resolved_pin), None, "EXCLUSIVE");
                                return Some((fuzzer, false));
                            }
                        }
                    }
                }
            }
            nloc.2 += 1;
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (_, tile, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{:#}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            let out_name = intdb.wires.key(wire_to.1);
            if out_name.contains("OMUX") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(NodeMutexExclusive::new(wire_to))];
                if tile.starts_with("IO") {
                    for i in 0..4 {
                        props.push(Box::new(BaseBelMode::new(
                            bels::IO[i],
                            ["EMPTYIOB", "IOB", "IOB", "IOB"][i].into(),
                        )));
                        props.push(Box::new(BaseBelPin::new(bels::IO[i], "I".into())));
                    }
                    let clb_id = intdb.get_tile_class("CLB");
                    let clb = &intdb.tile_classes[clb_id];
                    let wire_name = intdb.wires.key(wire_to.1);
                    let clb_wire = if tile == "IO.L" {
                        format!("{wire_name}.W")
                    } else {
                        format!("{wire_name}.E")
                    };
                    let clb_wire = (TileCellId::from_idx(0), intdb.get_wire(&clb_wire));
                    let wire_pin = 'omux_pin: {
                        for (&wire, mux) in &clb.muxes {
                            if mux.ins.contains(&clb_wire) {
                                break 'omux_pin wire;
                            }
                        }
                        panic!("NO WAY TO PIN {tile} {mux_name}");
                    };
                    let relation = if tile == "IO.L" {
                        Delta::new(2, 0, "CLB")
                    } else {
                        Delta::new(-2, 0, "CLB")
                    };
                    props.push(Box::new(Related::new(
                        relation.clone(),
                        BaseIntPip::new(wire_pin, clb_wire),
                    )));
                    props.push(Box::new(Related::new(
                        relation,
                        NodeMutexExclusive::new(wire_pin),
                    )));
                } else {
                    let wire_pin = 'omux_pin: {
                        for (&wire, mux) in &node.muxes {
                            if mux.ins.contains(&wire_to) {
                                break 'omux_pin wire;
                            }
                        }
                        panic!("NO WAY TO PIN {tile} {mux_name}");
                    };
                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_to)));
                    props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                }
                for &wire_from in &mux.ins {
                    let in_name = if node.cells.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let mut builder = ctx
                        .build()
                        .test_manual("INT", &mux_name, in_name)
                        .prop(FuzzIntPip::new(wire_to, wire_from));
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.starts_with("BRAM.QUAD") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(NodeMutexExclusive::new(wire_to))];

                let (is_s, wire_to_root) = if let Some(root_name) = out_name.strip_suffix(".S") {
                    (true, (wire_to.0, intdb.get_wire(root_name)))
                } else {
                    (false, wire_to)
                };
                let wire_pin = 'quad_dst_pin: {
                    for (&wire_pin, mux) in &node.muxes {
                        let wire_pin_name = intdb.wires.key(wire_pin.1);
                        if mux.ins.contains(&wire_to_root)
                            && (wire_pin_name.starts_with("IMUX")
                                || wire_pin_name.starts_with("HEX"))
                        {
                            break 'quad_dst_pin wire_pin;
                        }
                    }
                    panic!("NO WAY TO PIN {tile} {mux_name}");
                };
                if !is_s {
                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_to)));
                    props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                } else {
                    let related = Delta::new(0, 4, tile);
                    props.push(Box::new(Related::new(
                        related.clone(),
                        BaseIntPip::new(wire_pin, wire_to_root),
                    )));
                    props.push(Box::new(Related::new(
                        related,
                        NodeMutexExclusive::new(wire_pin),
                    )));
                }
                if !out_name.starts_with("BRAM.QUAD.DOUT") {
                    // pin every input
                    let mut pins = HashSet::new();
                    for &wire_from in &mux.ins {
                        let in_wire_name = intdb.wires.key(wire_from.1);
                        'quad_src_all_pin: {
                            if in_wire_name.starts_with("SINGLE") {
                                let wire_buf = format!("{in_wire_name}.BUF");
                                let wire_buf = (TileCellId::from_idx(0), intdb.get_wire(&wire_buf));
                                let related = Delta::new(
                                    -1,
                                    wire_from.0.to_idx() as i32 - 4,
                                    if tile == "LBRAM" { "IO.L" } else { "CLB" },
                                );
                                props.push(Box::new(Related::new(
                                    related.clone(),
                                    BaseIntPip::new(
                                        wire_buf,
                                        (TileCellId::from_idx(0), wire_from.1),
                                    ),
                                )));
                                props.push(Box::new(Related::new(
                                    related,
                                    NodeMutexExclusive::new(wire_buf),
                                )));
                                props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                                break 'quad_src_all_pin;
                            } else if in_wire_name.starts_with("HEX") {
                                for (&wire_pin, mux) in &node.muxes {
                                    if wire_pin != wire_to
                                        && !pins.contains(&wire_pin)
                                        && mux.ins.contains(&wire_from)
                                    {
                                        props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                        props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                                        props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                        pins.insert(wire_pin);
                                        break 'quad_src_all_pin;
                                    }
                                }
                            } else {
                                break 'quad_src_all_pin;
                            }
                            panic!("NO WAY TO PIN {tile} {mux_name} {in_wire_name}");
                        }
                    }
                }
                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    let in_name = if node.cells.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, in_wire_name)
                    };
                    let mut props = props.clone();
                    if in_wire_name.starts_with("BRAM.QUAD") {
                        'quad_src_pin: {
                            let (is_s, wire_from_root) =
                                if let Some(root_name) = in_wire_name.strip_suffix(".S") {
                                    (true, (wire_from.0, intdb.get_wire(root_name)))
                                } else {
                                    (false, wire_from)
                                };

                            let from_mux = &node.muxes[&wire_from_root];
                            for &wire_pin in &from_mux.ins {
                                let wire_pin_name = intdb.wires.key(wire_pin.1);
                                if intdb.wires.key(wire_pin.1).starts_with("HEX")
                                    || wire_pin_name.starts_with("OUT")
                                {
                                    if !is_s {
                                        props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                        props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                                        props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                    } else {
                                        let related = Delta::new(0, 4, tile);
                                        props.push(Box::new(Related::new(
                                            related.clone(),
                                            BaseIntPip::new(wire_from_root, wire_pin),
                                        )));
                                        props.push(Box::new(Related::new(
                                            related.clone(),
                                            NodeMutexExclusive::new(wire_pin),
                                        )));
                                        props.push(Box::new(Related::new(
                                            related,
                                            NodeMutexExclusive::new(wire_from_root),
                                        )));
                                    }
                                    break 'quad_src_pin;
                                }
                            }
                            panic!("NO WAY TO PIN {tile} {mux_name} {in_name}");
                        }
                    }
                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    let mut builder = ctx.build().test_manual("INT", &mux_name, &in_name);
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.starts_with("SINGLE") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(NodeMutexExclusive::new(wire_to))];

                let wire_buf = format!("{out_name}.BUF");
                let wire_buf = (TileCellId::from_idx(0), intdb.get_wire(&wire_buf));
                if !tile.contains("BRAM") {
                    props.push(Box::new(BaseIntPip::new(wire_buf, wire_to)));
                    props.push(Box::new(NodeMutexExclusive::new(wire_buf)));
                } else {
                    let related = Delta::new(
                        -1,
                        wire_to.0.to_idx() as i32 - 4,
                        if tile == "LBRAM" { "IO.L" } else { "CLB" },
                    );
                    props.push(Box::new(Related::new(
                        related.clone(),
                        BaseIntPip::new(wire_buf, (TileCellId::from_idx(0), wire_to.1)),
                    )));
                    props.push(Box::new(Related::new(
                        related,
                        NodeMutexExclusive::new(wire_buf),
                    )));
                }
                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    let in_name = if node.cells.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, in_wire_name)
                    };

                    let mut props = props.clone();
                    'single_pin: {
                        if in_wire_name.starts_with("SINGLE") {
                            let from_mux = &node.muxes[&wire_from];
                            for &wire_pin in &from_mux.ins {
                                let wire_pin_name = intdb.wires.key(wire_pin.1);
                                if intdb.wires.key(wire_pin.1).starts_with("HEX")
                                    || wire_pin_name.starts_with("OMUX")
                                    || wire_pin_name.starts_with("BRAM.QUAD.DOUT")
                                {
                                    props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                    props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                                    props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                    break 'single_pin;
                                }
                            }
                        } else {
                            for (&wire_pin, mux) in &node.muxes {
                                let wire_pin_name = intdb.wires.key(wire_pin.1);
                                if wire_pin != wire_to
                                    && mux.ins.contains(&wire_from)
                                    && wire_pin_name.starts_with("SINGLE")
                                {
                                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                    props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                    break 'single_pin;
                                }
                            }
                        }
                        panic!("NO WAY TO PIN {tile} {mux_name} {in_name}");
                    };

                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    let mut builder = ctx.build().test_manual("INT", &mux_name, &in_name);
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.starts_with("LH")
                || out_name.starts_with("LV")
                || out_name.starts_with("HEX")
            {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(NodeMutexExclusive::new(wire_to))];

                if out_name.starts_with("LH") && matches!(&tile[..], "IO.B" | "IO.T") {
                    let wire_buf = format!("{out_name}.FAKE");
                    let wire_buf = (TileCellId::from_idx(0), intdb.get_wire(&wire_buf));
                    props.push(Box::new(BaseIntPip::new(wire_buf, wire_to)));
                    props.push(Box::new(NodeMutexExclusive::new(wire_buf)));
                } else if out_name.starts_with("LV") && matches!(&tile[..], "BRAM_BOT" | "BRAM_TOP")
                {
                    props.push(Box::new(VirtexPinBramLv(wire_to)));
                } else if out_name.starts_with("LH") && tile.ends_with("BRAM") {
                    props.push(Box::new(VirtexPinLh(wire_to)));
                } else if out_name.starts_with("LH") && tile.starts_with("CLK") {
                    props.push(Box::new(VirtexPinIoLh(wire_to)));
                } else if out_name.starts_with("HEX.H")
                    || out_name.starts_with("HEX.E")
                    || out_name.starts_with("HEX.W")
                {
                    props.push(Box::new(VirtexPinHexH(wire_to)));
                } else if out_name.starts_with("HEX.V")
                    || out_name.starts_with("HEX.S")
                    || out_name.starts_with("HEX.N")
                {
                    props.push(Box::new(VirtexPinHexV(wire_to)));
                } else {
                    'll_pin: {
                        for (&wire_pin, mux) in &node.muxes {
                            let wire_pin_name = intdb.wires.key(wire_pin.1);
                            if mux.ins.contains(&wire_to)
                                && (wire_pin_name.starts_with("HEX")
                                    || wire_pin_name.starts_with("IMUX.BRAM"))
                            {
                                props.push(Box::new(BaseIntPip::new(wire_pin, wire_to)));
                                props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                break 'll_pin;
                            }
                        }
                        println!("NO WAY TO PIN {tile} {mux_name}");
                    }
                }

                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    'll_src_pin: {
                        if let Some(wire_unbuf) = in_wire_name.strip_suffix(".BUF") {
                            let wire_unbuf = (TileCellId::from_idx(0), intdb.get_wire(wire_unbuf));
                            props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                            props.push(Box::new(NodeMutexExclusive::new(wire_unbuf)));
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("OMUX")
                            || in_wire_name.starts_with("BRAM.QUAD.DOUT")
                        {
                            let from_mux = &node.muxes[&wire_from];
                            for &wire_pin in &from_mux.ins {
                                if intdb.wires.key(wire_pin.1).starts_with("OUT") {
                                    props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                    props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                                    props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                    break 'll_src_pin;
                                }
                            }
                        } else if in_wire_name.starts_with("HEX") {
                            if in_wire_name.starts_with("HEX.E")
                                || in_wire_name.starts_with("HEX.W")
                                || in_wire_name.starts_with("HEX.H")
                            {
                                props.push(Box::new(VirtexDriveHexH(wire_from)));
                            } else {
                                props.push(Box::new(VirtexDriveHexV(wire_from)));
                            }
                            break 'll_src_pin;
                        } else if let Some(wire_unbuf) = in_wire_name.strip_suffix(".FAKE") {
                            let wire_unbuf = (TileCellId::from_idx(0), intdb.get_wire(wire_unbuf));
                            props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                            props.push(Box::new(NodeMutexExclusive::new(wire_unbuf)));
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("LH") && tile.starts_with("CNR") {
                            // it's fine.
                            props.push(Box::new(VirtexPinIoLh(wire_from)));
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("LH") || in_wire_name.starts_with("LV") {
                            let from_mux = &node.muxes[&wire_from];
                            for &wire_pin in &from_mux.ins {
                                if intdb.wires.key(wire_pin.1).starts_with("OMUX")
                                    || intdb.wires.key(wire_pin.1).starts_with("OUT")
                                    || (intdb.wires.key(wire_pin.1).starts_with("HEX")
                                        && tile.starts_with("CNR"))
                                {
                                    props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                    props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                                    props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                    break 'll_src_pin;
                                }
                            }
                        } else if in_wire_name.starts_with("SINGLE") {
                            let wire_buf = format!("{in_wire_name}.BUF");
                            let wire_buf = (TileCellId::from_idx(0), intdb.get_wire(&wire_buf));
                            if tile.ends_with("BRAM") {
                                let related = Delta::new(
                                    -1,
                                    wire_from.0.to_idx() as i32 - 4,
                                    if tile == "LBRAM" { "IO.L" } else { "CLB" },
                                );
                                props.push(Box::new(Related::new(
                                    related.clone(),
                                    BaseIntPip::new(
                                        wire_buf,
                                        (TileCellId::from_idx(0), wire_from.1),
                                    ),
                                )));
                                props.push(Box::new(Related::new(
                                    related,
                                    NodeMutexExclusive::new(wire_buf),
                                )));
                                props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                            } else {
                                props.push(Box::new(BaseIntPip::new(wire_buf, wire_from)));
                                props.push(Box::new(NodeMutexExclusive::new(wire_buf)));
                                props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                            }
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("OUT.IO") {
                            for i in 0..4 {
                                props.push(Box::new(BaseBelMode::new(
                                    bels::IO[i],
                                    [
                                        "EMPTYIOB",
                                        "IOB",
                                        "IOB",
                                        if tile == "IO.L" || tile == "IO.R" {
                                            "IOB"
                                        } else {
                                            "EMPTYIOB"
                                        },
                                    ][i]
                                        .into(),
                                )));
                                props.push(Box::new(BaseBelPin::new(bels::IO[i], "I".into())));
                                props.push(Box::new(BaseBelPin::new(bels::IO[i], "IQ".into())));
                            }
                            break 'll_src_pin;
                        } else if let Some(pin) = in_wire_name.strip_prefix("OUT.BSCAN.") {
                            props.push(Box::new(BaseBelMode::new(bels::BSCAN, "BSCAN".into())));
                            props.push(Box::new(BaseBelPin::new(bels::BSCAN, pin.into())));
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("CLK.OUT")
                            || in_wire_name.starts_with("DLL.OUT")
                            || in_wire_name == "PCI_CE"
                        {
                            // already ok
                            break 'll_src_pin;
                        }
                        panic!("NO WAY TO PIN {tile} {mux_name} {in_wire_name}");
                    };
                }

                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    let in_name = if node.cells.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, in_wire_name)
                    };

                    let mut props = props.clone();
                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));

                    let mut builder = ctx.build().test_manual("INT", &mux_name, &in_name);
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.contains("IMUX") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(NodeMutexExclusive::new(wire_to))];
                if let Some(pin) = out_name.strip_prefix("IMUX.STARTUP.") {
                    props.push(Box::new(BaseBelMode::new(bels::STARTUP, "STARTUP".into())));
                    props.push(Box::new(BaseBelPin::new(bels::STARTUP, pin.into())));
                }
                let mut alt_out_wire = None;
                if out_name.starts_with("DLL.IMUX") {
                    for i in 0..4 {
                        for ps in ["", "P", "S"] {
                            props.push(Box::new(BaseRaw::new(
                                Key::GlobalOpt(format!("IDLL{i}{ps}FB2X")),
                                "0".into(),
                            )))
                        }
                    }
                    if out_name == "DLL.IMUX.CLKIN" {
                        alt_out_wire = Some((
                            TileCellId::from_idx(0),
                            backend.egrid.db.get_wire("DLL.IMUX.CLKFB"),
                        ));
                    }
                    if out_name == "DLL.IMUX.CLKFB" {
                        alt_out_wire = Some((
                            TileCellId::from_idx(0),
                            backend.egrid.db.get_wire("DLL.IMUX.CLKIN"),
                        ));
                    }
                }
                if let Some(alt_out) = alt_out_wire {
                    props.push(Box::new(NodeMutexExclusive::new(alt_out)));
                }
                if out_name.starts_with("CLK.IMUX.BUFGCE.CLK") {
                    props.push(Box::new(if out_name.ends_with("1") {
                        FuzzBelMode::new(bels::BUFG1, "".into(), "GCLK".into())
                    } else {
                        FuzzBelMode::new(bels::BUFG0, "".into(), "GCLK".into())
                    }));
                }
                if (out_name.starts_with("IMUX.TBUF") && out_name.ends_with("I"))
                    || out_name.starts_with("IMUX.BRAM.DI")
                {
                    for &wire_from in &mux.ins {
                        let in_wire_name = intdb.wires.key(wire_from.1);
                        'imux_pin: {
                            if let Some(wire_unbuf) = in_wire_name.strip_suffix(".BUF") {
                                let wire_unbuf =
                                    (TileCellId::from_idx(0), intdb.get_wire(wire_unbuf));
                                props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                                props.push(Box::new(NodeMutexExclusive::new(wire_unbuf)));
                                break 'imux_pin;
                            } else if out_name.starts_with("IMUX.BRAM.DI") {
                                let from_mux = &node.muxes[&wire_from];
                                for &wire_pin in &from_mux.ins {
                                    if intdb.wires.key(wire_pin.1).starts_with("HEX") {
                                        props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                        props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                                        props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                        break 'imux_pin;
                                    }
                                }
                            } else {
                                for (&wire_pin, mux) in &node.muxes {
                                    if wire_pin != wire_to && mux.ins.contains(&wire_from) {
                                        if let Some(from_mux) = node.muxes.get(&wire_from) {
                                            if from_mux.ins.contains(&wire_pin) {
                                                continue;
                                            }
                                        }
                                        props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                        props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                        break 'imux_pin;
                                    }
                                }
                            }
                            panic!("NO WAY TO PIN {tile} {mux_name} {in_wire_name}");
                        };
                    }
                }
                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    let in_name = if node.cells.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, in_wire_name)
                    };

                    let mut props = props.clone();
                    'imux_pin: {
                        if in_wire_name.starts_with("GCLK") || in_wire_name.ends_with("BUF") {
                            // no need to pin
                            break 'imux_pin;
                        } else if out_name.starts_with("IMUX.TBUF") && out_name.ends_with("I") {
                            // already pinned above
                            break 'imux_pin;
                        } else if out_name == "PCI.IMUX.I3" {
                            let wire_buf = format!("{in_wire_name}.BUF");
                            let wire_buf = (TileCellId::from_idx(0), intdb.get_wire(&wire_buf));
                            let related =
                                Delta::new(0, 0, if tile == "CLKL" { "IO.L" } else { "IO.R" });
                            props.push(Box::new(Related::new(
                                related.clone(),
                                BaseIntPip::new(wire_buf, wire_from),
                            )));
                            props.push(Box::new(Related::new(
                                related,
                                NodeMutexExclusive::new(wire_buf),
                            )));
                            break 'imux_pin;
                        } else if out_name.starts_with("DLL.IMUX") {
                            if in_wire_name.starts_with("HEX") {
                                props.push(Box::new(VirtexDriveHexH(wire_from)));
                            } else {
                                // don't bother pinning.
                            }
                            break 'imux_pin;
                        } else {
                            for (&wire_pin, mux) in &node.muxes {
                                if wire_pin != wire_to && mux.ins.contains(&wire_from) {
                                    if let Some(from_mux) = node.muxes.get(&wire_from) {
                                        if from_mux.ins.contains(&wire_pin) {
                                            continue;
                                        }
                                    }
                                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                    props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                    break 'imux_pin;
                                }
                            }
                            // try to drive it instead.
                            let from_mux = &node.muxes[&wire_from];
                            for &wire_pin in &from_mux.ins {
                                if let Some(pin_mux) = node.muxes.get(&wire_pin) {
                                    if pin_mux.ins.contains(&wire_from) {
                                        continue;
                                    }
                                }
                                props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                props.push(Box::new(NodeMutexExclusive::new(wire_from)));
                                props.push(Box::new(NodeMutexExclusive::new(wire_pin)));
                                break 'imux_pin;
                            }
                        }
                        panic!("NO WAY TO PIN {tile} {mux_name} {in_name}");
                    };

                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    if let Some(alt_out) = alt_out_wire {
                        if in_wire_name.starts_with("CLK.OUT") {
                            let mut builder = ctx.build().test_manual(
                                "INT",
                                &mux_name,
                                format!("{in_name}.NOALT"),
                            );
                            for prop in &props {
                                builder = builder.prop_box(prop.clone());
                            }
                            builder.commit();
                            props.push(Box::new(BaseIntPip::new(alt_out, wire_from)));
                        }
                    }

                    let mut builder = ctx.build().test_manual("INT", &mux_name, &in_name);
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else {
                panic!("UNHANDLED MUX: {tile} {mux_name}");
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, tile, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.tile_index[node_kind].is_empty() {
            continue;
        }

        for (&wire_to, mux) in &node.muxes {
            if matches!(
                intdb.wires[wire_to.1],
                WireKind::PipOut | WireKind::PipBranch(_)
            ) {
                let out_name = if node.cells.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{:#}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                for &wire_from in &mux.ins {
                    let in_name = if node.cells.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let diff = ctx
                        .state
                        .get_diff(tile, "INT", format!("MUX.{out_name}"), &in_name);
                    if in_name.starts_with("OUT.IO0")
                        || matches!(&tile[..], "IO.B" | "IO.T") && in_name.starts_with("OUT.IO3")
                    {
                        diff.assert_empty();
                        continue;
                    }
                    if diff.bits.is_empty() {
                        println!("UMM {out_name} {in_name} PASS IS EMPTY");
                        continue;
                    }
                    let item = xlat_bit(diff);
                    let mut is_bidi = false;
                    if let Some(omux) = node.muxes.get(&wire_from) {
                        if omux.ins.contains(&wire_to) {
                            is_bidi = true;
                        }
                    }
                    let name = if !is_bidi {
                        format!("PASS.{out_name}.{in_name}")
                    } else if wire_from < wire_to {
                        format!("BIPASS.{in_name}.{out_name}")
                    } else {
                        format!("BIPASS.{out_name}.{in_name}")
                    };
                    ctx.tiledb.insert(tile, "INT", name, item);
                }
            } else {
                let out_name = if node.cells.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{:#}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                let mux_name = format!("MUX.{out_name}");

                let mut inps = vec![];
                let mut got_empty = false;
                for &wire_from in &mux.ins {
                    let in_name = if node.cells.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{:#}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let mut diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    if mux_name.contains("DLL.IMUX") && in_name.contains("CLK.OUT") {
                        let noalt_diff =
                            ctx.state
                                .get_diff(tile, "INT", &mux_name, format!("{in_name}.NOALT"));
                        let (alt, noalt, common) = Diff::split(diff, noalt_diff);
                        if mux_name.contains("CLKIN") {
                            ctx.tiledb.insert(tile, "DLL", "CLKIN_PAD", xlat_bit(noalt));
                            ctx.tiledb.insert(tile, "DLL", "CLKFB_PAD", xlat_bit(!alt));
                        } else {
                            ctx.tiledb.insert(tile, "DLL", "CLKFB_PAD", xlat_bit(noalt));
                            ctx.tiledb.insert(tile, "DLL", "CLKIN_PAD", xlat_bit(!alt));
                        }
                        diff = common;
                    }
                    if in_name.starts_with("OUT.IO0")
                        || (in_name.starts_with("OUT.IO3") && matches!(&tile[..], "IO.B" | "IO.T"))
                    {
                        diff.assert_empty();
                    } else if (out_name.contains("BRAM.QUAD") && in_name.contains("BRAM.QUAD"))
                        || out_name.contains("BRAM.QUAD.DOUT")
                        || (out_name.contains("HEX.H") && in_name == "PCI_CE")
                        || (tile.starts_with("CNR") && out_name.contains("LV"))
                        || (tile.starts_with("BRAM_") && out_name.contains("LV"))
                    {
                        if diff.bits.is_empty() {
                            println!("UMM {out_name} {in_name} BUF IS EMPTY");
                            continue;
                        }
                        ctx.tiledb.insert(
                            tile,
                            "INT",
                            format!("BUF.{out_name}.{in_name}"),
                            xlat_bit(diff),
                        );
                    } else {
                        if diff.bits.is_empty() {
                            got_empty = true;
                        }
                        inps.push((in_name.to_string(), diff));
                    }
                }
                if inps.is_empty() {
                    continue;
                }
                if out_name.contains("BRAM.QUAD")
                    || out_name.contains("LV")
                    || out_name.contains("LH")
                    || out_name.contains("HEX.H")
                    || out_name.contains("HEX.V")
                {
                    let mut drive_bits: HashSet<_> = inps[0].1.bits.keys().copied().collect();
                    for (_, diff) in &inps {
                        drive_bits.retain(|bit| diff.bits.contains_key(bit));
                    }
                    if drive_bits.len() > 1 {
                        if tile.starts_with("CNR") {
                            // sigh. I give up. those are obtained from looking at left-hand
                            // corners with easier-to-disambiguate muxes, and correlating with
                            // bitstream geometry in right-hand corners. also confirmed by some
                            // manual bitgen tests.
                            drive_bits.retain(|bit| matches!(bit.frame % 6, 0 | 5));
                        } else {
                            let btile = match &tile[..] {
                                "IO.L" => edev.btile_main(edev.chip.col_w(), RowId::from_idx(1)),
                                "IO.R" => edev.btile_main(edev.chip.col_e(), RowId::from_idx(1)),
                                _ => panic!(
                                    "CAN'T FIGURE OUT DRIVE {tile} {mux_name} {drive_bits:?} {inps:?}"
                                ),
                            };
                            drive_bits.retain(|bit| {
                                !ctx.empty_bs
                                    .get_bit(btile.xlat_pos_fwd((bit.frame, bit.bit)))
                            });
                        }
                    }
                    if drive_bits.len() != 1 {
                        panic!("FUCKY WACKY {tile} {out_name} {inps:?}");
                    }
                    let drive = Diff {
                        bits: drive_bits
                            .into_iter()
                            .map(|bit| (bit, inps[0].1.bits[&bit]))
                            .collect(),
                    };
                    for (_, diff) in &mut inps {
                        *diff = diff.combine(&!&drive);
                    }
                    if inps.iter().all(|(_, diff)| !diff.bits.is_empty()) {
                        inps.push(("NONE".to_string(), Diff::default()));
                    }
                    let item = xlat_enum_ocd(inps, OcdMode::Mux);
                    ctx.tiledb.insert(tile, "INT", mux_name, item);
                    ctx.tiledb
                        .insert(tile, "INT", format!("DRIVE.{out_name}"), xlat_bit(drive));
                } else {
                    if !got_empty {
                        inps.push(("NONE".to_string(), Diff::default()));
                    }
                    let item = xlat_enum_ocd(inps, OcdMode::Mux);
                    if item.bits.is_empty() {
                        if mux_name == "MUX.IMUX.IO0.T" {
                            // empty on Virtex E?
                            continue;
                        }
                        if mux_name.starts_with("MUX.HEX.S") && tile == "IO.T"
                            || mux_name.starts_with("MUX.HEX.N") && tile == "IO.B"
                            || mux_name.starts_with("MUX.HEX.E") && tile == "IO.L"
                            || mux_name.starts_with("MUX.HEX.W") && tile == "IO.R"
                        {
                            continue;
                        }
                        println!("UMMM MUX {tile} {mux_name} is empty");
                    }
                    ctx.tiledb.insert(tile, "INT", mux_name, item);
                }
            }
        }
    }
}
