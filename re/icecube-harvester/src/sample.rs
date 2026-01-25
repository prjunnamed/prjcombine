use std::collections::{BTreeMap, HashMap, HashSet};

use prjcombine_entity::{EntityBundleIndex, EntityBundleItemIndex, EntityId};
use prjcombine_interconnect::{
    db::{
        BelAttributeType, BelInfo, BelInput, BelKind, SwitchBoxItem, TileClassId, TileWireCoord,
        WireSlotId,
    },
    dir::{Dir, DirH, DirV},
    grid::{CellCoord, ColId, DieId, RowId, WireCoord},
};
use prjcombine_re_fpga_hammer::diff::DiffKey;
use prjcombine_re_harvester::Sample;
use prjcombine_siliconblue::{
    bitstream::Bitstream,
    chip::{ChipKind, SpecialIoKey, SpecialTileKey},
    defs::{self, bslots as bels, tslots},
    expanded::{BitOwner, ExpandedDevice},
};
use prjcombine_types::{bitrect::BitRect as _, bitvec::BitVec};

use crate::{
    PkgInfo,
    run::{Design, InstId, InstPin, InstPinSource, RawLoc, RunResult},
    specials,
    xlat::{GenericNet, xlat_mux_in, xlat_wire},
};

fn get_colbuf_tile_kind(edev: &ExpandedDevice, col: ColId) -> TileClassId {
    if edev.chip.kind.has_ioi_we() && col == edev.chip.col_w() {
        defs::tcls::COLBUF_IO_W
    } else if edev.chip.kind.has_ioi_we() && col == edev.chip.col_e() {
        defs::tcls::COLBUF_IO_E
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
        for (key, bslot, pin) in [
            (
                SpecialTileKey::LsOsc,
                defs::bslots::LSOSC,
                defs::bcls::LSOSC::CLKK,
            ),
            (
                SpecialTileKey::HsOsc,
                defs::bslots::HSOSC,
                defs::bcls::HSOSC::CLKM,
            ),
        ] {
            let crd = *edev.chip.special_tiles[&key].cells.first().unwrap();
            for wire in edev.get_bel_output(crd.bel(bslot), pin) {
                let Some(ioi) = defs::wires::IOB_DIN.index_of(wire.slot) else {
                    unreachable!();
                };
                io_hardip_outs.insert(wire.wire(defs::wires::OUT_LC[2 * ioi]));
                io_hardip_outs.insert(wire.wire(defs::wires::OUT_LC[2 * ioi + 4]));
            }
        }
    }
    let unoptinv: HashMap<_, _> = HashMap::from_iter([
        (defs::wires::IMUX_CLK_OPTINV, defs::wires::IMUX_CLK),
        (defs::wires::IMUX_IO_ICLK_OPTINV, defs::wires::IMUX_IO_ICLK),
        (defs::wires::IMUX_IO_OCLK_OPTINV, defs::wires::IMUX_IO_OCLK),
    ]);
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
                        let tile = &edev[cell.tile(tslots::MAIN)];
                        let wan = edev.db.wires.key(wa);
                        let wbn = edev.db.wires.key(wb);
                        if let Some(idx) = defs::wires::GLOBAL.index_of(wb) {
                            let root = TileWireCoord::new_idx(0, defs::wires::GLOBAL_ROOT[idx]);
                            let tcid = edev.chip.kind.tile_class_gb_root();
                            let tcrd = edev.chip.special_tile(SpecialTileKey::GbRoot);
                            if wa == defs::wires::IMUX_IO_EXTRA {
                                let cslot = edev[tcrd]
                                    .cells
                                    .iter()
                                    .find(|&(_, &c)| c == cell)
                                    .unwrap()
                                    .0;
                                sample.add_tiled_pattern(
                                    &[BitOwner::Clock(0), BitOwner::Clock(1)],
                                    DiffKey::Routing(
                                        tcid,
                                        root,
                                        TileWireCoord {
                                            cell: cslot,
                                            wire: wa,
                                        }
                                        .pos(),
                                    ),
                                );
                            } else {
                                let lc_idx = defs::wires::OUT_LC.index_of(wa).unwrap();
                                let wa_xlat = if edev.chip.kind == ChipKind::Ice40R04 {
                                    match lc_idx {
                                        0 | 4 => defs::wires::IOB_DIN[0],
                                        2 | 6 => defs::wires::IOB_DIN[1],
                                        _ => unreachable!(),
                                    }
                                } else {
                                    defs::wires::LC_LTIN[lc_idx]
                                };

                                let mut found = false;
                                for (key, bslot, bout, src) in [
                                    (
                                        SpecialTileKey::LsOsc,
                                        defs::bslots::LSOSC,
                                        defs::bcls::LSOSC::CLKK,
                                        defs::wires::LSOSC_GLOBAL,
                                    ),
                                    (
                                        SpecialTileKey::HsOsc,
                                        defs::bslots::HSOSC,
                                        defs::bcls::HSOSC::CLKM,
                                        defs::wires::HSOSC_GLOBAL,
                                    ),
                                    (
                                        SpecialTileKey::Misc,
                                        defs::bslots::LFOSC,
                                        defs::bcls::LFOSC::CLKLF,
                                        defs::wires::LSOSC_GLOBAL,
                                    ),
                                    (
                                        SpecialTileKey::Misc,
                                        defs::bslots::HFOSC,
                                        defs::bcls::HFOSC::CLKHF,
                                        defs::wires::HSOSC_GLOBAL,
                                    ),
                                ] {
                                    if !edev.chip.special_tiles.contains_key(&key) {
                                        continue;
                                    }
                                    let tcrd = edev.chip.special_tile(key);
                                    if edev
                                        .get_bel_output(tcrd.bel(bslot), bout)
                                        .contains(&cell.wire(wa_xlat))
                                    {
                                        sample.add_tiled_pattern(
                                            &[BitOwner::Clock(0), BitOwner::Clock(1)],
                                            DiffKey::Routing(
                                                tcid,
                                                root,
                                                TileWireCoord::new_idx(0, src).pos(),
                                            ),
                                        );
                                        found = true;
                                        break;
                                    }
                                }
                                assert!(found);
                            }
                            continue;
                        }
                        pips.insert((tile.class, wb, wa));
                        let key = DiffKey::Routing(
                            tile.class,
                            TileWireCoord::new_idx(0, wb),
                            TileWireCoord::new_idx(0, wa).pos(),
                        );
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
                        if let Some(idx) = defs::wires::GLOBAL.index_of(wa)
                            && edev.chip.kind.tile_class_colbuf().is_some()
                        {
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
                                let tcid = get_colbuf_tile_kind(edev, cell.col);
                                let wt = TileWireCoord::new_idx(0, defs::wires::GLOBAL[idx]);
                                let wf = TileWireCoord::new_idx(0, defs::wires::GLOBAL_ROOT[idx]);
                                sample.add_tiled_pattern(
                                    &[BitOwner::Main(cell.col, trow)],
                                    DiffKey::Routing(tcid, wt, wf.pos()),
                                );
                            } else {
                                sample.add_global_pattern_single(DiffKey::GlobalRouting(
                                    cell.wire(defs::wires::GLOBAL[idx]),
                                    cell.wire(defs::wires::GLOBAL_ROOT[idx]).pos(),
                                ));
                            };
                        }
                        if io_hardip_outs.contains(&iwa) {
                            let crd = iwa.cell;
                            let io = match defs::wires::OUT_LC.index_of(iwa.slot).unwrap() {
                                0 | 4 => 0,
                                2 | 6 => 1,
                                _ => unreachable!(),
                            };
                            let tile = &edev[iwa.cell.tile(defs::tslots::MAIN)];
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(crd.col, crd.row)],
                                DiffKey::BelAttrBit(
                                    tile.class,
                                    defs::bslots::IOI[io],
                                    defs::bcls::IOI::PIN_TYPE,
                                    0,
                                ),
                            );
                        }
                    }
                    (GenericNet::Ltout(cell, lc), GenericNet::Int(iwb)) => {
                        let dst_lc = if lc == 7 {
                            println!("long ltout edge {ax}:{ay}:{aw} -> {bx}:{by}:{bw}");
                            assert_eq!(cell.delta(0, 1), iwb.cell);
                            assert_eq!(iwb.slot, defs::wires::IMUX_LC_I2[0]);
                            0
                        } else {
                            assert_eq!(cell, iwb.cell);
                            assert_eq!(iwb.slot, defs::wires::IMUX_LC_I2[lc + 1]);
                            lc + 1
                        };
                        let tcid = edev.chip.kind.tile_class_plb();
                        sample.add_tiled_pattern(
                            &[BitOwner::Main(iwb.cell.col, iwb.cell.row)],
                            DiffKey::BelAttrBit(
                                tcid,
                                defs::bslots::LC[dst_lc],
                                defs::bcls::LC::LTIN_ENABLE,
                                0,
                            ),
                        );
                        int_source.insert(iwb, (src_inst, InstPin::Simple("O".to_string())));
                    }
                    (GenericNet::Cout(cell, lc), GenericNet::Int(iwb)) => {
                        assert_ne!(lc, 7);
                        assert_eq!(cell, iwb.cell);
                        let dst_lc = lc + 1;
                        assert_eq!(iwb.slot, defs::wires::IMUX_LC_I3[dst_lc]);
                        let tcid = edev.chip.kind.tile_class_plb();
                        pips.insert((tcid, iwb.slot, defs::wires::LC_CI_OUT[dst_lc]));
                        sample.add_tiled_pattern(
                            &[BitOwner::Main(iwb.cell.col, iwb.cell.row)],
                            DiffKey::Routing(
                                tcid,
                                TileWireCoord::new_idx(0, iwb.slot),
                                TileWireCoord::new_idx(0, defs::wires::LC_CI_OUT[dst_lc]).pos(),
                            ),
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
                        let wires = if xi >= 8 {
                            &defs::wires::IMUX_LC_I2
                        } else {
                            &defs::wires::IMUX_LC_I0
                        };
                        assert_eq!(iwa.slot, wires[lc]);
                        let (row, which) = if cell.row.to_idx() % 2 == 1 {
                            (
                                cell.row,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    defs::bcls::BRAM::CASCADE_OUT_RADDR
                                } else {
                                    defs::bcls::BRAM::CASCADE_OUT_WADDR
                                },
                            )
                        } else {
                            (
                                cell.row - 1,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    defs::bcls::BRAM::CASCADE_OUT_WADDR
                                } else {
                                    defs::bcls::BRAM::CASCADE_OUT_RADDR
                                },
                            )
                        };
                        let tiles = [
                            BitOwner::Main(cell.col, row),
                            BitOwner::Main(cell.col, row + 1),
                        ];
                        let tcid = edev.chip.kind.tile_class_bram();
                        sample.add_tiled_pattern(
                            &tiles,
                            DiffKey::BelAttrBit(tcid, defs::bslots::BRAM, which, 0),
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
                        let wires = if xi >= 8 {
                            &defs::wires::IMUX_LC_I2
                        } else {
                            &defs::wires::IMUX_LC_I0
                        };
                        assert_eq!(iwb.slot, wires[lc]);
                        let (row, which) = if cell.row.to_idx() % 2 == 1 {
                            (
                                cell.row - 2,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    defs::bcls::BRAM::CASCADE_IN_RADDR
                                } else {
                                    defs::bcls::BRAM::CASCADE_IN_WADDR
                                },
                            )
                        } else {
                            (
                                cell.row - 3,
                                if edev.chip.kind.has_ice40_bramv2() {
                                    defs::bcls::BRAM::CASCADE_IN_WADDR
                                } else {
                                    defs::bcls::BRAM::CASCADE_IN_RADDR
                                },
                            )
                        };
                        let tiles = [
                            BitOwner::Main(cell.col, row),
                            BitOwner::Main(cell.col, row + 1),
                        ];
                        let tcid = edev.chip.kind.tile_class_bram();
                        sample.add_tiled_pattern(
                            &tiles,
                            DiffKey::BelAttrBit(tcid, defs::bslots::BRAM, which, 0),
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
                        let tcid = edev.chip.kind.tile_class_gb_root();
                        let tcrd = edev.chip.special_tile(SpecialTileKey::GbRoot);
                        let idx = defs::wires::GLOBAL.index_of(iw.slot).unwrap();
                        let root = TileWireCoord::new_idx(0, defs::wires::GLOBAL_ROOT[idx]);
                        let src = match na {
                            GenericNet::GlobalPadIn(cell) => {
                                let cslot = edev[tcrd]
                                    .cells
                                    .iter()
                                    .find(|&(_, &c)| c == cell)
                                    .unwrap()
                                    .0;
                                TileWireCoord {
                                    cell: cslot,
                                    wire: defs::wires::IO_GLOBAL,
                                }
                            }
                            GenericNet::GlobalClkh => {
                                TileWireCoord::new_idx(0, defs::wires::HSOSC_GLOBAL)
                            }
                            GenericNet::GlobalClkl => {
                                TileWireCoord::new_idx(0, defs::wires::LSOSC_GLOBAL)
                            }
                            _ => unreachable!(),
                        };
                        sample.add_tiled_pattern(
                            &[BitOwner::Clock(0), BitOwner::Clock(1)],
                            DiffKey::Routing(tcid, root, src.pos()),
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
                    let tcid = edev.chip.kind.tile_class_plb();
                    let crd = CellCoord::new(
                        DieId::from_idx(0),
                        pkg_info.xlat_col[loc.loc.x as usize],
                        pkg_info.xlat_row[loc.loc.y as usize],
                    );
                    let btile = BitOwner::Main(crd.col, crd.row);
                    let lc = loc.loc.bel as usize;
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
                            if let Some(src) = int_source.get(&crd.wire(
                                [
                                    &defs::wires::IMUX_LC_I0,
                                    &defs::wires::IMUX_LC_I1,
                                    &defs::wires::IMUX_LC_I2,
                                    &defs::wires::IMUX_LC_I3,
                                ][idx][lc],
                            )) {
                                pin_to_orig[src]
                            } else if idx == 3 {
                                let InstPinSource::FromInst(_cid, cpin) =
                                    &inst.pins[&InstPin::Simple("I3".into())]
                                else {
                                    unreachable!();
                                };
                                assert_eq!(*cpin, InstPin::Simple("CO".into()));
                                pips.insert((
                                    tcid,
                                    defs::wires::IMUX_LC_I3[lc],
                                    defs::wires::LC_CI_OUT[lc],
                                ));
                                sample.add_tiled_pattern(
                                    &[btile],
                                    DiffKey::Routing(
                                        tcid,
                                        TileWireCoord::new_idx(0, defs::wires::IMUX_LC_I3[lc]),
                                        TileWireCoord::new_idx(0, defs::wires::LC_CI_OUT[lc]).pos(),
                                    ),
                                );
                                if lc == 0 {
                                    sample.add_tiled_pattern(
                                        &[btile],
                                        DiffKey::BelAttrValue(
                                            tcid,
                                            defs::bslots::LC[lc],
                                            defs::bcls::LC::MUX_CI,
                                            defs::enums::LC_MUX_CI::CHAIN,
                                        ),
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
                                    DiffKey::BelAttrBit(
                                        tcid,
                                        defs::bslots::LC[lc],
                                        defs::bcls::LC::LUT_INIT,
                                        i,
                                    ),
                                );
                            }
                        }
                    }
                }
                "SB_CARRY" => {
                    let tcid = edev.chip.kind.tile_class_plb();
                    let col = pkg_info.xlat_col[loc.loc.x as usize];
                    let row = pkg_info.xlat_row[loc.loc.y as usize];
                    let lc = loc.loc.bel as usize;
                    if lc == 0 {
                        let ci = &inst.pins[&InstPin::Simple("CI".into())];
                        if matches!(ci, InstPinSource::Gnd) {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                DiffKey::BelAttrValue(
                                    tcid,
                                    defs::bslots::LC[lc],
                                    defs::bcls::LC::MUX_CI,
                                    defs::enums::LC_MUX_CI::ZERO,
                                ),
                            );
                        } else if matches!(ci, InstPinSource::Vcc) {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                DiffKey::BelAttrValue(
                                    tcid,
                                    defs::bslots::LC[lc],
                                    defs::bcls::LC::MUX_CI,
                                    defs::enums::LC_MUX_CI::ONE,
                                ),
                            );
                        } else {
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(col, row)],
                                DiffKey::BelAttrValue(
                                    tcid,
                                    defs::bslots::LC[lc],
                                    defs::bcls::LC::MUX_CI,
                                    defs::enums::LC_MUX_CI::CHAIN,
                                ),
                            );
                        }
                    }
                    sample.add_tiled_pattern_single(
                        &[BitOwner::Main(col, row)],
                        DiffKey::BelAttrBit(
                            tcid,
                            defs::bslots::LC[lc],
                            defs::bcls::LC::CARRY_ENABLE,
                            0,
                        ),
                    );
                }
                "SB_IO" | "SB_IO_DS" | "SB_GB_IO" | "SB_IO_OD" | "SB_IO_I3C" => {
                    let ioi = pkg_info.xlat_ioi[&(loc.loc.x, loc.loc.y, loc.loc.bel)];
                    let idx = defs::bslots::IOI.index_of(ioi.slot).unwrap();
                    let iob = edev.chip.ioi_to_iob(ioi).unwrap();
                    let btile = BitOwner::Main(ioi.col, ioi.row);
                    let tcid_ioi = edev
                        .chip
                        .kind
                        .tile_class_ioi(edev.chip.get_io_edge(ioi))
                        .unwrap();
                    let tcid_iob = edev
                        .chip
                        .kind
                        .tile_class_iob(edev.chip.get_io_edge(ioi))
                        .unwrap();
                    let mut global_idx = None;
                    let special = &edev.chip.special_tiles[&SpecialTileKey::GbRoot];
                    for (&key, &kio) in &special.io {
                        if let SpecialIoKey::GbIn(idx) = key
                            && kio == ioi
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
                                    DiffKey::BelAttrBit(
                                        tcid_ioi,
                                        ioi.slot,
                                        defs::bcls::IOI::PIN_TYPE,
                                        i,
                                    ),
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
                                DiffKey::BelAttrBit(
                                    tcid_ioi,
                                    ioi.slot,
                                    defs::bcls::IOI::OUTPUT_ENABLE,
                                    0,
                                ),
                            );
                            if is_lvds {
                                sample.add_tiled_pattern(
                                    &[btile],
                                    DiffKey::BelAttrBit(
                                        tcid_ioi,
                                        defs::bslots::IOI[idx ^ 1],
                                        defs::bcls::IOI::OUTPUT_ENABLE,
                                        0,
                                    ),
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
                                && let Some((side, attr65, attr40)) = match global_idx {
                                    6 => Some((
                                        DirV::S,
                                        defs::bcls::PLL65::LATCH_GLOBAL_OUT_A,
                                        defs::bcls::PLL40::LATCH_GLOBAL_OUT_A,
                                    )),
                                    3 => Some((
                                        DirV::S,
                                        defs::bcls::PLL65::LATCH_GLOBAL_OUT_B,
                                        defs::bcls::PLL40::LATCH_GLOBAL_OUT_B,
                                    )),
                                    7 => Some((
                                        DirV::N,
                                        defs::bcls::PLL65::LATCH_GLOBAL_OUT_A,
                                        defs::bcls::PLL40::LATCH_GLOBAL_OUT_A,
                                    )),
                                    2 => Some((
                                        DirV::N,
                                        defs::bcls::PLL65::LATCH_GLOBAL_OUT_B,
                                        defs::bcls::PLL40::LATCH_GLOBAL_OUT_B,
                                    )),
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
                                                    .take(special.cells.len() - 2)
                                                    .map(|&crd| BitOwner::Main(crd.col, crd.row)),
                                            )
                                        };
                                        sample.add_tiled_pattern(
                                            &tiles,
                                            if edev.chip.kind.is_ice65() {
                                                DiffKey::BelAttrBit(
                                                    tcid,
                                                    defs::bslots::PLL65,
                                                    attr65,
                                                    0,
                                                )
                                            } else {
                                                DiffKey::BelAttrBit(
                                                    tcid,
                                                    defs::bslots::PLL40,
                                                    attr40,
                                                    0,
                                                )
                                            },
                                        );
                                        handled = true;
                                    }
                                }
                            }
                            if !handled {
                                sample.add_tiled_pattern(
                                    &[btile],
                                    DiffKey::BelAttrBit(
                                        tcid_iob,
                                        defs::bslots::IOB_PAIR,
                                        defs::bcls::IOB_PAIR::LATCH_GLOBAL_OUT,
                                        0,
                                    ),
                                );
                            }
                        }
                    }
                    if let Some(neg_trigger) = inst.props.get("NEG_TRIGGER")
                        && neg_trigger.ends_with('1')
                    {
                        sample.add_tiled_pattern(
                            &[btile],
                            DiffKey::RoutingInv(
                                tcid_ioi,
                                TileWireCoord::new_idx(0, defs::wires::IMUX_IO_ICLK_OPTINV),
                                true,
                            ),
                        );
                        sample.add_tiled_pattern(
                            &[btile],
                            DiffKey::RoutingInv(
                                tcid_ioi,
                                TileWireCoord::new_idx(0, defs::wires::IMUX_IO_OCLK_OPTINV),
                                true,
                            ),
                        );
                    }

                    if inst.kind == "SB_IO_I3C" {
                        let weak_pullup = &inst.props["WEAK_PULLUP"];
                        if weak_pullup.ends_with("0") {
                            sample.add_tiled_pattern_single(
                                &[btile],
                                DiffKey::BelAttrSpecial(
                                    tcid_iob,
                                    iob.slot,
                                    defs::bcls::IOB::WEAK_PULLUP,
                                    specials::DISABLE,
                                ),
                            );
                        }
                        let pullup = &inst.props["PULLUP"];
                        if pullup.ends_with("1") {
                            let pullup_kind = &inst.props["PULLUP_RESISTOR"];
                            sample.add_tiled_pattern_single(
                                &[btile],
                                DiffKey::BelAttrBit(
                                    tcid_iob,
                                    iob.slot,
                                    match pullup_kind.as_str() {
                                        "3P3K" => defs::bcls::IOB::PULLUP_3P3K,
                                        "6P8K" => defs::bcls::IOB::PULLUP_6P8K,
                                        "10K" => defs::bcls::IOB::PULLUP_10K,
                                        _ => unreachable!(),
                                    },
                                    0,
                                ),
                            );
                        } else {
                            sample.add_tiled_pattern_single(
                                &[btile],
                                DiffKey::BelAttrSpecial(
                                    tcid_iob,
                                    iob.slot,
                                    defs::bcls::IOB::PULLUP,
                                    specials::DISABLE,
                                ),
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
                                    DiffKey::BelAttrSpecial(
                                        tcid_iob,
                                        iob.slot,
                                        defs::bcls::IOB::PULLUP,
                                        specials::DISABLE,
                                    ),
                                );
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    DiffKey::BelAttrSpecial(
                                        tcid_iob,
                                        iob.slot,
                                        defs::bcls::IOB::WEAK_PULLUP,
                                        specials::DISABLE,
                                    ),
                                );
                            } else if let Some(pullup_kind) = inst.props.get("PULLUP_RESISTOR")
                                && pullup_kind != "100K"
                            {
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    DiffKey::BelAttrSpecial(
                                        tcid_iob,
                                        iob.slot,
                                        defs::bcls::IOB::WEAK_PULLUP,
                                        specials::DISABLE,
                                    ),
                                );
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    DiffKey::BelAttrBit(
                                        tcid_iob,
                                        iob.slot,
                                        match pullup_kind.as_str() {
                                            "3P3K" => defs::bcls::IOB::PULLUP_3P3K,
                                            "6P8K" => defs::bcls::IOB::PULLUP_6P8K,
                                            "10K" => defs::bcls::IOB::PULLUP_10K,
                                            _ => unreachable!(),
                                        },
                                        0,
                                    ),
                                );
                            }
                        } else if edev.chip.kind != ChipKind::Ice40P01 {
                            if !pullup
                                && !(ioi.col == edev.chip.col_w() && edev.chip.kind.has_vref())
                            {
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    DiffKey::BelAttrSpecial(
                                        tcid_iob,
                                        iob.slot,
                                        defs::bcls::IOB::PULLUP,
                                        specials::DISABLE,
                                    ),
                                );
                            }
                        } else {
                            if !pullup {
                                sample.add_global_pattern(DiffKey::GlobalBelAttrSpecial(
                                    iob,
                                    defs::bcls::IOB::PULLUP,
                                    specials::DISABLE,
                                ));
                            }
                        }
                    }
                    if is_lvds && !edev.chip.kind.has_vref() {
                        sample.add_tiled_pattern_single(
                            &[btile],
                            DiffKey::BelAttrBit(
                                tcid_iob,
                                defs::bslots::IOB_PAIR,
                                defs::bcls::IOB_PAIR::LVDS_INPUT,
                                0,
                            ),
                        );
                        let oiob = iob.bel(defs::bslots::IOB[idx ^ 1]);
                        if edev.chip.kind == ChipKind::Ice40P01 {
                            sample.add_global_pattern_single(DiffKey::GlobalBelAttrSpecial(
                                oiob,
                                defs::bcls::IOB::PULLUP,
                                specials::DISABLE,
                            ));
                        } else {
                            sample.add_tiled_pattern_single(
                                &[btile],
                                DiffKey::BelAttrSpecial(
                                    tcid_iob,
                                    oiob.slot,
                                    defs::bcls::IOB::PULLUP,
                                    specials::DISABLE,
                                ),
                            );
                            if edev.chip.kind.has_multi_pullup() {
                                sample.add_tiled_pattern_single(
                                    &[btile],
                                    DiffKey::BelAttrSpecial(
                                        tcid_iob,
                                        oiob.slot,
                                        defs::bcls::IOB::WEAK_PULLUP,
                                        specials::DISABLE,
                                    ),
                                );
                            }
                        }
                    }
                    if ioi.col == edev.chip.col_w()
                        && edev.chip.kind.has_vref()
                        && let Some(iostd) = iostd
                    {
                        sample.add_tiled_pattern(
                            &[btile],
                            DiffKey::BelSpecialString(
                                tcid_iob,
                                iob.slot,
                                specials::IOSTD,
                                iostd.to_string(),
                            ),
                        );
                    }

                    if ((edev.chip.kind.is_ice40() && !is_lvds)
                        || (edev.chip.kind.has_vref() && ioi.col == edev.chip.col_w()))
                        && ibuf_used.contains(&iid)
                    {
                        if edev.chip.kind == ChipKind::Ice40P01 {
                            sample.add_global_pattern_single(DiffKey::GlobalBelAttrBit(
                                iob,
                                defs::bcls::IOB::IBUF_ENABLE,
                                0,
                            ));
                        } else {
                            sample.add_tiled_pattern_single(
                                &[btile],
                                DiffKey::BelAttrBit(
                                    tcid_iob,
                                    iob.slot,
                                    defs::bcls::IOB::IBUF_ENABLE,
                                    0,
                                ),
                            );
                        }
                    }
                }
                kind if kind.starts_with("SB_DFF") => {
                    let tcid = edev.chip.kind.tile_class_plb();
                    let col = pkg_info.xlat_col[loc.loc.x as usize];
                    let row = pkg_info.xlat_row[loc.loc.y as usize];
                    let lc = loc.loc.bel as usize;
                    let mut kind = kind.strip_prefix("SB_DFF").unwrap();
                    sample.add_tiled_pattern_single(
                        &[BitOwner::Main(col, row)],
                        DiffKey::BelAttrBit(
                            tcid,
                            defs::bslots::LC[lc],
                            defs::bcls::LC::FF_ENABLE,
                            0,
                        ),
                    );
                    if let Some(rest) = kind.strip_prefix('N') {
                        sample.add_tiled_pattern_single(
                            &[BitOwner::Main(col, row)],
                            DiffKey::RoutingInv(
                                tcid,
                                TileWireCoord::new_idx(0, defs::wires::IMUX_CLK_OPTINV),
                                true,
                            ),
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
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::LC[lc],
                                    defs::bcls::LC::FF_SR_VALUE,
                                    0,
                                ),
                            );
                        }
                        "R" => {
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::LC[lc],
                                    defs::bcls::LC::FF_SR_ASYNC,
                                    0,
                                ),
                            );
                        }
                        "S" => {
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::LC[lc],
                                    defs::bcls::LC::FF_SR_VALUE,
                                    0,
                                ),
                            );
                            sample.add_tiled_pattern_single(
                                &[BitOwner::Main(col, row)],
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::LC[lc],
                                    defs::bcls::LC::FF_SR_ASYNC,
                                    0,
                                ),
                            );
                        }
                        "" => (),
                        _ => unreachable!(),
                    }
                }
                kind if kind.starts_with("SB_RAM") => {
                    let bcls = edev.db.bel_classes.get("BRAM").unwrap().1;
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
                    for (key, pin, pinn, pid) in [
                        ("NW", "WCLK", "WCLKN", defs::bcls::BRAM::WCLK),
                        ("NR", "RCLK", "RCLKN", defs::bcls::BRAM::RCLK),
                    ] {
                        let mut wire = edev.get_bel_input(bel, pid);
                        wire.slot = unoptinv[&wire.slot];
                        if kind.contains(key) {
                            let pin = InstPin::Simple(pinn.into());
                            if inst.pins.contains_key(&pin) {
                                let src = int_source[&wire].clone();
                                assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                            }
                            sample.add_tiled_pattern(
                                &[BitOwner::Main(wire.cell.col, wire.cell.row)],
                                DiffKey::RoutingInv(
                                    defs::tcls::INT_BRAM,
                                    TileWireCoord::new_idx(0, defs::wires::IMUX_CLK_OPTINV),
                                    true,
                                ),
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
                        let EntityBundleIndex::Single(pid) = bcls.inputs.get(pin).unwrap().0 else {
                            unreachable!()
                        };
                        let wire = edev.get_bel_input(bel, pid);
                        let pin = InstPin::Simple(pin.into());
                        if inst.pins.contains_key(&pin) {
                            let src = int_source[&wire].clone();
                            assert_eq!(inst.pins[&pin], InstPinSource::FromInst(src.0, src.1));
                        }
                    }
                    let abits = if edev.chip.kind.is_ice40() { 11 } else { 8 };
                    for pin in ["WADDR", "RADDR"] {
                        let EntityBundleIndex::Array(range) = bcls.inputs.get(pin).unwrap().0
                        else {
                            unreachable!()
                        };
                        for idx in 0..abits {
                            let wire = edev.get_bel_input(bel, range.index(idx));
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
                    for pin in ["WDATA", "MASK"] {
                        let EntityBundleIndex::Array(range) = bcls.inputs.get(pin).unwrap().0
                        else {
                            unreachable!()
                        };
                        for idx in 0..16 {
                            let wire = edev.get_bel_input(bel, range.index(idx));
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
                    {
                        let pin = "RDATA";
                        let EntityBundleIndex::Array(range) = bcls.outputs.get(pin).unwrap().0
                        else {
                            unreachable!()
                        };
                        for idx in 0..16 {
                            let wire = edev.get_bel_output(bel, range.index(idx))[0];
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

                    let tcid = edev.chip.kind.tile_class_bram();
                    if design.kind.is_ice40() {
                        sample.add_tiled_pattern_single(
                            &btiles,
                            DiffKey::BelAttrBit(
                                tcid,
                                defs::bslots::BRAM,
                                defs::bcls::BRAM::ENABLE,
                                0,
                            ),
                        );
                    }
                    if let Some(read_mode) = inst.props.get("READ_MODE") {
                        sample.add_tiled_pattern(
                            &btiles,
                            DiffKey::BelAttrValue(
                                tcid,
                                defs::bslots::BRAM,
                                defs::bcls::BRAM::READ_MODE,
                                [
                                    defs::enums::BRAM_MODE::_0,
                                    defs::enums::BRAM_MODE::_1,
                                    defs::enums::BRAM_MODE::_2,
                                    defs::enums::BRAM_MODE::_3,
                                ][read_mode.parse::<usize>().unwrap()],
                            ),
                        );
                    }
                    if let Some(write_mode) = inst.props.get("WRITE_MODE") {
                        sample.add_tiled_pattern(
                            &btiles,
                            DiffKey::BelAttrValue(
                                tcid,
                                defs::bslots::BRAM,
                                defs::bcls::BRAM::WRITE_MODE,
                                [
                                    defs::enums::BRAM_MODE::_0,
                                    defs::enums::BRAM_MODE::_1,
                                    defs::enums::BRAM_MODE::_2,
                                    defs::enums::BRAM_MODE::_3,
                                ][write_mode.parse::<usize>().unwrap()],
                            ),
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
                                            &[
                                                BitOwner::Null,
                                                BitOwner::Null,
                                                BitOwner::Bram(bel.col, bel.row),
                                            ],
                                            DiffKey::BelAttrBit(
                                                tcid,
                                                defs::bslots::BRAM,
                                                defs::bcls::BRAM::INIT,
                                                bit,
                                            ),
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
                    let bslot = if edev.chip.kind.is_ice65() {
                        defs::bslots::PLL65
                    } else {
                        defs::bslots::PLL40
                    };
                    let bcls = &edev.db.bel_classes[if edev.chip.kind.is_ice65() {
                        defs::bcls::PLL65
                    } else {
                        defs::bcls::PLL40
                    }];
                    let ioi_a = special.io[&SpecialIoKey::PllA];
                    let ioi_b = special.io[&SpecialIoKey::PllB];
                    let iob_a = edev.chip.ioi_to_iob(ioi_a).unwrap();
                    let tiles = if edev.chip.kind.is_ice65() {
                        vec![BitOwner::Pll(0), BitOwner::Pll(1)]
                    } else {
                        Vec::from_iter(
                            special
                                .cells
                                .values()
                                .take(special.cells.len() - 2)
                                .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                        )
                    };
                    let tiles_io_a = [BitOwner::Main(ioi_a.col, ioi_a.row)];
                    let tiles_io_b = [BitOwner::Main(ioi_b.col, ioi_b.row)];
                    let tcid_pll = SpecialTileKey::Pll(side).tile_class(edev.chip.kind);
                    if edev.chip.kind.is_ice65() {
                        sample.add_tiled_pattern(
                            &tiles,
                            DiffKey::BelAttrValue(
                                tcid_pll,
                                bslot,
                                defs::bcls::PLL65::MODE,
                                match kind {
                                    "SB_PLL_PAD" => defs::enums::PLL65_MODE::PLL_PAD,
                                    "SB_PLL_CORE" => defs::enums::PLL65_MODE::PLL_CORE,
                                    "SB_PLL_2_PAD" => defs::enums::PLL65_MODE::PLL_2_PAD,
                                    _ => unreachable!(),
                                },
                            ),
                        );
                    } else {
                        sample.add_tiled_pattern(
                            &tiles,
                            DiffKey::BelAttrValue(
                                tcid_pll,
                                bslot,
                                defs::bcls::PLL40::MODE,
                                match kind {
                                    "SB_PLL40_PAD" => defs::enums::PLL40_MODE::PLL40_PAD,
                                    "SB_PLL40_CORE" => defs::enums::PLL40_MODE::PLL40_CORE,
                                    "SB_PLL40_2_PAD" => defs::enums::PLL40_MODE::PLL40_2_PAD,
                                    "SB_PLL40_2F_PAD" => defs::enums::PLL40_MODE::PLL40_2F_PAD,
                                    "SB_PLL40_2F_CORE" => defs::enums::PLL40_MODE::PLL40_2F_CORE,
                                    _ => unreachable!(),
                                },
                            ),
                        );
                    }
                    let tcid_ioi = edev.chip.kind.tile_class_ioi(Dir::V(side)).unwrap();
                    let tcid_iob = edev.chip.kind.tile_class_iob(Dir::V(side)).unwrap();
                    sample.add_tiled_pattern(
                        &tiles_io_a,
                        DiffKey::BelAttrBit(
                            tcid_ioi,
                            defs::bslots::IOI[1],
                            defs::bcls::IOI::PIN_TYPE,
                            0,
                        ),
                    );
                    if edev.chip.kind == ChipKind::Ice40P01 {
                        if kind.ends_with("_PAD") {
                            sample.add_global_pattern_single(DiffKey::GlobalBelAttrSpecial(
                                iob_a,
                                defs::bcls::IOB::PULLUP,
                                specials::DISABLE,
                            ));
                        }
                        sample.add_global_pattern(DiffKey::GlobalBelAttrBit(
                            iob_a,
                            defs::bcls::IOB::IBUF_ENABLE,
                            0,
                        ));
                    } else if kind.ends_with("_PAD") && edev.chip.kind.is_ice40() {
                        sample.add_tiled_pattern_single(
                            &tiles_io_a,
                            DiffKey::BelAttrSpecial(
                                tcid_iob,
                                iob_a.slot,
                                defs::bcls::IOB::PULLUP,
                                specials::DISABLE,
                            ),
                        );
                        sample.add_tiled_pattern_single(
                            &tiles_io_a,
                            DiffKey::BelAttrBit(
                                tcid_iob,
                                iob_a.slot,
                                defs::bcls::IOB::IBUF_ENABLE,
                                0,
                            ),
                        );
                    }
                    if edev.chip.kind.is_ultra() {
                        sample.add_tiled_pattern(
                            &tiles_io_a,
                            DiffKey::BelAttrBit(
                                tcid_ioi,
                                defs::bslots::IOI[1],
                                defs::bcls::IOI::OUTPUT_ENABLE,
                                0,
                            ),
                        );
                    }
                    if matches!(
                        kind,
                        "SB_PLL_2_PAD" | "SB_PLL40_2_PAD" | "SB_PLL40_2F_CORE" | "SB_PLL40_2F_PAD"
                    ) {
                        sample.add_tiled_pattern(
                            &tiles_io_b,
                            DiffKey::BelAttrBit(
                                tcid_ioi,
                                defs::bslots::IOI[0],
                                defs::bcls::IOI::PIN_TYPE,
                                0,
                            ),
                        );
                    }
                    for (prop, val) in &inst.props {
                        let mut prop = prop.as_str();
                        if matches!(prop, "ENABLE_ICEGATE" | "ENABLE_ICEGATE_PORTA") {
                            if val == "1" {
                                sample.add_tiled_pattern(
                                    &tiles_io_a,
                                    DiffKey::BelAttrBit(
                                        tcid_ioi,
                                        defs::bslots::IOI[1],
                                        defs::bcls::IOI::PIN_TYPE,
                                        1,
                                    ),
                                );
                                if edev.chip.kind == ChipKind::Ice40P01 {
                                    sample.add_tiled_pattern(
                                        &tiles_io_a,
                                        DiffKey::BelAttrBit(
                                            tcid_iob,
                                            defs::bslots::IOB_PAIR,
                                            defs::bcls::IOB_PAIR::LATCH_GLOBAL_OUT,
                                            0,
                                        ),
                                    );
                                } else if edev.chip.kind.is_ice65() {
                                    sample.add_tiled_pattern(
                                        &tiles,
                                        DiffKey::BelAttrBit(
                                            tcid_pll,
                                            defs::bslots::PLL65,
                                            defs::bcls::PLL65::LATCH_GLOBAL_OUT_A,
                                            0,
                                        ),
                                    );
                                } else {
                                    sample.add_tiled_pattern(
                                        &tiles,
                                        DiffKey::BelAttrBit(
                                            tcid_pll,
                                            defs::bslots::PLL40,
                                            defs::bcls::PLL40::LATCH_GLOBAL_OUT_A,
                                            0,
                                        ),
                                    );
                                }
                            }
                            continue;
                        }
                        if prop == "ENABLE_ICEGATE_PORTB" {
                            if val == "1" {
                                sample.add_tiled_pattern(
                                    &tiles_io_b,
                                    DiffKey::BelAttrBit(
                                        tcid_ioi,
                                        defs::bslots::IOI[0],
                                        defs::bcls::IOI::PIN_TYPE,
                                        1,
                                    ),
                                );
                                if edev.chip.kind == ChipKind::Ice40P01 {
                                    sample.add_tiled_pattern(
                                        &tiles_io_b,
                                        DiffKey::BelAttrBit(
                                            tcid_iob,
                                            defs::bslots::IOB_PAIR,
                                            defs::bcls::IOB_PAIR::LATCH_GLOBAL_OUT,
                                            0,
                                        ),
                                    );
                                } else if edev.chip.kind.is_ice65() {
                                    sample.add_tiled_pattern(
                                        &tiles,
                                        DiffKey::BelAttrBit(
                                            tcid_pll,
                                            defs::bslots::PLL65,
                                            defs::bcls::PLL65::LATCH_GLOBAL_OUT_B,
                                            0,
                                        ),
                                    );
                                } else {
                                    sample.add_tiled_pattern(
                                        &tiles,
                                        DiffKey::BelAttrBit(
                                            tcid_pll,
                                            defs::bslots::PLL40,
                                            defs::bcls::PLL40::LATCH_GLOBAL_OUT_B,
                                            0,
                                        ),
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
                                    DiffKey::BelAttrBit(
                                        tcid_pll,
                                        bslot,
                                        bcls.attributes.get(prop).unwrap().0,
                                        i,
                                    ),
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
                                        DiffKey::BelAttrBit(
                                            tcid_pll,
                                            bslot,
                                            bcls.attributes.get(prop).unwrap().0,
                                            i,
                                        ),
                                    );
                                }
                            }
                        } else if prop == "PLLOUT_PHASE" {
                            sample.add_tiled_pattern(
                                &tiles,
                                DiffKey::BelAttrValue(
                                    tcid_pll,
                                    bslot,
                                    defs::bcls::PLL65::PLLOUT_PHASE,
                                    match val.as_str() {
                                        "NONE" => defs::enums::PLL65_PLLOUT_PHASE::NONE,
                                        "0deg" => defs::enums::PLL65_PLLOUT_PHASE::_0DEG,
                                        "90deg" => defs::enums::PLL65_PLLOUT_PHASE::_90DEG,
                                        "180deg" => defs::enums::PLL65_PLLOUT_PHASE::_180DEG,
                                        "270deg" => defs::enums::PLL65_PLLOUT_PHASE::_270DEG,
                                        _ => unreachable!(),
                                    },
                                ),
                            );
                        } else if prop == "DELAY_ADJUSTMENT_MODE" {
                            if val == "DYNAMIC" {
                                sample.add_tiled_pattern(
                                    &tiles,
                                    DiffKey::BelAttrBit(
                                        tcid_pll,
                                        bslot,
                                        defs::bcls::PLL65::DELAY_ADJUSTMENT_MODE_DYNAMIC,
                                        0,
                                    ),
                                );
                            }
                        } else {
                            let (aid, attr) = bcls.attributes.get(prop).unwrap();
                            let BelAttributeType::Enum(ecid) = attr.typ else {
                                unreachable!()
                            };
                            sample.add_tiled_pattern(
                                &tiles,
                                DiffKey::BelAttrValue(
                                    tcid_pll,
                                    bslot,
                                    aid,
                                    edev.db.enum_classes[ecid]
                                        .values
                                        .get(&val.to_uppercase())
                                        .unwrap(),
                                ),
                            );
                        }
                    }
                }
                "SB_MAC16" => {
                    let col = pkg_info.xlat_col[loc.loc.x as usize];
                    let row = pkg_info.xlat_row[loc.loc.y as usize];
                    let cell = CellCoord::new(DieId::from_idx(0), col, row);
                    let tiles = Vec::from_iter((0..5).map(|i| BitOwner::Main(col, row + i)));
                    let tcid = edev[cell.tile(defs::tslots::BEL)].class;
                    let tcid_plb = edev.chip.kind.tile_class_plb();
                    for i in 0..4 {
                        for j in 0..8 {
                            for k in [4, 5, 6, 7, 12, 13, 14, 15] {
                                sample.add_tiled_pattern(
                                    &tiles[i..i + 1],
                                    DiffKey::BelAttrBit(
                                        tcid_plb,
                                        defs::bslots::LC[j],
                                        defs::bcls::LC::LUT_INIT,
                                        k,
                                    ),
                                );
                            }
                            sample.add_tiled_pattern(
                                &tiles[i..i + 1],
                                DiffKey::BelAttrBit(
                                    tcid_plb,
                                    defs::bslots::LC[j],
                                    defs::bcls::LC::LTIN_ENABLE,
                                    0,
                                ),
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
                                DiffKey::BelAttrBit(
                                    tcid_plb,
                                    defs::bslots::LC[0],
                                    defs::bcls::LC::LUT_INIT,
                                    k,
                                ),
                            );
                        }
                        sample.add_tiled_pattern(
                            &tiles[4..5],
                            DiffKey::BelAttrBit(
                                tcid_plb,
                                defs::bslots::LC[0],
                                defs::bcls::LC::LTIN_ENABLE,
                                0,
                            ),
                        );
                    }
                    for (prop, val) in &inst.props {
                        for (i, c) in val.chars().rev().enumerate() {
                            assert!(c == '0' || c == '1');
                            if c == '1' {
                                if prop == "NEG_TRIGGER" {
                                    sample.add_tiled_pattern_single(
                                        &tiles[2..3],
                                        DiffKey::RoutingInv(
                                            tcid_plb,
                                            TileWireCoord::new_idx(0, defs::wires::IMUX_CLK_OPTINV),
                                            true,
                                        ),
                                    );
                                } else {
                                    let bcls = &edev.db.bel_classes[defs::bcls::MAC16];
                                    sample.add_tiled_pattern_single(
                                        &tiles,
                                        DiffKey::BelAttrBit(
                                            tcid,
                                            defs::bslots::MAC16,
                                            bcls.attributes.get(&prop.to_uppercase()).unwrap().0,
                                            i,
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
                "SB_HFOSC" => {
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Misc]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let tcid_globals = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    if let Some(val) = design.props.get("VPP_2V5_TO_1P8V")
                        && val == "1"
                    {
                        sample.add_tiled_pattern(
                            &tiles,
                            DiffKey::BelAttrBit(
                                tcid_globals,
                                defs::bslots::HFOSC,
                                defs::bcls::HFOSC::TRIM_FABRIC,
                                0,
                            ),
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
                                DiffKey::BelAttrBit(
                                    tcid_globals,
                                    defs::bslots::HFOSC,
                                    defs::bcls::HFOSC::CLKHF_DIV,
                                    i,
                                ),
                            );
                        }
                    }
                }
                "SB_LFOSC" => {
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Misc]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let tcid_globals = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    if let Some(val) = design.props.get("VPP_2V5_TO_1P8V")
                        && val == "1"
                    {
                        sample.add_tiled_pattern(
                            &tiles,
                            DiffKey::BelAttrBit(
                                tcid_globals,
                                defs::bslots::LFOSC,
                                defs::bcls::LFOSC::TRIM_FABRIC,
                                0,
                            ),
                        );
                    }
                }
                "SB_LED_DRV_CUR" => {
                    let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Misc]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    sample.add_tiled_pattern_single(
                        &tiles,
                        DiffKey::BelAttrBit(
                            tcid,
                            defs::bslots::LED_DRV_CUR,
                            defs::bcls::LED_DRV_CUR::ENABLE,
                            0,
                        ),
                    );
                    if let Some(val) = design.props.get("VPP_2V5_TO_1P8V")
                        && val == "1"
                    {
                        sample.add_tiled_pattern_single(
                            &tiles,
                            DiffKey::BelAttrBit(
                                tcid,
                                defs::bslots::LED_DRV_CUR,
                                defs::bcls::LED_DRV_CUR::TRIM_FABRIC,
                                0,
                            ),
                        );
                    }
                }
                "SB_RGB_DRV" | "SB_RGBA_DRV" => {
                    let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Misc]
                            .cells
                            .values()
                            .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                    );
                    let mut got_any = false;
                    for (attr, prop) in [
                        (defs::bcls::RGB_DRV::RGB0_CURRENT, "RGB0_CURRENT"),
                        (defs::bcls::RGB_DRV::RGB1_CURRENT, "RGB1_CURRENT"),
                        (defs::bcls::RGB_DRV::RGB2_CURRENT, "RGB2_CURRENT"),
                    ] {
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
                                    DiffKey::BelAttrBit(tcid, defs::bslots::RGB_DRV, attr, i),
                                );
                            }
                        }
                    }
                    if inst.kind == "SB_RGBA_DRV" {
                        has_led_v2 = true;
                        if inst.props["CURRENT_MODE"] == "0b1" {
                            sample.add_tiled_pattern_single(
                                &tiles,
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::RGB_DRV,
                                    defs::bcls::RGB_DRV::CURRENT_MODE,
                                    0,
                                ),
                            );
                        }
                        // *not* single because it includes the LED_DRV_CUR RGB_ENABLE bit on T01.
                        sample.add_tiled_pattern(
                            &tiles,
                            DiffKey::BelAttrBit(
                                tcid,
                                defs::bslots::RGB_DRV,
                                defs::bcls::RGB_DRV::ENABLE,
                                0,
                            ),
                        );
                    } else {
                        if got_any {
                            sample.add_tiled_pattern_single(
                                &tiles,
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::RGB_DRV,
                                    defs::bcls::RGB_DRV::ENABLE,
                                    0,
                                ),
                            );
                        }
                    }
                }
                "SB_IR_DRV" => {
                    let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Misc]
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
                                    DiffKey::BelAttrBit(
                                        tcid,
                                        defs::bslots::IR_DRV,
                                        defs::bcls::IR_DRV::IR_CURRENT,
                                        i,
                                    ),
                                );
                            } else {
                                sample.add_tiled_pattern_single(
                                    &tiles,
                                    DiffKey::BelAttrBit(
                                        tcid,
                                        defs::bslots::IR_DRV,
                                        defs::bcls::IR_DRV::IR_CURRENT,
                                        i,
                                    ),
                                );
                            }
                        }
                    }
                }
                "SB_IR500_DRV" => {
                    has_led_v2 = true;
                    let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Misc]
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
                                    DiffKey::BelAttrBit(
                                        tcid,
                                        defs::bslots::IR500_DRV,
                                        defs::bcls::IR500_DRV::BARCODE_CURRENT,
                                        i,
                                    ),
                                );
                            } else {
                                sample.add_tiled_pattern_single(
                                    &tiles,
                                    DiffKey::BelAttrBit(
                                        tcid,
                                        defs::bslots::IR500_DRV,
                                        defs::bcls::IR500_DRV::IR400_CURRENT,
                                        i - 4,
                                    ),
                                );
                            }
                        }
                    }
                    for attr in [
                        defs::bcls::IR500_DRV::BARCODE_ENABLE,
                        defs::bcls::IR500_DRV::IR400_ENABLE,
                        defs::bcls::IR500_DRV::IR500_ENABLE,
                    ] {
                        sample.add_tiled_pattern_single(
                            &tiles,
                            DiffKey::BelAttrBit(tcid, defs::bslots::IR500_DRV, attr, 0),
                        );
                    }
                    if inst.props["CURRENT_MODE"] == "0b1" {
                        led_v2_current_mode = true;
                    }
                }
                "SB_IR400_DRV" => {
                    has_led_v2 = true;
                    let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Misc]
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
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::IR500_DRV,
                                    defs::bcls::IR500_DRV::IR400_CURRENT,
                                    i,
                                ),
                            );
                        }
                    }
                    sample.add_tiled_pattern_single(
                        &tiles,
                        DiffKey::BelAttrBit(
                            tcid,
                            defs::bslots::IR500_DRV,
                            defs::bcls::IR500_DRV::IR400_ENABLE,
                            0,
                        ),
                    );
                    if inst.props["CURRENT_MODE"] == "0b1" {
                        led_v2_current_mode = true;
                    }
                }
                "SB_BARCODE_DRV" => {
                    has_led_v2 = true;
                    let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    let tiles = Vec::from_iter(
                        edev.chip.special_tiles[&SpecialTileKey::Misc]
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
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::IR500_DRV,
                                    defs::bcls::IR500_DRV::BARCODE_CURRENT,
                                    i,
                                ),
                            );
                        }
                    }
                    sample.add_tiled_pattern_single(
                        &tiles,
                        DiffKey::BelAttrBit(
                            tcid,
                            defs::bslots::IR500_DRV,
                            defs::bcls::IR500_DRV::BARCODE_ENABLE,
                            0,
                        ),
                    );
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
                                    DiffKey::BelAttrBit(
                                        defs::tcls::SPRAM,
                                        defs::bslots::SPRAM[i],
                                        defs::bcls::SPRAM::ENABLE,
                                        0,
                                    ),
                                );
                            }
                        }
                    }
                }
                "SB_FILTER_50NS" => {
                    let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
                    let filters = &special_tiles.get(&SpecialTileKey::Misc).unwrap()[..2];
                    for (i, &sloc) in filters.iter().enumerate() {
                        if loc.loc == sloc {
                            let tiles = Vec::from_iter(
                                edev.chip.special_tiles[&SpecialTileKey::Misc]
                                    .cells
                                    .values()
                                    .map(|&cell| BitOwner::Main(cell.col, cell.row)),
                            );
                            // actually sets all three bits; fixed by collector
                            sample.add_tiled_pattern(
                                &tiles,
                                DiffKey::BelAttrBit(
                                    tcid,
                                    defs::bslots::FILTER[i],
                                    defs::bcls::FILTER::ENABLE,
                                    0,
                                ),
                            );
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
                                let ioi = special.io[&xnio];
                                let tcid_iob = edev
                                    .chip
                                    .kind
                                    .tile_class_iob(edev.chip.get_io_edge(ioi))
                                    .unwrap();
                                let btile_io = BitOwner::Main(ioi.col, ioi.row);
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
                                            DiffKey::BelAttrBit(
                                                tcid_iob,
                                                defs::bslots::IOB_PAIR,
                                                defs::bcls::IOB_PAIR::HARDIP_DEDICATED_OUT,
                                                0,
                                            ),
                                        );
                                    } else {
                                        sample.add_tiled_pattern_single(
                                            &[btile_io],
                                            DiffKey::BelAttrBit(
                                                tcid_iob,
                                                defs::bslots::IOB
                                                    [defs::bslots::IOI.index_of(ioi.slot).unwrap()],
                                                defs::bcls::IOB::HARDIP_DEDICATED_OUT,
                                                0,
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
                                            DiffKey::BelAttrBit(
                                                tcid_iob,
                                                defs::bslots::IOB_PAIR,
                                                defs::bcls::IOB_PAIR::HARDIP_FABRIC_IN,
                                                0,
                                            ),
                                        );
                                    } else {
                                        sample.add_tiled_pattern_single(
                                            &[btile_io],
                                            DiffKey::BelAttrBit(
                                                tcid_iob,
                                                defs::bslots::IOB
                                                    [defs::bslots::IOI.index_of(ioi.slot).unwrap()],
                                                defs::bcls::IOB::HARDIP_FABRIC_IN,
                                                0,
                                            ),
                                        );
                                    }
                                    all_ded_ins = false;
                                }
                            }
                            if edev.chip.kind == ChipKind::Ice40R04 {
                                let tcid = key.tile_class(edev.chip.kind);
                                let tcls = &edev.db.tile_classes[tcid];
                                let bslot = match key {
                                    SpecialTileKey::Spi(_) => bels::SPI,
                                    SpecialTileKey::I2c(_) => bels::I2C,
                                    SpecialTileKey::I2cFifo(_) => bels::I2C_FIFO,
                                    _ => unreachable!(),
                                };
                                let BelInfo::Bel(bel) = &tcls.bels[bslot] else {
                                    unreachable!()
                                };
                                let BelKind::Class(bcid) = edev.db.bel_slots[bslot].kind else {
                                    unreachable!()
                                };
                                let bcls = &edev.db.bel_classes[bcid];
                                for (pid, pin_info) in &bel.inputs {
                                    let (pin, _) = bcls.inputs.key(pid);
                                    if all_ded_ins
                                        && matches!(
                                            pin,
                                            "SCLI" | "SDAI" | "SCKI" | "MI" | "SI" | "SCSNI"
                                        )
                                    {
                                        continue;
                                    }
                                    let BelInput::Fixed(pin_wire) = *pin_info else {
                                        unreachable!()
                                    };
                                    let pin_crd = special.cells[pin_wire.cell];
                                    let pin_tile = &edev[pin_crd.tile(defs::tslots::MAIN)];
                                    let pin_btile = BitOwner::Main(pin_crd.col, pin_crd.row);
                                    let ioi =
                                        defs::wires::IOB_DOUT.index_of(pin_wire.wire).unwrap();
                                    sample.add_tiled_pattern_single(
                                        &[pin_btile],
                                        DiffKey::BelAttrBit(
                                            pin_tile.class,
                                            defs::bslots::IOI[ioi],
                                            defs::bcls::IOI::PIN_TYPE,
                                            3,
                                        ),
                                    );
                                    sample.add_tiled_pattern_single(
                                        &[pin_btile],
                                        DiffKey::BelAttrBit(
                                            pin_tile.class,
                                            defs::bslots::IOI[ioi],
                                            defs::bcls::IOI::PIN_TYPE,
                                            4,
                                        ),
                                    );
                                }
                                for (pid, pin_info) in &bel.outputs {
                                    let (pin, idx) = bcls.outputs.key(pid);
                                    if all_ded_outs
                                        && matches!(
                                            pin,
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
                                        )
                                    {
                                        continue;
                                    }
                                    if all_ded_outs
                                        && matches!(pin, "MCSNO" | "MCSNOE")
                                        && let EntityBundleItemIndex::Array { index, .. } = idx
                                        && index < 2
                                    {
                                        continue;
                                    }
                                    let pin_wire = *pin_info.iter().next().unwrap();
                                    let pin_crd = special.cells[pin_wire.cell];
                                    let pin_tile = &edev[pin_crd.tile(defs::tslots::MAIN)];
                                    let pin_btile = BitOwner::Main(pin_crd.col, pin_crd.row);
                                    let ioi = defs::wires::IOB_DIN.index_of(pin_wire.wire).unwrap();
                                    sample.add_tiled_pattern_single(
                                        &[pin_btile],
                                        DiffKey::BelAttrBit(
                                            pin_tile.class,
                                            defs::bslots::IOI[ioi],
                                            defs::bcls::IOI::PIN_TYPE,
                                            0,
                                        ),
                                    );
                                }
                            }
                            for prop in ["SDA_INPUT_DELAYED", "SDA_OUTPUT_DELAYED"] {
                                if let Some(val) = inst.props.get(prop)
                                    && val == "1"
                                {
                                    let ioi = special.io[&SpecialIoKey::I2cSda];
                                    let tcid_iob = edev
                                        .chip
                                        .kind
                                        .tile_class_iob(edev.chip.get_io_edge(ioi))
                                        .unwrap();
                                    sample.add_tiled_pattern_single(
                                        &[BitOwner::Main(ioi.col, ioi.row)],
                                        DiffKey::BelAttrBit(
                                            tcid_iob,
                                            defs::bslots::IOB_PAIR,
                                            edev.db.bel_classes[defs::bcls::IOB_PAIR]
                                                .attributes
                                                .get(prop)
                                                .unwrap()
                                                .0,
                                            0,
                                        ),
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
        let tcid = SpecialTileKey::Misc.tile_class(edev.chip.kind);
        let tiles = Vec::from_iter(
            edev.chip.special_tiles[&SpecialTileKey::Misc]
                .cells
                .values()
                .map(|&cell| BitOwner::Main(cell.col, cell.row)),
        );
        if led_v2_current_mode {
            sample.add_tiled_pattern(
                &tiles,
                DiffKey::BelAttrBit(
                    tcid,
                    defs::bslots::IR500_DRV,
                    defs::bcls::IR500_DRV::CURRENT_MODE,
                    0,
                ),
            );
        }
        if let Some(val) = design.props.get("VPP_2V5_TO_1P8V")
            && val == "1"
        {
            sample.add_tiled_pattern_single(
                &tiles,
                DiffKey::BelAttrBit(
                    tcid,
                    defs::bslots::LED_DRV_CUR,
                    defs::bcls::LED_DRV_CUR::TRIM_FABRIC,
                    0,
                ),
            );
        }
    }
    for opt in &design.opts {
        match opt.as_str() {
            "--frequency low" => {
                sample.add_tiled_pattern(
                    &[BitOwner::CReg, BitOwner::Speed],
                    DiffKey::BelAttrValue(
                        defs::tcls::GLOBALS,
                        defs::bslots::GLOBAL_OPTIONS,
                        defs::bcls::GLOBAL_OPTIONS::SPEED,
                        defs::enums::CONFIG_SPEED::LOW,
                    ),
                );
            }
            "--frequency medium" => {
                sample.add_tiled_pattern(
                    &[BitOwner::CReg, BitOwner::Speed],
                    DiffKey::BelAttrValue(
                        defs::tcls::GLOBALS,
                        defs::bslots::GLOBAL_OPTIONS,
                        defs::bcls::GLOBAL_OPTIONS::SPEED,
                        defs::enums::CONFIG_SPEED::MEDIUM,
                    ),
                );
            }
            "--frequency high" => {
                sample.add_tiled_pattern(
                    &[BitOwner::CReg, BitOwner::Speed],
                    DiffKey::BelAttrValue(
                        defs::tcls::GLOBALS,
                        defs::bslots::GLOBAL_OPTIONS,
                        defs::bcls::GLOBAL_OPTIONS::SPEED,
                        defs::enums::CONFIG_SPEED::HIGH,
                    ),
                );
            }
            _ => panic!("ummm {opt}"),
        }
    }
    (sample, pips)
}

pub fn wanted_keys_tiled(edev: &ExpandedDevice) -> Vec<DiffKey> {
    let mut result = vec![];
    // PLB
    let tcid = edev.chip.kind.tile_class_plb();
    for lc in 0..8 {
        if edev.chip.kind.is_ice40() {
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::LC[lc],
                defs::bcls::LC::LTIN_ENABLE,
                0,
            ));
        }
        for i in 0..16 {
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::LC[lc],
                defs::bcls::LC::LUT_INIT,
                i,
            ));
        }
        for attr in [
            defs::bcls::LC::CARRY_ENABLE,
            defs::bcls::LC::FF_ENABLE,
            defs::bcls::LC::FF_SR_VALUE,
            defs::bcls::LC::FF_SR_ASYNC,
        ] {
            result.push(DiffKey::BelAttrBit(tcid, defs::bslots::LC[lc], attr, 0));
        }
    }
    for val in [
        defs::enums::LC_MUX_CI::ZERO,
        defs::enums::LC_MUX_CI::ONE,
        defs::enums::LC_MUX_CI::CHAIN,
    ] {
        result.push(DiffKey::BelAttrValue(
            tcid,
            defs::bslots::LC[0],
            defs::bcls::LC::MUX_CI,
            val,
        ));
    }
    result.push(DiffKey::RoutingInv(
        tcid,
        TileWireCoord::new_idx(0, defs::wires::IMUX_CLK_OPTINV),
        true,
    ));
    if let Some(tcid) = edev.chip.kind.tile_class_colbuf() {
        for i in 0..8 {
            let wt = TileWireCoord::new_idx(0, defs::wires::GLOBAL[i]);
            let wf = TileWireCoord::new_idx(0, defs::wires::GLOBAL_ROOT[i]).pos();
            result.push(DiffKey::Routing(tcid, wt, wf));
            if edev.chip.kind.has_ioi_we() {
                result.push(DiffKey::Routing(defs::tcls::COLBUF_IO_W, wt, wf));
                result.push(DiffKey::Routing(defs::tcls::COLBUF_IO_E, wt, wf));
            }
        }
    }
    // BRAM
    if !edev.chip.cols_bram.is_empty() {
        let tcid = edev.chip.kind.tile_class_bram();
        if edev.chip.kind.is_ice40() {
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::CASCADE_OUT_WADDR,
                0,
            ));
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::CASCADE_OUT_RADDR,
                0,
            ));
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::CASCADE_IN_WADDR,
                0,
            ));
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::CASCADE_IN_RADDR,
                0,
            ));
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::ENABLE,
                0,
            ));
            result.push(DiffKey::BelAttrValue(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::READ_MODE,
                defs::enums::BRAM_MODE::_0,
            ));
            result.push(DiffKey::BelAttrValue(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::READ_MODE,
                defs::enums::BRAM_MODE::_1,
            ));
            result.push(DiffKey::BelAttrValue(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::READ_MODE,
                defs::enums::BRAM_MODE::_2,
            ));
            result.push(DiffKey::BelAttrValue(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::READ_MODE,
                defs::enums::BRAM_MODE::_3,
            ));
            result.push(DiffKey::BelAttrValue(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::WRITE_MODE,
                defs::enums::BRAM_MODE::_0,
            ));
            result.push(DiffKey::BelAttrValue(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::WRITE_MODE,
                defs::enums::BRAM_MODE::_1,
            ));
            result.push(DiffKey::BelAttrValue(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::WRITE_MODE,
                defs::enums::BRAM_MODE::_2,
            ));
            result.push(DiffKey::BelAttrValue(
                tcid,
                defs::bslots::BRAM,
                defs::bcls::BRAM::WRITE_MODE,
                defs::enums::BRAM_MODE::_3,
            ));
            for i in 0..4096 {
                result.push(DiffKey::BelAttrBit(
                    tcid,
                    defs::bslots::BRAM,
                    defs::bcls::BRAM::INIT,
                    i,
                ));
            }
        }
        result.push(DiffKey::RoutingInv(
            defs::tcls::INT_BRAM,
            TileWireCoord::new_idx(0, defs::wires::IMUX_CLK_OPTINV),
            true,
        ));
    }
    // IO
    for edge in Dir::DIRS {
        let Some(tcid) = edev.chip.kind.tile_class_ioi(edge) else {
            continue;
        };
        result.push(DiffKey::RoutingInv(
            tcid,
            TileWireCoord::new_idx(0, defs::wires::IMUX_IO_ICLK_OPTINV),
            true,
        ));
        result.push(DiffKey::RoutingInv(
            tcid,
            TileWireCoord::new_idx(0, defs::wires::IMUX_IO_OCLK_OPTINV),
            true,
        ));
        for io in 0..2 {
            for i in 0..6 {
                result.push(DiffKey::BelAttrBit(
                    tcid,
                    defs::bslots::IOI[io],
                    defs::bcls::IOI::PIN_TYPE,
                    i,
                ));
            }
            if edev.chip.kind.is_ultra() {
                result.push(DiffKey::BelAttrBit(
                    tcid,
                    defs::bslots::IOI[io],
                    defs::bcls::IOI::OUTPUT_ENABLE,
                    0,
                ));
            }
        }
        let Some(tcid) = edev.chip.kind.tile_class_iob(edge) else {
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
            let bel = defs::bslots::IOB[iob];
            if edev.chip.kind.is_ice40() || (edge == Dir::W && edev.chip.kind.has_vref()) {
                result.push(DiffKey::BelAttrBit(
                    tcid,
                    bel,
                    defs::bcls::IOB::IBUF_ENABLE,
                    0,
                ));
            }
            if edev.chip.kind.is_ultra()
                && !(edge == Dir::N && edev.chip.kind == ChipKind::Ice40T01)
            {
                for attr in [
                    defs::bcls::IOB::HARDIP_FABRIC_IN,
                    defs::bcls::IOB::HARDIP_DEDICATED_OUT,
                ] {
                    result.push(DiffKey::BelAttrBit(tcid, bel, attr, 0));
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
                    result.push(DiffKey::BelSpecialString(
                        tcid,
                        bel,
                        specials::IOSTD,
                        iostd.to_string(),
                    ));
                }
                if iob == 0 {
                    for iostd in ["SB_LVDS_INPUT", "SB_SUBLVDS_INPUT"] {
                        result.push(DiffKey::BelSpecialString(
                            tcid,
                            bel,
                            specials::IOSTD,
                            iostd.to_string(),
                        ));
                    }
                }
            } else {
                result.push(DiffKey::BelAttrSpecial(
                    tcid,
                    bel,
                    defs::bcls::IOB::PULLUP,
                    specials::DISABLE,
                ));
                if edev.chip.kind.has_multi_pullup() {
                    for attr in [
                        defs::bcls::IOB::PULLUP_3P3K,
                        defs::bcls::IOB::PULLUP_6P8K,
                        defs::bcls::IOB::PULLUP_10K,
                    ] {
                        result.push(DiffKey::BelAttrBit(tcid, bel, attr, 0));
                    }
                    result.push(DiffKey::BelAttrSpecial(
                        tcid,
                        bel,
                        defs::bcls::IOB::WEAK_PULLUP,
                        specials::DISABLE,
                    ));
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
        if has_lvds && !(edge == Dir::W && edev.chip.kind.has_vref()) {
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::IOB_PAIR,
                defs::bcls::IOB_PAIR::LVDS_INPUT,
                0,
            ));
        }
        if has_latch_global_out {
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::IOB_PAIR,
                defs::bcls::IOB_PAIR::LATCH_GLOBAL_OUT,
                0,
            ));
        }
        if edev.chip.kind == ChipKind::Ice40R04 {
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::IOB_PAIR,
                defs::bcls::IOB_PAIR::HARDIP_FABRIC_IN,
                0,
            ));
            result.push(DiffKey::BelAttrBit(
                tcid,
                defs::bslots::IOB_PAIR,
                defs::bcls::IOB_PAIR::HARDIP_DEDICATED_OUT,
                0,
            ));
        }
        if edev.chip.kind.is_ultra() {
            let i2c_edge = if edev.chip.kind == ChipKind::Ice40T01 {
                Dir::S
            } else {
                Dir::N
            };
            if edge == i2c_edge {
                for attr in [
                    defs::bcls::IOB_PAIR::SDA_INPUT_DELAYED,
                    defs::bcls::IOB_PAIR::SDA_OUTPUT_DELAYED,
                ] {
                    result.push(DiffKey::BelAttrBit(tcid, defs::bslots::IOB_PAIR, attr, 0));
                }
            }
        }
    }
    for side in [DirV::S, DirV::N] {
        let key = SpecialTileKey::Pll(side);
        if edev.chip.special_tiles.contains_key(&key) {
            let tcid = key.tile_class(edev.chip.kind);
            if edev.chip.kind.is_ice65() {
                for val in edev.db.enum_classes[defs::enums::PLL65_MODE].values.ids() {
                    if val == defs::enums::PLL40_MODE::NONE {
                        continue;
                    }
                    result.push(DiffKey::BelAttrValue(
                        tcid,
                        defs::bslots::PLL65,
                        defs::bcls::PLL65::MODE,
                        val,
                    ));
                }
                for attr in [
                    defs::bcls::PLL65::FEEDBACK_PATH,
                    defs::bcls::PLL65::PLLOUT_PHASE,
                ] {
                    let BelAttributeType::Enum(ecid) =
                        edev.db.bel_classes[defs::bcls::PLL65].attributes[attr].typ
                    else {
                        unreachable!()
                    };
                    for val in edev.db.enum_classes[ecid].values.ids() {
                        result.push(DiffKey::BelAttrValue(tcid, defs::bslots::PLL65, attr, val));
                    }
                }
                result.push(DiffKey::BelAttrBit(
                    tcid,
                    defs::bslots::PLL65,
                    defs::bcls::PLL65::DELAY_ADJUSTMENT_MODE_DYNAMIC,
                    0,
                ));
                for (attr, width) in [
                    (defs::bcls::PLL65::FIXED_DELAY_ADJUSTMENT, 4),
                    (defs::bcls::PLL65::DIVR, 4),
                    (defs::bcls::PLL65::DIVF, 6),
                    (defs::bcls::PLL65::DIVQ, 3),
                    (defs::bcls::PLL65::FILTER_RANGE, 3),
                    (defs::bcls::PLL65::TEST_MODE, 1),
                    (defs::bcls::PLL65::LATCH_GLOBAL_OUT_A, 1),
                    (defs::bcls::PLL65::LATCH_GLOBAL_OUT_B, 1),
                ] {
                    for i in 0..width {
                        result.push(DiffKey::BelAttrBit(tcid, defs::bslots::PLL65, attr, i));
                    }
                }
            } else {
                for val in edev.db.enum_classes[defs::enums::PLL40_MODE].values.ids() {
                    if val == defs::enums::PLL40_MODE::NONE {
                        continue;
                    }
                    result.push(DiffKey::BelAttrValue(
                        tcid,
                        defs::bslots::PLL40,
                        defs::bcls::PLL40::MODE,
                        val,
                    ));
                }
                for attr in [
                    defs::bcls::PLL40::FEEDBACK_PATH,
                    defs::bcls::PLL40::DELAY_ADJUSTMENT_MODE_FEEDBACK,
                    defs::bcls::PLL40::DELAY_ADJUSTMENT_MODE_RELATIVE,
                ] {
                    let BelAttributeType::Enum(ecid) =
                        edev.db.bel_classes[defs::bcls::PLL40].attributes[attr].typ
                    else {
                        unreachable!()
                    };
                    for val in edev.db.enum_classes[ecid].values.ids() {
                        result.push(DiffKey::BelAttrValue(tcid, defs::bslots::PLL40, attr, val));
                    }
                }
                for attr in [
                    defs::bcls::PLL40::PLLOUT_SELECT_PORTA,
                    defs::bcls::PLL40::PLLOUT_SELECT_PORTB,
                ] {
                    for val in [
                        defs::enums::PLL40_PLLOUT_SELECT::GENCLK_HALF,
                        defs::enums::PLL40_PLLOUT_SELECT::SHIFTREG_0DEG,
                        defs::enums::PLL40_PLLOUT_SELECT::SHIFTREG_90DEG,
                    ] {
                        result.push(DiffKey::BelAttrValue(tcid, defs::bslots::PLL40, attr, val));
                    }
                }
                for (attr, width) in [
                    (defs::bcls::PLL40::SHIFTREG_DIV_MODE, 1),
                    (defs::bcls::PLL40::FDA_FEEDBACK, 4),
                    (defs::bcls::PLL40::FDA_RELATIVE, 4),
                    (defs::bcls::PLL40::DIVR, 4),
                    (defs::bcls::PLL40::DIVF, 7),
                    (defs::bcls::PLL40::DIVQ, 3),
                    (defs::bcls::PLL40::FILTER_RANGE, 3),
                    (defs::bcls::PLL40::TEST_MODE, 1),
                ] {
                    for i in 0..width {
                        result.push(DiffKey::BelAttrBit(tcid, defs::bslots::PLL40, attr, i));
                    }
                }
                if edev.chip.kind != ChipKind::Ice40P01 {
                    for attr in [
                        defs::bcls::PLL40::LATCH_GLOBAL_OUT_A,
                        defs::bcls::PLL40::LATCH_GLOBAL_OUT_B,
                    ] {
                        result.push(DiffKey::BelAttrBit(tcid, defs::bslots::PLL40, attr, 0));
                    }
                }
            }
        }
        let key = SpecialTileKey::PllStub(side);
        if edev.chip.special_tiles.contains_key(&key) {
            let tcid = key.tile_class(edev.chip.kind);
            for attr in [
                defs::bcls::PLL40::LATCH_GLOBAL_OUT_A,
                defs::bcls::PLL40::LATCH_GLOBAL_OUT_B,
            ] {
                result.push(DiffKey::BelAttrBit(tcid, defs::bslots::PLL40, attr, 0));
            }
        }
    }
    if edev.chip.kind.is_ultra() {
        // OSC & TRIM
        let tcid_globals = SpecialTileKey::Misc.tile_class(edev.chip.kind);
        result.push(DiffKey::BelAttrBit(
            tcid_globals,
            defs::bslots::HFOSC,
            defs::bcls::HFOSC::CLKHF_DIV,
            0,
        ));
        result.push(DiffKey::BelAttrBit(
            tcid_globals,
            defs::bslots::HFOSC,
            defs::bcls::HFOSC::CLKHF_DIV,
            1,
        ));
        result.push(DiffKey::BelAttrBit(
            tcid_globals,
            defs::bslots::HFOSC,
            defs::bcls::HFOSC::TRIM_FABRIC,
            0,
        ));
        result.push(DiffKey::BelAttrBit(
            tcid_globals,
            defs::bslots::LFOSC,
            defs::bcls::LFOSC::TRIM_FABRIC,
            0,
        ));
        result.push(DiffKey::BelAttrBit(
            tcid_globals,
            defs::bslots::LED_DRV_CUR,
            defs::bcls::LED_DRV_CUR::TRIM_FABRIC,
            0,
        ));

        // DRV
        result.push(DiffKey::BelAttrBit(
            tcid_globals,
            defs::bslots::RGB_DRV,
            defs::bcls::RGB_DRV::ENABLE,
            0,
        ));
        for attr in [
            defs::bcls::RGB_DRV::RGB0_CURRENT,
            defs::bcls::RGB_DRV::RGB1_CURRENT,
            defs::bcls::RGB_DRV::RGB2_CURRENT,
        ] {
            for j in 0..6 {
                result.push(DiffKey::BelAttrBit(
                    tcid_globals,
                    defs::bslots::RGB_DRV,
                    attr,
                    j,
                ));
            }
        }
        if edev.chip.kind == ChipKind::Ice40T04 {
            result.push(DiffKey::BelAttrBit(
                tcid_globals,
                defs::bslots::LED_DRV_CUR,
                defs::bcls::LED_DRV_CUR::ENABLE,
                0,
            ));
            for j in 0..10 {
                result.push(DiffKey::BelAttrBit(
                    tcid_globals,
                    defs::bslots::IR_DRV,
                    defs::bcls::IR_DRV::IR_CURRENT,
                    j,
                ));
            }
        } else {
            result.push(DiffKey::BelAttrBit(
                tcid_globals,
                defs::bslots::RGB_DRV,
                defs::bcls::RGB_DRV::CURRENT_MODE,
                0,
            ));
            if edev.chip.kind == ChipKind::Ice40T01 {
                for attr in [
                    defs::bcls::IR500_DRV::BARCODE_ENABLE,
                    defs::bcls::IR500_DRV::IR400_ENABLE,
                    defs::bcls::IR500_DRV::IR500_ENABLE,
                    defs::bcls::IR500_DRV::CURRENT_MODE,
                ] {
                    result.push(DiffKey::BelAttrBit(
                        tcid_globals,
                        defs::bslots::IR500_DRV,
                        attr,
                        0,
                    ));
                }
                for j in 0..8 {
                    result.push(DiffKey::BelAttrBit(
                        tcid_globals,
                        defs::bslots::IR500_DRV,
                        defs::bcls::IR500_DRV::IR400_CURRENT,
                        j,
                    ));
                }
                for j in 0..4 {
                    result.push(DiffKey::BelAttrBit(
                        tcid_globals,
                        defs::bslots::IR500_DRV,
                        defs::bcls::IR500_DRV::BARCODE_CURRENT,
                        j,
                    ));
                }
            }
        }
    }
    // MAC16
    if matches!(edev.chip.kind, ChipKind::Ice40T04 | ChipKind::Ice40T05) {
        for tcid in [defs::tcls::MAC16, defs::tcls::MAC16_TRIM] {
            if tcid == defs::tcls::MAC16_TRIM && edev.chip.kind != ChipKind::Ice40T05 {
                continue;
            }
            for (attr, width) in [
                (defs::bcls::MAC16::A_REG, 1),
                (defs::bcls::MAC16::B_REG, 1),
                (defs::bcls::MAC16::C_REG, 1),
                (defs::bcls::MAC16::D_REG, 1),
                (defs::bcls::MAC16::TOP_8X8_MULT_REG, 1),
                (defs::bcls::MAC16::BOT_8X8_MULT_REG, 1),
                (defs::bcls::MAC16::PIPELINE_16X16_MULT_REG1, 1),
                (defs::bcls::MAC16::PIPELINE_16X16_MULT_REG2, 1),
                (defs::bcls::MAC16::TOPOUTPUT_SELECT, 2),
                (defs::bcls::MAC16::BOTOUTPUT_SELECT, 2),
                (defs::bcls::MAC16::TOPADDSUB_LOWERINPUT, 2),
                (defs::bcls::MAC16::BOTADDSUB_LOWERINPUT, 2),
                (defs::bcls::MAC16::TOPADDSUB_UPPERINPUT, 1),
                (defs::bcls::MAC16::BOTADDSUB_UPPERINPUT, 1),
                (defs::bcls::MAC16::TOPADDSUB_CARRYSELECT, 2),
                (defs::bcls::MAC16::BOTADDSUB_CARRYSELECT, 2),
                (defs::bcls::MAC16::MODE_8X8, 1),
                (defs::bcls::MAC16::A_SIGNED, 1),
                (defs::bcls::MAC16::B_SIGNED, 1),
            ] {
                for i in 0..width {
                    result.push(DiffKey::BelAttrBit(tcid, defs::bslots::MAC16, attr, i));
                }
            }
        }
    }
    // SPRAM, FILTER
    if edev.chip.kind == ChipKind::Ice40T05 {
        for slot in defs::bslots::SPRAM {
            result.push(DiffKey::BelAttrBit(
                defs::tcls::SPRAM,
                slot,
                defs::bcls::SPRAM::ENABLE,
                0,
            ));
        }
        for slot in defs::bslots::FILTER {
            result.push(DiffKey::BelAttrBit(
                defs::tcls::MISC_T05,
                slot,
                defs::bcls::FILTER::ENABLE,
                0,
            ));
        }
    }
    // misc
    let tcid_gb_root = edev.chip.kind.tile_class_gb_root();
    let BelInfo::SwitchBox(ref sb) = edev.db.tile_classes[tcid_gb_root].bels[defs::bslots::GB_ROOT]
    else {
        unreachable!()
    };
    for item in &sb.items {
        let SwitchBoxItem::Mux(mux) = item else {
            unreachable!()
        };
        for &src in mux.src.keys() {
            result.push(DiffKey::Routing(tcid_gb_root, mux.dst, src));
        }
    }
    if edev.chip.kind != ChipKind::Ice40T04 {
        for val in [
            defs::enums::CONFIG_SPEED::LOW,
            defs::enums::CONFIG_SPEED::MEDIUM,
            defs::enums::CONFIG_SPEED::HIGH,
        ] {
            result.push(DiffKey::BelAttrValue(
                defs::tcls::GLOBALS,
                defs::bslots::GLOBAL_OPTIONS,
                defs::bcls::GLOBAL_OPTIONS::SPEED,
                val,
            ));
        }
    }
    result
}

