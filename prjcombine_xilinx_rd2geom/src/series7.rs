use crate::verify::Verifier;
use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_xilinx_geom::series7::{
    self, expand_grid, ColumnKind, GridKind, GtColumn, GtKind, Hole, HoleKind, IoColumn, IoKind,
};
use prjcombine_xilinx_geom::{
    self as geom, int, int::Dir, Bond, BondPin, CfgPin, ColId, ExtraDie, GtPin, GtRegionPin, PsPin,
    RowId, SlrId, SysMonPin,
};
use prjcombine_xilinx_rawdump::{Coord, Part, PkgPin};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;

use crate::intb::IntBuilder;

use crate::grid::{
    extract_int_slr, find_columns, find_row, find_rows, make_device_multi, ExtraCol, IntGrid,
    PreDevice,
};

fn get_kind(rd: &Part) -> GridKind {
    if find_columns(rd, &["GTX_COMMON", "GTH_COMMON"]).is_empty() {
        GridKind::Artix
    } else if !find_columns(rd, &["MONITOR_BOT_FUJI2", "MONITOR_BOT_PELE1"]).is_empty() {
        GridKind::Kintex
    } else {
        GridKind::Virtex
    }
}

fn make_columns(int: &IntGrid) -> EntityVec<ColId, ColumnKind> {
    let mut res: EntityVec<ColId, Option<ColumnKind>> = int.cols.map_values(|_| None);
    *res.first_mut().unwrap() = Some(ColumnKind::Gt);
    *res.last_mut().unwrap() = Some(ColumnKind::Gt);
    for c in int.find_columns(&["CLBLL_L"]) {
        res[int.lookup_column(c + 1)] = Some(ColumnKind::ClbLL);
    }
    for c in int.find_columns(&["CLBLM_L"]) {
        res[int.lookup_column(c + 1)] = Some(ColumnKind::ClbLM);
    }
    for c in int.find_columns(&["CLBLL_R"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLL);
    }
    for c in int.find_columns(&["CLBLM_R"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLM);
    }
    for c in int.find_columns(&["BRAM_L"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Bram);
    }
    for c in int.find_columns(&["BRAM_R"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Bram);
    }
    for c in int.find_columns(&["DSP_L"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Dsp);
    }
    for c in int.find_columns(&["DSP_R"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Dsp);
    }
    for c in int.find_columns(&["RIOI", "RIOI3"]) {
        res[int.lookup_column_inter(c) - 1] = Some(ColumnKind::Io);
    }
    for c in int.find_columns(&["LIOI", "LIOI3"]) {
        res[int.lookup_column_inter(c)] = Some(ColumnKind::Io);
    }
    for c in int.find_columns(&["CMT_FIFO_R"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Cmt);
    }
    for c in int.find_columns(&["CMT_FIFO_L"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Cmt);
    }
    for c in int.find_columns(&["VFRAME"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Cfg);
    }
    for c in int.find_columns(&["CLK_HROW_BOT_R"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Clk);
    }
    for c in int.find_columns(&["CFG_CENTER_BOT"]) {
        for d in [-10, -9, -6, -5, -2, -1] {
            res[int.lookup_column(c + d)] = Some(ColumnKind::ClbLL);
        }
    }
    for c in int.find_columns(&["INT_INTERFACE_PSS_L"]) {
        for (d, kind) in [
            (-46, ColumnKind::Io),
            (-45, ColumnKind::Cmt),
            (-39, ColumnKind::ClbLM),
            (-38, ColumnKind::ClbLM),
            (-35, ColumnKind::ClbLM),
            (-34, ColumnKind::ClbLM),
            (-29, ColumnKind::Bram),
            (-28, ColumnKind::ClbLM),
            (-25, ColumnKind::ClbLM),
            (-24, ColumnKind::Dsp),
            (-19, ColumnKind::ClbLM),
            (-18, ColumnKind::ClbLM),
            (-15, ColumnKind::ClbLM),
            (-14, ColumnKind::ClbLM),
            (-9, ColumnKind::Dsp),
            (-8, ColumnKind::ClbLM),
            (-5, ColumnKind::ClbLM),
            (-4, ColumnKind::Bram),
            (1, ColumnKind::ClbLL),
        ] {
            res[int.lookup_column(c + d)] = Some(kind);
        }
    }
    res.map_values(|x| x.unwrap())
}

fn get_cols_vbrk(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["VBRK"]) {
        res.insert(int.lookup_column_inter(c));
    }
    for c in int.find_columns(&["INT_INTERFACE_PSS_L"]) {
        res.insert(int.lookup_column_inter(c - 41));
        res.insert(int.lookup_column_inter(c - 32));
        res.insert(int.lookup_column_inter(c - 21));
        res.insert(int.lookup_column_inter(c - 12));
        res.insert(int.lookup_column_inter(c - 1));
    }
    res
}

