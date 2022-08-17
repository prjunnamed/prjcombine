use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write;
use prjcombine_xilinx_rawdump::{Part, PkgPin, NodeId, Coord};
use prjcombine_xilinx_geom::{self as geom, CfgPin, Bond, BondPin, GtPin, GtRegionPin, SysMonPin, DisabledPart, PsPin, HbmPin, AdcPin, DacPin, ColId, SlrId, int, int::Dir};
use prjcombine_xilinx_geom::ultrascale::{self, GridKind, Column, ColumnKindLeft, ColumnKindRight, IoColumn, IoRowKind, HardColumn, HardRowKind, Ps, IoKind, Gt, ColSide, expand_grid};
use prjcombine_entity::{EntityVec, EntityId};
use crate::verify::Verifier;

use enum_map::enum_map;

use crate::grid::{extract_int_slr, find_rows, IntGrid, PreDevice, make_device_multi};
use crate::intb::IntBuilder;

fn make_columns(int: &IntGrid) -> EntityVec<ColId, Column> {
    let mut res: EntityVec<ColId, (Option<ColumnKindLeft>, Option<ColumnKindRight>)> = int.cols.map_values(|_| (None, None));
    for (tkn, delta, kind) in [
        ("CLEL_L", 1, ColumnKindLeft::CleL),
        ("CLE_M", 1, ColumnKindLeft::CleM),
        ("CLE_M_R", 1, ColumnKindLeft::CleM),
        ("CLEM", 1, ColumnKindLeft::CleM),
        ("CLEM_R", 1, ColumnKindLeft::CleM),
        ("INT_INTF_LEFT_TERM_PSS", 1, ColumnKindLeft::CleM),
        ("BRAM", 2, ColumnKindLeft::Bram),
        ("URAM_URAM_FT", 2, ColumnKindLeft::Uram),
        ("INT_INT_INTERFACE_GT_LEFT_FT", 1, ColumnKindLeft::Gt),
        ("INT_INTF_L_TERM_GT", 1, ColumnKindLeft::Gt),
        ("INT_INT_INTERFACE_XIPHY_FT", 1, ColumnKindLeft::Io),
        ("INT_INTF_LEFT_TERM_IO_FT", 1, ColumnKindLeft::Io),
        ("INT_INTF_L_IO", 1, ColumnKindLeft::Io),
    ] {
        for c in int.find_columns(&[tkn]) {
            res[int.lookup_column(c + delta)].0 = Some(kind);
        }
    }
    for (tkn, delta, kind) in [
        ("CLEL_R", 1, ColumnKindRight::CleL),
        ("DSP", 2, ColumnKindRight::Dsp),
        ("URAM_URAM_FT", 2, ColumnKindRight::Uram),
        ("INT_INTERFACE_GT_R", 1, ColumnKindRight::Gt),
        ("INT_INTF_R_TERM_GT", 1, ColumnKindRight::Gt),
        ("INT_INTF_RIGHT_TERM_IO", 1, ColumnKindRight::Io),
    ] {
        for c in int.find_columns(&[tkn]) {
            res[int.lookup_column(c - delta)].1 = Some(kind);
        }
    }
    for c in int.find_columns(&[
        // Ultrascale
        "CFG_CFG",
        "PCIE",
        "CMAC_CMAC_FT",
        "ILMAC_ILMAC_FT",
        // Ultrascale+
        "CFG_CONFIG",
        "PCIE4_PCIE4_FT",
        "PCIE4C_PCIE4C_FT",
        "CMAC",
        "ILKN_ILKN_FT",
        "HDIO_BOT_RIGHT",
        "DFE_DFE_TILEA_FT",
        "DFE_DFE_TILEG_FT",
    ]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::Hard);
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::Hard);
    }
    for c in int.find_columns(&["FE_FE_FT"]) {
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::Sdfec);
    }
    for c in int.find_columns(&["DFE_DFE_TILEB_FT"]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::DfeB);
    }
    for c in int.find_columns(&["DFE_DFE_TILEC_FT"]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::DfeC);
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::DfeC);
    }
    for c in int.find_columns(&["DFE_DFE_TILED_FT"]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::DfeDF);
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::DfeDF);
    }
    for c in int.find_columns(&["DFE_DFE_TILEE_FT"]) {
        res[int.lookup_column_inter(c) - 1].1 = Some(ColumnKindRight::DfeE);
        res[int.lookup_column_inter(c)].0 = Some(ColumnKindLeft::DfeE);
    }
    for c in int.find_columns(&["RCLK_CLEM_CLKBUF_L"]) {
        let c = int.lookup_column(c + 1);
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM));
        res[c].0 = Some(ColumnKindLeft::CleMClkBuf);
    }
    for c in int.find_columns(&["LAGUNA_TILE"]) {
        let c = int.lookup_column(c + 1);
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM));
        res[c].0 = Some(ColumnKindLeft::CleMLaguna);
    }
    for c in int.find_columns(&["LAG_LAG"]) {
        let c = int.lookup_column(c + 2);
        assert_eq!(res[c].0, Some(ColumnKindLeft::CleM));
        res[c].0 = Some(ColumnKindLeft::CleMLaguna);
    }
    for c in int.find_columns(&["RCLK_CLEL_R_DCG10_R"]) {
        let c = int.lookup_column(c - 1);
        assert_eq!(res[c].1, Some(ColumnKindRight::CleL));
        res[c].1 = Some(ColumnKindRight::CleLDcg10);
    }
    for (tkn, kind) in [
        ("RCLK_RCLK_BRAM_L_AUXCLMP_FT", ColumnKindLeft::BramAuxClmp),
        ("RCLK_RCLK_BRAM_L_BRAMCLMP_FT", ColumnKindLeft::BramBramClmp),
        ("RCLK_BRAM_INTF_TD_L", ColumnKindLeft::BramTd),
        ("RCLK_BRAM_INTF_TD_R", ColumnKindLeft::BramTd),
    ] {
        for c in int.find_columns(&[tkn]) {
            let c = int.lookup_column(c + 2);
            assert_eq!(res[c].0, Some(ColumnKindLeft::Bram));
            res[c].0 = Some(kind);
        }
    }
    for c in int.find_columns(&["RCLK_DSP_CLKBUF_L"]) {
        let c = int.lookup_column(c - 2);
        assert_eq!(res[c].1, Some(ColumnKindRight::Dsp));
        res[c].1 = Some(ColumnKindRight::DspClkBuf);
    }
    for c in int.find_columns(&["RCLK_DSP_INTF_CLKBUF_L"]) {
        let c = int.lookup_column(c - 1);
        assert_eq!(res[c].1, Some(ColumnKindRight::Dsp));
        res[c].1 = Some(ColumnKindRight::DspClkBuf);
    }
    for (i, &(l, r)) in res.iter() {
        if l.is_none() {
            println!("FAILED TO DETERMINE COLUMN {}.L", i.to_idx());
        }
        if r.is_none() {
            println!("FAILED TO DETERMINE COLUMN {}.R", i.to_idx());
        }
    }
    res.into_map_values(|(l, r)| Column {l: l.unwrap(), r: r.unwrap()})
}

