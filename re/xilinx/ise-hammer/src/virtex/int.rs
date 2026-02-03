use std::collections::HashSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord, WireSlotId},
    grid::{ColId, RowId, TileCoord},
};
use prjcombine_re_collector::{
    diff::{Diff, OcdMode},
    legacy::{xlat_bit_legacy, xlat_enum_legacy_ocd},
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bitrect::BitRect as _;
use prjcombine_virtex::{defs, defs::tcls, defs::wires};

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::FuzzCtx,
        int::{BaseIntPip, FuzzIntPip, resolve_int_pip},
        props::{
            BaseRaw, DynProp,
            bel::{BaseBelMode, BaseBelPin, FuzzBelMode},
            mutex::WireMutexExclusive,
            relation::{Delta, Related},
        },
    },
};

#[derive(Clone, Debug)]
struct VirtexPinBramLv(TileWireCoord);

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
        let tcls_index = &backend.edev.db_index[tile.class];
        for i in 0..12 {
            let wire_pin = TileWireCoord::new_idx(0, wires::LH[i]);
            let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
            if resolved_pin != resolved_wire {
                continue;
            }
            for (&wire_out, mux_data) in &tcls_index.pips_bwd {
                if mux_data.contains(&wire_pin.pos()) {
                    // FOUND
                    let resolved_out = backend.edev.resolve_wire(tcrd.wire(wire_out.wire)).unwrap();
                    let (tile, wt, wf) =
                        resolve_int_pip(backend, tcrd, wire_out, wire_pin).unwrap();
                    fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                    fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_out), None, "EXCLUSIVE");
                    return Some((fuzzer, false));
                }
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
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let resolved_wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.0))?;
        let mut tcrd = backend
            .edev
            .tile_cell(tcrd, self.0.cell)
            .with_col(ColId::from_idx(0))
            .tile(defs::tslots::MAIN);
        loop {
            let tile = &backend.edev[tcrd];
            if matches!(tile.class, tcls::IO_S | tcls::IO_N) {
                for (i, wfake) in [(0, wires::LH_FAKE0), (6, wires::LH_FAKE6)] {
                    let wire_pin = TileWireCoord::new_idx(0, wires::LH[i]);
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    // FOUND
                    let wire_buf = TileWireCoord::new_idx(0, wfake);
                    let resolved_buf = backend.edev.resolve_wire(tcrd.wire(wire_buf.wire)).unwrap();
                    let (tile, wt, wf) =
                        resolve_int_pip(backend, tcrd, wire_buf, wire_pin).unwrap();
                    fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                    fuzzer = fuzzer.fuzz(Key::WireMutex(resolved_buf), None, "EXCLUSIVE");
                    return Some((fuzzer, false));
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
                let tcls_index = &backend.edev.db_index[tile.class];
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.edev.db.get_wire(&format!("HEX_{h}{j}[{i}]")),
                    );
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for (&wire_out, mux_data) in &tcls_index.pips_bwd {
                        if mux_data.contains(&wire_pin.pos()) {
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
                                    resolve_int_pip(backend, tcrd, wire_out, wire_pin).unwrap();
                                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                                fuzzer =
                                    fuzzer.fuzz(Key::WireMutex(resolved_out), None, "EXCLUSIVE");
                                return Some((fuzzer, false));
                            }
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
                let tcls_index = &backend.edev.db_index[tile.class];
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.edev.db.get_wire(&format!("HEX_{v}{j}[{i}]")),
                    );
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for (&wire_out, mux_data) in &tcls_index.pips_bwd {
                        if mux_data.contains(&wire_pin.pos()) {
                            let out_name = backend.edev.db.wires.key(wire_out.wire);
                            if out_name.starts_with("SINGLE")
                                || (out_name.starts_with("HEX_N") && tile.class == tcls::IO_S)
                                || (out_name.starts_with("HEX_S") && tile.class == tcls::IO_N)
                            {
                                // FOUND
                                let resolved_out =
                                    backend.edev.resolve_wire(tcrd.wire(wire_out.wire)).unwrap();
                                let (tile, wt, wf) =
                                    resolve_int_pip(backend, tcrd, wire_out, wire_pin).unwrap();
                                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                                fuzzer =
                                    fuzzer.fuzz(Key::WireMutex(resolved_out), None, "EXCLUSIVE");
                                return Some((fuzzer, false));
                            }
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
                let tcls_index = &backend.edev.db_index[tile.class];
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.edev.db.get_wire(&format!("HEX_{h}{j}[{i}]")),
                    );
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    if let Some(mux_data) = tcls_index.pips_bwd.get(&wire_pin) {
                        for &inp in mux_data {
                            let inp_name = backend.edev.db.wires.key(inp.wire);
                            if inp_name.starts_with("OMUX")
                                || inp_name.starts_with("OUT")
                                || (h == 'E'
                                    && tile.class == tcls::IO_W
                                    && inp_name.starts_with("HEX"))
                                || (h == 'W'
                                    && tile.class == tcls::IO_E
                                    && inp_name.starts_with("HEX"))
                            {
                                // FOUND
                                let resolved_inp =
                                    backend.edev.resolve_wire(tcrd.wire(inp.wire)).unwrap();
                                let (tile, wt, wf) =
                                    resolve_int_pip(backend, tcrd, wire_pin, inp.tw).unwrap();
                                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                                fuzzer =
                                    fuzzer.fuzz(Key::WireMutex(resolved_inp), None, "EXCLUSIVE");
                                fuzzer =
                                    fuzzer.fuzz(Key::WireMutex(resolved_pin), None, "EXCLUSIVE");
                                return Some((fuzzer, false));
                            }
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
                let tcls_index = &backend.edev.db_index[tile.class];
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.edev.db.get_wire(&format!("HEX_{v}{j}[{i}]")),
                    );
                    let resolved_pin = backend.edev.resolve_wire(tcrd.wire(wire_pin.wire)).unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    if let Some(mux_data) = tcls_index.pips_bwd.get(&wire_pin) {
                        for &inp in mux_data {
                            let inp_name = backend.edev.db.wires.key(inp.wire);
                            if inp_name.starts_with("OMUX")
                                || inp_name.starts_with("OUT")
                                || (v == 'N'
                                    && tile.class == tcls::IO_S
                                    && inp_name.starts_with("HEX"))
                                || (v == 'S'
                                    && tile.class == tcls::IO_N
                                    && inp_name.starts_with("HEX"))
                            {
                                // FOUND
                                let resolved_inp =
                                    backend.edev.resolve_wire(tcrd.wire(inp.wire)).unwrap();
                                let (tile, wt, wf) =
                                    resolve_int_pip(backend, tcrd, wire_pin, inp.tw).unwrap();
                                fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true);
                                fuzzer =
                                    fuzzer.fuzz(Key::WireMutex(resolved_inp), None, "EXCLUSIVE");
                                fuzzer =
                                    fuzzer.fuzz(Key::WireMutex(resolved_pin), None, "EXCLUSIVE");
                                return Some((fuzzer, false));
                            }
                        }
                    }
                }
            }
            tcrd.row += 1;
        }
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
        Some(wires::GCLK[idx])
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
    } else if wire == wires::LH_FAKE0 {
        Some(wires::LH[0])
    } else if wire == wires::LH_FAKE6 {
        Some(wires::LH[6])
    } else {
        None
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        let tcls_index = &backend.edev.db_index[tcid];
        let Some(mut ctx) = FuzzCtx::try_new_legacy(session, backend, tcname) else {
            continue;
        };
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let mux_name = if tcls.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.wire))
            } else {
                format!("MUX.{:#}.{}", wire_to.cell, intdb.wires.key(wire_to.wire))
            };
            let out_name = intdb.wires.key(wire_to.wire);
            if wire_unbuf(wire_to.wire).is_some() {
                continue;
            } else if out_name.contains("OMUX") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];
                if matches!(tcid, tcls::IO_W | tcls::IO_E) {
                    for i in 0..4 {
                        props.push(Box::new(BaseBelMode::new(
                            defs::bslots::IO[i],
                            0,
                            ["EMPTYIOB", "IOB", "IOB", "IOB"][i].into(),
                        )));
                        props.push(Box::new(BaseBelPin::new(
                            defs::bslots::IO[i],
                            0,
                            "I".into(),
                        )));
                    }
                    let clb_id = intdb.get_tile_class("CLB");
                    let clb_index = &backend.edev.db_index[clb_id];
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
                    let wire_pin = clb_index.pips_fwd[&clb_wire].iter().next().unwrap().tw;
                    let relation = if tcid == tcls::IO_W {
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
                        WireMutexExclusive::new(wire_pin),
                    )));
                } else {
                    let wire_pin = tcls_index.pips_fwd[&wire_to].iter().next().unwrap().tw;
                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_to)));
                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                }
                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let in_name = if tcls.cells.len() == 1 {
                        intdb.wires.key(wire_from.wire).to_string()
                    } else {
                        format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire))
                    };
                    let mut builder = ctx
                        .build()
                        .test_manual_legacy("INT", &mux_name, in_name)
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
                    for &wire_pin in &tcls_index.pips_fwd[&wire_to_root] {
                        let wire_pin = wire_pin.tw;
                        let wire_pin_name = intdb.wires.key(wire_pin.wire);
                        if wire_pin_name.starts_with("IMUX") || wire_pin_name.starts_with("HEX") {
                            break 'quad_dst_pin wire_pin;
                        }
                    }
                    panic!("NO WAY TO PIN {tcname} {mux_name}");
                };
                if !is_s {
                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_to)));
                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                } else {
                    let related = Delta::new(0, 4, tcname);
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
                                    if tcid == tcls::BRAM_W { "IO_W" } else { "CLB" },
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
                                for &wire_pin in &tcls_index.pips_fwd[&wire_from] {
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
                            panic!("NO WAY TO PIN {tcname} {mux_name} {in_wire_name}");
                        }
                    }
                }
                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let in_wire_name = intdb.wires.key(wire_from.wire);
                    let in_name = if tcls.cells.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{:#}.{}", wire_from.cell, in_wire_name)
                    };
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

                            for &wire_pin in &tcls_index.pips_bwd[&wire_from_root] {
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
                                        let related = Delta::new(0, 4, tcname);
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
                            panic!("NO WAY TO PIN {tcname} {mux_name} {in_name}");
                        }
                    }
                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    let mut builder = ctx.build().test_manual_legacy("INT", &mux_name, &in_name);
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
                        if tcid == tcls::BRAM_W { "IO_W" } else { "CLB" },
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
                    let in_name = if tcls.cells.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{:#}.{}", wire_from.cell, in_wire_name)
                    };

                    let mut props = props.clone();
                    'single_pin: {
                        if in_wire_name.starts_with("SINGLE") {
                            for &wire_pin in &tcls_index.pips_bwd[&wire_from] {
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
                            for &wire_pin in &tcls_index.pips_fwd[&wire_from] {
                                let wire_pin = wire_pin.tw;
                                let wire_pin_name = intdb.wires.key(wire_pin.wire);
                                if wire_pin != wire_to && wire_pin_name.starts_with("SINGLE") {
                                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                    break 'single_pin;
                                }
                            }
                        }
                        panic!("NO WAY TO PIN {tcname} {mux_name} {in_name}");
                    };

                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    let mut builder = ctx.build().test_manual_legacy("INT", &mux_name, &in_name);
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

                if out_name.starts_with("LH") && matches!(tcid, tcls::IO_S | tcls::IO_N) {
                    let wire_buf = TileWireCoord::new_idx(
                        0,
                        if wire_to.wire == wires::LH[0] {
                            wires::LH_FAKE0
                        } else if wire_to.wire == wires::LH[6] {
                            wires::LH_FAKE6
                        } else {
                            unreachable!()
                        },
                    );
                    props.push(Box::new(BaseIntPip::new(wire_buf, wire_to)));
                    props.push(Box::new(WireMutexExclusive::new(wire_buf)));
                } else if out_name.starts_with("LV") && matches!(tcid, tcls::BRAM_S | tcls::BRAM_N)
                {
                    props.push(Box::new(VirtexPinBramLv(wire_to)));
                } else if out_name.starts_with("LH")
                    && matches!(tcid, tcls::BRAM_W | tcls::BRAM_E | tcls::BRAM_M)
                {
                    props.push(Box::new(VirtexPinLh(wire_to)));
                } else if out_name.starts_with("LH") && tcname.starts_with("CLK") {
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
                        for &wire_pin in &tcls_index.pips_fwd[&wire_to] {
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
                        println!("NO WAY TO PIN {tcname} {mux_name}");
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
                            for &wire_pin in &tcls_index.pips_bwd[&wire_from] {
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
                                tcls::CNR_SW | tcls::CNR_SE | tcls::CNR_NW | tcls::CNR_NE
                            )
                        {
                            // it's fine.
                            props.push(Box::new(VirtexPinIoLh(wire_from)));
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("LH") || in_wire_name.starts_with("LV") {
                            for &wire_pin in &tcls_index.pips_bwd[&wire_from] {
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
                            if matches!(tcid, tcls::BRAM_W | tcls::BRAM_E | tcls::BRAM_M) {
                                let related = Delta::new(
                                    -1,
                                    wire_from.cell.to_idx() as i32 - 4,
                                    if tcid == tcls::BRAM_W { "IO_W" } else { "CLB" },
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
                                    defs::bslots::IO[i],
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
                                    defs::bslots::IO[i],
                                    0,
                                    "I".into(),
                                )));
                                props.push(Box::new(BaseBelPin::new(
                                    defs::bslots::IO[i],
                                    0,
                                    "IQ".into(),
                                )));
                            }
                            break 'll_src_pin;
                        } else if let Some(pin) = in_wire_name.strip_prefix("OUT_BSCAN_") {
                            props.push(Box::new(BaseBelMode::new(
                                defs::bslots::BSCAN,
                                0,
                                "BSCAN".into(),
                            )));
                            props.push(Box::new(BaseBelPin::new(
                                defs::bslots::BSCAN,
                                0,
                                pin.into(),
                            )));
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
                        panic!("NO WAY TO PIN {tcname} {mux_name} {in_wire_name}");
                    };
                }

                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let in_wire_name = intdb.wires.key(wire_from.wire);
                    let in_name = if tcls.cells.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{:#}.{}", wire_from.cell, in_wire_name)
                    };

                    let mut props = props.clone();
                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));

                    let mut builder = ctx.build().test_manual_legacy("INT", &mux_name, &in_name);
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.contains("IMUX") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];
                if let Some(pin) = out_name.strip_prefix("IMUX_STARTUP_") {
                    props.push(Box::new(BaseBelMode::new(
                        defs::bslots::STARTUP,
                        0,
                        "STARTUP".into(),
                    )));
                    props.push(Box::new(BaseBelPin::new(
                        defs::bslots::STARTUP,
                        0,
                        pin.into(),
                    )));
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
                        defs::bslots::BUFG[idx],
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
                        let wire_from = wire_from.tw;
                        let in_wire_name = intdb.wires.key(wire_from.wire);
                        'imux_pin: {
                            if let Some(wire_unbuf) = wire_unbuf(wire_from.wire) {
                                let wire_unbuf = TileWireCoord::new_idx(0, wire_unbuf);
                                props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                                props.push(Box::new(WireMutexExclusive::new(wire_unbuf)));
                                break 'imux_pin;
                            } else if wires::IMUX_BRAM_DIA.contains(wire_to.wire)
                                || wires::IMUX_BRAM_DIB.contains(wire_to.wire)
                            {
                                for &wire_pin in &tcls_index.pips_bwd[&wire_from] {
                                    let wire_pin = wire_pin.tw;
                                    if intdb.wires.key(wire_pin.wire).starts_with("HEX") {
                                        props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                        break 'imux_pin;
                                    }
                                }
                            } else {
                                for &wire_pin in &tcls_index.pips_fwd[&wire_from] {
                                    let wire_pin = wire_pin.tw;
                                    if wire_pin != wire_to {
                                        if let Some(from_mux) = tcls_index.pips_bwd.get(&wire_from)
                                            && from_mux.contains(&wire_pin.pos())
                                        {
                                            continue;
                                        }
                                        props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                        props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                        break 'imux_pin;
                                    }
                                }
                            }
                            panic!("NO WAY TO PIN {tcname} {mux_name} {in_wire_name}");
                        };
                    }
                }
                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let in_wire_name = intdb.wires.key(wire_from.wire);
                    let in_name = if tcls.cells.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{:#}.{}", wire_from.cell, in_wire_name)
                    };

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
                            let related =
                                Delta::new(0, 0, if tcid == tcls::PCI_W { "IO_W" } else { "IO_E" });
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
                            for &wire_pin in &tcls_index.pips_fwd[&wire_from] {
                                let wire_pin = wire_pin.tw;
                                if wire_pin != wire_to {
                                    if let Some(from_mux) = tcls_index.pips_bwd.get(&wire_from)
                                        && from_mux.contains(&wire_pin.pos())
                                    {
                                        continue;
                                    }
                                    props.push(Box::new(BaseIntPip::new(wire_pin, wire_from)));
                                    props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                    break 'imux_pin;
                                }
                            }
                            // try to drive it instead.
                            for &wire_pin in &tcls_index.pips_bwd[&wire_from] {
                                let wire_pin = wire_pin.tw;
                                if tcls_index.pips_fwd[&wire_from].contains(&wire_pin.pos()) {
                                    continue;
                                }
                                props.push(Box::new(BaseIntPip::new(wire_from, wire_pin)));
                                props.push(Box::new(WireMutexExclusive::new(wire_from)));
                                props.push(Box::new(WireMutexExclusive::new(wire_pin)));
                                break 'imux_pin;
                            }
                        }
                        panic!("NO WAY TO PIN {tcname} {mux_name} {in_name}");
                    };

                    props.push(Box::new(FuzzIntPip::new(wire_to, wire_from)));
                    if let Some(alt_out) = alt_out_wire
                        && (wires::OUT_CLKPAD.contains(wire_from.wire)
                            || wires::OUT_IOFB.contains(wire_from.wire))
                    {
                        let mut builder = ctx.build().test_manual_legacy(
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

                    let mut builder = ctx.build().test_manual_legacy("INT", &mux_name, &in_name);
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else {
                panic!("UNHANDLED MUX: {tcname} {mux_name}");
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
        if !ctx.has_tile_legacy(tcname) {
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
                        let out_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(mux.dst.wire).to_string()
                        } else {
                            format!("{:#}.{}", mux.dst.cell, intdb.wires.key(mux.dst.wire))
                        };
                        let mux_name = format!("MUX.{out_name}");

                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &wire_from in mux.src.keys() {
                            let wire_from = wire_from.tw;
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(wire_from.wire).to_string()
                            } else {
                                format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire))
                            };
                            let mut diff = ctx.get_diff_legacy(tcname, "INT", &mux_name, &in_name);
                            if matches!(mux.dst.wire, wires::IMUX_DLL_CLKIN | wires::IMUX_DLL_CLKFB)
                                && (wires::OUT_CLKPAD.contains(wire_from.wire)
                                    || wires::OUT_IOFB.contains(wire_from.wire))
                            {
                                let noalt_diff = ctx.get_diff_legacy(
                                    tcname,
                                    "INT",
                                    &mux_name,
                                    format!("{in_name}.NOALT"),
                                );
                                let (alt, noalt, common) = Diff::split(diff, noalt_diff);
                                if mux_name.contains("CLKIN") {
                                    ctx.insert_legacy(
                                        tcname,
                                        "DLL",
                                        "CLKIN_PAD",
                                        xlat_bit_legacy(noalt),
                                    );
                                    ctx.insert_legacy(
                                        tcname,
                                        "DLL",
                                        "CLKFB_PAD",
                                        xlat_bit_legacy(!alt),
                                    );
                                } else {
                                    ctx.insert_legacy(
                                        tcname,
                                        "DLL",
                                        "CLKFB_PAD",
                                        xlat_bit_legacy(noalt),
                                    );
                                    ctx.insert_legacy(
                                        tcname,
                                        "DLL",
                                        "CLKIN_PAD",
                                        xlat_bit_legacy(!alt),
                                    );
                                }
                                diff = common;
                            }
                            if (in_name.starts_with("OUT_IO") && in_name.ends_with("[0]"))
                                || (in_name.starts_with("OUT_IO")
                                    && in_name.ends_with("[3]")
                                    && matches!(tcid, tcls::IO_S | tcls::IO_N))
                            {
                                diff.assert_empty();
                            } else if (out_name.contains("BRAM_QUAD")
                                && in_name.contains("BRAM_QUAD"))
                                || out_name.contains("BRAM_QUAD_DOUT")
                                || (out_name.contains("HEX_H") && wire_from.wire == wires::PCI_CE)
                                || (matches!(
                                    tcid,
                                    tcls::CNR_SW | tcls::CNR_SE | tcls::CNR_NW | tcls::CNR_NE
                                ) && wires::LV.contains(mux.dst.wire))
                                || (matches!(tcid, tcls::BRAM_S | tcls::BRAM_N)
                                    && wires::LV.contains(mux.dst.wire))
                            {
                                if diff.bits.is_empty() {
                                    println!("UMM {out_name} {in_name} BUF IS EMPTY");
                                    continue;
                                }
                                ctx.insert_legacy(
                                    tcname,
                                    bel,
                                    format!("BUF.{out_name}.{in_name}"),
                                    xlat_bit_legacy(diff),
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
                        if out_name.contains("BRAM_QUAD")
                            || wires::LV.contains(mux.dst.wire)
                            || wires::LH.contains(mux.dst.wire)
                            || out_name.contains("HEX_H")
                            || out_name.contains("HEX_V")
                        {
                            let mut drive_bits: HashSet<_> =
                                inps[0].1.bits.keys().copied().collect();
                            for (_, diff) in &inps {
                                drive_bits.retain(|bit| diff.bits.contains_key(bit));
                            }
                            if drive_bits.len() > 1 {
                                if matches!(
                                    tcid,
                                    tcls::CNR_SW | tcls::CNR_SE | tcls::CNR_NW | tcls::CNR_NE
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
                                            "CAN'T FIGURE OUT DRIVE {tcname} {mux_name} {drive_bits:?} {inps:?}"
                                        ),
                                    };
                                    drive_bits.retain(|bit| {
                                        !ctx.empty_bs
                                            .get_bit(btile.xlat_pos_fwd((bit.frame, bit.bit)))
                                    });
                                }
                            }
                            if drive_bits.len() != 1 {
                                panic!("FUCKY WACKY {tcname} {out_name} {inps:?}");
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
                            let item = xlat_enum_legacy_ocd(inps, OcdMode::Mux);
                            ctx.insert_legacy(tcname, bel, mux_name, item);
                            ctx.insert_legacy(
                                tcname,
                                bel,
                                format!("DRIVE.{out_name}"),
                                xlat_bit_legacy(drive),
                            );
                        } else {
                            if !got_empty {
                                inps.push(("NONE".to_string(), Diff::default()));
                            }
                            let item = xlat_enum_legacy_ocd(inps, OcdMode::Mux);
                            if item.bits.is_empty() {
                                if mux.dst.wire == wires::IMUX_IO_T[0] {
                                    // empty on Virtex E?
                                    continue;
                                }
                                if mux_name.starts_with("MUX.HEX_S") && tcid == tcls::IO_N
                                    || mux_name.starts_with("MUX.HEX_N") && tcid == tcls::IO_S
                                    || mux_name.starts_with("MUX.HEX_E") && tcid == tcls::IO_W
                                    || mux_name.starts_with("MUX.HEX_W") && tcid == tcls::IO_E
                                {
                                    continue;
                                }
                                println!("UMMM MUX {tcname} {mux_name} is empty");
                            }
                            ctx.insert_legacy(tcname, bel, mux_name, item);
                        }
                    }
                    SwitchBoxItem::Pass(pass) => {
                        let out_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(pass.dst.wire).to_string()
                        } else {
                            format!("{:#}.{}", pass.dst.cell, intdb.wires.key(pass.dst.wire))
                        };
                        let in_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(pass.src.wire).to_string()
                        } else {
                            format!("{:#}.{}", pass.src.cell, intdb.wires.key(pass.src.wire))
                        };
                        let diff =
                            ctx.get_diff_legacy(tcname, "INT", format!("MUX.{out_name}"), &in_name);
                        if (in_name.starts_with("OUT_IO") && in_name.ends_with("[0]"))
                            || (matches!(tcid, tcls::IO_S | tcls::IO_N)
                                && in_name.starts_with("OUT_IO")
                                && in_name.ends_with("[3]"))
                        {
                            diff.assert_empty();
                            continue;
                        }
                        if diff.bits.is_empty() {
                            println!("UMM {out_name} {in_name} PASS IS EMPTY");
                            continue;
                        }
                        let item = xlat_bit_legacy(diff);
                        let name = format!("PASS.{out_name}.{in_name}");
                        ctx.insert_legacy(tcname, bel, name, item);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        let a_name = intdb.wires.key(pass.a.wire);
                        let b_name = intdb.wires.key(pass.b.wire);
                        let name = if tcls.cells.len() == 1 {
                            format!("BIPASS.{a_name}.{b_name}")
                        } else {
                            format!(
                                "BIPASS.{a_cell:#}.{a_name}.{b_cell:#}.{b_name}",
                                a_cell = pass.a.cell,
                                b_cell = pass.b.cell,
                            )
                        };
                        for (wdst, wsrc) in [(pass.a, pass.b), (pass.b, pass.a)] {
                            let out_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(wdst.wire).to_string()
                            } else {
                                format!("{:#}.{}", wdst.cell, intdb.wires.key(wdst.wire))
                            };
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(wsrc.wire).to_string()
                            } else {
                                format!("{:#}.{}", wsrc.cell, intdb.wires.key(wsrc.wire))
                            };
                            let diff = ctx.get_diff_legacy(
                                tcname,
                                "INT",
                                format!("MUX.{out_name}"),
                                &in_name,
                            );
                            let item = xlat_bit_legacy(diff);
                            ctx.insert_legacy(tcname, bel, &name, item);
                        }
                    }
                    SwitchBoxItem::PermaBuf(_) => (),
                    SwitchBoxItem::ProgInv(_) => (),
                    _ => unreachable!(),
                }
            }
        }
    }
}
