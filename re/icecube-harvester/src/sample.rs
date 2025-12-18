use std::collections::{BTreeMap, HashMap, HashSet};

use prjcombine_interconnect::{
    db::{BelInfo, PinDir, TileClassId, WireSlotId},
    dir::{Dir, DirH, DirV},
    grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId, TileIobId, WireCoord},
};
use prjcombine_re_harvester::Sample;
use prjcombine_siliconblue::{
    bels,
    bitstream::Bitstream,
    chip::{ChipKind, SpecialIoKey, SpecialTileKey},
    expanded::{BitOwner, ExpandedDevice},
    tslots,
};
use prjcombine_types::{bittile::BitTile as _, bitvec::BitVec};
use unnamed_entity::EntityId;

use crate::{
    PkgInfo,
    run::{Design, InstId, InstPin, InstPinSource, RawLoc, RunResult},
    xlat::{GenericNet, xlat_mux_in, xlat_wire},
};

fn get_main_tile_kind(edev: &ExpandedDevice, col: ColId, row: RowId) -> &'static str {
    if row == edev.chip.row_s() {
        if col == edev.chip.col_w() || col == edev.chip.col_e() {
            "CNR"
        } else {
            edev.chip.kind.tile_class_ioi(Dir::S).unwrap()
        }
    } else if row == edev.chip.row_n() {
        if col == edev.chip.col_w() || col == edev.chip.col_e() {
            "CNR"
        } else {
            edev.chip.kind.tile_class_ioi(Dir::N).unwrap()
        }
    } else {
        if edev.chip.kind.has_ioi_we() && col == edev.chip.col_w() {
            edev.chip.kind.tile_class_ioi(Dir::W).unwrap()
        } else if edev.chip.kind.has_ioi_we() && col == edev.chip.col_e() {
            edev.chip.kind.tile_class_ioi(Dir::E).unwrap()
        } else if edev.chip.cols_bram.contains(&col) {
            "INT_BRAM"
        } else {
            edev.chip.kind.tile_class_plb()
        }
    }
}