fn get_cols_vbrk(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["CFRM_CBRK_L", "CFRM_CBRK_R"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_fsr_gap(int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in int.find_columns(&["FSR_GAP"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_hard(int: &IntGrid) -> Vec<HardColumn> {
    let mut vp_aux0: HashSet<NodeId> = HashSet::new();
    if let Some((_, tk)) = int.rd.tile_kinds.get("AMS") {
        for (i, &v) in tk.conn_wires.iter() {
            if &int.rd.wires[v] == "AMS_AMS_CORE_0_VP_AUX0" {
                for crd in &tk.tiles {
                    let tile = &int.rd.tiles[crd];
                    if let Some(&n) = tile.conn_wires.get(i) {
                        vp_aux0.insert(n);
                    }
                }
            }
        }
    }
    let mut cells = BTreeMap::new();
    for (tt, kind) in [
        // Ultrascale
        ("CFG_CFG", HardRowKind::Cfg),
        ("CFGIO_IOB", HardRowKind::Ams),
        ("PCIE", HardRowKind::Pcie),
        ("CMAC_CMAC_FT", HardRowKind::Cmac),
        ("ILMAC_ILMAC_FT", HardRowKind::Ilkn),
        // Ultrascale+
        ("CFG_CONFIG", HardRowKind::Cfg),
        ("CFGIO_IOB20", HardRowKind::Ams),
        ("PCIE4_PCIE4_FT", HardRowKind::Pcie),
        ("PCIE4C_PCIE4C_FT", HardRowKind::PciePlus),
        ("CMAC", HardRowKind::Cmac),
        ("ILKN_ILKN_FT", HardRowKind::Ilkn),
        ("DFE_DFE_TILEA_FT", HardRowKind::DfeA),
        ("DFE_DFE_TILEG_FT", HardRowKind::DfeG),
        ("HDIO_BOT_RIGHT", HardRowKind::Hdio),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x);
            let row = int.lookup_row(y).to_idx() / 60;
            cells.insert((col, row), kind);
        }
    }
    if let Some((_, tk)) = int.rd.tile_kinds.get("HDIO_TOP_RIGHT") {
        for (i, &v) in tk.conn_wires.iter() {
            if &int.rd.wires[v] == "HDIO_IOBPAIR_53_SWITCH_OUT" {
                for crd in &tk.tiles {
                    if !(int.slr_start..int.slr_end).contains(&crd.y) {
                        continue;
                    }
                    let col = int.lookup_column_inter(crd.x as i32);
                    let row = int.lookup_row(crd.y as i32).to_idx() / 60;
                    let tile = &int.rd.tiles[crd];
                    if let Some(&n) = tile.conn_wires.get(i) {
                        if vp_aux0.contains(&n) {
                            cells.insert((col, row), HardRowKind::HdioAms);
                        }
                    }
                }
            }
        }
    }
    let cols: BTreeSet<ColId> = cells.keys().map(|&(c, _)| c).collect();
    let mut res = Vec::new();
    for col in cols {
        let mut regs = Vec::new();
        for _ in 0..(int.rows.len() / 60) {
            regs.push(HardRowKind::None);
        }
        for (&(c, r), &kind) in cells.iter() {
            if c == col {
                assert_eq!(regs[r], HardRowKind::None);
                regs[r] = kind;
            }
        }
        res.push(HardColumn {
            col,
            regs,
        });
    }
    res
}

fn get_cols_io(int: &IntGrid) -> Vec<IoColumn> {
    let mut cells = BTreeMap::new();
    for (tt, kind) in [
        // Ultrascale
        ("HPIO_L", IoRowKind::Hpio),
        ("HRIO_L", IoRowKind::Hrio),
        ("GTH_QUAD_LEFT_FT", IoRowKind::Gth),
        ("GTY_QUAD_LEFT_FT", IoRowKind::Gty),
        // Ultrascale+
        // [reuse HPIO_L]
        ("GTH_QUAD_LEFT", IoRowKind::Gth),
        ("GTY_L", IoRowKind::Gty),
        ("GTM_DUAL_LEFT_FT", IoRowKind::Gtm),
        ("GTFY_QUAD_LEFT_FT", IoRowKind::Gtf),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x);
            let row = int.lookup_row(y).to_idx() / 60;
            cells.insert((col, ColSide::Left, row), kind);
        }
    }
    for (tt, kind) in [
        // Ultrascale
        ("GTH_R", IoRowKind::Gth),
        // Ultrascale+
        ("HPIO_RIGHT", IoRowKind::Hpio),
        ("GTH_QUAD_RIGHT", IoRowKind::Gth),
        ("GTY_R", IoRowKind::Gty),
        ("GTM_DUAL_RIGHT_FT", IoRowKind::Gtm),
        ("GTFY_QUAD_RIGHT_FT", IoRowKind::Gtf),
        ("HSADC_HSADC_RIGHT_FT", IoRowKind::HsAdc),
        ("HSDAC_HSDAC_RIGHT_FT", IoRowKind::HsDac),
        ("RFADC_RFADC_RIGHT_FT", IoRowKind::RfAdc),
        ("RFDAC_RFDAC_RIGHT_FT", IoRowKind::RfDac),
    ] {
        for (x, y) in int.find_tiles(&[tt]) {
            let col = int.lookup_column_inter(x) - 1;
            let row = int.lookup_row(y).to_idx() / 60;
            cells.insert((col, ColSide::Right, row), kind);
        }
    }
    let cols: BTreeSet<(ColId, ColSide)> = cells.keys().map(|&(c, s, _)| (c, s)).collect();
    let mut res = Vec::new();
    for (col, side) in cols {
        let mut regs = Vec::new();
        for _ in 0..(int.rows.len() / 60) {
            regs.push(IoRowKind::None);
        }
        for (&(c, s, r), &kind) in cells.iter() {
            if c == col && side == s {
                assert_eq!(regs[r], IoRowKind::None);
                regs[r] = kind;
            }
        }
        res.push(IoColumn {
            col,
            side,
            regs,
        });
    }
    res
}

fn get_ps(int: &IntGrid) -> Option<Ps> {
    let col = int.lookup_column(int.find_column(&["INT_INTF_LEFT_TERM_PSS"])? + 1);
    Some(Ps {
        col,
        has_vcu: int.find_column(&["VCU_VCU_FT"]).is_some(),
    })
}

fn make_int_db_u(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("ultrascale", rd);
    builder.node_type("INT", "INT", "INT");
    let w = builder.wire("VCC", int::WireKind::Tie1, &["VCC_WIRE"]);
    builder.extra_name("VCC_WIRE", w);
    builder.wire("GND", int::WireKind::Tie1, &["GND_WIRE"]);

    for i in 0..16 {
        builder.wire(format!("GCLK{i}"), int::WireKind::ClkOut(i), &[
            format!("GCLK_B_0_{i}"),
        ]);
    }

    for (iq, q) in ["NE", "NW", "SE", "SW"].into_iter().enumerate() {
        for (ih, h) in ["E", "W"].into_iter().enumerate() {
            for i in 0..16 {
                match (iq, i) {
                    (1 | 3, 0) => {
                        let w = builder.mux_out(
                            format!("SDND.{q}.{h}.{i}"),
                            &[format!("SDND{q}_{h}_{i}_FTS")],
                        );
                        builder.branch(w, Dir::S,
                            format!("SDND.{q}.{h}.{i}.S"),
                            &[format!("SDND{q}_{h}_BLS_{i}_FTN")],
                        );
                    }
                    (1 | 3, 15) => {
                        let w = builder.mux_out(
                            format!("SDND.{q}.{h}.{i}"),
                            &[format!("SDND{q}_{h}_{i}_FTN")],
                        );
                        builder.branch(w, Dir::N,
                            format!("SDND.{q}.{h}.{i}.N"),
                            &[format!("SDND{q}_{h}_BLN_{i}_FTS")],
                        );
                    }
                    _ => {
                        let xlat = [
                            0, 7, 8, 9, 10, 11, 12, 13,
                            14, 15, 1, 2, 3, 4, 5, 6,
                        ];
                        builder.mux_out(
                            format!("SDND.{q}.{h}.{i}"),
                            &[format!("INT_NODE_SINGLE_DOUBLE_{n}_INT_OUT", n = iq * 32 + ih * 16 + xlat[i])],
                        );
                    }
                }
            }
        }
    }
    // Singles.
    for i in 0..8 {
        let beg = builder.mux_out(
            format!("SNG.E.E.{i}.0"),
            &[format!("EE1_E_BEG{i}")],
        );
        let end = builder.branch(beg, Dir::E,
            format!("SNG.E.E.{i}.1"),
            &[format!("EE1_E_END{i}")],
        );
        if i == 0 {
            builder.branch(end, Dir::S,
                format!("SNG.E.E.{i}.1.S"),
                &[format!("EE1_E_BLS_{i}_FTN")],
            );
        }
    }
    for i in 0..8 {
        if i == 0 {
            let beg = builder.mux_out(
                format!("SNG.E.W.{i}.0"),
                &[format!("EE1_W_{i}_FTS")],
            );
            builder.branch(beg, Dir::S,
                format!("SNG.E.W.{i}.0.S"),
                &[format!("EE1_W_BLS_{i}_FTN")],
            );
        } else {
            builder.mux_out(
                format!("SNG.E.W.{i}.0"),
                &[format!("INT_INT_SINGLE_{n}_INT_OUT", n = i + 8)],
            );
        }
    }
    for i in 0..8 {
        builder.mux_out(
            format!("SNG.W.E.{i}.0"),
            &[format!("INT_INT_SINGLE_{n}_INT_OUT", n = i + 48)],
        );
    }
    for i in 0..8 {
        let beg = builder.mux_out(
            format!("SNG.W.W.{i}.0"),
            &[format!("WW1_W_BEG{i}")],
        );
        builder.branch(beg, Dir::W,
            format!("SNG.W.W.{i}.1"),
            &[format!("WW1_W_END{i}")],
        );
    }
    for dir in [Dir::N, Dir::S] {
        for ew in ["E", "W"] {
            for i in 0..8 {
                let beg = builder.mux_out(
                    format!("SNG.{dir}.{ew}.{i}.0"),
                    &[format!("{dir}{dir}1_{ew}_BEG{i}")],
                );
                let end = builder.branch(beg, dir,
                    format!("SNG.{dir}.{ew}.{i}.1"),
                    &[format!("{dir}{dir}1_{ew}_END{i}")],
                );
                if i == 0 && dir == Dir::S {
                    builder.branch(end, Dir::S,
                        format!("SNG.{dir}.{ew}.{i}.1.S"),
                        &[format!("{dir}{dir}1_{ew}_BLS_{i}_FTN")],
                    );
                }
            }
        }
    }
    // Doubles.
    for dir in [Dir::E, Dir::W] {
        for ew in ["E", "W"] {
            for i in 0..8 {
                let beg = builder.mux_out(
                    format!("DBL.{dir}.{ew}.{i}.0"),
                    &[format!("{dir}{dir}2_{ew}_BEG{i}")],
                );
                let end = builder.branch(beg, dir,
                    format!("DBL.{dir}.{ew}.{i}.1"),
                    &[format!("{dir}{dir}2_{ew}_END{i}")],
                );
                if i == 7 && dir == Dir::E {
                    builder.branch(end, Dir::N,
                        format!("DBL.{dir}.{ew}.{i}.1.N"),
                        &[format!("{dir}{dir}2_{ew}_BLN_{i}_FTS")],
                    );
                }
            }
        }
    }
    for dir in [Dir::N, Dir::S] {
        let ftd = !dir;
        for ew in ["E", "W"] {
            for i in 0..8 {
                let beg = builder.mux_out(
                    format!("DBL.{dir}.{ew}.{i}.0"),
                    &[format!("{dir}{dir}2_{ew}_BEG{i}")],
                );
                let a = builder.branch(beg, dir,
                    format!("DBL.{dir}.{ew}.{i}.1"),
                    &[format!("{dir}{dir}2_{ew}_A_FT{ftd}{i}")],
                );
                let end = builder.branch(a, dir,
                    format!("DBL.{dir}.{ew}.{i}.2"),
                    &[format!("{dir}{dir}2_{ew}_END{i}")],
                );
                if i == 7 && dir == Dir::N {
                    builder.branch(end, Dir::N,
                        format!("DBL.{dir}.{ew}.{i}.2.N"),
                        &[format!("{dir}{dir}2_{ew}_BLN_{i}_FTS")],
                    );
                }
            }
        }
    }

    for (iq, q) in ["NE", "NW", "SE", "SW"].into_iter().enumerate() {
        for (ih, h) in ['E', 'W'].into_iter().enumerate() {
            for i in 0..16 {
                match (q, h, i) {
                    ("NW", 'E', 0) |
                    ("SW", 'E', 0) |
                    ("NW", 'W', 0) |
                    ("NW", 'W', 1) => {
                        let w = builder.mux_out(
                            format!("QLND.{q}.{h}.{i}"),
                            &[format!("QLND{q}_{h}_{i}_FTS")],
                        );
                        builder.branch(w, Dir::S,
                            format!("QLND.{q}.{h}.{i}.S"),
                            &[format!("QLND{q}_{h}_BLS_{i}_FTN")],
                        );
                    }
                    ("NW", 'E', 15) |
                    ("SW", 'E', 15) |
                    ("SE", 'W', 15) => {
                        let w = builder.mux_out(
                            format!("QLND.{q}.{h}.{i}"),
                            &[format!("QLND{q}_{h}_{i}_FTN")],
                        );
                        builder.branch(w, Dir::N,
                            format!("QLND.{q}.{h}.{i}.N"),
                            &[format!("QLND{q}_{h}_BLN_{i}_FTS")],
                        );
                    }
                    _ => {
                        let xlat = [
                            0, 7, 8, 9, 10, 11, 12, 13,
                            14, 15, 1, 2, 3, 4, 5, 6,
                        ];
                        builder.mux_out(
                            format!("QLND.{q}.{h}.{i}"),
                            &[format!("INT_NODE_QUAD_LONG_{n}_INT_OUT", n = iq * 32 + ih * 16 + xlat[i])],
                        );
                    }
                }
            }
        }
    }
    for (dir, name, l, n, fts, ftn) in [
        (Dir::E, "QUAD", 2, 16, true, true),
        (Dir::W, "QUAD", 2, 16, false, false),
        (Dir::N, "QUAD.4", 4, 8, false, false),
        (Dir::N, "QUAD.5", 5, 8, false, true),
        (Dir::S, "QUAD.4", 4, 8, false, false),
        (Dir::S, "QUAD.5", 5, 8, false, false),
        (Dir::E, "LONG", 6, 8, true, false),
        (Dir::W, "LONG", 6, 8, false, true),
        (Dir::N, "LONG.12", 12, 4, false, false),
        (Dir::N, "LONG.16", 16, 4, false, true),
        (Dir::S, "LONG.12", 12, 4, true, false),
        (Dir::S, "LONG.16", 16, 4, false, false),
    ] {
        let ftd = !dir;
        let ll = if matches!(dir, Dir::E | Dir::W) {l * 2} else {l};
        for i in 0..n {
            let mut w = builder.mux_out(
                format!("{name}.{dir}.{i}.0"),
                &[format!("{dir}{dir}{ll}_BEG{i}")],
            );
            for j in 1..l {
                let nn = (b'A' + (j - 1)) as char;
                w = builder.branch(w, dir,
                    format!("{name}.{dir}.{i}.{j}"),
                    &[format!("{dir}{dir}{ll}_{nn}_FT{ftd}{i}")],
                );
            }
            w = builder.branch(w, dir,
                format!("{name}.{dir}.{i}.{l}"),
                &[format!("{dir}{dir}{ll}_END{i}")],
            );
            if i == 0 && fts {
                builder.branch(w, Dir::S,
                    format!("{name}.{dir}.{i}.{l}.S"),
                    &[format!("{dir}{dir}{ll}_BLS_{i}_FTN")],
                );
            }
            if i == (n - 1) && ftn {
                builder.branch(w, Dir::N,
                    format!("{name}.{dir}.{i}.{l}.N"),
                    &[format!("{dir}{dir}{ll}_BLN_{i}_FTS")],
                );
            }
        }
    }

    for i in 0..16 {
        for j in 0..2 {
            builder.mux_out(
                format!("INT_NODE_GLOBAL.{i}.{j}"),
                &[format!("INT_NODE_GLOBAL_{i}_OUT{j}")],
            );
        }
    }
    for i in 0..8 {
        builder.mux_out(
            format!("IMUX.E.CTRL.{i}"),
            &[format!("CTRL_E_B{i}")],
        );
    }
    for i in 0..10 {
        builder.mux_out(
            format!("IMUX.W.CTRL.{i}"),
            &[format!("CTRL_W_B{i}")],
        );
    }

    for (iq, q) in ["1", "2"].into_iter().enumerate() {
        for (ih, h) in ['E', 'W'].into_iter().enumerate() {
            for i in 0..32 {
                match i {
                    1 | 3 => {
                        let w = builder.mux_out(
                            format!("INODE.{q}.{h}.{i}"),
                            &[format!("INODE_{q}_{h}_{i}_FTS")],
                        );
                        builder.branch(w, Dir::S,
                            format!("INODE.{q}.{h}.{i}.S"),
                            &[format!("INODE_{q}_{h}_BLS_{i}_FTN")],
                        );
                    }
                    28 | 30 => {
                        let w = builder.mux_out(
                            format!("INODE.{q}.{h}.{i}"),
                            &[format!("INODE_{q}_{h}_{i}_FTN")],
                        );
                        builder.branch(w, Dir::N,
                            format!("INODE.{q}.{h}.{i}.N"),
                            &[format!("INODE_{q}_{h}_BLN_{i}_FTS")],
                        );
                    }
                    _ => {
                        let xlat = [
                            0, 11, 22, 25, 26, 27, 28, 29,
                            30, 31, 1, 2, 3, 4, 5, 6,
                            7, 8, 9, 10, 12, 13, 14, 15,
                            16, 17, 18, 19, 20, 21, 23, 24,
                        ];
                        builder.mux_out(
                            format!("INODE.{q}.{h}.{i}"),
                            &[format!("INT_NODE_IMUX_{n}_INT_OUT", n = iq * 64 + ih * 32 + xlat[i])],
                        );
                    }
                }
            }
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..16 {
            match i {
                1 | 3 | 5 | 7 | 11 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FTS")],
                    );
                    builder.branch(w, Dir::S,
                        format!("IMUX.{ew}.BYP.{i}.S"),
                        &[format!("BOUNCE_{ew}_BLS_{i}_FTN")],
                    );
                }
                8 | 10 | 12 | 14 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FTN")],
                    );
                    builder.branch(w, Dir::N,
                        format!("IMUX.{ew}.BYP.{i}.N"),
                        &[format!("BOUNCE_{ew}_BLN_{i}_FTS")],
                    );
                }
                _ => {
                    builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BYPASS_{ew}{i}")],
                    );
                }
            }
        }
    }
    for ew in ['E', 'W'] {
        for i in 0..48 {
            builder.mux_out(
                format!("IMUX.{ew}.IMUX.{i}"),
                &[format!("IMUX_{ew}{i}")],
            );
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..32 {
            builder.logic_out(
                format!("OUT.{ew}.{i}"),
                &[format!("LOGIC_OUTS_{ew}{i}")],
            );
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..4 {
            let w = builder.test_out(format!("TEST.{ew}.{i}"));
            let tiles: &[&str] = if ew == 'W' {&[
                "INT_INTERFACE_L",
                "INT_INT_INTERFACE_XIPHY_FT",
                "INT_INTERFACE_PCIE_L",
                "INT_INT_INTERFACE_GT_LEFT_FT",
            ]} else {&[
                "INT_INTERFACE_R",
                "INT_INTERFACE_PCIE_R",
                "INT_INTERFACE_GT_R",
            ]};
            for &t in tiles {
                builder.extra_name_tile(t, format!("BLOCK_OUTS{i}"), w);
            }
        }
    }

    for i in 0..16 {
        let w = builder.mux_out(format!("RCLK.IMUX.CE.{i}"), &[""]);
        builder.extra_name(format!("CLK_BUFCE_LEAF_X16_0_CE_INT{i}"), w);
    }
    for i in 0..2 {
        for j in 0..4 {
            let w = builder.mux_out(format!("RCLK.IMUX.LEFT.{i}.{j}"), &[""]);
            builder.extra_name(format!("INT_RCLK_TO_CLK_LEFT_{i}_{j}"), w);
        }
    }
    for i in 0..2 {
        for j in 0..4 {
            let w = builder.mux_out(format!("RCLK.IMUX.RIGHT.{i}.{j}"), &[""]);
            builder.extra_name(format!("INT_RCLK_TO_CLK_RIGHT_{i}_{j}"), w);
        }
    }
    for i in 0..48 {
        let w = builder.mux_out(format!("RCLK.INODE.{i}"), &[""]);
        builder.extra_name(format!("INT_NODE_IMUX_{i}_INT_OUT"), w);
    }

    builder.extract_nodes();

    builder.extract_term_conn("W", Dir::W, "INT_TERM_L_IO", &[]);
    builder.extract_term_conn("W", Dir::W, "INT_INT_INTERFACE_GT_LEFT_FT", &[]);
    builder.extract_term_conn("E", Dir::E, "INT_INTERFACE_GT_R", &[]);
    builder.extract_term_conn("S", Dir::S, "INT_TERM_B", &[]);
    builder.extract_term_conn("N", Dir::N, "INT_TERM_T", &[]);

    for (dir, tkn) in [
        (Dir::W, "INT_INTERFACE_L"),
        (Dir::E, "INT_INTERFACE_R"),
    ] {
        builder.extract_intf(format!("INTF.{dir}"), dir, tkn, format!("INTF.{dir}"), true);
    }

    for (dir, n, tkn) in [
        (Dir::W, "IO", "INT_INT_INTERFACE_XIPHY_FT"),
        (Dir::W, "PCIE", "INT_INTERFACE_PCIE_L"),
        (Dir::E, "PCIE", "INT_INTERFACE_PCIE_R"),
        (Dir::W, "GT", "INT_INT_INTERFACE_GT_LEFT_FT"),
        (Dir::E, "GT", "INT_INTERFACE_GT_R"),
    ] {
        builder.extract_intf(format!("INTF.{dir}.DELAY"), dir, tkn, format!("INTF.{dir}.{n}"), true);
    }

    for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = Coord {
                x: xy.x,
                y: xy.y + 1,
            };
            builder.extract_xnode("RCLK", xy, &[int_xy], "RCLK", &[]);
        }
    }

    builder.build()
}