fn get_holes(int: &IntGrid) -> Vec<Hole> {
    let mut res = Vec::new();
    for (x, y) in int.find_tiles(&["PCIE_BOT"]) {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 10);
        assert_eq!(row.to_idx() % 50, 0);
        res.push(Hole {
            kind: HoleKind::Pcie2Right,
            col,
            row,
        });
    }
    for (x, y) in int.find_tiles(&["PCIE_BOT_LEFT"]) {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 10);
        assert_eq!(row.to_idx() % 50, 0);
        res.push(Hole {
            kind: HoleKind::Pcie2Left,
            col,
            row,
        });
    }
    for (x, y) in int.find_tiles(&["PCIE3_BOT_RIGHT"]) {
        let col = int.lookup_column(x - 2);
        let row = int.lookup_row(y - 7);
        assert_eq!(row.to_idx() % 50, 25);
        res.push(Hole {
            kind: HoleKind::Pcie3,
            col,
            row,
        });
    }
    for (x, y) in int.find_tiles(&["GTP_CHANNEL_0_MID_LEFT"]) {
        let col = int.lookup_column(x - 14);
        let row = int.lookup_row(y - 5);
        assert_eq!(row.to_idx() % 50, 0);
        res.push(Hole {
            kind: HoleKind::GtpLeft,
            col,
            row,
        });
    }
    for (x, y) in int.find_tiles(&["GTP_CHANNEL_0_MID_RIGHT"]) {
        let col = int.lookup_column(x + 19);
        let row = int.lookup_row(y - 5);
        assert_eq!(row.to_idx() % 50, 0);
        res.push(Hole {
            kind: HoleKind::GtpRight,
            col,
            row,
        });
    }
    res
}

fn get_cols_io(int: &IntGrid) -> [Option<IoColumn>; 2] {
    let mut res = [None, None];
    if let Some(x) = int.find_column(&["LIOI", "LIOI3"]) {
        let col = int.lookup_column_inter(x);
        let mut regs = Vec::new();
        for i in 0..(int.rows.len() / 50) {
            let c = Coord {
                x: x as u16,
                y: int.rows[RowId::from_idx(i * 50 + 1)] as u16,
            };
            let kind = match &int.rd.tile_kinds.key(int.rd.tiles[&c].kind)[..] {
                "LIOI" => Some(IoKind::Hpio),
                "LIOI3" => Some(IoKind::Hrio),
                "PCIE_NULL" | "NULL" => None,
                _ => unreachable!(),
            };
            regs.push(kind);
        }
        res[0] = Some(IoColumn { col, regs });
    }
    if let Some(x) = int.find_column(&["RIOI", "RIOI3"]) {
        let col = int.lookup_column_inter(x) - 1;
        let mut regs = Vec::new();
        for i in 0..(int.rows.len() / 50) {
            let c = Coord {
                x: x as u16,
                y: int.rows[RowId::from_idx(i * 50 + 1)] as u16,
            };
            let kind = match &int.rd.tile_kinds.key(int.rd.tiles[&c].kind)[..] {
                "RIOI" => Some(IoKind::Hpio),
                "RIOI3" => Some(IoKind::Hrio),
                "NULL" => None,
                _ => unreachable!(),
            };
            regs.push(kind);
        }
        res[1] = Some(IoColumn { col, regs });
    }
    res
}

fn get_cols_gt(int: &IntGrid, columns: &EntityVec<ColId, ColumnKind>) -> [Option<GtColumn>; 2] {
    let mut res = [None, None];
    if *columns.first().unwrap() == ColumnKind::Gt {
        let mut regs = Vec::new();
        for i in 0..(int.rows.len() / 50) {
            let c = Coord {
                x: 0,
                y: int.rows[RowId::from_idx(i * 50 + 5)] as u16,
            };
            let kind = match &int.rd.tile_kinds.key(int.rd.tiles[&c].kind)[..] {
                "GTH_CHANNEL_0" => Some(GtKind::Gth),
                "GTX_CHANNEL_0" => Some(GtKind::Gtx),
                _ => unreachable!(),
            };
            regs.push(kind);
        }
        res[0] = Some(GtColumn {
            col: columns.first_id().unwrap(),
            regs,
        });
    }
    let col = if *columns.last().unwrap() == ColumnKind::Gt {
        columns.last_id().unwrap()
    } else {
        columns.last_id().unwrap() - 6
    };
    let x = int.cols[col] + 4;
    let mut regs = Vec::new();
    for i in 0..(int.rows.len() / 50) {
        let c = Coord {
            x: x as u16,
            y: int.rows[RowId::from_idx(i * 50 + 5)] as u16,
        };
        let kind = match &int.rd.tile_kinds.key(int.rd.tiles[&c].kind)[..] {
            "GTH_CHANNEL_0" => Some(GtKind::Gth),
            "GTX_CHANNEL_0" => Some(GtKind::Gtx),
            "GTP_CHANNEL_0" => Some(GtKind::Gtp),
            _ => None,
        };
        regs.push(kind);
    }
    if regs.iter().any(|&x| x.is_some()) {
        res[1] = Some(GtColumn { col, regs });
    }
    res
}

