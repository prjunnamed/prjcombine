use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{
        BelInfo, BelInput, BelSlotId, ProgDelay, SwitchBoxItem, TileClassId, TileWireCoord,
        WireSlotId,
    },
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_enum_raw};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_spartan6::defs::{bslots as bslots_s6, tcls as tcls_s6, wires as wires_s6};
use prjcombine_virtex2::defs::{
    bslots as bslots_v2,
    spartan3::{tcls as tcls_s3, wires as wires_s3},
    tslots as tslots_v2,
    virtex2::wires as wires_v2,
};

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::FuzzBuilderBase,
        props::{
            NullBits,
            bel::{BaseBelMode, FuzzBelPin},
            mutex::TileMutexExclusive,
            pip::{FuzzPip, PipWire},
            relation::NoopRelation,
        },
    },
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
        let tile = &backend.edev[tcrd];
        match backend.edev {
            ExpandedDevice::Virtex2(edev) => {
                let ntile = &backend.ngrid.tiles[&tcrd];
                if backend
                    .edev
                    .db
                    .tile_classes
                    .key(tile.class)
                    .starts_with("INT_BRAM")
                {
                    let mut tgt = None;
                    for i in 0..4 {
                        let bram_tcrd = tcrd.delta(0, -(i as i32)).tile(tslots_v2::BEL);
                        if edev.has_bel(bram_tcrd.bel(bslots_v2::BRAM))
                            || edev.has_bel(bram_tcrd.bel(bslots_v2::DSP))
                        {
                            tgt = Some((&edev[bram_tcrd], i));
                            break;
                        }
                    }
                    let (bram_tile, idx) = tgt.unwrap();
                    let bram_tcls = &intdb[bram_tile.class];
                    if (edev.chip.kind.is_virtex2()
                        || edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3)
                        && (wire_name.starts_with("IMUX_CLK")
                            || wire_name.starts_with("IMUX_SR")
                            || wire_name.starts_with("IMUX_CE")
                            || wire_name.starts_with("IMUX_TS"))
                    {
                        let wire_oi = backend.edev.db_index.tile_classes[tile.class].pips_fwd
                            [&self.wire]
                            .iter()
                            .next()
                            .unwrap()
                            .wire;
                        assert!(intdb.wires.key(wire_oi).contains("OPTINV"));
                        let mut found = false;
                        for bel in bram_tcls.bels.values() {
                            let BelInfo::Bel(bel) = bel else {
                                unreachable!()
                            };
                            for &inp in bel.inputs.values() {
                                let wire = match inp {
                                    BelInput::Fixed(ptwc) => ptwc.tw,
                                    BelInput::Invertible(twc, _) => twc,
                                };
                                if wire == TileWireCoord::new_idx(idx, wire_oi) {
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
                if backend.edev.db.tile_classes.key(tile.class) == "INT_IOI_S3E"
                    || backend.edev.db.tile_classes.key(tile.class) == "INT_IOI_S3A_WE"
                {
                    if matches!(
                        &wire_name[..],
                        "IMUX_DATA[3]"
                            | "IMUX_DATA[7]"
                            | "IMUX_DATA[11]"
                            | "IMUX_DATA[15]"
                            | "IMUX_DATA[19]"
                            | "IMUX_DATA[23]"
                            | "IMUX_DATA[27]"
                            | "IMUX_DATA[31]"
                    ) && tcrd.row != edev.chip.row_mid() - 1
                        && tcrd.row != edev.chip.row_mid()
                    {
                        return None;
                    }
                    if wire_name == "IMUX_DATA[13]"
                        && edev.chip.kind == prjcombine_virtex2::chip::ChipKind::Spartan3ADsp
                        && tcrd.col == edev.chip.col_w()
                    {
                        // ISE bug. sigh.
                        return None;
                    }
                    if matches!(
                        &wire_name[..],
                        "IMUX_DATA[12]" | "IMUX_DATA[13]" | "IMUX_DATA[14]"
                    ) && tcrd.row != edev.chip.row_mid()
                    {
                        return None;
                    }
                }
                if backend.edev.db.tile_classes.key(tile.class) == "INT_IOI_S3A_SN"
                    && wire_name == "IMUX_DATA[15]"
                    && tcrd.row == edev.chip.row_n()
                {
                    // also ISE bug.
                    return None;
                }
                if edev.chip.kind.is_spartan3a()
                    && backend.edev.db.tile_classes.key(tile.class) == "INT_CLB"
                {
                    // avoid SR in corners — it causes the inverter bit to be auto-set
                    let is_lr = tcrd.col == edev.chip.col_w() || tcrd.col == edev.chip.col_e();
                    let is_bt = tcrd.row == edev.chip.row_s() || tcrd.row == edev.chip.row_n();
                    if intdb.wires.key(self.wire.wire).starts_with("IMUX_SR") && is_lr && is_bt {
                        return None;
                    }
                }
                if matches!(&wire_name[..], "IMUX_DATA[15]" | "IMUX_DATA[31]")
                    && ndb
                        .tile_class_namings
                        .key(ntile.naming)
                        .starts_with("INT_MACC")
                {
                    // ISE bug.
                    return None;
                }
            }
            ExpandedDevice::Virtex4(edev) => {
                if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex4 {
                    // avoid CLK in center column — using it on DCM tiles causes the inverter bit to be auto-set
                    if intdb.wires.key(self.wire.wire).starts_with("IMUX_CLK")
                        && tcrd.col == edev.col_clk
                    {
                        return None;
                    }
                }
            }
            ExpandedDevice::Spartan6(edev) => {
                if prjcombine_spartan6::defs::wires::HCLK.contains(self.wire.wire)
                    && !edev[tile.cells[self.wire.cell]]
                        .tiles
                        .contains_id(prjcombine_spartan6::defs::tslots::INT)
                {
                    return None;
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
                    && intdb.tile_classes.key(tile.class).starts_with("INT_DCM")
                {
                    let site = backend
                        .ngrid
                        .get_bel_name(tcrd.bel(prjcombine_virtex2::defs::bslots::DCM))
                        .unwrap();
                    fuzzer = fuzzer.base(Key::SiteMode(site), "DCM").base(
                        Key::BelMutex(
                            tcrd.bel(prjcombine_virtex2::defs::bslots::DCM),
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
                if wire_name == "OUT_PCI[0]"
                    && tcrd.row != edev.chip.row_pci.unwrap() - 2
                    && tcrd.row != edev.chip.row_pci.unwrap() - 1
                    && tcrd.row != edev.chip.row_pci.unwrap()
                    && tcrd.row != edev.chip.row_pci.unwrap() + 1
                {
                    return None;
                }
                if wire_name == "OUT_PCI[1]"
                    && tcrd.row != edev.chip.row_pci.unwrap() - 1
                    && tcrd.row != edev.chip.row_pci.unwrap()
                {
                    return None;
                }
                if (backend.edev.db.tile_classes.key(tile.class) == "INT_IOI_S3E"
                    || backend.edev.db.tile_classes.key(tile.class) == "INT_IOI_S3A_WE")
                    && matches!(
                        &wire_name[..],
                        "OUT_FAN[3]" | "OUT_FAN[7]" | "OUT_SEC[11]" | "OUT_SEC[15]"
                    )
                    && tcrd.row != edev.chip.row_mid() - 1
                    && tcrd.row != edev.chip.row_mid()
                {
                    return None;
                }
                if wire_name.starts_with("GCLK")
                    && matches!(
                        &ndb.tile_class_namings.key(ntile.naming)[..],
                        "INT_BRAM_BRK" | "INT_BRAM_S3ADSP_BRK" | "INT_MACC_BRK"
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
            let nt = tile_naming.wires.get(&wire_to)?;
            let nf = tile_naming.wires.get(&wire_from)?;
            let name_to = if nt.alt_pips_to.contains(&wire_from) {
                nt.alt_name.as_ref().unwrap()
            } else {
                &nt.name
            };
            let name_from = if nf.alt_pips_from.contains(&wire_to) {
                nf.alt_name.as_ref().unwrap()
            } else {
                &nf.name
            };
            (&ntile.names[RawTileId::from_idx(0)], name_to, name_from)
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
                        .tile(prjcombine_xc2000::xc4000::tslots::MAIN);
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
                        .tile(prjcombine_virtex2::defs::tslots::INT);
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
                        .tile(prjcombine_xc2000::xc4000::tslots::MAIN);
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
                        .tile(prjcombine_virtex2::defs::tslots::INT);
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
    delay: &ProgDelay,
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
    let name_out = tile_naming.wires[&delay.dst].name.as_str();
    let name_delay = tile_naming.delay_wires[&delay.dst].as_str();
    let name_in = tile_naming.wires[&delay.src.tw].name.as_str();
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
        let (tile, wa, wb, wc) = resolve_intf_delay(backend, tcrd, &self.delay)?;
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

fn build_pip_fuzzer(
    backend: &IseBackend,
    ctx: &mut FuzzCtx,
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
    is_permabuf: bool,
) {
    let intdb = backend.edev.db;
    let tcid = ctx.tile_class.unwrap();
    let tcname = intdb.tile_classes.key(tcid);
    let tcls_index = &backend.edev.db_index[tcid];
    let mut builder = ctx
        .build()
        .test_routing(wire_to, wire_from.pos())
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
    if let ExpandedDevice::Virtex2(edev) = backend.edev
        && edev.chip.kind.is_virtex2()
        && wires_v2::GCLK_ROW.contains(wire_to.wire)
    {
        builder = builder.prop(BaseRaw::new(Key::GlobalMutex("BUFG".into()), "USE".into()));
    }
    if is_permabuf {
        builder = builder.prop(NullBits);
    }
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
    if intdb.wires.key(wire_from.wire) == "OUT_TBUS" {
        builder = builder.prop(RowMutex::new("TBUF".to_string(), "INT".to_string()));
    }
    if matches!(backend.edev, ExpandedDevice::Spartan6(_)) {
        if wires_s6::CMT_OUT.contains(wire_to.wire) {
            builder = builder.prop(BaseRaw::new(
                Key::GlobalMutex("CMT".into()),
                "MUX_PLL_HCLK".into(),
            ));
        }
        if let Some(idx) = wires_s6::DIVCLK_CLKC.index_of(wire_to.wire)
            && wires_s6::OUT_DIVCLK.contains(wire_from.wire)
        {
            builder = builder
                .prop(BaseBelMode::new(bslots_s6::BUFIO2[idx], 0, "BUFIO2".into()))
                .prop(FuzzBelPin::new(bslots_s6::BUFIO2[idx], 0, "DIVCLK".into()))
                .prop(FuzzPip::new(
                    NoopRelation,
                    PipWire::BelPinFar(bslots_s6::BUFIO2[idx], "DIVCLK".into()),
                    PipWire::BelPinNear(bslots_s6::BUFIO2[idx], "DIVCLK".into()),
                ));
        }
    }
    builder.commit();
}

fn is_anon_wire(edev: &ExpandedDevice, wire: WireSlotId) -> bool {
    match edev {
        ExpandedDevice::Virtex2(_) => false,
        ExpandedDevice::Spartan6(_) => prjcombine_spartan6::defs::wires::OUT_TEST.contains(wire),
        ExpandedDevice::Virtex4(edev) => match edev.kind {
            prjcombine_virtex4::chip::ChipKind::Virtex4 => {
                prjcombine_virtex4::defs::virtex4::wires::OUT_HALF0_TEST.contains(wire)
                    || prjcombine_virtex4::defs::virtex4::wires::OUT_HALF1_TEST.contains(wire)
            }
            prjcombine_virtex4::chip::ChipKind::Virtex5 => {
                prjcombine_virtex4::defs::virtex5::wires::OUT_TEST.contains(wire)
            }
            prjcombine_virtex4::chip::ChipKind::Virtex6 => {
                prjcombine_virtex4::defs::virtex6::wires::OUT_TEST.contains(wire)
            }
            prjcombine_virtex4::chip::ChipKind::Virtex7 => {
                prjcombine_virtex4::defs::virtex7::wires::OUT_TEST.contains(wire)
            }
        },
        _ => unreachable!(),
    }
}

fn skip_permabuf(
    edev: &ExpandedDevice,
    tcid: TileClassId,
    _bslot: BelSlotId,
    dst: TileWireCoord,
    src: TileWireCoord,
) -> bool {
    match edev {
        ExpandedDevice::Virtex2(edev) => {
            if !edev.chip.kind.is_virtex2() {
                if matches!(tcid, tcls_s3::CLK_S_S3E | tcls_s3::CLK_N_S3E)
                    && edev.chip.dcms == Some(prjcombine_virtex2::chip::Dcms::Two)
                    && wires_s3::DCM_CLKPAD.contains(dst.wire)
                    && dst.cell.to_idx() == 3
                {
                    return true;
                }
                if tcid == tcls_s3::CLK_S_S3A
                    && edev.chip.dcms == Some(prjcombine_virtex2::chip::Dcms::Two)
                    && wires_s3::DCM_CLKPAD.contains(dst.wire)
                {
                    return true;
                }
                if matches!(
                    tcid,
                    tcls_s3::CLK_W_S3E
                        | tcls_s3::CLK_E_S3E
                        | tcls_s3::CLK_W_S3A
                        | tcls_s3::CLK_E_S3A
                ) && wires_s3::DCM_CLKPAD.contains(dst.wire)
                {
                    return true;
                }
            }
        }
        ExpandedDevice::Spartan6(_) => {
            if matches!(
                tcid,
                tcls_s6::PLL_BUFPLL_S
                    | tcls_s6::PLL_BUFPLL_N
                    | tcls_s6::PLL_BUFPLL_OUT0_S
                    | tcls_s6::PLL_BUFPLL_OUT0_N
                    | tcls_s6::PLL_BUFPLL_OUT1_S
                    | tcls_s6::PLL_BUFPLL_OUT1_N
            ) && src.wire != wires_s6::OUT_PLL_LOCKED
            {
                return true;
            }
        }
        _ => (),
    }
    false
}

#[allow(clippy::single_match)]
fn skip_mux(
    edev: &ExpandedDevice,
    _tcid: TileClassId,
    bslot: BelSlotId,
    dst: TileWireCoord,
) -> bool {
    match edev {
        ExpandedDevice::Spartan6(_) => {
            if bslot == bslots_s6::IOI_INT {
                return true;
            }
            if bslot == bslots_s6::CLK_INT
                && (wires_s6::IMUX_BUFIO2_I.contains(dst.wire)
                    || wires_s6::IMUX_BUFIO2_IB.contains(dst.wire)
                    || wires_s6::IMUX_BUFIO2FB.contains(dst.wire)
                    || wires_s6::IMUX_BUFPLL_PLLIN.contains(dst.wire)
                    || wires_s6::IMUX_BUFPLL_LOCKED.contains(dst.wire))
            {
                return true;
            }
            if wires_s6::CMT_BUFPLL_V_CLKOUT_S.contains(dst.wire)
                || wires_s6::CMT_BUFPLL_V_CLKOUT_N.contains(dst.wire)
                || wires_s6::CMT_BUFPLL_H_CLKOUT.contains(dst.wire)
                || wires_s6::IMUX_DCM_CLKIN.contains(dst.wire)
                || wires_s6::IMUX_DCM_CLKFB.contains(dst.wire)
                || wires_s6::OMUX_DCM_SKEWCLKIN1.contains(dst.wire)
                || wires_s6::OMUX_DCM_SKEWCLKIN2.contains(dst.wire)
                || wires_s6::IMUX_PLL_CLKFB == dst.wire
                || wires_s6::IMUX_PLL_CLKIN1 == dst.wire
                || wires_s6::IMUX_PLL_CLKIN2 == dst.wire
                || wires_s6::OMUX_PLL_SKEWCLKIN1_BUF == dst.wire
                || wires_s6::OMUX_PLL_SKEWCLKIN2_BUF == dst.wire
                || wires_s6::OMUX_PLL_SKEWCLKIN1 == dst.wire
                || wires_s6::OMUX_PLL_SKEWCLKIN2 == dst.wire
                || wires_s6::CMT_TEST_CLK == dst.wire
            {
                return true;
            }
        }
        _ => (),
    }
    false
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, _, tcls) in &intdb.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for (bslot, bel) in &tcls.bels {
            if let ExpandedDevice::Virtex2(_) = backend.edev
                && bslot == prjcombine_virtex2::defs::bslots::PTE2OMUX
            {
                continue;
            }
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        if is_anon_wire(backend.edev, mux.dst.wire) {
                            continue;
                        }
                        if skip_mux(backend.edev, tcid, bslot, mux.dst) {
                            continue;
                        }
                        for &src in mux.src.keys() {
                            build_pip_fuzzer(backend, &mut ctx, mux.dst, src.tw, false);
                        }
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        if skip_mux(backend.edev, tcid, bslot, buf.dst) {
                            continue;
                        }
                        build_pip_fuzzer(backend, &mut ctx, buf.dst, buf.src.tw, false);
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        if skip_permabuf(backend.edev, tcid, bslot, buf.dst, buf.src.tw) {
                            continue;
                        }
                        build_pip_fuzzer(backend, &mut ctx, buf.dst, buf.src.tw, true);
                    }
                    SwitchBoxItem::ProgInv(_) => (),
                    SwitchBoxItem::ProgDelay(delay) => {
                        for val in 0..2 {
                            ctx.build()
                                .prop(IntMutex::new("INTF".into()))
                                .test_raw(DiffKey::ProgDelay(tcid, delay.dst, val))
                                .prop(TileMutexExclusive::new("INTF".into()))
                                .prop(WireMutexExclusive::new(delay.dst))
                                .prop(WireMutexExclusive::new(delay.src.tw))
                                .prop(FuzzIntfDelay::new(delay.clone(), val != 0))
                                .commit();
                        }
                    }
                    SwitchBoxItem::PairMux(_) => (),
                    _ => unreachable!(),
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            if let ExpandedDevice::Virtex2(_) = ctx.edev
                && bslot == prjcombine_virtex2::defs::bslots::PTE2OMUX
            {
                continue;
            }
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let mut is_empty_ok = false;
            if matches!(ctx.edev, ExpandedDevice::Virtex2(_))
                && bslot == prjcombine_virtex2::defs::bslots::MULT_INT
            {
                is_empty_ok = true;
            }
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        if is_anon_wire(ctx.edev, mux.dst.wire) {
                            continue;
                        }
                        if skip_mux(ctx.edev, tcid, bslot, mux.dst) {
                            continue;
                        }
                        let out_name = intdb.wires.key(mux.dst.wire);
                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &src in mux.src.keys() {
                            let in_name = intdb.wires.key(src.wire);
                            let mut diff = ctx.get_diff_routing(tcid, mux.dst, src);
                            let mut diff_fucked = false;
                            if let ExpandedDevice::Virtex2(edev) = ctx.edev
                                && edev.chip.kind
                                    == prjcombine_virtex2::chip::ChipKind::Spartan3ADsp
                                && tcid == tcls_s3::INT_IOI_S3A_WE
                                && mux.dst.wire
                                    == prjcombine_virtex2::defs::spartan3::wires::IMUX_DATA[3]
                                && src.wire == prjcombine_virtex2::defs::spartan3::wires::OMUX_N10
                            {
                                // ISE is bad and should feel bad.
                                diff_fucked = true;
                            }
                            if diff.bits.is_empty() {
                                if intdb.wires.key(mux.dst.wire).starts_with("IMUX")
                                    && !intdb[src.wire].is_tie()
                                    && !is_empty_ok
                                {
                                    if tcname.starts_with("INT_IOI_S3")
                                        && out_name.starts_with("IMUX_DATA")
                                        && (in_name.starts_with("OUT_FAN")
                                            || in_name.starts_with("IMUX_FAN")
                                            || in_name.starts_with("OMUX"))
                                    {
                                        // ISE is kind of bad.
                                        diff_fucked = true;
                                    } else {
                                        println!(
                                            "UMMMMM PIP {tcname} {mux_name} {in_name} is empty",
                                            mux_name = mux.dst.to_string(intdb, &intdb[tcid]),
                                            in_name = src.to_string(intdb, &intdb[tcid])
                                        );
                                        got_empty = true;
                                    }
                                } else {
                                    got_empty = true;
                                }
                            }
                            if diff_fucked {
                                let ExpandedDevice::Virtex2(edev) = ctx.edev else {
                                    unreachable!()
                                };
                                assert!(!edev.chip.kind.is_virtex2());
                                let mux = ctx.sb_mux(tcls_s3::INT_CLB, mux.dst);
                                diff.bits.clear();
                                for (idx, val) in mux.values[&Some(src)].iter().enumerate() {
                                    if val {
                                        diff.bits.insert(mux.bits[idx], true);
                                    }
                                }
                            }
                            inps.push((Some(src), diff));
                        }
                        if !got_empty {
                            inps.push((None, Diff::default()));
                        }
                        let ti = xlat_enum_raw(inps, OcdMode::Mux);
                        if ti.bits.is_empty() {
                            println!(
                                "UMMM MUX {tcname} {mux_name} is empty",
                                mux_name = mux.dst.to_string(intdb, &intdb[tcid])
                            );
                        }
                        ctx.insert_mux(tcid, mux.dst, ti);
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        if skip_mux(ctx.edev, tcid, bslot, buf.dst) {
                            continue;
                        }
                        ctx.collect_progbuf(tcid, buf.dst, buf.src);
                    }
                    SwitchBoxItem::PermaBuf(_) => (),
                    SwitchBoxItem::ProgInv(_) => (),
                    SwitchBoxItem::ProgDelay(delay) => {
                        ctx.collect_delay(tcid, delay.dst, delay.steps.len());
                    }
                    SwitchBoxItem::PairMux(_) => (),
                    _ => unreachable!(),
                }
            }
        }
    }
}
