use std::{
    collections::{BTreeMap, BTreeSet, btree_map},
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use clap::Parser;
use collect::{collect, collect_iob};
use generate::{GeneratorConfig, generate};
use intdb::MiscTileBuilder;
use parts::Part;
use pkg::get_pkg_pins;
use prims::{Primitive, get_prims};
use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::{
        BelAttribute, BelInfo, CellSlotId, IntDb, Mux, ProgBuf, ProgInv, SwitchBox, SwitchBoxItem,
        TileClass, TileClassId, TileWireCoord,
    },
    dir::{Dir, DirH, DirPartMap, DirV},
    grid::{BelCoord, CellCoord, ColId, DieId, RowId, WireCoord},
};
use prjcombine_re_fpga_hammer::DiffKey;
use prjcombine_re_harvester::Harvester;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_siliconblue::{
    bond::Bond,
    chip::{Chip, ChipKind, SpecialIoKey, SpecialTile, SpecialTileKey},
    db::Database,
    defs::{self, bslots as bels},
    expanded::{BitOwner, ExpandedDevice},
};
use prjcombine_types::{
    bimap::BiMap,
    bsdata::{BitRectId, PolTileBit, TileBit},
    speed::Speed,
};
use rand::Rng;
use rayon::prelude::*;
use run::{Design, InstPin, RawLoc, RunResult, get_cached_designs, remove_cache_key, run};
use sample::{get_golden_mux_stats, make_sample, wanted_keys_global, wanted_keys_tiled};
use sites::{
    BelPins, SiteInfo, find_bel_pins, find_io_latch_locs, find_sites_iox3, find_sites_misc,
    find_sites_plb,
};
use speed::{SpeedCollector, finish_speed, get_speed_data, init_speed_data};

use crate::xlat::{is_long, is_long_h, is_long_v, is_quad, is_quad_h, is_quad_v, is_quad_v_w};

mod collect;
mod generate;
mod intdb;
mod parts;
mod pkg;
mod prims;
mod run;
mod sample;
mod sites;
mod specials;
mod speed;
mod xlat;

#[derive(Parser)]
struct Args {
    toolchain: PathBuf,
    kinds: Vec<String>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

pub struct PkgInfo {
    pub part: &'static Part,
    pub bond: Bond,
    pub empty_run: RunResult,
    pub bel_info: BTreeMap<&'static str, Vec<SiteInfo>>,
    pub xlat_col: Vec<ColId>,
    pub xlat_row: Vec<RowId>,
    pub xlat_ioi: BTreeMap<(u32, u32, u32), BelCoord>,
}

#[allow(clippy::type_complexity)]
struct PartContext<'a> {
    parts: Vec<&'static Part>,
    chip: Chip,
    intdb: IntDb,
    toolchain: &'a Toolchain,
    prims: BTreeMap<&'static str, Primitive>,
    pkgs: BTreeMap<(&'static str, &'static str), PkgInfo>,
    extra_wire_names: BTreeMap<(u32, u32, String), WireCoord>,
    bel_pins: BTreeMap<(&'static str, RawLoc), BelPins>,
    special_tiles: BTreeMap<SpecialTileKey, Vec<RawLoc>>,
    speed: BTreeMap<(&'static str, &'static str), Speed>,
    tcls_filled: BTreeSet<TileClassId>,
    debug: u8,
}

struct HarvestContext<'a> {
    ctx: &'a PartContext<'a>,
    edev: &'a ExpandedDevice<'a>,
    gencfg: GeneratorConfig<'a>,
    harvester: Mutex<Harvester<BitOwner>>,
    speed: BTreeMap<(&'static str, &'static str), Mutex<SpeedCollector>>,
    pips: Mutex<BTreeMap<TileClassId, BTreeSet<(TileWireCoord, TileWireCoord)>>>,
}

impl HarvestContext<'_> {
    fn compute_rows_colbuf(
        &self,
        colbuf_map: BTreeMap<RowId, RowId>,
    ) -> Option<Vec<(RowId, RowId, RowId)>> {
        let mut row_c = *colbuf_map.get(&RowId::from_idx(1))?;
        let mut row_b = RowId::from_idx(0);
        let mut row_prev = RowId::from_idx(0);
        let mut in_top = false;
        let mut result = vec![];
        for (row, trow) in colbuf_map {
            if trow != row_c {
                if row != row_prev + 1 {
                    return None;
                }
                if !in_top {
                    assert_eq!(trow, row_c + 1);
                    assert_eq!(trow, row);
                    in_top = true;
                } else {
                    result.push((row_c, row_b, row));
                    row_b = row;
                    in_top = false;
                }
                row_c = trow;
            }
            row_prev = row;
        }
        if row_prev != self.ctx.chip.row_n() - 1 {
            return None;
        }
        assert!(in_top);
        result.push((row_c, row_b, row_prev + 2));
        Some(result)
    }

    fn handle_colbufs(&mut self) {
        if self.ctx.chip.kind.tile_class_colbuf().is_none() {
            return;
        }
        if !self.gencfg.rows_colbuf.is_empty() {
            return;
        }
        let mut plb_bits = [const { None }; 8];
        let mut colbuf_map = BTreeMap::new();
        let harvester = self.harvester.get_mut().unwrap();
        for (key, bits) in &harvester.known_global {
            let DiffKey::GlobalRouting(wt, _wf) = key else {
                continue;
            };
            let Some(idx) = defs::wires::GLOBAL.index_of(wt.slot) else {
                continue;
            };
            assert_eq!(bits.len(), 1);
            let (&bit, &val) = bits.iter().next().unwrap();
            plb_bits[idx] = Some(BTreeMap::from_iter([(
                TileBit {
                    rect: BitRectId::from_idx(0),
                    frame: bit.1,
                    bit: bit.2,
                },
                val,
            )]));
            let BitOwner::Main(_, row) = bit.0 else {
                unreachable!()
            };
            colbuf_map.insert(wt.row, row);
        }
        if self.ctx.debug >= 3 {
            println!("COLBUF ROWS: {colbuf_map:?}");
        }
        if !plb_bits.iter().all(|x| x.is_some()) {
            return;
        }
        let Some(new_rows_colbuf) = self.compute_rows_colbuf(colbuf_map) else {
            return;
        };
        if self.ctx.debug >= 1 {
            println!("HEEEEEEY WE GOT COLBUFS!");
        }
        let harvester = self.harvester.get_mut().unwrap();
        self.gencfg.rows_colbuf = new_rows_colbuf;
        for col in self.edev.chip.columns() {
            if self.edev.chip.kind.has_ioi_we()
                && (col == self.edev.chip.col_w() || col == self.edev.chip.col_e())
            {
                continue;
            }
            if self.edev.chip.cols_bram.contains(&col) {
                continue;
            }
            for cell in self.edev.column(DieId::from_idx(0), col) {
                let row = cell.row;
                if row == self.edev.chip.row_s() || row == self.edev.chip.row_n() {
                    continue;
                }
                let (row_colbuf, _, _) = self
                    .gencfg
                    .rows_colbuf
                    .iter()
                    .copied()
                    .find(|&(_, row_b, row_t)| row >= row_b && row < row_t)
                    .unwrap();
                let trow = if row < row_colbuf {
                    if self.edev.chip.cols_bram.contains(&col)
                        && !self.edev.chip.kind.has_ice40_bramv2()
                    {
                        row_colbuf - 2
                    } else {
                        row_colbuf - 1
                    }
                } else {
                    row_colbuf
                };

                for (idx, bits) in plb_bits.iter().enumerate() {
                    let bits = bits.as_ref().unwrap();
                    let key = DiffKey::GlobalRouting(
                        cell.wire(defs::wires::GLOBAL[idx]),
                        cell.wire(defs::wires::GLOBAL_ROOT[idx]).pos(),
                    );
                    let bits = bits
                        .iter()
                        .map(|(&bit, &val)| ((BitOwner::Main(col, trow), bit.frame, bit.bit), val))
                        .collect();
                    harvester.force_global(key.clone(), bits);
                    harvester.known_global.remove(&key);
                }
            }
        }

        let tcid = self.edev.chip.kind.tile_class_colbuf().unwrap();
        for (idx, bits) in plb_bits.into_iter().enumerate() {
            harvester.force_tiled(
                DiffKey::Routing(
                    tcid,
                    TileWireCoord::new_idx(0, defs::wires::GLOBAL[idx]),
                    TileWireCoord::new_idx(0, defs::wires::GLOBAL_ROOT[idx]).pos(),
                ),
                bits.unwrap(),
            );
        }
        harvester.process();
    }

    fn pips_complete(&self) -> bool {
        let mut tiles_complete = 0;
        let pips = self.pips.lock().unwrap();
        for (&tcid, pips) in &*pips {
            let mut stats: BTreeMap<String, usize> = BTreeMap::new();
            for &(wt, wf) in pips {
                let bucket = if is_quad_v(wt.wire) && is_quad(wf.wire) {
                    "QUAD-QUAD.V"
                } else if is_quad_h(wt.wire) && is_quad(wf.wire) {
                    "QUAD-QUAD.H"
                } else if is_quad_v(wt.wire) && is_long(wf.wire) {
                    "LONG-QUAD.V"
                } else if is_quad_h(wt.wire) && is_long(wf.wire) {
                    "LONG-QUAD.H"
                } else if (is_quad_v(wt.wire) || is_quad_v_w(wt.wire))
                    && defs::wires::OUT_LC.contains(wf.wire)
                {
                    "OUT-QUAD.V"
                } else if is_quad_h(wt.wire) && defs::wires::OUT_LC.contains(wf.wire) {
                    "OUT-QUAD.H"
                } else if is_long_v(wt.wire) && is_long(wf.wire) {
                    "LONG-LONG.V"
                } else if is_long_h(wt.wire) && is_long(wf.wire) {
                    "LONG-LONG.H"
                } else if is_long_v(wt.wire) && defs::wires::OUT_LC.contains(wf.wire) {
                    "OUT-LONG.V"
                } else if is_long_h(wt.wire) && defs::wires::OUT_LC.contains(wf.wire) {
                    "OUT-LONG.H"
                } else {
                    self.edev.db.wires.key(wt.wire)
                };
                *stats.entry(bucket.to_string()).or_default() += 1;
            }
            let golden_stats = get_golden_mux_stats(self.edev.chip.kind, tcid);
            if stats == golden_stats {
                tiles_complete += 1;
            } else {
                for (k, &v) in &stats {
                    let gv = golden_stats.get(k).copied().unwrap_or(0);
                    if v > gv {
                        println!("UMMMM GOT MORE MUXES THAN BARGAINED FOR AT {tcid} {k} {v}/{gv}");
                    }
                }
                let mut missing = BTreeMap::new();
                for (k, &gv) in &golden_stats {
                    let v = stats.get(k).copied().unwrap_or(0);
                    if v < gv {
                        missing.insert(k, gv - v);
                    }
                }
                if self.ctx.debug >= 1 && !missing.is_empty() {
                    print!("missing muxes in {tcid}:");
                    for (k, v) in missing {
                        print!(" {v}×{k}");
                    }
                    println!();
                }
            }
        }
        let golden_tiles_complete = if self.edev.chip.kind == ChipKind::Ice40P03 {
            5 // PLB, 4×IO
        } else if self.edev.chip.kind.has_ioi_we() {
            6 // PLB, INT.BRAM, 4×IO
        } else {
            4 // PLB, INT.BRAM, 2×IO
        };
        golden_tiles_complete == tiles_complete
    }

    fn speed_complete(&mut self) -> bool {
        let mut res = true;
        for ((dev, sname), collector) in &mut self.speed {
            let collector = collector.get_mut().unwrap();
            for key in &collector.wanted_keys {
                if !collector.db.vals.contains_key(key) {
                    if self.ctx.debug >= 1 {
                        println!("WANTED SPEED DATA: {dev} {sname} {key}");
                    }
                    res = false;
                }
            }
        }
        res
    }

    fn new_sample(&self) -> Option<(String, Design, RunResult)> {
        let design = generate(&self.gencfg);
        let uniq: u128 = rand::rng().random();
        let prefix = if !self.gencfg.allow_global {
            "gen-noglobal"
        } else if self.ctx.chip.kind.tile_class_colbuf().is_some()
            && self.gencfg.rows_colbuf.is_empty()
        {
            "gen-nocolbuf"
        } else {
            "gen-full"
        };
        let key = format!("{prefix}-{uniq:032x}");
        match run(self.ctx.toolchain, &design, &key) {
            Ok(res) => Some((key, design, res)),
            Err(err) => {
                if self.ctx.debug >= 2 {
                    println!("OOPS {err:?}");
                }
                None
            }
        }
    }

    fn add_sample(&self, key: &str, design: Design, result: RunResult) -> bool {
        let speed = get_speed_data(&design, &result);
        if self.ctx.debug >= 2 {
            println!("SPEED MERGE {key}");
        }
        let mut changed = self.speed[&(design.device.as_str(), design.speed.as_str())]
            .lock()
            .unwrap()
            .merge(&speed.db);
        if matches!(
            design.device.as_str(),
            "iCE40LP640" | "iCE40HX640" | "iCE40LP4K" | "iCE40HX4K"
        ) {
            return changed;
        }
        let (sample, cur_pips) = make_sample(
            &design,
            self.edev,
            &result,
            &self.ctx.pkgs[&(design.device.as_str(), design.package.as_str())],
            &self.gencfg.rows_colbuf,
            &self.ctx.extra_wire_names,
            &self.ctx.special_tiles,
        );
        let mut harvester = self.harvester.lock().unwrap();
        let mut pips = self.pips.lock().unwrap();
        let mut ctr = 0;
        for pip in cur_pips {
            let wt = TileWireCoord::new_idx(0, pip.1);
            let wf = TileWireCoord::new_idx(0, pip.2);
            if pips.entry(pip.0).or_default().insert((wt, wf)) {
                ctr += 1;
                changed = true;
            }
        }
        if self.ctx.debug >= 2 {
            println!("{key} TOTAL NEW PIPS: {ctr} / {tot}", tot = pips.len());
        }
        drop(pips);
        if let Some(sid) = harvester.add_sample(sample) {
            if self.ctx.debug >= 2 {
                println!("SAMPLE {sid}: {key}");
            }
            changed = true;
        }
        changed
    }

    fn run(&mut self) {
        let mut ctr = AtomicU32::new(0);
        get_cached_designs(self.ctx.chip.kind, "gen-noglobal").for_each(|(key, design, result)| {
            self.add_sample(&key, design, result);
            let new_cnt = ctr.fetch_add(1, Ordering::Relaxed) + 1;
            if new_cnt.is_multiple_of(20) {
                self.harvester.lock().unwrap().process();
            }
        });
        let ctr = *ctr.get_mut();
        if ctr != 0 {
            self.harvester.get_mut().unwrap().process();
        }
        println!("{ctr} cached noglobal designs");
        if self.harvester.get_mut().unwrap().samples.len() < 40 {
            (0..40).into_par_iter().for_each(|_| {
                if let Some((key, design, result)) = self.new_sample() {
                    self.add_sample(&key, design, result);
                }
            });
            self.harvester.get_mut().unwrap().process();
        }
        self.gencfg.allow_global = true;
        let mut ctr = AtomicU32::new(0);
        get_cached_designs(self.ctx.chip.kind, "gen-nocolbuf").for_each(|(key, design, result)| {
            self.add_sample(&key, design, result);
            let new_cnt = ctr.fetch_add(1, Ordering::Relaxed) + 1;
            if new_cnt.is_multiple_of(100) {
                self.harvester.lock().unwrap().process();
            }
        });
        let ctr = *ctr.get_mut();
        if ctr != 0 {
            self.harvester.get_mut().unwrap().process();
        }
        println!("{ctr} cached nocolbuf designs");
        self.handle_colbufs();
        while self.ctx.chip.kind.tile_class_colbuf().is_some() && self.gencfg.rows_colbuf.is_empty()
        {
            (0..40).into_par_iter().for_each(|_| {
                if let Some((key, design, result)) = self.new_sample() {
                    self.add_sample(&key, design, result);
                }
            });
            self.harvester.get_mut().unwrap().process();
            self.handle_colbufs();
        }
        let mut ctr = AtomicU32::new(0);
        get_cached_designs(self.ctx.chip.kind, "gen-full").for_each(|(key, design, result)| {
            self.add_sample(&key, design, result);
            let new_cnt = ctr.fetch_add(1, Ordering::Relaxed) + 1;
            if new_cnt.is_multiple_of(100) {
                self.harvester.lock().unwrap().process();
            }
        });
        let ctr = *ctr.get_mut();
        if ctr != 0 {
            self.harvester.get_mut().unwrap().process();
        }
        println!("{ctr} cached full designs");
        while !self.speed_complete()
            || !self.pips_complete()
            || self.harvester.get_mut().unwrap().has_unresolved()
        {
            (0..40).into_par_iter().for_each(|_| {
                if let Some((key, design, result)) = self.new_sample()
                    && !self.add_sample(&key, design, result)
                {
                    remove_cache_key(self.ctx.chip.kind, &key);
                }
            });
            self.harvester.get_mut().unwrap().process();
        }
        println!("DONE with {}!", self.ctx.chip.kind);
    }
}

