use prjcombine_interconnect::{
    db::{TileCellId, TileClassWire},
    grid::NodeLoc,
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, OcdMode, xlat_enum_ocd};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_re_xilinx_naming::db::RawTileId;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
};

use super::{
    fbuild::FuzzCtx,
    props::{
        BaseRaw, DynProp,
        mutex::{IntMutex, NodeMutexExclusive, NodeMutexShared, RowMutex},
    },
};

#[derive(Clone, Debug)]
pub struct NodeIntDistinct {
    wire_a: TileClassWire,
    wire_b: TileClassWire,
}

impl NodeIntDistinct {
    pub fn new(wire_a: TileClassWire, wire_b: TileClassWire) -> Self {
        Self { wire_a, wire_b }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for NodeIntDistinct {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(nloc);
        let a = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.wire_a.0], self.wire_a.1))?;
        let b = backend
            .egrid
            .resolve_wire((nloc.0, node.cells[self.wire_b.0], self.wire_b.1))?;
        if a == b {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct NodeIntDstFilter {
    wire: TileClassWire,
}

impl NodeIntDstFilter {
    pub fn new(wire: TileClassWire) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for NodeIntDstFilter {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let intdb = backend.egrid.db;
        let ndb = backend.ngrid.db;
        let wire_name = intdb.wires.key(self.wire.1);
        match backend.edev {
            ExpandedDevice::Virtex2(edev) => {
                let node = backend.egrid.tile(nloc);
                let nnode = &backend.ngrid.tiles[&nloc];
                if backend
                    .egrid
                    .db
                    .tile_classes
                    .key(node.class)
                    .starts_with("INT.BRAM")
                {
                    let mut tgt = None;
                    for i in 0..4 {
                        if let Some(bram_node) =
                            backend
                                .egrid
                                .find_tile(nloc.0, (nloc.1, nloc.2 - i), |node| {
                                    intdb.tile_classes.key(node.class).starts_with("BRAM")
                                        || intdb.tile_classes.key(node.class) == "DSP"
                                })
                        {
                            tgt = Some((bram_node, i));
                            break;
                        }
                    }
                    let (bram_node, idx) = tgt.unwrap();
                    let node_tile = TileCellId::from_idx(idx);
                    let bram_node_kind = &intdb.tile_classes[bram_node.class];
                    if (edev.chip.kind.is_virtex2()
                        || edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3)
                        && (wire_name.starts_with("IMUX.CLK")
                            || wire_name.starts_with("IMUX.SR")
                            || wire_name.starts_with("IMUX.CE")
                            || wire_name.starts_with("IMUX.TS"))
                    {
                        let mut found = false;
                        for bel in bram_node_kind.bels.values() {
                            for pin in bel.pins.values() {
                                if pin.wires.contains(&(node_tile, self.wire.1)) {
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            return None;
                        }
                    }
                }
                if backend.egrid.db.tile_classes.key(node.class) == "INT.IOI.S3E"
                    || backend.egrid.db.tile_classes.key(node.class) == "INT.IOI.S3A.LR"
                {
                    if matches!(
                        &wire_name[..],
                        "IMUX.DATA3"
                            | "IMUX.DATA7"
                            | "IMUX.DATA11"
                            | "IMUX.DATA15"
                            | "IMUX.DATA19"
                            | "IMUX.DATA23"
                            | "IMUX.DATA27"
                            | "IMUX.DATA31"
                    ) && nloc.2 != edev.chip.row_mid() - 1
                        && nloc.2 != edev.chip.row_mid()
                    {
                        return None;
                    }
                    if wire_name == "IMUX.DATA13"
                        && edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3ADsp
                        && nloc.1 == edev.chip.col_w()
                    {
                        // ISE bug. sigh.
                        return None;
                    }
                    if matches!(
                        &wire_name[..],
                        "IMUX.DATA12" | "IMUX.DATA13" | "IMUX.DATA14"
                    ) && nloc.2 != edev.chip.row_mid()
                    {
                        return None;
                    }
                }
                if backend.egrid.db.tile_classes.key(node.class) == "INT.IOI.S3A.TB"
                    && wire_name == "IMUX.DATA15"
                    && nloc.2 == edev.chip.row_n()
                {
                    // also ISE bug.
                    return None;
                }
                if edev.chip.kind.is_spartan3a()
                    && backend.egrid.db.tile_classes.key(node.class) == "INT.CLB"
                {
                    // avoid SR in corners — it causes the inverter bit to be auto-set
                    let is_lr = nloc.1 == edev.chip.col_w() || nloc.1 == edev.chip.col_e();
                    let is_bt = nloc.2 == edev.chip.row_s() || nloc.2 == edev.chip.row_n();
                    if intdb.wires.key(self.wire.1).starts_with("IMUX.SR") && is_lr && is_bt {
                        return None;
                    }
                }
                if matches!(&wire_name[..], "IMUX.DATA15" | "IMUX.DATA31")
                    && ndb
                        .tile_class_namings
                        .key(nnode.naming)
                        .starts_with("INT.MACC")
                {
                    // ISE bug.
                    return None;
                }
            }
            ExpandedDevice::Virtex4(edev) => {
                if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex4 {
                    // avoid CLK in center column — using it on DCM tiles causes the inverter bit to be auto-set
                    if intdb.wires.key(self.wire.1).starts_with("IMUX.CLK")
                        && nloc.1 == edev.col_clk
                    {
                        return None;
                    }
                }
            }
            _ => (),
        }

        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct NodeIntSrcFilter {
    wire: TileClassWire,
}

impl NodeIntSrcFilter {
    pub fn new(wire: TileClassWire) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for NodeIntSrcFilter {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let intdb = backend.egrid.db;
        let ndb = backend.ngrid.db;
        let wire_name = intdb.wires.key(self.wire.1);
        let node = backend.egrid.tile(nloc);
        let nnode = &backend.ngrid.tiles[&nloc];
        #[allow(clippy::single_match)]
        match backend.edev {
            ExpandedDevice::Virtex2(edev) => {
                if (edev.chip.kind.is_virtex2()
                    || edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3)
                    && wire_name.starts_with("OUT")
                    && intdb.tile_classes.key(node.class).starts_with("INT.DCM")
                {
                    let (layer, _) = backend
                        .egrid
                        .find_tile_loc(nloc.0, (nloc.1, nloc.2), |node| {
                            intdb.tile_classes.key(node.class).starts_with("DCM.")
                        })
                        .unwrap();
                    let ndcm = &backend.ngrid.tiles[&(nloc.0, nloc.1, nloc.2, layer)];
                    let site = &ndcm.bels[prjcombine_virtex2::bels::DCM];
                    fuzzer = fuzzer.base(Key::SiteMode(site), "DCM").base(
                        Key::BelMutex(
                            (nloc.0, (nloc.1, nloc.2), prjcombine_virtex2::bels::DCM),
                            "MODE".into(),
                        ),
                        "INT",
                    );
                    for pin in [
                        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                        "CLKFX180", "CONCUR", "STATUS1", "STATUS7",
                    ] {
                        fuzzer = fuzzer.base(Key::SitePin(site, pin.into()), true);
                    }
                }
                if wire_name == "OUT.PCI0"
                    && nloc.2 != edev.chip.row_pci.unwrap() - 2
                    && nloc.2 != edev.chip.row_pci.unwrap() - 1
                    && nloc.2 != edev.chip.row_pci.unwrap()
                    && nloc.2 != edev.chip.row_pci.unwrap() + 1
                {
                    return None;
                }
                if wire_name == "OUT.PCI1"
                    && nloc.2 != edev.chip.row_pci.unwrap() - 1
                    && nloc.2 != edev.chip.row_pci.unwrap()
                {
                    return None;
                }
                if (backend.egrid.db.tile_classes.key(node.class) == "INT.IOI.S3E"
                    || backend.egrid.db.tile_classes.key(node.class) == "INT.IOI.S3A.LR")
                    && matches!(
                        &wire_name[..],
                        "OUT.FAN3" | "OUT.FAN7" | "OUT.SEC11" | "OUT.SEC15"
                    )
                    && nloc.2 != edev.chip.row_mid() - 1
                    && nloc.2 != edev.chip.row_mid()
                {
                    return None;
                }
                if wire_name.starts_with("GCLK")
                    && matches!(
                        &ndb.tile_class_namings.key(nnode.naming)[..],
                        "INT.BRAM.BRK" | "INT.BRAM.S3ADSP.BRK" | "INT.MACC.BRK"
                    )
                {
                    // ISE bug.
                    return None;
                }
            }
            _ => (),
        }
        Some((fuzzer, false))
    }
}

pub fn resolve_int_pip<'a>(
    backend: &IseBackend<'a>,
    loc: NodeLoc,
    wire_to: TileClassWire,
    wire_from: TileClassWire,
) -> Option<(&'a str, &'a str, &'a str)> {
    let node = backend.egrid.tile(loc);
    let nnode = &backend.ngrid.tiles[&loc];
    let ndb = backend.ngrid.db;
    let node_naming = &ndb.tile_class_namings[nnode.naming];
    backend
        .egrid
        .resolve_wire((loc.0, node.cells[wire_to.0], wire_to.1))?;
    backend
        .egrid
        .resolve_wire((loc.0, node.cells[wire_from.0], wire_from.1))?;
    Some(
        if let Some(ext) = node_naming.ext_pips.get(&(wire_to, wire_from)) {
            (&nnode.names[ext.tile], &ext.wire_to, &ext.wire_from)
        } else {
            (
                &nnode.names[RawTileId::from_idx(0)],
                node_naming.wires.get(&wire_to)?,
                node_naming.wires.get(&wire_from)?,
            )
        },
    )
}

#[derive(Clone, Debug)]
pub struct BaseIntPip {
    wire_to: TileClassWire,
    wire_from: TileClassWire,
}

impl BaseIntPip {
    pub fn new(wire_to: TileClassWire, wire_from: TileClassWire) -> Self {
        Self { wire_to, wire_from }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseIntPip {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let (tile, wt, wf) = resolve_int_pip(backend, nloc, self.wire_to, self.wire_from)?;
        let fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzIntPip {
    wire_to: TileClassWire,
    wire_from: TileClassWire,
}

impl FuzzIntPip {
    pub fn new(wire_to: TileClassWire, wire_from: TileClassWire) -> Self {
        Self { wire_to, wire_from }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzIntPip {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let (tile, wt, wf) = resolve_int_pip(backend, nloc, self.wire_to, self.wire_from)?;
        let fuzzer = fuzzer.fuzz(Key::Pip(tile, wf, wt), None, true);
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct DriveLLH {
    pub wire: TileClassWire,
}

impl DriveLLH {
    pub fn new(wire: TileClassWire) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DriveLLH {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        match backend.edev {
            ExpandedDevice::Xc2000(edev) => {
                assert_eq!(edev.chip.kind, prjcombine_xc2000::chip::ChipKind::Xc5200);
                let node = backend.egrid.tile(nloc);
                let wnode =
                    backend
                        .egrid
                        .resolve_wire((nloc.0, node.cells[self.wire.0], self.wire.1))?;
                let mut src_col = if node.cells[self.wire.0].0 < edev.chip.col_mid() {
                    edev.chip.col_mid() - 1
                } else {
                    edev.chip.col_mid()
                };
                loop {
                    if let Some((src_layer, src_node)) =
                        backend
                            .egrid
                            .find_tile_loc(nloc.0, (src_col, nloc.2), |src_node| {
                                backend
                                    .egrid
                                    .db
                                    .tile_classes
                                    .key(src_node.class)
                                    .starts_with("IO")
                                    || backend.egrid.db.tile_classes.key(src_node.class) == "CLB"
                            })
                    {
                        let dwire = (TileCellId::from_idx(0), self.wire.1);
                        let src_node_kind = &backend.egrid.db.tile_classes[src_node.class];
                        if let Some(mux) = src_node_kind.muxes.get(&dwire) {
                            let Some(dnode) = backend.egrid.resolve_wire((
                                nloc.0,
                                src_node.cells[dwire.0],
                                dwire.1,
                            )) else {
                                continue;
                            };
                            assert_eq!(dnode, wnode);
                            let swire = *mux.ins.first().unwrap();
                            let (tile, wa, wb) = resolve_int_pip(
                                backend,
                                (nloc.0, src_col, nloc.2, src_layer),
                                swire,
                                dwire,
                            )?;
                            fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                            return Some((fuzzer, false));
                        }
                    }
                    if src_col == edev.chip.col_w() || src_col == edev.chip.col_e() {
                        return None;
                    }
                    if src_col < edev.chip.col_mid() {
                        src_col -= 1;
                    } else {
                        src_col += 1;
                    }
                }
            }
            ExpandedDevice::Virtex2(edev) => {
                let node = backend.egrid.tile(nloc);
                let wnode =
                    backend
                        .egrid
                        .resolve_wire((nloc.0, node.cells[self.wire.0], self.wire.1))?;
                let mut src_col = node.cells[self.wire.0].0;
                loop {
                    if let Some((src_layer, src_node)) =
                        backend
                            .egrid
                            .find_tile_loc(nloc.0, (src_col, nloc.2), |src_node| {
                                backend
                                    .egrid
                                    .db
                                    .tile_classes
                                    .key(src_node.class)
                                    .starts_with("INT")
                            })
                    {
                        let src_node_kind = &backend.egrid.db.tile_classes[src_node.class];
                        for (&dwire, mux) in &src_node_kind.muxes {
                            if !backend.egrid.db.wires.key(dwire.1).starts_with("LH") {
                                continue;
                            }
                            let Some(dnode) = backend.egrid.resolve_wire((
                                nloc.0,
                                src_node.cells[dwire.0],
                                dwire.1,
                            )) else {
                                continue;
                            };
                            if dnode != wnode {
                                continue;
                            }
                            let swire = *mux.ins.first().unwrap();
                            let (tile, wa, wb) = resolve_int_pip(
                                backend,
                                (nloc.0, src_col, nloc.2, src_layer),
                                swire,
                                dwire,
                            )?;
                            fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                            return Some((fuzzer, false));
                        }
                    }
                    if src_col == edev.chip.col_w() || src_col == edev.chip.col_e() {
                        return None;
                    }
                    if self.wire.0.to_idx() == 0 {
                        src_col -= 1;
                    } else {
                        src_col += 1;
                    }
                }
            }
            _ => todo!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DriveLLV {
    pub wire: TileClassWire,
}

impl DriveLLV {
    pub fn new(wire: TileClassWire) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DriveLLV {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        match backend.edev {
            ExpandedDevice::Xc2000(edev) => {
                assert_eq!(edev.chip.kind, prjcombine_xc2000::chip::ChipKind::Xc5200);
                let node = backend.egrid.tile(nloc);
                let wnode =
                    backend
                        .egrid
                        .resolve_wire((nloc.0, node.cells[self.wire.0], self.wire.1))?;
                let mut src_row = if node.cells[self.wire.0].1 < edev.chip.row_mid() {
                    edev.chip.row_mid() - 1
                } else {
                    edev.chip.row_mid()
                };
                loop {
                    if let Some((src_layer, src_node)) =
                        backend
                            .egrid
                            .find_tile_loc(nloc.0, (nloc.1, src_row), |src_node| {
                                backend
                                    .egrid
                                    .db
                                    .tile_classes
                                    .key(src_node.class)
                                    .starts_with("IO")
                                    || backend.egrid.db.tile_classes.key(src_node.class) == "CLB"
                            })
                    {
                        let dwire = (TileCellId::from_idx(0), self.wire.1);
                        let src_node_kind = &backend.egrid.db.tile_classes[src_node.class];
                        if let Some(mux) = src_node_kind.muxes.get(&dwire) {
                            let Some(dnode) = backend.egrid.resolve_wire((
                                nloc.0,
                                src_node.cells[dwire.0],
                                dwire.1,
                            )) else {
                                continue;
                            };
                            assert_eq!(dnode, wnode);
                            let swire = *mux.ins.first().unwrap();
                            let (tile, wa, wb) = resolve_int_pip(
                                backend,
                                (nloc.0, nloc.1, src_row, src_layer),
                                swire,
                                dwire,
                            )?;
                            fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                            return Some((fuzzer, false));
                        }
                    }
                    if src_row == edev.chip.row_s() || src_row == edev.chip.row_n() {
                        return None;
                    }
                    if src_row < edev.chip.row_mid() {
                        src_row -= 1;
                    } else {
                        src_row += 1;
                    }
                }
            }
            ExpandedDevice::Virtex2(edev) => {
                let node = backend.egrid.tile(nloc);
                let wnode =
                    backend
                        .egrid
                        .resolve_wire((nloc.0, node.cells[self.wire.0], self.wire.1))?;
                let mut src_row = node.cells[self.wire.0].1;
                loop {
                    if let Some((src_layer, src_node)) =
                        backend
                            .egrid
                            .find_tile_loc(nloc.0, (nloc.1, src_row), |src_node| {
                                backend
                                    .egrid
                                    .db
                                    .tile_classes
                                    .key(src_node.class)
                                    .starts_with("INT")
                            })
                    {
                        let src_node_kind = &backend.egrid.db.tile_classes[src_node.class];
                        for (&dwire, mux) in &src_node_kind.muxes {
                            if !backend.egrid.db.wires.key(dwire.1).starts_with("LV") {
                                continue;
                            }
                            let Some(dnode) = backend.egrid.resolve_wire((
                                nloc.0,
                                src_node.cells[dwire.0],
                                dwire.1,
                            )) else {
                                continue;
                            };
                            if dnode != wnode {
                                continue;
                            }
                            let swire = *mux.ins.first().unwrap();
                            let (tile, wa, wb) = resolve_int_pip(
                                backend,
                                (nloc.0, nloc.1, src_row, src_layer),
                                swire,
                                dwire,
                            )?;
                            fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                            return Some((fuzzer, false));
                        }
                    }
                    if src_row == edev.chip.row_s() || src_row == edev.chip.row_n() {
                        return None;
                    }
                    if self.wire.0.to_idx() == 0 {
                        src_row -= 1;
                    } else {
                        src_row += 1;
                    }
                }
            }
            _ => todo!(),
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (node_kind, name, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        if backend.egrid.tile_index[node_kind].is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, name);
        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            for &wire_from in &mux.ins {
                let in_name = if node.cells.len() == 1 {
                    intdb.wires.key(wire_from.1).to_string()
                } else {
                    format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                };
                let mut builder = ctx
                    .build()
                    .test_manual("INT", &mux_name, in_name)
                    .prop(NodeIntDistinct::new(wire_to, wire_from))
                    .prop(NodeIntDstFilter::new(wire_to))
                    .prop(NodeIntSrcFilter::new(wire_from))
                    .prop(NodeMutexShared::new(wire_from))
                    .prop(IntMutex::new("MAIN".to_string()))
                    .prop(BaseRaw::new(
                        Key::GlobalMutex("MISR_CLOCK".to_string()),
                        Value::None,
                    ))
                    .prop(NodeMutexExclusive::new(wire_to))
                    .prop(FuzzIntPip::new(wire_to, wire_from));
                if let Some(inmux) = node.muxes.get(&wire_from) {
                    if inmux.ins.contains(&wire_to) {
                        if name.starts_with("LLH") {
                            builder = builder.prop(DriveLLH::new(wire_from));
                        } else if name.starts_with("LLV") {
                            builder = builder.prop(DriveLLV::new(wire_from));
                        } else {
                            let mut wire_help = None;
                            for &help in &inmux.ins {
                                if let Some(helpmux) = node.muxes.get(&help) {
                                    if helpmux.ins.contains(&wire_from) {
                                        continue;
                                    }
                                }
                                // println!("HELP {} <- {} <- {}", intdb.wires.key(wire_to.1), intdb.wires.key(wire_from.1), intdb.wires.key(help.1));
                                wire_help = Some(help);
                                break;
                            }
                            let wire_help = wire_help.unwrap();
                            builder = builder.prop(BaseIntPip::new(wire_from, wire_help));
                        }
                    }
                }
                if matches!(backend.edev, ExpandedDevice::Virtex2(_)) {
                    builder = builder.prop(BaseRaw::new(
                        Key::GlobalOpt("TESTLL".to_string()),
                        Value::None,
                    ));
                }
                if intdb.wires.key(wire_from.1) == "OUT.TBUS" {
                    builder = builder.prop(RowMutex::new("TBUF".to_string(), "INT".to_string()));
                }
                builder.commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, name, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.tile_index[node_kind].is_empty() {
            continue;
        }

        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            let mut inps = vec![];
            let mut got_empty = false;
            for &wire_from in &mux.ins {
                let in_name = if node.cells.len() == 1 {
                    intdb.wires.key(wire_from.1).to_string()
                } else {
                    format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                };
                let diff = ctx.state.get_diff(name, "INT", &mux_name, &in_name);
                if let ExpandedDevice::Virtex2(edev) = ctx.edev {
                    if edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3ADsp
                        && name == "INT.IOI.S3A.LR"
                        && mux_name == "MUX.IMUX.DATA3"
                        && in_name == "OMUX10.N"
                    {
                        // ISE is bad and should feel bad.
                        continue;
                    }
                }
                if diff.bits.is_empty() {
                    if intdb.wires.key(wire_to.1).starts_with("IMUX")
                        && !intdb.wires[wire_from.1].is_tie()
                    {
                        // suppress message on known offenders.
                        if name == "INT.BRAM.S3A.03"
                            && (mux_name.starts_with("MUX.IMUX.CLK")
                                || mux_name.starts_with("MUX.IMUX.CE"))
                        {
                            // these muxes don't actually exist.
                            continue;
                        }
                        if name.starts_with("INT.IOI.S3")
                            && mux_name.starts_with("MUX.IMUX.DATA")
                            && (in_name.starts_with("OUT.FAN")
                                || in_name.starts_with("IMUX.FAN")
                                || in_name.starts_with("OMUX"))
                        {
                            // ISE is kind of bad. fill these from INT.CLB and verify later?
                            continue;
                        }
                        println!("UMMMMM PIP {name} {mux_name} {in_name} is empty");
                        continue;
                    }
                    got_empty = true;
                }
                inps.push((in_name.to_string(), diff));
            }
            if !got_empty {
                inps.push(("NONE".to_string(), Diff::default()));
            }
            let ti = xlat_enum_ocd(inps, OcdMode::Mux);
            if ti.bits.is_empty()
                && !(name == "INT.GT.CLKPAD"
                    && matches!(
                        &mux_name[..],
                        "MUX.IMUX.CE0" | "MUX.IMUX.CE1" | "MUX.IMUX.TS0" | "MUX.IMUX.TS1"
                    ))
                && !(name == "INT.BRAM.S3A.03"
                    && (mux_name.starts_with("MUX.IMUX.CLK")
                        || mux_name.starts_with("MUX.IMUX.CE")))
            {
                println!("UMMM MUX {name} {mux_name} is empty");
            }
            ctx.tiledb.insert(name, "INT", mux_name, ti);
        }
    }
}