fn make_int_db_up(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("ultrascaleplus", rd);
    builder.node_type("INT", "INT", "INT");

    let d2n = enum_map!(
        Dir::N => 0,
        Dir::S => 1,
        Dir::E => 2,
        Dir::W => 3,
    );

    let w = builder.wire("VCC", int::WireKind::Tie1, &["VCC_WIRE"]);
    builder.extra_name("VCC_WIRE", w);

    for i in 0..16 {
        builder.wire(format!("GCLK{i}"), int::WireKind::ClkOut(i), &[
            format!("GCLK_B_0_{i}"),
        ]);
    }

    for (ih, h) in ["E", "W"].into_iter().enumerate() {
        for i in 0..96 {
            match i {
                0 | 2 => {
                    let w = builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("SDQNODE_{h}_{i}_FT1")],
                    );
                    builder.branch(w, Dir::S,
                        format!("SDQNODE.{h}.{i}.S"),
                        &[format!("SDQNODE_{h}_BLS_{i}_FT0")],
                    );
                }
                91 | 93 | 95 => {
                    let w = builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("SDQNODE_{h}_{i}_FT0")],
                    );
                    builder.branch(w, Dir::N,
                        format!("SDQNODE.{h}.{i}.N"),
                        &[format!("SDQNODE_{h}_BLN_{i}_FT1")],
                    );
                }
                _ => {
                    // TODO not the true permutation
                    let a = [
                        0, 11, 22, 33, 44, 1, 2, 3,
                        4, 5, 6, 7, 8, 9, 10, 12,
                        13, 14, 15, 16, 17, 18, 19, 20,
                        21, 23, 24, 25, 26, 27, 28, 29,
                        30, 31, 32, 34, 35, 36, 37, 38,
                        39, 40, 41, 42, 43, 45, 46, 47,
                    ][i >> 1];
                    let aa = a + ih * 48;
                    let b = i & 1;
                    builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("INT_NODE_SDQ_{aa}_INT_OUT{b}")],
                    );
                }
            }
        }
    }
    for (dir, name, l, ll, fts, ftn) in [
        (Dir::E, "SNG", 1, 1, false, false),
        (Dir::W, "SNG", 1, 1, false, true),
        (Dir::N, "SNG", 1, 1, false, false),
        (Dir::S, "SNG", 1, 1, false, false),

        (Dir::E, "DBL", 1, 2, false, false),
        (Dir::W, "DBL", 1, 2, true, false),
        (Dir::N, "DBL", 2, 2, false, false),
        (Dir::S, "DBL", 2, 2, false, false),

        (Dir::E, "QUAD", 2, 4, false, false),
        (Dir::W, "QUAD", 2, 4, false, false),
        (Dir::N, "QUAD", 4, 4, false, true),
        (Dir::S, "QUAD", 4, 4, true, false),
    ] {
        let ftd = d2n[!dir];
        for ew in ['E', 'W'] {
            for i in 0..8 {
                match (ll, dir, ew) {
                    (1, Dir::E, 'W') => {
                        let (a, b) = [
                            (60, 1),
                            (4, 0),
                            (61, 1),
                            (5, 0),
                            (62, 1),
                            (6, 0),
                            (63, 1),
                            (7, 0),
                        ][i];
                        builder.mux_out(
                            format!("{name}.{dir}.{ew}.{i}.0"),
                            &[format!("INT_INT_SDQ_{a}_INT_OUT{b}")],
                        );
                    }
                    (1, Dir::W, 'E') => {
                        if i == 7 {
                            let w = builder.mux_out(
                                format!("{name}.{dir}.{ew}.{i}.0"),
                                &[format!("{dir}{dir}{ll}_{ew}_{i}_FT0")],
                            );
                            builder.branch(w, Dir::N,
                                format!("{name}.{dir}.{ew}.{i}.{l}.N"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLN_{i}_FT1")],
                            );
                        } else {
                            let (a, b) = [
                                (72, 0),
                                (32, 1),
                                (73, 0),
                                (33, 1),
                                (74, 0),
                                (34, 1),
                                (75, 0),
                            ][i];
                            builder.mux_out(
                                format!("{name}.{dir}.{ew}.{i}.0"),
                                &[format!("INT_INT_SDQ_{a}_INT_OUT{b}")],
                            );
                        }
                    }
                    _ => {
                        let mut w = builder.mux_out(
                            format!("{name}.{dir}.{ew}.{i}.0"),
                            &[format!("{dir}{dir}{ll}_{ew}_BEG{i}")],
                        );
                        for j in 1..l {
                            let nn = (b'A' + (j - 1)) as char;
                            w = builder.branch(w, dir,
                                format!("{name}.{dir}.{ew}.{i}.{j}"),
                                &[format!("{dir}{dir}{ll}_{ew}_{nn}_FT{ftd}_{i}")],
                            );
                        }
                        w = builder.branch(w, dir,
                            format!("{name}.{dir}.{ew}.{i}.{l}"),
                            &[format!("{dir}{dir}{ll}_{ew}_END{i}")],
                        );
                        if i == 0 && fts {
                            builder.branch(w, Dir::S,
                                format!("{name}.{dir}.{ew}.{i}.{l}.S"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLS_{i}_FT0")],
                            );
                        }
                        if i == 7 && ftn {
                            builder.branch(w, Dir::N,
                                format!("{name}.{dir}.{ew}.{i}.{l}.N"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLN_{i}_FT1")],
                            );
                        }
                    }
                }
            }
        }
    }

    for (dir, name, l, fts, ftn) in [
        (Dir::E, "LONG", 6, true, true),
        (Dir::W, "LONG", 6, false, false),
        (Dir::N, "LONG", 12, false, false),
        (Dir::S, "LONG", 12, false, false),
    ] {
        let ftd = d2n[!dir];
        for i in 0..8 {
            let mut w = builder.mux_out(
                format!("{name}.{dir}.{i}.0"),
                &[format!("{dir}{dir}12_BEG{i}")],
            );
            for j in 1..l {
                let nn = (b'A' + (j - 1)) as char;
                w = builder.branch(w, dir,
                    format!("{name}.{dir}.{i}.{j}"),
                    &[format!("{dir}{dir}12_{nn}_FT{ftd}_{i}")],
                );
            }
            w = builder.branch(w, dir,
                format!("{name}.{dir}.{i}.{l}"),
                &[format!("{dir}{dir}12_END{i}")],
            );
            if i == 0 && fts {
                builder.branch(w, Dir::S,
                    format!("{name}.{dir}.{i}.{l}.S"),
                    &[format!("{dir}{dir}12_BLS_{i}_FT0")],
                );
            }
            if i == 7 && ftn {
                builder.branch(w, Dir::N,
                    format!("{name}.{dir}.{i}.{l}.N"),
                    &[format!("{dir}{dir}12_BLN_{i}_FT1")],
                );
            }
        }
    }

    for i in 0..16 {
        for j in 0..2 {
            builder.mux_out(
                format!("INT_NODE_GLOBAL.{i}.{j}"),
                &[format!("INT_NODE_GLOBAL_{i}_INT_OUT{j}")],
            );
        }
    }
    for i in 0..8 {
        builder.mux_out(
            format!("IMUX.E.CTRL.{i}"),
            &[format!("CTRL_E{i}")],
        );
    }
    for i in 0..10 {
        builder.mux_out(
            format!("IMUX.W.CTRL.{i}"),
            &[format!("CTRL_W{i}")],
        );
    }

    for (ih, h) in ['E', 'W'].into_iter().enumerate() {
        for i in 0..64 {
            match i {
                1 | 3 | 5 | 9 => {
                    let w = builder.mux_out(
                        format!("INODE.{h}.{i}"),
                        &[format!("INODE_{h}_{i}_FT1")],
                    );
                    builder.branch(w, Dir::S,
                        format!("INODE.{h}.{i}.S"),
                        &[format!("INODE_{h}_BLS_{i}_FT0")],
                    );
                }
                54 | 58 | 60 | 62 => {
                    let w = builder.mux_out(
                        format!("INODE.{h}.{i}"),
                        &[format!("INODE_{h}_{i}_FT0")],
                    );
                    builder.branch(w, Dir::N,
                        format!("INODE.{h}.{i}.N"),
                        &[format!("INODE_{h}_BLN_{i}_FT1")],
                    );
                }
                _ => {
                    // TODO not the true permutation
                    let a = [
                        0, 11, 22, 30, 31, 1, 2, 3,
                        4, 5, 6, 7, 8, 9, 10, 12,
                        13, 14, 15, 16, 17, 18, 19, 20,
                        21, 23, 24, 25, 26, 27, 28, 29,
                    ][i >> 1];
                    let aa = a + ih * 32;
                    let b = i & 1;
                    builder.mux_out(
                        format!("INODE.{h}.{i}"),
                        &[format!("INT_NODE_IMUX_{aa}_INT_OUT{b}")],
                    );
                }
            }
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..16 {
            match i {
                0 | 2 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FT1")],
                    );
                    builder.branch(w, Dir::S,
                        format!("IMUX.{ew}.BYP.{i}.S"),
                        &[format!("BOUNCE_{ew}_BLS_{i}_FT0")],
                    );
                }
                13 | 15 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FT0")],
                    );
                    builder.branch(w, Dir::N,
                        format!("IMUX.{ew}.BYP.{i}.N"),
                        &[format!("BOUNCE_{ew}_BLN_{i}_FT1")],
                    );
                }
                _ => {
                    builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BYPASS_{ew}{i}")],
                    );
                }
            }
        }
    }
    for ew in ['E', 'W'] {
        for i in 0..48 {
            builder.mux_out(
                format!("IMUX.{ew}.IMUX.{i}"),
                &[format!("IMUX_{ew}{i}")],
            );
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..32 {
            builder.logic_out(
                format!("OUT.{ew}.{i}"),
                &[format!("LOGIC_OUTS_{ew}{i}")],
            );
        }
    }

    for i in 0..32 {
        let w = builder.mux_out(format!("RCLK.IMUX.CE.{i}"), &[""]);
        builder.extra_name(format!("CLK_LEAF_SITES_{i}_CE_INT"), w);
    }
    let w = builder.mux_out("RCLK.IMUX.ENSEL_PROG", &[""]);
    builder.extra_name("CLK_LEAF_SITES_0_ENSEL_PROG", w);
    let w = builder.mux_out("RCLK.IMUX.CLK_CASC_IN", &[""]);
    builder.extra_name("CLK_LEAF_SITES_0_CLK_CASC_IN", w);
    for i in 0..2 {
        for j in 0..4 {
            let w = builder.mux_out(format!("RCLK.IMUX.LEFT.{i}.{j}"), &[""]);
            builder.extra_name(format!("INT_RCLK_TO_CLK_LEFT_{i}_{j}"), w);
        }
    }
    for i in 0..2 {
        for j in 0..3 {
            let w = builder.mux_out(format!("RCLK.IMUX.RIGHT.{i}.{j}"), &[""]);
            builder.extra_name(format!("INT_RCLK_TO_CLK_RIGHT_{i}_{j}"), w);
        }
    }
    for i in 0..2 {
        for j in 0..24 {
            let w = builder.mux_out(format!("RCLK.INODE.{i}.{j}"), &[""]);
            builder.extra_name(format!("INT_NODE_IMUX_{j}_INT_OUT{i}"), w);
        }
    }
    for i in 0..48 {
        let w = builder.wire(format!("RCLK.GND.{i}"), int::WireKind::Tie0, &[""]);
        builder.extra_name_tile("RCLK_INT_L", format!("GND_WIRE{i}"), w);
        builder.extra_name_tile("RCLK_INT_R", format!("GND_WIRE{i}"), w);
    }
    builder.extract_nodes();

    builder.extract_term_conn("W", Dir::W, "INT_INTF_L_TERM_GT", &[]);
    builder.extract_term_conn("W", Dir::W, "INT_INTF_LEFT_TERM_PSS", &[]);
    builder.extract_term_conn("W", Dir::W, "INT_INTF_LEFT_TERM_IO_FT", &[]);
    builder.extract_term_conn("E", Dir::E, "INT_INTF_R_TERM_GT", &[]);
    builder.extract_term_conn("E", Dir::E, "INT_INTF_RIGHT_TERM_IO", &[]);
    builder.extract_term_conn("S", Dir::S, "INT_TERM_B", &[]);
    builder.extract_term_conn("S", Dir::S, "INT_TERM_P", &[]);
    builder.extract_term_conn("S", Dir::S, "INT_INT_TERM_H_FT", &[]);
    builder.extract_term_conn("N", Dir::N, "INT_TERM_T", &[]);

    for (dir, tkn) in [
        (Dir::W, "INT_INTF_L"),
        (Dir::E, "INT_INTF_R"),
    ] {
        builder.extract_intf(format!("INTF.{dir}"), dir, tkn, format!("INTF.{dir}"), true);
    }

    builder.extract_intf(format!("INTF.W.IO"), Dir::W, "INT_INTF_LEFT_TERM_PSS", "INTF.PSS", true);
    for (dir, tkn) in [
        (Dir::W, "INT_INTF_LEFT_TERM_IO_FT"),
        (Dir::W, "INT_INTF_L_CMT"),
        (Dir::W, "INT_INTF_L_IO"),
        (Dir::E, "INT_INTF_RIGHT_TERM_IO"),
    ] {
        builder.extract_intf(format!("INTF.{dir}.IO"), dir, tkn, format!("INTF.{dir}.IO"), true);
    }

    for (dir, n, tkn) in [
        (Dir::W, "PCIE", "INT_INTF_L_PCIE4"),
        (Dir::E, "PCIE", "INT_INTF_R_PCIE4"),
        (Dir::W, "GT", "INT_INTF_L_TERM_GT"),
        (Dir::E, "GT", "INT_INTF_R_TERM_GT"),
    ] {
        builder.extract_intf(format!("INTF.{dir}.DELAY"), dir, tkn, format!("INTF.{dir}.{n}"), true);
    }

    builder.extract_pass_simple("IO", Dir::W, "INT_IBRK_FSR2IO", &[]);

    for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = Coord {
                x: xy.x,
                y: xy.y + 1,
            };
            builder.extract_xnode("RCLK", xy, &[int_xy], "RCLK", &[]);
        }
    }

    builder.build()
}

