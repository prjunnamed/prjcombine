use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    dir::Dir,
    grid::{ColId, DieId, RowId},
};
use prjcombine_re_xilinx_naming_versal::{DeviceNaming, DieNaming, HdioNaming, VNoc2Naming};
use prjcombine_re_xilinx_rawdump::{Coord, NodeOrWire, Part, Tile, TkSiteSlot};
use prjcombine_versal::chip::{
    BotKind, BramKind, Chip, CleKind, Column, ColumnKind, CpmKind, DisabledPart, GtRowKind,
    HardColumn, HardRowKind, Interposer, InterposerKind, PsKind, RegId, RightKind, TopKind,
};
use std::collections::{BTreeMap, BTreeSet};

use prjcombine_re_xilinx_rd2db_grid::{
    IntGrid, extract_int_slr, extract_int_slr_column, find_rows,
};

fn adjust_column(col: ColId, side: Dir) -> ColId {
    match side {
        Dir::W => ColId::from_idx(col.to_idx() * 2),
        Dir::E => ColId::from_idx(col.to_idx() * 2 + 1),
        _ => unreachable!(),
    }
}

struct IntGridWrapper<'a> {
    int: IntGrid<'a>,
}

impl IntGridWrapper<'_> {
    pub fn num_cols(&self) -> usize {
        self.int.cols.len() * 2
    }

    pub fn find_columns(&self, tts: &[&str]) -> Vec<i32> {
        self.int.find_columns(tts)
    }

    pub fn find_column(&self, tts: &[&str]) -> Option<i32> {
        self.int.find_column(tts)
    }

    pub fn find_tiles(&self, tts: &[&str]) -> BTreeSet<(i32, i32)> {
        self.int.find_tiles(tts)
    }

    pub fn find_rows(&self, tts: &[&str]) -> Vec<i32> {
        self.int.find_rows(tts)
    }

    pub fn lookup_column_inter(&self, col: i32) -> ColId {
        adjust_column(self.int.lookup_column_inter(col), Dir::W)
    }

    pub fn lookup_row(&self, row: i32) -> RowId {
        self.int.lookup_row(row)
    }

    pub fn delta(&self, xy: Coord, dx: i32, dy: i32) -> Coord {
        self.int.delta(xy, dx, dy)
    }
}

fn split_xy(s: &str) -> Option<(&str, u32, u32)> {
    let (l, r) = s.rsplit_once("_X")?;
    let (x, y) = r.rsplit_once('Y')?;
    let x = x.parse().ok()?;
    let y = y.parse().ok()?;
    Some((l, x, y))
}

fn split_sxy(s: &str) -> Option<(&str, u32, u32, u32)> {
    let (l, r) = s.rsplit_once("_S")?;
    let (s, r) = r.rsplit_once("X")?;
    let (x, y) = r.rsplit_once('Y')?;
    let s = s.parse().ok()?;
    let x = x.parse().ok()?;
    let y = y.parse().ok()?;
    Some((l, s, x, y))
}

fn split_xy_sxy(s: &str) -> Option<(&str, Option<u32>, u32, u32)> {
    if let Some((p, x, y)) = split_xy(s) {
        Some((p, None, x, y))
    } else if let Some((p, s, x, y)) = split_sxy(s) {
        Some((p, Some(s), x, y))
    } else {
        None
    }
}

fn extract_site_xy(rd: &Part, tile: &Tile, sname: &str) -> Option<(u32, u32)> {
    let tk = &rd.tile_kinds[tile.kind];
    let tks = TkSiteSlot::Xy(rd.slot_kinds.get(sname)?, 0, 0);
    let si = tk.sites.get(&tks)?.0;
    let name = tile.sites.get(si)?;
    let (_, _, x, y) = split_xy_sxy(name).unwrap();
    Some((x, y))
}

