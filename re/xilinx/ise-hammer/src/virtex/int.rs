use std::collections::HashSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{
        BelInfo, PolTileWireCoord, SwitchBoxItem, TileClassId, TileWireCoord, WireSlotId,
        WireSlotIdExt,
    },
    grid::{ColId, RowId, TileCoord},
};
use prjcombine_re_collector::diff::{Diff, OcdMode, xlat_bit, xlat_enum_raw};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_types::bitrect::BitRect as _;
use prjcombine_virtex::defs::{
    self, bcls::DLL, bslots, tcls, tslots, wire_from_mux, wire_to_mux, wires,
};

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip, resolve_int_pip},
        props::{
            BaseRaw, DynProp, NullBits,
            bel::{BaseBelMode, BaseBelPin, FuzzBelMode},
            mutex::WireMutexExclusive,
            relation::{Delta, Related},
        },
    },
    virtex::specials,
};

#[derive(Clone, Debug)]
struct VirtexPinBramLv(TileWireCoord);

fn pips_bwd(edev: &ExpandedDevice, tcid: TileClassId, tw: TileWireCoord) -> Vec<PolTileWireCoord> {
    let Some(bwd) = edev.db_index.tile_classes[tcid].pips_bwd.get(&tw) else {
        return vec![];
    };
    let mut res = vec![];
    for &w in bwd {
        if wire_to_mux(tw.wire) == Some(w.wire) {
            for &ww in &edev.db_index.tile_classes[tcid].pips_bwd[&w.tw] {
                res.push(ww);
            }
        } else {
            res.push(w);
        }
    }
    res
}

fn pips_fwd(edev: &ExpandedDevice, tcid: TileClassId, tw: TileWireCoord) -> Vec<PolTileWireCoord> {
    let Some(fwd) = edev.db_index.tile_classes[tcid].pips_fwd.get(&tw) else {
        return vec![];
    };
    let mut res = vec![];
    for &w in fwd {
        if let Some(ww) = wire_from_mux(w.wire) {
            res.push(PolTileWireCoord {
                tw: TileWireCoord {
                    wire: ww,
                    cell: w.cell,
                },
                inv: w.inv,
            });
        } else {
            res.push(w);
        }
    }
    res
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinBramLv {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.0))?;
        let mut tcrd = tcrd;
        tcrd.row = RowId::from_idx(1);
        tcrd.slot = defs::tslots::MAIN;
        for i in 0..12 {
            let wire_pin = TileWireCoord::new_idx(0, wires::LV[i]);
            let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
            let wire_clk = TileWireCoord::new_idx(0, wires::IMUX_BRAM_CLKA);
            let resolved_clk = backend.edev.resolve_wire(tcrd.wire(wire_clk.wire)).unwrap();
            if resolved_pin == wire {
                let (tile, wt, wf) = resolve_int_pip(backend, tcrd, wire_clk, wire_pin).unwrap();
                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_clk), None, "EXCLUSIVE");
                return Some((fuzzer, false));
            }
        }
        panic!("UMM FAILED TO PIN BRAM LV");
    }
}

#[derive(Clone, Debug)]
struct VirtexPinLh(TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinLh {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let resolved_wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.0))?;
        let tcrd = backend
            .edev
            .tile_cell(tcrd, self.0.cell)
            .with_col(ColId::from_idx(0))
            .tile(defs::tslots::MAIN);
        let tile = &backend.edev[tcrd];
        for i in 0..12 {
            let wire_pin = TileWireCoord::new_idx(0, wires::LH[i]);
            let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
            if resolved_pin != resolved_wire {
                continue;
            }
            if let Some(&wire_out) = pips_fwd(backend.edev, tile.class, wire_pin).first() {
                // FOUND
                let resolved_out = backend.edev.resolve_wire(tcrd.wire(wire_out.wire)).unwrap();
                let (tile, wt, wf) = resolve_int_pip(backend, tcrd, wire_out.tw, wire_pin).unwrap();
                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_out), None, "EXCLUSIVE");
                return Some((fuzzer, false));
            }
        }
        unreachable!()
    }
}

