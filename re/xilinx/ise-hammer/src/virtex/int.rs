use std::collections::HashSet;

use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord},
    grid::{ColId, RowId, TileCoord},
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, OcdMode, xlat_bit, xlat_enum_ocd};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bittile::BitTile as _;
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
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.0))?;
        let mut tcrd = tcrd;
        tcrd.row = RowId::from_idx(1);
        tcrd.slot = tslots::MAIN;
        for i in 0..12 {
            let wire_pin = TileWireCoord::new_idx(0, backend.egrid.db.get_wire(&format!("LV.{i}")));

            let resolved_pin = backend
                .egrid
                .resolve_wire(tcrd.wire(wire_pin.wire))
                .unwrap();
            let wire_clk = TileWireCoord::new_idx(0, backend.egrid.db.get_wire("IMUX.BRAM.CLKA"));
            let resolved_clk = backend
                .egrid
                .resolve_wire(tcrd.wire(wire_clk.wire))
                .unwrap();
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
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.0))?;
        let tcrd = backend
            .egrid
            .tile_cell(tcrd, self.0.cell)
            .with_col(ColId::from_idx(0))
            .tile(tslots::MAIN);
        let tile = &backend.egrid[tcrd];
        let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
        for i in 0..12 {
            let wire_pin = TileWireCoord::new_idx(0, backend.egrid.db.get_wire(&format!("LH.{i}")));
            let resolved_pin = backend
                .egrid
                .resolve_wire(tcrd.wire(wire_pin.wire))
                .unwrap();
            if resolved_pin != resolved_wire {
                continue;
            }
            for (&wire_out, mux_data) in &tcls_index.pips_bwd {
                if mux_data.contains(&wire_pin.pos()) {
                    // FOUND
                    let resolved_out = backend
                        .egrid
                        .resolve_wire(tcrd.wire(wire_out.wire))
                        .unwrap();
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
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.0))?;
        let mut tcrd = backend
            .egrid
            .tile_cell(tcrd, self.0.cell)
            .with_col(ColId::from_idx(0))
            .tile(tslots::MAIN);
        loop {
            let tile = &backend.egrid[tcrd];
            if matches!(
                &backend.egrid.db.tile_classes.key(tile.class)[..],
                "IO.B" | "IO.T"
            ) {
                for i in [0, 6] {
                    let wire_pin =
                        TileWireCoord::new_idx(0, backend.egrid.db.get_wire(&format!("LH.{i}")));
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire(tcrd.wire(wire_pin.wire))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    // FOUND
                    let wire_buf = TileWireCoord::new_idx(
                        0,
                        backend.egrid.db.get_wire(&format!("LH.{i}.FAKE")),
                    );
                    let resolved_buf = backend
                        .egrid
                        .resolve_wire(tcrd.wire(wire_buf.wire))
                        .unwrap();
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
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.0))?;
        let wire_name = backend.egrid.db.wires.key(self.0.wire);
        let h = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[5..6].parse().unwrap();
        let mut tcrd = backend
            .egrid
            .tile_cell(tcrd, self.0.cell)
            .tile(tslots::MAIN);
        if tcrd.col.to_idx() >= 8 {
            tcrd.col -= 8;
        } else {
            tcrd.col = ColId::from_idx(0)
        };
        loop {
            if let Some(tile) = backend.egrid.get_tile(tcrd)
                && matches!(
                    &backend.egrid.db.tile_classes.key(tile.class)[..],
                    "IO.L" | "IO.R" | "IO.B" | "IO.T" | "CLB" | "CNR.BR" | "CNR.TR"
                )
            {
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.egrid.db.get_wire(&format!("HEX.{h}{i}.{j}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire(tcrd.wire(wire_pin.wire))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for (&wire_out, mux_data) in &tcls_index.pips_bwd {
                        if mux_data.contains(&wire_pin.pos()) {
                            let out_name = backend.egrid.db.wires.key(wire_out.wire);
                            if out_name.starts_with("SINGLE")
                                || (out_name.starts_with("LV") && i >= 4)
                                || (out_name.starts_with("HEX.E")
                                    && backend.egrid.db.tile_classes.key(tile.class) == "IO.L")
                                || (out_name.starts_with("HEX.W")
                                    && backend.egrid.db.tile_classes.key(tile.class) == "IO.R")
                            {
                                // FOUND
                                let resolved_out = backend
                                    .egrid
                                    .resolve_wire(tcrd.wire(wire_out.wire))
                                    .unwrap();
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
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.0))?;
        let wire_name = backend.egrid.db.wires.key(self.0.wire);
        let v = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[5..6].parse().unwrap();
        let mut tcrd = backend
            .egrid
            .tile_cell(tcrd, self.0.cell)
            .tile(tslots::MAIN);
        if tcrd.row.to_idx() >= 6 {
            tcrd.row -= 6;
        } else {
            tcrd.row = RowId::from_idx(0)
        };
        loop {
            if let Some(tile) = backend.egrid.get_tile(tcrd)
                && matches!(
                    &backend.egrid.db.tile_classes.key(tile.class)[..],
                    "IO.L" | "IO.R" | "CLB" | "IO.B" | "IO.T"
                )
            {
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.egrid.db.get_wire(&format!("HEX.{v}{i}.{j}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire(tcrd.wire(wire_pin.wire))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    for (&wire_out, mux_data) in &tcls_index.pips_bwd {
                        if mux_data.contains(&wire_pin.pos()) {
                            let out_name = backend.egrid.db.wires.key(wire_out.wire);
                            if out_name.starts_with("SINGLE")
                                || (out_name.starts_with("HEX.N")
                                    && backend.egrid.db.tile_classes.key(tile.class) == "IO.B")
                                || (out_name.starts_with("HEX.S")
                                    && backend.egrid.db.tile_classes.key(tile.class) == "IO.T")
                            {
                                // FOUND
                                let resolved_out = backend
                                    .egrid
                                    .resolve_wire(tcrd.wire(wire_out.wire))
                                    .unwrap();
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
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.0))?;
        let wire_name = backend.egrid.db.wires.key(self.0.wire);
        let h = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[5..6].parse().unwrap();
        let mut tcrd = backend
            .egrid
            .tile_cell(tcrd, self.0.cell)
            .tile(tslots::MAIN);
        if tcrd.col.to_idx() >= 8 {
            tcrd.col -= 8;
        } else {
            tcrd.col = ColId::from_idx(0)
        };
        loop {
            if let Some(tile) = backend.egrid.get_tile(tcrd)
                && matches!(
                    &backend.egrid.db.tile_classes.key(tile.class)[..],
                    "IO.L" | "IO.R" | "IO.B" | "IO.T" | "CLB"
                )
            {
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.egrid.db.get_wire(&format!("HEX.{h}{i}.{j}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire(tcrd.wire(wire_pin.wire))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    if let Some(mux_data) = tcls_index.pips_bwd.get(&wire_pin) {
                        for &inp in mux_data {
                            let inp_name = backend.egrid.db.wires.key(inp.wire);
                            if inp_name.starts_with("OMUX")
                                || inp_name.starts_with("OUT")
                                || (h == 'E'
                                    && backend.egrid.db.tile_classes.key(tile.class) == "IO.L"
                                    && inp_name.starts_with("HEX"))
                                || (h == 'W'
                                    && backend.egrid.db.tile_classes.key(tile.class) == "IO.R"
                                    && inp_name.starts_with("HEX"))
                            {
                                // FOUND
                                let resolved_inp =
                                    backend.egrid.resolve_wire(tcrd.wire(inp.wire)).unwrap();
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
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.0))?;
        let wire_name = backend.egrid.db.wires.key(self.0.wire);
        let v = wire_name[4..5].chars().next().unwrap();
        let i: usize = wire_name[5..6].parse().unwrap();
        let mut tcrd = backend
            .egrid
            .tile_cell(tcrd, self.0.cell)
            .tile(tslots::MAIN);

        if tcrd.row.to_idx() >= 6 {
            tcrd.row -= 6;
        } else {
            tcrd.row = RowId::from_idx(0)
        };
        loop {
            if let Some(tile) = backend.egrid.get_tile(tcrd)
                && matches!(
                    &backend.egrid.db.tile_classes.key(tile.class)[..],
                    "IO.L" | "IO.R" | "CLB" | "IO.B" | "IO.T"
                )
            {
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                for j in 0..=6 {
                    let wire_pin = TileWireCoord::new_idx(
                        0,
                        backend.egrid.db.get_wire(&format!("HEX.{v}{i}.{j}")),
                    );
                    let resolved_pin = backend
                        .egrid
                        .resolve_wire(tcrd.wire(wire_pin.wire))
                        .unwrap();
                    if resolved_pin != resolved_wire {
                        continue;
                    }
                    if let Some(mux_data) = tcls_index.pips_bwd.get(&wire_pin) {
                        for &inp in mux_data {
                            let inp_name = backend.egrid.db.wires.key(inp.wire);
                            if inp_name.starts_with("OMUX")
                                || inp_name.starts_with("OUT")
                                || (v == 'N'
                                    && backend.egrid.db.tile_classes.key(tile.class) == "IO.B"
                                    && inp_name.starts_with("HEX"))
                                || (v == 'S'
                                    && backend.egrid.db.tile_classes.key(tile.class) == "IO.T"
                                    && inp_name.starts_with("HEX"))
                            {
                                // FOUND
                                let resolved_inp =
                                    backend.egrid.resolve_wire(tcrd.wire(inp.wire)).unwrap();
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

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        let tcls_index = &backend.egrid.db_index.tile_classes[tcid];
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcname) else {
            continue;
        };
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let mux_name = if tcls.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.wire))
            } else {
                format!("MUX.{:#}.{}", wire_to.cell, intdb.wires.key(wire_to.wire))
            };
            let out_name = intdb.wires.key(wire_to.wire);
            if out_name.ends_with(".BUF") || out_name.ends_with(".FAKE") {
                continue;
            } else if out_name.contains("OMUX") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];
                if tcname.starts_with("IO") {
                    for i in 0..4 {
                        props.push(Box::new(BaseBelMode::new(
                            bels::IO[i],
                            ["EMPTYIOB", "IOB", "IOB", "IOB"][i].into(),
                        )));
                        props.push(Box::new(BaseBelPin::new(bels::IO[i], "I".into())));
                    }
                    let clb_id = intdb.get_tile_class("CLB");
                    let clb_index = &backend.egrid.db_index.tile_classes[clb_id];
                    let wire_name = intdb.wires.key(wire_to.wire);
                    let clb_wire = if tcname == "IO.L" {
                        format!("{wire_name}.W")
                    } else {
                        format!("{wire_name}.E")
                    };
                    let clb_wire = TileWireCoord::new_idx(0, intdb.get_wire(&clb_wire));
                    let wire_pin = clb_index.pips_fwd[&clb_wire].iter().next().unwrap().tw;
                    let relation = if tcname == "IO.L" {
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
                        .test_manual("INT", &mux_name, in_name)
                        .prop(FuzzIntPip::new(wire_to, wire_from));
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.starts_with("BRAM.QUAD") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];

                let (is_s, wire_to_root) = if let Some(root_name) = out_name.strip_suffix(".S") {
                    (
                        true,
                        TileWireCoord {
                            cell: wire_to.cell,
                            wire: intdb.get_wire(root_name),
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
                if !out_name.starts_with("BRAM.QUAD.DOUT") {
                    // pin every input
                    let mut pins = HashSet::new();
                    for &wire_from in ins {
                        let wire_from = wire_from.tw;
                        let in_wire_name = intdb.wires.key(wire_from.wire);
                        'quad_src_all_pin: {
                            if in_wire_name.starts_with("SINGLE") {
                                let wire_buf = format!("{in_wire_name}.BUF");
                                let wire_buf = TileWireCoord::new_idx(0, intdb.get_wire(&wire_buf));
                                let related = Delta::new(
                                    -1,
                                    wire_from.cell.to_idx() as i32 - 4,
                                    if tcname == "LBRAM" { "IO.L" } else { "CLB" },
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
                    if in_wire_name.starts_with("BRAM.QUAD") {
                        'quad_src_pin: {
                            let (is_s, wire_from_root) =
                                if let Some(root_name) = in_wire_name.strip_suffix(".S") {
                                    (
                                        true,
                                        TileWireCoord {
                                            cell: wire_from.cell,
                                            wire: intdb.get_wire(root_name),
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
                    let mut builder = ctx.build().test_manual("INT", &mux_name, &in_name);
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.starts_with("SINGLE") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];

                let wire_buf =
                    TileWireCoord::new_idx(0, intdb.get_wire(&format!("{out_name}.BUF")));
                if !tcname.contains("BRAM") {
                    props.push(Box::new(BaseIntPip::new(wire_buf, wire_to)));
                    props.push(Box::new(WireMutexExclusive::new(wire_buf)));
                } else {
                    let related = Delta::new(
                        -1,
                        wire_to.cell.to_idx() as i32 - 4,
                        if tcname == "LBRAM" { "IO.L" } else { "CLB" },
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
                                    || wire_pin_name.starts_with("BRAM.QUAD.DOUT")
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
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];

                if out_name.starts_with("LH") && matches!(&tcname[..], "IO.B" | "IO.T") {
                    let wire_buf =
                        TileWireCoord::new_idx(0, intdb.get_wire(&format!("{out_name}.FAKE")));
                    props.push(Box::new(BaseIntPip::new(wire_buf, wire_to)));
                    props.push(Box::new(WireMutexExclusive::new(wire_buf)));
                } else if out_name.starts_with("LV")
                    && matches!(&tcname[..], "BRAM_BOT" | "BRAM_TOP")
                {
                    props.push(Box::new(VirtexPinBramLv(wire_to)));
                } else if out_name.starts_with("LH") && tcname.ends_with("BRAM") {
                    props.push(Box::new(VirtexPinLh(wire_to)));
                } else if out_name.starts_with("LH") && tcname.starts_with("CLK") {
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
                        for &wire_pin in &tcls_index.pips_fwd[&wire_to] {
                            let wire_pin = wire_pin.tw;
                            let wire_pin_name = intdb.wires.key(wire_pin.wire);
                            if wire_pin_name.starts_with("HEX")
                                || wire_pin_name.starts_with("IMUX.BRAM")
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
                        if let Some(wire_unbuf) = in_wire_name.strip_suffix(".BUF") {
                            let wire_unbuf = TileWireCoord::new_idx(0, intdb.get_wire(wire_unbuf));
                            props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                            props.push(Box::new(WireMutexExclusive::new(wire_unbuf)));
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("OMUX")
                            || in_wire_name.starts_with("BRAM.QUAD.DOUT")
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
                            let wire_unbuf = TileWireCoord::new_idx(0, intdb.get_wire(wire_unbuf));
                            props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                            props.push(Box::new(WireMutexExclusive::new(wire_unbuf)));
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("LH") && tcname.starts_with("CNR") {
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
                            let wire_buf = TileWireCoord::new_idx(
                                0,
                                intdb.get_wire(&format!("{in_wire_name}.BUF")),
                            );
                            if tcname.ends_with("BRAM") {
                                let related = Delta::new(
                                    -1,
                                    wire_from.cell.to_idx() as i32 - 4,
                                    if tcname == "LBRAM" { "IO.L" } else { "CLB" },
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
                        } else if in_wire_name.starts_with("OUT.IO") {
                            for i in 0..4 {
                                props.push(Box::new(BaseBelMode::new(
                                    bels::IO[i],
                                    [
                                        "EMPTYIOB",
                                        "IOB",
                                        "IOB",
                                        if tcname == "IO.L" || tcname == "IO.R" {
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

                    let mut builder = ctx.build().test_manual("INT", &mux_name, &in_name);
                    for prop in &props {
                        builder = builder.prop_box(prop.clone());
                    }
                    builder.commit();
                }
            } else if out_name.contains("IMUX") {
                let mut props: Vec<Box<DynProp>> = vec![Box::new(WireMutexExclusive::new(wire_to))];
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
                        alt_out_wire = Some(TileWireCoord::new_idx(
                            0,
                            backend.egrid.db.get_wire("DLL.IMUX.CLKFB"),
                        ));
                    }
                    if out_name == "DLL.IMUX.CLKFB" {
                        alt_out_wire = Some(TileWireCoord::new_idx(
                            0,
                            backend.egrid.db.get_wire("DLL.IMUX.CLKIN"),
                        ));
                    }
                }
                if let Some(alt_out) = alt_out_wire {
                    props.push(Box::new(WireMutexExclusive::new(alt_out)));
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
                    for &wire_from in ins {
                        let wire_from = wire_from.tw;
                        let in_wire_name = intdb.wires.key(wire_from.wire);
                        'imux_pin: {
                            if let Some(wire_unbuf) = in_wire_name.strip_suffix(".BUF") {
                                let wire_unbuf =
                                    TileWireCoord::new_idx(0, intdb.get_wire(wire_unbuf));
                                props.push(Box::new(BaseIntPip::new(wire_from, wire_unbuf)));
                                props.push(Box::new(WireMutexExclusive::new(wire_unbuf)));
                                break 'imux_pin;
                            } else if out_name.starts_with("IMUX.BRAM.DI") {
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
                        if in_wire_name.starts_with("GCLK") || in_wire_name.ends_with("BUF") {
                            // no need to pin
                            break 'imux_pin;
                        } else if out_name.starts_with("IMUX.TBUF") && out_name.ends_with("I") {
                            // already pinned above
                            break 'imux_pin;
                        } else if out_name == "PCI.IMUX.I3" {
                            let wire_buf = TileWireCoord::new_idx(
                                0,
                                intdb.get_wire(&format!("{in_wire_name}.BUF")),
                            );
                            let related =
                                Delta::new(0, 0, if tcname == "CLKL" { "IO.L" } else { "IO.R" });
                            props.push(Box::new(Related::new(
                                related.clone(),
                                BaseIntPip::new(wire_buf, wire_from),
                            )));
                            props.push(Box::new(Related::new(
                                related,
                                WireMutexExclusive::new(wire_buf),
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
                        && in_wire_name.starts_with("CLK.OUT")
                    {
                        let mut builder =
                            ctx.build()
                                .test_manual("INT", &mux_name, format!("{in_name}.NOALT"));
                        for prop in &props {
                            builder = builder.prop_box(prop.clone());
                        }
                        builder.commit();
                        props.push(Box::new(BaseIntPip::new(alt_out, wire_from)));
                    }

                    let mut builder = ctx.build().test_manual("INT", &mux_name, &in_name);
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
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
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
                        let out_name = if tcls.cells.len() == 1 {
                            intdb.wires.key(mux.dst.wire).to_string()
                        } else {
                            format!("{:#}.{}", mux.dst.cell, intdb.wires.key(mux.dst.wire))
                        };
                        let mux_name = format!("MUX.{out_name}");

                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &wire_from in &mux.src {
                            let wire_from = wire_from.tw;
                            let in_name = if tcls.cells.len() == 1 {
                                intdb.wires.key(wire_from.wire).to_string()
                            } else {
                                format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire))
                            };
                            let mut diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                            if mux_name.contains("DLL.IMUX") && in_name.contains("CLK.OUT") {
                                let noalt_diff = ctx.state.get_diff(
                                    tcname,
                                    "INT",
                                    &mux_name,
                                    format!("{in_name}.NOALT"),
                                );
                                let (alt, noalt, common) = Diff::split(diff, noalt_diff);
                                if mux_name.contains("CLKIN") {
                                    ctx.tiledb
                                        .insert(tcname, "DLL", "CLKIN_PAD", xlat_bit(noalt));
                                    ctx.tiledb
                                        .insert(tcname, "DLL", "CLKFB_PAD", xlat_bit(!alt));
                                } else {
                                    ctx.tiledb
                                        .insert(tcname, "DLL", "CLKFB_PAD", xlat_bit(noalt));
                                    ctx.tiledb
                                        .insert(tcname, "DLL", "CLKIN_PAD", xlat_bit(!alt));
                                }
                                diff = common;
                            }
                            if in_name.starts_with("OUT.IO0")
                                || (in_name.starts_with("OUT.IO3")
                                    && matches!(&tcname[..], "IO.B" | "IO.T"))
                            {
                                diff.assert_empty();
                            } else if (out_name.contains("BRAM.QUAD")
                                && in_name.contains("BRAM.QUAD"))
                                || out_name.contains("BRAM.QUAD.DOUT")
                                || (out_name.contains("HEX.H") && in_name == "PCI_CE")
                                || (tcname.starts_with("CNR") && out_name.contains("LV"))
                                || (tcname.starts_with("BRAM_") && out_name.contains("LV"))
                            {
                                if diff.bits.is_empty() {
                                    println!("UMM {out_name} {in_name} BUF IS EMPTY");
                                    continue;
                                }
                                ctx.tiledb.insert(
                                    tcname,
                                    bel,
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
                            let mut drive_bits: HashSet<_> =
                                inps[0].1.bits.keys().copied().collect();
                            for (_, diff) in &inps {
                                drive_bits.retain(|bit| diff.bits.contains_key(bit));
                            }
                            if drive_bits.len() > 1 {
                                if tcname.starts_with("CNR") {
                                    // sigh. I give up. those are obtained from looking at left-hand
                                    // corners with easier-to-disambiguate muxes, and correlating with
                                    // bitstream geometry in right-hand corners. also confirmed by some
                                    // manual bitgen tests.
                                    drive_bits.retain(|bit| matches!(bit.frame % 6, 0 | 5));
                                } else {
                                    let btile = match &tcname[..] {
                                        "IO.L" => {
                                            edev.btile_main(edev.chip.col_w(), RowId::from_idx(1))
                                        }
                                        "IO.R" => {
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
                            let item = xlat_enum_ocd(inps, OcdMode::Mux);
                            ctx.tiledb.insert(tcname, bel, mux_name, item);
                            ctx.tiledb.insert(
                                tcname,
                                bel,
                                format!("DRIVE.{out_name}"),
                                xlat_bit(drive),
                            );
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
                                if mux_name.starts_with("MUX.HEX.S") && tcname == "IO.T"
                                    || mux_name.starts_with("MUX.HEX.N") && tcname == "IO.B"
                                    || mux_name.starts_with("MUX.HEX.E") && tcname == "IO.L"
                                    || mux_name.starts_with("MUX.HEX.W") && tcname == "IO.R"
                                {
                                    continue;
                                }
                                println!("UMMM MUX {tcname} {mux_name} is empty");
                            }
                            ctx.tiledb.insert(tcname, bel, mux_name, item);
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
                            ctx.state
                                .get_diff(tcname, "INT", format!("MUX.{out_name}"), &in_name);
                        if in_name.starts_with("OUT.IO0")
                            || matches!(&tcname[..], "IO.B" | "IO.T")
                                && in_name.starts_with("OUT.IO3")
                        {
                            diff.assert_empty();
                            continue;
                        }
                        if diff.bits.is_empty() {
                            println!("UMM {out_name} {in_name} PASS IS EMPTY");
                            continue;
                        }
                        let item = xlat_bit(diff);
                        let name = format!("PASS.{out_name}.{in_name}");
                        ctx.tiledb.insert(tcname, bel, name, item);
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
                            let diff = ctx.state.get_diff(
                                tcname,
                                "INT",
                                format!("MUX.{out_name}"),
                                &in_name,
                            );
                            let item = xlat_bit(diff);
                            ctx.tiledb.insert(tcname, bel, &name, item);
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
