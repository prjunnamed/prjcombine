#![allow(clippy::too_many_arguments)]

use bincode::{Decode, Encode};
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec, entity_id};
use prjcombine_interconnect::{
    db::{TileClass, TileClassId},
    dir::{DirH, DirV},
    grid::{CellCoord, ColId, DieId, RowId, TileCoord},
};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_versal::{
    chip::{BramKind, CleKind, ColumnKind, HardRowKind, InterposerKind, RegId, RightKind},
    defs::{bslots, tcls},
    expanded::ExpandedDevice,
};
use std::{cmp::max, collections::BTreeMap};

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct DeviceNaming {
    pub die: EntityVec<DieId, DieNaming>,
    pub is_dsp_v2: bool,
    pub is_vnoc2_scan_offset: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct DieNaming {
    pub hdio: BTreeMap<(ColId, RegId), HdioNaming>,
    pub sysmon_sat_vnoc: BTreeMap<(ColId, RegId), (u32, u32)>,
    pub sysmon_sat_gt: BTreeMap<(ColId, RegId), (u32, u32)>,
    pub dpll_gt: BTreeMap<(ColId, RegId), (u32, u32)>,
    pub vnoc2: BTreeMap<(ColId, RegId), VNoc2Naming>,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct HdioNaming {
    pub iob_xy: (u32, u32),
    pub dpll_xy: (u32, u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct VNoc2Naming {
    pub nsu_xy: (u32, u32),
    pub nmu_xy: (u32, u32),
    pub nps_xy: (u32, u32),
    pub scan_xy: (u32, u32),
}

pub const BUFDIV_LEAF_SWZ_A: [u32; 32] = [
    3, 2, 1, 0, 8, 9, 10, 11, 19, 18, 17, 16, 24, 25, 26, 27, 4, 5, 6, 7, 15, 14, 13, 12, 20, 21,
    22, 23, 31, 30, 29, 28,
];

pub const BUFDIV_LEAF_SWZ_B: [u32; 32] = [
    7, 6, 5, 4, 12, 13, 14, 15, 23, 22, 21, 20, 28, 29, 30, 31, 0, 1, 2, 3, 11, 10, 9, 8, 16, 17,
    18, 19, 27, 26, 25, 24,
];

pub const BUFDIV_LEAF_SWZ_AH: [u32; 32] = [
    35, 34, 33, 32, 40, 41, 42, 43, 51, 50, 49, 48, 56, 57, 58, 59, 36, 37, 38, 39, 47, 46, 45, 44,
    52, 53, 54, 55, 63, 62, 61, 60,
];

pub const BUFDIV_LEAF_SWZ_BH: [u32; 32] = [
    39, 38, 37, 36, 44, 45, 46, 47, 55, 54, 53, 52, 60, 61, 62, 63, 32, 33, 34, 35, 43, 42, 41, 40,
    48, 49, 50, 51, 59, 58, 57, 56,
];

entity_id! {
    id EColId u32, delta;
}

#[derive(Clone, Debug)]
struct BelGrid {
    mirror_square: bool,
    xlut: EntityVec<DieId, EntityPartVec<ColId, i32>>,
    ylut: EntityVec<DieId, EntityPartVec<RowId, i32>>,
}

impl BelGrid {
    #[track_caller]
    fn name(&self, prefix: &str, die: DieId, col: ColId, row: RowId, dx: i32, dy: i32) -> String {
        self.name_mult(prefix, die, col, row, 1, dx, 1, dy)
    }

    #[track_caller]
    fn name_mult(
        &self,
        prefix: &str,
        die: DieId,
        col: ColId,
        row: RowId,
        mx: i32,
        dx: i32,
        my: i32,
        dy: i32,
    ) -> String {
        self.name_manual(
            prefix,
            die,
            (mx * self.xlut[die][col] + dx) as u32,
            (my * self.ylut[die][row] + dy) as u32,
        )
    }

    fn name_manual(&self, prefix: &str, die: DieId, x: u32, y: u32) -> String {
        if self.mirror_square {
            format!("{prefix}_S{die}X{x}Y{y}")
        } else {
            format!("{prefix}_X{x}Y{y}")
        }
    }
}

fn make_grid(
    edev: &ExpandedDevice,
    f: impl Fn(TileClassId, &str, &TileClass) -> bool,
    n: (i32, i32),
) -> BelGrid {
    make_grid_complex(edev, f, |_, _, _, _| n)
}

fn make_grid_complex(
    edev: &ExpandedDevice,
    f: impl Fn(TileClassId, &str, &TileClass) -> bool,
    n: impl Fn(TileClassId, &str, &TileClass, TileCoord) -> (i32, i32),
) -> BelGrid {
    if edev.interposer.kind == InterposerKind::MirrorSquare {
        let mut res = BelGrid {
            mirror_square: true,
            xlut: EntityVec::new(),
            ylut: EntityVec::new(),
        };
        for die in edev.chips.ids() {
            let mut cols = BTreeMap::new();
            let mut rows = BTreeMap::new();
            for (tcid, tcname, tcls) in &edev.db.tile_classes {
                if f(tcid, tcname, tcls) {
                    for &tcrd in &edev.tile_index[tcid] {
                        if tcrd.die != die {
                            continue;
                        }
                        let (n_x, n_y) = n(tcid, tcname, tcls, tcrd);
                        let v_c = cols.entry(tcrd.col).or_default();
                        *v_c = max(*v_c, n_x);
                        let v_r = rows.entry(tcrd.row).or_default();
                        *v_r = max(*v_r, n_y);
                    }
                }
            }
            let mut xlut = EntityPartVec::new();
            let mut ylut = EntityPartVec::new();
            let mut x = 0;
            for (col, num) in cols {
                xlut.insert(col, x);
                x += num;
            }
            let mut y = 0;
            for (row, num) in rows {
                ylut.insert(row, y);
                y += num;
            }
            res.xlut.push(xlut);
            res.ylut.push(ylut);
        }
        res
    } else {
        let mut cols: EntityVec<_, _> = edev.chips.ids().map(|_| BTreeMap::new()).collect();
        let mut rows = BTreeMap::new();
        for (tcid, tcname, tcls) in &edev.db.tile_classes {
            if f(tcid, tcname, tcls) {
                for &tcrd in &edev.tile_index[tcid] {
                    let (n_x, n_y) = n(tcid, tcname, tcls, tcrd);
                    let v_c = cols[tcrd.die].entry(tcrd.col).or_default();
                    *v_c = max(*v_c, n_x);
                    let v_r = rows.entry((tcrd.die, tcrd.row)).or_default();
                    *v_r = max(*v_r, n_y);
                }
            }
        }
        let mut xlut: EntityVec<_, _> = edev.chips.ids().map(|_| EntityPartVec::new()).collect();
        let mut x = 0;
        for die in edev.chips.ids() {
            let mut lx = 0;
            for (&col, &num) in &cols[die] {
                if col >= edev.col_cfrm[die] {
                    continue;
                }
                xlut[die].insert(col, lx);
                lx += num;
            }
            x = max(x, lx);
        }
        for die in edev.chips.ids() {
            let mut lx = x;
            for (&col, &num) in &cols[die] {
                if col < edev.col_cfrm[die] {
                    continue;
                }
                xlut[die].insert(col, lx);
                lx += num;
            }
        }
        let mut y = 0;
        let mut ylut: EntityVec<_, _> = edev.chips.ids().map(|_| EntityPartVec::new()).collect();
        for ((die, row), num) in rows {
            ylut[die].insert(row, y);
            y += num;
        }
        BelGrid {
            mirror_square: false,
            xlut,
            ylut,
        }
    }
}

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
}

pub fn name_device<'a>(
    edev: &'a ExpandedDevice<'a>,
    ndb: &'a NamingDb,
    dev_naming: &DeviceNaming,
) -> ExpandedNamedDevice<'a> {
    let mut int_grid = make_grid(edev, |tcid, _, _| tcid == tcls::INT, (1, 1));
    for (die, &chip) in &edev.chips {
        for col in chip.columns.ids() {
            if chip.col_side(col) == DirH::E {
                let x = int_grid.xlut[die][col - 1];
                int_grid.xlut[die].insert(col, x);
            }
        }
    }
    let mut int_grid_gt = int_grid.clone();
    let max_x = int_grid
        .xlut
        .values()
        .map(|xlut| xlut.values().copied().max().unwrap())
        .max()
        .unwrap();
    for (die, &chip) in &edev.chips {
        int_grid_gt.xlut[die][chip.columns.last_id().unwrap()] = max_x;
    }
    let bufdiv_grid = make_grid_complex(
        edev,
        |tcid, _, _| {
            matches!(
                tcid,
                tcls::RCLK_CLE
                    | tcls::RCLK_CLE_HALF
                    | tcls::RCLK_INTF_W
                    | tcls::RCLK_INTF_E
                    | tcls::RCLK_INTF_W_HALF
                    | tcls::RCLK_INTF_E_HALF
            )
        },
        |_, _, _, tcrd| {
            let chip = edev.chips[tcrd.die];
            let cd = &chip.columns[tcrd.col];
            match cd.kind {
                ColumnKind::Dsp => (1, 64),
                ColumnKind::ContHard => {
                    let hc = chip.get_col_hard(tcrd.col - 1).unwrap();
                    let reg = chip.row_to_reg(tcrd.row);
                    if matches!(
                        hc.regs[reg],
                        HardRowKind::DcmacT | HardRowKind::HscT | HardRowKind::IlknT
                    ) {
                        (1, 32)
                    } else {
                        (0, 0)
                    }
                }
                ColumnKind::Hard => {
                    let hc = chip.get_col_hard(tcrd.col).unwrap();
                    let reg = chip.row_to_reg(tcrd.row);
                    if matches!(
                        hc.regs[reg],
                        HardRowKind::DcmacT | HardRowKind::HscT | HardRowKind::IlknT
                    ) {
                        (1, 32)
                    } else {
                        (1, 64)
                    }
                }
                ColumnKind::ContDsp => (0, 0),
                _ => (1, 32),
            }
        },
    );
    let iri_grid = make_grid_complex(
        edev,
        |tcid, _, _| {
            matches!(
                tcid,
                tcls::INTF_W
                    | tcls::INTF_E
                    | tcls::INTF_W_TERM_GT
                    | tcls::INTF_E_TERM_GT
                    | tcls::INTF_W_HDIO
                    | tcls::INTF_E_HDIO
                    | tcls::INTF_W_HB
                    | tcls::INTF_E_HB
                    | tcls::INTF_W_TERM_PSS
                    | tcls::INTF_W_PSS
                    | tcls::INTF_BLI_CLE_W_S0
                    | tcls::INTF_BLI_CLE_E_S0
                    | tcls::INTF_BLI_CLE_W_N0
                    | tcls::INTF_BLI_CLE_E_N0
            )
        },
        |tcid, _, _, _| {
            if matches!(
                tcid,
                tcls::INTF_BLI_CLE_W_S0
                    | tcls::INTF_BLI_CLE_E_S0
                    | tcls::INTF_BLI_CLE_W_N0
                    | tcls::INTF_BLI_CLE_E_N0
            ) {
                (1, 16)
            } else {
                (1, 4)
            }
        },
    );
    let rclk_dfx_grid = make_grid_complex(
        edev,
        |tcid, _, _| matches!(tcid, tcls::RCLK_DFX_W | tcls::RCLK_DFX_E),
        |_, _, _, tcrd| {
            let chip = edev.chips[tcrd.die];
            if chip.columns[tcrd.col].kind == ColumnKind::Bram(BramKind::MaybeClkBufNoPd) {
                (2, 1)
            } else {
                (1, 1)
            }
        },
    );
    let slice_grid = make_grid(
        edev,
        |tcid, _, _| {
            matches!(
                tcid,
                tcls::CLE_W | tcls::CLE_E | tcls::CLE_W_VR | tcls::CLE_E_VR
            )
        },
        (2, 1),
    );
    let dsp_grid = make_grid(edev, |tcid, _, _| tcid == tcls::DSP, (1, 1));
    let bram_grid = make_grid(
        edev,
        |tcid, _, _| matches!(tcid, tcls::BRAM_W | tcls::BRAM_E),
        (1, 1),
    );
    let uram_grid = make_grid(
        edev,
        |tcid, _, _| matches!(tcid, tcls::URAM | tcls::URAM_DELAY),
        (1, 1),
    );
    let uram_delay_grid = make_grid(edev, |tcid, _, _| tcid == tcls::URAM_DELAY, (1, 1));
    let pcie4_grid = make_grid(edev, |tcid, _, _| tcid == tcls::PCIE4, (1, 1));
    let pcie5_grid = make_grid(edev, |tcid, _, _| tcid == tcls::PCIE5, (1, 1));
    let mrmac_grid = make_grid(edev, |tcid, _, _| tcid == tcls::MRMAC, (1, 1));
    let sdfec_grid = make_grid(edev, |tcid, _, _| tcid == tcls::SDFEC, (1, 1));
    let dfe_cfc_bot_grid = make_grid(edev, |tcid, _, _| tcid == tcls::DFE_CFC_S, (1, 1));
    let dfe_cfc_top_grid = make_grid(edev, |tcid, _, _| tcid == tcls::DFE_CFC_N, (1, 1));
    let dcmac_grid = make_grid(edev, |tcid, _, _| tcid == tcls::DCMAC, (1, 1));
    let ilkn_grid = make_grid(edev, |tcid, _, _| tcid == tcls::ILKN, (1, 1));
    let hsc_grid = make_grid(edev, |tcid, _, _| tcid == tcls::HSC, (1, 1));
    let hdio_grid = make_grid(edev, |tcid, _, _| tcid == tcls::HDIO, (1, 1));
    let vnoc_grid = make_grid(edev, |tcid, _, _| tcid == tcls::VNOC, (1, 1));
    let misr_grid = make_grid(edev, |tcid, _, _| tcid == tcls::MISR, (1, 1));
    let vdu_grid = make_grid(edev, |tcid, _, _| tcid == tcls::VDU_E, (1, 1));
    let bfr_b_grid = make_grid(edev, |tcid, _, _| tcid == tcls::BFR_B_E, (1, 1));

    let mut ngrid = ExpandedGridNaming::new(ndb, edev);

    for (tcrd, tile) in edev.tiles() {
        let chip = edev.chips[tcrd.die];
        let reg = chip.row_to_reg(tcrd.row);
        let CellCoord { die, col, row } = tcrd.cell;
        let kind = edev.db.tile_classes.key(tile.class);
        match tile.class {
            tcls::INT => {
                ngrid.name_tile(tcrd, "INT", [int_grid.name("INT", die, col, row, 0, 0)]);
            }
            tcls::RCLK => {
                let lr = if col < edev.col_cfrm[die] { 'L' } else { 'R' };
                let vr = if chip.is_vr { "_VR" } else { "" };
                ngrid.name_tile(
                    tcrd,
                    "RCLK",
                    [int_grid.name(
                        &format!("RCLK_INT_{lr}{vr}_FT"),
                        die,
                        col,
                        if reg.to_idx() % 2 == 1 { row - 1 } else { row },
                        0,
                        0,
                    )],
                );
            }
            tcls::CLE_BC | tcls::CLE_BC_SLL | tcls::CLE_BC_SLL2 => {
                let tk = match tile.class {
                    tcls::CLE_BC => "CLE_BC_CORE",
                    tcls::CLE_BC_SLL => "SLL",
                    tcls::CLE_BC_SLL2 => "SLL2",
                    _ => unreachable!(),
                };
                let bump_cur = col > edev.col_cfrm[die] && chip.cols_vbrk.contains(&(col + 1));
                let bump_prev = matches!(chip.columns[col - 1].kind, ColumnKind::Cle(_))
                    && col - 1 > edev.col_cfrm[die]
                    && chip.cols_vbrk.contains(&(col - 1));
                ngrid.name_tile(
                    tcrd,
                    kind,
                    [int_grid.name(
                        &if bump_prev && !bump_cur {
                            format!("{tk}_1")
                        } else {
                            tk.to_string()
                        },
                        die,
                        if bump_cur { col + 1 } else { col },
                        row,
                        0,
                        0,
                    )],
                );
            }
            tcls::RCLK_CLE | tcls::RCLK_CLE_HALF => {
                let ColumnKind::Cle(cle_kind) = chip.columns[col].kind else {
                    unreachable!()
                };
                let naming = &if cle_kind == CleKind::Plain {
                    if chip.is_vr {
                        format!("{kind}_VR")
                    } else {
                        kind.to_string()
                    }
                } else {
                    format!("{kind}_LAG")
                };
                let ntile = ngrid.name_tile(
                    tcrd,
                    naming,
                    [int_grid.name(
                        if cle_kind == CleKind::Plain {
                            if chip.is_vr {
                                "RCLK_CLE_VR_CORE"
                            } else {
                                "RCLK_CLE_CORE"
                            }
                        } else {
                            "RCLK_CLE_LAG_CORE"
                        },
                        die,
                        col - 1,
                        if reg.to_idx() % 2 == 1 { row - 1 } else { row },
                        0,
                        0,
                    )],
                );
                let swz = if cle_kind == CleKind::Plain && !chip.is_vr {
                    BUFDIV_LEAF_SWZ_A
                } else {
                    BUFDIV_LEAF_SWZ_B
                };
                for (i, dy) in swz.into_iter().enumerate() {
                    ntile.add_bel(
                        bslots::BUFDIV_LEAF[i],
                        bufdiv_grid.name(
                            if chip.is_vr {
                                "BUFDIV_LEAF_ULVT"
                            } else {
                                "BUFDIV_LEAF"
                            },
                            die,
                            col,
                            row,
                            0,
                            dy as i32,
                        ),
                    );
                }
            }
            tcls::INTF_W
            | tcls::INTF_W_TERM_GT
            | tcls::INTF_W_HDIO
            | tcls::INTF_W_HB
            | tcls::INTF_W_PSS
            | tcls::INTF_W_TERM_PSS => {
                let ocf = if col < edev.col_cfrm[die] {
                    "LOCF"
                } else {
                    "ROCF"
                };
                let bt = if chip.is_reg_n(reg) { 'T' } else { 'B' };
                let name = int_grid.name(
                    &match tile.class {
                        tcls::INTF_W => format!("INTF_{ocf}_{bt}L_TILE"),
                        tcls::INTF_W_TERM_GT => format!("INTF_GT_{bt}L_TILE"),
                        tcls::INTF_W_HDIO => format!("INTF_HDIO_{ocf}_{bt}L_TILE"),
                        tcls::INTF_W_HB => format!("INTF_HB_{ocf}_{bt}L_TILE"),
                        tcls::INTF_W_PSS => format!("INTF_CFRM_{bt}L_TILE"),
                        tcls::INTF_W_TERM_PSS => format!("INTF_PSS_{bt}L_TILE"),
                        _ => unreachable!(),
                    },
                    die,
                    col,
                    row,
                    0,
                    0,
                );
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                for i in 0..4 {
                    ntile.add_bel(
                        bslots::IRI[i],
                        iri_grid.name("IRI_QUAD", die, col, row, 0, i as i32),
                    );
                }
            }
            tcls::INTF_E | tcls::INTF_E_TERM_GT | tcls::INTF_E_HDIO | tcls::INTF_E_HB => {
                let ocf = if col < edev.col_cfrm[die] {
                    "LOCF"
                } else {
                    "ROCF"
                };
                let bt = if chip.is_reg_n(reg) { 'T' } else { 'B' };
                let name = int_grid.name(
                    &match tile.class {
                        tcls::INTF_E => format!("INTF_{ocf}_{bt}R_TILE"),
                        tcls::INTF_E_TERM_GT => format!("INTF_GT_{bt}R_TILE"),
                        tcls::INTF_E_HDIO => format!("INTF_HDIO_{ocf}_{bt}R_TILE"),
                        tcls::INTF_E_HB => format!("INTF_HB_{ocf}_{bt}R_TILE"),
                        _ => unreachable!(),
                    },
                    die,
                    col,
                    row,
                    0,
                    0,
                );
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                for i in 0..4 {
                    ntile.add_bel(
                        bslots::IRI[i],
                        iri_grid.name("IRI_QUAD", die, col, row, 0, i as i32),
                    );
                }
            }
            tcls::INTF_BLI_CLE_W_S0
            | tcls::INTF_BLI_CLE_W_S1
            | tcls::INTF_BLI_CLE_W_S2
            | tcls::INTF_BLI_CLE_W_S3
            | tcls::INTF_BLI_CLE_W_N0
            | tcls::INTF_BLI_CLE_W_N1
            | tcls::INTF_BLI_CLE_W_N2
            | tcls::INTF_BLI_CLE_W_N3 => {
                let (dy, side, srow) = match tile.class {
                    tcls::INTF_BLI_CLE_W_S0 => (12, DirV::S, row),
                    tcls::INTF_BLI_CLE_W_S1 => (8, DirV::S, row - 1),
                    tcls::INTF_BLI_CLE_W_S2 => (4, DirV::S, row - 2),
                    tcls::INTF_BLI_CLE_W_S3 => (0, DirV::S, row - 3),
                    tcls::INTF_BLI_CLE_W_N0 => (0, DirV::N, row),
                    tcls::INTF_BLI_CLE_W_N1 => (4, DirV::N, row - 1),
                    tcls::INTF_BLI_CLE_W_N2 => (8, DirV::N, row - 2),
                    tcls::INTF_BLI_CLE_W_N3 => (12, DirV::N, row - 3),
                    _ => unreachable!(),
                };
                let name = int_grid.name(
                    if side == DirV::S {
                        "BLI_CLE_BOT_CORE"
                    } else {
                        "BLI_CLE_TOP_CORE"
                    },
                    die,
                    col,
                    srow,
                    0,
                    0,
                );
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                for i in 0..4 {
                    ntile.add_bel(
                        bslots::IRI[i],
                        iri_grid.name("IRI_QUAD", die, col, srow, 0, dy + i as i32),
                    );
                }
            }
            tcls::INTF_BLI_CLE_E_S0
            | tcls::INTF_BLI_CLE_E_S1
            | tcls::INTF_BLI_CLE_E_S2
            | tcls::INTF_BLI_CLE_E_S3
            | tcls::INTF_BLI_CLE_E_N0
            | tcls::INTF_BLI_CLE_E_N1
            | tcls::INTF_BLI_CLE_E_N2
            | tcls::INTF_BLI_CLE_E_N3 => {
                let (dy, side, srow) = match tile.class {
                    tcls::INTF_BLI_CLE_E_S0 => (12, DirV::S, row),
                    tcls::INTF_BLI_CLE_E_S1 => (8, DirV::S, row - 1),
                    tcls::INTF_BLI_CLE_E_S2 => (4, DirV::S, row - 2),
                    tcls::INTF_BLI_CLE_E_S3 => (0, DirV::S, row - 3),
                    tcls::INTF_BLI_CLE_E_N0 => (0, DirV::N, row),
                    tcls::INTF_BLI_CLE_E_N1 => (4, DirV::N, row - 1),
                    tcls::INTF_BLI_CLE_E_N2 => (8, DirV::N, row - 2),
                    tcls::INTF_BLI_CLE_E_N3 => (12, DirV::N, row - 3),
                    _ => unreachable!(),
                };
                let name = int_grid.name(
                    if side == DirV::S {
                        if matches!(chip.columns[col - 1].kind, ColumnKind::Cle(_))
                            && chip.columns[col - 1].has_bli_s
                        {
                            "BLI_CLE_BOT_CORE_1"
                        } else {
                            "BLI_CLE_BOT_CORE"
                        }
                    } else {
                        if matches!(chip.columns[col - 1].kind, ColumnKind::Cle(_))
                            && chip.columns[col - 1].has_bli_n
                        {
                            "BLI_CLE_TOP_CORE_1"
                        } else {
                            "BLI_CLE_TOP_CORE"
                        }
                    },
                    die,
                    col,
                    srow,
                    0,
                    0,
                );
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                for i in 0..4 {
                    ntile.add_bel(
                        bslots::IRI[i],
                        iri_grid.name("IRI_QUAD", die, col, srow, 0, dy + i as i32),
                    );
                }
            }
            tcls::RCLK_INTF_W | tcls::RCLK_INTF_W_HALF => {
                let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                let (subkind, name, swz, wide) = match chip.columns[col].kind {
                    ColumnKind::ContDsp => (
                        if chip.is_vr { "DSP_VR" } else { "DSP" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_DSP_VR_CORE"
                            } else {
                                "RCLK_DSP_CORE"
                            },
                            die,
                            col - 1,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_BH
                        } else {
                            BUFDIV_LEAF_SWZ_AH
                        },
                        true,
                    ),
                    ColumnKind::Bram(BramKind::Plain) => (
                        if chip.is_vr { "BRAM_VR" } else { "BRAM" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_BRAM_VR_CORE"
                            } else {
                                "RCLK_BRAM_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                        false,
                    ),
                    ColumnKind::Uram => (
                        if chip.is_vr { "URAM_VR" } else { "URAM" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_URAM_VR_CORE"
                            } else {
                                "RCLK_URAM_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                        false,
                    ),
                    ColumnKind::Gt => (
                        if chip.is_vr { "GT_VR" } else { "GT" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_INTF_TERM_LEFT_VR_CORE"
                            } else {
                                "RCLK_INTF_TERM_LEFT_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                        false,
                    ),
                    ColumnKind::Cfrm => (
                        if chip.is_vr { "CFRM_VR" } else { "CFRM" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_INTF_OPT_VR_CORE"
                            } else {
                                "RCLK_INTF_OPT_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                        false,
                    ),
                    ColumnKind::ContVNoc => (
                        if chip.is_vr { "VNOC_VR" } else { "VNOC" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_INTF_L_VR_CORE"
                            } else {
                                "RCLK_INTF_L_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        BUFDIV_LEAF_SWZ_B,
                        false,
                    ),
                    ColumnKind::ContHard => {
                        let hc = chip.get_col_hard(col - 1).unwrap();
                        if reg.to_idx().is_multiple_of(2) {
                            if hc.regs[reg] == HardRowKind::Hdio {
                                (
                                    if chip.is_vr { "HDIO_VR" } else { "HDIO" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HDIO_VR_CORE"
                                        } else {
                                            "RCLK_HDIO_CORE"
                                        },
                                        die,
                                        col - 1,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if chip.is_vr {
                                        BUFDIV_LEAF_SWZ_BH
                                    } else {
                                        BUFDIV_LEAF_SWZ_AH
                                    },
                                    true,
                                )
                            } else {
                                (
                                    if chip.is_vr { "HB_VR" } else { "HB" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HB_VR_CORE"
                                        } else {
                                            "RCLK_HB_CORE"
                                        },
                                        die,
                                        col - 1,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if chip.is_vr {
                                        BUFDIV_LEAF_SWZ_BH
                                    } else {
                                        BUFDIV_LEAF_SWZ_AH
                                    },
                                    true,
                                )
                            }
                        } else {
                            if hc.regs[reg] == HardRowKind::Hdio {
                                (
                                    if chip.is_vr { "HDIO_VR" } else { "HDIO" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HDIO_VR_CORE"
                                        } else {
                                            "RCLK_HDIO_CORE"
                                        },
                                        die,
                                        col - 1,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if chip.is_vr {
                                        BUFDIV_LEAF_SWZ_BH
                                    } else {
                                        BUFDIV_LEAF_SWZ_AH
                                    },
                                    true,
                                )
                            } else if hc.regs[reg - 1] == HardRowKind::Hdio {
                                (
                                    if chip.is_vr { "HB_HDIO_VR" } else { "HB_HDIO" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HB_HDIO_VR_CORE"
                                        } else {
                                            "RCLK_HB_HDIO_CORE"
                                        },
                                        die,
                                        col - 1,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    BUFDIV_LEAF_SWZ_BH,
                                    true,
                                )
                            } else if hc.regs[reg - 1] == HardRowKind::DfeCfcS {
                                (
                                    "SDFEC",
                                    int_grid.name("RCLK_SDFEC_CORE", die, col - 1, srow, 0, 0),
                                    BUFDIV_LEAF_SWZ_BH,
                                    true,
                                )
                            } else if matches!(
                                hc.regs[reg],
                                HardRowKind::DcmacT | HardRowKind::HscT | HardRowKind::IlknT
                            ) {
                                (
                                    if chip.is_vr { "HB_FULL_VR" } else { "HB_FULL" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HB_FULL_R_VR_CORE"
                                        } else {
                                            "RCLK_HB_FULL_R_CORE"
                                        },
                                        die,
                                        col,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    BUFDIV_LEAF_SWZ_B,
                                    false,
                                )
                            } else {
                                (
                                    if chip.is_vr { "HB_VR" } else { "HB" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HB_VR_CORE"
                                        } else {
                                            "RCLK_HB_CORE"
                                        },
                                        die,
                                        col - 1,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if chip.is_vr {
                                        BUFDIV_LEAF_SWZ_BH
                                    } else {
                                        BUFDIV_LEAF_SWZ_AH
                                    },
                                    true,
                                )
                            }
                        }
                    }
                    _ => unreachable!(),
                };
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}_{subkind}"), [name]);
                for (i, dy) in swz.into_iter().enumerate() {
                    ntile.add_bel(
                        bslots::BUFDIV_LEAF[i],
                        if wide {
                            bufdiv_grid.name(
                                if chip.is_vr {
                                    "BUFDIV_LEAF_ULVT"
                                } else {
                                    "BUFDIV_LEAF"
                                },
                                die,
                                col - 1,
                                row,
                                0,
                                dy as i32,
                            )
                        } else {
                            bufdiv_grid.name(
                                if chip.is_vr {
                                    "BUFDIV_LEAF_ULVT"
                                } else {
                                    "BUFDIV_LEAF"
                                },
                                die,
                                col,
                                row,
                                0,
                                dy as i32,
                            )
                        },
                    );
                }
            }
            tcls::RCLK_INTF_E | tcls::RCLK_INTF_E_HALF => {
                let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                let (subkind, name, swz) = match chip.columns[col].kind {
                    ColumnKind::Dsp => (
                        if chip.is_vr { "DSP_VR" } else { "DSP" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_DSP_VR_CORE"
                            } else {
                                "RCLK_DSP_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                    ),
                    ColumnKind::Bram(BramKind::Plain) => (
                        if chip.is_vr { "BRAM_VR" } else { "BRAM" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_BRAM_VR_CORE"
                            } else {
                                "RCLK_BRAM_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                    ),
                    ColumnKind::Bram(BramKind::ClkBuf) => (
                        if chip.is_vr {
                            "BRAM_CLKBUF_VR"
                        } else {
                            "BRAM_CLKBUF"
                        },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_BRAM_CLKBUF_VR_CORE"
                            } else {
                                "RCLK_BRAM_CLKBUF_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                    ),
                    ColumnKind::Bram(BramKind::ClkBufNoPd) => (
                        if chip.is_vr {
                            "BRAM_CLKBUF_NOPD_VR"
                        } else {
                            "BRAM_CLKBUF_NOPD"
                        },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_BRAM_CLKBUF_NOPD_VR_CORE"
                            } else {
                                "RCLK_BRAM_CLKBUF_NOPD_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        BUFDIV_LEAF_SWZ_B,
                    ),
                    ColumnKind::Bram(BramKind::MaybeClkBufNoPd) => (
                        if row.to_idx() < chip.get_ps_height() {
                            if chip.is_vr { "BRAM_VR" } else { "BRAM" }
                        } else {
                            if chip.is_vr {
                                "BRAM_CLKBUF_NOPD_VR"
                            } else {
                                "BRAM_CLKBUF_NOPD"
                            }
                        },
                        int_grid.name(
                            if row.to_idx() < chip.get_ps_height() {
                                if chip.is_vr {
                                    "RCLK_BRAM_VR_CORE"
                                } else {
                                    "RCLK_BRAM_CORE"
                                }
                            } else {
                                if chip.is_vr {
                                    "RCLK_BRAM_CLKBUF_NOPD_VR_CORE"
                                } else {
                                    "RCLK_BRAM_CLKBUF_NOPD_CORE"
                                }
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr || row.to_idx() >= chip.get_ps_height() {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                    ),
                    ColumnKind::Uram => (
                        if chip.is_vr { "URAM_VR" } else { "URAM" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_URAM_VR_CORE"
                            } else {
                                "RCLK_URAM_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if chip.is_vr {
                            BUFDIV_LEAF_SWZ_B
                        } else {
                            BUFDIV_LEAF_SWZ_A
                        },
                    ),
                    ColumnKind::Gt => {
                        if reg.to_idx() == 1 && matches!(chip.right, RightKind::Term2) {
                            (
                                if chip.is_vr { "GT.ALT_VR" } else { "GT.ALT" },
                                int_grid.name(
                                    if chip.is_vr {
                                        "RCLK_INTF_TERM2_RIGHT_VR_CORE"
                                    } else {
                                        "RCLK_INTF_TERM2_RIGHT_CORE"
                                    },
                                    die,
                                    col,
                                    srow,
                                    0,
                                    0,
                                ),
                                BUFDIV_LEAF_SWZ_B,
                            )
                        } else {
                            (
                                if chip.is_vr { "GT_VR" } else { "GT" },
                                int_grid.name(
                                    if chip.is_vr {
                                        "RCLK_INTF_TERM_RIGHT_VR_CORE"
                                    } else {
                                        "RCLK_INTF_TERM_RIGHT_CORE"
                                    },
                                    die,
                                    col,
                                    srow,
                                    0,
                                    0,
                                ),
                                if chip.is_vr {
                                    BUFDIV_LEAF_SWZ_B
                                } else {
                                    BUFDIV_LEAF_SWZ_A
                                },
                            )
                        }
                    }
                    ColumnKind::VNoc | ColumnKind::VNoc2 | ColumnKind::VNoc4 => (
                        if chip.is_vr { "VNOC_VR" } else { "VNOC" },
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_INTF_R_VR_CORE"
                            } else {
                                "RCLK_INTF_R_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        BUFDIV_LEAF_SWZ_B,
                    ),
                    ColumnKind::Hard => {
                        let hc = chip.get_col_hard(col).unwrap();
                        if reg.to_idx().is_multiple_of(2) {
                            if hc.regs[reg] == HardRowKind::Hdio {
                                (
                                    if chip.is_vr { "HDIO_VR" } else { "HDIO" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HDIO_VR_CORE"
                                        } else {
                                            "RCLK_HDIO_CORE"
                                        },
                                        die,
                                        col,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if chip.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                )
                            } else {
                                (
                                    if chip.is_vr { "HB_VR" } else { "HB" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HB_VR_CORE"
                                        } else {
                                            "RCLK_HB_CORE"
                                        },
                                        die,
                                        col,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if chip.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                )
                            }
                        } else {
                            if hc.regs[reg] == HardRowKind::Hdio {
                                (
                                    if chip.is_vr { "HDIO_VR" } else { "HDIO" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HDIO_VR_CORE"
                                        } else {
                                            "RCLK_HDIO_CORE"
                                        },
                                        die,
                                        col,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if chip.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                )
                            } else if hc.regs[reg - 1] == HardRowKind::Hdio {
                                (
                                    if chip.is_vr { "HB_HDIO_VR" } else { "HB_HDIO" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HB_HDIO_VR_CORE"
                                        } else {
                                            "RCLK_HB_HDIO_CORE"
                                        },
                                        die,
                                        col,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    BUFDIV_LEAF_SWZ_B,
                                )
                            } else if hc.regs[reg - 1] == HardRowKind::DfeCfcS {
                                (
                                    "SDFEC",
                                    int_grid.name("RCLK_SDFEC_CORE", die, col, srow, 0, 0),
                                    BUFDIV_LEAF_SWZ_B,
                                )
                            } else if matches!(
                                hc.regs[reg],
                                HardRowKind::DcmacT | HardRowKind::HscT | HardRowKind::IlknT
                            ) {
                                (
                                    if chip.is_vr { "HB_FULL_VR" } else { "HB_FULL" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HB_FULL_L_VR_CORE"
                                        } else {
                                            "RCLK_HB_FULL_L_CORE"
                                        },
                                        die,
                                        col,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    BUFDIV_LEAF_SWZ_B,
                                )
                            } else {
                                (
                                    if chip.is_vr { "HB_VR" } else { "HB" },
                                    int_grid.name(
                                        if chip.is_vr {
                                            "RCLK_HB_VR_CORE"
                                        } else {
                                            "RCLK_HB_CORE"
                                        },
                                        die,
                                        col,
                                        srow,
                                        0,
                                        0,
                                    ),
                                    if chip.is_vr {
                                        BUFDIV_LEAF_SWZ_B
                                    } else {
                                        BUFDIV_LEAF_SWZ_A
                                    },
                                )
                            }
                        }
                    }
                    _ => unreachable!(),
                };
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}_{subkind}"), [name]);
                for (i, dy) in swz.into_iter().enumerate() {
                    ntile.add_bel(
                        bslots::BUFDIV_LEAF[i],
                        bufdiv_grid.name(
                            if chip.is_vr {
                                "BUFDIV_LEAF_ULVT"
                            } else {
                                "BUFDIV_LEAF"
                            },
                            die,
                            col,
                            row,
                            0,
                            dy as i32,
                        ),
                    );
                }
            }
            tcls::RCLK_DFX_W => {
                let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                let (subkind, name) = match chip.columns[col].kind {
                    ColumnKind::ContDsp => (
                        "DSP",
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_DSP_VR_CORE"
                            } else {
                                "RCLK_DSP_CORE"
                            },
                            die,
                            col - 1,
                            srow,
                            0,
                            0,
                        ),
                    ),
                    ColumnKind::Bram(BramKind::Plain) => (
                        "BRAM",
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_BRAM_VR_CORE"
                            } else {
                                "RCLK_BRAM_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                    ),
                    ColumnKind::Uram => (
                        "URAM",
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_URAM_VR_CORE"
                            } else {
                                "RCLK_URAM_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                    ),
                    _ => unreachable!(),
                };
                let vr = if chip.is_vr { "_VR" } else { "" };
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}_{subkind}{vr}"), [name]);
                ntile.add_bel(
                    bslots::RCLK_DFX_TEST,
                    rclk_dfx_grid.name("RCLK", die, col, row, 0, 0),
                );
            }
            tcls::RCLK_DFX_E => {
                let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                let (subkind, name, dx) = match chip.columns[col].kind {
                    ColumnKind::Bram(BramKind::Plain) => (
                        "BRAM",
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_BRAM_VR_CORE"
                            } else {
                                "RCLK_BRAM_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        0,
                    ),
                    ColumnKind::Bram(BramKind::ClkBuf) => (
                        "BRAM_CLKBUF",
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_BRAM_CLKBUF_VR_CORE"
                            } else {
                                "RCLK_BRAM_CLKBUF_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        0,
                    ),
                    ColumnKind::Bram(BramKind::ClkBufNoPd) => (
                        "BRAM_CLKBUF_NOPD",
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_BRAM_CLKBUF_NOPD_VR_CORE"
                            } else {
                                "RCLK_BRAM_CLKBUF_NOPD_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        0,
                    ),
                    ColumnKind::Bram(BramKind::MaybeClkBufNoPd) => (
                        if row.to_idx() < chip.get_ps_height() {
                            "BRAM"
                        } else {
                            "BRAM_CLKBUF_NOPD"
                        },
                        int_grid.name(
                            if row.to_idx() < chip.get_ps_height() {
                                if chip.is_vr {
                                    "RCLK_BRAM_VR_CORE"
                                } else {
                                    "RCLK_BRAM_CORE"
                                }
                            } else {
                                if chip.is_vr {
                                    "RCLK_BRAM_CLKBUF_NOPD_VR_CORE"
                                } else {
                                    "RCLK_BRAM_CLKBUF_NOPD_CORE"
                                }
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        if row.to_idx() < chip.get_ps_height() {
                            0
                        } else {
                            1
                        },
                    ),
                    ColumnKind::Uram => (
                        "URAM",
                        int_grid.name(
                            if chip.is_vr {
                                "RCLK_URAM_VR_CORE"
                            } else {
                                "RCLK_URAM_CORE"
                            },
                            die,
                            col,
                            srow,
                            0,
                            0,
                        ),
                        0,
                    ),
                    _ => unreachable!(),
                };
                let vr = if chip.is_vr { "_VR" } else { "" };
                let ntile = ngrid.name_tile(tcrd, &format!("{kind}_{subkind}{vr}"), [name]);
                ntile.add_bel(
                    bslots::RCLK_DFX_TEST,
                    rclk_dfx_grid.name("RCLK", die, col, row, dx, 0),
                );
            }
            tcls::RCLK_HDIO | tcls::RCLK_HB_HDIO => {
                let srow = if reg.to_idx() % 2 == 1 { row - 1 } else { row };
                let naming = if chip.is_vr {
                    format!("{kind}_VR")
                } else {
                    kind.to_string()
                };
                let name = int_grid.name(
                    &if chip.is_vr {
                        format!("{kind}_VR_CORE")
                    } else {
                        format!("{kind}_CORE")
                    },
                    die,
                    col - 1,
                    srow,
                    0,
                    0,
                );
                ngrid.name_tile(tcrd, &naming, [name]);
            }
            tcls::CLE_W | tcls::CLE_W_VR => {
                let tkn = if !chip.is_vr {
                    "CLE_E_CORE"
                } else {
                    "CLE_E_VR_CORE"
                };
                let ntile = ngrid.name_tile(tcrd, kind, [int_grid.name(tkn, die, col, row, 0, 0)]);
                for i in 0..2 {
                    ntile.add_bel(
                        bslots::SLICE[i],
                        slice_grid.name("SLICE", die, col, row, i as i32, 0),
                    );
                }
            }
            tcls::CLE_E | tcls::CLE_E_VR => {
                let tkn = if !chip.is_vr {
                    "CLE_W_CORE"
                } else {
                    "CLE_W_VR_CORE"
                };
                let ntile = ngrid.name_tile(tcrd, kind, [int_grid.name(tkn, die, col, row, 0, 0)]);
                for i in 0..2 {
                    ntile.add_bel(
                        bslots::SLICE[i],
                        slice_grid.name("SLICE", die, col, row, i as i32, 0),
                    );
                }
            }
            tcls::DSP => {
                let ocf = if col < edev.col_cfrm[die] {
                    "LOCF"
                } else {
                    "ROCF"
                };
                let bt = if chip.is_reg_n(reg) { 'T' } else { 'B' };
                let name = int_grid.name(&format!("DSP_{ocf}_{bt}_TILE"), die, col, row, 0, 0);
                let ntile = ngrid.name_tile(
                    tcrd,
                    if dev_naming.is_dsp_v2 {
                        "DSP_V2"
                    } else {
                        "DSP_V1"
                    },
                    [name],
                );
                for i in 0..2 {
                    ntile.add_bel(
                        bslots::DSP[i],
                        dsp_grid.name_mult("DSP", die, col, row, 2, i as i32, 1, 0),
                    );
                }
                ntile.add_bel(
                    bslots::DSP_CPLX,
                    dsp_grid.name("DSP58_CPLX", die, col, row, 0, 0),
                );
            }
            tcls::BRAM_E | tcls::BRAM_W => {
                let lr = if kind == "BRAM_W" { 'L' } else { 'R' };
                let ocf = if col < edev.col_cfrm[die] {
                    "LOCF"
                } else {
                    "ROCF"
                };
                let bt = if chip.is_reg_n(reg) { 'T' } else { 'B' };
                let name = int_grid.name(&format!("BRAM_{ocf}_{bt}{lr}_TILE"), die, col, row, 0, 0);
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(
                    bslots::BRAM_F,
                    bram_grid.name("RAMB36", die, col, row, 0, 0),
                );
                for i in 0..2 {
                    ntile.add_bel(
                        bslots::BRAM_H[i],
                        bram_grid.name_mult("RAMB18", die, col, row, 1, 0, 2, i as i32),
                    );
                }
            }
            tcls::URAM | tcls::URAM_DELAY => {
                let ocf = if col < edev.col_cfrm[die] {
                    "LOCF"
                } else {
                    "ROCF"
                };
                let bt = if chip.is_reg_n(reg) { 'T' } else { 'B' };
                let name = int_grid.name(&format!("{kind}_{ocf}_{bt}L_TILE"), die, col, row, 0, 0);
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(bslots::URAM, uram_grid.name("URAM288", die, col, row, 0, 0));
                if kind == "URAM_DELAY" {
                    ntile.add_bel(
                        bslots::URAM_CAS_DLY,
                        uram_delay_grid.name("URAM_CAS_DLY", die, col, row, 0, 0),
                    );
                }
            }
            tcls::PCIE4
            | tcls::PCIE5
            | tcls::MRMAC
            | tcls::SDFEC
            | tcls::DFE_CFC_S
            | tcls::DFE_CFC_N => {
                let (slot, tk, bk, bel_grid) = match tile.class {
                    tcls::PCIE4 => (bslots::PCIE4, "PCIEB", "PCIE40", &pcie4_grid),
                    tcls::PCIE5 => (bslots::PCIE5, "PCIEB5", "PCIE50", &pcie5_grid),
                    tcls::MRMAC => (bslots::MRMAC, "MRMAC", "MRMAC", &mrmac_grid),
                    tcls::SDFEC => (bslots::SDFEC, "SDFECA", "SDFEC_A", &sdfec_grid),
                    tcls::DFE_CFC_S => (
                        bslots::DFE_CFC_S,
                        "DFE_CFC",
                        "DFE_CFC_BOT",
                        &dfe_cfc_bot_grid,
                    ),
                    tcls::DFE_CFC_N => (
                        bslots::DFE_CFC_N,
                        "DFE_CFC",
                        "DFE_CFC_TOP",
                        &dfe_cfc_top_grid,
                    ),
                    _ => unreachable!(),
                };
                let bt = if chip.is_reg_n(reg) { "TOP" } else { "BOT" };
                let name = int_grid.name(&format!("{tk}_{bt}_TILE"), die, col, row, 0, 0);
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(slot, bel_grid.name(bk, die, col, row, 0, 0));
            }
            tcls::DCMAC | tcls::ILKN | tcls::HSC => {
                let (slot, bk, bel_grid) = match tile.class {
                    tcls::DCMAC => (bslots::DCMAC, "DCMAC", &dcmac_grid),
                    tcls::ILKN => (bslots::ILKN, "ILKNF", &ilkn_grid),
                    tcls::HSC => (bslots::HSC, "HSC", &hsc_grid),
                    _ => unreachable!(),
                };
                let name = int_grid.name(&format!("{kind}_TILE"), die, col, row, 0, 0);
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                ntile.add_bel(slot, bel_grid.name(bk, die, col, row, 0, 0));
            }
            tcls::HDIO => {
                let name = int_grid.name(
                    if chip.is_reg_n(reg) {
                        "HDIO_TILE"
                    } else {
                        "HDIO_BOT_TILE"
                    },
                    die,
                    col,
                    row,
                    0,
                    0,
                );
                let ntile = ngrid.name_tile(tcrd, kind, [name]);
                let naming = &dev_naming.die[die].hdio[&(col, reg)];
                for i in 0..11 {
                    ntile.add_bel(
                        bslots::HDIOLOGIC[i],
                        hdio_grid.name_mult("HDIOLOGIC", die, col, row, 1, 0, 11, i as i32),
                    );
                }
                for i in 0..11 {
                    ntile.add_bel(
                        bslots::HDIOB[i],
                        hdio_grid.name_manual(
                            "IOB",
                            die,
                            naming.iob_xy.0,
                            naming.iob_xy.1 + i as u32,
                        ),
                    );
                }
                for i in 0..4 {
                    ntile.add_bel(
                        bslots::BUFGCE_HDIO[i],
                        hdio_grid.name_mult("BUFGCE_HDIO", die, col, row, 1, 0, 4, i as i32),
                    );
                }
                ntile.add_bel(
                    bslots::DPLL_HDIO,
                    hdio_grid.name_manual("DPLL", die, naming.dpll_xy.0, naming.dpll_xy.1),
                );
                ntile.add_bel(
                    bslots::HDIO_BIAS,
                    hdio_grid.name("HDIO_BIAS", die, col, row, 0, 0),
                );
                ntile.add_bel(
                    bslots::RPI_HD_APB,
                    hdio_grid.name("RPI_HD_APB", die, col, row, 0, 0),
                );
                ntile.add_bel(
                    bslots::HDLOGIC_APB,
                    hdio_grid.name("HDLOGIC_APB", die, col, row, 0, 0),
                );
            }
            tcls::VNOC => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        int_grid.name("NOC_NSU512_TOP", die, col, row + 7, 0, 0),
                        int_grid.name("NOC_NPS_VNOC_TOP", die, col, row + 15, 0, 0),
                        int_grid.name("NOC_NPS_VNOC_TOP", die, col, row + 23, 0, 0),
                        int_grid.name("NOC_NMU512_TOP", die, col, row + 31, 0, 0),
                    ],
                );
                ntile.add_bel(
                    bslots::VNOC_NSU512,
                    vnoc_grid.name("NOC_NSU512", die, col, row, 0, 0),
                );
                ntile.add_bel(
                    bslots::VNOC_NPS_A,
                    vnoc_grid.name_mult("NOC_NPS_VNOC", die, col, row, 1, 0, 2, 0),
                );
                ntile.add_bel(
                    bslots::VNOC_NPS_B,
                    vnoc_grid.name_mult("NOC_NPS_VNOC", die, col, row, 1, 0, 2, 1),
                );
                ntile.add_bel(
                    bslots::VNOC_NMU512,
                    vnoc_grid.name("NOC_NMU512", die, col, row, 0, 0),
                );
            }
            tcls::VNOC2 => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        int_grid.name("NOC2_NSU512_VNOC_TILE", die, col, row + 7, 0, 0),
                        int_grid.name("NOC2_NPS5555_TOP", die, col, row + 11, 0, 0),
                        int_grid.name("NOC2_NPS5555_TOP", die, col, row + 14, 0, 0),
                        int_grid.name("NOC2_NMU512_VNOC_TILE", die, col, row + 16, 0, 0),
                        int_grid.name(
                            "NOC2_SCAN_TOP",
                            die,
                            if dev_naming.is_vnoc2_scan_offset {
                                col + 1
                            } else {
                                col
                            },
                            row + 7,
                            0,
                            0,
                        ),
                    ],
                );
                let naming = &dev_naming.die[die].vnoc2[&(col, reg)];
                ntile.add_bel(
                    bslots::VNOC2_NSU512,
                    vnoc_grid.name_manual("NOC2_NSU512", die, naming.nsu_xy.0, naming.nsu_xy.1),
                );
                ntile.add_bel(
                    bslots::VNOC2_NPS_A,
                    vnoc_grid.name_manual("NOC2_NPS5555", die, naming.nps_xy.0, naming.nps_xy.1),
                );
                ntile.add_bel(
                    bslots::VNOC2_NPS_B,
                    vnoc_grid.name_manual(
                        "NOC2_NPS5555",
                        die,
                        naming.nps_xy.0,
                        naming.nps_xy.1 + 1,
                    ),
                );
                ntile.add_bel(
                    bslots::VNOC2_NMU512,
                    vnoc_grid.name_manual("NOC2_NMU512", die, naming.nmu_xy.0, naming.nmu_xy.1),
                );
                ntile.add_bel(
                    bslots::VNOC2_SCAN,
                    vnoc_grid.name_manual("NOC2_SCAN", die, naming.scan_xy.0, naming.scan_xy.1),
                );
            }
            tcls::VNOC4 => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [
                        int_grid.name("NOC2_NSU512_VNOC4_TILE", die, col, row + 7, 0, 0),
                        int_grid.name("NOC2_NPS6X_TOP", die, col, row + 11, 0, 0),
                        int_grid.name("NOC2_NPS6X_TOP", die, col, row + 14, 0, 0),
                        int_grid.name("NOC2_NMU512_VNOC4_TILE", die, col, row + 16, 0, 0),
                        int_grid.name("NOC2_SCAN_TOP", die, col, row + 7, 0, 0),
                    ],
                );
                let naming = &dev_naming.die[die].vnoc2[&(col, reg)];
                ntile.add_bel(
                    bslots::VNOC4_NSU512,
                    vnoc_grid.name_manual("NOC2_NSU512", die, naming.nsu_xy.0, naming.nsu_xy.1),
                );
                ntile.add_bel(
                    bslots::VNOC4_NPS_A,
                    vnoc_grid.name_manual("NOC2_NPS6X", die, naming.nps_xy.0, naming.nps_xy.1),
                );
                ntile.add_bel(
                    bslots::VNOC4_NPS_B,
                    vnoc_grid.name_manual("NOC2_NPS6X", die, naming.nps_xy.0, naming.nps_xy.1 + 1),
                );
                ntile.add_bel(
                    bslots::VNOC4_NMU512,
                    vnoc_grid.name_manual("NOC2_NMU512", die, naming.nmu_xy.0, naming.nmu_xy.1),
                );
                ntile.add_bel(
                    bslots::VNOC4_SCAN,
                    vnoc_grid.name_manual("NOC2_SCAN", die, naming.scan_xy.0, naming.scan_xy.1),
                );
            }
            tcls::MISR => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [int_grid.name(
                        "MISR_TILE",
                        die,
                        col,
                        if reg.to_idx().is_multiple_of(2) {
                            row
                        } else {
                            row - 1
                        },
                        0,
                        0,
                    )],
                );
                ntile.add_bel(bslots::MISR, misr_grid.name("MISR", die, col, row, 0, 0));
            }
            tcls::SYSMON_SAT_VNOC => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [int_grid.name("AMS_SAT_VNOC_TILE", die, col, row + 39, 0, 0)],
                );
                let (sx, sy) = dev_naming.die[die].sysmon_sat_vnoc[&(col, reg)];
                ntile.add_bel(
                    bslots::SYSMON_SAT_VNOC,
                    vnoc_grid.name_manual("SYSMON_SAT", die, sx, sy),
                );
            }
            tcls::SYSMON_SAT_GT_W | tcls::SYSMON_SAT_GT_E => {
                let bt = if chip.is_reg_n(reg) { "TOP" } else { "BOT" };
                let ntile =
                    ngrid.name_tile(
                        tcrd,
                        kind,
                        [int_grid_gt.name(
                            &format!("AMS_SAT_GT_{bt}_TILE"),
                            die,
                            col,
                            row + 19,
                            0,
                            0,
                        )],
                    );
                let (sx, sy) = dev_naming.die[die].sysmon_sat_gt[&(col, reg)];
                ntile.add_bel(
                    bslots::SYSMON_SAT_GT,
                    vnoc_grid.name_manual("SYSMON_SAT", die, sx, sy),
                );
            }
            tcls::DPLL_GT_W | tcls::DPLL_GT_E => {
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [int_grid_gt.name("CMT_DPLL", die, col, row + 7, 0, 0)],
                );
                let (sx, sy) = dev_naming.die[die].dpll_gt[&(col, reg)];
                ntile.add_bel(bslots::DPLL_GT, vnoc_grid.name_manual("DPLL", die, sx, sy));
            }
            tcls::BFR_B_E => {
                let bt = if chip.is_reg_n(reg) { "TOP" } else { "BOT" };
                let ntile = ngrid.name_tile(
                    tcrd,
                    kind,
                    [int_grid.name(&format!("BFR_TILE_B_{bt}_CORE"), die, col, row, 0, 0)],
                );
                ntile.add_bel(bslots::BFR_B, bfr_b_grid.name("BFR_B", die, col, row, 0, 0));
            }
            tcls::VDU_E => {
                let ntile =
                    ngrid.name_tile(tcrd, kind, [int_grid.name("VDU_CORE", die, col, row, 0, 0)]);
                ntile.add_bel(bslots::VDU, vdu_grid.name("VDU", die, col, row, 0, 0));
            }

            _ => panic!("how to {kind}"),
        }
    }

    ExpandedNamedDevice { edev, ngrid }
}