fn insert_tile_class(
    filled: &mut BTreeSet<TileClassId>,
    intdb: &mut IntDb,
    tcid: TileClassId,
    tcls: TileClass,
) {
    if filled.insert(tcid) {
        intdb.tile_classes[tcid] = tcls;
    } else {
        assert_eq!(intdb.tile_classes[tcid], tcls);
    }
}

impl PartContext<'_> {
    fn def_pkg(&self) -> (&'static str, &'static str) {
        (self.parts[0].name, self.parts[0].packages[0])
    }

    fn fill_sites(&mut self) {
        let toolchain = self.toolchain;
        let parts = &self.parts;
        let bel_info = Mutex::new(vec![]);
        let bel_info_ref = &bel_info;
        let prims_ref = &self.prims;
        let empty_runs = Mutex::new(BTreeMap::new());
        let empty_runs_ref = &empty_runs;
        rayon::scope(|s| {
            for &part in parts {
                s.spawn(move |_| {
                    let locs = find_sites_plb(toolchain, part);
                    let mut binfo = bel_info_ref.lock().unwrap();
                    binfo.push((part.name, part.packages[0], "PLB", locs));
                });
                for kind in [
                    "SB_GB",
                    "SB_RAM4K",
                    "SB_RAM40_4K",
                    "SB_RAM40_16K",
                    "SB_MAC16",
                    "SB_SPRAM256KA",
                    "SB_WARMBOOT",
                    "SB_SPI",
                    "SB_I2C",
                    "SB_FILTER_50NS",
                    "SB_HSOSC",
                    "SB_LSOSC",
                    "SB_HFOSC",
                    "SB_LFOSC",
                    "SB_LEDD_IP",
                    "SB_LEDDA_IP",
                    "SB_IR_IP",
                    "SB_I2C_FIFO",
                ] {
                    if !self.prims.contains_key(kind) {
                        continue;
                    }
                    s.spawn(move |_| {
                        let locs =
                            find_sites_misc(toolchain, prims_ref, part, part.packages[0], kind);
                        let mut binfo = bel_info_ref.lock().unwrap();
                        binfo.push((part.name, part.packages[0], kind, locs));
                    });
                }
                for &pkg in part.packages {
                    s.spawn(move |_| {
                        let design = Design::new(part, pkg, part.speeds[0], part.temps[0]);
                        empty_runs_ref.lock().unwrap().insert(
                            (part.name, pkg),
                            run(
                                toolchain,
                                &design,
                                &format!("empty-{dev}-{pkg}", dev = part.name),
                            )
                            .unwrap(),
                        );
                    });

                    for kind in [
                        "SB_IO",
                        "SB_IO_DS",
                        "SB_IO_DLY",
                        "SB_IO_I3C",
                        "SB_IO_OD",
                        "SB_GB_IO",
                        "SB_IR_DRV",
                        "SB_BARCODE_DRV",
                        "SB_IR400_DRV",
                        "SB_IR500_DRV",
                        "SB_RGB_DRV",
                        "SB_RGBA_DRV",
                        "SB_PLL_CORE",
                        "SB_PLL_PAD",
                        "SB_PLL_2_PAD",
                        "SB_PLL40_CORE",
                        "SB_PLL40_PAD",
                        "SB_PLL40_PAD_DS",
                        "SB_PLL40_2_PAD",
                        "SB_PLL40_2F_CORE",
                        "SB_PLL40_2F_PAD",
                        "SB_PLL40_2F_PAD_DS",
                        "SB_MIPI_RX_2LANE",
                        "SB_MIPI_TX_4LANE",
                        "SB_TMDS_deserializer",
                    ] {
                        if !self.prims.contains_key(kind) {
                            continue;
                        }
                        if kind.starts_with("SB_PLL")
                            && matches!(part.kind, ChipKind::Ice40M08 | ChipKind::Ice40M16)
                        {
                            continue;
                        }
                        s.spawn(move |_| {
                            let locs = find_sites_misc(toolchain, prims_ref, part, pkg, kind);
                            let mut binfo = bel_info_ref.lock().unwrap();
                            binfo.push((part.name, pkg, kind, locs));
                        });
                    }
                    s.spawn(move |_| {
                        let locs = find_sites_iox3(toolchain, part, pkg);
                        let mut binfo = bel_info_ref.lock().unwrap();
                        binfo.push((part.name, pkg, "IOx3", locs));
                    });
                }
            }
        });

        let empty_runs = empty_runs.into_inner().unwrap();
        let bel_info = bel_info.into_inner().unwrap();
        self.pkgs = empty_runs
            .into_iter()
            .map(|((dev, pkg), v)| {
                let part = self
                    .parts
                    .iter()
                    .copied()
                    .find(|part| part.name == dev)
                    .unwrap();
                (
                    (dev, pkg),
                    PkgInfo {
                        part,
                        bond: Bond {
                            pins: Default::default(),
                        },
                        empty_run: v,
                        bel_info: Default::default(),
                        xlat_col: Default::default(),
                        xlat_row: Default::default(),
                        xlat_ioi: Default::default(),
                    },
                )
            })
            .collect();
        for (dev, pkg, kind, locs) in bel_info {
            let pkg_info = self.pkgs.get_mut(&(dev, pkg)).unwrap();
            pkg_info.bel_info.insert(kind, locs);
        }
        for &part in &self.parts {
            for &pkg in part.packages {
                let (sdev, spkg) = if part.name == "iCE40LP640" && pkg == "SWG16TR" {
                    ("iCE40LP1K", "DI")
                } else {
                    (part.name, part.packages[0])
                };
                let sbels = self.pkgs[&(sdev, spkg)].bel_info.clone();
                let dbels = &mut self.pkgs.get_mut(&(part.name, pkg)).unwrap().bel_info;
                for (k, v) in sbels {
                    dbels.entry(k).or_insert(v);
                }
            }
        }
        self.chip.row_mid = RowId::from_idx(
            self.pkgs[&self.def_pkg()].empty_run.bitstream.cram[0]
                .frame_present
                .len()
                / 16,
        )
    }

    fn fill_xlat_rc(&mut self) {
        let mut first = true;
        let mut cols_bram = BTreeSet::new();
        for pkg_info in self.pkgs.values_mut() {
            let mut xlat_col = BTreeMap::new();
            let mut xlat_row = BTreeMap::new();
            let mut info_sets = vec![
                (
                    &pkg_info.bel_info["PLB"],
                    InstPin::Simple("Q".into()),
                    true,
                    false,
                ),
                (
                    &pkg_info.bel_info["SB_GB"],
                    InstPin::Simple("USER_SIGNAL_TO_GLOBAL_BUFFER".into()),
                    true,
                    false,
                ),
                (
                    &pkg_info.bel_info[if self.chip.kind.is_ice65() {
                        "SB_RAM4K"
                    } else {
                        "SB_RAM40_4K"
                    }],
                    InstPin::Indexed("RDATA".into(), 0),
                    false,
                    true,
                ),
            ];
            if pkg_info.bel_info.contains_key("SB_MAC16") {
                info_sets.push((
                    &pkg_info.bel_info["SB_MAC16"],
                    InstPin::Indexed("O".into(), 0),
                    false,
                    false,
                ));
            }
            if pkg_info.bel_info.contains_key("SB_LSOSC") {
                info_sets.push((
                    &pkg_info.bel_info["SB_LSOSC"],
                    InstPin::Simple("CLKK".into()),
                    false,
                    false,
                ));
                info_sets.push((
                    &pkg_info.bel_info["SB_HSOSC"],
                    InstPin::Simple("CLKM".into()),
                    false,
                    false,
                ));
            }

            for (infos, pin, do_y, is_ram) in info_sets {
                for info in infos {
                    let (col, row, _) = *info
                        .out_wires
                        .get(&pin)
                        .unwrap_or_else(|| &info.in_wires[&pin]);
                    let col = ColId::from_idx(col.try_into().unwrap());
                    match xlat_col.entry(info.loc.x) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(col);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), col);
                        }
                    }
                    if is_ram {
                        cols_bram.insert(col);
                    }
                    if do_y {
                        let row = RowId::from_idx(row.try_into().unwrap());
                        match xlat_row.entry(info.loc.y) {
                            btree_map::Entry::Vacant(e) => {
                                e.insert(row);
                            }
                            btree_map::Entry::Occupied(e) => {
                                assert_eq!(*e.get(), row);
                            }
                        }
                    }
                }
            }
            let mut columns = xlat_col.values().max().unwrap().to_idx() + 1;
            let rows = xlat_row.values().max().unwrap().to_idx() + 1;

            // iCE5LP1K fixup.
            if self.chip.kind == ChipKind::Ice40T04 {
                columns = 26;
            }

            if first {
                self.chip.columns = columns;
                self.chip.rows = rows;
            } else {
                assert_eq!(self.chip.columns, columns);
                assert_eq!(self.chip.rows, rows);
            }

            for (i, (&j, _)) in xlat_col.iter().enumerate() {
                assert_eq!(i, usize::try_from(j).unwrap());
            }
            for (i, (&j, _)) in xlat_row.iter().enumerate() {
                assert_eq!(i, usize::try_from(j).unwrap());
            }
            pkg_info.xlat_col = xlat_col.into_values().collect();
            pkg_info.xlat_row = xlat_row.into_values().collect();

            first = false;
        }
        self.chip.cols_bram = Vec::from_iter(cols_bram);
    }

    fn fill_cfg_io(&mut self) {
        let mut special = SpecialTile {
            io: Default::default(),
            cells: Default::default(),
        };
        let col_base = if matches!(self.chip.kind, ChipKind::Ice65L08 | ChipKind::Ice40P08) {
            self.chip.col_e() - 4
        } else {
            self.chip.col_e() - 3
        };
        for (dx, idx, key) in [
            (0, 0, SpecialIoKey::CbSel0),
            (0, 1, SpecialIoKey::CbSel1),
            (1, 0, SpecialIoKey::CfgSdo(0)),
            (1, 1, SpecialIoKey::CfgSdi(0)),
            (2, 0, SpecialIoKey::CfgSck),
            (2, 1, SpecialIoKey::CfgCsB),
        ] {
            let ioi = CellCoord::new(DieId::from_idx(0), col_base + dx, self.chip.row_s())
                .bel(defs::bslots::IOI[idx]);
            special.io.insert(key, ioi);
        }
        if matches!(
            self.chip.kind,
            ChipKind::Ice40P01 | ChipKind::Ice40P08 | ChipKind::Ice40P03
        ) {
            for (dy, idx, key) in [
                (1, 0, SpecialIoKey::JtagTdi),
                (1, 1, SpecialIoKey::JtagTms),
                (2, 0, SpecialIoKey::JtagTck),
                (2, 1, SpecialIoKey::JtagTdo),
            ] {
                let ioi = CellCoord::new(
                    DieId::from_idx(0),
                    self.chip.col_e(),
                    self.chip.row_s() + dy,
                )
                .bel(defs::bslots::IOI[idx]);
                special.io.insert(key, ioi);
            }
        }
        if matches!(
            self.chip.kind,
            ChipKind::Ice65P04 | ChipKind::Ice65L04 | ChipKind::Ice65L08 | ChipKind::Ice40P08
        ) {
            if matches!(self.chip.kind, ChipKind::Ice65L04 | ChipKind::Ice65P04) {
                for (dx, idx, key) in [
                    (-6, 0, SpecialIoKey::CfgSdo(2)),
                    (-6, 1, SpecialIoKey::CfgSdo(3)),
                    (-5, 0, SpecialIoKey::CfgSdo(4)),
                    (-5, 1, SpecialIoKey::CfgSdo(5)),
                    (-4, 0, SpecialIoKey::CfgSdo(6)),
                    (-4, 1, SpecialIoKey::CfgSdo(7)),
                ] {
                    let ioi = CellCoord::new(
                        DieId::from_idx(0),
                        self.chip.col_e() + dx,
                        self.chip.row_s(),
                    )
                    .bel(defs::bslots::IOI[idx]);
                    special.io.insert(key, ioi);
                }
            } else {
                for (dx, idx, key) in [
                    (-8, 1, SpecialIoKey::CfgSdo(7)),
                    (-7, 0, SpecialIoKey::CfgSdo(6)),
                    (-7, 1, SpecialIoKey::CfgSdo(5)),
                    (-6, 0, SpecialIoKey::CfgSdo(4)),
                    (-6, 1, SpecialIoKey::CfgSdo(3)),
                    (-5, 0, SpecialIoKey::CfgSdo(2)),
                ] {
                    let ioi = CellCoord::new(
                        DieId::from_idx(0),
                        self.chip.col_e() + dx,
                        self.chip.row_s(),
                    )
                    .bel(defs::bslots::IOI[idx]);
                    special.io.insert(key, ioi);
                }
            }
            let row_base: RowId = match self.chip.kind {
                ChipKind::Ice65L04 | ChipKind::Ice65P04 => self.chip.row_s() + 1,
                ChipKind::Ice65L08 | ChipKind::Ice40P08 => self.chip.row_s() + 3,
                _ => unreachable!(),
            };
            for (dy, idx, key) in [
                (0, 0, SpecialIoKey::CfgSdo(1)),
                (0, 1, SpecialIoKey::CfgSdi(1)),
                (1, 0, SpecialIoKey::CfgSdi(2)),
                (1, 1, SpecialIoKey::CfgSdi(3)),
                (2, 0, SpecialIoKey::CfgSdi(4)),
                (2, 1, SpecialIoKey::CfgSdi(5)),
                (3, 0, SpecialIoKey::CfgSdi(6)),
                (3, 1, SpecialIoKey::CfgSdi(7)),
            ] {
                let ioi = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), row_base + dy)
                    .bel(defs::bslots::IOI[idx]);
                special.io.insert(key, ioi);
            }
        }
        self.chip
            .special_tiles
            .insert(SpecialTileKey::Globals, special);
    }

    fn fill_bonds(&mut self) {
        for (&(dev, pkg), pkg_info) in &mut self.pkgs {
            for info in &pkg_info.bel_info["SB_IO"] {
                let (col, row, ref wn) = info.in_wires[&InstPin::Simple("D_OUT_0".into())];
                let cell = CellCoord::new(
                    DieId::from_idx(0),
                    ColId::from_idx(col.try_into().unwrap()),
                    RowId::from_idx(row.try_into().unwrap()),
                );
                let idx = if wn == "wire_io_cluster/io_0/D_OUT_0" {
                    0
                } else if wn == "wire_io_cluster/io_1/D_OUT_0" {
                    1
                } else {
                    panic!("ummm {wn}?")
                };
                let (loc, ref pin) = info.pads["PACKAGE_PIN"];
                let xy = (loc.x, loc.y, loc.bel);
                assert_eq!(loc, info.loc);
                let ioi = cell.bel(defs::bslots::IOI[idx]);
                // will be fixed up later.
                let iob = cell.bel(defs::bslots::IOB[idx]);
                self.chip.ioi_iob.insert(ioi, iob);
                assert_eq!(
                    pkg_info
                        .bond
                        .pins
                        .insert(pin.clone(), vec![iob.pad(defs::bcls::IOB::PAD)]),
                    None
                );
                match pkg_info.xlat_ioi.entry(xy) {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(ioi);
                    }
                    btree_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), ioi);
                    }
                }
            }
            if self.chip.kind.is_ice65() {
                for info in &pkg_info.bel_info["SB_IO_DS"] {
                    for pin in ["PACKAGE_PIN", "PACKAGE_PIN_B"] {
                        let (loc, ref pin) = info.pads[pin];
                        let cell = CellCoord::new(
                            DieId::from_idx(0),
                            ColId::from_idx(loc.x.try_into().unwrap()),
                            RowId::from_idx(loc.y.try_into().unwrap()),
                        );
                        let idx: usize = loc.bel.try_into().unwrap();
                        let ioi = cell.bel(defs::bslots::IOI[idx]);
                        // will be fixed up later.
                        let iob = cell.bel(defs::bslots::IOB[idx]);
                        self.chip.ioi_iob.insert(ioi, iob);
                        assert_eq!(
                            pkg_info
                                .bond
                                .pins
                                .insert(pin.clone(), vec![iob.pad(defs::bcls::IOB::PAD)]),
                            None
                        );
                    }
                }
            }
            if let Some(infos) = pkg_info.bel_info.get("SB_IO_OD") {
                for info in infos {
                    let (col, row, ref wn) = info.in_wires[&InstPin::Simple("DOUT0".into())];
                    let cell = CellCoord::new(
                        DieId::from_idx(0),
                        ColId::from_idx(col.try_into().unwrap()),
                        RowId::from_idx(row.try_into().unwrap()),
                    );
                    let idx = if wn == "wire_io_cluster/io_0/D_OUT_0" {
                        0
                    } else if wn == "wire_io_cluster/io_1/D_OUT_0" {
                        1
                    } else {
                        panic!("ummm {wn}?")
                    };
                    let ioi = cell.bel(defs::bslots::IOI[idx]);
                    // will be fixed up later.
                    let iob = cell.bel(defs::bslots::IOB[idx]);
                    self.chip.ioi_iob.insert(ioi, iob);
                    self.chip.iob_od.insert(iob);
                    let (loc, ref pin) = info.pads["PACKAGEPIN"];
                    let xy = (loc.x, loc.y, loc.bel);
                    assert_eq!(loc, info.loc);
                    assert_eq!(
                        pkg_info
                            .bond
                            .pins
                            .insert(pin.clone(), vec![iob.pad(defs::bcls::IOB::PAD)]),
                        None
                    );
                    match pkg_info.xlat_ioi.entry(xy) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(ioi);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), ioi);
                        }
                    }
                }
            }
            if matches!(dev, "iCE65L04" | "iCE65P04") && pkg == "CB132" {
                // AAAAAAAAAAAAAAAAAAAaaaaaaaaaaaa
                for (row, idx, pin) in [(11, 0, "G1"), (10, 1, "H1")] {
                    let cell = CellCoord::new(
                        DieId::from_idx(0),
                        self.chip.col_w(),
                        RowId::from_idx(row.try_into().unwrap()),
                    );
                    let ioi = cell.bel(defs::bslots::IOI[idx]);
                    let iob = cell.bel(defs::bslots::IOB[idx]);
                    self.chip.ioi_iob.insert(ioi, iob);
                    pkg_info
                        .bond
                        .pins
                        .insert(pin.into(), vec![iob.pad(defs::bcls::IOB::PAD)]);
                }
            }
            if self.chip.kind.is_ice65() {
                for &ioi in self.chip.ioi_iob.keys_left() {
                    let idx = defs::bslots::IOI.index_of(ioi.slot).unwrap();
                    let xy = (
                        ioi.col.to_idx().try_into().unwrap(),
                        ioi.row.to_idx().try_into().unwrap(),
                        idx.try_into().unwrap(),
                    );
                    match pkg_info.xlat_ioi.entry(xy) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(ioi);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), ioi);
                        }
                    }
                }
            }
            for (pin, info) in &pkg_info.empty_run.pin_table {
                let typ = &info.typ[..];
                let pad = match typ {
                    "GND" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::POWER)
                            .pad(defs::bcls::POWER::GND),
                    ],
                    "VCC" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::POWER)
                            .pad(defs::bcls::POWER::VCCINT),
                    ],
                    "VCCIO_0" => {
                        if self.chip.kind == ChipKind::Ice40T01 && pkg == "SWG16" {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[0])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::CONFIG)
                                    .pad(defs::bcls::CONFIG::VPP_2V5),
                            ]
                        } else if self.chip.kind == ChipKind::Ice40P01
                            && matches!(pkg, "CM36" | "CM49")
                        {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[0])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[1])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                            ]
                        } else {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[0])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                            ]
                        }
                    }
                    "VCCIO_1" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::IO_BANK[1])
                            .pad(defs::bcls::IO_BANK::VCCIO),
                    ],
                    "VCCIO_2" => {
                        if (self.chip.kind == ChipKind::Ice40T01 && pkg == "SWG16")
                            || (self.chip.kind == ChipKind::Ice40T04 && pkg == "UWG20")
                            || (self.chip.kind == ChipKind::Ice40R04
                                && matches!(pkg, "SWG25TR" | "CM36" | "FC36"))
                        {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[2])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK_SPI)
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                            ]
                        } else if (self.chip.kind == ChipKind::Ice40P03
                            && matches!(pkg, "CM36" | "CM49"))
                            || (self.chip.kind == ChipKind::Ice40P01
                                && matches!(pkg, "CM36A" | "CM49A" | "CM36" | "CM49" | "CY36"))
                        {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[2])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[3])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                            ]
                        } else {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[2])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                            ]
                        }
                    }
                    "VCCIO_3" => {
                        if self.chip.kind == ChipKind::Ice40P01 && pkg == "SWG16TR" {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[1])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[2])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[3])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK_SPI)
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::CONFIG)
                                    .pad(defs::bcls::CONFIG::VPP_2V5),
                            ]
                        } else {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::IO_BANK[3])
                                    .pad(defs::bcls::IO_BANK::VCCIO),
                            ]
                        }
                    }
                    "VDDIO_SPI" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::IO_BANK_SPI)
                            .pad(defs::bcls::IO_BANK::VCCIO),
                    ],
                    "VPP_DIRECT" | "VPP" => {
                        vec![self.chip.globals().bel(defs::bslots::CONFIG).pad(
                            if self.chip.kind.is_ultra() {
                                defs::bcls::CONFIG::VPP_2V5
                            } else {
                                defs::bcls::CONFIG::VPP_FAST
                            },
                        )]
                    }
                    "VPP_PUMP" | "VDDP" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::VPP_2V5),
                    ],
                    "VREF" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::IO_BANK[3])
                            .pad(defs::bcls::IO_BANK::VREF),
                    ],
                    "VSSIO_LED" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::LED_DRV_CUR)
                            .pad(defs::bcls::LED_DRV_CUR::GND_LED),
                    ],
                    "AGND" | "AGND_BOT" => {
                        if self.chip.kind == ChipKind::Ice40P01 && pkg == "SWG16TR" {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::POWER)
                                    .pad(defs::bcls::POWER::GND),
                            ]
                        } else {
                            vec![
                                self.chip
                                    .bel_pll(DirV::S)
                                    .pad(if self.chip.kind.is_ice65() {
                                        defs::bcls::PLL65::AGND
                                    } else {
                                        defs::bcls::PLL40::AGND
                                    }),
                            ]
                        }
                    }
                    "AVDD" | "AVDD_BOT" => {
                        if self.chip.kind == ChipKind::Ice40P01 && pkg == "CB81" {
                            vec![
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::POWER)
                                    .pad(defs::bcls::POWER::VCCINT),
                            ]
                        } else {
                            vec![
                                self.chip
                                    .bel_pll(DirV::S)
                                    .pad(if self.chip.kind.is_ice65() {
                                        defs::bcls::PLL65::AVCC
                                    } else {
                                        defs::bcls::PLL40::AVCC
                                    }),
                            ]
                        }
                    }
                    "AGND_TOP" => {
                        vec![self.chip.bel_pll(DirV::N).pad(defs::bcls::PLL40::AGND)]
                    }
                    "AVDD_TOP" => {
                        vec![self.chip.bel_pll(DirV::N).pad(defs::bcls::PLL40::AVCC)]
                    }
                    "CRESET_B" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::CRESET_B),
                    ],
                    "CDONE" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::CDONE),
                    ],
                    "TCK" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::TCK),
                    ],
                    "TMS" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::TMS),
                    ],
                    "TDI" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::TDI),
                    ],
                    "TDO" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::TDO),
                    ],
                    "TRST_B" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::TRST_B),
                    ],
                    "POR_test" => vec![
                        self.chip
                            .globals()
                            .bel(defs::bslots::CONFIG)
                            .pad(defs::bcls::CONFIG::POR_TEST),
                    ],
                    "NC" => vec![],
                    "PIO" | "PIO_GBIN" | "PIO_GBIN_CDONE" | "PIO_LED" | "PIO_RGB"
                    | "PIO_BARCODE" | "PIO_I3C" => {
                        let pads = pkg_info.bond.pins.get_mut(pin).unwrap();
                        assert_eq!(pads.len(), 1);
                        if typ == "PIO_GBIN_CDONE" {
                            pads.push(
                                self.chip
                                    .globals()
                                    .bel(defs::bslots::CONFIG)
                                    .pad(defs::bcls::CONFIG::CDONE),
                            );
                        }
                        continue;
                    }
                    "SPI_SCK" | "SPI_SI" | "SPI_SO" | "SPI_SS_B" => {
                        let pads = &pkg_info.bond.pins[pin];
                        assert_eq!(pads.len(), 1);
                        let ioi = self.chip.iob_to_ioi(pads[0].bel).unwrap();
                        let cpin = match typ {
                            "SPI_SCK" => SpecialIoKey::CfgSck,
                            "SPI_SI" => SpecialIoKey::CfgSdi(0),
                            "SPI_SO" => SpecialIoKey::CfgSdo(0),
                            "SPI_SS_B" => SpecialIoKey::CfgCsB,
                            _ => unreachable!(),
                        };
                        assert_eq!(
                            self.chip.special_tiles[&SpecialTileKey::Globals].io[&cpin],
                            ioi
                        );
                        continue;
                    }
                    _ => panic!("ummm {}", info.typ),
                };
                assert_eq!(pkg_info.bond.pins.insert(pin.clone(), pad), None);
            }
            let mut x3 = BTreeMap::new();
            for info in &pkg_info.bel_info["IOx3"] {
                let xy = (info.loc.x, info.loc.y, info.loc.bel);
                let ioi = pkg_info.xlat_ioi[&xy];
                let iob = self.chip.ioi_to_iob(ioi).unwrap();
                let r0 = info.dedio["REP0"];
                let ior0 = CellCoord::new(
                    DieId::from_idx(0),
                    ColId::from_idx(r0.x as usize),
                    RowId::from_idx(r0.y as usize),
                )
                .bel(bels::IOB[r0.bel as usize])
                .pad(defs::bcls::IOB::PAD);
                let r1 = info.dedio["REP1"];
                let ior1 = CellCoord::new(
                    DieId::from_idx(0),
                    ColId::from_idx(r1.x as usize),
                    RowId::from_idx(r1.y as usize),
                )
                .bel(bels::IOB[r1.bel as usize])
                .pad(defs::bcls::IOB::PAD);
                x3.insert(iob, (ior0, ior1));
            }
            for pads in pkg_info.bond.pins.values_mut() {
                if pads.len() != 1 {
                    continue;
                }
                if let Some(&(ior0, ior1)) = x3.get(&pads[0].bel) {
                    let mut ior = [ior0, ior1];
                    ior.sort();
                    pads.extend(ior);
                }
            }
            if pkg == "SG48" {
                pkg_info.bond.pins.insert(
                    "PAD".to_string(),
                    if self.chip.kind == ChipKind::Ice40T04 {
                        vec![
                            self.chip
                                .globals()
                                .bel(defs::bslots::POWER)
                                .pad(defs::bcls::POWER::GND),
                            self.chip
                                .globals()
                                .bel(defs::bslots::LED_DRV_CUR)
                                .pad(defs::bcls::LED_DRV_CUR::GND_LED),
                        ]
                    } else {
                        vec![
                            self.chip
                                .globals()
                                .bel(defs::bslots::POWER)
                                .pad(defs::bcls::POWER::GND),
                        ]
                    },
                );
            }
            if pkg != "DI" {
                let all_pins = get_pkg_pins(pkg);
                for pin in &all_pins {
                    if let btree_map::Entry::Vacant(e) = pkg_info.bond.pins.entry(pin.to_string()) {
                        e.insert(vec![]);
                    }
                }
                assert_eq!(pkg_info.bond.pins.len(), all_pins.len());
            }
        }
        if self.chip.kind == ChipKind::Ice40P03 {
            let cell = CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(3));
            let ioi = cell.bel(defs::bslots::IOI[0]);
            let iob = cell.bel(defs::bslots::IOB[0]);
            self.chip.ioi_iob.insert(ioi, iob);
        }
        self.chip.col_bio_split = match self.chip.kind {
            ChipKind::Ice40T04 | ChipKind::Ice40T05 => ColId::from_idx(12),
            _ => self.chip.special_tiles[&SpecialTileKey::Globals].io[&SpecialIoKey::CfgSdo(0)].col,
        };
    }

    fn fill_cbsel(&mut self) {
        if !self.chip.kind.has_iob_we() {
            return;
        }
        for (&(_dev, pkg), pkg_info) in &self.pkgs {
            let balls = match pkg {
                "CB132" => [(SpecialIoKey::CbSel0, "L9"), (SpecialIoKey::CbSel1, "P10")],
                "CM36" | "CM36A" => [(SpecialIoKey::CbSel0, "E3"), (SpecialIoKey::CbSel1, "F3")],
                "CM49" => [(SpecialIoKey::CbSel0, "F4"), (SpecialIoKey::CbSel1, "G4")],
                "CB81" | "CM81" => [(SpecialIoKey::CbSel0, "G5"), (SpecialIoKey::CbSel1, "H5")],
                "CB121" => [(SpecialIoKey::CbSel0, "H6"), (SpecialIoKey::CbSel1, "J6")],
                "VQ100" => [(SpecialIoKey::CbSel0, "41"), (SpecialIoKey::CbSel1, "42")],
                _ => continue,
            };
            for (cpin, ball) in balls {
                let pads = &pkg_info.bond.pins[ball];
                assert_eq!(pads.len(), 1);
                let ioi = self.chip.iob_to_ioi(pads[0].bel).unwrap();
                assert_eq!(
                    self.chip.special_tiles[&SpecialTileKey::Globals].io[&cpin],
                    ioi,
                );
            }
        }
    }

    fn fill_io_latch(&mut self) {
        let (&(_dev, pkg), pkg_info) = self
            .pkgs
            .iter()
            .max_by_key(|(_, pkg_info)| {
                pkg_info
                    .bond
                    .pins
                    .values()
                    .filter(|pads| !pads.is_empty() && defs::bslots::IOB.contains(pads[0].slot))
                    .count()
            })
            .unwrap();
        let mut pkg_pins = DirPartMap::new();
        for (pin, pads) in &pkg_info.bond.pins {
            if pads.is_empty() {
                continue;
            }
            if !defs::bslots::IOB.contains(pads[0].slot) {
                continue;
            }
            if self.chip.iob_od.contains(&pads[0].bel) {
                continue;
            }
            let edge = self.chip.get_io_edge(pads[0].bel);
            if !pkg_pins.contains_key(edge) {
                pkg_pins.insert(edge, pin.as_str());
            }
        }
        let expected = if self.chip.kind.has_ioi_we() && self.chip.kind != ChipKind::Ice40R04 {
            4
        } else {
            2
        };
        assert_eq!(pkg_pins.iter().count(), expected);
        for (edge, (x, y)) in find_io_latch_locs(self.toolchain, pkg_info.part, pkg, &pkg_pins) {
            self.chip.special_tiles.insert(
                SpecialTileKey::LatchIo(edge),
                SpecialTile {
                    io: Default::default(),
                    cells: EntityVec::from_iter([CellCoord::new(
                        DieId::from_idx(0),
                        ColId::from_idx(x as usize),
                        RowId::from_idx(y as usize),
                    )]),
                },
            );
        }
        if self.chip.kind == ChipKind::Ice40R04 {
            for edge in DirH::DIRS {
                self.chip.special_tiles.insert(
                    SpecialTileKey::LatchIo(edge.into()),
                    SpecialTile {
                        io: Default::default(),
                        cells: EntityVec::from_iter([CellCoord::new(
                            DieId::from_idx(0),
                            self.chip.col_edge(edge),
                            RowId::from_idx(12),
                        )]),
                    },
                );
            }
        }
    }

    fn fill_bel_pins(&mut self) {
        let mut worklist = BTreeMap::new();
        let defdev = self.parts[0].name;
        let defpkg = self.parts[0].packages[0];
        for (&(dev, pkg), pkg_info) in &self.pkgs {
            for (&kind, sites) in &pkg_info.bel_info {
                if kind.starts_with("SB_RAM") {
                    continue;
                }
                if matches!(
                    kind,
                    "PLB" | "SB_GB" | "SB_IO" | "SB_IO_DS" | "SB_IO_OD" | "SB_GB_IO" | "IOx3"
                ) {
                    continue;
                }
                if !(kind.contains("PLL") || kind.contains("DRV") || kind.contains("_IO"))
                    && (dev, pkg) != (defdev, defpkg)
                {
                    continue;
                }
                for site in sites {
                    worklist.insert((kind, site.loc), (dev, pkg, site));
                }
            }
        }
        let edev = self.chip.expand_grid(&self.intdb);
        let db = if edev.chip.kind.has_iob_we() {
            None
        } else {
            Some(Database::from_file("db/icecube/ice40p01.zstd").unwrap())
        };
        let p01_db = db.as_ref().map(|x| &x.int);
        let extra_wire_names = Mutex::new(BTreeMap::new());
        let bel_pins = Mutex::new(BTreeMap::new());
        worklist
            .into_par_iter()
            .for_each(|((kind, _), (dev, pkg, site))| {
                let mut pins = find_bel_pins(
                    self.toolchain,
                    &self.prims,
                    self.pkgs[&(dev, pkg)].part,
                    &edev,
                    p01_db,
                    pkg,
                    kind,
                    site,
                );
                let mut extra_wire_names = extra_wire_names.lock().unwrap();
                for (wn, iw) in std::mem::take(&mut pins.wire_names) {
                    match extra_wire_names.entry(wn) {
                        btree_map::Entry::Vacant(entry) => {
                            entry.insert(iw);
                        }
                        btree_map::Entry::Occupied(entry) => {
                            assert_eq!(*entry.get(), iw);
                        }
                    }
                }
                std::mem::drop(extra_wire_names);
                let mut bel_pins = bel_pins.lock().unwrap();
                bel_pins.insert((kind, site.loc), pins);
            });
        self.extra_wire_names = extra_wire_names.into_inner().unwrap();
        if self.chip.kind == ChipKind::Ice40T01 {
            let wire = self.extra_wire_names[&(13, 15, "wire_ir400_drv/IRLEDEN".into())];
            self.extra_wire_names
                .insert((13, 15, "wire_ir500_drv/IRLEDEN".into()), wire);
            let wire = self.extra_wire_names[&(13, 15, "wire_ir400_drv/IRPWM".into())];
            self.extra_wire_names
                .insert((13, 15, "wire_ir500_drv/IRPWM".into()), wire);
            let wire = self.extra_wire_names[&(13, 15, "wire_bc_drv/BARCODEEN".into())];
            self.extra_wire_names
                .insert((13, 15, "wire_ir500_drv/IRLEDEN2".into()), wire);
            let wire = self.extra_wire_names[&(13, 15, "wire_bc_drv/BARCODEPWM".into())];
            self.extra_wire_names
                .insert((13, 15, "wire_ir500_drv/IRPWM2".into()), wire);
        }
        self.bel_pins = bel_pins.into_inner().unwrap();
    }

    fn fill_gbin_fabric(&mut self) {
        let sb_gb = &self.pkgs[&self.def_pkg()].bel_info["SB_GB"];
        let mut cells = [None; 8];
        for site in sb_gb {
            let (x, y) = site.fabout_wires[&InstPin::Simple("USER_SIGNAL_TO_GLOBAL_BUFFER".into())];
            let cell = CellCoord::new(
                DieId::from_idx(0),
                ColId::from_idx(x as usize),
                RowId::from_idx(y as usize),
            );
            let index = site.global_nets[&InstPin::Simple("GLOBAL_BUFFER_OUTPUT".into())] as usize;
            assert_eq!(cells[index], None);
            cells[index] = Some(cell);
        }
        let cells = EntityVec::from_iter(cells.into_iter().map(Option::unwrap));
        self.chip.special_tiles.insert(
            SpecialTileKey::GbRoot,
            SpecialTile {
                io: Default::default(),
                cells,
            },
        );
    }

    fn fill_gbin_io(&mut self) {
        let mut gb_io = BTreeMap::new();
        for pkg_info in self.pkgs.values() {
            let sb_gb_io = &pkg_info.bel_info["SB_GB_IO"];
            for site in sb_gb_io {
                let index =
                    site.global_nets[&InstPin::Simple("GLOBAL_BUFFER_OUTPUT".into())] as usize;
                let xy = (site.loc.x, site.loc.y, site.loc.bel);
                let io = pkg_info.xlat_ioi[&xy];
                match gb_io.entry(index) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(io);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), io);
                    }
                }
            }
        }

        if self.chip.kind.is_ice65() {
            // sigh.
            if !gb_io.contains_key(&1) {
                let ioi = gb_io[&0];
                gb_io.insert(1, ioi.with_col(self.chip.col_w()).bel(ioi.slot));
            }
            if !gb_io.contains_key(&4) {
                let ioi = gb_io[&5];
                gb_io.insert(4, ioi.with_col(self.chip.col_w()).bel(ioi.slot));
            }
        }

        if self.chip.kind.has_iob_we() {
            for i in 0..8 {
                assert!(gb_io.contains_key(&i));
            }
        } else {
            for i in [0, 1, 2, 3, 6, 7] {
                assert!(gb_io.contains_key(&i));
            }
            for i in [4, 5] {
                assert!(!gb_io.contains_key(&i));
            }
        }

        for (index, io) in gb_io {
            let special = self
                .chip
                .special_tiles
                .get_mut(&SpecialTileKey::GbRoot)
                .unwrap();
            special.io.insert(SpecialIoKey::GbIn(index), io);
        }
    }

    fn fill_mac16(&mut self) {
        let pkg_info = &self.pkgs[&self.def_pkg()];
        let Some(sites) = pkg_info.bel_info.get("SB_MAC16") else {
            return;
        };
        let mut rows_mac16_w = BTreeSet::new();
        let mut rows_mac16_e = BTreeSet::new();
        for site in sites {
            let col = pkg_info.xlat_col[site.loc.x as usize];
            let row = pkg_info.xlat_row[site.loc.y as usize];
            if col == self.chip.col_w() {
                rows_mac16_w.insert(row);
            } else if col == self.chip.col_e() {
                rows_mac16_e.insert(row);
            } else {
                unreachable!()
            }
            let tcid = if self.chip.kind == ChipKind::Ice40T05
                && col == self.chip.col_w()
                && row.to_idx() == 15
            {
                defs::tcls::MAC16_TRIM
            } else {
                defs::tcls::MAC16
            };
            let bel_pins = &self.bel_pins[&("SB_MAC16", site.loc)];
            let cells: [_; 5] = CellCoord::new(DieId::from_idx(0), col, row).cells_n_const();
            let mut builder = MiscTileBuilder::new(&self.intdb, tcid, &cells);
            builder.add_bel(defs::bslots::MAC16, bel_pins);
            let (tcls, _special) = builder.finish();
            insert_tile_class(&mut self.tcls_filled, &mut self.intdb, tcid, tcls);
        }
        assert_eq!(rows_mac16_w, rows_mac16_e);
        self.chip.rows_mac16 = Vec::from_iter(rows_mac16_w);
    }

    fn fill_hardip(&mut self) {
        for kind in ["SB_SPI", "SB_I2C", "SB_I2C_FIFO", "SB_LSOSC", "SB_HSOSC"] {
            let pkg_info = &self.pkgs[&self.def_pkg()];
            let Some(sites) = pkg_info.bel_info.get(kind) else {
                continue;
            };
            for site in sites {
                let mut cells = vec![];
                let (key, bel, dedio) = match kind {
                    "SB_SPI" => {
                        let edge = if site.loc.x == 0 { DirH::W } else { DirH::E };
                        if self.chip.kind == ChipKind::Ice40R04 {
                            cells = CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_edge(edge),
                                RowId::from_idx(1),
                            )
                            .cells_n(10);
                            let cols = match edge {
                                DirH::W => [1, 2, 3, 4, 9],
                                DirH::E => [15, 16, 17, 18, 20],
                            };
                            for c in cols {
                                cells.push(CellCoord::new(
                                    DieId::from_idx(0),
                                    ColId::from_idx(c),
                                    self.chip.row_s(),
                                ));
                            }
                        } else {
                            cells = CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_edge(edge),
                                match self.chip.kind {
                                    ChipKind::Ice40T04 => RowId::from_idx(1),
                                    ChipKind::Ice40T05 => RowId::from_idx(19),
                                    _ => unreachable!(),
                                },
                            )
                            .cells_n(4);
                        }
                        (
                            SpecialTileKey::Spi(edge),
                            bels::SPI,
                            [
                                (SpecialIoKey::SpiCopi, "MOSI"),
                                (SpecialIoKey::SpiCipo, "MISO"),
                                (SpecialIoKey::SpiSck, "SCK"),
                                (SpecialIoKey::SpiCsB0, "CSN0"),
                                (SpecialIoKey::SpiCsB1, "CSN1"),
                            ]
                            .as_slice(),
                        )
                    }
                    "SB_I2C" => {
                        let edge = if site.loc.x == 0 { DirH::W } else { DirH::E };
                        if self.chip.kind == ChipKind::Ice40R04 {
                            cells = CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_edge(edge),
                                RowId::from_idx(11),
                            )
                            .cells_n(10);
                            let cols = match edge {
                                DirH::W => [2, 3],
                                DirH::E => [10, 11],
                            };
                            for c in cols {
                                cells.push(CellCoord::new(
                                    DieId::from_idx(0),
                                    ColId::from_idx(c),
                                    self.chip.row_n(),
                                ));
                            }
                        } else {
                            cells = CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_edge(edge),
                                match self.chip.kind {
                                    ChipKind::Ice40T04 => RowId::from_idx(19),
                                    ChipKind::Ice40T05 => RowId::from_idx(29),
                                    _ => unreachable!(),
                                },
                            )
                            .cells_n(2);
                        }
                        (
                            SpecialTileKey::I2c(edge),
                            bels::I2C,
                            [(SpecialIoKey::I2cScl, "SCL"), (SpecialIoKey::I2cSda, "SDA")]
                                .as_slice(),
                        )
                    }
                    "SB_I2C_FIFO" => {
                        let edge = if site.loc.x == 0 { DirH::W } else { DirH::E };
                        for r in [1, 2, 3, 12] {
                            cells.push(CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_edge(edge),
                                RowId::from_idx(r),
                            ));
                        }
                        (
                            SpecialTileKey::I2cFifo(edge),
                            bels::I2C_FIFO,
                            [(SpecialIoKey::I2cScl, "SCL"), (SpecialIoKey::I2cSda, "SDA")]
                                .as_slice(),
                        )
                    }
                    "SB_LSOSC" => {
                        cells = vec![
                            CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_e() - 1,
                                self.chip.row_n(),
                            ),
                            CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_e(),
                                self.chip.row_n() - 1,
                            ),
                        ];
                        (SpecialTileKey::LsOsc, bels::LSOSC, &[][..])
                    }
                    "SB_HSOSC" => {
                        cells = vec![
                            CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_w() + 1,
                                self.chip.row_n(),
                            ),
                            CellCoord::new(
                                DieId::from_idx(0),
                                self.chip.col_w(),
                                self.chip.row_n() - 1,
                            ),
                        ];
                        (SpecialTileKey::HsOsc, bels::HSOSC, &[][..])
                    }
                    _ => unreachable!(),
                };
                let bel_pins = &self.bel_pins[&(kind, site.loc)];
                let tcid = key.tile_class(self.chip.kind);
                let mut builder = MiscTileBuilder::new(&self.intdb, tcid, &cells);
                builder.add_bel(bel, bel_pins);
                for &(slot, pin) in dedio {
                    let loc = site.dedio[pin];
                    let xy = (loc.x, loc.y, loc.bel);
                    let io = pkg_info.xlat_ioi[&xy];
                    builder.insert_io(slot, io);
                }
                let (tcls, special) = builder.finish();
                insert_tile_class(&mut self.tcls_filled, &mut self.intdb, tcid, tcls);
                self.chip.special_tiles.insert(key, special);
                self.special_tiles.insert(key, vec![site.loc]);
            }
        }
    }

    fn fill_misc(&mut self) {
        if !self.chip.kind.is_ultra() {
            return;
        }
        let key = SpecialTileKey::Misc;
        let tcid = key.tile_class(self.chip.kind);
        let cells = match self.chip.kind {
            ChipKind::Ice40T04 => vec![
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(18)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(19)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(20)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(17)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(18)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(19)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(20)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(16)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(3)),
            ],
            ChipKind::Ice40T01 => vec![
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(1)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(2)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(3)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(12)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(13)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(14)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(1)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(2)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(3)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(12)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(13)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(14)),
            ],
            ChipKind::Ice40T05 => vec![
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(28)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(29)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(30)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(27)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(28)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(29)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(30)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), RowId::from_idx(16)),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), RowId::from_idx(9)),
            ],
            _ => vec![],
        };
        let mut builder = MiscTileBuilder::new(&self.intdb, tcid, &cells);

        for (kind, bslot) in [
            ("SB_HFOSC", defs::bslots::HFOSC),
            ("SB_LFOSC", defs::bslots::LFOSC),
            ("SB_LEDD_IP", defs::bslots::LEDD_IP),
            ("SB_LEDDA_IP", defs::bslots::LEDD_IP),
            ("SB_IR_IP", defs::bslots::IR_IP),
            ("SB_WARMBOOT", defs::bslots::WARMBOOT),
        ] {
            if bslot == defs::bslots::WARMBOOT && self.chip.kind != ChipKind::Ice40T01 {
                continue;
            }
            let pkg_info = &self.pkgs[&self.def_pkg()];
            let Some(sites) = pkg_info.bel_info.get(kind) else {
                continue;
            };
            let Some(site) = sites.first() else {
                continue;
            };
            let bel_pins = &self.bel_pins[&(kind, site.loc)];
            builder.add_bel(bslot, bel_pins);
        }

        if self.chip.kind == ChipKind::Ice40T05 {
            // FILTER
            let mut filter_sites = self.pkgs[&self.def_pkg()].bel_info["SB_FILTER_50NS"].clone();
            filter_sites.sort_by_key(|site| site.loc);
            assert_eq!(filter_sites.len(), 2);

            for (i, site) in filter_sites.iter().enumerate() {
                let bel_pins = &self.bel_pins[&("SB_FILTER_50NS", site.loc)];
                builder.add_bel(bels::FILTER[i], bel_pins);
            }

            // IOB_I3C
            let mut io_sites = None;
            for pkg_info in self.pkgs.values() {
                let Some(sites) = pkg_info.bel_info.get("SB_IO_I3C") else {
                    continue;
                };
                let mut sites = sites.clone();
                sites.sort_by_key(|site| site.loc);
                let sites = Vec::from_iter(sites.into_iter().map(|site| {
                    let xy = (site.loc.x, site.loc.y, site.loc.bel);
                    let crd = pkg_info.xlat_ioi[&xy];
                    (site.loc, crd)
                }));
                if io_sites.is_none() {
                    io_sites = Some(sites);
                } else {
                    assert_eq!(io_sites, Some(sites));
                }
            }
            let io_sites = io_sites.unwrap();
            for &(site_loc, crd) in &io_sites {
                let idx = defs::bslots::IOI.index_of(crd.slot).unwrap();
                let mut bel_pins = self.bel_pins[&("SB_IO_I3C", site_loc)].clone();
                bel_pins.outs.clear();
                builder.add_bel(bels::IOB_I3C[idx], &bel_pins);
                builder.insert_io([SpecialIoKey::I3c0, SpecialIoKey::I3c1][idx], crd);
            }

            self.special_tiles.insert(
                key,
                vec![
                    filter_sites[0].loc,
                    filter_sites[1].loc,
                    io_sites[0].0,
                    io_sites[1].0,
                ],
            );
        }

        for pkg_info in self.pkgs.values() {
            for (kind, slot, io_pins) in [
                (
                    "SB_RGB_DRV",
                    defs::bslots::RGB_DRV,
                    [
                        (SpecialIoKey::RgbLed0, "RGB0"),
                        (SpecialIoKey::RgbLed1, "RGB1"),
                        (SpecialIoKey::RgbLed2, "RGB2"),
                    ]
                    .as_slice(),
                ),
                (
                    "SB_IR_DRV",
                    defs::bslots::IR_DRV,
                    [(SpecialIoKey::IrLed, "IRLED")].as_slice(),
                ),
                (
                    "SB_RGBA_DRV",
                    defs::bslots::RGB_DRV,
                    [
                        (SpecialIoKey::RgbLed0, "RGB0"),
                        (SpecialIoKey::RgbLed1, "RGB1"),
                        (SpecialIoKey::RgbLed2, "RGB2"),
                    ]
                    .as_slice(),
                ),
                (
                    "SB_IR400_DRV",
                    defs::bslots::IR500_DRV,
                    [(SpecialIoKey::IrLed, "IRLED")].as_slice(),
                ),
                (
                    "SB_BARCODE_DRV",
                    defs::bslots::IR500_DRV,
                    [(SpecialIoKey::BarcodeLed, "BARCODE")].as_slice(),
                ),
            ] {
                let Some(sites) = pkg_info.bel_info.get(kind) else {
                    continue;
                };
                if sites.is_empty() {
                    continue;
                }
                assert_eq!(sites.len(), 1);
                let site = &sites[0];
                let mut bel_pins = self.bel_pins[&(kind, site.loc)].clone();
                let mut bel_pins_drv = BelPins::default();
                bel_pins.ins.retain(|pin, &mut iw| {
                    if let InstPin::Simple(pin) = pin {
                        if let Some(pin) = pin.strip_prefix("LED_DRV_CUR__") {
                            bel_pins_drv
                                .ins
                                .insert(InstPin::Simple(pin.to_string()), iw);
                            false
                        } else if pin == "CURREN" {
                            bel_pins_drv
                                .ins
                                .insert(InstPin::Simple("EN".to_string()), iw);
                            false
                        } else if pin.starts_with("TRIM") {
                            bel_pins_drv.ins.insert(InstPin::Simple(pin.clone()), iw);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                });
                for &(slot, pin) in io_pins {
                    let io = site.pads[pin].0;
                    let xy = (io.x, io.y, io.bel);
                    let crd = pkg_info.xlat_ioi[&xy];
                    builder.insert_io(slot, crd);
                }
                builder.add_bel(slot, &bel_pins);
                builder.add_bel(defs::bslots::LED_DRV_CUR, &bel_pins_drv);
            }
        }

        let (tcls, special) = builder.finish();
        insert_tile_class(&mut self.tcls_filled, &mut self.intdb, tcid, tcls);
        self.chip.special_tiles.insert(key, special);
    }

    fn fill_warmboot(&mut self) {
        if matches!(self.chip.kind, ChipKind::Ice40T01 | ChipKind::Ice40P03) {
            return;
        }
        let pkg_info = &self.pkgs[&self.def_pkg()];
        let sites = &pkg_info.bel_info["SB_WARMBOOT"];
        assert_eq!(sites.len(), 1);
        let site = &sites[0];
        let key = SpecialTileKey::Warmboot;
        let bel = defs::bslots::WARMBOOT;
        let bel_pins = &self.bel_pins[&("SB_WARMBOOT", site.loc)];
        let mut cells = vec![];
        for pin in ["BOOT", "S0", "S1"] {
            cells.push(bel_pins.ins[&InstPin::Simple(pin.into())].cell);
        }
        let tcid = key.tile_class(self.chip.kind);
        let mut builder = MiscTileBuilder::new(&self.intdb, tcid, &cells);
        builder.add_bel(bel, bel_pins);
        let (tcls, special) = builder.finish();
        insert_tile_class(&mut self.tcls_filled, &mut self.intdb, tcid, tcls);
        self.chip.special_tiles.insert(key, special);
        self.special_tiles.insert(key, vec![site.loc]);
    }

    fn fill_pll(&mut self) {
        let kind = if self.chip.kind.is_ice40() {
            "SB_PLL40_CORE"
        } else {
            "SB_PLL_CORE"
        };
        for pkg_info in self.pkgs.values() {
            let Some(sites) = pkg_info.bel_info.get(kind) else {
                continue;
            };
            for site in sites {
                let xy = (site.loc.x, site.loc.y, site.loc.bel);
                let ioi = pkg_info.xlat_ioi[&xy];
                let ioi2 = ioi.delta(1, 0).bel(defs::bslots::IOI[0]);
                let mut bel_pins = self.bel_pins[&(kind, site.loc)].clone();
                bel_pins
                    .ins
                    .remove(&InstPin::Simple("LATCHINPUTVALUE".into()));
                bel_pins.outs.retain(|k, _| match k {
                    InstPin::Simple(pin) => !pin.starts_with("PLLOUT"),
                    InstPin::Indexed(_, _) => true,
                });
                let edge = if ioi.row == self.chip.row_s() {
                    DirV::S
                } else {
                    DirV::N
                };
                let loc = SpecialTileKey::Pll(edge);
                let tcid = loc.tile_class(self.chip.kind);
                let (col_first, num, num_side) = match self.chip.kind {
                    ChipKind::Ice65P04 => (self.chip.col_w() + 6, 13, 0),
                    ChipKind::Ice40P01 => (self.chip.col_w() + 1, 7, 14),
                    ChipKind::Ice40P08 => (self.chip.col_w() + 5, 18, 0),
                    ChipKind::Ice40R04 | ChipKind::Ice40T04 | ChipKind::Ice40T05 => {
                        (self.chip.col_w() + 1, 18, 0)
                    }
                    ChipKind::Ice40T01 => (self.chip.col_w() + 1, 9, 1),
                    _ => unreachable!(),
                };
                let mut cells =
                    CellCoord::new(DieId::from_idx(0), col_first, self.chip.row_edge(edge))
                        .cells_e(num);
                cells.extend(
                    CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_s() + 1)
                        .cells_n(num_side),
                );
                cells.extend([
                    CellCoord::new(
                        DieId::from_idx(0),
                        self.chip.col_w(),
                        self.chip.row_edge(edge),
                    ),
                    CellCoord::new(
                        DieId::from_idx(0),
                        self.chip.col_e(),
                        self.chip.row_edge(edge),
                    ),
                ]);
                let mut builder = MiscTileBuilder::new(&self.intdb, tcid, &cells);
                builder.insert_io(SpecialIoKey::PllA, ioi);
                builder.insert_io(SpecialIoKey::PllB, ioi2);
                if self.chip.kind.is_ice40() {
                    builder.add_bel(bels::PLL40, &bel_pins);
                } else {
                    builder.add_bel(bels::PLL65, &bel_pins);
                }
                let (tcls, special) = builder.finish();
                match self.chip.special_tiles.entry(loc) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(special);
                        insert_tile_class(&mut self.tcls_filled, &mut self.intdb, tcid, tcls);
                        self.special_tiles.insert(loc, vec![site.loc]);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), special);
                    }
                }
            }
        }
        if matches!(self.chip.kind, ChipKind::Ice40T04 | ChipKind::Ice40T05) {
            let mut cells =
                CellCoord::new(DieId::from_idx(0), self.chip.col_w() + 1, self.chip.row_s())
                    .cells_e(18);
            cells.extend([
                CellCoord::new(DieId::from_idx(0), self.chip.col_w(), self.chip.row_s()),
                CellCoord::new(DieId::from_idx(0), self.chip.col_e(), self.chip.row_s()),
            ]);

            let special = SpecialTile {
                io: BTreeMap::from_iter([
                    (
                        SpecialIoKey::PllA,
                        self.chip.special_tiles[&SpecialTileKey::GbRoot].io[&SpecialIoKey::GbIn(6)],
                    ),
                    (
                        SpecialIoKey::PllB,
                        self.chip.special_tiles[&SpecialTileKey::GbRoot].io[&SpecialIoKey::GbIn(3)],
                    ),
                ]),
                cells: cells.into_iter().collect(),
            };
            let key = SpecialTileKey::PllStub(DirV::S);
            self.chip.special_tiles.insert(key, special);
        }
    }

    fn fill_spram(&mut self) {
        let Some(sites) = self.pkgs[&self.def_pkg()].bel_info.get("SB_SPRAM256KA") else {
            return;
        };
        let mut sites = sites.clone();
        sites.sort_by_key(|site| site.loc);
        assert_eq!(sites.len(), 4);
        for edge_sites in sites.chunks_exact(2) {
            assert_eq!(edge_sites[0].loc.x, edge_sites[1].loc.x);
            let edge = if edge_sites[0].loc.x == 0 {
                DirH::W
            } else {
                DirH::E
            };
            let key = SpecialTileKey::SpramPair(edge);
            let tcid = key.tile_class(self.chip.kind);
            let cells: [_; 4] = CellCoord {
                die: DieId::from_idx(0),
                col: self.chip.col_edge(edge),
                row: RowId::from_idx(1),
            }
            .cells_n_const();
            let mut builder = MiscTileBuilder::new(&self.intdb, tcid, &cells);
            for (i, site) in edge_sites.iter().enumerate() {
                let bel_pins = &self.bel_pins[&("SB_SPRAM256KA", site.loc)];
                builder.add_bel(bels::SPRAM[i], bel_pins);
            }
            let (tcls, special) = builder.finish();
            insert_tile_class(&mut self.tcls_filled, &mut self.intdb, tcid, tcls);
            self.chip.special_tiles.insert(key, special);
            self.special_tiles
                .insert(key, vec![edge_sites[0].loc, edge_sites[1].loc]);
        }
    }

    fn inject_lut0_cascade(&mut self, harvester: &mut Harvester<BitOwner>) {
        let tcid = self.chip.kind.tile_class_plb();
        harvester.force_tiled(
            DiffKey::BelAttrBit(tcid, defs::bslots::LC[0], defs::bcls::LC::LTIN_ENABLE, 0),
            BTreeMap::from_iter([(TileBit::new(0, 0, 50), true)]),
        );
    }

    fn inject_io_inv_clk(&mut self, harvester: &mut Harvester<BitOwner>) {
        for (tcid, wire, bit) in [
            (
                self.chip.kind.tile_class_ioi(Dir::W),
                defs::wires::IMUX_IO_ICLK_OPTINV,
                TileBit::new(0, 9, 4),
            ),
            (
                self.chip.kind.tile_class_ioi(Dir::W),
                defs::wires::IMUX_IO_OCLK_OPTINV,
                TileBit::new(0, 15, 4),
            ),
            (
                self.chip.kind.tile_class_ioi(Dir::E),
                defs::wires::IMUX_IO_ICLK_OPTINV,
                TileBit::new(0, 9, 13),
            ),
            (
                self.chip.kind.tile_class_ioi(Dir::E),
                defs::wires::IMUX_IO_OCLK_OPTINV,
                TileBit::new(0, 15, 13),
            ),
            (
                self.chip.kind.tile_class_ioi(Dir::S),
                defs::wires::IMUX_IO_ICLK_OPTINV,
                TileBit::new(0, 6, 35),
            ),
            (
                self.chip.kind.tile_class_ioi(Dir::S),
                defs::wires::IMUX_IO_OCLK_OPTINV,
                TileBit::new(0, 1, 35),
            ),
            (
                self.chip.kind.tile_class_ioi(Dir::N),
                defs::wires::IMUX_IO_ICLK_OPTINV,
                TileBit::new(0, 9, 35),
            ),
            (
                self.chip.kind.tile_class_ioi(Dir::N),
                defs::wires::IMUX_IO_OCLK_OPTINV,
                TileBit::new(0, 14, 35),
            ),
        ] {
            let Some(tcid) = tcid else { continue };
            harvester.force_tiled(
                DiffKey::RoutingInv(
                    tcid,
                    TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire,
                    },
                ),
                BTreeMap::from_iter([(bit, true)]),
            );
        }
    }

    fn transplant_r04(
        &mut self,
        harvester: &mut Harvester<BitOwner>,
        pips: &mut BTreeMap<TileClassId, BTreeSet<(TileWireCoord, TileWireCoord)>>,
    ) {
        let db = Database::from_file("db/icecube/ice40p01.zstd").unwrap();
        for tcid in [defs::tcls::COLBUF_IO_W, defs::tcls::COLBUF_IO_E] {
            let BelInfo::SwitchBox(ref sb) = db.int.tile_classes[tcid].bels[defs::bslots::COLBUF]
            else {
                unreachable!()
            };
            for item in &sb.items {
                let SwitchBoxItem::ProgBuf(buf) = item else {
                    unreachable!();
                };
                harvester.force_tiled(
                    DiffKey::Routing(tcid, buf.dst, buf.src),
                    BTreeMap::from_iter([(buf.bit.bit, !buf.bit.inv)]),
                );
            }
        }
        for dir in [Dir::W, Dir::E] {
            let tcid = self.chip.kind.tile_class_ioi(dir).unwrap();
            let tcls = &db.int.tile_classes[tcid];
            let BelInfo::SwitchBox(sb) = &tcls.bels[bels::INT] else {
                unreachable!()
            };
            let mut tcls_pips = BTreeSet::new();
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        for (&src, val) in &mux.src {
                            if matches!(src.wire, defs::wires::TIE_0 | defs::wires::TIE_1) {
                                continue;
                            }
                            tcls_pips.insert((mux.dst, src.tw));
                            harvester.force_tiled(
                                DiffKey::Routing(tcid, mux.dst, src),
                                BTreeMap::from_iter(
                                    mux.bits
                                        .iter()
                                        .zip(val.iter())
                                        .filter(|&(_, bval)| bval)
                                        .map(|(&bit, _)| (bit, true)),
                                ),
                            );
                        }
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        tcls_pips.insert((buf.dst, buf.src.tw));
                        harvester.force_tiled(
                            DiffKey::Routing(tcid, buf.dst, buf.src),
                            BTreeMap::from_iter([(buf.bit.bit, !buf.bit.inv)]),
                        );
                    }
                    SwitchBoxItem::ProgInv(inv) => {
                        harvester.force_tiled(
                            DiffKey::RoutingInv(tcid, inv.dst),
                            BTreeMap::from_iter([(inv.bit.bit, !inv.bit.inv)]),
                        );
                    }
                    _ => (),
                }
            }
            pips.insert(tcid, tcls_pips);
            for bslot in [defs::bslots::IOI[0], defs::bslots::IOI[1]] {
                let BelInfo::Bel(ref bel) = tcls.bels[bslot] else {
                    unreachable!()
                };
                let BelAttribute::BitVec(ref bits) = bel.attributes[defs::bcls::IOI::PIN_TYPE]
                else {
                    unreachable!()
                };
                for (i, &bit) in bits.iter().enumerate() {
                    harvester.force_tiled(
                        DiffKey::BelAttrBit(tcid, bslot, defs::bcls::IOI::PIN_TYPE, i),
                        BTreeMap::from_iter([(bit.bit, !bit.inv)]),
                    );
                }
            }
        }
    }

    fn make_switchbox(&self, pips: &BTreeSet<(TileWireCoord, TileWireCoord)>) -> SwitchBox {
        let mut muxes: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        let mut items = vec![];
        let mut g2l = BTreeMap::new();
        let mut has_g2l = false;
        for &(wt, wf) in pips {
            let wtn = self.intdb.wires.key(wt.wire);
            let wfn = self.intdb.wires.key(wf.wire);
            if wtn.starts_with("LOCAL") && wfn.starts_with("GLOBAL") {
                has_g2l = true;
                break;
            }
        }
        if has_g2l {
            for i in 0..4 {
                let wt = TileWireCoord::new_idx(0, defs::wires::LOCAL_0[i + 4]);
                let wf = TileWireCoord::new_idx(0, defs::wires::GLOBAL_OUT[i]);
                g2l.insert(wt, wf);
                muxes.entry(wt).or_default().insert(wf.pos());
            }
        }
        for &(wt, wf) in pips {
            let wtn = self.intdb.wires.key(wt.wire);
            let wfn = self.intdb.wires.key(wf.wire);
            if ((wtn.starts_with("LONG") || wtn.starts_with("QUAD")) && wfn.starts_with("OUT"))
                || (wtn.starts_with("QUAD") && wfn.starts_with("LONG"))
            {
                items.push(SwitchBoxItem::ProgBuf(ProgBuf {
                    dst: wt,
                    src: wf.pos(),
                    bit: PolTileBit::DUMMY,
                }));
            } else if wtn.starts_with("LOCAL") && wfn.starts_with("GLOBAL") {
                let wgo = g2l[&wt];
                muxes.entry(wgo).or_default().insert(wf.pos());
            } else {
                muxes.entry(wt).or_default().insert(wf.pos());
            }
        }
        for (wt, mut wf) in muxes {
            let wtn = self.intdb.wires.key(wt.wire);
            if wtn.starts_with("IMUX") || wtn.starts_with("LOCAL") || wtn.starts_with("GLOBAL_OUT")
            {
                let tie = if wt.wire == defs::wires::IMUX_CE {
                    defs::wires::TIE_1
                } else {
                    defs::wires::TIE_0
                };
                wf.insert(TileWireCoord::new_idx(0, tie).pos());
            }
            items.push(SwitchBoxItem::Mux(Mux {
                dst: wt,
                bits: vec![],
                src: wf.into_iter().map(|k| (k, Default::default())).collect(),
                bits_off: None,
            }));
            let wtn = self.intdb.wires.key(wt.wire);
            if wtn.ends_with("CLK") {
                let wi = TileWireCoord::new_idx(0, self.intdb.get_wire(&format!("{wtn}_OPTINV")));
                items.push(SwitchBoxItem::ProgInv(ProgInv {
                    dst: wi,
                    src: wt,
                    bit: PolTileBit::DUMMY,
                }));
            }
        }
        items.sort();
        SwitchBox { items }
    }

    fn harvest(&mut self) {
        let mut harvester = Harvester::new();
        let mut pips = BTreeMap::new();
        if self.chip.kind.is_ice40() {
            self.inject_lut0_cascade(&mut harvester);
        }
        self.inject_io_inv_clk(&mut harvester);
        if self.chip.kind == ChipKind::Ice40R04 {
            self.transplant_r04(&mut harvester, &mut pips);
        }

        let edev = self.chip.expand_grid(&self.intdb);
        let gencfg = GeneratorConfig {
            prims: &self.prims,
            edev: &edev,
            pkgs: &self.pkgs,
            allow_global: false,
            rows_colbuf: vec![],
            special_tiles: &self.special_tiles,
        };
        let muxes = Mutex::new(pips);
        harvester.debug = self.debug;
        for key in wanted_keys_tiled(&edev) {
            harvester.want_tiled(key);
        }
        for key in wanted_keys_global(&edev) {
            harvester.want_global(key);
        }
        let harvester = Mutex::new(harvester);
        let speed = self
            .parts
            .iter()
            .flat_map(|part| {
                part.speeds.iter().map(|&speed| {
                    ((part.name, speed), {
                        Mutex::new(init_speed_data(self.chip.kind, part.name, speed))
                    })
                })
            })
            .collect();
        let mut hctx = HarvestContext {
            ctx: self,
            edev: &edev,
            gencfg,
            harvester,
            speed,
            pips: muxes,
        };

        hctx.run();

        let mut pips = hctx.pips.into_inner().unwrap();
        let mut harvester = hctx.harvester.into_inner().unwrap();
        let new_ioi_iob = collect_iob(&edev, &mut harvester);

        if self.chip.kind != ChipKind::Ice40P03 {
            let tcid_plb = self.chip.kind.tile_class_plb();
            let pips_plb = pips[&tcid_plb].clone();
            let pips_int_bram = pips.get_mut(&defs::tcls::INT_BRAM).unwrap();
            for (wt, wf) in pips_plb {
                if defs::wires::IMUX_LC_I0.contains(wt.wire)
                    || defs::wires::IMUX_LC_I1.contains(wt.wire)
                    || defs::wires::IMUX_LC_I2.contains(wt.wire)
                {
                    let diff_plb =
                        harvester.known_tiled[&DiffKey::Routing(tcid_plb, wt, wf.pos())].clone();
                    if let Some(diff_bram) = harvester.known_tiled.get(&DiffKey::Routing(
                        defs::tcls::INT_BRAM,
                        wt,
                        wf.pos(),
                    )) {
                        assert_eq!(diff_plb, *diff_bram);
                    } else {
                        harvester.force_tiled(
                            DiffKey::Routing(defs::tcls::INT_BRAM, wt, wf.pos()),
                            diff_plb,
                        );
                    }
                    pips_int_bram.insert((wt, wf));
                } else if !defs::wires::LC_CI_OUT.contains(wf.wire) {
                    assert!(pips_int_bram.contains(&(wt, wf)));
                }
            }
        }

        let speed = hctx.speed;
        self.chip.rows_colbuf = hctx.gencfg.rows_colbuf;
        for pkg in self.pkgs.values_mut() {
            for pad in pkg.bond.pins.values_mut().flatten() {
                if let Some(ioi) = self.chip.iob_to_ioi(pad.bel) {
                    *pad = new_ioi_iob
                        .get_left(&ioi)
                        .unwrap()
                        .pad(defs::bcls::IOB::PAD);
                }
            }
        }
        self.chip.ioi_iob = new_ioi_iob;

        for (tcid, tcls_pips) in pips {
            let sb = self.make_switchbox(&tcls_pips);
            self.intdb.tile_classes[tcid]
                .bels
                .insert(bels::INT, BelInfo::SwitchBox(sb));
        }

        let edev = self.chip.expand_grid(&self.intdb);

        let cdata = collect(&edev, &harvester);

        cdata.insert_into(&mut self.intdb, true);

        for (k, v) in speed {
            self.speed.insert(k, finish_speed(v.into_inner().unwrap()));
        }
    }

    fn write_db(&mut self) {
        let mut db = Database {
            chips: EntityVec::new(),
            bonds: EntityVec::new(),
            speeds: EntityVec::new(),
            devices: vec![],
            int: self.intdb.clone(),
        };
        db.int.validate();
        let chip = db.chips.push(self.chip.clone());
        for &part in &self.parts {
            let bonds = part
                .packages
                .iter()
                .map(|&pkg| {
                    let bond = &self.pkgs[&(part.name, pkg)].bond;
                    let bid = 'bond: {
                        for (bid, db_bond) in &db.bonds {
                            if db_bond == bond {
                                break 'bond bid;
                            }
                        }
                        db.bonds.push(bond.clone())
                    };
                    (pkg.to_string(), bid)
                })
                .collect();
            let speeds = part
                .speeds
                .iter()
                .map(|&sname| {
                    let speed = &self.speed[&(part.name, sname)];
                    let sid = 'speed: {
                        for (sid, db_speed) in &db.speeds {
                            if db_speed == speed {
                                break 'speed sid;
                            }
                        }
                        db.speeds.push(speed.clone())
                    };
                    (sname.to_string(), sid)
                })
                .collect();
            db.devices.push(prjcombine_siliconblue::db::Device {
                name: part.name.to_string(),
                chip,
                bonds,
                speeds,
                temps: part.temps.iter().map(|x| x.to_string()).collect(),
            });
        }
        db.to_file(format!("db/icecube/{}.zstd", self.chip.kind))
            .unwrap();
    }
}

