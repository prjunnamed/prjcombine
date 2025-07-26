use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::dir::{DirH, DirV};
use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, ExpandedGrid, RowId, WireCoord};
use prjcombine_xilinx_bitstream::{
    BitstreamGeom, DeviceKind, DieBitstreamGeom, FrameAddr, FrameInfo,
};
use std::collections::{BTreeSet, HashSet};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::chip::{Chip, ChipKind, DisabledPart};
use crate::expanded::ExpandedDevice;
use crate::regions;

struct Expander<'a, 'b> {
    chip: &'b Chip,
    db: &'b IntDb,
    disabled: &'a BTreeSet<DisabledPart>,
    die: DieId,
    egrid: &'a mut ExpandedGrid<'b>,
    cols_bram: Vec<ColId>,
    spine_frame: usize,
    frame_info: Vec<FrameInfo>,
    col_frame: EntityVec<ColId, usize>,
    bram_frame: EntityPartVec<ColId, usize>,
    clkv_frame: EntityPartVec<ColId, usize>,
    blackhole_wires: HashSet<WireCoord>,
}

impl Expander<'_, '_> {
    fn fill_int(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            if cell.col == self.chip.col_w() {
                if cell.row == self.chip.row_s() {
                    self.egrid.add_tile_single(cell, "CNR.BL");
                } else if cell.row == self.chip.row_n() {
                    self.egrid.add_tile_single(cell, "CNR.TL");
                } else {
                    self.egrid.add_tile_single(cell, "IO.L");
                }
            } else if cell.col == self.chip.col_e() {
                if cell.row == self.chip.row_s() {
                    self.egrid.add_tile_single(cell, "CNR.BR");
                } else if cell.row == self.chip.row_n() {
                    self.egrid.add_tile_single(cell, "CNR.TR");
                } else {
                    self.egrid.add_tile_single(cell, "IO.R");
                }
            } else if self.chip.cols_bram.contains(&cell.col) {
                // skip for now
            } else {
                if cell.row == self.chip.row_s() {
                    self.egrid.add_tile_single(cell, "IO.B");
                } else if cell.row == self.chip.row_n() {
                    self.egrid.add_tile_single(cell, "IO.T");
                } else {
                    self.egrid.add_tile_single(cell, "CLB");
                }
            }
        }
    }

    fn fill_bram(&mut self) {
        for &col in &self.cols_bram {
            if self.disabled.contains(&DisabledPart::Bram(col)) {
                continue;
            }

            let cell = CellCoord::new(self.die, col, self.chip.row_s());
            self.egrid
                .add_tile(cell, "BRAM_BOT", &[cell, cell.delta(-1, 0)]);

            let mut prev_cell = cell;
            for cell in self.egrid.column(self.die, col) {
                if cell.row == self.chip.row_n() || cell.row.to_idx() % 4 != 1 {
                    continue;
                }
                let kind;
                if cell.col == self.chip.col_w() + 1 {
                    kind = "LBRAM";
                } else if cell.col == self.chip.col_e() - 1 {
                    kind = "RBRAM";
                } else {
                    kind = "MBRAM";
                }
                self.egrid.add_tile(
                    cell,
                    kind,
                    &[
                        cell.delta(0, 0),
                        cell.delta(0, 1),
                        cell.delta(0, 2),
                        cell.delta(0, 3),
                        cell.delta(-1, 0),
                        cell.delta(-1, 1),
                        cell.delta(-1, 2),
                        cell.delta(-1, 3),
                        cell.delta(1, 0),
                        cell.delta(1, 1),
                        cell.delta(1, 2),
                        cell.delta(1, 3),
                    ],
                );
                self.egrid
                    .fill_conn_pair(prev_cell, cell, "MAIN.N", "MAIN.S");
                prev_cell = cell;
            }

            let cell = CellCoord::new(self.die, col, self.chip.row_n());
            self.egrid
                .add_tile(cell, "BRAM_TOP", &[cell, cell.delta(-1, 0)]);
            self.egrid
                .fill_conn_pair(prev_cell, cell, "MAIN.N", "MAIN.S");

            // special hack!
            for (wire, wname, _) in &self.db.wires {
                if wname.starts_with("BRAM.QUAD") {
                    self.blackhole_wires.insert(cell.wire(wire));
                }
            }
        }
    }

    fn fill_clkbt(&mut self) {
        for edge in [DirV::S, DirV::N] {
            let row = self.chip.row_edge(edge);
            let cell = CellCoord::new(self.die, self.chip.col_clk(), row);
            // CLKB/CLKT and DLLs
            if self.chip.kind == ChipKind::Virtex {
                let cell_dll_w = cell.with_col(self.chip.col_w() + 1);
                let cell_dll_e = cell.with_col(self.chip.col_e() - 1);
                self.egrid.add_tile(
                    cell,
                    match edge {
                        DirV::S => "CLKB",
                        DirV::N => "CLKT",
                    },
                    &[cell, cell_dll_w, cell_dll_e],
                );
                self.egrid.add_tile(
                    cell_dll_w,
                    match edge {
                        DirV::S => "DLL.BOT",
                        DirV::N => "DLL.TOP",
                    },
                    &[cell_dll_w, cell_dll_w.delta(-1, 0), cell],
                );
                self.egrid.add_tile(
                    cell_dll_e,
                    match edge {
                        DirV::S => "DLL.BOT",
                        DirV::N => "DLL.TOP",
                    },
                    &[cell_dll_e, cell_dll_e.delta(-1, 0), cell],
                );
            } else {
                let bram_mid = self.cols_bram.len() / 2;
                let cell_dllp_w = cell.with_col(self.cols_bram[bram_mid - 1]);
                let cell_dllp_e = cell.with_col(self.cols_bram[bram_mid]);
                let cell_dlls_w = cell.with_col(self.cols_bram[bram_mid - 2]);
                let cell_dlls_e = cell.with_col(self.cols_bram[bram_mid + 1]);
                let kind = if self.disabled.contains(&DisabledPart::PrimaryDlls) {
                    match edge {
                        DirV::S => "CLKB_2DLL",
                        DirV::N => "CLKT_2DLL",
                    }
                } else {
                    match edge {
                        DirV::S => "CLKB_4DLL",
                        DirV::N => "CLKT_4DLL",
                    }
                };
                self.egrid.add_tile(
                    cell,
                    kind,
                    &[cell, cell_dllp_w, cell_dllp_e, cell_dlls_w, cell_dlls_e],
                );
                // DLLS
                let (kind_p, kind_s) = match edge {
                    DirV::S => ("DLLP.BOT", "DLLS.BOT"),
                    DirV::N => ("DLLP.TOP", "DLLS.TOP"),
                };
                self.egrid.add_tile(
                    cell_dlls_w,
                    kind_s,
                    &[cell_dlls_w, cell_dlls_w.delta(-1, 0), cell],
                );
                self.egrid.add_tile(
                    cell_dlls_e,
                    kind_s,
                    &[cell_dlls_e, cell_dlls_e.delta(-1, 0), cell],
                );
                if !self.disabled.contains(&DisabledPart::PrimaryDlls) {
                    self.egrid.add_tile(
                        cell_dllp_w,
                        kind_p,
                        &[cell_dllp_w, cell_dllp_w.delta(-1, 0), cell, cell_dlls_w],
                    );
                    self.egrid.add_tile(
                        cell_dllp_e,
                        kind_p,
                        &[cell_dllp_e, cell_dllp_e.delta(-1, 0), cell, cell_dlls_e],
                    );
                }
            }
        }
    }

    fn fill_pcilogic(&mut self) {
        // CLKL/CLKR
        self.egrid
            .add_tile_single(self.chip.bel_pci(DirH::W).cell, "CLKL");
        self.egrid
            .add_tile_single(self.chip.bel_pci(DirH::E).cell, "CLKR");
    }

    fn fill_clk(&mut self) {
        for cell in self.egrid.die_cells(self.die) {
            self.egrid[cell].region_root[regions::GLOBAL] =
                CellCoord::new(DieId::from_idx(0), ColId::from_idx(0), RowId::from_idx(0));
        }
        for &(col_m, col_l, col_r) in &self.chip.cols_clkv {
            let is_bram = col_m == self.chip.col_w() + 1 || col_m == self.chip.col_e() - 1;
            for col in col_l.range(col_r) {
                for cell in self.egrid.column(self.die, col) {
                    let cell_clk = if col < col_m {
                        cell.with_col(col_m - 1)
                    } else if !is_bram || self.chip.is_row_io(cell.row) {
                        cell.with_col(col_m)
                    } else if col > col_m {
                        cell.with_col(col_m + 1)
                    } else {
                        CellCoord::new(self.die, col_m, self.chip.row_clk())
                    };
                    self.egrid[cell].region_root[regions::LEAF] = cell_clk;
                }
            }
            if is_bram {
                let cell = CellCoord::new(self.die, col_m, self.chip.row_s());
                self.egrid.add_tile(
                    cell,
                    "CLKV_BRAM_S",
                    &[cell, cell.delta(-1, 0), cell.delta(0, 1)],
                );
                let cell = CellCoord::new(self.die, col_m, self.chip.row_n());
                self.egrid.add_tile(
                    cell,
                    "CLKV_BRAM_N",
                    &[cell, cell.delta(-1, 0), cell.delta(0, -4)],
                );
                self.egrid.add_tile_single(
                    CellCoord::new(self.die, col_m, self.chip.row_clk()),
                    "BRAM_CLKH",
                );
            } else {
                for cell in self.egrid.column(self.die, col_m) {
                    let kind = if self.chip.is_row_io(cell.row) {
                        "CLKV.NULL"
                    } else if col_m == self.chip.col_clk() {
                        "CLKV.CLKV"
                    } else {
                        "CLKV.GCLKV"
                    };
                    self.egrid.add_tile(cell, kind, &[cell.delta(-1, 0), cell]);
                }
                if col_m == self.chip.col_clk() {
                    self.egrid.add_tile(
                        CellCoord::new(self.die, col_m, self.chip.row_clk()),
                        "CLKC",
                        &[],
                    );
                } else {
                    self.egrid.add_tile(
                        CellCoord::new(self.die, col_m, self.chip.row_clk()),
                        "GCLKC",
                        &[],
                    );
                }
            }
        }
    }

    fn fill_frame_info(&mut self) {
        let mut major = 0;
        // spine
        self.spine_frame = 0;
        for minor in 0..8 {
            self.frame_info.push(FrameInfo {
                addr: FrameAddr {
                    typ: 0,
                    region: 0,
                    major,
                    minor,
                },
                mask_mode: [].into_iter().collect(),
            });
        }
        major += 1;
        self.clkv_frame.insert(self.chip.col_clk(), 7);

        for _ in self.chip.columns() {
            self.col_frame.push(0);
        }

        let split_bram = self.chip.kind != ChipKind::VirtexE;
        let mut clkv_frame = 0;

        for dx in 0..(self.chip.columns / 2) {
            for lr in ['R', 'L'] {
                let col = if lr == 'R' {
                    self.chip.col_clk() + dx
                } else {
                    self.chip.col_clk() - 1 - dx
                };
                if self.chip.cols_bram.contains(&col) && split_bram {
                    continue;
                }
                if col != self.chip.col_clk()
                    && !self.chip.cols_bram.contains(&col)
                    && self
                        .chip
                        .cols_clkv
                        .iter()
                        .any(|&(col_m, _, _)| col_m == col)
                {
                    self.clkv_frame.insert(col, clkv_frame);
                    clkv_frame += 1;
                }
                self.col_frame[col] = self.frame_info.len();
                let width = if col == self.chip.col_w() || col == self.chip.col_e() {
                    54
                } else if self.chip.cols_bram.contains(&col) {
                    27
                } else {
                    48
                };
                for minor in 0..width {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 0,
                            region: 0,
                            major,
                            minor,
                        },
                        mask_mode: [].into_iter().collect(),
                    });
                }
                major += 1;
            }
        }

        // bram main
        if split_bram {
            for dx in 0..(self.chip.columns / 2) {
                for lr in ['R', 'L'] {
                    let col = if lr == 'R' {
                        self.chip.col_clk() + dx
                    } else {
                        self.chip.col_clk() - 1 - dx
                    };
                    if !self.chip.cols_bram.contains(&col) {
                        continue;
                    }
                    self.col_frame[col] = self.frame_info.len();
                    for minor in 0..27 {
                        self.frame_info.push(FrameInfo {
                            addr: FrameAddr {
                                typ: 0,
                                region: 0,
                                major,
                                minor,
                            },
                            mask_mode: [].into_iter().collect(),
                        });
                    }
                    major += 1;
                }
            }
        }

        // bram data
        major = u32::from(self.chip.kind != ChipKind::Virtex);
        for dx in 0..(self.chip.columns / 2) {
            let lrorder = if self.chip.kind == ChipKind::Virtex {
                ['L', 'R']
            } else {
                ['R', 'L']
            };
            for lr in lrorder {
                let col = if lr == 'R' {
                    self.chip.col_clk() + dx
                } else {
                    self.chip.col_clk() - 1 - dx
                };
                if !self.chip.cols_bram.contains(&col) {
                    continue;
                }
                self.bram_frame.insert(col, self.frame_info.len());
                for minor in 0..64 {
                    self.frame_info.push(FrameInfo {
                        addr: FrameAddr {
                            typ: 1,
                            region: 0,
                            major,
                            minor,
                        },
                        mask_mode: [].into_iter().collect(),
                    });
                }
                major += 1;
            }
        }
    }
}