pub fn wanted_keys_global(edev: &ExpandedDevice) -> Vec<DiffKey> {
    let mut result = vec![];
    if edev.chip.kind == ChipKind::Ice40P01 {
        for &bel in edev.chip.ioi_iob.keys_right() {
            result.push(DiffKey::GlobalBelAttrBit(
                bel,
                defs::bcls::IOB::IBUF_ENABLE,
                0,
            ));
            result.push(DiffKey::GlobalBelAttrSpecial(
                bel,
                defs::bcls::IOB::PULLUP,
                specials::DISABLE,
            ));
        }
    }
    result
}

pub fn get_golden_mux_stats(kind: ChipKind, tcid: TileClassId) -> BTreeMap<String, usize> {
    let mut golden_stats = BTreeMap::new();
    if matches!(
        tcid,
        defs::tcls::INT_BRAM | defs::tcls::PLB_L04 | defs::tcls::PLB_L08 | defs::tcls::PLB_P01
    ) {
        golden_stats.insert("IMUX_CLK".to_string(), 12);
        golden_stats.insert("IMUX_CE".to_string(), 8);
        golden_stats.insert("IMUX_RST".to_string(), 8);
        for lc in 0..8 {
            for i in 0..4 {
                if i == 2 && tcid == defs::tcls::INT_BRAM {
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
                golden_stats.insert(
                    format!("IMUX_LC_I{i}[{lc}]"),
                    if i == 3 && tcid == defs::tcls::INT_BRAM {
                        15
                    } else {
                        16
                    },
                );
            }
        }
        for g in 0..4 {
            for i in 0..8 {
                golden_stats.insert(
                    format!("LOCAL_{g}[{i}]"),
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
            ("IMUX_IO_ICLK", 12),
            ("IMUX_IO_OCLK", 12),
            ("IMUX_CE", 8),
            ("IMUX_IO_EXTRA", 8),
            ("IMUX_IO_DOUT0[0]", 8),
            ("IMUX_IO_DOUT1[0]", 8),
            ("IMUX_IO_OE[0]", 8),
            ("IMUX_IO_DOUT0[1]", 8),
            ("IMUX_IO_DOUT1[1]", 8),
            ("IMUX_IO_OE[1]", 8),
        ] {
            golden_stats.insert(k.into(), v);
        }
        for g in 0..2 {
            for i in 0..8 {
                golden_stats.insert(format!("LOCAL_{g}[{i}]"), 14);
            }
        }
        if matches!(
            tcid,
            defs::tcls::IOI_S_L04
                | defs::tcls::IOI_N_L04
                | defs::tcls::IOI_S_L08
                | defs::tcls::IOI_N_L08
                | defs::tcls::IOI_S_T04
                | defs::tcls::IOI_N_T04
        ) {
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