fn main() {
    let args = Args::parse();
    let toolchain = Toolchain::from_file(args.toolchain).unwrap();
    let mut kinds = Vec::from_iter(args.kinds.iter().map(|kind| match kind.as_str() {
        "ice65l01" => ChipKind::Ice65L01,
        "ice65l04" => ChipKind::Ice65L04,
        "ice65l08" => ChipKind::Ice65L08,
        "ice65p04" => ChipKind::Ice65P04,
        "ice40p01" => ChipKind::Ice40P01,
        "ice40p03" => ChipKind::Ice40P03,
        "ice40p08" => ChipKind::Ice40P08,
        "ice40r04" => ChipKind::Ice40R04,
        "ice40t04" => ChipKind::Ice40T04,
        "ice40t01" => ChipKind::Ice40T01,
        "ice40t05" => ChipKind::Ice40T05,
        _ => panic!("unknown kind {kind}"),
    }));
    if kinds.is_empty() {
        kinds = vec![
            ChipKind::Ice65L01,
            ChipKind::Ice65L04,
            ChipKind::Ice65L08,
            ChipKind::Ice65P04,
            ChipKind::Ice40P01,
            ChipKind::Ice40P03,
            ChipKind::Ice40P08,
            ChipKind::Ice40R04,
            ChipKind::Ice40T04,
            ChipKind::Ice40T01,
            ChipKind::Ice40T05,
        ];
    }
    for kind in kinds {
        let parts = Vec::from_iter(parts::PARTS.iter().filter(|part| part.kind == kind));
        let mut ctx = PartContext {
            parts,
            chip: Chip {
                kind,
                columns: 0,
                cols_bram: vec![],
                col_bio_split: ColId::from_idx(0),
                rows: 0,
                row_mid: RowId::from_idx(0),
                rows_colbuf: vec![],
                rows_mac16: vec![],
                ioi_iob: BiMap::new(),
                iob_od: BTreeSet::new(),
                special_tiles: BTreeMap::new(),
            },
            intdb: bincode::decode_from_slice(
                prjcombine_siliconblue::defs::INIT,
                bincode::config::standard(),
            )
            .unwrap()
            .0,
            toolchain: &toolchain,
            prims: get_prims(kind),
            pkgs: BTreeMap::new(),
            extra_wire_names: BTreeMap::new(),
            bel_pins: BTreeMap::new(),
            special_tiles: BTreeMap::new(),
            speed: BTreeMap::new(),
            debug: args.debug,
            tcls_filled: BTreeSet::new(),
        };

        println!("{kind}: initializing");

        // ctx.intdb.print(&mut std::io::stdout()).unwrap();

        ctx.fill_sites();
        ctx.fill_xlat_rc();
        ctx.fill_cfg_io();
        ctx.fill_bonds();
        ctx.fill_cbsel();
        ctx.fill_io_latch();
        ctx.fill_bel_pins();
        ctx.fill_gbin_fabric();
        ctx.fill_gbin_io();
        ctx.fill_mac16();
        ctx.fill_hardip();
        ctx.fill_misc();
        ctx.fill_warmboot();
        ctx.fill_pll();
        ctx.fill_spram();

        println!("{kind}: initial geometry done; starting harvest");

        ctx.harvest();
        ctx.chip.expand_grid(&ctx.intdb);
        ctx.write_db();
    }
}