impl Chip {
    pub fn expand_grid<'a>(
        &'a self,
        disabled: &BTreeSet<DisabledPart>,
        db: &'a IntDb,
    ) -> ExpandedDevice<'a> {
        let mut egrid = ExpandedGrid::new(db);
        let die = egrid.add_die(self.columns, self.rows);

        let mut expander = Expander {
            chip: self,
            db,
            die,
            egrid: &mut egrid,
            disabled,
            cols_bram: self.cols_bram.iter().copied().collect(),
            frame_info: vec![],
            spine_frame: 0,
            col_frame: EntityVec::new(),
            bram_frame: EntityPartVec::new(),
            clkv_frame: EntityPartVec::new(),
            blackhole_wires: HashSet::new(),
        };
        expander.fill_int();
        expander.egrid.fill_main_passes(die);
        expander.fill_bram();
        expander.fill_clkbt();
        expander.fill_pcilogic();
        expander.fill_clk();
        expander.fill_frame_info();

        let spine_frame = expander.spine_frame;
        let col_frame = expander.col_frame;
        let bram_frame = expander.bram_frame;
        let clkv_frame = expander.clkv_frame;

        let die_bs_geom = DieBitstreamGeom {
            frame_len: self.rows * 18,
            frame_info: expander.frame_info,
            bram_frame_len: 0,
            bram_frame_info: vec![],
            iob_frame_len: 0,
        };
        let bs_geom = BitstreamGeom {
            kind: DeviceKind::Virtex,
            die: [die_bs_geom].into_iter().collect(),
            die_order: vec![expander.die],
            has_gtz_bot: false,
            has_gtz_top: false,
        };

        egrid.blackhole_wires = expander.blackhole_wires;
        egrid.finish();
        ExpandedDevice {
            chip: self,
            egrid,
            bs_geom,
            spine_frame,
            col_frame,
            bram_frame,
            clkv_frame,
            disabled: disabled.clone(),
        }
    }
}