fn make_columns(
    die: DieId,
    int: &IntGridWrapper,
    disabled: &mut BTreeSet<DisabledPart>,
    naming: &mut DieNaming,
) -> (EntityVec<ColId, Column>, Vec<HardColumn>) {
    let mut res: EntityVec<ColId, Column> = (0..int.num_cols())
        .map(|_| Column {
            kind: ColumnKind::None,
            has_bli_s: false,
            has_bli_n: false,
        })
        .collect();

    for (tkn, kind_w, kind_e) in [
        (
            "CLE_W_CORE",
            ColumnKind::Cle(CleKind::Plain),
            ColumnKind::Cle(CleKind::Plain),
        ),
        (
            "CLE_W_VR_CORE",
            ColumnKind::Cle(CleKind::Plain),
            ColumnKind::Cle(CleKind::Plain),
        ),
        ("DSP_LOCF_B_TILE", ColumnKind::Dsp, ColumnKind::ContDsp),
        ("DSP_LOCF_T_TILE", ColumnKind::Dsp, ColumnKind::ContDsp),
        ("DSP_ROCF_B_TILE", ColumnKind::Dsp, ColumnKind::ContDsp),
        ("DSP_ROCF_T_TILE", ColumnKind::Dsp, ColumnKind::ContDsp),
        ("NOC_NSU512_TOP", ColumnKind::VNoc, ColumnKind::ContVNoc),
        (
            "NOC2_NSU512_VNOC_TILE",
            ColumnKind::VNoc2,
            ColumnKind::ContVNoc,
        ),
        (
            "NOC2_NSU512_VNOC4_TILE",
            ColumnKind::VNoc4,
            ColumnKind::ContVNoc,
        ),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c - 1].kind = kind_w;
            res[c].kind = kind_e;
        }
    }
    for (tkn, kind) in [
        ("BRAM_LOCF_TR_TILE", ColumnKind::Bram(BramKind::Plain)),
        ("BRAM_LOCF_BR_TILE", ColumnKind::Bram(BramKind::Plain)),
        ("BRAM_ROCF_TR_TILE", ColumnKind::Bram(BramKind::Plain)),
        ("BRAM_ROCF_BR_TILE", ColumnKind::Bram(BramKind::Plain)),
        ("INTF_GT_TR_TILE", ColumnKind::Gt),
        ("INTF_GT_BR_TILE", ColumnKind::Gt),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c - 1].kind = kind;
        }
    }
    for (tkn, kind) in [
        ("BRAM_LOCF_TL_TILE", ColumnKind::Bram(BramKind::Plain)),
        ("BRAM_LOCF_BL_TILE", ColumnKind::Bram(BramKind::Plain)),
        ("BRAM_ROCF_TL_TILE", ColumnKind::Bram(BramKind::Plain)),
        ("BRAM_ROCF_BL_TILE", ColumnKind::Bram(BramKind::Plain)),
        ("URAM_LOCF_TL_TILE", ColumnKind::Uram),
        ("URAM_LOCF_BL_TILE", ColumnKind::Uram),
        ("URAM_ROCF_TL_TILE", ColumnKind::Uram),
        ("URAM_ROCF_BL_TILE", ColumnKind::Uram),
        ("INTF_GT_TL_TILE", ColumnKind::Gt),
        ("INTF_GT_BL_TILE", ColumnKind::Gt),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c].kind = kind;
        }
    }
    for (tkn, kind) in [("SLL", CleKind::Sll), ("SLL2", CleKind::Sll2)] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            assert_eq!(res[c].kind, ColumnKind::Cle(CleKind::Plain));
            assert_eq!(res[c - 1].kind, ColumnKind::Cle(CleKind::Plain));
            res[c].kind = ColumnKind::Cle(kind);
            res[c - 1].kind = ColumnKind::Cle(kind);
        }
    }
    for c in int.find_columns(&["RCLK_BRAM_CLKBUF_CORE", "RCLK_BRAM_CLKBUF_VR_CORE"]) {
        let c = int.lookup_column_inter(c);
        assert_eq!(res[c - 1].kind, ColumnKind::Bram(BramKind::Plain));
        res[c - 1].kind = ColumnKind::Bram(BramKind::ClkBuf);
    }
    for c in int.find_columns(&[
        "RCLK_BRAM_CLKBUF_NOPD_CORE",
        "RCLK_BRAM_CLKBUF_NOPD_VR_CORE",
    ]) {
        let c = int.lookup_column_inter(c);
        assert_eq!(res[c - 1].kind, ColumnKind::Bram(BramKind::Plain));
        res[c - 1].kind = ColumnKind::Bram(BramKind::ClkBufNoPd);
    }
    for c in int.find_columns(&["RCLK_BRAM_CORE", "RCLK_BRAM_VR_CORE"]) {
        let c = int.lookup_column_inter(c);
        if res[c - 1].kind == ColumnKind::Bram(BramKind::ClkBufNoPd) {
            res[c - 1].kind = ColumnKind::Bram(BramKind::MaybeClkBufNoPd);
        }
    }

    for c in int.find_columns(&[
        "BLI_CLE_TOP_CORE",
        "BLI_DSP_LOCF_TR_TILE",
        "BLI_DSP_ROCF_TR_TILE",
        "BLI_BRAM_LOCF_TR_TILE",
        "BLI_BRAM_ROCF_TR_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c - 1].has_bli_n = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_TOP_CORE_MY",
        "BLI_DSP_LOCF_TL_TILE",
        "BLI_DSP_ROCF_TL_TILE",
        "BLI_BRAM_ROCF_TL_TILE",
        "BLI_URAM_LOCF_TL_TILE",
        "BLI_URAM_ROCF_TL_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c].has_bli_n = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_BOT_CORE",
        "BLI_DSP_ROCF_BR_TILE",
        "BLI_BRAM_ROCF_BR_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c - 1].has_bli_s = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_BOT_CORE_MY",
        "BLI_DSP_ROCF_BL_TILE",
        "BLI_BRAM_ROCF_BL_TILE",
        "BLI_URAM_ROCF_BL_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c].has_bli_s = true;
    }

    let col_cfrm = int.lookup_column_inter(
        int.find_column(&["CFRM_PMC_TILE", "CFRM_PMC_VR_TILE"])
            .unwrap(),
    );
    res[col_cfrm].kind = ColumnKind::Cfrm;

    let mut hard_cells = BTreeMap::new();
    for (tt, kind) in [
        ("HDIO_TILE", HardRowKind::Hdio),
        ("HDIO_BOT_TILE", HardRowKind::Hdio),
        ("PCIEB_TOP_TILE", HardRowKind::Pcie4),
        ("PCIEB_BOT_TILE", HardRowKind::Pcie4),
        ("PCIEB5_TOP_TILE", HardRowKind::Pcie5),
        ("PCIEB5_BOT_TILE", HardRowKind::Pcie5),
        ("MRMAC_TOP_TILE", HardRowKind::Mrmac),
        ("MRMAC_BOT_TILE", HardRowKind::Mrmac),
        ("SDFECA_TOP_TILE", HardRowKind::SdfecA),
        ("DFE_CFC_BOT_TILE", HardRowKind::DfeCfcS),
        ("DFE_CFC_TOP_TILE", HardRowKind::DfeCfcN),
        ("CPM_EXT_TILE", HardRowKind::CpmExt),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let tile = &int.int.rd.tiles[&Coord {
                x: x as u16,
                y: y as u16,
            }];
            let col = int.lookup_column_inter(x) - 1;
            let reg = RegId::from_idx(int.lookup_row(y).to_idx() / 48);
            if tile.sites.iter().next().is_none() {
                disabled.insert(DisabledPart::HardIpSite(die, col, reg));
            }
            hard_cells.insert((col, reg), kind);
            if tt.starts_with("HDIO") {
                let iob_xy = extract_site_xy(int.int.rd, tile, "IOB").unwrap();
                let dpll_xy = extract_site_xy(int.int.rd, tile, "DPLL").unwrap_or_else(|| {
                    disabled.insert(DisabledPart::HdioDpll(die, col, reg));
                    let is_vc1902 = ["vc1902", "vc1802", "vm1802", "v65"]
                        .into_iter()
                        .any(|x| int.int.rd.part.contains(x));
                    if is_vc1902 {
                        let dpll_x = match col.to_idx() {
                            9 => 3,
                            215 => 12,
                            _ => unreachable!(),
                        };
                        (dpll_x, 7)
                    } else {
                        panic!("MISSING DPLL FOR UNK PART {part}", part = int.int.rd.part);
                    }
                });
                naming
                    .hdio
                    .insert((col, reg), HdioNaming { iob_xy, dpll_xy });
            }
        }
    }
    for (tt, kind_b, kind_t) in [
        ("ILKN_TILE", HardRowKind::IlknB, HardRowKind::IlknT),
        ("DCMAC_TILE", HardRowKind::DcmacB, HardRowKind::DcmacT),
        ("HSC_TILE", HardRowKind::HscB, HardRowKind::HscT),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let tile = &int.int.rd.tiles[&Coord {
                x: x as u16,
                y: y as u16,
            }];
            let col = int.lookup_column_inter(x) - 1;
            let reg = RegId::from_idx(int.lookup_row(y).to_idx() / 48);
            if tile.sites.iter().next().is_none() {
                disabled.insert(DisabledPart::HardIpSite(die, col, reg));
            }
            hard_cells.insert((col, reg), kind_b);
            hard_cells.insert((col, reg + 1), kind_t);
        }
    }
    let mut cols_hard = Vec::new();
    let cols: BTreeSet<ColId> = hard_cells.keys().map(|&(c, _)| c).collect();
    for col in cols {
        res[col].kind = ColumnKind::Hard;
        res[col + 1].kind = ColumnKind::ContHard;
        let mut regs = EntityVec::new();
        for _ in 0..(int.int.rows.len() / 48) {
            regs.push(HardRowKind::None);
        }
        for (&(c, r), &kind) in hard_cells.iter() {
            if c == col {
                assert_eq!(regs[r], HardRowKind::None);
                regs[r] = kind;
            }
        }
        cols_hard.push(HardColumn { col, regs });
    }
    (res, cols_hard)
}