fn make_int_db(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("series7", rd);

    builder.wire("GND", int::WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", int::WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..6 {
        builder.wire(
            format!("GCLK{i}"),
            int::WireKind::ClkOut(i),
            &[format!("GCLK_B{i}_EAST"), format!("GCLK_L_B{i}")],
        );
    }
    for i in 6..12 {
        builder.wire(
            format!("GCLK{i}"),
            int::WireKind::ClkOut(i),
            &[format!("GCLK_B{i}"), format!("GCLK_L_B{i}_WEST")],
        );
    }

    for (lr, dir, dbeg, dend) in [
        ("L", Dir::E, Some((3, Dir::N)), Some((0, Dir::S, 3))),
        ("R", Dir::E, Some((0, Dir::S)), Some((3, Dir::N, 3))),
        ("L", Dir::W, Some((3, Dir::N)), Some((3, Dir::N, 1))),
        ("R", Dir::W, Some((0, Dir::S)), Some((0, Dir::S, 1))),
        ("L", Dir::N, Some((3, Dir::N)), Some((0, Dir::S, 3))),
        ("R", Dir::N, None, None),
        ("L", Dir::S, None, None),
        ("R", Dir::S, Some((0, Dir::S)), Some((3, Dir::N, 3))),
    ] {
        for i in 0..4 {
            let beg;
            if let Some((xi, dbeg)) = dbeg {
                if xi == i {
                    let beg_x = builder.mux_out(
                        format!("SNG.{dir}{lr}{i}.0.{dbeg}"),
                        &[format!("{dir}{lr}1BEG_{dbeg}{i}")],
                    );
                    if dir == dbeg {
                        continue;
                    }
                    beg = builder.branch(
                        beg_x,
                        !dbeg,
                        format!("SNG.{dir}{lr}{i}.0"),
                        &[format!("{dir}{lr}1BEG{i}")],
                    );
                } else {
                    beg = builder.mux_out(
                        format!("SNG.{dir}{lr}{i}.0"),
                        &[format!("{dir}{lr}1BEG{i}")],
                    );
                }
            } else {
                beg = builder.mux_out(
                    format!("SNG.{dir}{lr}{i}.0"),
                    &[format!("{dir}{lr}1BEG{i}")],
                );
            }
            let end = builder.branch(
                beg,
                dir,
                format!("SNG.{dir}{lr}{i}.1"),
                &[format!("{dir}{lr}1END{i}")],
            );
            if let Some((xi, dend, n)) = dend {
                if i == xi {
                    builder.branch(
                        end,
                        dend,
                        format!("SNG.{dir}{lr}{i}.2"),
                        &[format!("{dir}{lr}1END_{dend}{n}_{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::E, Dir::E, None),
        (Dir::W, Dir::W, Some((3, Dir::N, 0))),
        (Dir::N, Dir::N, Some((0, Dir::S, 2))),
        (Dir::N, Dir::E, Some((0, Dir::S, 3))),
        (Dir::N, Dir::W, Some((0, Dir::S, 0))),
        (Dir::S, Dir::S, Some((3, Dir::N, 0))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N, 0))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("DBL.{da}{db}{i}.0"), &[format!("{da}{db}2BEG{i}")]);
            let m = builder.branch(
                b,
                da,
                format!("DBL.{da}{db}{i}.1"),
                &[format!("{da}{db}2A{i}")],
            );
            let e = builder.branch(
                m,
                db,
                format!("DBL.{da}{db}{i}.2"),
                &[format!("{da}{db}2END{i}")],
            );
            if let Some((xi, dend, n)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("DBL.{da}{db}{i}.3"),
                        &[format!("{da}{db}2END_{dend}{n}_{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::E, Dir::E, None),
        (Dir::W, Dir::W, Some((0, Dir::S, 0))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("QUAD.{da}{db}{i}.0"), &[format!("{da}{db}4BEG{i}")]);
            let a = builder.branch(
                b,
                db,
                format!("QUAD.{da}{db}{i}.1"),
                &[format!("{da}{db}4A{i}")],
            );
            let m = builder.branch(
                a,
                da,
                format!("QUAD.{da}{db}{i}.2"),
                &[format!("{da}{db}4B{i}")],
            );
            let c = builder.branch(
                m,
                da,
                format!("QUAD.{da}{db}{i}.3"),
                &[format!("{da}{db}4C{i}")],
            );
            let e = builder.branch(
                c,
                db,
                format!("QUAD.{da}{db}{i}.4"),
                &[format!("{da}{db}4END{i}")],
            );
            if let Some((xi, dend, n)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("QUAD.{da}{db}{i}.5"),
                        &[format!("{da}{db}4END_{dend}{n}_{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::N, Dir::N, Some((0, Dir::S, 1))),
        (Dir::N, Dir::E, None),
        (Dir::N, Dir::W, Some((0, Dir::S, 0))),
        (Dir::S, Dir::S, Some((3, Dir::N, 0))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N, 0))),
    ] {
        for i in 0..4 {
            let beg = builder.mux_out(format!("HEX.{da}{db}{i}.0"), &[format!("{da}{db}6BEG{i}")]);
            let a = builder.branch(
                beg,
                db,
                format!("HEX.{da}{db}{i}.1"),
                &[format!("{da}{db}6A{i}")],
            );
            let b = builder.branch(
                a,
                da,
                format!("HEX.{da}{db}{i}.2"),
                &[format!("{da}{db}6B{i}")],
            );
            let c = builder.branch(
                b,
                da,
                format!("HEX.{da}{db}{i}.3"),
                &[format!("{da}{db}6C{i}")],
            );
            let d = builder.branch(
                c,
                da,
                format!("HEX.{da}{db}{i}.4"),
                &[format!("{da}{db}6D{i}")],
            );
            let e = builder.branch(
                d,
                da,
                format!("HEX.{da}{db}{i}.5"),
                &[format!("{da}{db}6E{i}")],
            );
            let end = builder.branch(
                e,
                db,
                format!("HEX.{da}{db}{i}.6"),
                &[format!("{da}{db}6END{i}")],
            );
            if let Some((xi, dend, n)) = dend {
                if i == xi {
                    builder.branch(
                        end,
                        dend,
                        format!("HEX.{da}{db}{i}.7"),
                        &[format!("{da}{db}6END_{dend}{n}_{i}")],
                    );
                }
            }
        }
    }

    // The long wires.
    let mid = builder.wire("LH.6", int::WireKind::MultiOut, &["LH6"]);
    let mut prev = mid;
    for i in (0..6).rev() {
        prev = builder.multi_branch(prev, Dir::E, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mut prev = mid;
    for i in 7..13 {
        prev = builder.multi_branch(prev, Dir::W, format!("LH.{i}"), &[format!("LH{i}")]);
    }

    let mut lv_bh_n = Vec::new();
    let mut lv_bh_s = Vec::new();

    let mid = builder.wire("LV.9", int::WireKind::MultiOut, &["LV9", "LV_L9"]);
    let mut prev = mid;
    for i in (0..9).rev() {
        prev = builder.multi_branch(
            prev,
            Dir::S,
            format!("LV.{i}"),
            &[format!("LV{i}"), format!("LV_L{i}")],
        );
        lv_bh_n.push(prev);
    }
    let mut prev = mid;
    for i in 10..19 {
        prev = builder.multi_branch(
            prev,
            Dir::N,
            format!("LV.{i}"),
            &[format!("LV{i}"), format!("LV_L{i}")],
        );
        lv_bh_s.push(prev);
    }
    let mid = builder.wire(
        "LVB.6",
        int::WireKind::MultiOut,
        &["LVB6", "LVB_L6", "LVB6_SLV", "LVB_L6_SLV"],
    );
    let mut prev = mid;
    for i in (0..6).rev() {
        prev = builder.multi_branch(
            prev,
            Dir::S,
            format!("LVB.{i}"),
            &[format!("LVB{i}"), format!("LVB_L{i}")],
        );
        lv_bh_n.push(prev);
    }
    let mut prev = mid;
    for i in 7..13 {
        prev = builder.multi_branch(
            prev,
            Dir::N,
            format!("LVB.{i}"),
            &[format!("LVB{i}"), format!("LVB_L{i}")],
        );
        lv_bh_s.push(prev);
    }

    // The control inputs.
    for i in 0..2 {
        builder.mux_out(format!("IMUX.GFAN{i}"), &[format!("GFAN{i}")]);
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[format!("CLK{i}"), format!("CLK_L{i}")],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.CTRL{i}"),
            &[format!("CTRL{i}"), format!("CTRL_L{i}")],
        );
    }
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.BYP{i}"), &[format!("BYP_ALT{i}")]);
        builder.buf(
            w,
            format!("IMUX.BYP{i}.SITE"),
            &[format!("BYP{i}"), format!("BYP_L{i}")],
        );
        let b = builder.buf(
            w,
            format!("IMUX.BYP{i}.BOUNCE"),
            &[format!("BYP_BOUNCE{i}")],
        );
        if matches!(i, 2 | 3 | 6 | 7) {
            builder.branch(
                b,
                Dir::N,
                format!("IMUX.BYP{i}.BOUNCE.N"),
                &[format!("BYP_BOUNCE_N3_{i}")],
            );
        }
    }
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.FAN{i}"), &[format!("FAN_ALT{i}")]);
        builder.buf(
            w,
            format!("IMUX.FAN{i}.SITE"),
            &[format!("FAN{i}"), format!("FAN_L{i}")],
        );
        let b = builder.buf(
            w,
            format!("IMUX.FAN{i}.BOUNCE"),
            &[format!("FAN_BOUNCE{i}")],
        );
        if matches!(i, 0 | 2 | 4 | 6) {
            builder.branch(
                b,
                Dir::S,
                format!("IMUX.FAN{i}.BOUNCE.S"),
                &[format!("FAN_BOUNCE_S3_{i}")],
            );
        }
    }
    for i in 0..48 {
        builder.mux_out(
            format!("IMUX.IMUX{i}"),
            &[format!("IMUX{i}"), format!("IMUX_L{i}")],
        );
    }
    for i in 0..48 {
        builder.test_out(
            format!("IMUX.BRAM{i}"),
            &[
                format!("INT_INTERFACE_BRAM_UTURN_IMUX{i}"),
                format!("INT_INTERFACE_BRAM_UTURN_R_IMUX{i}"),
            ],
        );
    }

    for i in 0..24 {
        builder.logic_out(
            format!("OUT{i}"),
            &[format!("LOGIC_OUTS{i}"), format!("LOGIC_OUTS_L{i}")],
        );
    }

    for i in 0..4 {
        builder.test_out(
            format!("TEST{i}"),
            &[
                format!("INT_INTERFACE_BLOCK_OUTS_B{i}"),
                format!("INT_INTERFACE_BLOCK_OUTS_L_B{i}"),
                format!("INT_INTERFACE_PSS_BLOCK_OUTS_L_B{i}"),
            ],
        );
    }

    builder.extract_main_passes();

    builder.node_type("INT_L", "INT", "INT.L");
    builder.node_type("INT_R", "INT", "INT.R");
    builder.node_type("INT_L_SLV_FLY", "INT", "INT.L");
    builder.node_type("INT_R_SLV_FLY", "INT", "INT.R");
    builder.node_type("INT_L_SLV", "INT", "INT.L.SLV");
    builder.node_type("INT_R_SLV", "INT", "INT.R.SLV");

    let forced: Vec<_> = (0..6)
        .map(|i| {
            (
                builder.find_wire(format!("LH.{}", i)),
                builder.find_wire(format!("LH.{}", 11 - i)),
            )
        })
        .collect();
    for tkn in [
        "L_TERM_INT",
        "L_TERM_INT_BRAM",
        "INT_INTERFACE_PSS_L",
        "GTP_INT_INTERFACE_L",
        "GTP_INT_INT_TERM_L",
    ] {
        builder.extract_term_conn("TERM.W", Dir::W, tkn, &forced);
    }
    let forced: Vec<_> = (0..6)
        .map(|i| {
            (
                builder.find_wire(format!("LH.{}", 12 - i)),
                builder.find_wire(format!("LH.{}", i + 1)),
            )
        })
        .collect();
    for tkn in [
        "R_TERM_INT",
        "R_TERM_INT_GTX",
        "GTP_INT_INTERFACE_R",
        "GTP_INT_INT_TERM_R",
    ] {
        builder.extract_term_conn("TERM.E", Dir::E, tkn, &forced);
    }
    let forced = [
        (
            builder.find_wire("SNG.WL3.2"),
            builder.find_wire("SNG.WR0.1"),
        ),
        (
            builder.find_wire("SNG.ER0.0"),
            builder.find_wire("SNG.EL3.0.N"),
        ),
        (
            builder.find_wire("DBL.NW0.1"),
            builder.find_wire("DBL.SW3.0"),
        ),
        (
            builder.find_wire("DBL.NE0.1"),
            builder.find_wire("DBL.SE3.0"),
        ),
        (
            builder.find_wire("HEX.SW3.7"),
            builder.find_wire("HEX.NW0.6"),
        ),
        (
            builder.find_wire("HEX.NE0.5"),
            builder.find_wire("HEX.SE3.4"),
        ),
    ];
    for tkn in [
        "B_TERM_INT",
        "B_TERM_INT_SLV",
        "BRKH_B_TERM_INT",
        "HCLK_L_BOT_UTURN",
        "HCLK_R_BOT_UTURN",
    ] {
        builder.extract_term_conn("TERM.S", Dir::S, tkn, &forced);
    }
    let forced = [
        (
            builder.find_wire("SNG.EL3.0"),
            builder.find_wire("SNG.ER0.0.S"),
        ),
        (
            builder.find_wire("SNG.WR0.2"),
            builder.find_wire("SNG.WL3.1"),
        ),
        (
            builder.find_wire("DBL.SE3.1"),
            builder.find_wire("DBL.NE0.0"),
        ),
        (
            builder.find_wire("HEX.SE3.5"),
            builder.find_wire("HEX.NE0.4"),
        ),
    ];
    for tkn in [
        "T_TERM_INT",
        "T_TERM_INT_SLV",
        "BRKH_TERM_INT",
        "BRKH_INT_PSS",
        "HCLK_L_TOP_UTURN",
        "HCLK_R_TOP_UTURN",
    ] {
        builder.extract_term_conn("TERM.N", Dir::N, tkn, &forced);
    }
    // TODO: this enough?
    builder.make_blackhole_term("TERM.S.HOLE", Dir::S, &lv_bh_s);
    builder.make_blackhole_term("TERM.N.HOLE", Dir::N, &lv_bh_n);

    for (dir, n, tkn) in [
        (Dir::W, "L", "INT_INTERFACE_L"),
        (Dir::E, "R", "INT_INTERFACE_R"),
        (Dir::W, "L", "IO_INT_INTERFACE_L"),
        (Dir::E, "R", "IO_INT_INTERFACE_R"),
        (Dir::W, "PSS", "INT_INTERFACE_PSS_L"),
    ] {
        builder.extract_intf("INTF", dir, tkn, format!("INTF.{n}"), true);
    }
    for (dir, n, tkn) in [
        (Dir::W, "L", "BRAM_INT_INTERFACE_L"),
        (Dir::E, "R", "BRAM_INT_INTERFACE_R"),
    ] {
        builder.extract_intf("INTF.BRAM", dir, tkn, format!("INTF.{n}"), true);
    }
    for (dir, n, tkn) in [
        (Dir::E, "GTP", "GTP_INT_INTERFACE"),
        (Dir::W, "GTP_L", "GTP_INT_INTERFACE_L"),
        (Dir::E, "GTP_R", "GTP_INT_INTERFACE_R"),
        (Dir::E, "GTX", "GTX_INT_INTERFACE"),
        (Dir::W, "GTX_L", "GTX_INT_INTERFACE_L"),
        (Dir::E, "GTH", "GTH_INT_INTERFACE"),
        (Dir::W, "GTH_L", "GTH_INT_INTERFACE_L"),
        (Dir::W, "PCIE_L", "PCIE_INT_INTERFACE_L"),
        (Dir::W, "PCIE_LEFT_L", "PCIE_INT_INTERFACE_LEFT_L"),
        (Dir::E, "PCIE_R", "PCIE_INT_INTERFACE_R"),
        (Dir::W, "PCIE3_L", "PCIE3_INT_INTERFACE_L"),
        (Dir::E, "PCIE3_R", "PCIE3_INT_INTERFACE_R"),
    ] {
        builder.extract_intf("INTF.DELAY", dir, tkn, format!("INTF.{n}"), true);
    }

    let forced: Vec<_> = builder
        .db
        .wires
        .iter()
        .filter_map(|(w, wi)| {
            if wi.name.starts_with("SNG.S") || wi.name.starts_with("SNG.N") {
                None
            } else {
                Some(w)
            }
        })
        .collect();

    builder.extract_pass_buf("BRKH", Dir::S, "BRKH_INT", "BRKH", &forced);

    builder.build()
}

fn make_grids(rd: &Part) -> (EntityVec<SlrId, series7::Grid>, SlrId, Vec<ExtraDie>) {
    let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["B_TERM_INT_SLV"])
        .into_iter()
        .map(|x| x as u16)
        .collect();
    rows_slr_split.insert(0);
    rows_slr_split.insert(rd.height);
    if rows_slr_split.contains(&2) {
        rows_slr_split.remove(&0);
    }
    if rows_slr_split.contains(&(rd.height - 2)) {
        rows_slr_split.remove(&rd.height);
    }
    let rows_slr_split: Vec<_> = rows_slr_split.iter().collect();
    let kind = get_kind(rd);
    let mut grids = EntityVec::new();
    let mut grid_master = None;
    for w in rows_slr_split.windows(2) {
        let int = extract_int_slr(
            rd,
            &[
                "INT_L",
                "INT_R",
                "INT_L_SLV",
                "INT_L_SLV_FLY",
                "INT_R_SLV",
                "INT_R_SLV_FLY",
            ],
            &[
                ExtraCol {
                    tts: &["CFG_CENTER_BOT"],
                    dx: &[-10, -9, -6, -5, -2, -1],
                },
                ExtraCol {
                    tts: &["INT_INTERFACE_PSS_L"],
                    dx: &[
                        -46, -45, -39, -38, -35, -34, -29, -28, -25, -24, -19, -18, -15, -14, -9,
                        -8, -5, -4,
                    ],
                },
            ],
            *w[0],
            *w[1],
        );
        let columns = make_columns(&int);
        let cols_vbrk = get_cols_vbrk(&int);
        let col_cfg = int.lookup_column(int.find_column(&["CFG_CENTER_BOT"]).unwrap() + 3);
        let col_clk = int.lookup_column(int.find_column(&["CLK_HROW_BOT_R"]).unwrap() - 2);
        let has_no_tbuturn = !int.find_rows(&["T_TERM_INT_NOUTURN"]).is_empty();
        let row_cfg = int.lookup_row(int.find_row(&["CFG_CENTER_BOT"]).unwrap() - 10) + 50;
        let row_clk = int.lookup_row(int.find_row(&["CLK_BUFG_BOT_R"]).unwrap()) + 4;
        let has_ps = !int.find_columns(&["INT_INTERFACE_PSS_L"]).is_empty();
        let has_slr = !int.find_columns(&["INT_L_SLV"]).is_empty();
        assert_eq!(row_cfg.to_idx() % 50, 0);
        assert_eq!(row_clk.to_idx() % 50, 0);
        assert_eq!(int.rows.len() % 50, 0);
        let slr = grids.push(series7::Grid {
            kind,
            columns: columns.clone(),
            cols_vbrk: cols_vbrk.clone(),
            col_cfg,
            col_clk,
            cols_io: get_cols_io(&int),
            cols_gt: get_cols_gt(&int, &columns),
            regs: int.rows.len() / 50,
            reg_cfg: row_cfg.to_idx() / 50,
            reg_clk: row_clk.to_idx() / 50,
            holes: get_holes(&int),
            has_ps,
            has_slr,
            has_no_tbuturn,
        });
        if int.find_row(&["CFG_CENTER_MID"]).is_some() {
            grid_master = Some(slr);
        }
    }
    let grid_master = grid_master.unwrap();
    let mut extras = Vec::new();
    if find_row(rd, &["GTZ_BOT"]).is_some() {
        extras.push(ExtraDie::GtzBottom);
    }
    if find_row(rd, &["GTZ_TOP"]).is_some() {
        extras.push(ExtraDie::GtzTop);
    }
    (grids, grid_master, extras)
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_ascii_digit())?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(
    rd: &Part,
    pkg: &str,
    grids: &EntityVec<SlrId, series7::Grid>,
    grid_master: SlrId,
    extras: &[ExtraDie],
    pins: &[PkgPin],
) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let is_7k70t = rd.part.contains("7k70t");
    let io_lookup: HashMap<_, _> = series7::get_io(grids, grid_master)
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = series7::get_gt(grids, grid_master, extras, is_7k70t)
        .into_iter()
        .flat_map(|gt| {
            gt.get_pads()
                .into_iter()
                .map(move |(name, func, pin, idx)| (name, (func, gt.bank, pin, idx)))
        })
        .collect();
    let gtz_lookup: HashMap<_, _> = series7::get_gtz_pads(extras)
        .into_iter()
        .map(|(name, func, bank, pin, bel)| (name, (func, bank, pin, bel)))
        .collect();
    let sm_lookup: HashMap<_, _> = series7::get_sysmon_pads(grids, extras, is_7k70t)
        .into_iter()
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    let ps_lookup: HashMap<_, _> = series7::get_ps_pads(grids)
        .into_iter()
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    let has_14 = io_lookup.values().any(|io| io.bank == 14);
    let has_15 = io_lookup.values().any(|io| io.bank == 15);
    let has_35 = io_lookup.values().any(|io| io.bank == 35);
    let is_spartan = rd.part.contains("7s");
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func = match io.row.to_idx() % 50 {
                    0 => "IO_25".to_string(),
                    49 => "IO_0".to_string(),
                    n => format!(
                        "IO_L{}{}_T{}",
                        (50 - n) / 2,
                        ['P', 'N'][n as usize % 2],
                        3 - (n - 1) / 12
                    ),
                };
                if matches!(pkg, "fbg484" | "fbv484")
                    && rd.part.contains("7k")
                    && io.bank == 16
                    && matches!(io.row.to_idx() % 50, 2 | 14 | 37)
                {
                    exp_func = format!(
                        "IO_{}_T{}",
                        (50 - io.row.to_idx() % 50) / 2,
                        3 - (io.row.to_idx() % 50 - 1) / 12
                    );
                }
                if io.bank == 35 && matches!(io.row.to_idx() % 50, 21 | 22) {
                    if let Some(sm) = io.sm_pair(has_15, has_35) {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.row.to_idx() % 2]).unwrap();
                    }
                }
                if io.is_srcc() {
                    exp_func += "_SRCC";
                }
                if io.is_mrcc() {
                    exp_func += "_MRCC";
                }
                if io.is_dqs() {
                    exp_func += "_DQS";
                }
                match io.get_cfg(has_14) {
                    Some(CfgPin::Data(d)) => {
                        if d >= 16 && !is_spartan {
                            write!(exp_func, "_A{:02}", d - 16).unwrap();
                        }
                        write!(exp_func, "_D{d:02}").unwrap();
                        if d == 0 {
                            exp_func += "_MOSI";
                        }
                        if d == 1 {
                            exp_func += "_DIN";
                        }
                    }
                    Some(CfgPin::Addr(a)) => {
                        if !is_spartan {
                            write!(exp_func, "_A{a}").unwrap();
                        }
                    }
                    Some(CfgPin::Rs(a)) => {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(CfgPin::HswapEn) => exp_func += "_PUDC_B",
                    Some(CfgPin::UserCclk) => exp_func += "_EMCCLK",
                    Some(CfgPin::RdWrB) => exp_func += "_RDWR_B",
                    Some(CfgPin::CsiB) => exp_func += "_CSI_B",
                    Some(CfgPin::Dout) => exp_func += "_DOUT_CSO_B",
                    Some(CfgPin::FweB) => {
                        if !is_spartan {
                            exp_func += "_FWE_B"
                        }
                    }
                    Some(CfgPin::FoeB) => {
                        if !is_spartan {
                            exp_func += "_FOE_B"
                        }
                    }
                    Some(CfgPin::FcsB) => exp_func += "_FCS_B",
                    Some(CfgPin::AdvB) => {
                        if !is_spartan {
                            exp_func += "_ADV_B"
                        }
                    }
                    None => (),
                    _ => unreachable!(),
                }
                if !(io.bank == 35 && matches!(io.row.to_idx() % 50, 21 | 22)) {
                    if let Some(sm) = io.sm_pair(has_15, has_35) {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.row.to_idx() % 2]).unwrap();
                    }
                }
                if io.is_vref() {
                    exp_func += "_VREF";
                }
                if io.is_vrp() {
                    exp_func += "_VRP";
                }
                if io.is_vrn() {
                    exp_func += "_VRN";
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!(
                        "pad {pkg} {pad} {io:?} got {f} exp {exp_func}",
                        f = pin.func
                    );
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::IoByBank(io.bank, (io.row.to_idx() % 50) as u32)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gtz_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
            } else if let Some(&(bank, spin)) = sm_lookup.get(pad) {
                let exp_func = match spin {
                    SysMonPin::VP => "VP_0",
                    SysMonPin::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::SysMonByBank(bank, spin)
            } else if let Some(&(bank, spin)) = ps_lookup.get(pad) {
                let exp_func = match spin {
                    PsPin::Clk => format!("PS_CLK_{bank}"),
                    PsPin::PorB => format!("PS_POR_B_{bank}"),
                    PsPin::SrstB => format!("PS_SRST_B_{bank}"),
                    PsPin::Mio(x) => format!("PS_MIO{x}_{bank}"),
                    PsPin::DdrDm(x) => format!("PS_DDR_DM{x}_{bank}"),
                    PsPin::DdrDq(x) => format!("PS_DDR_DQ{x}_{bank}"),
                    PsPin::DdrDqsP(x) => format!("PS_DDR_DQS_P{x}_{bank}"),
                    PsPin::DdrDqsN(x) => format!("PS_DDR_DQS_N{x}_{bank}"),
                    PsPin::DdrA(x) => format!("PS_DDR_A{x}_{bank}"),
                    PsPin::DdrBa(x) => format!("PS_DDR_BA{x}_{bank}"),
                    PsPin::DdrVrP => format!("PS_DDR_VRP_{bank}"),
                    PsPin::DdrVrN => format!("PS_DDR_VRN_{bank}"),
                    PsPin::DdrCkP(0) => format!("PS_DDR_CKP_{bank}"),
                    PsPin::DdrCkN(0) => format!("PS_DDR_CKN_{bank}"),
                    PsPin::DdrCke(0) => format!("PS_DDR_CKE_{bank}"),
                    PsPin::DdrOdt(0) => format!("PS_DDR_ODT_{bank}"),
                    PsPin::DdrDrstB => format!("PS_DDR_DRST_B_{bank}"),
                    PsPin::DdrCsB(0) => format!("PS_DDR_CS_B_{bank}"),
                    PsPin::DdrRasB => format!("PS_DDR_RAS_B_{bank}"),
                    PsPin::DdrCasB => format!("PS_DDR_CAS_B_{bank}"),
                    PsPin::DdrWeB => format!("PS_DDR_WE_B_{bank}"),
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f = pin.func);
                }
                BondPin::IoPs(bank, spin)
            } else {
                println!("unk iopad {pad} {f}", f = pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VCCBRAM" => BondPin::VccBram,
                "VCCBATT_0" => BondPin::VccBatt,
                "TCK_0" => BondPin::Cfg(CfgPin::Tck),
                "TDI_0" => BondPin::Cfg(CfgPin::Tdi),
                "TDO_0" => BondPin::Cfg(CfgPin::Tdo),
                "TMS_0" => BondPin::Cfg(CfgPin::Tms),
                "CCLK_0" => BondPin::Cfg(CfgPin::Cclk),
                "RSVDGND" if !has_14 => BondPin::Cfg(CfgPin::Cclk),
                "RSVDVCC3" if !has_14 => BondPin::Cfg(CfgPin::M0),
                "RSVDVCC2" if !has_14 => BondPin::Cfg(CfgPin::M1),
                "RSVDVCC1" if !has_14 => BondPin::Cfg(CfgPin::M2),
                "RSVDGND" => BondPin::RsvdGnd, // used for disabled transceiver RX pins on 7a12t
                "DONE_0" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM_B_0" => BondPin::Cfg(CfgPin::ProgB),
                "INIT_B_0" => BondPin::Cfg(CfgPin::InitB),
                "M0_0" => BondPin::Cfg(CfgPin::M0),
                "M1_0" => BondPin::Cfg(CfgPin::M1),
                "M2_0" => BondPin::Cfg(CfgPin::M2),
                "CFGBVS_0" => BondPin::Cfg(CfgPin::CfgBvs),
                "DXN_0" => BondPin::Dxn,
                "DXP_0" => BondPin::Dxp,
                "GNDADC_0" | "GNDADC" => {
                    BondPin::SysMonByBank(grid_master.to_idx() as u32, SysMonPin::AVss)
                }
                "VCCADC_0" | "VCCADC" => {
                    BondPin::SysMonByBank(grid_master.to_idx() as u32, SysMonPin::AVdd)
                }
                "VREFP_0" => BondPin::SysMonByBank(grid_master.to_idx() as u32, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMonByBank(grid_master.to_idx() as u32, SysMonPin::VRefN),
                "MGTAVTT" => BondPin::GtByRegion(10, GtRegionPin::AVtt),
                "MGTAVCC" => BondPin::GtByRegion(10, GtRegionPin::AVcc),
                "MGTVCCAUX" => BondPin::GtByRegion(10, GtRegionPin::VccAux),
                "VCCO_MIO0_500" => BondPin::VccO(500),
                "VCCO_MIO1_501" => BondPin::VccO(501),
                "VCCO_DDR_502" => BondPin::VccO(502),
                "VCCPINT" => BondPin::VccPsInt,
                "VCCPAUX" => BondPin::VccPsAux,
                "VCCPLL" => BondPin::VccPsPll,
                "PS_MIO_VREF_501" => BondPin::IoVref(501, 0),
                "PS_DDR_VREF0_502" => BondPin::IoVref(502, 0),
                "PS_DDR_VREF1_502" => BondPin::IoVref(502, 1),
                _ => {
                    if let Some((n, b)) = split_num(&pin.func) {
                        match n {
                            "VCCO_" => BondPin::VccO(b),
                            "VCCAUX_IO_G" => BondPin::VccAuxIo(b),
                            "MGTAVTTRCAL_" => BondPin::GtByBank(b, GtPin::AVttRCal, 0),
                            "MGTRREF_" => BondPin::GtByBank(b, GtPin::RRef, 0),
                            "MGTAVTT_G" => BondPin::GtByRegion(b, GtRegionPin::AVtt),
                            "MGTAVCC_G" => BondPin::GtByRegion(b, GtRegionPin::AVcc),
                            "MGTVCCAUX_G" => BondPin::GtByRegion(b, GtRegionPin::VccAux),
                            "MGTZAGND_" => BondPin::GtByBank(b, GtPin::GtzAGnd, 0),
                            "MGTZAVCC_" => BondPin::GtByBank(b, GtPin::GtzAVcc, 0),
                            "MGTZVCCH_" => BondPin::GtByBank(b, GtPin::GtzVccH, 0),
                            "MGTZVCCL_" => BondPin::GtByBank(b, GtPin::GtzVccL, 0),
                            "MGTZ_OBS_CLK_P_" => BondPin::GtByBank(b, GtPin::GtzObsClkP, 0),
                            "MGTZ_OBS_CLK_N_" => BondPin::GtByBank(b, GtPin::GtzObsClkN, 0),
                            "MGTZ_SENSE_AVCC_" => BondPin::GtByBank(b, GtPin::GtzSenseAVcc, 0),
                            "MGTZ_SENSE_AGND_" => BondPin::GtByBank(b, GtPin::GtzSenseAGnd, 0),
                            "MGTZ_SENSE_GNDL_" => BondPin::GtByBank(b, GtPin::GtzSenseGndL, 0),
                            "MGTZ_SENSE_GND_" => BondPin::GtByBank(b, GtPin::GtzSenseGnd, 0),
                            "MGTZ_SENSE_VCC_" => BondPin::GtByBank(b, GtPin::GtzSenseVcc, 0),
                            "MGTZ_SENSE_VCCL_" => BondPin::GtByBank(b, GtPin::GtzSenseVccL, 0),
                            "MGTZ_SENSE_VCCH_" => BondPin::GtByBank(b, GtPin::GtzSenseVccH, 0),
                            "MGTZ_THERM_IN_" => BondPin::GtByBank(b, GtPin::GtzThermIn, 0),
                            "MGTZ_THERM_OUT_" => BondPin::GtByBank(b, GtPin::GtzThermOut, 0),
                            _ => {
                                println!("UNK FUNC {} {} {:?}", pkg, pin.func, pin);
                                continue;
                            }
                        }
                    } else {
                        println!("UNK FUNC {} {} {:?}", pkg, pin.func, pin);
                        continue;
                    }
                }
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks: Default::default(),
    }
}

pub fn ingest(rd: &Part) -> (PreDevice, Option<int::IntDb>) {
    let (grids, grid_master, extras) = make_grids(rd);
    let int_db = make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(rd, pkg, &grids, grid_master, &extras, pins),
        ));
    }
    let grid_refs = grids.map_values(|x| x);
    let eint = expand_grid(&grid_refs, grid_master, &extras, &int_db);
    let vrf = Verifier::new(rd, &eint);
    vrf.finish();
    let grids = grids.into_map_values(geom::Grid::Series7);
    (
        make_device_multi(rd, grids, grid_master, extras, bonds, BTreeSet::new()),
        Some(int_db),
    )
}