fn make_grids(rd: &Part) -> (EntityVec<SlrId, ultrascale::Grid>, SlrId, BTreeSet<DisabledPart>) {
    let is_plus = rd.family == "ultrascaleplus";
    let mut rows_slr_split: BTreeSet<_> = find_rows(rd, &["INT_TERM_T"]).into_iter().map(|r| (r + 1) as u16).collect();
    rows_slr_split.insert(0);
    rows_slr_split.insert(rd.height);
    let rows_slr_split: Vec<_> = rows_slr_split.iter().collect();
    let kind = if is_plus { GridKind::UltrascalePlus } else { GridKind::Ultrascale };
    let mut grids = EntityVec::new();
    for w in rows_slr_split.windows(2) {
        let int = extract_int_slr(rd, &["INT"], &[], *w[0], *w[1]);
        let columns = make_columns(&int);
        let cols_vbrk = get_cols_vbrk(&int);
        let cols_fsr_gap = get_cols_fsr_gap(&int);
        let cols_hard = get_cols_hard(&int);
        let cols_io = get_cols_io(&int);
        let is_alt_cfg = is_plus && int.find_tiles(&["CFG_M12BUF_CTR_RIGHT_CFG_OLY_BOT_L_FT", "CFG_M12BUF_CTR_RIGHT_CFG_OLY_DK_BOT_L_FT"]).is_empty();

        let (col_hard, col_cfg) = match cols_hard.len() {
            1 => {
                let [col_cfg]: [_; 1] = cols_hard.try_into().unwrap();
                (None, col_cfg)
            }
            2 => {
                let [col_hard, col_cfg]: [_; 2] = cols_hard.try_into().unwrap();
                (Some(col_hard), col_cfg)
            }
            _ => unreachable!(),
        };
        assert_eq!(int.rows.len() % 60, 0);
        grids.push(ultrascale::Grid {
            kind,
            columns,
            cols_vbrk,
            cols_fsr_gap,
            col_cfg,
            col_hard,
            cols_io,
            regs: int.rows.len() / 60,
            ps: get_ps(&int),
            has_hbm: int.find_column(&["HBM_DMAH_FT"]).is_some(),
            is_dmc: int.find_column(&["FSR_DMC_TARGET_FT"]).is_some(),
            is_alt_cfg,
        });
    }
    let mut disabled = BTreeSet::new();
    let tterms = find_rows(rd, &["INT_TERM_T"]);
    if !tterms.contains(&(rd.height as i32 - 1)) {
        if rd.part.contains("ku025") {
            let s0 = SlrId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 3);
            assert_eq!(grids[s0].col_hard, None);
            assert_eq!(grids[s0].cols_io.len(), 3);
            grids[s0].regs = 5;
            grids[s0].col_cfg.regs.push(HardRowKind::Pcie);
            grids[s0].col_cfg.regs.push(HardRowKind::Pcie);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[2].regs.push(IoRowKind::Gth);
            grids[s0].cols_io[2].regs.push(IoRowKind::Gth);
            disabled.insert(DisabledPart::Region(s0, 3));
            disabled.insert(DisabledPart::Region(s0, 4));
        } else if rd.part.contains("ku085") {
            let s0 = SlrId::from_idx(0);
            let s1 = SlrId::from_idx(1);
            assert_eq!(grids.len(), 2);
            assert_eq!(grids[s0].regs, 5);
            assert_eq!(grids[s1].regs, 4);
            assert_eq!(grids[s1].col_hard, None);
            assert_eq!(grids[s1].cols_io.len(), 4);
            grids[s1].regs = 5;
            grids[s1].col_cfg.regs.push(HardRowKind::Pcie);
            grids[s1].cols_io[0].regs.push(IoRowKind::Gth);
            grids[s1].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s1].cols_io[2].regs.push(IoRowKind::Hpio);
            grids[s1].cols_io[3].regs.push(IoRowKind::Gth);
            assert_eq!(grids[s0], grids[s1]);
            disabled.insert(DisabledPart::Region(s1, 4));
        } else if rd.part.contains("zu25dr") {
            let s0 = SlrId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 6);
            assert_eq!(grids[s0].cols_io.len(), 3);
            grids[s0].regs = 8;
            grids[s0].col_cfg.regs.push(HardRowKind::Hdio);
            grids[s0].col_cfg.regs.push(HardRowKind::Hdio);
            grids[s0].col_hard.as_mut().unwrap().regs.push(HardRowKind::Cmac);
            grids[s0].col_hard.as_mut().unwrap().regs.push(HardRowKind::Pcie);
            grids[s0].cols_io[0].regs.push(IoRowKind::Gty);
            grids[s0].cols_io[0].regs.push(IoRowKind::Gty);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[2].regs.push(IoRowKind::HsDac);
            grids[s0].cols_io[2].regs.push(IoRowKind::HsDac);
            disabled.insert(DisabledPart::Region(s0, 6));
            disabled.insert(DisabledPart::Region(s0, 7));
        } else if rd.part.contains("ku19p") {
            let s0 = SlrId::from_idx(0);
            assert_eq!(grids.len(), 1);
            assert_eq!(grids[s0].regs, 9);
            assert_eq!(grids[s0].cols_io.len(), 2);
            assert_eq!(grids[s0].col_hard, None);
            grids[s0].regs = 11;
            grids[s0].col_cfg.regs.insert(0, HardRowKind::PciePlus);
            grids[s0].col_cfg.regs.push(HardRowKind::Cmac);
            grids[s0].cols_io[0].regs.insert(0, IoRowKind::Hpio);
            grids[s0].cols_io[0].regs.push(IoRowKind::Hpio);
            grids[s0].cols_io[1].regs.insert(0, IoRowKind::Gty);
            grids[s0].cols_io[1].regs.push(IoRowKind::Gtm);
            disabled.insert(DisabledPart::Region(s0, 0));
            disabled.insert(DisabledPart::Region(s0, 10));
        } else {
            println!("UNKNOWN CUT TOP {}", rd.part);
        }
    }
    let bterms = find_rows(rd, &["INT_TERM_B"]);
    if !bterms.contains(&0) && !grids.first().unwrap().has_hbm && grids.first().unwrap().ps.is_none() {
        if rd.part.contains("vu160") {
            let s0 = SlrId::from_idx(0);
            let s1 = SlrId::from_idx(1);
            let s2 = SlrId::from_idx(2);
            assert_eq!(grids.len(), 3);
            assert_eq!(grids[s0].regs, 4);
            assert_eq!(grids[s1].regs, 5);
            assert_eq!(grids[s2].regs, 5);
            assert_eq!(grids[s0].cols_io.len(), 4);
            grids[s0].regs = 5;
            grids[s0].col_cfg.regs.insert(0, HardRowKind::Pcie);
            grids[s0].col_hard.as_mut().unwrap().regs.insert(0, HardRowKind::Ilkn);
            grids[s0].cols_io[0].regs.insert(0, IoRowKind::Gty);
            grids[s0].cols_io[1].regs.insert(0, IoRowKind::Hpio);
            grids[s0].cols_io[2].regs.insert(0, IoRowKind::Hrio);
            grids[s0].cols_io[3].regs.insert(0, IoRowKind::Gth);
            assert_eq!(grids[s0], grids[s1]);
            disabled.insert(DisabledPart::Region(s0, 0));
        } else if rd.part.contains("ku19p") {
            // fixed above
        } else {
            println!("UNKNOWN CUT BOTTOM {}", rd.part);
        }
    }
    let mut grid_master = None;
    for (_, pins) in &rd.packages {
        for pin in pins {
            if pin.func == "VP" {
                if is_plus {
                    grid_master = Some(pin.pad.as_ref().unwrap().strip_prefix("SYSMONE4_X0Y").unwrap().parse().unwrap());
                } else {
                    grid_master = Some(pin.pad.as_ref().unwrap().strip_prefix("SYSMONE1_X0Y").unwrap().parse().unwrap());
                }
            }
        }
    }
    let grid_master = SlrId::from_idx(grid_master.unwrap());
    if grids.first().unwrap().ps.is_some() {
        let mut found = false;
        for pins in rd.packages.values() {
            for pin in pins {
                if pin.pad.as_ref().filter(|x| x.starts_with("PS8")).is_some() {
                    found = true;
                }
            }
        }
        if !found {
            disabled.insert(DisabledPart::Ps);
        }
    }
    (grids, grid_master, disabled)
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn lookup_nonpad_pin(rd: &Part, pin: &PkgPin) -> Option<BondPin> {
    match &pin.func[..] {
        "NC" => return Some(BondPin::Nc),
        "GND" => return Some(BondPin::Gnd),
        "VCCINT" => return Some(BondPin::VccInt),
        "VCCAUX" => return Some(BondPin::VccAux),
        "VCCAUX_HPIO" => return Some(BondPin::VccAuxHpio),
        "VCCAUX_HDIO" => return Some(BondPin::VccAuxHdio),
        "VCCBRAM" => return Some(BondPin::VccBram),
        "VCCINT_IO" => return Some(BondPin::VccIntIo),
        "VCCAUX_IO" => return Some(BondPin::VccAuxIo(0)),
        "VBATT" => return Some(BondPin::VccBatt),
        "D00_MOSI_0" => return Some(BondPin::Cfg(CfgPin::Data(0))),
        "D01_DIN_0" => return Some(BondPin::Cfg(CfgPin::Data(1))),
        "D02_0" => return Some(BondPin::Cfg(CfgPin::Data(2))),
        "D03_0" => return Some(BondPin::Cfg(CfgPin::Data(3))),
        "RDWR_FCS_B_0" => return Some(BondPin::Cfg(CfgPin::RdWrB)),
        "TCK_0" => return Some(BondPin::Cfg(CfgPin::Tck)),
        "TDI_0" => return Some(BondPin::Cfg(CfgPin::Tdi)),
        "TDO_0" => return Some(BondPin::Cfg(CfgPin::Tdo)),
        "TMS_0" => return Some(BondPin::Cfg(CfgPin::Tms)),
        "CCLK_0" => return Some(BondPin::Cfg(CfgPin::Cclk)),
        "PUDC_B_0" | "PUDC_B" => return Some(BondPin::Cfg(CfgPin::HswapEn)),
        "POR_OVERRIDE" => return Some(BondPin::Cfg(CfgPin::PorOverride)),
        "DONE_0" => return Some(BondPin::Cfg(CfgPin::Done)),
        "PROGRAM_B_0" => return Some(BondPin::Cfg(CfgPin::ProgB)),
        "INIT_B_0" => return Some(BondPin::Cfg(CfgPin::InitB)),
        "M0_0" => return Some(BondPin::Cfg(CfgPin::M0)),
        "M1_0" => return Some(BondPin::Cfg(CfgPin::M1)),
        "M2_0" => return Some(BondPin::Cfg(CfgPin::M2)),
        "CFGBVS_0" => return Some(BondPin::Cfg(CfgPin::CfgBvs)),
        "DXN" => return Some(BondPin::Dxn),
        "DXP" => return Some(BondPin::Dxp),
        "GNDADC" => return Some(BondPin::SysMonByBank(0, SysMonPin::AVss)),
        "VCCADC" => return Some(BondPin::SysMonByBank(0, SysMonPin::AVdd)),
        "VREFP" => return Some(BondPin::SysMonByBank(0, SysMonPin::VRefP)),
        "VREFN" => return Some(BondPin::SysMonByBank(0, SysMonPin::VRefN)),
        "GND_PSADC" => return Some(BondPin::SysMonByBank(1, SysMonPin::AVss)),
        "VCC_PSADC" => return Some(BondPin::SysMonByBank(1, SysMonPin::AVdd)),
        "GND_SENSE" => return Some(BondPin::GndSense),
        "VCCINT_SENSE" => return Some(BondPin::VccIntSense),
        "VCCO_PSIO0_500" => return Some(BondPin::VccO(500)),
        "VCCO_PSIO1_501" => return Some(BondPin::VccO(501)),
        "VCCO_PSIO2_502" => return Some(BondPin::VccO(502)),
        "VCCO_PSIO3_503" => return Some(BondPin::VccO(503)),
        "VCCO_PSDDR_504" => return Some(BondPin::VccO(504)),
        "VCC_PSAUX" => return Some(BondPin::VccPsAux),
        "VCC_PSINTLP" => return Some(BondPin::VccPsIntLp),
        "VCC_PSINTFP" => return Some(BondPin::VccPsIntFp),
        "VCC_PSINTFP_DDR" => return Some(BondPin::VccPsIntFpDdr),
        "VCC_PSPLL" => return Some(BondPin::VccPsPll),
        "VCC_PSDDR_PLL" => return Some(BondPin::VccPsDdrPll),
        "VCC_PSBATT" => return Some(BondPin::VccPsBatt),
        "VCCINT_VCU" => return Some(BondPin::VccIntVcu),
        "PS_MGTRAVCC" => return Some(BondPin::GtByBank(505, GtPin::AVcc, 0)),
        "PS_MGTRAVTT" => return Some(BondPin::GtByBank(505, GtPin::AVtt, 0)),
        "VCCSDFEC" => return Some(BondPin::VccSdfec),
        "VCCINT_AMS" => return Some(BondPin::VccIntAms),
        "DAC_GND" => return Some(BondPin::DacGnd),
        "DAC_SUB_GND" => return Some(BondPin::DacSubGnd),
        "DAC_AVCC" => return Some(BondPin::DacAVcc),
        "DAC_AVCCAUX" => return Some(BondPin::DacAVccAux),
        "DAC_AVTT" => return Some(BondPin::DacAVtt),
        "ADC_GND" => return Some(BondPin::DacGnd),
        "ADC_SUB_GND" => return Some(BondPin::DacSubGnd),
        "ADC_AVCC" => return Some(BondPin::DacAVcc),
        "ADC_AVCCAUX" => return Some(BondPin::DacAVccAux),
        "RSVD" => if let Some(bank) = pin.vcco_bank {
            return Some(BondPin::Hbm(bank, HbmPin::Rsvd))
        } else {
            // disabled DACs
            if rd.part.contains("zu25dr") {
                return Some(BondPin::Rsvd)
            }
        }
        "RSVDGND" => if let Some(bank) = pin.vcco_bank {
            if bank == 0 {
                return Some(BondPin::Cfg(CfgPin::CfgBvs))
            } else {
                return Some(BondPin::Hbm(bank, HbmPin::RsvdGnd))
            }
        } else {
            for p in ["zu2cg", "zu2eg", "zu3cg", "zu3eg", "zu4cg", "zu4eg", "zu5cg", "zu5eg", "zu7cg", "zu7eg"] {
                if rd.part.contains(p) {
                    return Some(BondPin::VccIntVcu)
                }
            }
            // disabled DACs
            if rd.part.contains("zu25dr") {
                return Some(BondPin::RsvdGnd)
            }
            // disabled GT VCCINT
            if rd.part.contains("ku19p") {
                return Some(BondPin::RsvdGnd)
            }
        }
        _ => (),
    }
    if let Some(b) = pin.func.strip_prefix("VCCO_") {
        return Some(BondPin::VccO(b.parse().ok()?))
    }
    if let Some(b) = pin.func.strip_prefix("VREF_") {
        return Some(BondPin::IoVref(b.parse().ok()?, 0))
    }
    if let Some(b) = pin.func.strip_prefix("VCC_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::Vcc))
    }
    if let Some(b) = pin.func.strip_prefix("VCCAUX_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::VccAux))
    }
    if let Some(b) = pin.func.strip_prefix("VCC_IO_HBM_") {
        return Some(BondPin::Hbm(b.parse().ok()?, HbmPin::VccIo))
    }
    if let Some(b) = pin.func.strip_prefix("VCM01_") {
        return Some(BondPin::AdcByBank(b.parse().ok()?, AdcPin::VCm, 0))
    }
    if let Some(b) = pin.func.strip_prefix("VCM23_") {
        return Some(BondPin::AdcByBank(b.parse().ok()?, AdcPin::VCm, 2))
    }
    if let Some(b) = pin.func.strip_prefix("ADC_REXT_") {
        return Some(BondPin::AdcByBank(b.parse().ok()?, AdcPin::RExt, 0))
    }
    if let Some(b) = pin.func.strip_prefix("DAC_REXT_") {
        return Some(BondPin::DacByBank(b.parse().ok()?, DacPin::RExt, 0))
    }
    for (suf, region) in [
        ("", 0),
        ("_L", 2),
        ("_R", 3),
        ("_LS", 4),
        ("_RS", 5),
        ("_LLC", 6),
        ("_RLC", 7),
        ("_LC", 8),
        ("_RC", 9),
        ("_LUC", 10),
        ("_RUC", 11),
        ("_LN", 12),
        ("_RN", 13),
    ] {
        if let Some(f) = pin.func.strip_suffix(suf) {
            match f {
                "MGTAVTT" => return Some(BondPin::GtByRegion(region, GtRegionPin::AVtt)),
                "MGTAVCC" => return Some(BondPin::GtByRegion(region, GtRegionPin::AVcc)),
                "MGTVCCAUX" => return Some(BondPin::GtByRegion(region, GtRegionPin::VccAux)),
                "MGTRREF" => return Some(BondPin::GtByBank(pin.vcco_bank.unwrap(), GtPin::RRef, 0)),
                "MGTAVTTRCAL" => return Some(BondPin::GtByBank(pin.vcco_bank.unwrap(), GtPin::AVttRCal, 0)),
                "VCCINT_GT" => return Some(BondPin::GtByRegion(region, GtRegionPin::VccInt)),
                _ => (),
            }
        }
    }
    None
}