#[derive(Clone, Debug)]
struct VirtexPinIoLh(TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinIoLh {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let resolved_wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.0))?;
        let mut tcrd = resolved_wire
            .with_col(ColId::from_idx(0))
            .tile(defs::tslots::MAIN);
        loop {
            let tile = &backend.edev[tcrd];
            if matches!(tile.class, tcls::IO_S | tcls::IO_N) {
                for i in [0, 6] {
                    let wire_pin = wires::LH[i].cell(0);
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin == resolved_wire {
                        // FOUND
                        return BaseFakeLhPip(wire_pin).apply(backend, tcrd, fuzzer);
                    }
                }
            }
            tcrd.col += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct VirtexPinHexH(TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinHexH {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let resolved_wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.0))?;
        let wire_name = backend.edev.db.wires.key(self.0.wire);
        let h = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[7..8].parse().unwrap();
        let mut tcrd = backend
            .edev
            .tile_cell(tcrd, self.0.cell)
            .tile(defs::tslots::MAIN);
        if tcrd.col.to_idx() >= 8 {
            tcrd.col -= 8;
        } else {
            tcrd.col = ColId::from_idx(0)
        };
        loop {
            if let Some(tile) = backend.edev.get_tile(tcrd)
                && matches!(
                    tile.class,
                    tcls::IO_W
                        | tcls::IO_E
                        | tcls::IO_S
                        | tcls::IO_N
                        | tcls::CLB
                        | tcls::CNR_SE
                        | tcls::CNR_NE
                )
            {
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.edev.db.get_wire(&format!("HEX_{h}{j}[{i}]")),
                    );
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for wire_out in pips_fwd(backend.edev, tile.class, wire_pin) {
                        let out_name = backend.edev.db.wires.key(wire_out.wire);
                        if out_name.starts_with("SINGLE")
                            || (out_name.starts_with("LV") && i >= 4)
                            || (out_name.starts_with("HEX_E") && tile.class == tcls::IO_W)
                            || (out_name.starts_with("HEX_W") && tile.class == tcls::IO_E)
                        {
                            // FOUND
                            let resolved_out =
                                backend.edev.resolve_wire(tcrd.wire(wire_out.wire)).unwrap();
                            let (tile, wt, wf) =
                                resolve_int_pip(backend, tcrd, wire_out.tw, wire_pin).unwrap();
                            fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                            fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_out), None, "EXCLUSIVE");
                            return Some((fuzzer, false));
                        }
                    }
                }
            }
            tcrd.col += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct VirtexPinHexV(TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexPinHexV {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let resolved_wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.0))?;
        let wire_name = backend.edev.db.wires.key(self.0.wire);
        let v = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[7..8].parse().unwrap();
        let mut tcrd = backend
            .edev
            .tile_cell(tcrd, self.0.cell)
            .tile(defs::tslots::MAIN);
        if tcrd.row.to_idx() >= 6 {
            tcrd.row -= 6;
        } else {
            tcrd.row = RowId::from_idx(0)
        };
        loop {
            if let Some(tile) = backend.edev.get_tile(tcrd)
                && matches!(
                    tile.class,
                    tcls::IO_W | tcls::IO_E | tcls::CLB | tcls::IO_S | tcls::IO_N
                )
            {
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.edev.db.get_wire(&format!("HEX_{v}{j}[{i}]")),
                    );
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for wire_out in pips_fwd(backend.edev, tile.class, wire_pin) {
                        let out_name = backend.edev.db.wires.key(wire_out.wire);
                        if out_name.starts_with("SINGLE")
                            || (out_name.starts_with("HEX_N") && tile.class == tcls::IO_S)
                            || (out_name.starts_with("HEX_S") && tile.class == tcls::IO_N)
                        {
                            // FOUND
                            let resolved_out =
                                backend.edev.resolve_wire(tcrd.wire(wire_out.wire)).unwrap();
                            let (tile, wt, wf) =
                                resolve_int_pip(backend, tcrd, wire_out.tw, wire_pin).unwrap();
                            fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                            fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_out), None, "EXCLUSIVE");
                            return Some((fuzzer, false));
                        }
                    }
                }
            }
            tcrd.row += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct VirtexDriveHexH(TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexDriveHexH {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let resolved_wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.0))?;
        let wire_name = backend.edev.db.wires.key(self.0.wire);
        let h = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[7..8].parse().unwrap();
        let mut tcrd = backend
            .edev
            .tile_cell(tcrd, self.0.cell)
            .tile(defs::tslots::MAIN);
        if tcrd.col.to_idx() >= 8 {
            tcrd.col -= 8;
        } else {
            tcrd.col = ColId::from_idx(0)
        };
        loop {
            if let Some(tile) = backend.edev.get_tile(tcrd)
                && matches!(
                    tile.class,
                    tcls::IO_W | tcls::IO_E | tcls::CLB | tcls::IO_S | tcls::IO_N
                )
            {
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.edev.db.get_wire(&format!("HEX_{h}{j}[{i}]")),
                    );
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for inp in pips_bwd(backend.edev, tile.class, wire_pin) {
                        let inp_name = backend.edev.db.wires.key(inp.wire);
                        if inp_name.starts_with("OMUX")
                            || inp_name.starts_with("OUT")
                            || (h == 'E' && tile.class == tcls::IO_W && inp_name.starts_with("HEX"))
                            || (h == 'W' && tile.class == tcls::IO_E && inp_name.starts_with("HEX"))
                        {
                            // FOUND
                            let resolved_inp =
                                backend.edev.resolve_wire(tcrd.wire(inp.wire)).unwrap();
                            let (tile, wt, wf) =
                                resolve_int_pip(backend, tcrd, wire_pin, inp.tw).unwrap();
                            fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                            fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_inp), None, "EXCLUSIVE");
                            fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_pin), None, "EXCLUSIVE");
                            return Some((fuzzer, false));
                        }
                    }
                }
            }
            tcrd.col += 1;
        }
    }
}

#[derive(Clone, Debug)]
struct VirtexDriveHexV(TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexDriveHexV {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let resolved_wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.0))?;
        let wire_name = backend.edev.db.wires.key(self.0.wire);
        let v = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[7..8].parse().unwrap();
        let mut tcrd = backend
            .edev
            .tile_cell(tcrd, self.0.cell)
            .tile(defs::tslots::MAIN);

        if tcrd.row.to_idx() >= 6 {
            tcrd.row -= 6;
        } else {
            tcrd.row = RowId::from_idx(0)
        };
        loop {
            if let Some(tile) = backend.edev.get_tile(tcrd)
                && matches!(
                    tile.class,
                    tcls::IO_W | tcls::IO_E | tcls::CLB | tcls::IO_S | tcls::IO_N
                )
            {
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.edev.db.get_wire(&format!("HEX_{v}{j}[{i}]")),
                    );
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for inp in pips_bwd(backend.edev, tile.class, wire_pin) {
                        let inp_name = backend.edev.db.wires.key(inp.wire);
                        if inp_name.starts_with("OMUX")
                            || inp_name.starts_with("OUT")
                            || (v == 'N' && tile.class == tcls::IO_S && inp_name.starts_with("HEX"))
                            || (v == 'S' && tile.class == tcls::IO_N && inp_name.starts_with("HEX"))
                        {
                            // FOUND
                            let resolved_inp =
                                backend.edev.resolve_wire(tcrd.wire(inp.wire)).unwrap();
                            let (tile, wt, wf) =
                                resolve_int_pip(backend, tcrd, wire_pin, inp.tw).unwrap();
                            fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                            fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_inp), None, "EXCLUSIVE");
                            fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_pin), None, "EXCLUSIVE");
                            return Some((fuzzer, false));
                        }
                    }
                }
            }
            tcrd.row += 1;
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BaseFakeLhPip(TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseFakeLhPip {
    fn dyn_clone(&self) -> Box<dyn FuzzerProp<'b, IseBackend<'b>> + 'b> {
        Box::new(*self)
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ntile = &backend.endev.ngrid().tiles[&tcrd];
        let tile = ntile.names[RawTileId::from_idx(0)].as_str();
        let tn = &backend.endev.ngrid().db.tile_class_namings[ntile.naming];
        let wn = &tn.wires[&self.0];
        let wt = wn.alt_name.as_ref().unwrap();
        let wf = wn.name.as_str();
        fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
        Some((fuzzer, false))
    }
}

fn single_to_buf(wire: WireSlotId) -> WireSlotId {
    if let Some(idx) = wires::SINGLE_W.index_of(wire) {
        wires::SINGLE_W_BUF[idx]
    } else if let Some(idx) = wires::SINGLE_E.index_of(wire) {
        wires::SINGLE_E_BUF[idx]
    } else if let Some(idx) = wires::SINGLE_S.index_of(wire) {
        wires::SINGLE_S_BUF[idx]
    } else if let Some(idx) = wires::SINGLE_N.index_of(wire) {
        wires::SINGLE_N_BUF[idx]
    } else {
        unreachable!()
    }
}

