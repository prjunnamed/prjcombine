use std::collections::BTreeSet;

use prjcombine_interconnect::{
    db::{BelInfo, ProgDelay, SwitchBoxItem, TileWireCoord},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, OcdMode, xlat_bit, xlat_enum_ocd};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_re_xilinx_naming::db::RawTileId;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
    generic::{fbuild::FuzzBuilderBase, props::mutex::TileMutexExclusive},
};

use super::{
    fbuild::FuzzCtx,
    props::{
        BaseRaw, DynProp,
        mutex::{IntMutex, RowMutex, WireMutexExclusive, WireMutexShared},
    },
};

#[derive(Clone, Debug)]
pub struct WireIntDistinct {
    wire_a: TileWireCoord,
    wire_b: TileWireCoord,
}

impl WireIntDistinct {
    pub fn new(wire_a: TileWireCoord, wire_b: TileWireCoord) -> Self {
        Self { wire_a, wire_b }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for WireIntDistinct {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let a = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_a))?;
        let b = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_b))?;
        if a == b {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct WireIntDstFilter {
    wire: TileWireCoord,
}

impl WireIntDstFilter {
    pub fn new(wire: TileWireCoord) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for WireIntDstFilter {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let intdb = backend.edev.db;
        let ndb = backend.ngrid.db;
        let wire_name = intdb.wires.key(self.wire.wire);
        match backend.edev {
            ExpandedDevice::Virtex2(edev) => {
                let tile = &backend.edev[tcrd];
                let ntile = &backend.ngrid.tiles[&tcrd];
                if backend
                    .edev
                    .db
                    .tile_classes
                    .key(tile.class)
                    .starts_with("INT.BRAM")
                {
                    let mut tgt = None;
                    for i in 0..4 {
                        if let Some(bram_tile) =
                            backend.edev.find_tile(tcrd.delta(0, -(i as i32)), |tile| {
                                intdb.tile_classes.key(tile.class).starts_with("BRAM")
                                    || intdb.tile_classes.key(tile.class) == "DSP"
                            })
                        {
                            tgt = Some((bram_tile, i));
                            break;
                        }
                    }
                    let (bram_tile, idx) = tgt.unwrap();
                    let bram_tcls = &intdb[bram_tile.class];
                    if (edev.chip.kind.is_virtex2()
                        || edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3)
                        && (wire_name.starts_with("IMUX.CLK")
                            || wire_name.starts_with("IMUX.SR")
                            || wire_name.starts_with("IMUX.CE")
                            || wire_name.starts_with("IMUX.TS"))
                    {
                        let mut found = false;
                        for bel in bram_tcls.bels.values() {
                            let BelInfo::Bel(bel) = bel else {
                                unreachable!()
                            };
                            for pin in bel.pins.values() {
                                if pin
                                    .wires
                                    .contains(&TileWireCoord::new_idx(idx, self.wire.wire))
                                {
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
                if backend.edev.db.tile_classes.key(tile.class) == "INT.IOI.S3E"
                    || backend.edev.db.tile_classes.key(tile.class) == "INT.IOI.S3A.LR"
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
                    ) && tcrd.row != edev.chip.row_mid() - 1
                        && tcrd.row != edev.chip.row_mid()
                    {
                        return None;
                    }
                    if wire_name == "IMUX.DATA13"
                        && edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3ADsp
                        && tcrd.col == edev.chip.col_w()
                    {
                        // ISE bug. sigh.
                        return None;
                    }
                    if matches!(
                        &wire_name[..],
                        "IMUX.DATA12" | "IMUX.DATA13" | "IMUX.DATA14"
                    ) && tcrd.row != edev.chip.row_mid()
                    {
                        return None;
                    }
                }
                if backend.edev.db.tile_classes.key(tile.class) == "INT.IOI.S3A.TB"
                    && wire_name == "IMUX.DATA15"
                    && tcrd.row == edev.chip.row_n()
                {
                    // also ISE bug.
                    return None;
                }
                if edev.chip.kind.is_spartan3a()
                    && backend.edev.db.tile_classes.key(tile.class) == "INT.CLB"
                {
                    // avoid SR in corners — it causes the inverter bit to be auto-set
                    let is_lr = tcrd.col == edev.chip.col_w() || tcrd.col == edev.chip.col_e();
                    let is_bt = tcrd.row == edev.chip.row_s() || tcrd.row == edev.chip.row_n();
                    if intdb.wires.key(self.wire.wire).starts_with("IMUX.SR") && is_lr && is_bt {
                        return None;
                    }
                }
                if matches!(&wire_name[..], "IMUX.DATA15" | "IMUX.DATA31")
                    && ndb
                        .tile_class_namings
                        .key(ntile.naming)
                        .starts_with("INT.MACC")
                {
                    // ISE bug.
                    return None;
                }
            }
            ExpandedDevice::Virtex4(edev) => {
                if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex4 {
                    // avoid CLK in center column — using it on DCM tiles causes the inverter bit to be auto-set
                    if intdb.wires.key(self.wire.wire).starts_with("IMUX.CLK")
                        && tcrd.col == edev.col_clk
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
pub struct WireIntSrcFilter {
    wire: TileWireCoord,
}

impl WireIntSrcFilter {
    pub fn new(wire: TileWireCoord) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for WireIntSrcFilter {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let intdb = backend.edev.db;
        let ndb = backend.ngrid.db;
        let wire_name = intdb.wires.key(self.wire.wire);
        let tile = &backend.edev[tcrd];
        let ntile = &backend.ngrid.tiles[&tcrd];
        #[allow(clippy::single_match)]
        match backend.edev {
            ExpandedDevice::Virtex2(edev) => {
                if (edev.chip.kind.is_virtex2()
                    || edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3)
                    && wire_name.starts_with("OUT")
                    && intdb.tile_classes.key(tile.class).starts_with("INT.DCM")
                {
                    let ndcm = &backend.ngrid.tiles[&tcrd.tile(prjcombine_virtex2::tslots::BEL)];
                    let site = &ndcm.bels[prjcombine_virtex2::bels::DCM];
                    fuzzer = fuzzer.base(Key::SiteMode(site), "DCM").base(
                        Key::BelMutex(tcrd.bel(prjcombine_virtex2::bels::DCM), "MODE".into()),
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
                    && tcrd.row != edev.chip.row_pci.unwrap() - 2
                    && tcrd.row != edev.chip.row_pci.unwrap() - 1
                    && tcrd.row != edev.chip.row_pci.unwrap()
                    && tcrd.row != edev.chip.row_pci.unwrap() + 1
                {
                    return None;
                }
                if wire_name == "OUT.PCI1"
                    && tcrd.row != edev.chip.row_pci.unwrap() - 1
                    && tcrd.row != edev.chip.row_pci.unwrap()
                {
                    return None;
                }
                if (backend.edev.db.tile_classes.key(tile.class) == "INT.IOI.S3E"
                    || backend.edev.db.tile_classes.key(tile.class) == "INT.IOI.S3A.LR")
                    && matches!(
                        &wire_name[..],
                        "OUT.FAN3" | "OUT.FAN7" | "OUT.SEC11" | "OUT.SEC15"
                    )
                    && tcrd.row != edev.chip.row_mid() - 1
                    && tcrd.row != edev.chip.row_mid()
                {
                    return None;
                }
                if wire_name.starts_with("GCLK")
                    && matches!(
                        &ndb.tile_class_namings.key(ntile.naming)[..],
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
    tcrd: TileCoord,
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
) -> Option<(&'a str, &'a str, &'a str)> {
    let ntile = &backend.ngrid.tiles[&tcrd];
    let ndb = backend.ngrid.db;
    let tile_naming = &ndb.tile_class_namings[ntile.naming];
    backend
        .edev
        .resolve_wire(backend.edev.tile_wire(tcrd, wire_to))?;
    backend
        .edev
        .resolve_wire(backend.edev.tile_wire(tcrd, wire_from))?;
    Some(
        if let Some(ext) = tile_naming.ext_pips.get(&(wire_to, wire_from)) {
            (&ntile.names[ext.tile], &ext.wire_to, &ext.wire_from)
        } else {
            (
                &ntile.names[RawTileId::from_idx(0)],
                tile_naming.wires.get(&wire_to)?,
                tile_naming.wires.get(&wire_from)?,
            )
        },
    )
}

#[derive(Clone, Debug)]
pub struct BaseIntPip {
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
}

impl BaseIntPip {
    pub fn new(wire_to: TileWireCoord, wire_from: TileWireCoord) -> Self {
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
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let (tile, wt, wf) = resolve_int_pip(backend, tcrd, self.wire_to, self.wire_from)?;
        let fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzIntPip {
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
}

impl FuzzIntPip {
    pub fn new(wire_to: TileWireCoord, wire_from: TileWireCoord) -> Self {
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
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let (tile, wt, wf) = resolve_int_pip(backend, tcrd, self.wire_to, self.wire_from)?;
        let fuzzer = fuzzer.fuzz(Key::Pip(tile, wf, wt), None, true);
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct DriveLLH {
    pub wire: TileWireCoord,
}

impl DriveLLH {
    pub fn new(wire: TileWireCoord) -> Self {
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
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        match backend.edev {
            ExpandedDevice::Xc2000(edev) => {
                assert_eq!(edev.chip.kind, prjcombine_xc2000::chip::ChipKind::Xc5200);
                let rwire = backend
                    .edev
                    .resolve_wire(backend.edev.tile_wire(tcrd, self.wire))?;
                let mut src_col =
                    if backend.edev.tile_cell(tcrd, self.wire.cell).col < edev.chip.col_mid() {
                        edev.chip.col_mid() - 1
                    } else {
                        edev.chip.col_mid()
                    };
                loop {
                    let int_tcrd = tcrd
                        .cell
                        .with_col(src_col)
                        .tile(prjcombine_xc2000::tslots::MAIN);
                    if let Some(src_tile) = backend.edev.get_tile(int_tcrd) {
                        let dwire = TileWireCoord::new_idx(0, self.wire.wire);
                        if let Some(ins) =
                            backend.edev.db_index[src_tile.class].pips_bwd.get(&dwire)
                        {
                            let Some(drwire) = backend
                                .edev
                                .resolve_wire(backend.edev.tile_wire(int_tcrd, dwire))
                            else {
                                continue;
                            };
                            assert_eq!(drwire, rwire);
                            let swire = ins.iter().next().unwrap().tw;
                            let (tile, wa, wb) = resolve_int_pip(backend, int_tcrd, swire, dwire)?;
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
                let rwire = backend
                    .edev
                    .resolve_wire(backend.edev.tile_wire(tcrd, self.wire))?;
                let mut src_col = backend.edev.tile_cell(tcrd, self.wire.cell).col;
                loop {
                    let int_tcrd = tcrd
                        .cell
                        .with_col(src_col)
                        .tile(prjcombine_virtex2::tslots::INT);
                    if let Some(src_tile) = backend.edev.get_tile(int_tcrd) {
                        for (&dwire, ins) in &backend.edev.db_index[src_tile.class].pips_bwd {
                            if !backend.edev.db.wires.key(dwire.wire).starts_with("LH") {
                                continue;
                            }
                            let Some(drwire) = backend
                                .edev
                                .resolve_wire(backend.edev.tile_wire(int_tcrd, dwire))
                            else {
                                continue;
                            };
                            if drwire != rwire {
                                continue;
                            }
                            let swire = ins.iter().next().unwrap().tw;
                            let (tile, wa, wb) = resolve_int_pip(backend, int_tcrd, swire, dwire)?;
                            fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                            return Some((fuzzer, false));
                        }
                    }
                    if src_col == edev.chip.col_w() || src_col == edev.chip.col_e() {
                        return None;
                    }
                    if self.wire.cell.to_idx() == 0 {
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
    pub wire: TileWireCoord,
}

impl DriveLLV {
    pub fn new(wire: TileWireCoord) -> Self {
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
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        match backend.edev {
            ExpandedDevice::Xc2000(edev) => {
                assert_eq!(edev.chip.kind, prjcombine_xc2000::chip::ChipKind::Xc5200);
                let rwire = backend
                    .edev
                    .resolve_wire(backend.edev.tile_wire(tcrd, self.wire))?;
                let mut src_row =
                    if backend.edev.tile_cell(tcrd, self.wire.cell).row < edev.chip.row_mid() {
                        edev.chip.row_mid() - 1
                    } else {
                        edev.chip.row_mid()
                    };
                loop {
                    let int_tcrd = tcrd
                        .cell
                        .with_row(src_row)
                        .tile(prjcombine_xc2000::tslots::MAIN);
                    if let Some(src_tile) = backend.edev.get_tile(int_tcrd) {
                        let dwire = TileWireCoord::new_idx(0, self.wire.wire);
                        if let Some(ins) =
                            backend.edev.db_index[src_tile.class].pips_bwd.get(&dwire)
                        {
                            let Some(drwire) = backend
                                .edev
                                .resolve_wire(backend.edev.tile_wire(int_tcrd, dwire))
                            else {
                                continue;
                            };
                            assert_eq!(drwire, rwire);
                            let swire = ins.iter().next().unwrap().tw;
                            let (tile, wa, wb) = resolve_int_pip(backend, int_tcrd, swire, dwire)?;
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
                let rwire = backend
                    .edev
                    .resolve_wire(backend.edev.tile_wire(tcrd, self.wire))?;
                let mut src_row = backend.edev.tile_cell(tcrd, self.wire.cell).row;
                loop {
                    let int_tcrd = tcrd
                        .cell
                        .with_row(src_row)
                        .tile(prjcombine_virtex2::tslots::INT);
                    if let Some(src_tile) = backend.edev.get_tile(int_tcrd) {
                        for (&dwire, ins) in &backend.edev.db_index[src_tile.class].pips_bwd {
                            if !backend.edev.db.wires.key(dwire.wire).starts_with("LV") {
                                continue;
                            }
                            let Some(drwire) = backend
                                .edev
                                .resolve_wire(backend.edev.tile_wire(int_tcrd, dwire))
                            else {
                                continue;
                            };
                            if drwire != rwire {
                                continue;
                            }
                            let swire = ins.iter().next().unwrap().tw;
                            let (tile, wa, wb) = resolve_int_pip(backend, int_tcrd, swire, dwire)?;
                            fuzzer = fuzzer.base(Key::Pip(tile, wa, wb), true);
                            return Some((fuzzer, false));
                        }
                    }
                    if src_row == edev.chip.row_s() || src_row == edev.chip.row_n() {
                        return None;
                    }
                    if self.wire.cell.to_idx() == 0 {
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

fn resolve_intf_delay<'a>(
    backend: &IseBackend<'a>,
    tcrd: TileCoord,
    delay: ProgDelay,
) -> Option<(&'a str, &'a str, &'a str, &'a str)> {
    let ntile = &backend.ngrid.tiles[&tcrd];
    let ndb = backend.ngrid.db;
    let tile_naming = &ndb.tile_class_namings[ntile.naming];
    backend
        .edev
        .resolve_wire(backend.edev.tile_wire(tcrd, delay.dst))?;
    backend
        .edev
        .resolve_wire(backend.edev.tile_wire(tcrd, delay.src.tw))?;
    let name_out = tile_naming.wires[&delay.dst].as_str();
    let name_delay = tile_naming.delay_wires[&delay.dst].as_str();
    let name_in = tile_naming.wires[&delay.src.tw].as_str();
    Some((
        &ntile.names[RawTileId::from_idx(0)],
        name_in,
        name_delay,
        name_out,
    ))
}

#[derive(Clone, Debug)]
struct FuzzIntfDelay {
    delay: ProgDelay,
    state: bool,
}

impl FuzzIntfDelay {
    pub fn new(delay: ProgDelay, state: bool) -> Self {
        Self { delay, state }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzIntfDelay {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let (tile, wa, wb, wc) = resolve_intf_delay(backend, tcrd, self.delay)?;
        let fuzzer = if self.state {
            fuzzer
                .fuzz(Key::Pip(tile, wa, wb), None, true)
                .fuzz(Key::Pip(tile, wb, wc), None, true)
        } else {
            fuzzer.fuzz(Key::Pip(tile, wa, wc), None, true)
        };
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcname) else {
            continue;
        };
        let mut skip_pips = BTreeSet::new();
        for (bslot, bel) in &tcls.bels {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let mut bctx = ctx.bel(bslot);
            for item in &sb.items {
                let SwitchBoxItem::ProgDelay(delay) = *item else {
                    continue;
                };
                skip_pips.insert((delay.dst, delay.src.tw));
                assert_eq!(tcls.cells.len(), 1);
                let del_name = format!("DELAY.{}", intdb.wires.key(delay.dst.wire));
                for val in ["0", "1"] {
                    bctx.build()
                        .prop(IntMutex::new("INTF".into()))
                        .test_manual(&del_name, val)
                        .prop(TileMutexExclusive::new("INTF".into()))
                        .prop(WireMutexExclusive::new(delay.dst))
                        .prop(WireMutexExclusive::new(delay.src.tw))
                        .prop(FuzzIntfDelay::new(delay, val == "1"))
                        .commit();
                }
            }
        }
        let tcls_index = &backend.edev.db_index[tcid];
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let mux_name = if tcls.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.wire))
            } else {
                format!("MUX.{:#}.{}", wire_to.cell, intdb.wires.key(wire_to.wire))
            };
            for &wire_from in ins {
                let wire_from = wire_from.tw;
                if skip_pips.contains(&(wire_to, wire_from)) {
                    continue;
                }
                let in_name = if tcls.cells.len() == 1 {
                    intdb.wires.key(wire_from.wire).to_string()
                } else {
                    format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire))
                };
                let mut builder = ctx
                    .build()
                    .test_manual("INT", &mux_name, in_name)
                    .prop(WireIntDistinct::new(wire_to, wire_from))
                    .prop(WireIntDstFilter::new(wire_to))
                    .prop(WireIntSrcFilter::new(wire_from))
                    .prop(WireMutexShared::new(wire_from))
                    .prop(IntMutex::new("MAIN".to_string()))
                    .prop(BaseRaw::new(
                        Key::GlobalMutex("MISR_CLOCK".to_string()),
                        Value::None,
                    ))
                    .prop(WireMutexExclusive::new(wire_to))
                    .prop(FuzzIntPip::new(wire_to, wire_from));
                if let Some(inmux) = tcls_index.pips_bwd.get(&wire_from)
                    && inmux.contains(&wire_to.pos())
                {
                    if tcname.starts_with("LLH") {
                        builder = builder.prop(DriveLLH::new(wire_from));
                    } else if tcname.starts_with("LLV") {
                        builder = builder.prop(DriveLLV::new(wire_from));
                    } else {
                        let mut wire_help = None;
                        for &help in inmux {
                            if let Some(helpmux) = tcls_index.pips_bwd.get(&help.tw)
                                && helpmux.contains(&wire_from.pos())
                            {
                                continue;
                            }
                            // println!("HELP {} <- {} <- {}", intdb.wires.key(wire_to.1), intdb.wires.key(wire_from.1), intdb.wires.key(help.1));
                            wire_help = Some(help.tw);
                            break;
                        }
                        let wire_help = wire_help.unwrap();
                        builder = builder.prop(BaseIntPip::new(wire_from, wire_help));
                    }
                }
                if matches!(backend.edev, ExpandedDevice::Virtex2(_)) {
                    builder = builder.prop(BaseRaw::new(
                        Key::GlobalOpt("TESTLL".to_string()),
                        Value::None,
                    ));
                }
                if intdb.wires.key(wire_from.wire) == "OUT.TBUS" {
                    builder = builder.prop(RowMutex::new("TBUF".to_string(), "INT".to_string()));
                }
                builder.commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for (_, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tile(tcname) {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let bel = intdb.bel_slots.key(bslot);
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        let mux_name = if tcls.cells.len() == 1 {
                            format!("MUX.{}", intdb.wires.key(mux.dst.wire))
                        } else {
                            format!("MUX.{:#}.{}", mux.dst.cell, intdb.wires.key(mux.dst.wire))
                        };
                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &wire_from in &mux.src {
                            let wire_from = wire_from.tw;
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(wire_from.wire).to_string()
                            } else {
                                format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire))
                            };
                            let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                            if let ExpandedDevice::Virtex2(edev) = ctx.edev
                                && edev.chip.kind
                                    == prjcombine_virtex2::chip::ChipKind::Spartan3ADsp
                                && tcname == "INT.IOI.S3A.LR"
                                && mux_name == "MUX.IMUX.DATA3"
                                && in_name == "OMUX10.N"
                            {
                                // ISE is bad and should feel bad.
                                continue;
                            }
                            if diff.bits.is_empty() {
                                if intdb.wires.key(mux.dst.wire).starts_with("IMUX")
                                    && !intdb[wire_from.wire].is_tie()
                                {
                                    // suppress message on known offenders.
                                    if tcname == "INT.BRAM.S3A.03"
                                        && (mux_name.starts_with("MUX.IMUX.CLK")
                                            || mux_name.starts_with("MUX.IMUX.CE"))
                                    {
                                        // these muxes don't actually exist.
                                        continue;
                                    }
                                    if tcname.starts_with("INT.IOI.S3")
                                        && mux_name.starts_with("MUX.IMUX.DATA")
                                        && (in_name.starts_with("OUT.FAN")
                                            || in_name.starts_with("IMUX.FAN")
                                            || in_name.starts_with("OMUX"))
                                    {
                                        // ISE is kind of bad. fill these from INT.CLB and verify later?
                                        continue;
                                    }
                                    println!("UMMMMM PIP {tcname} {mux_name} {in_name} is empty");
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
                            && !(tcname == "INT.GT.CLKPAD"
                                && matches!(
                                    &mux_name[..],
                                    "MUX.IMUX.CE0"
                                        | "MUX.IMUX.CE1"
                                        | "MUX.IMUX.TS0"
                                        | "MUX.IMUX.TS1"
                                ))
                            && !(tcname == "INT.BRAM.S3A.03"
                                && (mux_name.starts_with("MUX.IMUX.CLK")
                                    || mux_name.starts_with("MUX.IMUX.CE")))
                        {
                            println!("UMMM MUX {tcname} {mux_name} is empty");
                        }
                        ctx.tiledb.insert(tcname, bel, mux_name, ti);
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        let mux_name = if tcls.cells.len() == 1 {
                            format!("MUX.{}", intdb.wires.key(buf.dst.wire))
                        } else {
                            format!("MUX.{:#}.{}", buf.dst.cell, intdb.wires.key(buf.dst.wire))
                        };
                        let in_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(buf.src.wire).to_string()
                        } else {
                            format!("{:#}.{}", buf.src.cell, intdb.wires.key(buf.src.wire))
                        };
                        let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                        let buf_name = if tcls.cells.len() == 1 {
                            format!(
                                "BUF.{dst}.{src}",
                                dst = intdb.wires.key(buf.dst.wire),
                                src = intdb.wires.key(buf.src.wire)
                            )
                        } else {
                            format!(
                                "BUF.{dst_cell:#}.{dst}.{src_cell:#}.{src}",
                                dst_cell = buf.dst.cell,
                                src_cell = buf.src.cell,
                                dst = intdb.wires.key(buf.dst.wire),
                                src = intdb.wires.key(buf.src.wire)
                            )
                        };
                        ctx.tiledb.insert(tcname, bel, buf_name, xlat_bit(diff));
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        let mux_name = if tcls.cells.len() == 1 {
                            format!("MUX.{}", intdb.wires.key(buf.dst.wire))
                        } else {
                            format!("MUX.{:#}.{}", buf.dst.cell, intdb.wires.key(buf.dst.wire))
                        };
                        let in_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(buf.src.wire).to_string()
                        } else {
                            format!("{:#}.{}", buf.src.cell, intdb.wires.key(buf.src.wire))
                        };
                        let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                        diff.assert_empty();
                    }
                    SwitchBoxItem::ProgInv(_) => (),
                    SwitchBoxItem::ProgDelay(delay) => {
                        let del_name = format!("DELAY.{}", intdb.wires.key(delay.dst.wire));
                        ctx.collect_enum_bool(tcname, bel, &del_name, "0", "1");
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