fn get_colbuf_tile_kind(edev: &ExpandedDevice, col: ColId) -> &'static str {
    if edev.chip.kind.has_ioi_we() && col == edev.chip.col_w() {
        "COLBUF_IO_W"
    } else if edev.chip.kind.has_ioi_we() && col == edev.chip.col_e() {
        "COLBUF_IO_E"
    } else {
        edev.chip.kind.tile_class_colbuf().unwrap()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn make_sample(
    design: &Design,
    edev: &ExpandedDevice,
    runres: &RunResult,
    pkg_info: &PkgInfo,
    rows_colbuf: &[(RowId, RowId, RowId)],
    extra_wire_names: &BTreeMap<(u32, u32, String), WireCoord>,
    special_tiles: &BTreeMap<SpecialTileKey, Vec<RawLoc>>,
) -> (
    Sample<BitOwner>,
    HashSet<(TileClassId, WireSlotId, WireSlotId)>,
) {
    let mut sample = Sample::default();
    let mut pips = HashSet::new();
    let diff = Bitstream::diff(&pkg_info.empty_run.bitstream, &runres.bitstream);
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
    let mut io_hardip_outs = HashSet::new();
    if edev.chip.kind == ChipKind::Ice40R04 {
        for key in [SpecialTileKey::LsOsc, SpecialTileKey::HsOsc] {
            let crd = *edev.chip.special_tiles[&key].cells.first().unwrap();
            let tile = &edev[crd.tile(tslots::OSC)];
            let tcls = &edev.db[tile.class];
            for (bslot, bel) in &tcls.bels {
                let BelInfo::Bel(bel) = bel else {
                    unreachable!()
                };
                for (pin, pin_info) in &bel.pins {
                    for wire in edev.get_bel_pin(crd.bel(bslot), pin) {
                        if pin_info.dir == PinDir::Output {
                            io_hardip_outs.insert(wire);
                        }
                    }
                }
            }
        }
    }
    let mut unoptinv = HashMap::new();
    for w in ["IMUX.CLK", "IMUX.IO.ICLK", "IMUX.IO.OCLK"] {
        let wo = edev.db.get_wire(&format!("{w}.OPTINV"));
        let wi = edev.db.get_wire(w);
        unoptinv.insert(wo, wi);
    }
    let mut int_source: HashMap<WireCoord, (InstId, InstPin)> = HashMap::new();
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
                        let (cell, wa, wb) =
                            xlat_mux_in(edev, iwa, iwb, (ax, ay, aw), (bx, by, bw));
                        let tile_name = get_main_tile_kind(edev, cell.col, cell.row);
                        let tile = &edev[cell.tile(tslots::MAIN)];
                        let wan = edev.db.wires.key(wa);
                        let wbn = edev.db.wires.key(wb);
                        if let Some(idx) = wbn.strip_prefix("GLOBAL.") {
                            if wan != "IMUX.IO.EXTRA" {
                                let tcls_gb_root = edev.chip.kind.tile_class_gb_root();
                                // SB_*OSC
                                assert!(wan.starts_with("OUT"));
                                let idx: usize = idx.parse().unwrap();
                                sample.add_tiled_pattern(
                                    &[BitOwner::Clock(0), BitOwner::Clock(1)],
                                    format!("{tcls_gb_root}:GB_ROOT:MUX.GLOBAL.{idx}:IO"),
                                );
                            }
                            continue;
                        }
                        pips.insert((tile.class, wb, wa));
                        let key = format!("{tile_name}:INT:MUX.{wbn}:{wan}");
                        if (wbn.starts_with("QUAD") || wbn.starts_with("LONG"))
                            && wan.starts_with("OUT")
                        {
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(cell.col, cell.row)],
                                key,
                            );
                        } else {
                            sample.add_tiled_pattern(&[BitOwner::Main(cell.col, cell.row)], key);
                        }
                        if wan.starts_with("GLOBAL") && edev.chip.kind.tile_class_colbuf().is_some()
                        {
                            let idx: usize = wan.strip_prefix("GLOBAL.").unwrap().parse().unwrap();
                            if !rows_colbuf.is_empty() {
                                let (row_colbuf, _, _) = rows_colbuf
                                    .iter()
                                    .copied()
                                    .find(|&(_, row_b, row_t)| {
                                        cell.row >= row_b && cell.row < row_t
                                    })
                                    .unwrap();
                                let trow = if cell.row < row_colbuf {
                                    if edev.chip.cols_bram.contains(&cell.col)
                                        && !edev.chip.kind.has_ice40_bramv2()
                                    {
                                        row_colbuf - 2
                                    } else {
                                        row_colbuf - 1
                                    }
                                } else {
                                    row_colbuf
                                };
                                let cb_tile_name = get_colbuf_tile_kind(edev, cell.col);
                                sample.add_tiled_pattern(
                                    &[BitOwner::Main(cell.col, trow)],
                                    format!("{cb_tile_name}:COLBUF:GLOBAL.{idx}:BIT0"),
                                );
                            } else {
                                sample.add_global_pattern_single(format!(
                                    "COLBUF:{col:#}.{row:#}.{idx}",
                                    col = cell.col,
                                    row = cell.row
                                ));
                            };
                        }
                        if io_hardip_outs.contains(&iwa) {
                            let crd = iwa.cell;
                            let wn = edev.db.wires.key(iwa.slot).as_str();
                            let io = match wn {
                                "OUT.LC0" | "OUT.LC4" => 0,
                                "OUT.LC2" | "OUT.LC6" => 1,
                                _ => unreachable!(),
                            };
                            let tile_name = get_main_tile_kind(edev, crd.col, crd.row);
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(crd.col, crd.row)],
                                format!("{tile_name}:IO{io}:PIN_TYPE:BIT0"),
                            );
                        }
                    }
                    (GenericNet::Ltout(cell, lc), GenericNet::Int(iwb)) => {
                        let dst_lc = if lc == 7 {
                            println!("long ltout edge {ax}:{ay}:{aw} -> {bx}:{by}:{bw}");
                            assert_eq!(cell.delta(0, 1), iwb.cell);
                            assert_eq!(iwb.slot, edev.db.get_wire("IMUX.LC0.I2"));
                            0
                        } else {
                            assert_eq!(cell, iwb.cell);
                            assert_eq!(
                                iwb.slot,
                                edev.db.get_wire(&format!("IMUX.LC{i}.I2", i = lc + 1))
                            );
                            lc + 1
                        };
                        let tcls = edev.chip.kind.tile_class_plb();
                        sample.add_tiled_pattern(
                            &[BitOwner::Main(iwb.cell.col, iwb.cell.row)],
                            format!("{tcls}:LC{dst_lc}:MUX.I2:LTIN"),
                        );
                        int_source.insert(iwb, (src_inst, InstPin::Simple("O".to_string())));
                    }
                    (GenericNet::Cout(cell, lc), GenericNet::Int(iwb)) => {
                        assert_ne!(lc, 7);
                        assert_eq!(cell, iwb.cell);
                        let dst_lc = lc + 1;
                        assert_eq!(iwb.slot, edev.db.get_wire(&format!("IMUX.LC{dst_lc}.I3")));
                        let tcls = edev.chip.kind.tile_class_plb();
                        sample.add_tiled_pattern(
                            &[BitOwner::Main(iwb.cell.col, iwb.cell.row)],
                            format!("{tcls}:INT:MUX.IMUX.LC{dst_lc}.I3:CI"),
                        );
                        int_source.insert(iwb, (src_inst, src_pin.clone()));
                    }
                    (GenericNet::Int(iwa), GenericNet::CascAddr(cell, idx)) => {
                        assert_eq!(iwa.cell, cell);
                        let xi = if edev.chip.kind.has_ice40_bramv2() {
                            idx ^ 7
                        } else {
                            idx
                        };
                        let lc = xi % 8;
                        let ii = if xi >= 8 { 2 } else { 0 };
                        assert_eq!(*edev.db.wires.key(iwa.slot), format!("IMUX.LC{lc}.I{ii}"));
                        let (row, which) = if cell.row.to_idx() % 2 == 1 {
                            (
                                cell.row,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    "RADDR"
                                } else {
                                    "WADDR"
                                },
                            )
                        } else {
                            (
                                cell.row - 1,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    "WADDR"
                                } else {
                                    "RADDR"
                                },
                            )
                        };
                        let tiles = [
                            BitOwner::Main(cell.col, row),
                            BitOwner::Main(cell.col, row + 1),
                        ];
                        let tile = edev.chip.kind.tile_class_bram();
                        sample.add_tiled_pattern(
                            &tiles,
                            format!("{tile}:BRAM:CASCADE_OUT_{which}:BIT0"),
                        );
                    }
                    (GenericNet::CascAddr(cell, idx), GenericNet::Int(iwb)) => {
                        assert_eq!(iwb.cell, cell.delta(0, -2));
                        let xi = if edev.chip.kind.has_ice40_bramv2() {
                            idx ^ 7
                        } else {
                            idx
                        };
                        let lc = xi % 8;
                        let ii = if xi >= 8 { 2 } else { 0 };
                        assert_eq!(*edev.db.wires.key(iwb.slot), format!("IMUX.LC{lc}.I{ii}"));
                        let (row, which) = if cell.row.to_idx() % 2 == 1 {
                            (
                                cell.row - 2,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    "RADDR"
                                } else {
                                    "WADDR"
                                },
                            )
                        } else {
                            (
                                cell.row - 3,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    "WADDR"
                                } else {
                                    "RADDR"
                                },
                            )
                        };
                        let tiles = [
                            BitOwner::Main(cell.col, row),
                            BitOwner::Main(cell.col, row + 1),
                        ];
                        let tile = edev.chip.kind.tile_class_bram();
                        sample.add_tiled_pattern(
                            &tiles,
                            format!("{tile}:BRAM:CASCADE_IN_{which}:BIT0"),
                        );
                    }
                    (GenericNet::Gbout(..), GenericNet::GlobalPadIn(_)) => {
                        // handled below
                    }
                    (GenericNet::Int(_), GenericNet::GlobalClkl | GenericNet::GlobalClkh) => {
                        // handled below
                    }
                    (
                        GenericNet::GlobalPadIn(_)
                        | GenericNet::GlobalClkl
                        | GenericNet::GlobalClkh,
                        GenericNet::Int(iw),
                    ) => {
                        let idx = edev.db.wires.key(iw.slot).strip_prefix("GLOBAL.").unwrap();
                        let idx: usize = idx.parse().unwrap();
                        let tcls_gb_root = edev.chip.kind.tile_class_gb_root();
                        sample.add_tiled_pattern(
                            &[BitOwner::Clock(0), BitOwner::Clock(1)],
                            format!("{tcls_gb_root}:GB_ROOT:MUX.GLOBAL.{idx}:IO"),
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
    let mut has_led_v2 = false;
    let mut led_v2_current_mode = false;
    for (iid, inst) in &design.insts {
        if let Some(loc) = runres.loc_map.get(iid) {
            match &inst.kind[..] {
                "SB_LUT4" => {
                    let tcls = edev.chip.kind.tile_class_plb();
                    let crd = CellCoord::new(
                        DieId::from_idx(0),
                        pkg_info.xlat_col[loc.loc.x as usize],
                        pkg_info.xlat_row[loc.loc.y as usize],
                    );
                    let btile = BitOwner::Main(crd.col, crd.row);
                    let lc = loc.loc.bel;
                    if let Some(lut_init) = inst.props.get("LUT_INIT")
                        && lut_init != "16'h0000"
                    {
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
                            if let Some(src) = int_source
                                .get(&crd.wire(edev.db.get_wire(&format!("IMUX.LC{lc}.I{idx}"))))
                            {
                                pin_to_orig[src]
                            } else if idx == 3 {
                                let InstPinSource::FromInst(_cid, cpin) =
                                    &inst.pins[&InstPin::Simple("I3".into())]
                                else {
                                    unreachable!();
                                };
                                assert_eq!(*cpin, InstPin::Simple("CO".into()));
                                sample.add_tiled_pattern(
                                    &[btile],
                                    format!("{tcls}:INT:MUX.IMUX.LC{lc}.I3:CI"),
                                );
                                if lc == 0 {
                                    sample.add_tiled_pattern(
                                        &[btile],
                                        format!("{tcls}:LC{lc}:MUX.CI:CHAIN"),
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
                                    &[btile],
                                    format!("{tcls}:LC{lc}:LUT_INIT:BIT{i}"),
                                );
                            }
                        }
                    }
                }
                "SB_CARRY" => {
                    let tcls = edev.chip.kind.tile_class_plb();
                    let col = pkg_info.xlat_col[loc.loc.x as usize];
                    let row = pkg_info.xlat_row[loc.loc.y as usize];
                    let lc = loc.loc.bel;
                    if lc == 0 {
                        let ci = &inst.pins[&InstPin::Simple("CI".into())];
                        if matches!(ci, InstPinSource::Gnd) {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                format!("{tcls}:LC{lc}:MUX.CI:0"),
                            );
                        } else if matches!(ci, InstPinSource::Vcc) {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                format!("{tcls}:LC{lc}:MUX.CI:1"),
                            );
                        } else {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                format!("{tcls}:LC{lc}:MUX.CI:CHAIN"),
                            );
                        }
                    }
                    sample.add_tiled_pattern_single(
                        &[BitOwner::Main(col, row)],
                        format!("{tcls}:LC{lc}:CARRY_ENABLE:BIT0"),
                    );
                }
                "SB_IO" | "SB_IO_DS" | "SB_GB_IO" | "SB_IO_OD" | "SB_IO_I3C" => {
                    let io = pkg_info.xlat_io[&(loc.loc.x, loc.loc.y, loc.loc.bel)];
                    let bel = edev.chip.get_io_loc(io);
                    let btile = BitOwner::Main(bel.col, bel.row);
                    let iob = io.iob();
                    let slot_name = edev.db.bel_slots.key(bel.slot).as_str();
                    let tcls_ioi = edev.chip.kind.tile_class_ioi(io.edge()).unwrap();
                    let tcls_iob = edev.chip.kind.tile_class_iob(io.edge()).unwrap();
                    let mut global_idx = None;
                    for (&key, special) in &edev.chip.special_tiles {
                        if let SpecialTileKey::GbIo(idx) = key
                            && special.io[&SpecialIoKey::GbIn] == io
                        {
                            global_idx = Some(idx);
                        }
                    }

                    let iostd = inst.props.get("IO_STANDARD").map(|x| x.as_str());
                    let is_lvds = matches!(iostd, Some("SB_LVDS_INPUT" | "SB_SUBLVDS_INPUT"));

                    if let Some(pin_type) = inst.props.get("PIN_TYPE") {
                        let mut value = BitVec::new();
                        for (i, c) in pin_type.chars().rev().enumerate() {
                            if i >= 6 {
                                break;
                            }
                            assert!(c == '0' || c == '1');
                            value.push(c == '1');
                            if c == '1' {
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    format!("{tcls_ioi}:{slot_name}:PIN_TYPE:BIT{i}"),
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
                                &[btile],
                                format!("{tcls_ioi}:{slot_name}:OUTPUT_ENABLE:BIT0"),
                            );
                            if is_lvds {
                                let oiob = TileIobId::from_idx(iob.to_idx() ^ 1);
                                sample.add_tiled_pattern(
                                    &[btile],
                                    format!("{tcls_ioi}:IO{oiob:#}:OUTPUT_ENABLE:BIT0"),
                                );
                            }
                        }
                        if value[1]
                            && inst.kind == "SB_GB_IO"
                            && edev.chip.kind.has_latch_global_out()
                        {
                            let global_idx = global_idx.unwrap();
                            let mut handled = false;
                            if edev.chip.kind != ChipKind::Ice40P01
                                && let Some((side, ab)) = match global_idx {
                                    6 => Some((DirV::S, 'A')),
                                    3 => Some((DirV::S, 'B')),
                                    7 => Some((DirV::N, 'A')),
                                    2 => Some((DirV::N, 'B')),
                                    _ => None,
                                }
                            {
                                for key in
                                    [SpecialTileKey::Pll(side), SpecialTileKey::PllStub(side)]
                                {
                                    if let Some(special) = edev.chip.special_tiles.get(&key) {
                                        let tcid = key.tile_class(edev.chip.kind);
                                        let tiles = if edev.chip.kind.is_ice65() {
                                            vec![BitOwner::Pll(0), BitOwner::Pll(1)]
                                        } else {
                                            Vec::from_iter(
                                                special
                                                    .cells
                                                    .values()
                                                    .map(|&crd| BitOwner::Main(crd.col, crd.row)),
                                            )
                                        };
                                        sample.add_tiled_pattern(
                                            &tiles,
                                            format!("{tcid}:PLL:LATCH_GLOBAL_OUT_{ab}:BIT0"),
                                        );
                                        handled = true;
                                    }
                                }
                            }
                            if !handled {
                                sample.add_tiled_pattern(
                                    &[btile],
                                    format!("{tcls_iob}:IOB:LATCH_GLOBAL_OUT:BIT0"),
                                );
                            }
                        }
                    }
                    if let Some(neg_trigger) = inst.props.get("NEG_TRIGGER")
                        && neg_trigger.ends_with('1')
                    {
                        sample.add_tiled_pattern(
                            &[btile],
                            format!("{tcls_ioi}:INT:INV.IMUX.IO.ICLK.OPTINV:BIT0"),
                        );
                        sample.add_tiled_pattern(
                            &[btile],
                            format!("{tcls_ioi}:INT:INV.IMUX.IO.OCLK.OPTINV:BIT0"),
                        );
                    }

                    if inst.kind == "SB_IO_I3C" {
                        let weak_pullup = &inst.props["WEAK_PULLUP"];
                        if weak_pullup.ends_with("0") {
                            sample.add_tiled_pattern_single(
                                &[btile],
                                format!("{tcls_iob}:IOB{iob:#}:WEAK_PULLUP:DISABLE"),
                            );
                        }
                        let pullup = &inst.props["PULLUP"];
                        if pullup.ends_with("1") {
                            let pullup_kind = &inst.props["PULLUP_RESISTOR"];
                            sample.add_tiled_pattern_single(
                                &[btile],
                                format!("{tcls_iob}:IOB{iob:#}:PULLUP:{pullup_kind}"),
                            );
                        } else {
                            sample.add_tiled_pattern_single(
                                &[btile],
                                format!("{tcls_iob}:IOB{iob:#}:PULLUP:DISABLE"),
                            );
                        }
                    } else {
                        let pullup = match inst.props.get("PULLUP") {
                            None => false,
                            Some(val) => val.ends_with('1') && !is_lvds,
                        };
                        if edev.chip.kind.has_multi_pullup() {
                            if !pullup {
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    format!("{tcls_iob}:IOB{iob:#}:PULLUP:DISABLE"),
                                );
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    format!("{tcls_iob}:IOB{iob:#}:WEAK_PULLUP:DISABLE"),
                                );
                            } else if let Some(pullup_kind) = inst.props.get("PULLUP_RESISTOR")
                                && pullup_kind != "100K"
                            {
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    format!("{tcls_iob}:IOB{iob:#}:WEAK_PULLUP:DISABLE"),
                                );
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    format!("{tcls_iob}:IOB{iob:#}:PULLUP:{pullup_kind}"),
                                );
                            }
                        } else if edev.chip.kind != ChipKind::Ice40P01 {
                            if !pullup && !(io.edge() == Dir::W && edev.chip.kind.has_vref()) {
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    format!("{tcls_iob}:IOB{iob:#}:PULLUP:DISABLE"),
                                );
                            }
                        } else {
                            if !pullup {
                                sample.add_global_pattern(format!("{io}:PULLUP:DISABLE"));
                            }
                        }
                    }
                    if is_lvds && !edev.chip.kind.has_vref() {
                        sample.add_tiled_pattern_single(
                            &[btile],
                            format!("{tcls_iob}:IOB{iob:#}:LVDS_INPUT:BIT0"),
                        );
                        let oiob = TileIobId::from_idx(iob.to_idx() ^ 1);
                        let oio = io.with_iob(oiob);
                        if edev.chip.kind == ChipKind::Ice40P01 {
                            sample.add_global_pattern_single(format!("{oio}:PULLUP:DISABLE"));
                        } else {
                            sample.add_tiled_pattern_single(
                                &[btile],
                                format!("{tcls_iob}:IOB{oiob:#}:PULLUP:DISABLE"),
                            );
                            if edev.chip.kind.has_multi_pullup() {
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    format!("{tcls_iob}:IOB{oiob:#}:WEAK_PULLUP:DISABLE"),
                                );
                            }
                        }
                    }
                    if matches!(io, EdgeIoCoord::W(..))
                        && edev.chip.kind.has_vref()
                        && let Some(iostd) = iostd
                    {
                        sample.add_tiled_pattern(
                            &[btile],
                            format!("{tcls_iob}:IOB{iob:#}:IOSTD:{iostd}"),
                        );
                    }

                    if ((edev.chip.kind.is_ice40() && !is_lvds)
                        || (edev.chip.kind.has_vref() && matches!(io, EdgeIoCoord::W(..))))
                        && ibuf_used.contains(&iid)
                    {
                        if edev.chip.kind == ChipKind::Ice40P01 {
                            sample.add_global_pattern_single(format!("{io}:IBUF_ENABLE:BIT0"));
                        } else {
                            sample.add_tiled_pattern_single(
                                &[btile],
                                format!("{tcls_iob}:IOB{iob:#}:IBUF_ENABLE:BIT0"),
                            );
                        }
                    }
                }
                kind if kind.starts_with("SB_DFF") => {
                    let tcls = edev.chip.kind.tile_class_plb();
                    let col = pkg_info.xlat_col[loc.loc.x as usize];
                    let row = pkg_info.xlat_row[loc.loc.y as usize];
                    let lc = loc.loc.bel;
                    let mut kind = kind.strip_prefix("SB_DFF").unwrap();
                    sample.add_tiled_pattern_single(
                        &[BitOwner::Main(col, row)],
                        format!("{tcls}:LC{lc}:FF_ENABLE:BIT0"),
                    );
                    if let Some(rest) = kind.strip_prefix('N') {
                        sample.add_tiled_pattern_single(
                            &[BitOwner::Main(col, row)],
                            format!("{tcls}:INT:INV.IMUX.CLK.OPTINV:BIT0"),
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
                                format!("{tcls}:LC{lc}:FF_SR_VALUE:BIT0"),
                            );
                        }
                        "R" => {
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                format!("{tcls}:LC{lc}:FF_SR_ASYNC:BIT0"),
                            );
                        }
                        "S" => {
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                format!("{tcls}:LC{lc}:FF_SR_VALUE:BIT0"),
                            );
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                format!("{tcls}:LC{lc}:FF_SR_ASYNC:BIT0"),
                            );
                        }
                        "" => (),
                        _ => unreachable!(),
                    }
                }
                kind if kind.starts_with("SB_RAM") => {
                    let crd = CellCoord::new(
                        DieId::from_idx(0),
                        pkg_info.xlat_col[loc.loc.x as usize],
                        pkg_info.xlat_row[loc.loc.y as usize],
                    );
                    let bel = crd.bel(bels::BRAM);
                    let btiles = [
                        BitOwner::Main(crd.col, crd.row),
                        BitOwner::Main(crd.col, crd.row + 1),
                    ];
                    for (key, pin, pinn) in [("NW", "WCLK", "WCLKN"), ("NR", "RCLK", "RCLKN")] {
                        let mut wire = edev.get_bel_pin(bel, pin)[0];
                        wire.slot = unoptinv[&wire.slot];
                        if kind.contains(key) {
                            let pin = InstPin::Simple(pinn.into());
                            if inst.pins.contains_key(&pin) {
                                let src = int_source[&wire].clone();
                                assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                            }
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(wire.cell.col, wire.cell.row)],
                                "INT_BRAM:INT:INV.IMUX.CLK.OPTINV:BIT0",
                            );
                        } else {
                            let pin = InstPin::Simple(pin.into());
                            if inst.pins.contains_key(&pin) {
                                let src = int_source[&wire].clone();
                                assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                            }
                        }
                    }
                    for pin in ["WE", "RE", "WCLKE", "RCLKE"] {
                        let wire = edev.get_bel_pin(bel, pin)[0];
                        let pin = InstPin::Simple(pin.into());
                        if inst.pins.contains_key(&pin) {
                            let src = int_source[&wire].clone();
                            assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                        }
                    }
                    let abits = if edev.chip.kind.is_ice40() { 11 } else { 8 };
                    for pin in ["WADDR", "RADDR"] {
                        for idx in 0..abits {
                            let wire = edev.get_bel_pin(bel, &format!("{pin}{idx}"))[0];
                            let pin = InstPin::Indexed(pin.into(), idx);
                            if inst.pins.contains_key(&pin) {
                                let Some(src) = int_source.get(&wire) else {
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
                            let wire = edev.get_bel_pin(bel, &format!("{pin}{idx}"))[0];
                            let pin = InstPin::Indexed(pin.into(), idx);
                            if inst.pins.contains_key(&pin) {
                                let Some(src) = int_source.get(&wire) else {
                                    // avoid unconnected output etc. problems
                                    continue;
                                };
                                let src = src.clone();
                                assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                            }
                        }
                    }

                    let tcls = edev.chip.kind.tile_class_bram();
                    if design.kind.is_ice40() {
                        sample
                            .add_tiled_pattern_single(&btiles, format!("{tcls}:BRAM:ENABLE:BIT0"));
                    }
                    if let Some(read_mode) = inst.props.get("READ_MODE") {
                        sample.add_tiled_pattern(
                            &btiles,
                            format!("{tcls}:BRAM:READ_MODE:{read_mode}"),
                        );
                    }
                    if let Some(write_mode) = inst.props.get("WRITE_MODE") {
                        sample.add_tiled_pattern(
                            &btiles,
                            format!("{tcls}:BRAM:WRITE_MODE:{write_mode}"),
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
                                            &[BitOwner::Bram(bel.col, bel.row)],
                                            format!("BRAM_DATA:BRAM:INIT:BIT{bit}"),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                kind if kind.starts_with("SB_PLL") => {
                    let side = if loc.loc.y == 0 { DirV::S } else { DirV::N };
                    let special = &edev.chip.special_tiles[&SpecialTileKey::Pll(side)];
                    let io_a = special.io[&SpecialIoKey::PllA];
                    let io_b = special.io[&SpecialIoKey::PllB];
                    let crd_a = edev.chip.get_io_loc(io_a).cell;
                    let crd_b = edev.chip.get_io_loc(io_b).cell;
                    let tiles = if edev.chip.kind.is_ice65() {
                        vec![BitOwner::Pll(0), BitOwner::Pll(1)]
                    } else {
                        Vec::from_iter(
                            special
                                .cells
                                .values()
                                .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                        )
                    };
                    let tiles_io_a = [BitOwner::Main(crd_a.col, crd_a.row)];
                    let tiles_io_b = [BitOwner::Main(crd_b.col, crd_b.row)];
                    let tcls_pll = SpecialTileKey::Pll(side).tile_class(edev.chip.kind);
                    sample.add_tiled_pattern(&tiles, format!("{tcls_pll}:PLL:MODE:{kind}"));
                    let tcls_ioi = edev.chip.kind.tile_class_ioi(Dir::V(side)).unwrap();
                    let tcls_iob = edev.chip.kind.tile_class_iob(Dir::V(side)).unwrap();
                    sample.add_tiled_pattern(&tiles_io_a, format!("{tcls_ioi}:IO1:PIN_TYPE:BIT0"));
                    if edev.chip.kind == ChipKind::Ice40P01 {
                        if kind.ends_with("_PAD") {
                            sample.add_global_pattern_single(format!("{io_a}:PULLUP:DISABLE"));
                        }
                        sample.add_global_pattern(format!("{io_a}:IBUF_ENABLE:BIT0"));
                    } else if kind.ends_with("_PAD") && edev.chip.kind.is_ice40() {
                        sample.add_tiled_pattern_single(
                            &tiles_io_a,
                            format!("{tcls_iob}:IOB{iob_a:#}:PULLUP:DISABLE", iob_a = io_a.iob()),
                        );
                        sample.add_tiled_pattern_single(
                            &tiles_io_a,
                            format!(
                                "{tcls_iob}:IOB{iob_a:#}:IBUF_ENABLE:BIT0",
                                iob_a = io_a.iob()
                            ),
                        );
                    }
                    if edev.chip.kind.is_ultra() {
                        sample.add_tiled_pattern(
                            &tiles_io_a,
                            format!("{tcls_ioi}:IO1:OUTPUT_ENABLE:BIT0"),
                        );
                    }
                    if matches!(
                        kind,
                        "SB_PLL_2_PAD" | "SB_PLL40_2_PAD" | "SB_PLL40_2F_CORE" | "SB_PLL40_2F_PAD"
                    ) {
                        sample.add_tiled_pattern(
                            &tiles_io_b,
                            format!("{tcls_ioi}:IO0:PIN_TYPE:BIT0"),
                        );
                    }
                    for (prop, val) in &inst.props {
                        let mut prop = prop.as_str();
                        if matches!(prop, "ENABLE_ICEGATE" | "ENABLE_ICEGATE_PORTA") {
                            if val == "1" {
                                sample.add_tiled_pattern(
                                    &tiles_io_a,
                                    format!("{tcls_ioi}:IO1:PIN_TYPE:BIT1"),
                                );
                                if edev.chip.kind == ChipKind::Ice40P01 {
                                    sample.add_tiled_pattern(
                                        &tiles_io_a,
                                        format!("{tcls_iob}:IOB:LATCH_GLOBAL_OUT:BIT0"),
                                    );
                                } else {
                                    sample.add_tiled_pattern(
                                        &tiles,
                                        format!("{tcls_pll}:PLL:LATCH_GLOBAL_OUT_A:BIT0"),
                                    );
                                }
                            }
                            continue;
                        }
                        if prop == "ENABLE_ICEGATE_PORTB" {
                            if val == "1" {
                                sample.add_tiled_pattern(
                                    &tiles_io_b,
                                    format!("{tcls_ioi}:IO0:PIN_TYPE:BIT1"),
                                );
                                if edev.chip.kind == ChipKind::Ice40P01 {
                                    sample.add_tiled_pattern(
                                        &tiles_io_b,
                                        format!("{tcls_iob}:IOB:LATCH_GLOBAL_OUT:BIT0"),
                                    );
                                } else {
                                    sample.add_tiled_pattern(
                                        &tiles,
                                        format!("{tcls_pll}:PLL:LATCH_GLOBAL_OUT_B:BIT0"),
                                    );
                                }
                            }
                            continue;
                        }
                        if prop == "PLLOUT_SELECT" {
                            prop = "PLLOUT_SELECT_PORTA";
                        }
                        if matches!(prop, "PLLOUT_SELECT_PORTA" | "PLLOUT_SELECT_PORTB")
                            && val == "GENCLK"
                        {
                            continue;
                        }
                        if (prop == "FDA_FEEDBACK"
                            && inst.props["DELAY_ADJUSTMENT_MODE_FEEDBACK"] == "DYNAMIC")
                            || (prop == "FDA_RELATIVE"
                                && inst.props["DELAY_ADJUSTMENT_MODE_RELATIVE"] == "DYNAMIC")
                            || (prop == "FIXED_DELAY_ADJUSTMENT"
                                && inst.props["DELAY_ADJUSTMENT_MODE"] == "DYNAMIC")
                        {
                            for i in 0..4 {
                                sample.add_tiled_pattern_single(
                                    &tiles,
                                    format!("{tcls_pll}:PLL:{prop}:BIT{i}"),
                                );
                            }
                            continue;
                        }
                        if matches!(
                            prop,
                            "DIVR"
                                | "DIVF"
                                | "DIVQ"
                                | "FILTER_RANGE"
                                | "TEST_MODE"
                                | "SHIFTREG_DIV_MODE"
                                | "FDA_FEEDBACK"
                                | "FDA_RELATIVE"
                                | "FIXED_DELAY_ADJUSTMENT"
                        ) {
                            for (i, c) in val.chars().rev().enumerate() {
                                assert!(c == '0' || c == '1');
                                if prop == "SHIFTREG_DIV_MODE" && i == 1 {
                                    continue;
                                }
                                if c == '1' {
                                    sample.add_tiled_pattern_single(
                                        &tiles,
                                        format!("{tcls_pll}:PLL:{prop}:BIT{i}"),
                                    );
                                }
                            }
                        } else {
                            sample
                                .add_tiled_pattern(&tiles, format!("{tcls_pll}:PLL:{prop}:{val}"));
                        }
                    }
                }
                "SB_MAC16" => {
                    let col = pkg_info.xlat_col[loc.loc.x as usize];
                    let row = pkg_info.xlat_row[loc.loc.y as usize];
                    let mut key = SpecialTileKey::Mac16(col, row);
                    if !edev.chip.special_tiles.contains_key(&key) {
                        key = SpecialTileKey::Mac16Trim(col, row);
                    }
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&key]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let nk = key.tile_class(edev.chip.kind);
                    let tcls_plb = edev.chip.kind.tile_class_plb();
                    for i in 0..4 {
                        for j in 0..8 {
                            for k in [4, 5, 6, 7, 12, 13, 14, 15] {
                                sample.add_tiled_pattern(
                                    &tiles[i..i + 1],
                                    format!("{tcls_plb}:LC{j}:LUT_INIT:BIT{k}"),
                                );
                            }
                            sample.add_tiled_pattern(
                                &tiles[i..i + 1],
                                format!("{tcls_plb}:LC{j}:MUX.I2:LTIN"),
                            );
                        }
                    }
                    if matches!(
                        (edev.chip.kind, col.to_idx(), row.to_idx()),
                        (ChipKind::Ice40T04, _, 5) | (ChipKind::Ice40T05, 25, 10)
                    ) {
                        for k in [4, 5, 6, 7, 12, 13, 14, 15] {
                            sample.add_tiled_pattern(
                                &tiles[4..5],
                                format!("{tcls_plb}:LC0:LUT_INIT:BIT{k}"),
                            );
                        }
                        sample
                            .add_tiled_pattern(&tiles[4..5], format!("{tcls_plb}:LC0:MUX.I2:LTIN"));
                    }
                    for (prop, val) in &inst.props {
                        for (i, c) in val.chars().rev().enumerate() {
                            assert!(c == '0' || c == '1');
                            if c == '1' {
                                if prop == "NEG_TRIGGER" {
                                    sample.add_tiled_pattern_single(
                                        &tiles[2..3],
                                        format!("{tcls_plb}:INT:INV.IMUX.CLK.OPTINV:BIT0"),
                                    );
                                } else {
                                    sample.add_tiled_pattern_single(
                                        &tiles,
                                        format!("{nk}:MAC16:{prop}:BIT{i}"),
                                    );
                                }
                            }
                        }
                    }
                }
                "SB_HFOSC" => {
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Trim]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let tcls_trim = SpecialTileKey::Trim.tile_class(edev.chip.kind);
                    if let Some(val) = design.props.get("VPP_2V5_TO_1P8V")
                        && val == "1"
                    {
                        sample.add_tiled_pattern(
                            &tiles,
                            format!("{tcls_trim}:HFOSC:TRIM_FABRIC:BIT0"),
                        );
                    }
                    let clkhf_div = &inst.props["CLKHF_DIV"];
                    for (i, c) in clkhf_div.chars().rev().enumerate() {
                        if i >= 2 {
                            break;
                        }
                        assert!(c == '0' || c == '1');
                        if c == '1' {
                            sample.add_tiled_pattern(
                                &tiles,
                                format!("{tcls_trim}:HFOSC:CLKHF_DIV:BIT{i}"),
                            );
                        }
                    }
                }
                "SB_LFOSC" => {
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Trim]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let tcls_trim = SpecialTileKey::Trim.tile_class(edev.chip.kind);
                    if let Some(val) = design.props.get("VPP_2V5_TO_1P8V")
                        && val == "1"
                    {
                        sample.add_tiled_pattern(
                            &tiles,
                            format!("{tcls_trim}:LFOSC:TRIM_FABRIC:BIT0"),
                        );
                    }
                }
                "SB_LED_DRV_CUR" => {
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::LedDrvCur]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    sample.add_tiled_pattern_single(
                        &tiles,
                        "LED_DRV_CUR_T04:LED_DRV_CUR:ENABLE:BIT0",
                    );
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Trim]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let tcls_trim = SpecialTileKey::Trim.tile_class(edev.chip.kind);
                    if let Some(val) = design.props.get("VPP_2V5_TO_1P8V")
                        && val == "1"
                    {
                        sample.add_tiled_pattern_single(
                            &tiles,
                            format!("{tcls_trim}:LED_DRV_CUR:TRIM_FABRIC:BIT0"),
                        );
                    }
                }
                "SB_RGB_DRV" | "SB_RGBA_DRV" => {
                    let tcls_rgb_drv = SpecialTileKey::RgbDrv.tile_class(edev.chip.kind);
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::RgbDrv]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let mut got_any = false;
                    for prop in ["RGB0_CURRENT", "RGB1_CURRENT", "RGB2_CURRENT"] {
                        let val = &inst.props[prop];
                        for (i, c) in val.chars().rev().enumerate() {
                            if i >= 6 {
                                break;
                            }
                            assert!(c == '0' || c == '1');
                            if c == '1' {
                                got_any = true;
                                sample.add_tiled_pattern_single(
                                    &tiles,
                                    format!("{tcls_rgb_drv}:RGB_DRV:{prop}:BIT{i}"),
                                );
                            }
                        }
                    }
                    if inst.kind == "SB_RGBA_DRV" {
                        has_led_v2 = true;
                        if inst.props["CURRENT_MODE"] == "0b1" {
                            sample.add_tiled_pattern_single(
                                &tiles,
                                format!("{tcls_rgb_drv}:RGB_DRV:CURRENT_MODE:BIT0"),
                            );
                        }
                        sample.add_tiled_pattern_single(
                            &tiles,
                            format!("{tcls_rgb_drv}:RGB_DRV:ENABLE:BIT0"),
                        );
                        if edev.chip.kind == ChipKind::Ice40T01 {
                            let tiles = Vec::from_iter(
                                edev.chip.special_tiles[&SpecialTileKey::Ir500Drv]
                                    .cells
                                    .values()
                                    .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                            );
                            sample
                                .add_tiled_pattern_single(&tiles, "IR500_DRV:RGB_DRV:ENABLE:BIT0");
                        }
                    } else {
                        if got_any {
                            sample.add_tiled_pattern_single(
                                &tiles,
                                format!("{tcls_rgb_drv}:RGB_DRV:ENABLE:BIT0"),
                            );
                        }
                    }
                }
                "SB_IR_DRV" => {
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::IrDrv]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let val = &inst.props["IR_CURRENT"];
                    for (i, c) in val.chars().rev().enumerate() {
                        if i >= 10 {
                            break;
                        }
                        assert!(c == '0' || c == '1');
                        if c == '1' {
                            if i == 0 {
                                sample.add_tiled_pattern(
                                    &tiles,
                                    format!("IR_DRV:IR_DRV:IR_CURRENT:BIT{i}"),
                                );
                            } else {
                                sample.add_tiled_pattern_single(
                                    &tiles,
                                    format!("IR_DRV:IR_DRV:IR_CURRENT:BIT{i}"),
                                );
                            }
                        }
                    }
                }
                "SB_IR500_DRV" => {
                    has_led_v2 = true;
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Ir500Drv]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let val = &inst.props["IR500_CURRENT"];
                    for (i, c) in val.chars().rev().enumerate() {
                        if i >= 12 {
                            break;
                        }
                        assert!(c == '0' || c == '1');
                        if c == '1' {
                            if i < 4 {
                                sample.add_tiled_pattern_single(
                                    &tiles,
                                    format!("IR500_DRV:BARCODE_DRV:BARCODE_CURRENT:BIT{i}"),
                                );
                            } else {
                                sample.add_tiled_pattern_single(
                                    &tiles,
                                    format!(
                                        "IR500_DRV:IR400_DRV:IR400_CURRENT:BIT{ii}",
                                        ii = i - 4
                                    ),
                                );
                            }
                        }
                    }
                    sample.add_tiled_pattern_single(&tiles, "IR500_DRV:BARCODE_DRV:ENABLE:BIT0");
                    sample.add_tiled_pattern_single(&tiles, "IR500_DRV:IR400_DRV:ENABLE:BIT0");
                    sample.add_tiled_pattern_single(&tiles, "IR500_DRV:IR500_DRV:ENABLE:BIT0");
                    if inst.props["CURRENT_MODE"] == "0b1" {
                        led_v2_current_mode = true;
                    }
                }
                "SB_IR400_DRV" => {
                    has_led_v2 = true;
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Ir500Drv]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let val = &inst.props["IR400_CURRENT"];
                    for (i, c) in val.chars().rev().enumerate() {
                        if i >= 8 {
                            break;
                        }
                        assert!(c == '0' || c == '1');
                        if c == '1' {
                            sample.add_tiled_pattern_single(
                                &tiles,
                                format!("IR500_DRV:IR400_DRV:IR400_CURRENT:BIT{i}"),
                            );
                        }
                    }
                    sample.add_tiled_pattern_single(&tiles, "IR500_DRV:IR400_DRV:ENABLE:BIT0");
                    if inst.props["CURRENT_MODE"] == "0b1" {
                        led_v2_current_mode = true;
                    }
                }
                "SB_BARCODE_DRV" => {
                    has_led_v2 = true;
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Ir500Drv]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let val = &inst.props["BARCODE_CURRENT"];
                    for (i, c) in val.chars().rev().enumerate() {
                        if i >= 4 {
                            break;
                        }
                        assert!(c == '0' || c == '1');
                        if c == '1' {
                            sample.add_tiled_pattern_single(
                                &tiles,
                                format!("IR500_DRV:BARCODE_DRV:BARCODE_CURRENT:BIT{i}"),
                            );
                        }
                    }
                    sample.add_tiled_pattern_single(&tiles, "IR500_DRV:BARCODE_DRV:ENABLE:BIT0");
                    if inst.props["CURRENT_MODE"] == "0b1" {
                        led_v2_current_mode = true;
                    }
                }
                "SB_SPRAM256KA" => {
                    for key in [
                        SpecialTileKey::SpramPair(DirH::W),
                        SpecialTileKey::SpramPair(DirH::E),
                    ] {
                        let Some(sprams) = special_tiles.get(&key) else {
                            continue;
                        };
                        for (i, &sloc) in sprams.iter().enumerate() {
                            if loc.loc == sloc {
                                let tiles = Vec::from_iter(
                                    edev.chip.special_tiles[&key]
                                        .cells
                                        .values()
                                        .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                                );
                                sample.add_tiled_pattern(
                                    &tiles,
                                    format!("SPRAM:SPRAM{i}:ENABLE:BIT0"),
                                );
                            }
                        }
                    }
                }
                "SB_FILTER_50NS" => {
                    let filters = &special_tiles.get(&SpecialTileKey::I3c).unwrap()[..2];
                    for (i, &sloc) in filters.iter().enumerate() {
                        if loc.loc == sloc {
                            let tiles = Vec::from_iter(
                                edev.chip.special_tiles[&SpecialTileKey::I3c]
                                    .cells
                                    .values()
                                    .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                            );
                            sample.add_tiled_pattern(&tiles, format!("I3C:FILTER{i}:ENABLE:BIT0"));
                        }
                    }
                }
                "SB_SPI" | "SB_I2C" | "SB_I2C_FIFO" => {
                    for key in [
                        SpecialTileKey::Spi(DirH::W),
                        SpecialTileKey::Spi(DirH::E),
                        SpecialTileKey::I2c(DirH::W),
                        SpecialTileKey::I2c(DirH::E),
                        SpecialTileKey::I2cFifo(DirH::W),
                        SpecialTileKey::I2cFifo(DirH::E),
                    ] {
                        let Some(slocs) = special_tiles.get(&key) else {
                            continue;
                        };
                        if loc.loc == slocs[0] {
                            let special = &edev.chip.special_tiles[&key];
                            let dedio = if inst.kind == "SB_SPI" {
                                [
                                    (SpecialIoKey::SpiSck, "SCKO", "SCKOE", Some("SCKI")),
                                    (SpecialIoKey::SpiCopi, "MO", "MOE", Some("SI")),
                                    (SpecialIoKey::SpiCipo, "SO", "SOE", Some("MI")),
                                    (SpecialIoKey::SpiCsB0, "MCSNO0", "MCSNOE0", Some("SCSNI")),
                                    (SpecialIoKey::SpiCsB1, "MCSNO1", "MCSNOE1", None),
                                ]
                                .as_slice()
                            } else {
                                [
                                    (SpecialIoKey::I2cScl, "SCLO", "SCLOE", Some("SCLI")),
                                    (SpecialIoKey::I2cSda, "SDAO", "SDAOE", Some("SDAI")),
                                ]
                                .as_slice()
                            };
                            let mut all_ded_ins = true;
                            let mut all_ded_outs = true;
                            for &(xnio, o, _oe, i) in dedio {
                                let crd = special.io[&xnio];
                                let tcls_iob = edev.chip.kind.tile_class_iob(crd.edge()).unwrap();
                                let iobel = edev.chip.get_io_loc(crd);
                                let iob = crd.iob();
                                let btile_io = BitOwner::Main(iobel.col, iobel.row);
                                let mut ded_in = false;
                                let mut ded_out =
                                    runres.dedio.contains(&(iid, InstPin::Simple(o.into())));
                                if let Some(i) = i
                                    && let Some(&InstPinSource::FromInst(ioiid, ref pin)) =
                                        inst.pins.get(&InstPin::Simple(i.into()))
                                {
                                    ded_in = runres.dedio.contains(&(ioiid, pin.clone()))
                                }
                                if ded_in && edev.chip.kind == ChipKind::Ice40T01 {
                                    ded_out = true;
                                }
                                if ded_out {
                                    if edev.chip.kind == ChipKind::Ice40R04 {
                                        sample.add_tiled_pattern_single(
                                            &[btile_io],
                                            format!("{tcls_iob}:IOB:HARDIP_DEDICATED_OUT:BIT0"),
                                        );
                                    } else {
                                        sample.add_tiled_pattern_single(
                                            &[btile_io],
                                            format!(
                                                "{tcls_iob}:IOB{iob:#}:HARDIP_DEDICATED_OUT:BIT0"
                                            ),
                                        );
                                    }
                                } else {
                                    all_ded_outs = false;
                                }
                                if i.is_some() && !ded_in {
                                    if edev.chip.kind == ChipKind::Ice40R04 {
                                        sample.add_tiled_pattern_single(
                                            &[btile_io],
                                            format!("{tcls_iob}:IOB:HARDIP_FABRIC_IN:BIT0"),
                                        );
                                    } else {
                                        sample.add_tiled_pattern_single(
                                            &[btile_io],
                                            format!("{tcls_iob}:IOB{iob:#}:HARDIP_FABRIC_IN:BIT0"),
                                        );
                                    }
                                    all_ded_ins = false;
                                }
                            }
                            if edev.chip.kind == ChipKind::Ice40R04 {
                                let tcname = key.tile_class(edev.chip.kind);
                                let tcls = edev.db.tile_classes.get(&tcname).unwrap().1;
                                let bslot = match key {
                                    SpecialTileKey::Spi(_) => bels::SPI,
                                    SpecialTileKey::I2c(_) => bels::I2C,
                                    SpecialTileKey::I2cFifo(_) => bels::I2C_FIFO,
                                    _ => unreachable!(),
                                };
                                let BelInfo::Bel(bel) = &tcls.bels[bslot] else {
                                    unreachable!()
                                };
                                for (pin, pin_info) in &bel.pins {
                                    let pin_wire = *pin_info.wires.iter().next().unwrap();
                                    let pin_crd = special.cells[pin_wire.cell];
                                    let pin_btile = BitOwner::Main(pin_crd.col, pin_crd.row);
                                    if all_ded_outs
                                        && matches!(
                                            pin.as_str(),
                                            "SCLO"
                                                | "SCLOE"
                                                | "SDAO"
                                                | "SDAOE"
                                                | "SCKO"
                                                | "SCKOE"
                                                | "MO"
                                                | "MOE"
                                                | "SO"
                                                | "SOE"
                                                | "MCSNO0"
                                                | "MCSNOE0"
                                                | "MCSNO1"
                                                | "MCSNOE1"
                                        )
                                    {
                                        continue;
                                    }
                                    if all_ded_ins
                                        && matches!(
                                            pin.as_str(),
                                            "SCLI" | "SDAI" | "SCKI" | "MI" | "SI" | "SCSNI"
                                        )
                                    {
                                        continue;
                                    }
                                    if pin_info.dir == PinDir::Input {
                                        let iob = match edev.db.wires.key(pin_wire.wire).as_str() {
                                            "IMUX.IO0.DOUT0" => 0,
                                            "IMUX.IO1.DOUT0" => 1,
                                            _ => unreachable!(),
                                        };
                                        let io_tile_kind =
                                            get_main_tile_kind(edev, pin_crd.col, pin_crd.row);
                                        sample.add_tiled_pattern_single(
                                            &[pin_btile],
                                            format!("{io_tile_kind}:IO{iob}:PIN_TYPE:BIT3"),
                                        );
                                        sample.add_tiled_pattern_single(
                                            &[pin_btile],
                                            format!("{io_tile_kind}:IO{iob}:PIN_TYPE:BIT4"),
                                        );
                                    } else {
                                        let iob = match edev.db.wires.key(pin_wire.wire).as_str() {
                                            "OUT.LC0" | "OUT.LC4" => 0,
                                            "OUT.LC2" | "OUT.LC6" => 1,
                                            _ => unreachable!(),
                                        };
                                        let io_tile_kind =
                                            get_main_tile_kind(edev, pin_crd.col, pin_crd.row);
                                        sample.add_tiled_pattern_single(
                                            &[pin_btile],
                                            format!("{io_tile_kind}:IO{iob}:PIN_TYPE:BIT0"),
                                        );
                                    }
                                }
                            }
                            for prop in ["SDA_INPUT_DELAYED", "SDA_OUTPUT_DELAYED"] {
                                if let Some(val) = inst.props.get(prop)
                                    && val == "1"
                                {
                                    let crd = special.io[&SpecialIoKey::I2cSda];
                                    let iobel = edev.chip.get_io_loc(crd);
                                    let iob = crd.iob();
                                    let tcls_iob =
                                        edev.chip.kind.tile_class_iob(crd.edge()).unwrap();
                                    sample.add_tiled_pattern_single(
                                        &[BitOwner::Main(iobel.col, iobel.row)],
                                        format!("{tcls_iob}:IOB{iob:#}:{prop}:BIT0"),
                                    );
                                }
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
    if has_led_v2 {
        if led_v2_current_mode {
            let tiles = Vec::from_iter(
                edev.chip.special_tiles[&SpecialTileKey::Ir500Drv]
                    .cells
                    .values()
                    .map(|&cell| BitOwner::Main(cell.col, cell.row)),
            );
            sample.add_tiled_pattern(&tiles, "IR500_DRV:IR500_DRV:CURRENT_MODE:BIT0");
        }
        let tcls_trim = SpecialTileKey::Trim.tile_class(edev.chip.kind);
        if let Some(val) = design.props.get("VPP_2V5_TO_1P8V")
            && val == "1"
        {
            let tiles = Vec::from_iter(
                edev.chip.special_tiles[&SpecialTileKey::Trim]
                    .cells
                    .values()
                    .map(|&cell| BitOwner::Main(cell.col, cell.row)),
            );
            sample.add_tiled_pattern_single(
                &tiles,
                format!("{tcls_trim}:LED_DRV_CUR:TRIM_FABRIC:BIT0"),
            );
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
    let tile = edev.chip.kind.tile_class_plb();
    for lc in 0..8 {
        if edev.chip.kind.is_ice40() {
            result.push(format!("{tile}:LC{lc}:MUX.I2:LTIN"));
        }
        result.push(format!("{tile}:INT:MUX.IMUX.LC{lc}.I3:CI"));
        for i in 0..16 {
            result.push(format!("{tile}:LC{lc}:LUT_INIT:BIT{i}"));
        }
        result.push(format!("{tile}:LC{lc}:CARRY_ENABLE:BIT0"));
        result.push(format!("{tile}:LC{lc}:FF_ENABLE:BIT0"));
        result.push(format!("{tile}:LC{lc}:FF_SR_VALUE:BIT0"));
        result.push(format!("{tile}:LC{lc}:FF_SR_ASYNC:BIT0"));
    }
    result.push(format!("{tile}:LC0:MUX.CI:0"));
    result.push(format!("{tile}:LC0:MUX.CI:1"));
    result.push(format!("{tile}:LC0:MUX.CI:CHAIN"));
    result.push(format!("{tile}:INT:INV.IMUX.CLK.OPTINV:BIT0"));
    if let Some(tile) = edev.chip.kind.tile_class_colbuf() {
        for i in 0..8 {
            result.push(format!("{tile}:COLBUF:GLOBAL.{i}:BIT0"));
            if edev.chip.kind.has_ioi_we() {
                result.push(format!("COLBUF_IO_W:COLBUF:GLOBAL.{i}:BIT0"));
                result.push(format!("COLBUF_IO_E:COLBUF:GLOBAL.{i}:BIT0"));
            }
        }
    }
    // BRAM
    if !edev.chip.cols_bram.is_empty() {
        let tile = edev.chip.kind.tile_class_bram();
        if edev.chip.kind.is_ice40() {
            result.push(format!("{tile}:BRAM:CASCADE_OUT_WADDR:BIT0"));
            result.push(format!("{tile}:BRAM:CASCADE_OUT_RADDR:BIT0"));
            result.push(format!("{tile}:BRAM:CASCADE_IN_WADDR:BIT0"));
            result.push(format!("{tile}:BRAM:CASCADE_IN_RADDR:BIT0"));
            result.push(format!("{tile}:BRAM:ENABLE:BIT0"));
            result.push(format!("{tile}:BRAM:READ_MODE:0"));
            result.push(format!("{tile}:BRAM:READ_MODE:1"));
            result.push(format!("{tile}:BRAM:READ_MODE:2"));
            result.push(format!("{tile}:BRAM:READ_MODE:3"));
            result.push(format!("{tile}:BRAM:WRITE_MODE:0"));
            result.push(format!("{tile}:BRAM:WRITE_MODE:1"));
            result.push(format!("{tile}:BRAM:WRITE_MODE:2"));
            result.push(format!("{tile}:BRAM:WRITE_MODE:3"));
        }
        result.push("INT_BRAM:INT:INV.IMUX.CLK.OPTINV:BIT0".into());
        for i in 0..4096 {
            result.push(format!("BRAM_DATA:BRAM:INIT:BIT{i}"));
        }
    }
    // IO
    for edge in Dir::DIRS {
        let Some(tile) = edev.chip.kind.tile_class_ioi(edge) else {
            continue;
        };
        result.push(format!("{tile}:INT:INV.IMUX.IO.ICLK.OPTINV:BIT0"));
        result.push(format!("{tile}:INT:INV.IMUX.IO.OCLK.OPTINV:BIT0"));
        for io in 0..2 {
            for i in 0..6 {
                result.push(format!("{tile}:IO{io}:PIN_TYPE:BIT{i}"));
            }
            if edev.chip.kind.is_ultra() {
                result.push(format!("{tile}:IO{io}:OUTPUT_ENABLE:BIT0"));
            }
        }
        let Some(tile) = edev.chip.kind.tile_class_iob(edge) else {
            continue;
        };
        let has_lvds = if edev.chip.kind == ChipKind::Ice65L01 {
            false
        } else if edev.chip.kind.has_iob_we() {
            edge == Dir::W
        } else if edev.chip.kind == ChipKind::Ice40R04 {
            edge == Dir::N
        } else {
            true
        };
        if edev.chip.kind == ChipKind::Ice40P01 {
            continue;
        }
        for iob in 0..2 {
            if edev.chip.kind.is_ice40() || (edge == Dir::W && edev.chip.kind.has_vref()) {
                result.push(format!("{tile}:IOB{iob}:IBUF_ENABLE:BIT0"));
            }
            if edev.chip.kind.is_ultra()
                && !(edge == Dir::N && edev.chip.kind == ChipKind::Ice40T01)
            {
                result.push(format!("{tile}:IOB{iob}:HARDIP_FABRIC_IN:BIT0"));
                result.push(format!("{tile}:IOB{iob}:HARDIP_DEDICATED_OUT:BIT0"));
                if (edev.chip.kind == ChipKind::Ice40T01 && iob == 0) || edge == Dir::N {
                    result.push(format!("{tile}:IOB{iob}:SDA_INPUT_DELAYED:BIT0"));
                    result.push(format!("{tile}:IOB{iob}:SDA_OUTPUT_DELAYED:BIT0"));
                }
            }
            if edge == Dir::W && edev.chip.kind.has_vref() {
                for iostd in [
                    "SB_LVCMOS15_4",
                    "SB_LVCMOS15_2",
                    "SB_LVCMOS18_10",
                    "SB_LVCMOS18_8",
                    "SB_LVCMOS18_4",
                    "SB_LVCMOS18_2",
                    "SB_SSTL18_FULL",
                    "SB_SSTL18_HALF",
                    "SB_MDDR10",
                    "SB_MDDR8",
                    "SB_MDDR4",
                    "SB_MDDR2",
                    "SB_LVCMOS25_16",
                    "SB_LVCMOS25_12",
                    "SB_LVCMOS25_8",
                    "SB_LVCMOS25_4",
                    "SB_SSTL2_CLASS_2",
                    "SB_SSTL2_CLASS_1",
                    "SB_LVCMOS33_8",
                ] {
                    result.push(format!("{tile}:IOB{iob}:IOSTD:{iostd}"));
                }
                if iob == 0 {
                    for iostd in ["SB_LVDS_INPUT", "SB_SUBLVDS_INPUT"] {
                        result.push(format!("{tile}:IOB{iob}:IOSTD:{iostd}"));
                    }
                }
            } else {
                result.push(format!("{tile}:IOB{iob}:PULLUP:DISABLE"));
                if edev.chip.kind.has_multi_pullup() {
                    result.push(format!("{tile}:IOB{iob}:PULLUP:3P3K"));
                    result.push(format!("{tile}:IOB{iob}:PULLUP:6P8K"));
                    result.push(format!("{tile}:IOB{iob}:PULLUP:10K"));
                    result.push(format!("{tile}:IOB{iob}:WEAK_PULLUP:DISABLE"));
                }
                if has_lvds && iob == 0 {
                    result.push(format!("{tile}:IOB{iob}:LVDS_INPUT:BIT0"));
                }
            }
        }
        let mut has_latch_global_out = edev.chip.kind.has_latch_global_out();
        if edge == Dir::S
            && (edev
                .chip
                .special_tiles
                .contains_key(&SpecialTileKey::Pll(DirV::S))
                || edev
                    .chip
                    .special_tiles
                    .contains_key(&SpecialTileKey::PllStub(DirV::S)))
            && edev.chip.kind.has_iob_we()
        {
            has_latch_global_out = false;
        }
        if edge == Dir::N
            && (edev
                .chip
                .special_tiles
                .contains_key(&SpecialTileKey::Pll(DirV::N))
                || edev
                    .chip
                    .special_tiles
                    .contains_key(&SpecialTileKey::PllStub(DirV::N)))
        {
            has_latch_global_out = false;
        }
        if edev.chip.kind == ChipKind::Ice40P01 {
            has_latch_global_out = true;
        }
        if has_latch_global_out {
            result.push(format!("{tile}:IOB:LATCH_GLOBAL_OUT:BIT0"));
        }
        if edev.chip.kind == ChipKind::Ice40R04 {
            result.push(format!("{tile}:IOB:HARDIP_FABRIC_IN:BIT0"));
            result.push(format!("{tile}:IOB:HARDIP_DEDICATED_OUT:BIT0"));
        }
    }
    for side in [DirV::S, DirV::N] {
        let key = SpecialTileKey::Pll(side);
        if edev.chip.special_tiles.contains_key(&key) {
            let tile = key.tile_class(edev.chip.kind);
            if edev.chip.kind.is_ice65() {
                for (attr, vals) in [
                    (
                        "MODE",
                        ["SB_PLL_CORE", "SB_PLL_PAD", "SB_PLL_2_PAD"].as_slice(),
                    ),
                    (
                        "FEEDBACK_PATH",
                        ["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"].as_slice(),
                    ),
                    ("DELAY_ADJUSTMENT_MODE", ["DYNAMIC", "FIXED"].as_slice()),
                    (
                        "PLLOUT_PHASE",
                        ["NONE", "0deg", "90deg", "180deg", "270deg"].as_slice(),
                    ),
                ] {
                    for &val in vals {
                        result.push(format!("{tile}:PLL:{attr}:{val}"));
                    }
                }
                for (attr, width) in [
                    ("FIXED_DELAY_ADJUSTMENT", 4),
                    ("DIVR", 4),
                    ("DIVF", 6),
                    ("DIVQ", 3),
                    ("FILTER_RANGE", 3),
                    ("TEST_MODE", 1),
                    ("LATCH_GLOBAL_OUT_A", 1),
                    ("LATCH_GLOBAL_OUT_B", 1),
                ] {
                    for i in 0..width {
                        result.push(format!("{tile}:PLL:{attr}:BIT{i}"));
                    }
                }
            } else {
                for (attr, vals) in [
                    (
                        "MODE",
                        [
                            "SB_PLL40_CORE",
                            "SB_PLL40_PAD",
                            "SB_PLL40_2_PAD",
                            "SB_PLL40_2F_CORE",
                            "SB_PLL40_2F_PAD",
                        ]
                        .as_slice(),
                    ),
                    (
                        "FEEDBACK_PATH",
                        ["SIMPLE", "DELAY", "PHASE_AND_DELAY", "EXTERNAL"].as_slice(),
                    ),
                    (
                        "DELAY_ADJUSTMENT_MODE_FEEDBACK",
                        ["DYNAMIC", "FIXED"].as_slice(),
                    ),
                    (
                        "DELAY_ADJUSTMENT_MODE_RELATIVE",
                        ["DYNAMIC", "FIXED"].as_slice(),
                    ),
                    (
                        "PLLOUT_SELECT_PORTA",
                        ["GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"].as_slice(),
                    ),
                    (
                        "PLLOUT_SELECT_PORTB",
                        ["GENCLK_HALF", "SHIFTREG_0deg", "SHIFTREG_90deg"].as_slice(),
                    ),
                ] {
                    for &val in vals {
                        result.push(format!("{tile}:PLL:{attr}:{val}"));
                    }
                }
                for (attr, width) in [
                    ("SHIFTREG_DIV_MODE", 1),
                    ("FDA_FEEDBACK", 4),
                    ("FDA_RELATIVE", 4),
                    ("DIVR", 4),
                    ("DIVF", 7),
                    ("DIVQ", 3),
                    ("FILTER_RANGE", 3),
                    ("TEST_MODE", 1),
                    ("LATCH_GLOBAL_OUT_A", 1),
                    ("LATCH_GLOBAL_OUT_B", 1),
                ] {
                    if attr.starts_with("LATCH_GLOBAL_OUT") && edev.chip.kind == ChipKind::Ice40P01
                    {
                        continue;
                    }
                    for i in 0..width {
                        result.push(format!("{tile}:PLL:{attr}:BIT{i}"));
                    }
                }
            }
        }
        let key = SpecialTileKey::PllStub(side);
        let tile = key.tile_class(edev.chip.kind);
        if edev.chip.special_tiles.contains_key(&key) {
            for attr in ["LATCH_GLOBAL_OUT_A", "LATCH_GLOBAL_OUT_B"] {
                result.push(format!("{tile}:PLL:{attr}:BIT0"));
            }
        }
    }
    if edev.chip.kind.is_ultra() {
        // OSC & TRIM
        let tcls_trim = SpecialTileKey::Trim.tile_class(edev.chip.kind);
        result.push(format!("{tcls_trim}:HFOSC:CLKHF_DIV:BIT0"));
        result.push(format!("{tcls_trim}:HFOSC:CLKHF_DIV:BIT1"));
        result.push(format!("{tcls_trim}:HFOSC:TRIM_FABRIC:BIT0"));
        result.push(format!("{tcls_trim}:LFOSC:TRIM_FABRIC:BIT0"));
        result.push(format!("{tcls_trim}:LED_DRV_CUR:TRIM_FABRIC:BIT0"));
        // DRV
        let tcls_rgba_drv = SpecialTileKey::RgbDrv.tile_class(edev.chip.kind);
        result.push(format!("{tcls_rgba_drv}:RGB_DRV:ENABLE:BIT0"));
        for i in 0..3 {
            for j in 0..6 {
                result.push(format!("{tcls_rgba_drv}:RGB_DRV:RGB{i}_CURRENT:BIT{j}"));
            }
        }
        if edev.chip.kind == ChipKind::Ice40T04 {
            result.push("LED_DRV_CUR_T04:LED_DRV_CUR:ENABLE:BIT0".into());
            for j in 0..10 {
                result.push(format!("IR_DRV:IR_DRV:IR_CURRENT:BIT{j}"));
            }
        } else {
            result.push(format!("{tcls_rgba_drv}:RGB_DRV:CURRENT_MODE:BIT0"));
            if edev.chip.kind == ChipKind::Ice40T01 {
                result.push("IR500_DRV:RGB_DRV:ENABLE:BIT0".into());
                result.push("IR500_DRV:IR400_DRV:ENABLE:BIT0".into());
                result.push("IR500_DRV:IR500_DRV:ENABLE:BIT0".into());
                result.push("IR500_DRV:IR500_DRV:CURRENT_MODE:BIT0".into());
                result.push("IR500_DRV:BARCODE_DRV:ENABLE:BIT0".into());
                for j in 0..8 {
                    result.push(format!("IR500_DRV:IR400_DRV:IR400_CURRENT:BIT{j}"));
                }
                for j in 0..4 {
                    result.push(format!("IR500_DRV:BARCODE_DRV:BARCODE_CURRENT:BIT{j}"));
                }
            }
        }
    }
    // MAC16
    if matches!(edev.chip.kind, ChipKind::Ice40T04 | ChipKind::Ice40T05) {
        for tile in ["MAC16", "MAC16_TRIM"] {
            if tile == "MAC16_TRIM" && edev.chip.kind != ChipKind::Ice40T05 {
                continue;
            }
            for (attr, width) in [
                ("A_REG", 1),
                ("B_REG", 1),
                ("C_REG", 1),
                ("D_REG", 1),
                ("TOP_8x8_MULT_REG", 1),
                ("BOT_8x8_MULT_REG", 1),
                ("PIPELINE_16x16_MULT_REG1", 1),
                ("PIPELINE_16x16_MULT_REG2", 1),
                ("TOPOUTPUT_SELECT", 2),
                ("BOTOUTPUT_SELECT", 2),
                ("TOPADDSUB_LOWERINPUT", 2),
                ("BOTADDSUB_LOWERINPUT", 2),
                ("TOPADDSUB_UPPERINPUT", 1),
                ("BOTADDSUB_UPPERINPUT", 1),
                ("TOPADDSUB_CARRYSELECT", 2),
                ("BOTADDSUB_CARRYSELECT", 2),
                ("MODE_8x8", 1),
                ("A_SIGNED", 1),
                ("B_SIGNED", 1),
            ] {
                for i in 0..width {
                    result.push(format!("{tile}:MAC16:{attr}:BIT{i}"));
                }
            }
        }
    }
    // SPRAM
    if edev.chip.kind == ChipKind::Ice40T05 {
        result.push("SPRAM:SPRAM0:ENABLE:BIT0".into());
        result.push("SPRAM:SPRAM1:ENABLE:BIT0".into());
        result.push("I3C:FILTER0:ENABLE:BIT0".into());
        result.push("I3C:FILTER1:ENABLE:BIT0".into());
    }
    // misc
    let tcls_gb_root = edev.chip.kind.tile_class_gb_root();
    for i in 0..8 {
        result.push(format!("{tcls_gb_root}:GB_ROOT:MUX.GLOBAL.{i}:IO"));
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
    if edev.chip.kind == ChipKind::Ice40P01 {
        for &io in edev.chip.io_iob.keys() {
            result.push(format!("{io}:IBUF_ENABLE:BIT0"));
            result.push(format!("{io}:PULLUP:DISABLE"));
        }
    }
    result
}

pub fn get_golden_mux_stats(kind: ChipKind, nkn: &str) -> BTreeMap<String, usize> {
    let mut golden_stats = BTreeMap::new();
    if !nkn.starts_with("IOI") {
        golden_stats.insert("IMUX.CLK".to_string(), 12);
        golden_stats.insert("IMUX.CE".to_string(), 8);
        golden_stats.insert("IMUX.RST".to_string(), 8);
        for lc in 0..8 {
            for i in 0..4 {
                if i == 2 && nkn == "INT_BRAM" {
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
        if nkn.starts_with("IOI_S") | nkn.starts_with("IOI_N") {
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