fn hex_to_buf(wire: WireSlotId) -> WireSlotId {
    if let Some(idx) = wires::HEX_H0.index_of(wire) {
        wires::HEX_H0_BUF[idx]
    } else if let Some(idx) = wires::HEX_H1.index_of(wire) {
        wires::HEX_H1_BUF[idx]
    } else if let Some(idx) = wires::HEX_H2.index_of(wire) {
        wires::HEX_H2_BUF[idx]
    } else if let Some(idx) = wires::HEX_H3.index_of(wire) {
        wires::HEX_H3_BUF[idx]
    } else if let Some(idx) = wires::HEX_H4.index_of(wire) {
        wires::HEX_H4_BUF[idx]
    } else if let Some(idx) = wires::HEX_H5.index_of(wire) {
        wires::HEX_H5_BUF[idx]
    } else if let Some(idx) = wires::HEX_H6.index_of(wire) {
        wires::HEX_H6_BUF[idx]
    } else if let Some(idx) = wires::HEX_V0.index_of(wire) {
        wires::HEX_V0_BUF[idx]
    } else if let Some(idx) = wires::HEX_V1.index_of(wire) {
        wires::HEX_V1_BUF[idx]
    } else if let Some(idx) = wires::HEX_V2.index_of(wire) {
        wires::HEX_V2_BUF[idx]
    } else if let Some(idx) = wires::HEX_V3.index_of(wire) {
        wires::HEX_V3_BUF[idx]
    } else if let Some(idx) = wires::HEX_V4.index_of(wire) {
        wires::HEX_V4_BUF[idx]
    } else if let Some(idx) = wires::HEX_V5.index_of(wire) {
        wires::HEX_V5_BUF[idx]
    } else if let Some(idx) = wires::HEX_V6.index_of(wire) {
        wires::HEX_V6_BUF[idx]
    } else {
        unreachable!()
    }
}