fn get_cols_vbrk(int: &IntGridWrapper) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["CBRK_LOCF_TOP_TILE", "CBRK_TOP_TILE"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_cpipe(int: &IntGridWrapper) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["CPIPE_TOP_TILE"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_rows_gt_left(int: &IntGridWrapper) -> (EntityVec<RegId, GtRowKind>, bool) {
    let mut res = EntityVec::new();
    let mut has_xram_top = false;
    for _ in 0..(int.int.rows.len() / 48) {
        res.push(GtRowKind::None);
    }
    for (tkn, kind) in [
        ("GTY_QUAD_SINGLE_MY", GtRowKind::Gty),
        ("GTYP_QUAD_SINGLE_MY", GtRowKind::Gtyp),
        ("GTM_QUAD_SINGLE_MY", GtRowKind::Gtm),
        ("XRAM_CORE", GtRowKind::Xram),
    ] {
        for row in int.find_rows(&[tkn]) {
            let oob = if int.int.mirror_y {
                row < *int.int.rows.last().unwrap()
            } else {
                row > *int.int.rows.last().unwrap()
            };
            if oob {
                assert_eq!(tkn, "XRAM_CORE");
                has_xram_top = true;
            } else {
                let reg = RegId::from_idx(int.lookup_row(row).to_idx() / 48);
                res[reg] = kind;
            }
        }
    }
    (res, has_xram_top)
}

fn get_rows_gt_right(int: &IntGridWrapper) -> Option<EntityVec<RegId, GtRowKind>> {
    let mut res = EntityVec::new();
    for _ in 0..(int.int.rows.len() / 48) {
        res.push(GtRowKind::None);
    }
    for (tkn, kind) in [
        ("GTY_QUAD_SINGLE", GtRowKind::Gty),
        ("GTYP_QUAD_SINGLE", GtRowKind::Gtyp),
        ("GTM_QUAD_SINGLE", GtRowKind::Gtm),
        ("VDU_CORE_MY", GtRowKind::Vdu),
        ("BFR_TILE_B_BOT_CORE", GtRowKind::BfrB),
        ("BFR_TILE_B_TOP_CORE", GtRowKind::BfrB),
        ("ISP2_CORE", GtRowKind::Isp2),
        ("VCU2_TILE", GtRowKind::Vcu2B),
        ("RFADC_BOT_CORE", GtRowKind::RfAdc),
        ("RFADC_TOP_CORE", GtRowKind::RfAdc),
        ("RFDAC_BOT_CORE", GtRowKind::RfDac),
        ("RFDAC_TOP_CORE", GtRowKind::RfDac),
    ] {
        for row in int.find_rows(&[tkn]) {
            let reg = RegId::from_idx(int.lookup_row(row).to_idx() / 48);
            res[reg] = kind;
            if kind == GtRowKind::Vcu2B {
                res[reg + 1] = GtRowKind::Vcu2T;
            }
        }
    }
    if res.values().any(|&x| x != GtRowKind::None) {
        Some(res)
    } else {
        None
    }
}

fn get_vnoc_naming(int: &IntGridWrapper, naming: &mut DieNaming, is_vnoc2_scan_offset: &mut bool) {
    for (x, y) in int.find_tiles(&["AMS_SAT_VNOC_TILE"]) {
        let col = int.lookup_column_inter(x) - 1;
        let reg = RegId::from_idx(int.lookup_row(y + 1).to_idx() / 48);
        let tile = &int.int.rd.tiles[&Coord {
            x: x as u16,
            y: y as u16,
        }];
        if let Some(xy) = extract_site_xy(int.int.rd, tile, "SYSMON_SAT") {
            naming.sysmon_sat_vnoc.insert((col, reg), xy);
        }
    }
    for (x, y) in int.find_tiles(&["NOC2_NSU512_VNOC_TILE"]) {
        let col = int.lookup_column_inter(x) - 1;
        let reg = RegId::from_idx(int.lookup_row(y + 1).to_idx() / 48);
        let nsu_crd = Coord {
            x: x as u16,
            y: y as u16,
        };
        let mut nps_a_crd = int.delta(nsu_crd, 0, 4);
        if int.int.rd.tile_kinds.key(int.int.rd.tiles[&nps_a_crd].kind) == "NULL" {
            *is_vnoc2_scan_offset = true;
            nps_a_crd = int.delta(nps_a_crd, -1, 0);
        }
        let nmu_crd = int.delta(nps_a_crd, 0, 7);
        let scan_crd = int.delta(nsu_crd, 1, 0);
        naming.vnoc2.insert(
            (col, reg),
            VNoc2Naming {
                nsu_xy: extract_site_xy(int.int.rd, &int.int.rd.tiles[&nsu_crd], "NOC2_NSU512")
                    .unwrap(),
                nmu_xy: extract_site_xy(int.int.rd, &int.int.rd.tiles[&nmu_crd], "NOC2_NMU512")
                    .unwrap(),
                nps_xy: extract_site_xy(int.int.rd, &int.int.rd.tiles[&nps_a_crd], "NOC2_NPS5555")
                    .unwrap(),
                scan_xy: extract_site_xy(int.int.rd, &int.int.rd.tiles[&scan_crd], "NOC2_SCAN")
                    .unwrap(),
            },
        );
    }
    for (x, y) in int.find_tiles(&["NOC2_NSU512_VNOC4_TILE"]) {
        let col = int.lookup_column_inter(x) - 1;
        let reg = RegId::from_idx(int.lookup_row(y + 1).to_idx() / 48);
        let nsu_crd = Coord {
            x: x as u16,
            y: y as u16,
        };
        let mut nps_a_crd = int.delta(nsu_crd, 0, 4);
        if int.int.rd.tile_kinds.key(int.int.rd.tiles[&nps_a_crd].kind) == "NULL" {
            *is_vnoc2_scan_offset = true;
            nps_a_crd = int.delta(nps_a_crd, -1, 0);
        }
        let nmu_crd = int.delta(nps_a_crd, 0, 7);
        let scan_crd = int.delta(nsu_crd, 1, 0);
        naming.vnoc2.insert(
            (col, reg),
            VNoc2Naming {
                nsu_xy: extract_site_xy(int.int.rd, &int.int.rd.tiles[&nsu_crd], "NOC2_NSU512")
                    .unwrap(),
                nmu_xy: extract_site_xy(int.int.rd, &int.int.rd.tiles[&nmu_crd], "NOC2_NMU512")
                    .unwrap(),
                nps_xy: extract_site_xy(int.int.rd, &int.int.rd.tiles[&nps_a_crd], "NOC2_NPS6X")
                    .unwrap(),
                scan_xy: extract_site_xy(int.int.rd, &int.int.rd.tiles[&scan_crd], "NOC2_SCAN")
                    .unwrap(),
            },
        );
    }
}

fn get_gt_naming(int: &IntGridWrapper, naming: &mut DieNaming) {
    for tkn in [
        "AMS_SAT_GT_BOT_TILE",
        "AMS_SAT_GT_TOP_TILE",
        "AMS_SAT_GT_BOT_TILE_MY",
        "AMS_SAT_GT_TOP_TILE_MY",
    ] {
        for (x, y) in int.find_tiles(&[tkn]) {
            let xy = Coord {
                x: x as u16,
                y: y as u16,
            };
            let mut col = int.lookup_column_inter(x);
            if col.to_idx() != 0 {
                col -= 1;
            }
            let reg = RegId::from_idx(int.lookup_row(y + 1).to_idx() / 48);
            let tile = &int.int.rd.tiles[&xy];
            if let Some(xy) = extract_site_xy(int.int.rd, tile, "SYSMON_SAT") {
                naming.sysmon_sat_gt.insert((col, reg), xy);
            }
            let tile = &int.int.rd.tiles[&int.delta(xy, 0, -15)];
            if let Some(xy) = extract_site_xy(int.int.rd, tile, "DPLL") {
                naming.dpll_gt.insert((col, reg), xy);
            }
        }
    }
}

fn get_chip(
    die: DieId,
    int: &IntGridWrapper<'_>,
    disabled: &mut BTreeSet<DisabledPart>,
    is_vnoc2_scan_offset: &mut bool,
    sll_columns: &mut EntityVec<DieId, Vec<ColId>>,
) -> (Chip, DieNaming) {
    let mut naming = DieNaming {
        hdio: BTreeMap::new(),
        sysmon_sat_vnoc: BTreeMap::new(),
        sysmon_sat_gt: BTreeMap::new(),
        dpll_gt: BTreeMap::new(),
        vnoc2: BTreeMap::new(),
    };
    let (columns, cols_hard) = make_columns(die, int, disabled, &mut naming);
    let ps = if !int.find_tiles(&["PSS_BASE_CORE"]).is_empty() {
        PsKind::Ps9
    } else if !int.find_tiles(&["PSXL_CORE"]).is_empty() {
        PsKind::PsX
    } else if !int.find_tiles(&["PSXC_TILE"]).is_empty() {
        PsKind::PsXc
    } else {
        unreachable!()
    };
    let cpm = if !int.find_tiles(&["CPM_CORE"]).is_empty() {
        CpmKind::Cpm4
    } else if !int.find_tiles(&["CPM_G5_TILE"]).is_empty() {
        CpmKind::Cpm5
    } else if !int.find_tiles(&["CPM_G5N2X_TILE"]).is_empty() {
        CpmKind::Cpm5N
    } else {
        CpmKind::None
    };
    assert_eq!(int.int.rows.len() % 48, 0);
    let (regs_gt_left, has_xram_top) = get_rows_gt_left(int);
    let right = if !int.find_tiles(&["HNICX_TILE"]).is_empty() {
        RightKind::HNicX
    } else if let Some(gts) = get_rows_gt_right(int) {
        RightKind::Gt(gts)
    } else if !int.find_tiles(&["RCLK_CIDB_CORE"]).is_empty() {
        RightKind::Cidb
    } else if !int.find_tiles(&["RCLK_INTF_TERM2_RIGHT_CORE"]).is_empty() {
        RightKind::Term2
    } else {
        RightKind::Term
    };
    let is_vr = !int.find_tiles(&["CLE_W_VR_CORE"]).is_empty();
    let chip = Chip {
        columns,
        cols_vbrk: get_cols_vbrk(int),
        cols_cpipe: get_cols_cpipe(int),
        cols_hard,
        regs: int.int.rows.len() / 48,
        regs_gt_left,
        ps,
        cpm,
        has_xram_top,
        is_vr,
        top: TopKind::Ssit,    // XXX
        bottom: BotKind::Ssit, // XXX
        right,
    };
    get_vnoc_naming(int, &mut naming, is_vnoc2_scan_offset);
    get_gt_naming(int, &mut naming);
    let mut die_sll_columns = BTreeSet::new();
    for (x, y) in int.find_tiles(&["SLL", "SLL2"]) {
        let crd = Coord {
            x: x as u16,
            y: y as u16,
        };
        let Some(nw) = int.int.rd.lookup_wire(crd, "UBUMP2") else {
            continue;
        };
        let NodeOrWire::Node(node) = nw else {
            continue;
        };
        let node = &int.int.rd.nodes[node];
        let templ = &int.int.rd.templates[node.template];
        if templ.len() > 1 {
            let col = int.lookup_column_inter(x) - 1;
            die_sll_columns.insert(col);
        }
    }
    sll_columns.push(Vec::from_iter(die_sll_columns));
    (chip, naming)
}

pub fn make_chips(
    rd: &Part,
) -> (
    EntityVec<DieId, Chip>,
    Interposer,
    BTreeSet<DisabledPart>,
    DeviceNaming,
) {
    let mut disabled = BTreeSet::new();
    let crd = rd.tiles_by_kind_name("INT").first().unwrap();
    let tile = &rd.tiles[crd];
    let mut ikind = if tile.name.contains("_S") {
        InterposerKind::MirrorSquare
    } else {
        InterposerKind::Column
    };
    let mut chips = EntityVec::new();
    let mut namings = EntityVec::new();
    let mut is_vnoc2_scan_offset = false;
    let mut sll_columns = EntityVec::new();
    if ikind == InterposerKind::Column {
        let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["NOC_TNOC_BRIDGE_BOT_CORE"])
            .into_iter()
            .map(|r| r as u16)
            .collect();
        if rows_slr_split.is_empty() {
            ikind = InterposerKind::Single;
        }
        rows_slr_split.insert(0);
        rows_slr_split.insert(rd.height);
        let rows_slr_split: Vec<_> = rows_slr_split.iter().collect();
        for (dieid, w) in rows_slr_split.windows(2).enumerate() {
            let int = extract_int_slr_column(rd, &["INT"], &[], *w[0], *w[1]);
            let int = IntGridWrapper { int };
            let die = DieId::from_idx(dieid);
            let (chip, naming) = get_chip(
                die,
                &int,
                &mut disabled,
                &mut is_vnoc2_scan_offset,
                &mut sll_columns,
            );
            chips.push(chip);
            namings.push(naming);
        }
    } else {
        for (dieid, int) in [
            extract_int_slr(
                rd,
                &["INT"],
                &[],
                0,
                rd.width / 2,
                0,
                rd.height / 2,
                false,
                false,
            ),
            extract_int_slr(
                rd,
                &["INT"],
                &[],
                0,
                rd.width / 2,
                rd.height / 2,
                rd.height,
                false,
                true,
            ),
            extract_int_slr(
                rd,
                &["INT"],
                &[],
                rd.width / 2,
                rd.width,
                rd.height / 2,
                rd.height,
                true,
                true,
            ),
            extract_int_slr(
                rd,
                &["INT"],
                &[],
                rd.width / 2,
                rd.width,
                0,
                rd.height / 2,
                true,
                false,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let int = IntGridWrapper { int };
            let die = DieId::from_idx(dieid);
            let (chip, naming) = get_chip(
                die,
                &int,
                &mut disabled,
                &mut is_vnoc2_scan_offset,
                &mut sll_columns,
            );
            chips.push(chip);
            namings.push(naming);
        }
    }
    if rd.part.contains("vc1502") {
        let s0 = DieId::from_idx(0);
        assert_eq!(chips[s0].regs, 7);
        let col_hard_r = &mut chips[s0].cols_hard[1];
        for (reg, kind) in [(0, HardRowKind::Mrmac), (6, HardRowKind::Hdio)] {
            let reg = RegId::from_idx(reg);
            assert_eq!(col_hard_r.regs[reg], HardRowKind::None);
            col_hard_r.regs[reg] = kind;
            disabled.insert(DisabledPart::HardIp(s0, col_hard_r.col, reg));
        }
        let RightKind::Gt(ref mut regs_gt_r) = chips[s0].right else {
            unreachable!()
        };
        for reg in [0, 1, 6] {
            let reg = RegId::from_idx(reg);
            assert_eq!(regs_gt_r[reg], GtRowKind::None);
            regs_gt_r[reg] = GtRowKind::Gty;
            disabled.insert(DisabledPart::GtRight(s0, reg));
        }
    }
    if rd.part.contains("vm1302") {
        let s0 = DieId::from_idx(0);
        assert_eq!(chips[s0].regs, 9);
        assert_eq!(chips[s0].columns.len(), 76);
        while chips[s0].columns.len() != 122 {
            chips[s0].columns.push(Column {
                kind: ColumnKind::None,
                has_bli_s: false,
                has_bli_n: false,
            });
        }
        for i in [
            73, 75, 77, 81, 83, 87, 89, 91, 95, 97, 99, 103, 105, 107, 111, 113, 117, 119,
        ] {
            let col = ColId::from_idx(i);
            chips[s0].columns[col].kind = ColumnKind::Cle(CleKind::Plain);
            chips[s0].columns[col + 1].kind = ColumnKind::Cle(CleKind::Plain);
            chips[s0].columns[col].has_bli_s = true;
            chips[s0].columns[col].has_bli_n = true;
            chips[s0].columns[col + 1].has_bli_s = true;
            chips[s0].columns[col + 1].has_bli_n = true;
        }
        for i in [79, 109] {
            let col = ColId::from_idx(i);
            chips[s0].columns[col].kind = ColumnKind::Dsp;
            chips[s0].columns[col + 1].kind = ColumnKind::ContDsp;
            chips[s0].columns[col].has_bli_s = true;
            chips[s0].columns[col].has_bli_n = true;
            chips[s0].columns[col + 1].has_bli_s = true;
            chips[s0].columns[col + 1].has_bli_n = true;
        }
        for i in [72, 86, 116] {
            let col = ColId::from_idx(i);
            chips[s0].columns[col].kind = ColumnKind::Bram(BramKind::Plain);
        }
        for i in [85, 101, 115] {
            let col = ColId::from_idx(i);
            chips[s0].columns[col].kind = ColumnKind::Bram(BramKind::Plain);
        }
        let col = ColId::from_idx(100);
        chips[s0].columns[col].kind = ColumnKind::Uram;
        chips[s0].columns[col].has_bli_n = true;
        chips[s0].columns[col - 1].has_bli_n = true;
        let col = ColId::from_idx(93);
        chips[s0].columns[col].kind = ColumnKind::VNoc;
        chips[s0].columns[col + 1].kind = ColumnKind::VNoc;
        let col = ColId::from_idx(121);
        chips[s0].columns[col].kind = ColumnKind::Gt;
        for i in [74, 82, 92, 96, 106, 114, 118] {
            chips[s0].cols_vbrk.insert(ColId::from_idx(i));
        }
        for i in [86, 102] {
            chips[s0].cols_cpipe.insert(ColId::from_idx(i));
        }
        for i in 72..122 {
            disabled.insert(DisabledPart::Column(s0, ColId::from_idx(i)));
        }
        let dn = &mut namings[s0];
        for (i, y) in [(0, 1), (2, 2), (4, 5), (6, 8)] {
            dn.sysmon_sat_vnoc
                .insert((ColId::from_idx(93), RegId::from_idx(i)), (5, y));
        }
    }
    if rd.part.contains("vp1002") {
        let s0 = DieId::from_idx(0);
        assert_eq!(chips[s0].regs, 11);
        disabled.insert(DisabledPart::Region(s0, RegId::from_idx(8)));
        disabled.insert(DisabledPart::Region(s0, RegId::from_idx(9)));
        disabled.insert(DisabledPart::Region(s0, RegId::from_idx(10)));
        let col_hard_l = &mut chips[s0].cols_hard[0];
        col_hard_l.regs[RegId::from_idx(8)] = HardRowKind::DcmacB;
        col_hard_l.regs[RegId::from_idx(9)] = HardRowKind::DcmacT;
        col_hard_l.regs[RegId::from_idx(10)] = HardRowKind::Mrmac;
        let col_hard_r = &mut chips[s0].cols_hard[1];
        col_hard_r.regs[RegId::from_idx(8)] = HardRowKind::IlknB;
        col_hard_r.regs[RegId::from_idx(9)] = HardRowKind::IlknT;
        col_hard_r.regs[RegId::from_idx(10)] = HardRowKind::Mrmac;
        chips[s0].regs_gt_left[RegId::from_idx(8)] = GtRowKind::Gtm;
        chips[s0].regs_gt_left[RegId::from_idx(9)] = GtRowKind::Gtm;
        chips[s0].regs_gt_left[RegId::from_idx(10)] = GtRowKind::Gtm;
        let RightKind::Gt(ref mut col_gt_r) = chips[s0].right else {
            unreachable!()
        };
        col_gt_r[RegId::from_idx(8)] = GtRowKind::Gtm;
        col_gt_r[RegId::from_idx(9)] = GtRowKind::Gtm;
        col_gt_r[RegId::from_idx(10)] = GtRowKind::Gtm;
    }
    if rd.part.contains("vp1102") {
        let s0 = DieId::from_idx(0);
        assert_eq!(chips[s0].regs, 14);
        disabled.insert(DisabledPart::Region(s0, RegId::from_idx(10)));
        disabled.insert(DisabledPart::Region(s0, RegId::from_idx(11)));
        disabled.insert(DisabledPart::Region(s0, RegId::from_idx(12)));
        disabled.insert(DisabledPart::Region(s0, RegId::from_idx(13)));
        let col_hard_l = &mut chips[s0].cols_hard[0];
        col_hard_l.regs[RegId::from_idx(10)] = HardRowKind::DcmacB;
        col_hard_l.regs[RegId::from_idx(11)] = HardRowKind::DcmacT;
        col_hard_l.regs[RegId::from_idx(12)] = HardRowKind::DcmacB;
        col_hard_l.regs[RegId::from_idx(13)] = HardRowKind::DcmacT;
        let col_hard_m = &mut chips[s0].cols_hard[1];
        col_hard_m.regs[RegId::from_idx(10)] = HardRowKind::HscB;
        col_hard_m.regs[RegId::from_idx(11)] = HardRowKind::HscT;
        col_hard_m.regs[RegId::from_idx(12)] = HardRowKind::Hdio;
        col_hard_m.regs[RegId::from_idx(13)] = HardRowKind::Hdio;
        let col_hard_r = &mut chips[s0].cols_hard[2];
        col_hard_r.regs[RegId::from_idx(10)] = HardRowKind::DcmacB;
        col_hard_r.regs[RegId::from_idx(11)] = HardRowKind::DcmacT;
        col_hard_r.regs[RegId::from_idx(12)] = HardRowKind::DcmacB;
        col_hard_r.regs[RegId::from_idx(13)] = HardRowKind::DcmacT;
        chips[s0].regs_gt_left[RegId::from_idx(10)] = GtRowKind::Gtm;
        chips[s0].regs_gt_left[RegId::from_idx(11)] = GtRowKind::Gtm;
        chips[s0].regs_gt_left[RegId::from_idx(12)] = GtRowKind::Gtm;
        chips[s0].regs_gt_left[RegId::from_idx(13)] = GtRowKind::Gtm;
        let RightKind::Gt(ref mut col_gt_r) = chips[s0].right else {
            unreachable!()
        };
        col_gt_r[RegId::from_idx(10)] = GtRowKind::Gtm;
        col_gt_r[RegId::from_idx(11)] = GtRowKind::Gtm;
        col_gt_r[RegId::from_idx(12)] = GtRowKind::Gtm;
        col_gt_r[RegId::from_idx(13)] = GtRowKind::Gtm;
    }
    let is_dsp_v2 = rd.wires.contains("DSP_DSP58_4_CLK");
    let interposer = Interposer {
        kind: ikind,
        sll_columns,
    };
    let dev_naming = DeviceNaming {
        die: namings,
        is_dsp_v2,
        is_vnoc2_scan_offset,
    };
    (chips, interposer, disabled, dev_naming)
}
