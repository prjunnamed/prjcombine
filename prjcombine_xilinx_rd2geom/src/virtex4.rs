use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use prjcombine_xilinx_rawdump::{Part, PkgPin, Coord};
use prjcombine_xilinx_geom::{self as geom, CfgPin, Bond, BondPin, GtPin, SysMonPin, ColId, RowId, int, int::Dir};
use prjcombine_xilinx_geom::virtex4::{self, ColumnKind};
use prjcombine_entity::{EntityVec, EntityId};

use crate::grid::{extract_int, find_columns, find_rows, find_row, IntGrid, PreDevice, make_device};
use crate::intb::IntBuilder;
use crate::verify::Verifier;

fn make_columns(rd: &Part, int: &IntGrid) -> EntityVec<ColId, ColumnKind> {
    let mut res: EntityVec<ColId, Option<ColumnKind>> = int.cols.map_values(|_| None);
    for c in find_columns(rd, &["CLB"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Clb);
    }
    for c in find_columns(rd, &["BRAM"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["IOIS_LC", "IOIS_LC_L"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["MGT_AR"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::Gt);
    }
    for c in find_columns(rd, &["MGT_AL"]) {
        res[int.lookup_column(c + 1)] = Some(ColumnKind::Gt);
    }
    res.map_values(|x| x.unwrap())
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["CFG_VBRK_FRAME"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_io(columns: &EntityVec<ColId, ColumnKind>) -> [ColId; 3] {
    let v: Vec<_> = columns.iter().filter_map(|(k, &v)| if v == ColumnKind::Io {Some(k)} else {None}).collect();
    v.try_into().unwrap()
}

fn get_reg_cfg(rd: &Part, int: &IntGrid) -> usize {
    int.lookup_row_inter(find_row(rd, &["CFG_CENTER"]).unwrap()).to_idx() / 16
}

fn get_regs_cfg_io(rd: &Part, int: &IntGrid, reg_cfg: usize) -> usize {
    let d2i = int.lookup_row_inter(find_row(rd, &["HCLK_DCMIOB"]).unwrap()).to_idx();
    let i2d = int.lookup_row_inter(find_row(rd, &["HCLK_IOBDCM"]).unwrap()).to_idx();
    assert_eq!(i2d - reg_cfg * 16, reg_cfg * 16 - d2i);
    (i2d - reg_cfg * 16 - 8) / 16
}

fn get_ccm(rd: &Part) -> usize {
    find_rows(rd, &["CCM"]).len() / 2
}

fn get_has_sysmons(rd: &Part) -> (bool, bool) {
    let sysmons = find_rows(rd, &["SYS_MON"]);
    (sysmons.contains(&1), sysmons.contains(&((rd.height - 9) as i32)))
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(ColId, RowId)> {
    let mut res = Vec::new();
    if let Some(tk) = rd.tile_kinds.get("PB") {
        for tile in &tk.tiles {
            let x = int.lookup_column((tile.x - 1) as i32);
            let y = int.lookup_row((tile.y - 4) as i32);
            assert_eq!(y.to_idx() % 16, 12);
            res.push((x, y));
        }
    }
    res
}

fn get_has_bram_fx(rd: &Part) -> bool {
    !find_columns(rd, &["HCLK_BRAM_FX"]).is_empty()
}

fn make_int_db(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("virtex4", rd);
    builder.node_type("INT", "INT", "NODE.INT");
    builder.node_type("INT_SO", "INT", "NODE.INT");
    builder.node_type("INT_SO_DCM0", "INT", "NODE.INT.DCM0");

    builder.wire("PULLUP", int::WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", int::WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", int::WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..8 {
        builder.wire(format!("GCLK{i}"), int::WireKind::ClkOut(i), &[
            format!("GCLK{i}"),
        ]);
    }
    for i in 0..2 {
        builder.wire(format!("RCLK{i}"), int::WireKind::ClkOut(8+i), &[
            format!("RCLK{i}"),
        ]);
    }

    for (i, da1, da2, db) in [
        (0, Dir::S, None, None),
        (1, Dir::W, Some(Dir::S), None),
        (2, Dir::E, None, Some(Dir::S)),
        (3, Dir::S, Some(Dir::E), None),
        (4, Dir::S, None, None),
        (5, Dir::S, Some(Dir::W), None),
        (6, Dir::W, None, None),
        (7, Dir::E, Some(Dir::S), None),
        (8, Dir::E, Some(Dir::N), None),
        (9, Dir::W, None, None),
        (10, Dir::N, Some(Dir::W), None),
        (11, Dir::N, None, None),
        (12, Dir::N, Some(Dir::E), None),
        (13, Dir::E, None, Some(Dir::N)),
        (14, Dir::W, Some(Dir::N), None),
        (15, Dir::N, None, None),
    ] {
        let omux = builder.mux_out(format!("OMUX{i}"), &[
            format!("OMUX{i}"),
        ]);
        let omux_da1 = builder.branch(omux, da1, format!("OMUX{i}.{da1}"), &[
            format!("OMUX_{da1}{i}"),
        ]);
        if let Some(da2) = da2 {
            builder.branch(omux_da1, da2, format!("OMUX{i}.{da1}{da2}"), &[
                format!("OMUX_{da1}{da2}{i}"),
            ]);
        }
        if let Some(db) = db {
            builder.branch(omux, db, format!("OMUX{i}.{db}"), &[
                format!("OMUX_{db}{i}"),
            ]);
        }
        if i == 0 {
            builder.branch(omux, Dir::S, format!("OMUX0.S.ALT"), &["OUT_S"]);
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let beg = builder.mux_out(format!("DBL.{dir}{i}.0"), &[
                format!("{dir}2BEG{i}"),
            ]);
            let mid = builder.branch(beg, dir, format!("DBL.{dir}{i}.1"), &[
                format!("{dir}2MID{i}"),
            ]);
            let end = builder.branch(mid, dir, format!("DBL.{dir}{i}.2"), &[
                format!("{dir}2END{i}"),
            ]);
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(end, Dir::S, format!("DBL.{dir}{i}.3"), &[
                    format!("{dir}2END_S{i}"),
                ]);
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(end, Dir::N, format!("DBL.{dir}{i}.3"), &[
                    format!("{dir}2END_N{i}"),
                ]);
            }
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let mut last = builder.mux_out(format!("HEX.{dir}{i}.0"), &[
                format!("{dir}6BEG{i}"),
            ]);
            for (j, seg) in [
                (1, "A"),
                (2, "B"),
                (3, "MID"),
                (4, "C"),
                (5, "D"),
                (6, "END"),
            ] {
                last = builder.branch(last, dir, format!("HEX.{dir}{i}.{j}"), &[
                    format!("{dir}6{seg}{i}"),
                ]);
            }
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(last, Dir::S, format!("HEX.{dir}{i}.7"), &[
                    format!("{dir}6END_S{i}"),
                ]);
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(last, Dir::N, format!("HEX.{dir}{i}.7"), &[
                    format!("{dir}6END_N{i}"),
                ]);
            }
        }
    }

    // The long wires.
    let mid = builder.wire("LH.12", int::WireKind::MultiOut, &["LH12"]);
    let mut prev = mid;
    for i in (0..12).rev() {
        prev = builder.multi_branch(prev, Dir::E, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mut prev = mid;
    for i in 13..25 {
        prev = builder.multi_branch(prev, Dir::W, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mid = builder.wire("LV.12", int::WireKind::MultiOut, &["LV12"]);
    let mut prev = mid;
    for i in (0..12).rev() {
        prev = builder.multi_branch(prev, Dir::N, format!("LV.{i}"), &[format!("LV{i}")]);
    }
    let mut prev = mid;
    for i in 13..25 {
        prev = builder.multi_branch(prev, Dir::S, format!("LV.{i}"), &[format!("LV{i}")]);
    }

    // The control inputs.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.SR{i}"),
            &[format!("SR_B{i}")],
        );
    }
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.BOUNCE{i}"),
            &[format!("BOUNCE{i}")],
        );
    }
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[format!("CLK_B{i}"), format!("CLK_B{i}_DCM0")],
        );
    }
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CE{i}"),
            &[format!("CE_B{i}")],
        );
    }

    // The data inputs.
    for i in 0..8 {
        let w = builder.mux_out(
            format!("IMUX.BYP{i}"),
            &[format!("BYP_INT_B{i}")],
        );
        builder.buf(w,
            format!("IMUX.BYP{i}.BOUNCE"),
            &[format!("BYP_BOUNCE{i}")],
        );
    }

    for i in 0..32 {
        builder.mux_out(
            format!("IMUX.IMUX{i}"),
            &[format!("IMUX_B{i}")],
        );
    }

    for i in 0..8 {
        builder.logic_out(format!("OUT.BEST{i}"), &[
            format!("BEST_LOGIC_OUTS{i}"),
        ]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.SEC{i}"), &[
            format!("SECONDARY_LOGIC_OUTS{i}"),
        ]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.HALF.BOT{i}"), &[
            format!("HALF_OMUX_BOT{i}"),
        ]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.HALF.TOP{i}"), &[
            format!("HALF_OMUX_TOP{i}"),
        ]);
    }

    builder.extract_nodes();

    builder.extract_term("W", Dir::W, "L_TERM_INT", "TERM.W");
    builder.extract_term("E", Dir::E, "R_TERM_INT", "TERM.E");
    builder.extract_term("S", Dir::S, "B_TERM_INT", "TERM.S");
    builder.extract_term("N", Dir::N, "T_TERM_INT", "TERM.N");
    for tkn in [
        "MGT_AL_BOT",
        "MGT_AL_MID",
        "MGT_AL",
        "MGT_BL",
    ] {
        if let Some(tk) = rd.tile_kinds.get(tkn) {
            for &xy in &tk.tiles {
                for (i, delta) in [
                    0, 1, 2, 3, 4, 5, 6, 7,
                    9, 10, 11, 12, 13, 14, 15, 16,
                ].into_iter().enumerate() {
                    let int_xy = Coord {
                        x: xy.x + 1,
                        y: xy.y - 9 + delta,
                    };
                    builder.extract_term_tile("W", Dir::W, xy, format!("TERM.W.MGT{i}"), int_xy);
                }
            }
        }
    }
    for tkn in [
        "MGT_AR_BOT",
        "MGT_AR_MID",
        "MGT_AR",
        "MGT_BR",
    ] {
        if let Some(tk) = rd.tile_kinds.get(tkn) {
            for &xy in &tk.tiles {
                for (i, delta) in [
                    0, 1, 2, 3, 4, 5, 6, 7,
                    9, 10, 11, 12, 13, 14, 15, 16,
                ].into_iter().enumerate() {
                    let int_xy = Coord {
                        x: xy.x - 1,
                        y: xy.y - 9 + delta,
                    };
                    builder.extract_term_tile("E", Dir::E, xy, format!("TERM.E.MGT{i}"), int_xy);
                }
            }
        }
    }

    builder.extract_pass_simple("BRKH", Dir::S, "BRKH", &[]);
    builder.extract_pass_buf("CLB_BUFFER", Dir::W, "CLB_BUFFER", "PASS.CLB_BUFFER", &[]);

    builder.stub_out("PB_OMUX11_B5");
    builder.stub_out("PB_OMUX11_B6");

    if let Some(tk) = rd.tile_kinds.get("PB") {
        for &pb_xy in &tk.tiles {
            let pt_xy = Coord {
                x: pb_xy.x,
                y: pb_xy.y + 18,
            };
            for (i, delta) in [
                0, 1, 2,
                4, 5, 6, 7, 8, 9, 10, 11,
                13, 14, 15, 16, 17, 18, 19, 20,
                22, 23, 24,
            ].into_iter().enumerate() {
                let int_w_xy = Coord {
                    x: pb_xy.x - 1,
                    y: pb_xy.y - 3 + delta,
                };
                let int_e_xy = Coord {
                    x: pb_xy.x + 15,
                    y: pb_xy.y - 3 + delta,
                };
                let naming_w = format!("TERM.PPC.W{i}");
                let naming_wm = format!("TERM.PPC.W{i}.MID");
                let naming_e = format!("TERM.PPC.E{i}");
                let naming_em = format!("TERM.PPC.E{i}.MID");
                let xy = if i < 11 { pb_xy } else { pt_xy };
                builder.extract_pass_tile("PPC.W", Dir::W, int_e_xy, Some((xy, &naming_w, Some(&naming_em))), Some((xy, &naming_em, &naming_e)), int_w_xy, &[]);
                builder.extract_pass_tile("PPC.E", Dir::E, int_w_xy, Some((xy, &naming_e, Some(&naming_wm))), Some((xy, &naming_wm, &naming_w)), int_e_xy, &[]);
            }
            for (i, delta) in [
                1, 3, 5, 7, 9, 11, 13
            ].into_iter().enumerate() {
                let int_s_xy = Coord {
                    x: pb_xy.x + delta,
                    y: pb_xy.y - 4,
                };
                let int_n_xy = Coord {
                    x: pb_xy.x + delta,
                    y: pb_xy.y + 22,
                };
                let ab = if i < 5 {'A'} else {'B'};
                let naming_s = format!("TERM.PPC.S{i}");
                let naming_sf = format!("TERM.PPC.S{i}.FAR");
                let naming_so = format!("TERM.PPC.S{i}.OUT");
                let naming_n = format!("TERM.PPC.N{i}");
                let naming_nf = format!("TERM.PPC.N{i}.FAR");
                let naming_no = format!("TERM.PPC.N{i}.OUT");
                builder.extract_pass_tile(format!("PPC{ab}.S"), Dir::S, int_n_xy, Some((pt_xy, &naming_s, Some(&naming_sf))), Some((pb_xy, &naming_no, &naming_n)), int_s_xy, &[]);
                builder.extract_pass_tile(format!("PPC{ab}.N"), Dir::N, int_s_xy, Some((pb_xy, &naming_n, Some(&naming_nf))), Some((pt_xy, &naming_so, &naming_s)), int_n_xy, &[]);
            }
        }
    }

    for (tkn, n, height) in [
        ("BRAM", "BRAM", 4),
        ("DSP", "DSP", 4),
        ("CCM", "CCM", 4),
        ("DCM", "DCM", 4),
        ("DCM_BOT", "DCM", 4),
        ("SYS_MON", "SYSMON", 8),
    ] {
        if let Some(tk) = rd.tile_kinds.get(tkn) {
            for &xy in &tk.tiles {
                for i in 0..height {
                    let int_xy = Coord {
                        x: xy.x - 1,
                        y: xy.y + i,
                    };
                    builder.extract_intf_tile("INTF", xy, int_xy, format!("{n}.{i}"), Some(&format!("{n}.{i}.INTFBUF")), None, None);
                }
            }
        }
    }
    for tkn in [
        "IOIS_LC",
        "IOIS_NC",
    ] {
        builder.extract_intf("INTF", Dir::E, tkn, "IOIS", Some("IOIS.INTFBUF"), None, None);
    }
    if let Some(tk) = rd.tile_kinds.get("CFG_CENTER") {
        for &xy in &tk.tiles {
            for i in 0..16 {
                let int_xy = Coord {
                    x: xy.x - 1,
                    y: if i < 8 {xy.y - 8 + i} else {xy.y + 1 + i - 8},
                };
                builder.extract_intf_tile("INTF", xy, int_xy, format!("CFG_CENTER.{i}"), Some(&format!("CFG_CENTER.{i}.INTFBUF")), None, None);
            }
        }
    }
    for (dir, tkn) in [
        (Dir::W, "MGT_AL"),
        (Dir::W, "MGT_AL_BOT"),
        (Dir::W, "MGT_AL_MID"),
        (Dir::W, "MGT_BL"),
        (Dir::E, "MGT_AR"),
        (Dir::E, "MGT_AR_BOT"),
        (Dir::E, "MGT_AR_MID"),
        (Dir::E, "MGT_BR"),
    ] {
        if let Some(tk) = rd.tile_kinds.get(tkn) {
            for &xy in &tk.tiles {
                for i in 0..16 {
                    let int_xy = Coord {
                        x: if dir == Dir::E {xy.x - 1} else {xy.x + 1},
                        y: if i < 8 {xy.y - 9 + i} else {xy.y + i - 8},
                    };
                    builder.extract_intf_tile("INTF", xy, int_xy, format!("MGT.{i}"), Some(&format!("MGT.{i}.INTFBUF")), None, None);
                }
            }
        }
    }
    if let Some(tk) = rd.tile_kinds.get("PB") {
        for &pb_xy in &tk.tiles {
            let pt_xy = Coord {
                x: pb_xy.x,
                y: pb_xy.y + 18,
            };
            for (i, delta) in [
                0, 1, 2, 3,
                5, 6, 7, 8, 9, 10, 11, 12,
                14, 15, 16, 17, 18, 19, 20, 21,
                23, 24, 25, 26,
            ].into_iter().enumerate() {
                let int_w_xy = Coord {
                    x: pb_xy.x - 1,
                    y: pb_xy.y - 4 + delta,
                };
                let int_e_xy = Coord {
                    x: pb_xy.x + 15,
                    y: pb_xy.y - 4 + delta,
                };
                let xy = if i < 12 { pb_xy } else { pt_xy };
                builder.extract_intf_tile("INTF", xy, int_w_xy, format!("PPC.L{i}"), Some(&format!("PPC.L{i}.INTFBUF")), None, None);
                builder.extract_intf_tile("INTF", xy, int_e_xy, format!("PPC.R{i}"), Some(&format!("PPC.R{i}.INTFBUF")), None, None);
            }
            for (i, delta) in [
                1, 3, 5, 7, 9, 11, 13
            ].into_iter().enumerate() {
                let int_s_xy = Coord {
                    x: pb_xy.x + delta,
                    y: pb_xy.y - 4,
                };
                let int_n_xy = Coord {
                    x: pb_xy.x + delta,
                    y: pb_xy.y + 22,
                };
                builder.extract_intf_tile("INTF", pb_xy, int_s_xy, format!("PPC.B{i}"), Some(&format!("PPC.B{i}.INTFBUF")), None, None);
                builder.extract_intf_tile("INTF", pt_xy, int_n_xy, format!("PPC.T{i}"), Some(&format!("PPC.T{i}.INTFBUF")), None, None);
            }
        }
    }

    builder.build()
}

