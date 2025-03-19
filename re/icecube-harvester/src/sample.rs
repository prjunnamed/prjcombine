use std::collections::{BTreeMap, HashMap, HashSet};

use bitvec::prelude::*;
use prjcombine_interconnect::{
    db::{NodeKindId, NodeWireId, WireId},
    grid::{ColId, DieId, EdgeIoCoord, IntWire, RowId, TileIobId},
};
use prjcombine_re_harvester::Sample;
use prjcombine_siliconblue::{
    bels,
    bitstream::Bitstream,
    chip::{ChipKind, ExtraNodeIo, ExtraNodeLoc},
    expanded::{BitOwner, ExpandedDevice},
};
use unnamed_entity::EntityId;

use crate::{
    run::{Design, InstId, InstPin, InstPinSource, RunResult},
    xlat::{GenericNet, xlat_mux_in, xlat_wire},
};

#[allow(clippy::too_many_arguments)]
pub fn make_sample(
    design: &Design,
    edev: &ExpandedDevice,
    runres: &RunResult,
    empty: &RunResult,
    xlat_col: &[ColId],
    xlat_row: &[RowId],
    xlat_io: &BTreeMap<(u32, u32, u32), EdgeIoCoord>,
    rows_colbuf: &[(RowId, RowId, RowId)],
    extra_wire_names: &BTreeMap<(u32, u32, String), IntWire>,
) -> (Sample<BitOwner>, HashSet<(NodeKindId, WireId, WireId)>) {
    let mut sample = Sample::default();
    let mut pips = HashSet::new();
    let die = edev.egrid.die(DieId::from_idx(0));
    let diff = Bitstream::diff(&empty.bitstream, &runres.bitstream);
    let mut fucked_bits = 0;
    for (bit, val) in diff {
        if let Some((tile, owner)) = edev.classify_bit(bit) {
            let (tframe, tbit) = tile.xlat_pos_rev(bit).unwrap();
            sample.diff.insert((owner, tframe, tbit), val);
        } else {
            println!("DIFF UNK: {bit:?} {val}");
            fucked_bits += 1;
        }
    }
    if fucked_bits != 0 {
        panic!("FUCKED: {fucked_bits}");
    }
    let mut int_source: HashMap<IntWire, (InstId, InstPin)> = HashMap::new();
    let mut ibuf_used = HashSet::new();
    let mut gb_io_used = HashSet::new();
    for (&(src_inst, ref src_pin), route) in &runres.routes {
        for subroute in route {
            for window in subroute.windows(2) {
                let &[(ax, ay, ref aw), (bx, by, ref bw)] = window else {
                    unreachable!()
                };
                let na = if let Some(&iw) = extra_wire_names.get(&(ax, ay, aw.clone())) {
                    GenericNet::Int(iw)
                } else {
                    xlat_wire(edev, ax, ay, aw)
                };
                let nb = if let Some(&iw) = extra_wire_names.get(&(bx, by, bw.clone())) {
                    GenericNet::Int(iw)
                } else {
                    xlat_wire(edev, bx, by, bw)
                };
                if na == nb {
                    continue;
                }
                match (na, nb) {
                    (GenericNet::Int(iwa), GenericNet::Int(iwb)) => {
                        int_source.insert(iwb, (src_inst, src_pin.clone()));
                        let (col, row, wa, wb) =
                            xlat_mux_in(edev, iwa, iwb, (ax, ay, aw), (bx, by, bw));
                        let node = die[(col, row)].nodes.first().unwrap();
                        let mut tile_name: &str = edev.egrid.db.nodes.key(node.kind);
                        if tile_name == "INT" {
                            tile_name = "PLB";
                        }
                        let wan = edev.egrid.db.wires.key(wa);
                        let wbn = edev.egrid.db.wires.key(wb);
                        if wbn.starts_with("GLOBAL") {
                            assert_eq!(wan, "IMUX.IO.EXTRA");
                            continue;
                        }
                        pips.insert((node.kind, wb, wa));
                        let key = format!("{tile_name}:INT:MUX.{wbn}:{wan}");
                        if (wbn.starts_with("QUAD") || wbn.starts_with("LONG"))
                            && wan.starts_with("OUT")
                        {
                            sample.add_tiled_pattern_single(&[BitOwner::Main(col, row)], key);
                        } else {
                            sample.add_tiled_pattern(&[BitOwner::Main(col, row)], key);
                        }
                        if wan.starts_with("GLOBAL") && edev.chip.kind.has_colbuf() {
                            let idx: usize = wan.strip_prefix("GLOBAL.").unwrap().parse().unwrap();
                            if !rows_colbuf.is_empty() {
                                let (row_colbuf, _, _) = rows_colbuf
                                    .iter()
                                    .copied()
                                    .find(|&(_, row_b, row_t)| row >= row_b && row < row_t)
                                    .unwrap();
                                let trow = if row < row_colbuf {
                                    if edev.chip.cols_bram.contains(&col)
                                        && !edev.chip.kind.has_ice40_bramv2()
                                    {
                                        row_colbuf - 2
                                    } else {
                                        row_colbuf - 1
                                    }
                                } else {
                                    row_colbuf
                                };
                                let cb_tile_name = if edev.chip.kind.has_io_we()
                                    && col == edev.chip.col_w()
                                {
                                    "IO.W"
                                } else if edev.chip.kind.has_io_we() && col == edev.chip.col_e() {
                                    "IO.E"
                                } else if edev.chip.cols_bram.contains(&col) {
                                    "INT.BRAM"
                                } else {
                                    "PLB"
                                };
                                sample.add_tiled_pattern(
                                    &[BitOwner::Main(col, trow)],
                                    format!("{cb_tile_name}:COLBUF:GLOBAL.{idx}:BIT0"),
                                );
                            } else {
                                sample
                                    .add_global_pattern_single(format!("COLBUF:{col}.{row}.{idx}"));
                            };
                        }
                    }
                    (GenericNet::Ltout(col, row, lc), GenericNet::Int(iwb)) => {
                        let dst_lc = if lc == 7 {
                            println!("long ltout edge {ax}:{ay}:{aw} -> {bx}:{by}:{bw}");
                            assert_eq!((col, row + 1), iwb.1);
                            assert_eq!(iwb.2, edev.egrid.db.get_wire("IMUX.LC0.I2"));
                            0
                        } else {
                            assert_eq!((col, row), iwb.1);
                            assert_eq!(
                                iwb.2,
                                edev.egrid
                                    .db
                                    .get_wire(&format!("IMUX.LC{i}.I2", i = lc + 1))
                            );
                            lc + 1
                        };
                        sample.add_tiled_pattern(
                            &[BitOwner::Main(iwb.1.0, iwb.1.1)],
                            format!("PLB:LC{dst_lc}:MUX.I2:LTIN"),
                        );
                        int_source.insert(iwb, (src_inst, InstPin::Simple("O".to_string())));
                    }
                    (GenericNet::Cout(col, row, lc), GenericNet::Int(iwb)) => {
                        assert_ne!(lc, 7);
                        assert_eq!((col, row), iwb.1);
                        let dst_lc = lc + 1;
                        assert_eq!(
                            iwb.2,
                            edev.egrid.db.get_wire(&format!("IMUX.LC{dst_lc}.I3"))
                        );
                        sample.add_tiled_pattern(
                            &[BitOwner::Main(iwb.1.0, iwb.1.1)],
                            format!("PLB:INT:MUX.IMUX.LC{dst_lc}.I3:CI"),
                        );
                        int_source.insert(iwb, (src_inst, src_pin.clone()));
                    }
                    (GenericNet::Int(iwa), GenericNet::CascAddr(col, row, idx)) => {
                        assert_eq!(iwa.1, (col, row));
                        let xi = if edev.chip.kind.has_ice40_bramv2() {
                            idx ^ 7
                        } else {
                            idx
                        };
                        let lc = xi % 8;
                        let ii = if xi >= 8 { 2 } else { 0 };
                        assert_eq!(
                            *edev.egrid.db.wires.key(iwa.2),
                            format!("IMUX.LC{lc}.I{ii}")
                        );
                        let (row, which) = if row.to_idx() % 2 == 1 {
                            (
                                row,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    "RADDR"
                                } else {
                                    "WADDR"
                                },
                            )
                        } else {
                            (
                                row - 1,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    "WADDR"
                                } else {
                                    "RADDR"
                                },
                            )
                        };
                        let tiles = [BitOwner::Main(col, row), BitOwner::Main(col, row + 1)];
                        sample.add_tiled_pattern(
                            &tiles,
                            format!("BRAM:BRAM:CASCADE_OUT_{which}:BIT0"),
                        );
                    }
                    (GenericNet::CascAddr(col, row, idx), GenericNet::Int(iwb)) => {
                        assert_eq!(iwb.1, (col, row - 2));
                        let xi = if edev.chip.kind.has_ice40_bramv2() {
                            idx ^ 7
                        } else {
                            idx
                        };
                        let lc = xi % 8;
                        let ii = if xi >= 8 { 2 } else { 0 };
                        assert_eq!(
                            *edev.egrid.db.wires.key(iwb.2),
                            format!("IMUX.LC{lc}.I{ii}")
                        );
                        let (row, which) = if row.to_idx() % 2 == 1 {
                            (
                                row - 2,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    "RADDR"
                                } else {
                                    "WADDR"
                                },
                            )
                        } else {
                            (
                                row - 3,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    "WADDR"
                                } else {
                                    "RADDR"
                                },
                            )
                        };
                        let tiles = [BitOwner::Main(col, row), BitOwner::Main(col, row + 1)];
                        sample.add_tiled_pattern(
                            &tiles,
                            format!("BRAM:BRAM:CASCADE_IN_{which}:BIT0"),
                        );
                    }
                    (GenericNet::Gbout(_, _, _), GenericNet::GlobalPadIn(_, _)) => {
                        // handled below
                    }
                    (GenericNet::GlobalPadIn(_, _), GenericNet::Int(iw)) => {
                        let idx = edev
                            .egrid
                            .db
                            .wires
                            .key(iw.2)
                            .strip_prefix("GLOBAL.")
                            .unwrap();
                        let idx: usize = idx.parse().unwrap();
                        sample.add_tiled_pattern(
                            &[BitOwner::Clock(0), BitOwner::Clock(1)],
                            format!("GBOUT:GBOUT:MUX.GLOBAL.{idx}:IO"),
                        );
                    }
                    _ => {
                        panic!("umm weird edge {ax}:{ay}:{aw} -> {bx}:{by}:{bw}");
                    }
                }
            }
        }
        let inst = &design.insts[src_inst];
        if matches!(
            &inst.kind[..],
            "SB_IO" | "SB_IO_DS" | "SB_GB_IO" | "SB_IO_OD" | "SB_IO_I3C"
        ) {
            ibuf_used.insert(src_inst);
            if *src_pin == InstPin::Simple("GLOBAL_BUFFER_OUTPUT".into()) {
                gb_io_used.insert(src_inst);
            }
        }
    }
    for (iid, inst) in &design.insts {
        if let Some(loc) = runres.loc_map.get(iid) {
            match &inst.kind[..] {
                "SB_LUT4" => {
                    let col = xlat_col[loc.loc.x as usize];
                    let row = xlat_row[loc.loc.y as usize];
                    let lc = loc.loc.bel;
                    if let Some(lut_init) = inst.props.get("LUT_INIT") {
                        if lut_init != "16'h0000" {
                            let lut_init =
                                u16::from_str_radix(lut_init.strip_prefix("16'h").unwrap(), 16)
                                    .unwrap();
                            let mut swz_init: u16 = 0;
                            let pin_to_orig: HashMap<_, _> = (0..4)
                                .map(|idx| {
                                    let src = &inst.pins[&InstPin::Simple(format!("I{idx}"))];
                                    let InstPinSource::FromInst(si, ref sp) = *src else {
                                        unreachable!()
                                    };
                                    ((si, sp.clone()), idx)
                                })
                                .collect();
                            let swz_to_orig = Vec::from_iter((0..4).map(|idx| {
                                if let Some(src) = int_source.get(&(
                                    DieId::from_idx(0),
                                    (col, row),
                                    edev.egrid.db.get_wire(&format!("IMUX.LC{lc}.I{idx}")),
                                )) {
                                    pin_to_orig[src]
                                } else if idx == 3 {
                                    let InstPinSource::FromInst(_cid, cpin) =
                                        &inst.pins[&InstPin::Simple("I3".into())]
                                    else {
                                        unreachable!();
                                    };
                                    assert_eq!(*cpin, InstPin::Simple("CO".into()));
                                    sample.add_tiled_pattern(
                                        &[BitOwner::Main(col, row)],
                                        format!("PLB:INT:MUX.IMUX.LC{lc}.I3:CI"),
                                    );
                                    if lc == 0 {
                                        sample.add_tiled_pattern(
                                            &[BitOwner::Main(col, row)],
                                            format!("PLB:LC{lc}:MUX.CI:CHAIN"),
                                        );
                                    }
                                    3
                                } else {
                                    panic!("NO LUT INPUT {iid} {idx}");
                                }
                            }));
                            for swz_index in 0..16 {
                                let mut orig_index = 0;
                                for swz_input in 0..4 {
                                    if (swz_index & (1 << swz_input)) != 0 {
                                        let orig_input = swz_to_orig[swz_input];
                                        orig_index |= 1 << orig_input;
                                    }
                                }
                                if (lut_init & (1 << orig_index)) != 0 {
                                    swz_init |= 1 << swz_index;
                                }
                            }
                            for i in 0..16 {
                                if (swz_init & (1 << i)) != 0 {
                                    sample.add_tiled_pattern_single(
                                        &[BitOwner::Main(col, row)],
                                        format!("PLB:LC{lc}:LUT_INIT:BIT{i}"),
                                    );
                                }
                            }
                        }
                    }
                }
                "SB_CARRY" => {
                    let col = xlat_col[loc.loc.x as usize];
                    let row = xlat_row[loc.loc.y as usize];
                    let lc = loc.loc.bel;
                    if lc == 0 {
                        let ci = &inst.pins[&InstPin::Simple("CI".into())];
                        if matches!(ci, InstPinSource::Gnd) {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                format!("PLB:LC{lc}:MUX.CI:0"),
                            );
                        } else if matches!(ci, InstPinSource::Vcc) {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                format!("PLB:LC{lc}:MUX.CI:1"),
                            );
                        } else {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                format!("PLB:LC{lc}:MUX.CI:CHAIN"),
                            );
                        }
                    }
                    sample.add_tiled_pattern_single(
                        &[BitOwner::Main(col, row)],
                        format!("PLB:LC{lc}:CARRY_ENABLE:BIT0"),
                    );
                }
                "SB_IO" | "SB_IO_DS" | "SB_GB_IO" | "SB_IO_OD" | "SB_IO_I3C" => {
                    let io = xlat_io[&(loc.loc.x, loc.loc.y, loc.loc.bel)];
                    let (_, (col, row), slot) = edev.chip.get_io_loc(io);
                    let iob = io.iob();
                    let slot_name = edev.egrid.db.bel_slots[slot].as_str();
                    let tile_kind = match io {
                        EdgeIoCoord::W(..) => "IO.W",
                        EdgeIoCoord::E(..) => "IO.E",
                        EdgeIoCoord::S(..) => "IO.S",
                        EdgeIoCoord::N(..) => "IO.N",
                    };
                    let mut global_idx = None;
                    for (&loc, node) in &edev.chip.extra_nodes {
                        if let ExtraNodeLoc::GbIo(idx) = loc {
                            if node.io[&ExtraNodeIo::GbIn] == io {
                                global_idx = Some(idx);
                            }
                        }
                    }

                    let iostd = inst.props.get("IO_STANDARD").map(|x| x.as_str());
                    let is_lvds = iostd == Some("SB_LVDS_INPUT");
                    if is_lvds {
                        sample.add_tiled_pattern_single(
                            &[BitOwner::Main(col, row)],
                            format!("{tile_kind}:IO:LVDS_INPUT:BIT0"),
                        );
                        let oiob = TileIobId::from_idx(iob.to_idx() ^ 1);
                        sample.add_global_pattern(format!("IO:{col}.{row}.{oiob}:PULLUP:DISABLE"));
                    }

                    if let Some(pin_type) = inst.props.get("PIN_TYPE") {
                        let mut value = bitvec![];
                        for (i, c) in pin_type.chars().rev().enumerate() {
                            if i >= 6 {
                                break;
                            }
                            assert!(c == '0' || c == '1');
                            value.push(c == '1');
                            if c == '1' {
                                sample.add_tiled_pattern_single(
                                    &[BitOwner::Main(col, row)],
                                    format!("{tile_kind}:{slot_name}:PIN_TYPE:BIT{i}"),
                                );
                            }
                        }
                        if (value[4] || value[5])
                            && matches!(
                                design.kind,
                                ChipKind::Ice40T01 | ChipKind::Ice40T04 | ChipKind::Ice40T05
                            )
                        {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                format!("{tile_kind}:{slot_name}:OUTPUT_ENABLE:BIT0"),
                            );
                            if is_lvds {
                                let oiob = TileIobId::from_idx(iob.to_idx() ^ 1);
                                sample.add_tiled_pattern(
                                    &[BitOwner::Main(col, row)],
                                    format!("{tile_kind}:IO{oiob}:OUTPUT_ENABLE:BIT0"),
                                );
                            }
                        }
                        if value[1] && inst.kind == "SB_GB_IO" {
                            let key = ExtraNodeLoc::GbIo(global_idx.unwrap());
                            sample.add_global_pattern(format!("{key}:LATCH_GLOBAL_OUT:BIT0"));
                        }
                    }
                    if let Some(neg_trigger) = inst.props.get("NEG_TRIGGER") {
                        if neg_trigger.ends_with('1') {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                format!("{tile_kind}:IO:NEG_TRIGGER:BIT0"),
                            );
                        }
                    }

                    if inst.kind == "SB_IO_I3C" {
                        let weak_pullup = &inst.props["WEAK_PULLUP"];
                        if weak_pullup.ends_with("0") {
                            sample.add_global_pattern(format!(
                                "IO:{col}.{row}.{iob}:I3C_WEAK_PULLUP:DISABLE"
                            ));
                        } else {
                            sample.add_global_pattern(format!(
                                "IO:{col}.{row}.{iob}:I3C_WEAK_PULLUP:ENABLE"
                            ));
                        }
                        let pullup = &inst.props["PULLUP"];
                        if pullup.ends_with("1") {
                            let pullup_kind = &inst.props["PULLUP_RESISTOR"];
                            sample.add_global_pattern(format!(
                                "IO:{col}.{row}.{iob}:I3C_PULLUP:{pullup_kind}"
                            ));
                        } else {
                            sample.add_global_pattern(format!(
                                "IO:{col}.{row}.{iob}:I3C_PULLUP:DISABLE"
                            ));
                        }
                    } else {
                        if let Some(pullup) = inst.props.get("PULLUP") {
                            if pullup.ends_with('0') || is_lvds {
                                sample.add_global_pattern(format!(
                                    "IO:{col}.{row}.{iob}:PULLUP:DISABLE"
                                ));
                            } else if let Some(pullup_kind) = inst.props.get("PULLUP_RESISTOR") {
                                sample.add_global_pattern(format!(
                                    "IO:{col}.{row}.{iob}:PULLUP:{pullup_kind}"
                                ));
                            }
                        } else {
                            sample
                                .add_global_pattern(format!("IO:{col}.{row}.{iob}:PULLUP:DISABLE"));
                        }
                    }
                    if let Some(iostd) = iostd {
                        if col == edev.chip.col_w() && edev.chip.kind.has_vref() {
                            sample
                                .add_global_pattern(format!("IO:{col}.{row}.{iob}:IOSTD:{iostd}"));
                        }
                    }

                    if ibuf_used.contains(&iid) && !is_lvds {
                        if col == edev.chip.col_w() && edev.chip.kind.has_vref() {
                            let iostd = inst.props["IO_STANDARD"].as_str();
                            let mode = match iostd {
                                "SB_SSTL2_CLASS_2" | "SB_SSTL2_CLASS_1" | "SB_SSTL18_FULL"
                                | "SB_SSTL18_HALF" => "VREF",
                                "SB_LVDS_INPUT" => "DIFF",
                                _ => "CMOS",
                            };
                            sample.add_global_pattern(format!(
                                "IO:{col}.{row}.{iob}:IBUF_ENABLE:{mode}"
                            ));
                        } else {
                            sample.add_global_pattern(format!(
                                "IO:{col}.{row}.{iob}:IBUF_ENABLE:BIT0"
                            ));
                        }
                    }

                    // TODO: IO_STANDARD (LVDS, VREF, ...)
                    // TODO: I3C specials
                }
                kind if kind.starts_with("SB_DFF") => {
                    let col = xlat_col[loc.loc.x as usize];
                    let row = xlat_row[loc.loc.y as usize];
                    let lc = loc.loc.bel;
                    let mut kind = kind.strip_prefix("SB_DFF").unwrap();
                    sample.add_tiled_pattern_single(
                        &[BitOwner::Main(col, row)],
                        format!("PLB:LC{lc}:FF_ENABLE:BIT0"),
                    );
                    if let Some(rest) = kind.strip_prefix('N') {
                        sample.add_tiled_pattern_single(
                            &[BitOwner::Main(col, row)],
                            "PLB:INT:INV.IMUX.CLK:BIT0",
                        );
                        kind = rest;
                    }
                    if let Some(rest) = kind.strip_prefix('E') {
                        kind = rest;
                    }
                    match kind {
                        "SR" => (),
                        "SS" => {
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                format!("PLB:LC{lc}:FF_SR_VALUE:BIT0"),
                            );
                        }
                        "R" => {
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                format!("PLB:LC{lc}:FF_SR_ASYNC:BIT0"),
                            );
                        }
                        "S" => {
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                format!("PLB:LC{lc}:FF_SR_VALUE:BIT0"),
                            );
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                format!("PLB:LC{lc}:FF_SR_ASYNC:BIT0"),
                            );
                        }
                        "" => (),
                        _ => unreachable!(),
                    }
                }
                kind if kind.starts_with("SB_RAM") => {
                    let node = edev.egrid.db.get_node("BRAM");
                    let node = &edev.egrid.db.nodes[node];
                    let bel_info = &node.bels[bels::BRAM];
                    let get_pin = |pin: &str| -> NodeWireId {
                        bel_info.pins[pin].wires.iter().copied().next().unwrap()
                    };
                    let get_pin_idx = |pin: &str, idx: usize| -> NodeWireId {
                        bel_info.pins[&format!("{pin}{idx}")]
                            .wires
                            .iter()
                            .copied()
                            .next()
                            .unwrap()
                    };
                    let col = xlat_col[loc.loc.x as usize];
                    let row = xlat_row[loc.loc.y as usize];
                    let tiles = [BitOwner::Main(col, row), BitOwner::Main(col, row + 1)];
                    for (key, pin, pinn) in [("NW", "WCLK", "WCLKN"), ("NR", "RCLK", "RCLKN")] {
                        let (tile, wire) = get_pin(pin);
                        let irow = row + tile.to_idx();
                        if kind.contains(key) {
                            let pin = InstPin::Simple(pinn.into());
                            if inst.pins.contains_key(&pin) {
                                let src =
                                    int_source[&(DieId::from_idx(0), (col, irow), wire)].clone();
                                assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                            }
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, irow)],
                                "INT.BRAM:INT:INV.IMUX.CLK:BIT0",
                            );
                        } else {
                            let pin = InstPin::Simple(pin.into());
                            if inst.pins.contains_key(&pin) {
                                let src =
                                    int_source[&(DieId::from_idx(0), (col, irow), wire)].clone();
                                assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                            }
                        }
                    }
                    for pin in ["WE", "RE", "WCLKE", "RCLKE"] {
                        let (tile, wire) = get_pin(pin);
                        let irow = row + tile.to_idx();
                        let pin = InstPin::Simple(pin.into());
                        if inst.pins.contains_key(&pin) {
                            let src = int_source[&(DieId::from_idx(0), (col, irow), wire)].clone();
                            assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                        }
                    }
                    let abits = if edev.chip.kind.is_ice40() { 11 } else { 8 };
                    for pin in ["WADDR", "RADDR"] {
                        for idx in 0..abits {
                            let (tile, wire) = get_pin_idx(pin, idx);
                            let irow = row + tile.to_idx();
                            let pin = InstPin::Indexed(pin.into(), idx);
                            if inst.pins.contains_key(&pin) {
                                let Some(src) =
                                    int_source.get(&(DieId::from_idx(0), (col, irow), wire))
                                else {
                                    // avoid cascade problems.
                                    continue;
                                };
                                let src = src.clone();
                                assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                            }
                        }
                    }
                    for pin in ["RDATA", "WDATA", "MASK"] {
                        for idx in 0..16 {
                            let (tile, wire) = get_pin_idx(pin, idx);
                            let irow = row + tile.to_idx();
                            let pin = InstPin::Indexed(pin.into(), idx);
                            if inst.pins.contains_key(&pin) {
                                let Some(src) =
                                    int_source.get(&(DieId::from_idx(0), (col, irow), wire))
                                else {
                                    // avoid unconnected output etc. problems
                                    continue;
                                };
                                let src = src.clone();
                                assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                            }
                        }
                    }

                    if design.kind.is_ice40() {
                        sample.add_tiled_pattern_single(&tiles, "BRAM:BRAM:ENABLE:BIT0");
                    }
                    if let Some(read_mode) = inst.props.get("READ_MODE") {
                        sample
                            .add_tiled_pattern(&tiles, format!("BRAM:BRAM:READ_MODE:{read_mode}"));
                    }
                    if let Some(write_mode) = inst.props.get("WRITE_MODE") {
                        sample.add_tiled_pattern(
                            &tiles,
                            format!("BRAM:BRAM:WRITE_MODE:{write_mode}"),
                        );
                    }
                    for i in 0..16 {
                        if let Some(init) = inst.props.get(&format!("INIT_{i:X}")) {
                            for j in 0..64 {
                                let pos = init.len() - 1 - j;
                                let digit = u8::from_str_radix(&init[pos..pos + 1], 16).unwrap();
                                for k in 0..4 {
                                    if ((digit >> k) & 1) != 0 {
                                        let bit = (i << 8) | (j << 2) | k;
                                        sample.add_tiled_pattern(
                                            &[BitOwner::Bram(col, row)],
                                            format!("BRAM_DATA:BRAM:INIT:BIT{bit}"),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
    for opt in &design.opts {
        match opt.as_str() {
            "--frequency low" => {
                sample.add_tiled_pattern(&[BitOwner::Speed], "SPEED:SPEED:SPEED:LOW");
            }
            "--frequency medium" => {
                sample.add_tiled_pattern(&[BitOwner::Speed], "SPEED:SPEED:SPEED:MEDIUM");
            }
            "--frequency high" => {
                sample.add_tiled_pattern(&[BitOwner::Speed], "SPEED:SPEED:SPEED:HIGH");
            }
            _ => panic!("ummm {opt}"),
        }
    }
    (sample, pips)
}

pub fn wanted_keys_tiled(edev: &ExpandedDevice) -> Vec<String> {
    let mut result = vec![];
    // PLB
    for lc in 0..8 {
        if lc != 0 && edev.chip.kind.is_ice40() {
            result.push(format!("PLB:LC{lc}:MUX.I2:LTIN"));
        }
        result.push(format!("PLB:INT:MUX.IMUX.LC{lc}.I3:CI"));
        for i in 0..16 {
            result.push(format!("PLB:LC{lc}:LUT_INIT:BIT{i}"));
        }
        result.push(format!("PLB:LC{lc}:CARRY_ENABLE:BIT0"));
        result.push(format!("PLB:LC{lc}:FF_ENABLE:BIT0"));
        result.push(format!("PLB:LC{lc}:FF_SR_VALUE:BIT0"));
        result.push(format!("PLB:LC{lc}:FF_SR_ASYNC:BIT0"));
    }
    result.push("PLB:LC0:MUX.CI:0".into());
    result.push("PLB:LC0:MUX.CI:1".into());
    result.push("PLB:LC0:MUX.CI:CHAIN".into());
    result.push("PLB:INT:INV.IMUX.CLK:BIT0".into());
    if edev.chip.kind.has_colbuf() {
        for i in 0..8 {
            result.push(format!("PLB:COLBUF:GLOBAL.{i}:BIT0"));
            result.push(format!("INT.BRAM:COLBUF:GLOBAL.{i}:BIT0"));
            // TODO: adjust [?]
            if edev.chip.kind.has_actual_io_we() {
                result.push(format!("IO.W:COLBUF:GLOBAL.{i}:BIT0"));
                result.push(format!("IO.E:COLBUF:GLOBAL.{i}:BIT0"));
            }
        }
    }
    // BRAM
    if !edev.chip.cols_bram.is_empty() {
        if edev.chip.kind.is_ice40() {
            result.push("BRAM:BRAM:CASCADE_OUT_WADDR:BIT0".into());
            result.push("BRAM:BRAM:CASCADE_OUT_RADDR:BIT0".into());
            result.push("BRAM:BRAM:CASCADE_IN_WADDR:BIT0".into());
            result.push("BRAM:BRAM:CASCADE_IN_RADDR:BIT0".into());
            result.push("BRAM:BRAM:ENABLE:BIT0".into());
            result.push("BRAM:BRAM:READ_MODE:0".into());
            result.push("BRAM:BRAM:READ_MODE:1".into());
            result.push("BRAM:BRAM:READ_MODE:2".into());
            result.push("BRAM:BRAM:READ_MODE:3".into());
            result.push("BRAM:BRAM:WRITE_MODE:0".into());
            result.push("BRAM:BRAM:WRITE_MODE:1".into());
            result.push("BRAM:BRAM:WRITE_MODE:2".into());
            result.push("BRAM:BRAM:WRITE_MODE:3".into());
        }
        result.push("INT.BRAM:INT:INV.IMUX.CLK:BIT0".into());
        for i in 0..4096 {
            result.push(format!("BRAM_DATA:BRAM:INIT:BIT{i}"));
        }
    }
    // IO
    for tile in ["IO.W", "IO.E", "IO.S", "IO.N"] {
        if matches!(tile, "IO.W" | "IO.E") && !edev.chip.kind.has_actual_io_we() {
            continue;
        }
        for io in 0..2 {
            for i in 0..6 {
                result.push(format!("{tile}:IO{io}:PIN_TYPE:BIT{i}"));
            }
            if edev.chip.kind.is_ultra() {
                result.push(format!("{tile}:IO{io}:OUTPUT_ENABLE:BIT0"));
            }
        }
        result.push(format!("{tile}:IO:NEG_TRIGGER:BIT0"));
        let has_lvds = if edev.chip.kind == ChipKind::Ice65L01 {
            false
        } else if edev.chip.kind.has_actual_io_we() {
            tile == "IO.W"
        } else if edev.chip.kind == ChipKind::Ice40R04 {
            tile == "IO.N"
        } else {
            true
        };
        if has_lvds {
            result.push(format!("{tile}:IO:LVDS_INPUT:BIT0"));
        }
    }
    // misc
    for i in 0..8 {
        if matches!(i, 4 | 5) && !edev.chip.kind.has_io_we() {
            // TODO: remove
            continue;
        }
        result.push(format!("GBOUT:GBOUT:MUX.GLOBAL.{i}:IO"));
    }
    if edev.chip.kind != ChipKind::Ice40T04 {
        result.push("SPEED:SPEED:SPEED:LOW".into());
        result.push("SPEED:SPEED:SPEED:MEDIUM".into());
        result.push("SPEED:SPEED:SPEED:HIGH".into());
    }
    result
}

pub fn wanted_keys_global(edev: &ExpandedDevice) -> Vec<String> {
    let mut result = vec![];
    for &loc in edev.chip.extra_nodes.keys() {
        match loc {
            ExtraNodeLoc::GbFabric(_) => (),
            ExtraNodeLoc::GbIo(_) => {
                result.push(format!("{loc}:LATCH_GLOBAL_OUT:BIT0"));
            }
            ExtraNodeLoc::LatchIo(_) => (),
            ExtraNodeLoc::Warmboot => (),
            ExtraNodeLoc::IoI3c(crd) => {
                let (_, (col, row), _) = edev.chip.get_io_loc(crd);
                let iob = crd.iob();
                result.push(format!("IO:{col}.{row}.{iob}:I3C_WEAK_PULLUP:DISABLE"));
                result.push(format!("IO:{col}.{row}.{iob}:I3C_WEAK_PULLUP:ENABLE"));
                result.push(format!("IO:{col}.{row}.{iob}:I3C_PULLUP:DISABLE"));
                result.push(format!("IO:{col}.{row}.{iob}:I3C_PULLUP:3P3K"));
                result.push(format!("IO:{col}.{row}.{iob}:I3C_PULLUP:6P8K"));
                result.push(format!("IO:{col}.{row}.{iob}:I3C_PULLUP:10K"));
            }
            // TODO from here on
            ExtraNodeLoc::Pll(_) => (),
            ExtraNodeLoc::Spi(_) => (),
            ExtraNodeLoc::I2c(_) => (),
            ExtraNodeLoc::I2cFifo(_) => (),
            ExtraNodeLoc::LsOsc => (),
            ExtraNodeLoc::HsOsc => (),
            ExtraNodeLoc::LfOsc => (),
            ExtraNodeLoc::HfOsc => (),
            ExtraNodeLoc::IrDrv => (),
            ExtraNodeLoc::RgbDrv => (),
            ExtraNodeLoc::BarcodeDrv => (),
            ExtraNodeLoc::Ir400Drv => (),
            ExtraNodeLoc::RgbaDrv => (),
            ExtraNodeLoc::LedDrvCur => (),
            ExtraNodeLoc::LeddIp => (),
            ExtraNodeLoc::LeddaIp => (),
            ExtraNodeLoc::IrIp => (),
            ExtraNodeLoc::Mac16(_, _) => (),
            ExtraNodeLoc::SpramPair(_) => (),
        }
    }
    for &crd in edev.chip.io_iob.keys() {
        let (_, (col, row), _) = edev.chip.get_io_loc(crd);
        let iob = crd.iob();
        let is_od = edev.chip.io_od.contains(&crd);
        result.push(format!("IO:{col}.{row}.{iob}:IBUF_ENABLE:BIT0"));
        result.push(format!("IO:{col}.{row}.{iob}:PULLUP:DISABLE"));
        if matches!(edev.chip.kind, ChipKind::Ice40T01 | ChipKind::Ice40T05) && !is_od {
            result.push(format!("IO:{col}.{row}.{iob}:PULLUP:3P3K"));
            result.push(format!("IO:{col}.{row}.{iob}:PULLUP:6P8K"));
            result.push(format!("IO:{col}.{row}.{iob}:PULLUP:10K"));
            result.push(format!("IO:{col}.{row}.{iob}:PULLUP:100K"));
        }
    }
    result
}

pub fn get_golden_mux_stats(kind: ChipKind, nkn: &str) -> BTreeMap<String, usize> {
    let mut golden_stats = BTreeMap::new();
    if !nkn.starts_with("IO") {
        golden_stats.insert("IMUX.CLK".to_string(), 12);
        golden_stats.insert("IMUX.CE".to_string(), 8);
        golden_stats.insert("IMUX.RST".to_string(), 8);
        for lc in 0..8 {
            for i in 0..4 {
                if i == 2 && nkn == "INT.BRAM" {
                    if kind.is_ice65() {
                        continue;
                    } else if kind.has_ice40_bramv2() {
                        if lc < 5 {
                            continue;
                        }
                    } else {
                        if lc >= 3 {
                            continue;
                        }
                    }
                }
                golden_stats.insert(format!("IMUX.LC{lc}.I{i}"), if i == 3 { 15 } else { 16 });
            }
        }
        for g in 0..4 {
            for i in 0..8 {
                golden_stats.insert(
                    format!("LOCAL.{g}.{i}"),
                    if g == 0 && i >= 4 { 23 } else { 16 },
                );
            }
        }
        for (k, v) in [
            ("LONG-LONG.H", 12),
            ("LONG-LONG.V", 12),
            ("LONG-QUAD.H", 12),
            ("LONG-QUAD.V", 12),
            ("OUT-LONG.H", 12),
            ("OUT-LONG.V", 12),
            ("OUT-QUAD.H", 24),
            ("OUT-QUAD.V", 48),
            ("QUAD-QUAD.H", 168),
            ("QUAD-QUAD.V", 168),
        ] {
            golden_stats.insert(k.into(), v);
        }
    } else {
        for (k, v) in [
            ("IMUX.IO.ICLK", 12),
            ("IMUX.IO.OCLK", 12),
            ("IMUX.CE", 8),
            ("IMUX.IO.EXTRA", 8),
            ("IMUX.IO0.DOUT0", 8),
            ("IMUX.IO0.DOUT1", 8),
            ("IMUX.IO0.OE", 8),
            ("IMUX.IO1.DOUT0", 8),
            ("IMUX.IO1.DOUT1", 8),
            ("IMUX.IO1.OE", 8),
        ] {
            golden_stats.insert(k.into(), v);
        }
        for g in 0..2 {
            for i in 0..8 {
                golden_stats.insert(format!("LOCAL.{g}.{i}"), 14);
            }
        }
        if matches!(nkn, "IO.S" | "IO.N") {
            for (k, v) in [
                ("OUT-LONG.V", 12),
                ("OUT-QUAD.H", 16),
                ("OUT-QUAD.V", 24),
                ("QUAD-QUAD.H", 24),
                ("QUAD-QUAD.V", 24),
            ] {
                golden_stats.insert(k.into(), v);
            }
        } else {
            for (k, v) in [
                ("OUT-LONG.H", 12),
                ("OUT-QUAD.H", 24),
                ("OUT-QUAD.V", 16),
                ("QUAD-QUAD.H", 24),
                ("QUAD-QUAD.V", 24),
            ] {
                golden_stats.insert(k.into(), v);
            }
        }
    }
    golden_stats
}