fn lookup_gt_pin(gt_lookup: &HashMap<(IoRowKind, u32, u32), Gt>, pad: &str, func: &str) -> Option<BondPin> {
    if let Some(p) = pad.strip_prefix("HSADC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::HsAdc, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "ADC_VIN0_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 0)),
            "ADC_VIN0_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 0)),
            "ADC_VIN1_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 1)),
            "ADC_VIN1_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 1)),
            "ADC_VIN2_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 2)),
            "ADC_VIN2_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 2)),
            "ADC_VIN3_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 3)),
            "ADC_VIN3_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 3)),
            "ADC_VIN_I01_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairP, 0)),
            "ADC_VIN_I01_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairN, 0)),
            "ADC_VIN_I23_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairP, 2)),
            "ADC_VIN_I23_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairN, 2)),
            "ADC_CLK_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::ClkP, 0)),
            "ADC_CLK_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::ClkN, 0)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("RFADC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::RfAdc, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "ADC_VIN0_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 0)),
            "ADC_VIN0_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 0)),
            "ADC_VIN1_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 1)),
            "ADC_VIN1_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 1)),
            "ADC_VIN2_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 2)),
            "ADC_VIN2_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 2)),
            "ADC_VIN3_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInP, 3)),
            "ADC_VIN3_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInN, 3)),
            "ADC_VIN_I01_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairP, 0)),
            "ADC_VIN_I01_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairN, 0)),
            "ADC_VIN_I23_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairP, 2)),
            "ADC_VIN_I23_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::VInPairN, 2)),
            "ADC_CLK_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::ClkP, 0)),
            "ADC_CLK_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::ClkN, 0)),
            "ADC_PLL_TEST_OUT_P" => Some(BondPin::AdcByBank(gt.bank, AdcPin::PllTestOutP, 0)),
            "ADC_PLL_TEST_OUT_N" => Some(BondPin::AdcByBank(gt.bank, AdcPin::PllTestOutN, 0)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("HSDAC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::HsDac, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "DAC_VOUT0_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 0)),
            "DAC_VOUT0_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 0)),
            "DAC_VOUT1_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 1)),
            "DAC_VOUT1_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 1)),
            "DAC_VOUT2_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 2)),
            "DAC_VOUT2_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 2)),
            "DAC_VOUT3_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 3)),
            "DAC_VOUT3_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 3)),
            "DAC_CLK_P" => Some(BondPin::DacByBank(gt.bank, DacPin::ClkP, 0)),
            "DAC_CLK_N" => Some(BondPin::DacByBank(gt.bank, DacPin::ClkN, 0)),
            "SYSREF_P" => Some(BondPin::DacByBank(gt.bank, DacPin::SysRefP, 0)),
            "SYSREF_N" => Some(BondPin::DacByBank(gt.bank, DacPin::SysRefN, 0)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("RFDAC_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::RfDac, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "DAC_VOUT0_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 0)),
            "DAC_VOUT0_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 0)),
            "DAC_VOUT1_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 1)),
            "DAC_VOUT1_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 1)),
            "DAC_VOUT2_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 2)),
            "DAC_VOUT2_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 2)),
            "DAC_VOUT3_P" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutP, 3)),
            "DAC_VOUT3_N" => Some(BondPin::DacByBank(gt.bank, DacPin::VOutN, 3)),
            "DAC_CLK_P" => Some(BondPin::DacByBank(gt.bank, DacPin::ClkP, 0)),
            "DAC_CLK_N" => Some(BondPin::DacByBank(gt.bank, DacPin::ClkN, 0)),
            "SYSREF_P" => Some(BondPin::DacByBank(gt.bank, DacPin::SysRefP, 0)),
            "SYSREF_N" => Some(BondPin::DacByBank(gt.bank, DacPin::SysRefN, 0)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("GTM_DUAL_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::Gtm, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "MGTMRXP0" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, 0)),
            "MGTMRXN0" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, 0)),
            "MGTMTXP0" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, 0)),
            "MGTMTXN0" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, 0)),
            "MGTMRXP1" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, 1)),
            "MGTMRXN1" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, 1)),
            "MGTMTXP1" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, 1)),
            "MGTMTXN1" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, 1)),
            _ => None,
        }
    } else if let Some(p) = pad.strip_prefix("GTM_REFCLK_X") {
        let py = p.find('Y')?;
        let gx: u32 = p[..py].parse().ok()?;
        let gy: u32 = p[py+1..].parse().ok()?;
        let gt = gt_lookup.get(&(IoRowKind::Gtm, gx, gy))?;
        let suf = format!("_{}", gt.bank);
        let f = func.strip_suffix(&suf)?;
        match f {
            "MGTREFCLKP" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkP, 0)),
            "MGTREFCLKN" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkN, 0)),
            _ => None,
        }
    } else {
        let p;
        let kind;
        if let Some(x) = pad.strip_prefix("GTHE3_") {
            p = x;
            kind = IoRowKind::Gth;
        } else if let Some(x) = pad.strip_prefix("GTHE4_") {
            p = x;
            kind = IoRowKind::Gth;
        } else if let Some(x) = pad.strip_prefix("GTYE3_") {
            p = x;
            kind = IoRowKind::Gty;
        } else if let Some(x) = pad.strip_prefix("GTYE4_") {
            p = x;
            kind = IoRowKind::Gty;
        } else if let Some(x) = pad.strip_prefix("GTF_") {
            p = x;
            kind = IoRowKind::Gtf;
        } else {
            return None
        }
        if let Some(p) = p.strip_prefix("COMMON_X") {
            let py = p.find('Y')?;
            let gx: u32 = p[..py].parse().ok()?;
            let gy: u32 = p[py+1..].parse().ok()?;
            let gt = gt_lookup.get(&(kind, gx, gy))?;
            let suf = format!("_{}", gt.bank);
            let f = func.strip_suffix(&suf)?;
            match f {
                "MGTREFCLK0P" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkP, 0)),
                "MGTREFCLK0N" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkN, 0)),
                "MGTREFCLK1P" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkP, 1)),
                "MGTREFCLK1N" => Some(BondPin::GtByBank(gt.bank, GtPin::ClkN, 1)),
                _ => None,
            }
        } else if let Some(p) = p.strip_prefix("CHANNEL_X") {
            let py = p.find('Y')?;
            let gx: u32 = p[..py].parse().ok()?;
            let y: u32 = p[py+1..].parse().ok()?;
            let bel = y % 4;
            let gy = y / 4;
            let gt = gt_lookup.get(&(kind, gx, gy))?;
            let suf = format!("{}_{}", bel, gt.bank);
            let f = func.strip_suffix(&suf)?;
            match f {
                "MGTHRXP" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, bel)),
                "MGTHRXN" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, bel)),
                "MGTHTXP" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, bel)),
                "MGTHTXN" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, bel)),
                "MGTYRXP" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, bel)),
                "MGTYRXN" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, bel)),
                "MGTYTXP" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, bel)),
                "MGTYTXN" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, bel)),
                "MGTFRXP" => Some(BondPin::GtByBank(gt.bank, GtPin::RxP, bel)),
                "MGTFRXN" => Some(BondPin::GtByBank(gt.bank, GtPin::RxN, bel)),
                "MGTFTXP" => Some(BondPin::GtByBank(gt.bank, GtPin::TxP, bel)),
                "MGTFTXN" => Some(BondPin::GtByBank(gt.bank, GtPin::TxN, bel)),
                _ => None,
            }
        } else {
            None
        }
    }
}