fn wire_unbuf(wire: WireSlotId) -> Option<WireSlotId> {
    if let Some(idx) = wires::GCLK_BUF.index_of(wire) {
        Some(wires::GCLK_LEAF[idx])
    } else if let Some(idx) = wires::SINGLE_W_BUF.index_of(wire) {
        Some(wires::SINGLE_W[idx])
    } else if let Some(idx) = wires::SINGLE_E_BUF.index_of(wire) {
        Some(wires::SINGLE_E[idx])
    } else if let Some(idx) = wires::SINGLE_S_BUF.index_of(wire) {
        Some(wires::SINGLE_S[idx])
    } else if let Some(idx) = wires::SINGLE_N_BUF.index_of(wire) {
        Some(wires::SINGLE_N[idx])
    } else if let Some(idx) = wires::HEX_H0_BUF.index_of(wire) {
        Some(wires::HEX_H0[idx])
    } else if let Some(idx) = wires::HEX_H1_BUF.index_of(wire) {
        Some(wires::HEX_H1[idx])
    } else if let Some(idx) = wires::HEX_H2_BUF.index_of(wire) {
        Some(wires::HEX_H2[idx])
    } else if let Some(idx) = wires::HEX_H3_BUF.index_of(wire) {
        Some(wires::HEX_H3[idx])
    } else if let Some(idx) = wires::HEX_H4_BUF.index_of(wire) {
        Some(wires::HEX_H4[idx])
    } else if let Some(idx) = wires::HEX_H5_BUF.index_of(wire) {
        Some(wires::HEX_H5[idx])
    } else if let Some(idx) = wires::HEX_H6_BUF.index_of(wire) {
        Some(wires::HEX_H6[idx])
    } else if let Some(idx) = wires::HEX_V0_BUF.index_of(wire) {
        Some(wires::HEX_V0[idx])
    } else if let Some(idx) = wires::HEX_V1_BUF.index_of(wire) {
        Some(wires::HEX_V1[idx])
    } else if let Some(idx) = wires::HEX_V2_BUF.index_of(wire) {
        Some(wires::HEX_V2[idx])
    } else if let Some(idx) = wires::HEX_V3_BUF.index_of(wire) {
        Some(wires::HEX_V3[idx])
    } else if let Some(idx) = wires::HEX_V4_BUF.index_of(wire) {
        Some(wires::HEX_V4[idx])
    } else if let Some(idx) = wires::HEX_V5_BUF.index_of(wire) {
        Some(wires::HEX_V5[idx])
    } else if let Some(idx) = wires::HEX_V6_BUF.index_of(wire) {
        Some(wires::HEX_V6[idx])
    } else {
        None
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for &wire_to in backend.edev.db_index[tcid].pips_bwd.keys() {
            if wire_from_mux(wire_to.wire).is_some() {
                continue;
            }
            let ins = &pips_bwd(backend.edev, tcid, wire_to);
            let out_name = intdb.wires.key(wire_to.wire);
            if wire_unbuf(wire_to.wire).is_some() {
                continue;
            } else if out_name.contains("OMUX") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];
                if matches!(tcid, tcls::IO_W | tcls::IO_E) {
                    for i in 0..4 {
                        props.push(Box::new(BaseBelMode::new(
                            bslots::IOI[i],
                            0,
                            ["EMPTYIOB", "IOB", "IOB", "IOB"][i].into(),
                        )));
                        props.push(Box::new(BaseBelPin::new(bslots::IOI[i], 0, "I".into())));
                    }
                    let idx = wires::OMUX.index_of(wire_to.wire).unwrap();
                    let clb_wire = if tcid == tcls::IO_W {
                        match idx {
                            0 => wires::OMUX_E0,
                            1 => wires::OMUX_E1,
                            _ => unreachable!(),
                        }
                    } else {
                        match idx {
                            6 => wires::OMUX_W6,
                            7 => wires::OMUX_W7,
                            _ => unreachable!(),
                        }
                    };
                    let clb_wire = TileWireCoord::new_idx(0, clb_wire);
                    let wire_pin = pips_fwd(backend.edev, tcls::CLB, clb_wire)
                        .first()
                        .unwrap()
                        .tw;
                    let relation = if tcid == tcls::IO_W {
                        Delta::new(2, 0, tcls::CLB)
                    } else {
                        Delta::new(-2, 0, tcls::CLB)
                    };
                    props.push(Box::new(Related::new(
                        relation.clone(),
                        BaseIntPip::new(wire_pin, clb_wire),
                    )));
                    props.push(Box::new(Related::new(
                        relation,
                        WireMutexExclusive::new(wire_pin),
                    )));
                } else {
                    let wire_pin = pips_fwd(backend.edev, tcid, wire_to).first().unwrap().tw;
                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_to)));
                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                }
                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let mut builder = ctx
                        .build()
                        .test_routing(wire_to, wire_from.pos())
                        .prop(FuzzIntPip::new(wire_to, wire_from));
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.starts_with("BRAM_QUAD") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];

                let (is_s, wire_to_root) =
                    if let Some(idx) = wires::BRAM_QUAD_ADDR_S.index_of(wire_to.wire) {
                        (
                            true,
                            TileWireCoord {
                                cell: wire_to.cell,
                                wire: wires::BRAM_QUAD_ADDR[idx],
                            },
                        )
                    } else if let Some(idx) = wires::BRAM_QUAD_DIN_S.index_of(wire_to.wire) {
                        (
                            true,
                            TileWireCoord {
                                cell: wire_to.cell,
                                wire: wires::BRAM_QUAD_DIN[idx],
                            },
                        )
                    } else if let Some(idx) = wires::BRAM_QUAD_DOUT_S.index_of(wire_to.wire) {
                        (
                            true,
                            TileWireCoord {
                                cell: wire_to.cell,
                                wire: wires::BRAM_QUAD_DOUT[idx],
                            },
                        )
                    } else {
                        (false, wire_to)
                    };
                let wire_pin = 'quad_dst_pin: {
                    for wire_pin in pips_fwd(backend.edev, tcid, wire_to_root) {
                        let wire_pin = wire_pin.tw;
                        let wire_pin_name = intdb.wires.key(wire_pin.wire);
                        if wire_pin_name.starts_with("IMUX") || wire_pin_name.starts_with("HEX") {
                            break 'quad_dst_pin wire_pin;
                        }
                    }
                    panic!(
                        "NO WAY TO PIN {tcname} {dst}",
                        dst = wire_to.to_string(backend.edev.db, tcls)
                    );
                };
                if !is_s {
                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_to)));
                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                } else {
                    let related = Delta::new(0, 4, tcid);
                    props.push(Box::new(Related::new(
                        related.clone(),
                        BaseIntPip::new(wire_pin, wire_to_root),
                    )));
                    props.push(Box::new(Related::new(
                        related,
                        WireMutexExclusive::new(wire_pin),
                    )));
                }
                if !out_name.starts_with("BRAM_QUAD_DOUT") {
                    // pin every input
                    let mut pins = HashSet::new();
                    for &wire_from in ins {
                        let wire_from = wire_from.tw;
                        let in_wire_name = intdb.wires.key(wire_from.wire);
                        'quad_src_all_pin: {
                            if in_wire_name.starts_with("SINGLE") {
                                let wire_buf =
                                    TileWireCoord::new_idx(0, single_to_buf(wire_from.wire));
                                let related = Delta::new(
                                    -1,
                                    wire_from.cell.to_idx() as i32 - 4,
                                    if matches!(tcid, tcls::BRAM_W | tcls::BRAM_W_S2) {
                                        tcls::IO_W
                                    } else {
                                        tcls::CLB
                                    },
                                );
                                props.push(Box::new(Related::new(
                                    related.clone(),
                                    BaseIntPip::new(
                                        wire_buf,
                                        TileWireCoord::new_idx(0, wire_from.wire),
                                    ),
                                )));
                                props.push(Box::new(Related::new(
                                    related,
                                    WireMutexExclusive::new(wire_buf),
                                )));
                                props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                break 'quad_src_all_pin;
                            } else if in_wire_name.starts_with("HEX") {
                                for wire_pin in pips_fwd(backend.edev, tcid, wire_from) {
                                    let wire_pin = wire_pin.tw;
                                    if wire_pin != wire_to && !pins.contains(&wire_pin) {
                                        props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                        pins.insert(wire_pin);
                                        break 'quad_src_all_pin;
                                    }
                                }
                            } else {
                                break 'quad_src_all_pin;
                            }
                            panic!(
                                "NO WAY TO PIN {tcname} {dst} {src}",
                                dst = wire_to.to_string(backend.edev.db, tcls),
                                src = wire_from.to_string(backend.edev.db, tcls),
                            );
                        }
                    }
                }
                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let in_wire_name = intdb.wires.key(wire_from.wire);
                    let mut props = props.clone();
                    if in_wire_name.starts_with("BRAM_QUAD") {
                        'quad_src_pin: {
                            let (is_s, wire_from_root) = if let Some(idx) =
                                wires::BRAM_QUAD_ADDR_S.index_of(wire_from.wire)
                            {
                                (
                                    true,
                                    TileWireCoord {
                                        cell: wire_from.cell,
                                        wire: wires::BRAM_QUAD_ADDR[idx],
                                    },
                                )
                            } else if let Some(idx) =
                                wires::BRAM_QUAD_DIN_S.index_of(wire_from.wire)
                            {
                                (
                                    true,
                                    TileWireCoord {
                                        cell: wire_from.cell,
                                        wire: wires::BRAM_QUAD_DIN[idx],
                                    },
                                )
                            } else if let Some(idx) =
                                wires::BRAM_QUAD_DOUT_S.index_of(wire_from.wire)
                            {
                                (
                                    true,
                                    TileWireCoord {
                                        cell: wire_from.cell,
                                        wire: wires::BRAM_QUAD_DOUT[idx],
                                    },
                                )
                            } else {
                                (false, wire_from)
                            };

                            for wire_pin in pips_bwd(backend.edev, tcid, wire_from_root) {
                                let wire_pin = wire_pin.tw;
                                let wire_pin_name = intdb.wires.key(wire_pin.wire);
                                if intdb.wires.key(wire_pin.wire).starts_with("HEX")
                                    || wire_pin_name.starts_with("OUT")
                                {
                                    if !is_s {
                                        props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                    } else {
                                        let related = Delta::new(0, 4, tcid);
                                        props.push(Box::new(Related::new(
                                            related.clone(),
                                            BaseIntPip::new(wire_from_root, wire_pin),
                                        )));
                                        props.push(Box::new(Related::new(
                                            related.clone(),
                                            WireMutexExclusive::new(wire_pin),
                                        )));
                                        props.push(Box::new(Related::new(
                                            related,
                                            WireMutexExclusive::new(wire_from_root),
                                        )));
                                    }
                                    break 'quad_src_pin;
                                }
                            }
                            panic!(
                                "NO WAY TO PIN {tcname} {dst} {src}",
                                dst = wire_to.to_string(backend.edev.db, tcls),
                                src = wire_from.to_string(backend.edev.db, tcls),
                            );
                        }
                    }
                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    let mut builder = ctx.build().test_routing(wire_to, wire_from.pos());
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.starts_with("SINGLE") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];

                let wire_buf = TileWireCoord::new_idx(0, single_to_buf(wire_to.wire));
                if !tcname.contains("BRAM") {
                    props.push(Box::new(BaseIntPip::new(wire_buf, wire_to)));
                    props.push(Box::new(WireMutexExclusive::new(wire_buf)));
                } else {
                    let related = Delta::new(
                        -1,
                        wire_to.cell.to_idx() as i32 - 4,
                        if matches!(tcid, tcls::BRAM_W | tcls::BRAM_W_S2) {
                            tcls::IO_W
                        } else {
                            tcls::CLB
                        },
                    );
                    props.push(Box::new(Related::new(
                        related.clone(),
                        BaseIntPip::new(wire_buf, TileWireCoord::new_idx(0, wire_to.wire)),
                    )));
                    props.push(Box::new(Related::new(
                        related,
                        WireMutexExclusive::new(wire_buf),
                    )));
                }
                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let in_wire_name = intdb.wires.key(wire_from.wire);

                    let mut props = props.clone();
                    'single_pin: {
                        if in_wire_name.starts_with("SINGLE") {
                            for wire_pin in pips_bwd(backend.edev, tcid, wire_from) {
                                let wire_pin = wire_pin.tw;
                                let wire_pin_name = intdb.wires.key(wire_pin.wire);
                                if intdb.wires.key(wire_pin.wire).starts_with("HEX")
                                    || wire_pin_name.starts_with("OMUX")
                                    || wire_pin_name.starts_with("BRAM_QUAD_DOUT")
                                {
                                    props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                    break 'single_pin;
                                }
                            }
                        } else {
                            for wire_pin in pips_fwd(backend.edev, tcid, wire_from) {
                                let wire_pin = wire_pin.tw;
                                let wire_pin_name = intdb.wires.key(wire_pin.wire);
                                if wire_pin != wire_to && wire_pin_name.starts_with("SINGLE") {
                                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                    break 'single_pin;
                                }
                            }
                        }
                        panic!(
                            "NO WAY TO PIN {tcname} {dst} {src}",
                            dst = wire_to.to_string(backend.edev.db, tcls),
                            src = wire_from.to_string(backend.edev.db, tcls),
                        );
                    };

                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    let mut builder = ctx.build().test_routing(wire_to, wire_from.pos());
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.starts_with("LH")
                || out_name.starts_with("LV")
                || out_name.starts_with("HEX")
            {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];

                if wires::HEX_W2.contains(wire_to.wire)
                    || wires::HEX_W3.contains(wire_to.wire)
                    || wires::HEX_W4.contains(wire_to.wire)
                    || wires::HEX_W5.contains(wire_to.wire)
                    || wires::HEX_E2.contains(wire_to.wire)
                    || wires::HEX_E3.contains(wire_to.wire)
                    || wires::HEX_E4.contains(wire_to.wire)
                    || wires::HEX_E5.contains(wire_to.wire)
                    || wires::HEX_S2.contains(wire_to.wire)
                    || wires::HEX_S3.contains(wire_to.wire)
                    || wires::HEX_S4.contains(wire_to.wire)
                    || wires::HEX_S5.contains(wire_to.wire)
                    || wires::HEX_N2.contains(wire_to.wire)
                    || wires::HEX_N3.contains(wire_to.wire)
                    || wires::HEX_N4.contains(wire_to.wire)
                    || wires::HEX_N5.contains(wire_to.wire)
                {
                    props.push(Box::new(NullBits));
                }

                if out_name.starts_with("LH") && matches!(tcid, tcls::IO_S | tcls::IO_N) {
                    props.push(Box::new(BaseFakeLhPip(wire_to)));
                } else if out_name.starts_with("LV") && matches!(tcid, tcls::BRAM_S | tcls::BRAM_N)
                {
                    props.push(Box::new(VirtexPinBramLv(wire_to)));
                } else if out_name.starts_with("LH")
                    && matches!(
                        tcid,
                        tcls::BRAM_W
                            | tcls::BRAM_E
                            | tcls::BRAM_M
                            | tcls::BRAM_W_S2
                            | tcls::BRAM_E_S2
                    )
                {
                    props.push(Box::new(VirtexPinLh(wire_to)));
                } else if out_name.starts_with("LH") && tcls.slot == tslots::CLK {
                    props.push(Box::new(VirtexPinIoLh(wire_to)));
                } else if out_name.starts_with("HEX_H")
                    || out_name.starts_with("HEX_E")
                    || out_name.starts_with("HEX_W")
                {
                    props.push(Box::new(VirtexPinHexH(wire_to)));
                } else if out_name.starts_with("HEX_V")
                    || out_name.starts_with("HEX_S")
                    || out_name.starts_with("HEX_N")
                {
                    props.push(Box::new(VirtexPinHexV(wire_to)));
                } else {
                    'll_pin: {
                        for wire_pin in pips_fwd(backend.edev, tcid, wire_to) {
                            let wire_pin = wire_pin.tw;
                            let wire_pin_name = intdb.wires.key(wire_pin.wire);
                            if wire_pin_name.starts_with("HEX")
                                || wire_pin_name.starts_with("IMUX_BRAM")
                            {
                                props.push(Box::new(BaseIntPip::new(wire_pin, wire_to)));
                                props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                break 'll_pin;
                            }
                        }
                        panic!(
                            "NO WAY TO PIN {tcname} {dst}",
                            dst = wire_to.to_string(backend.edev.db, tcls),
                        );
                    }
                }

                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let in_wire_name = intdb.wires.key(wire_from.wire);
                    'll_src_pin: {
                        if let Some(wire_unbuf) = wire_unbuf(wire_from.wire) {
                            let wire_unbuf = TileWireCoord::new_idx(0, wire_unbuf);
                            props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                            props.push(Box::new(WireMutexExclusive::new(wire_unbuf)));
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("OMUX")
                            || in_wire_name.starts_with("BRAM_QUAD_DOUT")
                        {
                            for wire_pin in pips_bwd(backend.edev, tcid, wire_from) {
                                let wire_pin = wire_pin.tw;
                                if intdb.wires.key(wire_pin.wire).starts_with("OUT") {
                                    props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                    break 'll_src_pin;
                                }
                            }
                        } else if in_wire_name.starts_with("HEX") {
                            if in_wire_name.starts_with("HEX_E")
                                || in_wire_name.starts_with("HEX_W")
                                || in_wire_name.starts_with("HEX_H")
                            {
                                props.push(Box::new(VirtexDriveHexH(wire_from)));
                            } else {
                                props.push(Box::new(VirtexDriveHexV(wire_from)));
                            }
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("LH")
                            && matches!(
                                tcid,
                                tcls::CNR_SW
                                    | tcls::CNR_SE
                                    | tcls::CNR_NW
                                    | tcls::CNR_NE
                                    | tcls::CNR_SW_S2
                                    | tcls::CNR_NW_S2
                            )
                        {
                            // it's fine.
                            props.push(Box::new(VirtexPinIoLh(wire_from)));
                            break 'll_src_pin;
                        } else if wires::LH.contains(wire_from.wire)
                            || wires::LV.contains(wire_from.wire)
                        {
                            let extra_in = if matches!(tcid, tcls::IO_S | tcls::IO_N)
                                && let Some(idx) = wires::LV.index_of(wire_from.wire)
                            {
                                match idx {
                                    0 => Some(wires::OUT_IO_IQ[0].cell(0).pos()),
                                    1 => Some(wires::OUT_IO_I[0].cell(0).pos()),
                                    10 => Some(wires::OUT_IO_IQ[3].cell(0).pos()),
                                    11 => Some(wires::OUT_IO_I[3].cell(0).pos()),
                                    _ => None,
                                }
                            } else {
                                None
                            };
                            for wire_pin in pips_bwd(backend.edev, tcid, wire_from)
                                .into_iter()
                                .chain(extra_in)
                            {
                                let wire_pin = wire_pin.tw;
                                if intdb.wires.key(wire_pin.wire).starts_with("OMUX")
                                    || intdb.wires.key(wire_pin.wire).starts_with("OUT")
                                    || (intdb.wires.key(wire_pin.wire).starts_with("HEX")
                                        && tcname.starts_with("CNR"))
                                {
                                    props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                    break 'll_src_pin;
                                }
                            }
                        } else if in_wire_name.starts_with("SINGLE") {
                            let wire_buf = TileWireCoord::new_idx(0, single_to_buf(wire_from.wire));
                            if matches!(
                                tcid,
                                tcls::BRAM_W
                                    | tcls::BRAM_E
                                    | tcls::BRAM_M
                                    | tcls::BRAM_W_S2
                                    | tcls::BRAM_E_S2
                            ) {
                                let related = Delta::new(
                                    -1,
                                    wire_from.cell.to_idx() as i32 - 4,
                                    if matches!(tcid, tcls::BRAM_W | tcls::BRAM_W_S2) {
                                        tcls::IO_W
                                    } else {
                                        tcls::CLB
                                    },
                                );
                                props.push(Box::new(Related::new(
                                    related.clone(),
                                    BaseIntPip::new(
                                        wire_buf,
                                        TileWireCoord::new_idx(0, wire_from.wire),
                                    ),
                                )));
                                props.push(Box::new(Related::new(
                                    related,
                                    WireMutexExclusive::new(wire_buf),
                                )));
                                props.push(Box::new(WireMutexExclusive::new(wire_from)));
                            } else {
                                props.push(Box::new(BaseIntPip::new(wire_buf, wire_from)));
                                props.push(Box::new(WireMutexExclusive::new(wire_buf)));
                                props.push(Box::new(WireMutexExclusive::new(wire_from)));
                            }
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("OUT_IO") {
                            for i in 0..4 {
                                props.push(Box::new(BaseBelMode::new(
                                    bslots::IOI[i],
                                    0,
                                    [
                                        "EMPTYIOB",
                                        "IOB",
                                        "IOB",
                                        if matches!(tcid, tcls::IO_W | tcls::IO_E) {
                                            "IOB"
                                        } else {
                                            "EMPTYIOB"
                                        },
                                    ][i]
                                        .into(),
                                )));
                                props.push(Box::new(BaseBelPin::new(
                                    bslots::IOI[i],
                                    0,
                                    "I".into(),
                                )));
                                props.push(Box::new(BaseBelPin::new(
                                    bslots::IOI[i],
                                    0,
                                    "IQ".into(),
                                )));
                            }
                            break 'll_src_pin;
                        } else if let Some(pin) = in_wire_name.strip_prefix("OUT_BSCAN_") {
                            props.push(Box::new(BaseBelMode::new(
                                bslots::BSCAN,
                                0,
                                "BSCAN".into(),
                            )));
                            props.push(Box::new(BaseBelPin::new(bslots::BSCAN, 0, pin.into())));
                            break 'll_src_pin;
                        } else if wires::OUT_BUFGCE_O.contains(wire_from.wire)
                            || wires::OUT_CLKPAD.contains(wire_from.wire)
                            || wires::OUT_IOFB.contains(wire_from.wire)
                            || in_wire_name.starts_with("OUT_DLL")
                            || wire_from.wire == wires::PCI_CE
                        {
                            // already ok
                            break 'll_src_pin;
                        }
                        panic!(
                            "NO WAY TO PIN {tcname} {dst} {src}",
                            dst = wire_to.to_string(backend.edev.db, tcls),
                            src = wire_from.to_string(backend.edev.db, tcls),
                        );
                    };
                }

                for &wire_from in ins {
                    let wire_from = wire_from.tw;

                    let mut props = props.clone();
                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));

                    let mut builder = ctx.build().test_routing(wire_to, wire_from.pos());
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.contains("IMUX") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];
                if let Some(pin) = out_name.strip_prefix("IMUX_STARTUP_") {
                    props.push(Box::new(BaseBelMode::new(
                        bslots::STARTUP,
                        0,
                        "STARTUP".into(),
                    )));
                    props.push(Box::new(BaseBelPin::new(bslots::STARTUP, 0, pin.into())));
                }
                let mut alt_out_wire = None;
                if out_name.starts_with("IMUX_DLL") {
                    for i in 0..4 {
                        for ps in ["", "P", "S"] {
                            props.push(Box::new(BaseRaw::new(
                                Key::GlobalOpt(format!("IDLL{i}{ps}FB2X")),
                                "0".into(),
                            )))
                        }
                    }
                    if wire_to.wire == wires::IMUX_DLL_CLKIN {
                        alt_out_wire = Some(TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB));
                    }
                    if wire_to.wire == wires::IMUX_DLL_CLKFB {
                        alt_out_wire = Some(TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN));
                    }
                }
                if let Some(alt_out) = alt_out_wire {
                    props.push(Box::new(WireMutexExclusive::new(alt_out)));
                }
                if let Some(idx) = wires::IMUX_BUFGCE_CLK.index_of(wire_to.wire) {
                    props.push(Box::new(FuzzBelMode::new(
                        bslots::BUFGCE[idx],
                        0,
                        "".into(),
                        "GCLK".into(),
                    )));
                }
                if wires::IMUX_TBUF_I.contains(wire_to.wire)
                    || wires::IMUX_BRAM_DIA.contains(wire_to.wire)
                    || wires::IMUX_BRAM_DIB.contains(wire_to.wire)
                {
                    for &wire_from in ins {
                        if wire_from.wire == wires::PULLUP {
                            continue;
                        }
                        let wire_from = wire_from.tw;
                        'imux_pin: {
                            if let Some(wire_unbuf) = wire_unbuf(wire_from.wire) {
                                let wire_unbuf = TileWireCoord::new_idx(0, wire_unbuf);
                                props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                                props.push(Box::new(WireMutexExclusive::new(wire_unbuf)));
                                break 'imux_pin;
                            } else if wires::IMUX_BRAM_DIA.contains(wire_to.wire)
                                || wires::IMUX_BRAM_DIB.contains(wire_to.wire)
                            {
                                for wire_pin in pips_bwd(backend.edev, tcid, wire_from) {
                                    let wire_pin = wire_pin.tw;
                                    if intdb.wires.key(wire_pin.wire).starts_with("HEX") {
                                        props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                        break 'imux_pin;
                                    }
                                }
                            } else {
                                for wire_pin in pips_fwd(backend.edev, tcid, wire_from) {
                                    let wire_pin = wire_pin.tw;
                                    if wire_pin != wire_to {
                                        if pips_bwd(backend.edev, tcid, wire_from)
                                            .contains(&wire_pin.pos())
                                        {
                                            continue;
                                        }
                                        props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                        break 'imux_pin;
                                    }
                                }
                            }
                            panic!(
                                "NO WAY TO PIN {tcname} {dst} {src}",
                                dst = wire_to.to_string(backend.edev.db, tcls),
                                src = wire_from.to_string(backend.edev.db, tcls),
                            );
                        };
                    }
                }
                for &wire_from in ins {
                    if wire_from.wire == wires::PULLUP {
                        continue;
                    }
                    let wire_from = wire_from.tw;
                    let in_wire_name = intdb.wires.key(wire_from.wire);

                    let mut props = props.clone();
                    'imux_pin: {
                        if in_wire_name.starts_with("GCLK") || wire_unbuf(wire_from.wire).is_some()
                        {
                            // no need to pin
                            break 'imux_pin;
                        } else if wires::IMUX_TBUF_I.contains(wire_to.wire) {
                            // already pinned above
                            break 'imux_pin;
                        } else if wire_to.wire == wires::IMUX_PCI_I3 {
                            let wire_buf = TileWireCoord::new_idx(0, hex_to_buf(wire_from.wire));
                            let related = Delta::new(
                                0,
                                0,
                                if matches!(tcid, tcls::PCI_W_V | tcls::PCI_W_VE) {
                                    tcls::IO_W
                                } else {
                                    tcls::IO_E
                                },
                            );
                            props.push(Box::new(Related::new(
                                related.clone(),
                                BaseIntPip::new(wire_buf, wire_from),
                            )));
                            props.push(Box::new(Related::new(
                                related,
                                WireMutexExclusive::new(wire_buf),
                            )));
                            break 'imux_pin;
                        } else if out_name.starts_with("IMUX_DLL") {
                            if in_wire_name.starts_with("HEX") {
                                props.push(Box::new(VirtexDriveHexH(wire_from)));
                            } else {
                                // don't bother pinning.
                            }
                            break 'imux_pin;
                        } else {
                            for wire_pin in pips_fwd(backend.edev, tcid, wire_from) {
                                let wire_pin = wire_pin.tw;
                                if wire_pin != wire_to {
                                    if pips_bwd(backend.edev, tcid, wire_from)
                                        .contains(&wire_pin.pos())
                                    {
                                        continue;
                                    }
                                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                    break 'imux_pin;
                                }
                            }
                            // try to drive it instead.
                            for wire_pin in pips_bwd(backend.edev, tcid, wire_from) {
                                let wire_pin = wire_pin.tw;
                                if pips_fwd(backend.edev, tcid, wire_from).contains(&wire_pin.pos())
                                {
                                    continue;
                                }
                                props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                break 'imux_pin;
                            }
                        }
                        panic!(
                            "NO WAY TO PIN {tcname} {dst} {src}",
                            dst = wire_to.to_string(backend.edev.db, tcls),
                            src = wire_from.to_string(backend.edev.db, tcls),
                        );
                    };

                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    if let Some(alt_out) = alt_out_wire
                        && (wires::OUT_CLKPAD.contains(wire_from.wire)
                            || wires::OUT_IOFB.contains(wire_from.wire))
                    {
                        let mut builder = ctx.build().test_routing_pair_special(
                            wire_to,
                            wire_from.pos(),
                            specials::INT_NOALT,
                        );
                        for prop in &props {
                            builder = builder.prop_box(prop.clone());
                        }
                        builder.commit();
                        props.push(Box::new(BaseIntPip::new(alt_out, wire_from)));
                    }

                    let mut builder = ctx.build().test_routing(wire_to, wire_from.pos());
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if wires::GCLK.contains(wire_to.wire) {
                // skip
            } else if wires::GCLK_LEAF.contains(wire_to.wire) {
                // causes a crash on xcv405e. lmao.
                if matches!(tcid, tcls::CLKV_BRAM_S | tcls::CLKV_BRAM_N)
                    && backend.device.name.ends_with('e')
                {
                    continue;
                }
                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let mut builder = ctx.build().prop(WireMutexExclusive::new(wire_to));
                    if matches!(tcid, tcls::CLKV_BRAM_S_S2 | tcls::CLKV_BRAM_N_S2) {
                        builder = builder.tile_mutex_exclusive("GCLK_LEAF")
                    } else if matches!(
                        tcid,
                        tcls::CLKV_IO
                            | tcls::CLKV_BRAM_S
                            | tcls::CLKV_BRAM_N
                            | tcls::CLK_S_V
                            | tcls::CLK_N_V
                            | tcls::CLK_S_VE_4DLL
                            | tcls::CLK_N_VE_4DLL
                            | tcls::CLK_S_VE_2DLL
                            | tcls::CLK_N_VE_2DLL
                    ) || (tcid == tcls::BRAM_W && matches!(wire_to.cell.to_idx(), 4..8))
                        || (tcid == tcls::BRAM_E && matches!(wire_to.cell.to_idx(), 8..12))
                    {
                        builder = builder.null_bits();
                    }
                    builder
                        .test_routing(wire_to, wire_from.pos())
                        .prop(FuzzIntPip::new(wire_to, wire_from))
                        .commit();
                }
            } else {
                panic!(
                    "UNHANDLED MUX: {tcname} {dst}",
                    dst = wire_to.to_string(backend.edev.db, tcls)
                );
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    let intdb = edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for bel in tcls.bels.values() {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        let mut diffs = vec![];
                        let mut got_empty = false;
                        let fdst = if let Some(fdst) = wire_from_mux(mux.dst.wire) {
                            TileWireCoord {
                                wire: fdst,
                                cell: mux.dst.cell,
                            }
                        } else {
                            mux.dst
                        };
                        for &src in mux.src.keys() {
                            if src.wire == wires::PULLUP {
                                got_empty = true;
                                diffs.push((Some(src), Diff::default()));
                                continue;
                            }
                            let wire_from = src.tw;
                            let mut diff = ctx.get_diff_routing(tcid, fdst, src);
                            if matches!(mux.dst.wire, wires::IMUX_DLL_CLKIN | wires::IMUX_DLL_CLKFB)
                                && (wires::OUT_CLKPAD.contains(wire_from.wire)
                                    || wires::OUT_IOFB.contains(wire_from.wire))
                            {
                                let noalt_diff = ctx.get_diff_routing_pair_special(
                                    tcid,
                                    mux.dst,
                                    src,
                                    specials::INT_NOALT,
                                );
                                let (alt, noalt, common) = Diff::split(diff, noalt_diff);
                                if mux.dst.wire == wires::IMUX_DLL_CLKIN {
                                    ctx.insert_bel_attr_bool(
                                        tcid,
                                        bslots::DLL,
                                        DLL::CLKIN_PAD,
                                        xlat_bit(noalt),
                                    );
                                    ctx.insert_bel_attr_bool(
                                        tcid,
                                        bslots::DLL,
                                        DLL::CLKFB_PAD,
                                        xlat_bit(!alt),
                                    );
                                } else {
                                    ctx.insert_bel_attr_bool(
                                        tcid,
                                        bslots::DLL,
                                        DLL::CLKFB_PAD,
                                        xlat_bit(noalt),
                                    );
                                    ctx.insert_bel_attr_bool(
                                        tcid,
                                        bslots::DLL,
                                        DLL::CLKIN_PAD,
                                        xlat_bit(!alt),
                                    );
                                }
                                diff = common;
                            }
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            diffs.push((Some(src), diff));
                        }
                        if fdst != mux.dst {
                            let mut drive_bits: HashSet<_> =
                                diffs[0].1.bits.keys().copied().collect();
                            for (_, diff) in &diffs {
                                drive_bits.retain(|bit| diff.bits.contains_key(bit));
                            }
                            if drive_bits.len() > 1 {
                                if matches!(
                                    tcid,
                                    tcls::CNR_SW
                                        | tcls::CNR_SE
                                        | tcls::CNR_NW
                                        | tcls::CNR_NE
                                        | tcls::CNR_SW_S2
                                        | tcls::CNR_NW_S2
                                ) {
                                    // sigh. I give up. those are obtained from looking at left-hand
                                    // corners with easier-to-disambiguate muxes, and correlating with
                                    // bitstream geometry in right-hand corners. also confirmed by some
                                    // manual bitgen tests.
                                    drive_bits
                                        .retain(|bit| matches!(bit.frame.to_idx() % 6, 0 | 5));
                                } else {
                                    let btile = match tcid {
                                        tcls::IO_W => {
                                            edev.btile_main(edev.chip.col_w(), RowId::from_idx(1))
                                        }
                                        tcls::IO_E => {
                                            edev.btile_main(edev.chip.col_e(), RowId::from_idx(1))
                                        }
                                        _ => panic!(
                                            "CAN'T FIGURE OUT DRIVE {tcname} {dst} {drive_bits:?} {diffs:?}",
                                            dst = mux.dst.to_string(edev.db, tcls)
                                        ),
                                    };
                                    drive_bits.retain(|bit| {
                                        !ctx.empty_bs
                                            .get_bit(btile.xlat_pos_fwd((bit.frame, bit.bit)))
                                    });
                                }
                            }
                            if drive_bits.len() != 1 {
                                panic!(
                                    "FUCKY WACKY {tcname} {dst} {diffs:?}",
                                    dst = mux.dst.to_string(edev.db, tcls)
                                );
                            }
                            let drive = Diff {
                                bits: drive_bits
                                    .into_iter()
                                    .map(|bit| (bit, diffs[0].1.bits[&bit]))
                                    .collect(),
                            };
                            for (_, diff) in &mut diffs {
                                *diff = diff.combine(&!&drive);
                            }
                            if diffs.iter().all(|(_, diff)| !diff.bits.is_empty()) {
                                diffs.push((None, Diff::default()));
                            }
                            ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::Mux));
                            ctx.insert_progbuf(tcid, fdst, mux.dst.pos(), xlat_bit(drive));
                        } else {
                            if !got_empty {
                                diffs.push((None, Diff::default()));
                            }
                            let item = xlat_enum_raw(diffs, OcdMode::Mux);
                            if item.bits.is_empty() {
                                if mux.dst.wire == wires::IMUX_IO_T[0] {
                                    // empty on Virtex E?
                                    continue;
                                }
                                println!(
                                    "UMMM MUX {tcname} {dst} is empty",
                                    dst = mux.dst.to_string(edev.db, tcls)
                                );
                            }
                            ctx.insert_mux(tcid, mux.dst, item);
                        }
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        if wire_to_mux(buf.dst.wire) == Some(buf.src.wire) {
                            continue;
                        }
                        if matches!(tcid, tcls::CLKV_BRAM_S_S2 | tcls::CLKV_BRAM_N_S2) {
                            if buf.dst.cell.to_idx() != 0 {
                                continue;
                            }
                            // TODO: absolutely uncertain
                            let odst = buf.dst.wire.cell(1);
                            let mut diff0 = ctx.get_diff_routing(tcid, buf.dst, buf.src);
                            let diff1 = ctx.get_diff_routing(tcid, odst, buf.src);
                            assert_eq!(diff0, diff1);
                            let diff1 = diff0.split_bits_by(|bit| bit.frame.to_idx() >= 9);
                            ctx.insert_progbuf(tcid, buf.dst, buf.src, xlat_bit(diff0));
                            ctx.insert_progbuf(tcid, odst, buf.src, xlat_bit(diff1));
                            continue;
                        }
                        ctx.collect_progbuf(tcid, buf.dst, buf.src);
                    }
                    SwitchBoxItem::Pass(pass) => {
                        ctx.collect_pass(tcid, pass.dst, pass.src);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        ctx.collect_bipass(tcid, pass.a, pass.b);
                    }
                    SwitchBoxItem::PermaBuf(_) => (),
                    SwitchBoxItem::ProgInv(_) => (),
                    _ => unreachable!(),
                }
            }
        }
    }
}