fn make_grid(rd: &Part) -> virtex4::Grid {
    let int = extract_int(rd, &["INT", "INT_SO"], &[]);
    let columns = make_columns(rd, &int);
    let cols_io = get_cols_io(&columns);
    let (has_bot_sysmon, has_top_sysmon) = get_has_sysmons(rd);
    let reg_cfg = get_reg_cfg(rd, &int);
    virtex4::Grid {
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_io,
        regs: int.rows.len() / 16,
        has_bot_sysmon,
        has_top_sysmon,
        regs_cfg_io: get_regs_cfg_io(rd, &int, reg_cfg),
        ccm: get_ccm(rd),
        reg_cfg,
        holes_ppc: get_holes_ppc(rd, &int),
        has_bram_fx: get_has_bram_fx(rd),
    }
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(grid: &virtex4::Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.iob_name(), io))
        .collect();
    let gt_lookup: HashMap<_, _> = grid
        .get_gt()
        .into_iter()
        .flat_map(|gt| gt.get_pads(grid).into_iter().map(move |(name, func, pin, idx)| (name, (func, gt.bank, pin, idx))))
        .collect();
    let sm_lookup: HashMap<_, _> = grid
        .get_sysmon_pads()
        .into_iter()
        .map(|(name, bank, pin)| (name, (bank, pin)))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func = format!("IO_L{}{}", io.bbel / 2, ['N', 'P'][io.bbel as usize % 2]);
                match io.get_cfg() {
                    Some(CfgPin::Data(d)) => write!(exp_func, "_D{d}").unwrap(),
                    None => (),
                    _ => unreachable!(),
                }
                if io.is_gc() {
                    exp_func += "_GC";
                }
                if io.is_vref() {
                    exp_func += "_VREF";
                }
                if io.is_vr() {
                    match io.bel {
                        0 => exp_func += "_VRP",
                        1 => exp_func += "_VRN",
                        _ => unreachable!(),
                    }
                }
                if io.is_cc() {
                    exp_func += "_CC";
                }
                if let Some((bank, sm)) = io.sm_pair(grid) {
                    write!(exp_func, "_{}{}", ["SM", "ADC"][bank as usize], sm).unwrap();
                }
                if io.is_lc() {
                    exp_func += "_LC";
                }
                write!(exp_func, "_{}", io.bank).unwrap();
                if exp_func != pin.func {
                    println!("pad {pad} {io:?} got {f} exp {exp_func}", f=pin.func);
                }
                assert_eq!(pin.vref_bank, Some(io.bank));
                assert_eq!(pin.vcco_bank, Some(io.bank));
                BondPin::IoByBank(io.bank, io.bbel)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
            } else if let Some(&(bank, spin)) = sm_lookup.get(pad) {
                let exp_func = match (bank, spin) {
                    (0, SysMonPin::VP) => "VP_SM",
                    (0, SysMonPin::VN) => "VN_SM",
                    (1, SysMonPin::VP) => "VP_ADC",
                    (1, SysMonPin::VN) => "VN_ADC",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::SysMonByBank(bank, spin)
            } else {
                println!("unk iopad {pad} {f}", f=pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VBATT_0" => BondPin::VccBatt,
                "TCK_0" => BondPin::Cfg(CfgPin::Tck),
                "TDI_0" => BondPin::Cfg(CfgPin::Tdi),
                "TDO_0" => BondPin::Cfg(CfgPin::Tdo),
                "TMS_0" => BondPin::Cfg(CfgPin::Tms),
                "CCLK_0" => BondPin::Cfg(CfgPin::Cclk),
                "DONE_0" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM_B_0" => BondPin::Cfg(CfgPin::ProgB),
                "PWRDWN_B_0" => BondPin::Cfg(CfgPin::PwrdwnB),
                "INIT_0" => BondPin::Cfg(CfgPin::InitB),
                "RDWR_B_0" => BondPin::Cfg(CfgPin::RdWrB),
                "CS_B_0" => BondPin::Cfg(CfgPin::CsiB),
                "D_IN_0" => BondPin::Cfg(CfgPin::Din),
                "DOUT_BUSY_0" => BondPin::Cfg(CfgPin::Dout),
                "M0_0" => BondPin::Cfg(CfgPin::M0),
                "M1_0" => BondPin::Cfg(CfgPin::M1),
                "M2_0" => BondPin::Cfg(CfgPin::M2),
                "HSWAPEN_0" => BondPin::Cfg(CfgPin::HswapEn),
                "TDN_0" => BondPin::Dxn,
                "TDP_0" => BondPin::Dxp,
                "AVSS_SM" => BondPin::SysMonByBank(0, SysMonPin::AVss),
                "AVSS_ADC" => BondPin::SysMonByBank(1, SysMonPin::AVss),
                "AVDD_SM" => BondPin::SysMonByBank(0, SysMonPin::AVdd),
                "AVDD_ADC" => BondPin::SysMonByBank(1, SysMonPin::AVdd),
                "VREFP_SM" => BondPin::SysMonByBank(0, SysMonPin::VRefP),
                "VREFP_ADC" => BondPin::SysMonByBank(1, SysMonPin::VRefP),
                "VREFN_SM" => BondPin::SysMonByBank(0, SysMonPin::VRefN),
                "VREFN_ADC" => BondPin::SysMonByBank(1, SysMonPin::VRefN),
                _ => if let Some((n, b)) = split_num(&pin.func) {
                    match n {
                        "VCCO_" => BondPin::VccO(b),
                        "GNDA_" => BondPin::GtByBank(b, GtPin::GndA, 0),
                        "VTRXA_" => BondPin::GtByBank(b, GtPin::VtRx, 1),
                        "VTRXB_" => BondPin::GtByBank(b, GtPin::VtRx, 0),
                        "VTTXA_" => BondPin::GtByBank(b, GtPin::VtTx, 1),
                        "VTTXB_" => BondPin::GtByBank(b, GtPin::VtTx, 0),
                        "AVCCAUXRXA_" => BondPin::GtByBank(b, GtPin::AVccAuxRx, 1),
                        "AVCCAUXRXB_" => BondPin::GtByBank(b, GtPin::AVccAuxRx, 0),
                        "AVCCAUXTX_" => BondPin::GtByBank(b, GtPin::AVccAuxTx, 0),
                        "AVCCAUXMGT_" => BondPin::GtByBank(b, GtPin::AVccAuxMgt, 0),
                        "RTERM_" => BondPin::GtByBank(b, GtPin::RTerm, 0),
                        "MGTVREF_" => BondPin::GtByBank(b, GtPin::MgtVRef, 0),
                        _ => {
                            println!("UNK FUNC {}", pin.func);
                            continue;
                        }
                    }
                } else {
                    println!("UNK FUNC {}", pin.func);
                    continue;
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
    let grid = make_grid(rd);
    let int_db = make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(&grid, pins),
        ));
    }
    let eint = grid.expand_grid(&int_db);
    let mut vrf = Verifier::new(rd, &eint);
    vrf.finish();
    (make_device(rd, geom::Grid::Virtex4(grid), bonds, BTreeSet::new()), Some(int_db))
}