fn make_bond(rd: &Part, pkg: &str, grids: &EntityVec<SlrId, ultrascale::Grid>, grid_master: SlrId, disabled: &BTreeSet<DisabledPart>, pins: &[PkgPin]) -> Bond {
    let kind = grids[grid_master].kind;
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = ultrascale::get_io(grids, grid_master, disabled)
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = ultrascale::get_gt(grids, grid_master, disabled)
        .into_iter()
        .map(|gt| ((gt.kind, gt.gx, gt.gy), gt))
        .collect();
    let is_zynq = grids[grid_master].ps.is_some() && !disabled.contains(&DisabledPart::Ps);
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                if pin.vcco_bank.unwrap() != io.bank {
                    if pin.vcco_bank != Some(64) && !matches!(io.bank, 84 | 94) {
                        println!("wrong bank pad {pkg} {pad} {io:?} got {f} exp {b}", f=pin.func, b=io.bank);
                    }
                }
                let old = io_banks.insert(io.bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                let mut exp_func = format!("IO");
                if io.kind == IoKind::Hdio {
                    write!(exp_func, "_L{}{}", 1 + io.bel / 2, ['P', 'N'][io.bel as usize % 2]).unwrap();
                } else {
                    let group = io.bel / 13;
                    if io.bel % 13 != 12 {
                        write!(exp_func, "_L{}{}", 1 + group * 6 + io.bel % 13 / 2, ['P', 'N'][io.bel as usize % 13 % 2]).unwrap();
                    }
                    write!(exp_func, "_T{}{}_N{}", group, if io.bel % 13 < 6 {'L'} else {'U'}, io.bel % 13).unwrap();
                }
                if io.is_gc() {
                    if io.kind == IoKind::Hdio {
                        exp_func += "_HDGC";
                    } else {
                        exp_func += "_GC";
                    }
                }
                if io.is_dbc() {
                    exp_func += "_DBC";
                }
                if io.is_qbc() {
                    exp_func += "_QBC";
                }
                if io.is_vrp() {
                    exp_func += "_VRP";
                }
                if let Some(sm) = io.sm_pair() {
                    if io.kind == IoKind::Hdio {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.bel as usize % 2]).unwrap();
                    } else {
                        write!(exp_func, "_AD{}{}", sm, ['P', 'N'][io.bel as usize % 13 % 2]).unwrap();
                    }
                }
                match io.get_cfg() {
                    Some(CfgPin::Data(d)) => if !is_zynq {
                        if d >= 16 {
                            write!(exp_func, "_A{:02}", d - 16).unwrap();
                        }
                        write!(exp_func, "_D{d:02}").unwrap();
                    }
                    Some(CfgPin::Addr(a)) => if !is_zynq {
                        write!(exp_func, "_A{a}").unwrap();
                    }
                    Some(CfgPin::Rs(a)) => if !is_zynq {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(CfgPin::UserCclk) => if !is_zynq {exp_func += "_EMCCLK"},
                    Some(CfgPin::Dout) => if !is_zynq {exp_func += "_DOUT_CSO_B"},
                    Some(CfgPin::FweB) => if !is_zynq {exp_func += "_FWE_FCS2_B"},
                    Some(CfgPin::FoeB) => if !is_zynq {exp_func += "_FOE_B"},
                    Some(CfgPin::CsiB) => if !is_zynq {exp_func += "_CSI_ADV_B"},
                    Some(CfgPin::PerstN0) => exp_func += "_PERSTN0",
                    Some(CfgPin::PerstN1) => exp_func += "_PERSTN1",
                    Some(CfgPin::SmbAlert) => exp_func += "_SMBALERT",
                    Some(CfgPin::I2cSclk) => exp_func += "_I2C_SCLK",
                    Some(CfgPin::I2cSda) => exp_func += if kind == GridKind::Ultrascale {"_I2C_SDA"} else {"_PERSTN1_I2C_SDA"},
                    None => (),
                    _ => unreachable!(),
                }
                write!(exp_func, "_{}", io_banks[&io.bank]).unwrap();
                if exp_func != pin.func {
                    println!("pad {pkg} {pad} {io:?} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::IoByBank(io.bank, io.bel)
            } else if pad.starts_with("GT") || pad.starts_with("RF") || pad.starts_with("HS") {
                if let Some(pin) = lookup_gt_pin(&gt_lookup, pad, &pin.func) {
                    pin
                } else {
                    println!("weird gt iopad {pkg} {p} {pad} {f}", f=pin.func, p=rd.part);
                    continue
                }
            } else if pad.starts_with("SYSMON") {
                let exp_site = match kind {
                    GridKind::Ultrascale => format!("SYSMONE1_X0Y{}", grid_master.to_idx()),
                    GridKind::UltrascalePlus => format!("SYSMONE4_X0Y{}", grid_master.to_idx()),
                };
                if exp_site != *pad {
                    println!("weird sysmon iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                }
                match &pin.func[..] {
                    "VP" => BondPin::SysMonByBank(grid_master.to_idx() as u32, SysMonPin::VP),
                    "VN" => BondPin::SysMonByBank(grid_master.to_idx() as u32, SysMonPin::VN),
                    _ => {
                        println!("weird sysmon iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                        continue
                    }
                }
            } else if pad == "PS8_X0Y0" {
                let pos = pin.func.rfind('_').unwrap();
                let bank: u32 = pin.func[pos+1..].parse().unwrap();
                if bank == 505 {
                    let (gtpin, bel) = match &pin.func[..pos] {
                        "PS_MGTRREF" => (GtPin::RRef, 0),
                        "PS_MGTREFCLK0P" => (GtPin::ClkP, 0),
                        "PS_MGTREFCLK0N" => (GtPin::ClkN, 0),
                        "PS_MGTREFCLK1P" => (GtPin::ClkP, 1),
                        "PS_MGTREFCLK1N" => (GtPin::ClkN, 1),
                        "PS_MGTREFCLK2P" => (GtPin::ClkP, 2),
                        "PS_MGTREFCLK2N" => (GtPin::ClkN, 2),
                        "PS_MGTREFCLK3P" => (GtPin::ClkP, 3),
                        "PS_MGTREFCLK3N" => (GtPin::ClkN, 3),
                        x => if let Some((n, b)) = split_num(x) {
                            match n {
                                "PS_MGTRTXP" => (GtPin::TxP, b),
                                "PS_MGTRTXN" => (GtPin::TxN, b),
                                "PS_MGTRRXP" => (GtPin::RxP, b),
                                "PS_MGTRRXN" => (GtPin::RxN, b),
                                _ => {
                                    println!("weird ps8 iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                                    continue;
                                }
                            }
                        } else {
                            println!("weird ps8 iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                            continue;
                        }
                    };
                    BondPin::GtByBank(bank, gtpin, bel)
                } else {
                    let pspin = match &pin.func[..pos] {
                        "PS_DONE" => PsPin::Done,
                        "PS_PROG_B" => PsPin::ProgB,
                        "PS_INIT_B" => PsPin::InitB,
                        "PS_ERROR_OUT" => PsPin::ErrorOut,
                        "PS_ERROR_STATUS" => PsPin::ErrorStatus,
                        "PS_PADI" => PsPin::PadI,
                        "PS_PADO" => PsPin::PadO,
                        "PS_POR_B" => PsPin::PorB,
                        "PS_SRST_B" => PsPin::SrstB,
                        "PS_REF_CLK" => PsPin::Clk,
                        "PS_JTAG_TDO" => PsPin::JtagTdo,
                        "PS_JTAG_TDI" => PsPin::JtagTdi,
                        "PS_JTAG_TCK" => PsPin::JtagTck,
                        "PS_JTAG_TMS" => PsPin::JtagTms,
                        "PS_DDR_ACT_N" => PsPin::DdrActN,
                        "PS_DDR_ALERT_N" => PsPin::DdrAlertN,
                        "PS_DDR_PARITY" => PsPin::DdrParity,
                        "PS_DDR_RAM_RST_N" => PsPin::DdrDrstB,
                        "PS_DDR_ZQ" => PsPin::DdrZq,
                        x => if let Some((n, b)) = split_num(x) {
                            match n {
                                "PS_MIO" => PsPin::Mio(b),
                                "PS_MODE" => PsPin::Mode(b),
                                "PS_DDR_DQ" => PsPin::DdrDq(b),
                                "PS_DDR_DM" => PsPin::DdrDm(b),
                                "PS_DDR_DQS_P" => PsPin::DdrDqsP(b),
                                "PS_DDR_DQS_N" => PsPin::DdrDqsN(b),
                                "PS_DDR_A" => PsPin::DdrA(b),
                                "PS_DDR_BA" => PsPin::DdrBa(b),
                                "PS_DDR_BG" => PsPin::DdrBg(b),
                                "PS_DDR_CKE" => PsPin::DdrCke(b),
                                "PS_DDR_ODT" => PsPin::DdrOdt(b),
                                "PS_DDR_CS_N" => PsPin::DdrCsB(b),
                                "PS_DDR_CK" => PsPin::DdrCkP(b),
                                "PS_DDR_CK_N" => PsPin::DdrCkN(b),
                                _ => {
                                    println!("weird ps8 iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                                    continue;
                                }
                            }
                        } else {
                            println!("weird ps8 iopad {p} {pad} {f}", f=pin.func, p=rd.part);
                            continue;
                        }
                    };
                    BondPin::IoPs(bank, pspin)
                }
            } else {
                println!("unk iopad {pad} {f}", f=pin.func);
                continue;
            }
        } else {
            if let Some(p) = lookup_nonpad_pin(rd, pin) {
                p
            } else {
                println!("UNK FUNC {} {} {:?}", pkg, pin.func, pin);
                continue;
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
    let (grids, grid_master, disabled) = make_grids(rd);
    let int_db = if rd.family == "ultrascale" {
        make_int_db_u(rd)
    } else {
        make_int_db_up(rd)
    };
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(rd, pkg, &grids, grid_master, &disabled, pins),
        ));
    }
    let grid_refs = grids.map_values(|x| x);
    let eint = expand_grid(&grid_refs, grid_master, &disabled, &int_db);
    let mut vrf = Verifier::new(rd, &eint);
    vrf.finish();
    let grids = grids.into_map_values(|x| geom::Grid::Ultrascale(x));
    (make_device_multi(rd, grids, grid_master, Vec::new(), bonds, disabled), Some(int_db))
}
