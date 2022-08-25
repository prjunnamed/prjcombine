use crate::verify::Verifier;
use enum_map::EnumMap;
use prjcombine_entity::{EntityId, EntityPartVec, EntityVec};
use prjcombine_xilinx_geom::versal::{
    self, expand_grid, BotKind, Column, ColumnKind, CpmKind, GtRowKind, HardColumn, HardRowKind,
    TopKind,
};
use prjcombine_xilinx_geom::{self as geom, int, int::Dir, Bond, ColId, DisabledPart, SlrId};
use prjcombine_xilinx_rawdump::{self as rawdump, Coord, Part, PkgPin};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::grid::{extract_int_slr, find_rows, make_device_multi, IntGrid, PreDevice};
use crate::intb::IntBuilder;

fn make_columns(int: &IntGrid) -> (EntityVec<ColId, Column>, ColId, [Option<HardColumn>; 3]) {
    let mut res = int.cols.map_values(|_| Column {
        l: ColumnKind::None,
        r: ColumnKind::None,
        has_bli_bot_l: false,
        has_bli_bot_r: false,
        has_bli_top_l: false,
        has_bli_top_r: false,
    });

    for (tkn, kind) in [
        ("CLE_W_CORE", ColumnKind::Cle),
        ("DSP_ROCF_B_TILE", ColumnKind::Dsp),
        ("DSP_ROCF_T_TILE", ColumnKind::Dsp),
        ("NOC_NSU512_TOP", ColumnKind::VNoc),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c].l = kind;
            res[c - 1].r = kind;
        }
    }
    for (tkn, kind) in [
        ("BRAM_LOCF_TR_TILE", ColumnKind::Bram),
        ("BRAM_LOCF_BR_TILE", ColumnKind::Bram),
        ("BRAM_ROCF_TR_TILE", ColumnKind::Bram),
        ("BRAM_ROCF_BR_TILE", ColumnKind::Bram),
        ("INTF_GT_TR_TILE", ColumnKind::Gt),
        ("INTF_GT_BR_TILE", ColumnKind::Gt),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c - 1].r = kind;
        }
    }
    for (tkn, kind) in [
        ("BRAM_ROCF_TL_TILE", ColumnKind::Bram),
        ("BRAM_ROCF_BL_TILE", ColumnKind::Bram),
        ("URAM_LOCF_TL_TILE", ColumnKind::Uram),
        ("URAM_LOCF_BL_TILE", ColumnKind::Uram),
        ("URAM_ROCF_TL_TILE", ColumnKind::Uram),
        ("URAM_ROCF_BL_TILE", ColumnKind::Uram),
        ("INTF_GT_TL_TILE", ColumnKind::Gt),
        ("INTF_GT_BL_TILE", ColumnKind::Gt),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column_inter(c);
            res[c].l = kind;
        }
    }
    for c in int.find_columns(&["SLL"]) {
        let c = int.lookup_column_inter(c);
        assert_eq!(res[c].l, ColumnKind::Cle);
        assert_eq!(res[c - 1].r, ColumnKind::Cle);
        res[c].l = ColumnKind::CleLaguna;
        res[c - 1].r = ColumnKind::CleLaguna;
    }
    for c in int.find_columns(&["RCLK_BRAM_CLKBUF_CORE"]) {
        let c = int.lookup_column_inter(c);
        assert_eq!(res[c - 1].r, ColumnKind::Bram);
        res[c - 1].r = ColumnKind::BramClkBuf;
    }

    for c in int.find_columns(&[
        "BLI_CLE_TOP_CORE",
        "BLI_DSP_ROCF_TR_TILE",
        "BLI_BRAM_LOCF_TR_TILE",
        "BLI_BRAM_ROCF_TR_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c - 1].has_bli_top_r = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_TOP_CORE_MY",
        "BLI_DSP_ROCF_TL_TILE",
        "BLI_BRAM_ROCF_TL_TILE",
        "BLI_URAM_LOCF_TL_TILE",
        "BLI_URAM_ROCF_TL_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c].has_bli_top_l = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_BOT_CORE",
        "BLI_DSP_ROCF_BR_TILE",
        "BLI_BRAM_ROCF_BR_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c - 1].has_bli_bot_r = true;
    }
    for c in int.find_columns(&[
        "BLI_CLE_BOT_CORE_MY",
        "BLI_DSP_ROCF_BL_TILE",
        "BLI_BRAM_ROCF_BL_TILE",
        "BLI_URAM_ROCF_BL_TILE",
    ]) {
        let c = int.lookup_column_inter(c);
        res[c].has_bli_bot_l = true;
    }

    let col_cfrm = int.lookup_column_inter(int.find_column(&["CFRM_PMC_TILE"]).unwrap());
    res[col_cfrm].l = ColumnKind::Cfrm;

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
        ("CPM_EXT_TILE", HardRowKind::CpmExt),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x);
            let row = int.lookup_row(y).to_idx() / 48;
            hard_cells.insert((col, row), kind);
        }
    }
    for (tt, kind_b, kind_t) in [
        ("ILKN_TILE", HardRowKind::IlknB, HardRowKind::IlknT),
        ("DCMAC_TILE", HardRowKind::DcmacB, HardRowKind::DcmacT),
        ("HSC_TILE", HardRowKind::HscB, HardRowKind::HscT),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x);
            let row = int.lookup_row(y).to_idx() / 48;
            hard_cells.insert((col, row), kind_b);
            hard_cells.insert((col, row + 1), kind_t);
        }
    }
    let mut cols_hard = Vec::new();
    let cols: BTreeSet<ColId> = hard_cells.keys().map(|&(c, _)| c).collect();
    for col in cols {
        res[col].l = ColumnKind::Hard;
        res[col - 1].r = ColumnKind::Hard;
        let mut regs = Vec::new();
        for _ in 0..(int.rows.len() / 48) {
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
    let cols_hard = match cols_hard.len() {
        1 => {
            let [col_l]: [_; 1] = cols_hard.try_into().unwrap();
            [Some(col_l), None, None]
        }
        2 => {
            let [col_l, col_r]: [_; 2] = cols_hard.try_into().unwrap();
            [Some(col_l), None, Some(col_r)]
        }
        3 => {
            let [col_l, col_m, col_r]: [_; 3] = cols_hard.try_into().unwrap();
            [Some(col_l), Some(col_m), Some(col_r)]
        }
        _ => unreachable!(),
    };
    (res, col_cfrm, cols_hard)
}

fn get_cols_vbrk(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["CBRK_LOCF_TOP_TILE", "CBRK_TOP_TILE"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_cpipe(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["CPIPE_TOP_TILE"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_rows_gt_left(int: &IntGrid) -> Vec<GtRowKind> {
    let mut res = vec![GtRowKind::None; int.rows.len() / 48];
    for (tkn, kind) in [
        ("GTY_QUAD_SINGLE_MY", GtRowKind::Gty),
        ("GTYP_QUAD_SINGLE_MY", GtRowKind::Gtyp),
        ("GTM_QUAD_SINGLE_MY", GtRowKind::Gtm),
        ("XRAM_CORE", GtRowKind::Xram),
    ] {
        for row in int.find_rows(&[tkn]) {
            let row = int.lookup_row(row);
            res[row.to_idx() / 48] = kind;
        }
    }
    res
}

fn get_rows_gt_right(int: &IntGrid) -> Option<Vec<GtRowKind>> {
    let mut res = vec![GtRowKind::None; int.rows.len() / 48];
    for (tkn, kind) in [
        ("GTY_QUAD_SINGLE", GtRowKind::Gty),
        ("GTYP_QUAD_SINGLE", GtRowKind::Gtyp),
        ("GTM_QUAD_SINGLE", GtRowKind::Gtm),
        ("VDU_CORE_MY", GtRowKind::Vdu),
    ] {
        for row in int.find_rows(&[tkn]) {
            let row = int.lookup_row(row);
            res[row.to_idx() / 48] = kind;
        }
    }
    if res.iter().any(|&x| x != GtRowKind::None) {
        Some(res)
    } else {
        None
    }
}

fn make_grids(
    rd: &Part,
) -> (
    EntityVec<SlrId, versal::Grid>,
    SlrId,
    BTreeSet<DisabledPart>,
) {
    let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["NOC_TNOC_BRIDGE_BOT_CORE"])
        .into_iter()
        .map(|r| r as u16)
        .collect();
    rows_slr_split.insert(0);
    rows_slr_split.insert(rd.height);
    let rows_slr_split: Vec<_> = rows_slr_split.iter().collect();
    let mut grids = EntityVec::new();
    for w in rows_slr_split.windows(2) {
        let int = extract_int_slr(rd, &["INT"], &[], *w[0], *w[1]);
        let (columns, col_cfrm, cols_hard) = make_columns(&int);
        let cpm = if !int.find_tiles(&["CPM_G5_TILE"]).is_empty() {
            CpmKind::Cpm5
        } else if !int.find_tiles(&["CPM_CORE"]).is_empty() {
            CpmKind::Cpm4
        } else {
            CpmKind::None
        };
        assert_eq!(int.rows.len() % 48, 0);
        grids.push(versal::Grid {
            columns,
            cols_vbrk: get_cols_vbrk(&int),
            cols_cpipe: get_cols_cpipe(&int),
            cols_hard,
            col_cfrm,
            regs: int.rows.len() / 48,
            regs_gt_left: get_rows_gt_left(&int),
            regs_gt_right: get_rows_gt_right(&int),
            cpm,
            top: TopKind::Me,      // XXX
            bottom: BotKind::Ssit, // XXX
        });
    }
    let mut disabled = BTreeSet::new();
    if rd.part.contains("vc1502") {
        let s0 = SlrId::from_idx(0);
        assert_eq!(grids[s0].regs, 7);
        let col_hard_r = grids[s0].cols_hard[2].as_mut().unwrap();
        for (reg, kind) in [(0, HardRowKind::Mrmac), (6, HardRowKind::Hdio)] {
            assert_eq!(col_hard_r.regs[reg], HardRowKind::None);
            col_hard_r.regs[reg] = kind;
            disabled.insert(DisabledPart::VersalHardIp(s0, col_hard_r.col, reg));
        }
        let regs_gt_r = grids[s0].regs_gt_right.as_mut().unwrap();
        for reg in [0, 1, 6] {
            assert_eq!(regs_gt_r[reg], GtRowKind::None);
            regs_gt_r[reg] = GtRowKind::Gty;
            disabled.insert(DisabledPart::VersalGtRight(s0, reg));
        }
    }
    if rd.part.contains("vm1302") {
        let s0 = SlrId::from_idx(0);
        assert_eq!(grids[s0].regs, 9);
        assert_eq!(grids[s0].columns.len(), 38);
        while grids[s0].columns.len() != 61 {
            grids[s0].columns.push(Column {
                l: ColumnKind::None,
                r: ColumnKind::None,
                has_bli_bot_l: false,
                has_bli_top_l: false,
                has_bli_bot_r: false,
                has_bli_top_r: false,
            });
        }
        for i in [
            36, 37, 38, 40, 41, 43, 44, 45, 47, 48, 49, 51, 52, 53, 55, 56, 58, 59,
        ] {
            let col = ColId::from_idx(i);
            grids[s0].columns[col].r = ColumnKind::Cle;
            grids[s0].columns[col + 1].l = ColumnKind::Cle;
            grids[s0].columns[col].has_bli_bot_r = true;
            grids[s0].columns[col].has_bli_top_r = true;
            grids[s0].columns[col + 1].has_bli_bot_l = true;
            grids[s0].columns[col + 1].has_bli_top_l = true;
        }
        for i in [39, 54] {
            let col = ColId::from_idx(i);
            grids[s0].columns[col].r = ColumnKind::Dsp;
            grids[s0].columns[col + 1].l = ColumnKind::Dsp;
            grids[s0].columns[col].has_bli_bot_r = true;
            grids[s0].columns[col].has_bli_top_r = true;
            grids[s0].columns[col + 1].has_bli_bot_l = true;
            grids[s0].columns[col + 1].has_bli_top_l = true;
        }
        for i in [36, 43, 58] {
            let col = ColId::from_idx(i);
            grids[s0].columns[col].l = ColumnKind::Bram;
        }
        for i in [42, 50, 57] {
            let col = ColId::from_idx(i);
            grids[s0].columns[col].r = ColumnKind::Bram;
        }
        let col = ColId::from_idx(51);
        grids[s0].columns[col].l = ColumnKind::Uram;
        grids[s0].columns[col].has_bli_top_l = true;
        grids[s0].columns[col - 1].has_bli_top_r = true;
        let col = ColId::from_idx(46);
        grids[s0].columns[col].r = ColumnKind::VNoc;
        grids[s0].columns[col + 1].l = ColumnKind::VNoc;
        let col = ColId::from_idx(60);
        grids[s0].columns[col].r = ColumnKind::Gt;
        for i in [37, 41, 46, 48, 53, 57, 59] {
            grids[s0].cols_vbrk.insert(ColId::from_idx(i));
        }
        for i in [43, 51] {
            grids[s0].cols_cpipe.insert(ColId::from_idx(i));
        }
        for i in 36..61 {
            disabled.insert(DisabledPart::VersalColumn(s0, ColId::from_idx(i)));
        }
    }
    (grids, SlrId::from_idx(0), disabled)
}

fn make_bond(
    _rd: &Part,
    _pkg: &str,
    _grids: &EntityVec<SlrId, versal::Grid>,
    _grid_master: SlrId,
    _disabled: &BTreeSet<DisabledPart>,
    _pins: &[PkgPin],
) -> Bond {
    let bond_pins = BTreeMap::new();
    Bond {
        pins: bond_pins,
        io_banks: Default::default(),
    }
}

fn make_int_db(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("versal", rd);
    let mut term_wires: EnumMap<Dir, EntityPartVec<_, _>> = Default::default();
    let intf_kinds = [
        (Dir::W, "INTF_LOCF_BL_TILE", "INTF.W", false),
        (Dir::W, "INTF_LOCF_TL_TILE", "INTF.W", false),
        (Dir::E, "INTF_LOCF_BR_TILE", "INTF.E", false),
        (Dir::E, "INTF_LOCF_TR_TILE", "INTF.E", false),
        (Dir::W, "INTF_ROCF_BL_TILE", "INTF.W", false),
        (Dir::W, "INTF_ROCF_TL_TILE", "INTF.W", false),
        (Dir::E, "INTF_ROCF_BR_TILE", "INTF.E", false),
        (Dir::E, "INTF_ROCF_TR_TILE", "INTF.E", false),
        (Dir::W, "INTF_HB_LOCF_BL_TILE", "INTF.W.HB", false),
        (Dir::W, "INTF_HB_LOCF_TL_TILE", "INTF.W.HB", false),
        (Dir::E, "INTF_HB_LOCF_BR_TILE", "INTF.E.HB", false),
        (Dir::E, "INTF_HB_LOCF_TR_TILE", "INTF.E.HB", false),
        (Dir::W, "INTF_HB_ROCF_BL_TILE", "INTF.W.HB", false),
        (Dir::W, "INTF_HB_ROCF_TL_TILE", "INTF.W.HB", false),
        (Dir::E, "INTF_HB_ROCF_BR_TILE", "INTF.E.HB", false),
        (Dir::E, "INTF_HB_ROCF_TR_TILE", "INTF.E.HB", false),
        (Dir::W, "INTF_HDIO_LOCF_BL_TILE", "INTF.W.HB", false),
        (Dir::W, "INTF_HDIO_LOCF_TL_TILE", "INTF.W.HB", false),
        (Dir::E, "INTF_HDIO_LOCF_BR_TILE", "INTF.E.HB", false),
        (Dir::E, "INTF_HDIO_LOCF_TR_TILE", "INTF.E.HB", false),
        (Dir::W, "INTF_HDIO_ROCF_BL_TILE", "INTF.W.HB", false),
        (Dir::W, "INTF_HDIO_ROCF_TL_TILE", "INTF.W.HB", false),
        (Dir::E, "INTF_HDIO_ROCF_BR_TILE", "INTF.E.HB", false),
        (Dir::E, "INTF_HDIO_ROCF_TR_TILE", "INTF.E.HB", false),
        (Dir::W, "INTF_CFRM_BL_TILE", "INTF.W", false),
        (Dir::W, "INTF_CFRM_TL_TILE", "INTF.W", false),
        (Dir::W, "INTF_PSS_BL_TILE", "INTF.W.TERM", true),
        (Dir::W, "INTF_PSS_TL_TILE", "INTF.W.TERM", true),
        (Dir::W, "INTF_GT_BL_TILE", "INTF.W.TERM", true),
        (Dir::W, "INTF_GT_TL_TILE", "INTF.W.TERM", true),
        (Dir::E, "INTF_GT_BR_TILE", "INTF.E.TERM", true),
        (Dir::E, "INTF_GT_TR_TILE", "INTF.E.TERM", true),
    ];

    builder.wire("VCC", int::WireKind::Tie1, &["VCC_WIRE"]);

    for (iq, q) in ['E', 'N', 'S', 'W'].into_iter().enumerate() {
        for (ih, h) in ['E', 'W'].into_iter().enumerate() {
            for i in 0..32 {
                match (q, i) {
                    ('E', 0 | 2) | ('W', 0 | 2) | ('N', 0) => {
                        let w = builder.mux_out(
                            format!("SDQNODE.{q}.{h}.{i}"),
                            &[format!("OUT_{q}NODE_{h}_{i}")],
                        );
                        builder.branch(
                            w,
                            Dir::S,
                            format!("SDQNODE.{q}.{h}.{i}.S"),
                            &[format!("IN_{q}NODE_{h}_BLS_{i}")],
                        );
                    }
                    ('E', 29 | 31) | ('W', 31) | ('S', 31) => {
                        let w = builder.mux_out(
                            format!("SDQNODE.{q}.{h}.{i}"),
                            &[format!("OUT_{q}NODE_{h}_{i}")],
                        );
                        builder.branch(
                            w,
                            Dir::N,
                            format!("SDQNODE.{q}.{h}.{i}.N"),
                            &[format!("IN_{q}NODE_{h}_BLN_{i}")],
                        );
                    }
                    _ => {
                        // TODO not the true permutation
                        let a = [0, 11, 1, 2, 3, 4, 5, 6, 7, 8, 9, 13, 14, 15, 10, 12][i >> 1];
                        let aa = a + ih * 16 + iq * 32;
                        let b = i & 1;
                        builder.mux_out(
                            format!("SDQNODE.{q}.{h}.{i}"),
                            &[format!("INT_NODE_SDQ_ATOM_{aa}_INT_OUT{b}")],
                        );
                    }
                }
            }
        }
    }

    for i in 0..48 {
        builder.mux_out(
            format!("SDQ_RED.{i}"),
            &[format!("INT_SDQ_RED_ATOM_{i}_INT_OUT0")],
        );
    }
    for (fwd, name, l, ll, num) in [
        (Dir::E, "SNG", 1, 1, 16),
        (Dir::N, "SNG", 1, 1, 16),
        (Dir::E, "DBL", 1, 2, 8),
        (Dir::N, "DBL", 2, 2, 8),
        (Dir::E, "QUAD", 2, 4, 8),
        (Dir::N, "QUAD", 4, 4, 8),
    ] {
        let bwd = !fwd;
        for ew_f in [Dir::E, Dir::W] {
            let ew_b = if fwd == Dir::E { !ew_f } else { ew_f };
            for i in 0..num {
                if ll == 1 && fwd == Dir::E && ew_f == Dir::W {
                    continue;
                }
                let mut w_f = builder.mux_out(
                    format!("{name}.{fwd}.{ew_f}.{i}.0"),
                    &[format!("OUT_{fwd}{fwd}{ll}_{ew_f}_BEG{i}")],
                );
                let mut w_b = builder.mux_out(
                    format!("{name}.{bwd}.{ew_b}.{i}.0"),
                    &[format!("OUT_{bwd}{bwd}{ll}_{ew_b}_BEG{i}")],
                );
                match (fwd, ew_f, ll) {
                    (Dir::E, Dir::E, 1) => {
                        let ii = i;
                        builder.extra_name(format!("IF_HBUS_EBUS{ii}"), w_f);
                        builder.extra_name(format!("IF_HBUS_W_EBUS{ii}"), w_f);
                        builder.extra_name(format!("IF_HBUS_WBUS{ii}"), w_b);
                        builder.extra_name(format!("IF_HBUS_E_WBUS{ii}"), w_b);
                    }
                    (Dir::E, Dir::W, 2) => {
                        let ii = i + 24;
                        builder.extra_name(format!("IF_HBUS_EBUS{ii}"), w_f);
                        builder.extra_name(format!("IF_HBUS_W_EBUS{ii}"), w_f);
                        let ii = i + 16;
                        builder.extra_name(format!("IF_HBUS_WBUS{ii}"), w_b);
                        builder.extra_name(format!("IF_HBUS_E_WBUS{ii}"), w_b);
                    }
                    _ => (),
                }
                if bwd == Dir::W && i == 0 && ll == 1 {
                    let w =
                        builder.branch(w_b, Dir::S, format!("{name}.{bwd}.{ew_b}.{i}.0.S"), &[""]);
                    builder.extra_name_tile_sub("CLE_BC_CORE", "BNODE_TAP0", 1, w);
                    builder.extra_name_tile_sub("SLL", "BNODE_TAP0", 1, w);
                }
                for j in 1..l {
                    let n_f =
                        builder.branch(w_f, fwd, format!("{name}.{fwd}.{ew_f}.{i}.{j}"), &[""]);
                    let n_b =
                        builder.branch(w_b, bwd, format!("{name}.{bwd}.{ew_b}.{i}.{j}"), &[""]);
                    match (fwd, ew_f, ll, j) {
                        (Dir::E, Dir::W, 4, 1) => {
                            let ii = i + 40;
                            builder.extra_name(format!("IF_HBUS_WBUS{ii}"), n_b);
                            builder.extra_name(format!("IF_HBUS_E_WBUS{ii}"), n_b);
                            let ii = i + 56;
                            builder.extra_name(format!("IF_HBUS_EBUS{ii}"), n_f);
                            builder.extra_name(format!("IF_HBUS_W_EBUS{ii}"), n_f);
                            if i == 0 {
                                let w = builder.branch(
                                    n_f,
                                    Dir::S,
                                    format!("{name}.{fwd}.{ew_f}.{i}.{j}.S"),
                                    &[""],
                                );
                                for (dir, tkn, _, _) in intf_kinds {
                                    if dir == Dir::E {
                                        builder.extra_name_tile(tkn, "IF_LBC_N_BNODE_SOUTHBUS", w);
                                    }
                                }
                            }
                        }
                        (Dir::N, Dir::E, 2, 1) => {
                            let ii = i + 32;
                            builder.extra_name(format!("IF_VBUS_S_NBUS{ii}"), n_f);
                        }
                        (Dir::N, Dir::W, 2, 1) => {
                            let ii = i + 48;
                            builder.extra_name(format!("IF_VBUS_S_NBUS{ii}"), n_f);
                        }
                        (Dir::N, Dir::E, 4, 1) => {
                            let ii = i + 64;
                            builder.extra_name(format!("IF_VBUS_S_NBUS{ii}"), n_f);
                        }
                        (Dir::N, Dir::W, 4, 1) => {
                            let ii = i + 96;
                            builder.extra_name(format!("IF_VBUS_S_NBUS{ii}"), n_f);
                        }
                        _ => (),
                    }
                    term_wires[fwd].insert(n_b, int::TermInfo::PassNear(w_f));
                    term_wires[bwd].insert(n_f, int::TermInfo::PassNear(w_b));
                    w_f = n_f;
                    w_b = n_b;
                }
                let e_f = builder.branch(
                    w_f,
                    fwd,
                    format!("{name}.{fwd}.{ew_f}.{i}.{l}"),
                    &[format!("IN_{fwd}{fwd}{ll}_{ew_f}_END{i}")],
                );
                let e_b = builder.branch(
                    w_b,
                    bwd,
                    format!("{name}.{bwd}.{ew_b}.{i}.{l}"),
                    &[format!("IN_{bwd}{bwd}{ll}_{ew_b}_END{i}")],
                );
                match (fwd, ew_f, ll) {
                    (Dir::N, _, 1) => {
                        for (dir, tkn, _, _) in intf_kinds {
                            if dir == ew_f {
                                let ii = i;
                                builder.extra_name_tile(tkn, format!("IF_INT_VSINGLE{ii}"), e_b);
                                let ii = i + 16;
                                builder.extra_name_tile(tkn, format!("IF_INT_VSINGLE{ii}"), e_f);
                            }
                        }
                    }
                    (Dir::E, Dir::E, 2) => {
                        let ii = i + 16;
                        for (_, tkn, _, term) in intf_kinds {
                            if !term {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_EBUS{ii}"), e_f);
                                builder.extra_name_tile(tkn, format!("IF_HBUS_W_EBUS{ii}"), e_f);
                            } else {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_EBUS{ii}"), e_b);
                            }
                        }
                        let ii = i + 24;
                        for (_, tkn, _, term) in intf_kinds {
                            if !term {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_WBUS{ii}"), e_b);
                                builder.extra_name_tile(tkn, format!("IF_HBUS_E_WBUS{ii}"), e_b);
                            } else {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_WBUS{ii}"), e_f);
                            }
                        }
                    }
                    (Dir::E, Dir::E, 4) => {
                        let ii = i + 40;
                        for (_, tkn, _, term) in intf_kinds {
                            if !term {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_EBUS{ii}"), e_f);
                                builder.extra_name_tile(tkn, format!("IF_HBUS_W_EBUS{ii}"), e_f);
                            } else {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_EBUS{ii}"), e_b);
                            }
                        }
                        if i == 0 {
                            let w = builder.branch(
                                e_f,
                                Dir::S,
                                format!("{name}.{fwd}.{ew_f}.{i}.{l}.S"),
                                &[""],
                            );
                            for (dir, tkn, _, _) in intf_kinds {
                                if dir == Dir::W {
                                    builder.extra_name_tile(tkn, "IF_LBC_N_BNODE_SOUTHBUS", w);
                                }
                            }
                        }
                        let ii = i + 56;
                        for (_, tkn, _, term) in intf_kinds {
                            if !term {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_WBUS{ii}"), e_b);
                                builder.extra_name_tile(tkn, format!("IF_HBUS_E_WBUS{ii}"), e_b);
                            } else {
                                builder.extra_name_tile(tkn, format!("IF_HBUS_WBUS{ii}"), e_f);
                            }
                        }
                    }
                    _ => (),
                }
                term_wires[fwd].insert(e_b, int::TermInfo::PassNear(w_f));
                term_wires[bwd].insert(e_f, int::TermInfo::PassNear(w_b));
            }
        }
    }

    for (fwd, name, l, ll) in [
        (Dir::E, "LONG", 3, 6),
        (Dir::N, "LONG", 7, 7),
        (Dir::E, "LONG", 5, 10),
        (Dir::N, "LONG", 12, 12),
    ] {
        let bwd = !fwd;
        for i in 0..8 {
            let mut w_f = builder.mux_out(
                format!("{name}.{fwd}.{i}.0"),
                &[format!("OUT_{fwd}{fwd}{ll}_BEG{i}")],
            );
            let mut w_b = builder.mux_out(
                format!("{name}.{bwd}.{i}.0"),
                &[format!("OUT_{bwd}{bwd}{ll}_BEG{i}")],
            );
            for j in 1..l {
                let n_f = builder.branch(w_f, fwd, format!("{name}.{fwd}.{i}.{j}"), &[""]);
                let n_b = builder.branch(w_b, bwd, format!("{name}.{bwd}.{i}.{j}"), &[""]);
                term_wires[fwd].insert(n_b, int::TermInfo::PassNear(w_f));
                term_wires[bwd].insert(n_f, int::TermInfo::PassNear(w_b));
                w_f = n_f;
                w_b = n_b;
            }
            let e_f = builder.branch(
                w_f,
                fwd,
                format!("{name}.{fwd}.{i}.{l}"),
                &[format!("IN_{fwd}{fwd}{ll}_END{i}")],
            );
            let e_b = builder.branch(
                w_b,
                bwd,
                format!("{name}.{bwd}.{i}.{l}"),
                &[format!("IN_{bwd}{bwd}{ll}_END{i}")],
            );
            term_wires[fwd].insert(e_b, int::TermInfo::PassNear(w_f));
            term_wires[bwd].insert(e_f, int::TermInfo::PassNear(w_b));
            if i == 0 && fwd == Dir::E && ll == 6 {
                builder.branch(
                    e_f,
                    Dir::S,
                    format!("{name}.{fwd}.{i}.{l}.S"),
                    &[format!("IN_{fwd}{fwd}{ll}_BLS_{i}")],
                );
            }
            if i == 7 && fwd == Dir::E && ll == 10 {
                builder.branch(
                    e_f,
                    Dir::N,
                    format!("{name}.{fwd}.{i}.{l}.N"),
                    &[format!("IN_{fwd}{fwd}{ll}_BLN_{i}")],
                );
            }
        }
    }

    for i in 0..128 {
        for j in 0..2 {
            builder.mux_out(
                format!("INODE.{i}.{j}"),
                &[format!("INT_NODE_IMUX_ATOM_{i}_INT_OUT{j}")],
            );
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..96 {
            builder.mux_out(format!("IMUX.{ew}.IMUX.{i}"), &[format!("IMUX_B_{ew}{i}")]);
        }
    }

    let mut bounces = Vec::new();
    for ew in ['E', 'W'] {
        for i in 0..32 {
            let w = builder.mux_out(format!("IMUX.{ew}.BOUNCE.{i}"), &[""]);
            builder.extra_name_tile("INT", format!("BOUNCE_{ew}{i}"), w);
            bounces.push(w);
        }
    }

    let mut bnodes = Vec::new();
    for dir in [Dir::E, Dir::W] {
        for i in 0..64 {
            let w = builder.wire(
                format!("BNODE.{dir}.{i}"),
                int::WireKind::Branch(dir),
                &[format!("BNODE_{dir}{i}")],
            );
            bnodes.push(w);
        }
    }

    let mut logic_outs_w = EntityPartVec::new();
    let mut logic_outs_e = EntityPartVec::new();
    for (sub, ew) in [Dir::E, Dir::W].into_iter().enumerate() {
        let we = !ew;
        for i in 0..48 {
            let w = builder.logic_out(format!("OUT.{ew}.{i}"), &[""]);
            builder.extra_name_tile("INT", format!("LOGIC_OUTS_{ew}{i}"), w);
            match (ew, i) {
                (Dir::E, 1 | 4 | 5) | (Dir::W, 4 | 5) => {
                    builder.branch(
                        w,
                        Dir::S,
                        format!("OUT.{ew}.{i}.S"),
                        &[format!("IN_LOGIC_OUTS_{ew}_BLS_{i}")],
                    );
                }
                _ => (),
            }
            let cw = builder.wire(
                format!("CLE.OUT.{ew}.{i}"),
                int::WireKind::Branch(ew),
                &[""],
            );
            builder.extra_name_tile_sub("CLE_BC_CORE", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            builder.extra_name_tile_sub("SLL", format!("LOGIC_OUTS_{we}{i}"), sub, cw);
            if ew == Dir::E {
                logic_outs_e.insert(cw, int::TermInfo::PassNear(w));
            } else {
                logic_outs_w.insert(cw, int::TermInfo::PassNear(w));
            }
        }
    }

    let mut bnode_outs = Vec::new();
    for (sub, ew) in [Dir::E, Dir::W].into_iter().enumerate() {
        let we = !ew;
        for i in 0..32 {
            let w = builder.mux_out(format!("CLE.BNODE.{ew}.{i}"), &[""]);
            builder.extra_name_sub(format!("BNODE_OUTS_{we}{i}"), sub, w);
            bnode_outs.push(w);
        }
    }

    for (sub, ew) in [Dir::E, Dir::W].into_iter().enumerate() {
        let we = !ew;
        for i in 0..12 {
            let w = builder.mux_out(format!("CLE.CNODE.{ew}.{i}"), &[""]);
            builder.extra_name_sub(format!("CNODE_OUTS_{we}{i}"), sub, w);
            bnode_outs.push(w);
        }
    }

    for (sub, ew) in [Dir::W, Dir::E].into_iter().enumerate() {
        let lr = match ew {
            Dir::E => 'L',
            Dir::W => 'R',
            _ => unreachable!(),
        };
        for i in 0..13 {
            let w = builder.mux_out(format!("CLE.IMUX.{ew}.CTRL.{i}"), &[""]);
            builder.extra_name_sub(format!("CTRL_{lr}{i}"), sub, w);
        }
    }

    for i in 0..16 {
        let w = builder.wire(
            format!("CLE.GCLK.{i}"),
            int::WireKind::ClkOut(32 + i),
            &[""],
        );
        builder.extra_name_sub(format!("GCLK_B{i}"), 1, w);
    }

    for ew in [Dir::W, Dir::E] {
        for i in 0..4 {
            let rg = match i % 2 {
                0 => "RED",
                1 => "GREEN",
                _ => unreachable!(),
            };
            let w = builder.mux_out(format!("INTF.{ew}.IMUX.IRI{i}.CLK"), &[""]);
            for (dir, tkn, _, _) in intf_kinds {
                if dir == ew {
                    builder.extra_name_tile(tkn, format!("INTF_IRI_QUADRANT_{rg}_{i}_CLK"), w);
                }
            }
            let w = builder.mux_out(format!("INTF.{ew}.IMUX.IRI{i}.RST"), &[""]);
            for (dir, tkn, _, _) in intf_kinds {
                if dir == ew {
                    builder.extra_name_tile(tkn, format!("INTF_IRI_QUADRANT_{rg}_{i}_RST"), w);
                }
            }
            for j in 0..4 {
                let w = builder.mux_out(format!("INTF.{ew}.IMUX.IRI{i}.CE{j}"), &[""]);
                for (dir, tkn, _, _) in intf_kinds {
                    if dir == ew {
                        builder.extra_name_tile(
                            tkn,
                            format!("INTF_IRI_QUADRANT_{rg}_{i}_CE{j}"),
                            w,
                        );
                    }
                }
            }
        }
    }

    for ew in [Dir::W, Dir::E] {
        for i in 0..12 {
            for j in 0..2 {
                let w = builder.mux_out(format!("INTF.{ew}.CNODE.{i}.{j}"), &[""]);
                for (dir, tkn, _, _) in intf_kinds {
                    if dir == ew {
                        builder.extra_name_tile(tkn, format!("INTF_CNODE_ATOM_{i}_INT_OUT{j}"), w);
                    }
                }
            }
        }
    }

    for (b, ew) in [Dir::W, Dir::E].into_iter().enumerate() {
        for i in 0..16 {
            let w = builder.wire(
                format!("INTF.{ew}.GCLK.{i}"),
                int::WireKind::ClkOut(b * 16 + i),
                &[""],
            );
            for (dir, tkn, _, _) in intf_kinds {
                if dir == ew {
                    builder.extra_name_tile(tkn, format!("IF_GCLK_GCLK_B{i}"), w);
                }
            }
        }
    }

    for i in 0..40 {
        for j in 0..2 {
            builder.mux_out(
                format!("RCLK.INODE.{i}.{j}"),
                &[format!("INT_NODE_IMUX_ATOM_RCLK_{i}_INT_OUT{j}")],
            );
        }
    }

    for ew in [Dir::W, Dir::E] {
        for i in 0..20 {
            for j in 0..2 {
                builder.mux_out(
                    format!("RCLK.IMUX.{ew}.{i}.{j}"),
                    &[format!("IF_INT2COE_{ew}_INT_RCLK_TO_CLK_B_{i}_{j}")],
                );
            }
        }
    }

    builder.extract_main_passes();

    let t = builder.db.terms.get("MAIN.W").unwrap().1.clone();
    builder.db.terms.insert("CLE.W".to_string(), t);
    let t = builder.db.terms.get("MAIN.E").unwrap().1.clone();
    builder.db.terms.insert("CLE.E".to_string(), t);

    builder.node_type("INT", "INT", "INT");

    for tkn in ["CLE_BC_CORE", "SLL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = builder.walk_to_int(xy, Dir::W).unwrap();
            let xy_r = builder.walk_to_int(xy, Dir::E).unwrap();
            builder.extract_xnode("CLE_BC", xy, &[], &[xy_l, xy_r], "CLE_BC", &[], &[]);
            let tile = &rd.tiles[&xy];
            let tk = &rd.tile_kinds[tile.kind];
            let naming = builder.db.get_node_naming("CLE_BC");
            let int_naming = builder.db.get_node_naming("INT");
            for (int_xy, t, dir, tname) in [
                (xy_l, int::NodeTileId::from_idx(0), Dir::E, "CLE.E"),
                (xy_r, int::NodeTileId::from_idx(1), Dir::W, "CLE.W"),
            ] {
                let naming = &builder.db.node_namings[naming];
                let mut nodes = HashMap::new();
                for &w in &bnode_outs {
                    if let Some(n) = naming.wires.get(&(t, w)) {
                        let n = rd.wires.get(n).unwrap();
                        if let &rawdump::TkWire::Connected(idx) = tk.wires.get(&n).unwrap().1 {
                            nodes.insert(tile.conn_wires[idx], w);
                        }
                    }
                }
                let mut wires = EntityPartVec::new();
                let int_tile = &rd.tiles[&int_xy];
                let int_tk = &rd.tile_kinds[int_tile.kind];
                let int_naming = &builder.db.node_namings[int_naming];
                for &w in &bounces {
                    if let Some(n) = int_naming.wires.get(&(int::NodeTileId::from_idx(0), w)) {
                        let n = rd.wires.get(n).unwrap();
                        if let &rawdump::TkWire::Connected(idx) = int_tk.wires.get(&n).unwrap().1 {
                            nodes.insert(int_tile.conn_wires[idx], w);
                        }
                    }
                }
                for &w in &bnodes {
                    if let Some(n) = int_naming.wires.get(&(int::NodeTileId::from_idx(0), w)) {
                        let n = rd.wires.get(n).unwrap();
                        if let &rawdump::TkWire::Connected(idx) = int_tk.wires.get(&n).unwrap().1 {
                            if let Some(&cw) = nodes.get(&int_tile.conn_wires[idx]) {
                                wires.insert(w, int::TermInfo::PassNear(cw));
                            }
                        }
                    }
                }
                builder.insert_term_merge(tname, int::TermKind { dir, wires });
            }
        }
    }

    let t = builder.db.terms.get("CLE.W").unwrap().1.clone();
    builder.db.terms.insert("CLE.BLI.W".to_string(), t);
    let t = builder.db.terms.get("CLE.E").unwrap().1.clone();
    builder.db.terms.insert("CLE.BLI.E".to_string(), t);
    builder.insert_term_merge(
        "CLE.W",
        int::TermKind {
            dir: Dir::W,
            wires: logic_outs_w,
        },
    );
    builder.insert_term_merge(
        "CLE.E",
        int::TermKind {
            dir: Dir::E,
            wires: logic_outs_e,
        },
    );

    for (dir, tkn, name, _) in intf_kinds {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = builder.walk_to_int(xy, !dir).unwrap();
            builder.extract_xnode(name, xy, &[], &[int_xy], name, &[], &[]);
        }
    }

    for (dir, wires) in term_wires {
        builder.insert_term_merge(&format!("TERM.{dir}"), int::TermKind { dir, wires });
    }
    builder.extract_term_conn("TERM.W", Dir::W, "INTF_GT_BL_TILE", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INTF_GT_TL_TILE", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INTF_PSS_BL_TILE", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INTF_PSS_TL_TILE", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INTF_GT_BR_TILE", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INTF_GT_TR_TILE", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "TERM_B_INT_TILE", &[]);
    builder.extract_term_conn("TERM.N", Dir::N, "TERM_T_INT_TILE", &[]);

    for tkn in ["RCLK_INT_L_FT", "RCLK_INT_R_FT"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = Coord {
                x: xy.x,
                y: xy.y + 1,
            };
            let mut int_xy_b = Coord {
                x: xy.x,
                y: xy.y - 1,
            };
            if rd.tile_kinds.key(rd.tiles[&int_xy_b].kind) != "INT" {
                int_xy_b.y -= 1;
                if rd.tile_kinds.key(rd.tiles[&int_xy_b].kind) != "INT" {
                    continue;
                }
            }
            builder.extract_xnode("RCLK", xy, &[], &[int_xy], "RCLK", &[], &[]);
        }
    }

    builder.build()
}

pub fn ingest(rd: &Part) -> (PreDevice, Option<int::IntDb>) {
    let (grids, grid_master, disabled) = make_grids(rd);
    let int_db = make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(rd, pkg, &grids, grid_master, &disabled, pins),
        ));
    }
    let grid_refs = grids.map_values(|x| x);
    let eint = expand_grid(&grid_refs, grid_master, &disabled, &int_db);
    let vrf = Verifier::new(rd, &eint);
    vrf.finish();
    let grids = grids.into_map_values(geom::Grid::Versal);
    (
        make_device_multi(rd, grids, grid_master, Vec::new(), bonds, disabled),
        Some(int_db),
    )
}
