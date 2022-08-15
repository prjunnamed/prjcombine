use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use prjcombine_xilinx_rawdump::{Part, PkgPin};
use prjcombine_xilinx_geom::{self as geom, CfgPin, Bond, BondPin, GtPin, GtRegionPin, SysMonPin, ColId, RowId, int, int::Dir};
use prjcombine_xilinx_geom::virtex5::{self, ColumnKind, HardColumn};
use prjcombine_entity::{EntityVec, EntityId};

use crate::grid::{extract_int, find_column, find_columns, find_rows, find_row, IntGrid, PreDevice, make_device};
use crate::intb::IntBuilder;
use crate::verify::Verifier;

fn make_columns(rd: &Part, int: &IntGrid) -> EntityVec<ColId, ColumnKind> {
    let mut res: EntityVec<ColId, Option<ColumnKind>> = int.cols.map_values(|_| None);
    for c in find_columns(rd, &["CLBLL"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLL);
    }
    for c in find_columns(rd, &["CLBLM"]) {
        res[int.lookup_column(c - 1)] = Some(ColumnKind::ClbLM);
    }
    for c in find_columns(rd, &["BRAM", "PCIE_BRAM"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["DSP"]) {
        res[int.lookup_column(c - 2)] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["IOI"]) {
        res[int.lookup_column_inter(c) - 1] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["GT3"]) {
        res[int.lookup_column(c - 3)] = Some(ColumnKind::Gtp);
    }
    for c in find_columns(rd, &["GTX"]) {
        res[int.lookup_column(c - 3)] = Some(ColumnKind::Gtx);
    }
    for c in find_columns(rd, &["GTX_LEFT"]) {
        res[int.lookup_column(c + 2)] = Some(ColumnKind::Gtx);
    }
    res.map_values(|x| x.unwrap())
}

fn get_cols_vbrk(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["CFG_VBRK"]) {
        res.insert(int.lookup_column_inter(c));
    }
    res
}

fn get_cols_mgt_buf(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    let mut res = BTreeSet::new();
    for c in find_columns(rd, &["HCLK_BRAM_MGT, HCLK_BRAM_MGT_LEFT"]) {
        res.insert(int.lookup_column(c - 2));
    }
    res
}

fn get_col_hard(rd: &Part, int: &IntGrid) -> Option<HardColumn> {
    let col = int.lookup_column(find_column(rd, &["EMAC", "PCIE_B"])? - 2);
    let rows_emac = find_rows(rd, &["EMAC"]).into_iter().map(|r| int.lookup_row(r)).collect();
    let rows_pcie = find_rows(rd, &["PCIE_B"]).into_iter().map(|r| int.lookup_row(r) - 10).collect();
    Some(HardColumn {
        col,
        rows_emac,
        rows_pcie,
    })
}

fn get_cols_io(columns: &EntityVec<ColId, ColumnKind>) -> [Option<ColId>; 3] {
    let v: Vec<_> = columns.iter().filter_map(|(k, &v)| if v == ColumnKind::Io {Some(k)} else {None}).collect();
    if v.len() == 2 {
        [Some(v[0]), Some(v[1]), None]
    } else {
        [Some(v[0]), Some(v[1]), Some(v[2])]
    }
}

fn get_reg_cfg(rd: &Part, int: &IntGrid) -> usize {
    int.lookup_row_inter(find_row(rd, &["CFG_CENTER"]).unwrap()).to_idx() / 20
}

fn get_holes_ppc(rd: &Part, int: &IntGrid) -> Vec<(ColId, RowId)> {
    let mut res = Vec::new();
    for tile in rd.tiles_by_kind_name("PPC_B") {
        let x = int.lookup_column((tile.x - 11) as i32);
        let y = int.lookup_row((tile.y - 10) as i32);
        assert_eq!(y.to_idx() % 20, 0);
        res.push((x, y));
    }
    res
}

fn make_int_db(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("virtex5", rd);
    builder.node_type("INT", "INT", "INT");

    builder.wire("PULLUP", int::WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", int::WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", int::WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..10 {
        builder.wire(format!("GCLK{i}"), int::WireKind::ClkOut(i), &[
            format!("GCLK{i}"),
        ]);
    }
    for i in 0..4 {
        builder.wire(format!("RCLK{i}"), int::WireKind::ClkOut(10+i), &[
            format!("RCLK{i}"),
        ]);
    }

    for (name, da, db, dbeg, dend, dmid) in [
        ("EL", Dir::E, Dir::E, None, None, None),
        ("ER", Dir::E, Dir::E, Some((0, Dir::S)), None, None),
        ("EN", Dir::E, Dir::N, None, None, None),
        ("ES", Dir::E, Dir::S, None, None, None),
        ("WL", Dir::W, Dir::W, Some((0, Dir::S)), None, None),
        ("WR", Dir::W, Dir::W, Some((2, Dir::N)), None, None),
        ("WN", Dir::W, Dir::N, None, Some((0, Dir::S)), None),
        ("WS", Dir::W, Dir::S, None, None, Some((0, Dir::S))),
        ("NL", Dir::N, Dir::N, Some((0, Dir::S)), None, None),
        ("NR", Dir::N, Dir::N, Some((2, Dir::N)), None, None),
        ("NE", Dir::N, Dir::E, None, None, Some((2, Dir::N))),
        ("NW", Dir::N, Dir::W, None, Some((2, Dir::N)), None),
        ("SL", Dir::S, Dir::S, Some((2, Dir::N)), None, None),
        ("SR", Dir::S, Dir::S, None, None, None),
        ("SE", Dir::S, Dir::E, None, None, None),
        ("SW", Dir::S, Dir::W, None, None, None),
    ] {
        for i in 0..3 {
            let beg;
            if let Some((xi, dbeg)) = dbeg {
                if xi == i {
                    let beg_x = builder.mux_out(
                        format!("DBL.{name}{i}.0.{dbeg}"),
                        &[&format!("{name}2BEG_{dbeg}{i}")],
                    );
                    beg = builder.branch(beg_x, !dbeg,
                        format!("DBL.{name}{i}.0"),
                        &[format!("{name}2BEG{i}")]
                    );
                } else {
                    beg = builder.mux_out(
                        format!("DBL.{name}{i}.0"),
                        &[format!("{name}2BEG{i}")]
                    );
                }
            } else {
                beg = builder.mux_out(
                    format!("DBL.{name}{i}.0"),
                    &[format!("{name}2BEG{i}")]
                );
            }
            let mid = builder.branch(beg, da,
                format!("DBL.{name}{i}.1"),
                &[format!("{name}2MID{i}")],
            );
            if let Some((xi, dmid)) = dmid {
                if xi == i {
                    let mid_buf = builder.buf(mid,
                        format!("DBL.{name}{i}.1.BUF"),
                        &[format!("{name}2MID_FAKE{i}")],
                    );
                    builder.branch(mid_buf, dmid,
                        format!("DBL.{name}{i}.1.{dmid}"),
                        &[format!("{name}2MID_{dmid}{i}")],
                    );
                }
            }
            let end = builder.branch(mid, db,
                format!("DBL.{name}{i}.2"),
                &[format!("{name}2END{i}")],
            );
            if let Some((xi, dend)) = dend {
                if xi == i {
                    builder.branch(end, dend,
                        format!("DBL.{name}{i}.2.{dend}"),
                        &[format!("{name}2END_{dend}{i}")],
                    );
                }
            }
        }
    }

    for (name, da, db, dbeg, dend, dmid) in [
        ("EL", Dir::E, Dir::E, None, None, None),
        ("ER", Dir::E, Dir::E, None, None, None),
        ("EN", Dir::E, Dir::N, None, None, None),
        ("ES", Dir::E, Dir::S, None, None, None),
        ("WL", Dir::W, Dir::W, Some((0, Dir::S)), None, None),
        ("WR", Dir::W, Dir::W, None, None, None),
        ("WN", Dir::W, Dir::N, None, Some((0, Dir::S)), None),
        ("WS", Dir::W, Dir::S, None, None, Some((0, Dir::S))),
        ("NL", Dir::N, Dir::N, None, None, None),
        ("NR", Dir::N, Dir::N, Some((2, Dir::N)), None, None),
        ("NE", Dir::N, Dir::E, None, None, Some((2, Dir::N))),
        ("NW", Dir::N, Dir::W, None, Some((2, Dir::N)), None),
        ("SL", Dir::S, Dir::S, None, None, None),
        ("SR", Dir::S, Dir::S, None, None, None),
        ("SE", Dir::S, Dir::E, None, None, None),
        ("SW", Dir::S, Dir::W, None, None, None),
    ] {
        for i in 0..3 {
            let beg;
            if let Some((xi, dbeg)) = dbeg {
                if xi == i {
                    let beg_x = builder.mux_out(
                        format!("PENT.{name}{i}.0.{dbeg}"),
                        &[&format!("{name}5BEG_{dbeg}{i}")],
                    );
                    beg = builder.branch(beg_x, !dbeg,
                        format!("PENT.{name}{i}.0"),
                        &[format!("{name}5BEG{i}")]
                    );
                } else {
                    beg = builder.mux_out(
                        format!("PENT.{name}{i}.0"),
                        &[format!("{name}5BEG{i}")]
                    );
                }
            } else {
                beg = builder.mux_out(
                    format!("PENT.{name}{i}.0"),
                    &[format!("{name}5BEG{i}")]
                );
            }
            let a = builder.branch(beg, da,
                format!("PENT.{name}{i}.1"),
                &[format!("{name}5A{i}")],
            );
            let b = builder.branch(a, da,
                format!("PENT.{name}{i}.2"),
                &[format!("{name}5B{i}")],
            );
            let mid = builder.branch(b, da,
                format!("PENT.{name}{i}.3"),
                &[format!("{name}5MID{i}")],
            );
            if let Some((xi, dmid)) = dmid {
                if xi == i {
                    let mid_buf = builder.buf(mid,
                        format!("PENT.{name}{i}.3.BUF"),
                        &[format!("{name}5MID_FAKE{i}")],
                    );
                    builder.branch(mid_buf, dmid,
                        format!("PENT.{name}{i}.3.{dmid}"),
                        &[format!("{name}5MID_{dmid}{i}")],
                    );
                }
            }
            let c = builder.branch(mid, db,
                format!("PENT.{name}{i}.4"),
                &[format!("{name}5C{i}")],
            );
            let end = builder.branch(c, db,
                format!("PENT.{name}{i}.5"),
                &[format!("{name}5END{i}")],
            );
            if let Some((xi, dend)) = dend {
                if xi == i {
                    builder.branch(end, dend,
                        format!("PENT.{name}{i}.5.{dend}"),
                        &[format!("{name}5END_{dend}{i}")],
                    );
                }
            }
        }
    }

    // The long wires.
    let mid = builder.wire("LH.9", int::WireKind::MultiOut, &["LH9"]);
    let mut prev = mid;
    let mut lh_all = vec![mid];
    for i in (0..9).rev() {
        prev = builder.multi_branch(prev, Dir::E, format!("LH.{i}"), &[format!("LH{i}")]);
        lh_all.push(prev);
    }
    let mut prev = mid;
    let mut lh_bh_e = Vec::new();
    for i in 10..19 {
        prev = builder.multi_branch(prev, Dir::W, format!("LH.{i}"), &[format!("LH{i}")]);
        lh_bh_e.push(prev);
        lh_all.push(prev);
    }
    let mid = builder.wire("LV.9", int::WireKind::MultiOut, &["LV9"]);
    let mut prev = mid;
    let mut lv_bh_n = Vec::new();
    for i in (0..9).rev() {
        prev = builder.multi_branch(prev, Dir::S, format!("LV.{i}"), &[format!("LV{i}")]);
        lv_bh_n.push(prev);
    }
    let mut prev = mid;
    let mut lv_bh_s = Vec::new();
    for i in 10..19 {
        prev = builder.multi_branch(prev, Dir::N, format!("LV.{i}"), &[format!("LV{i}")]);
        lv_bh_s.push(prev);
    }

    // The control inputs.
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.GFAN{i}"),
            &[format!("GFAN{i}")],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[format!("CLK_B{i}")],
        );
    }
    for i in 0..4 {
        let w = builder.mux_out(
            format!("IMUX.CTRL{i}"),
            &[format!("CTRL{i}")],
        );
        builder.buf(w,
            format!("IMUX.CTRL{i}.SITE"),
            &[format!("CTRL_B{i}")],
        );
        let b = builder.buf(w,
            format!("IMUX.CTRL{i}.BOUNCE"),
            &[format!("CTRL_BOUNCE{i}")],
        );
        let dir = match i {
            0 => Dir::S,
            3 => Dir::N,
            _ => continue,
        };
        builder.branch(b, dir,
            format!("IMUX.CTRL{i}.BOUNCE.{dir}"),
            &[format!("CTRL_BOUNCE_{dir}{i}")],
        );
    }
    for i in 0..8 {
        let w = builder.mux_out(
            format!("IMUX.BYP{i}"),
            &[format!("BYP{i}")],
        );
        builder.buf(w,
            format!("IMUX.BYP{i}.SITE"),
            &[format!("BYP_B{i}")],
        );
        let b = builder.buf(w,
            format!("IMUX.BYP{i}.BOUNCE"),
            &[format!("BYP_BOUNCE{i}")],
        );
        let dir = match i {
            0 | 4 => Dir::S,
            3 | 7 => Dir::N,
            _ => continue,
        };
        builder.branch(b, dir,
            format!("IMUX.BYP{i}.BOUNCE.{dir}"),
            &[format!("BYP_BOUNCE_{dir}{i}")],
        );
    }
    for i in 0..8 {
        let w = builder.mux_out(
            format!("IMUX.FAN{i}"),
            &[format!("FAN{i}")],
        );
        builder.buf(w,
            format!("IMUX.FAN{i}.SITE"),
            &[format!("FAN_B{i}")],
        );
        let b = builder.buf(w,
            format!("IMUX.FAN{i}.BOUNCE"),
            &[format!("FAN_BOUNCE{i}")],
        );
        let dir = match i {
            0 => Dir::S,
            7 => Dir::N,
            _ => continue,
        };
        builder.branch(b, dir,
            format!("IMUX.FAN{i}.BOUNCE.{dir}"),
            &[format!("FAN_BOUNCE_{dir}{i}")],
        );
    }
    for i in 0..48 {
        builder.mux_out(
            format!("IMUX.IMUX{i}"),
            &[format!("IMUX_B{i}")],
        );
    }

    for i in 0..24 {
        let w = builder.logic_out(
            format!("OUT{i}"),
            &[format!("LOGIC_OUTS{i}")],
        );
        let dir = match i {
            15 | 17 => Dir::N,
            12 | 18 => Dir::S,
            _ => continue,
        };
        builder.branch(w, dir,
            format!("OUT{i}.{dir}.DBL"),
            &[format!("LOGIC_OUTS_{dir}{i}")],
        );
        builder.branch(w, dir,
            format!("OUT{i}.{dir}.PENT"),
            &[format!("LOGIC_OUTS_{dir}1_{i}")],
        );
    }

    for i in 0..4 {
        let w = builder.test_out(format!("TEST{i}"));
        builder.extra_name(format!("INT_INTERFACE_BLOCK_INPS_B{i}"), w);
        builder.extra_name(format!("PPC_L_INT_INTERFACE_BLOCK_INPS_B{i}"), w);
        builder.extra_name(format!("PPC_R_INT_INTERFACE_BLOCK_INPS_B{i}"), w);
        builder.extra_name(format!("GTX_LEFT_INT_INTERFACE_BLOCK_INPS_B{i}"), w);
    }

    builder.extract_nodes();

    builder.extract_term_buf("W", Dir::W, "L_TERM_INT", "TERM.W", &[]);
    builder.extract_term_buf("W", Dir::W, "GTX_L_TERM_INT", "TERM.W", &[]);
    builder.extract_term_buf("E", Dir::E, "R_TERM_INT", "TERM.E", &[]);
    builder.make_blackhole_term("E.HOLE", Dir::E, &lh_bh_e);
    builder.make_blackhole_term("S.HOLE", Dir::S, &lv_bh_s);
    builder.make_blackhole_term("N.HOLE", Dir::N, &lv_bh_n);
    let forced = [
        (builder.find_wire("PENT.NW2.5.N"), builder.find_wire("PENT.WN0.5")),
        (builder.find_wire("PENT.WN0.5"), builder.find_wire("PENT.WS2.4")),
    ];
    builder.extract_term_buf("S.PPC", Dir::S, "PPC_T_TERM", "TERM.PPC.S", &forced);
    let forced = [
        (builder.find_wire("PENT.NR2.0"), builder.find_wire("PENT.WL0.0.S")),
        (builder.find_wire("PENT.SL0.1"), builder.find_wire("PENT.NR2.0")),
    ];
    builder.extract_term_buf("N.PPC", Dir::N, "PPC_B_TERM", "TERM.PPC.N", &forced);

    for &xy_l in rd.tiles_by_kind_name("INT_BUFS_L") {
        let mut xy_r = xy_l;
        while !matches!(&rd.tile_kinds.key(rd.tiles[&xy_r].kind)[..], "INT_BUFS_R" | "INT_BUFS_R_MON") {
            xy_r.x += 1;
        }
        if xy_l.y < 10 || xy_l.y >= rd.height - 10 {
            // wheeee.
            continue;
        }
        let int_w_xy = builder.walk_to_int(xy_l, Dir::W).unwrap();
        let int_e_xy = builder.walk_to_int(xy_l, Dir::E).unwrap();
        builder.extract_pass_tile("INT_BUFS.W", Dir::W, int_e_xy, Some((xy_r, "INT_BUFS.W")), Some(xy_l), int_w_xy, &lh_all);
        builder.extract_pass_tile("INT_BUFS.E", Dir::E, int_w_xy, Some((xy_l, "INT_BUFS.E")), Some(xy_r), int_e_xy, &lh_all);
    }
    for &xy_l in rd.tiles_by_kind_name("L_TERM_PPC") {
        let mut xy_r = xy_l;
        while rd.tile_kinds.key(rd.tiles[&xy_r].kind) != "R_TERM_PPC" {
            xy_r.x += 1;
        }
        let int_w_xy = builder.walk_to_int(xy_l, Dir::W).unwrap();
        let int_e_xy = builder.walk_to_int(xy_l, Dir::E).unwrap();
        builder.extract_pass_tile("PPC.W", Dir::W, int_e_xy, Some((xy_r, "PPC.W")), Some(xy_l), int_w_xy, &lh_all);
        builder.extract_pass_tile("PPC.E", Dir::E, int_w_xy, Some((xy_l, "PPC.E")), Some(xy_r), int_e_xy, &lh_all);
    }

    builder.extract_intf("INTF", Dir::E, "INT_INTERFACE", "INTF", true);
    for (n, tkn) in [
        ("GTX_LEFT", "GTX_LEFT_INT_INTERFACE"),
        ("GTP", "GTP_INT_INTERFACE"),
        ("EMAC", "EMAC_INT_INTERFACE"),
        ("PCIE", "PCIE_INT_INTERFACE"),
        ("PPC_L", "PPC_L_INT_INTERFACE"),
        ("PPC_R", "PPC_R_INT_INTERFACE"),
    ] {
        builder.extract_intf("INTF.DELAY", Dir::E, tkn, format!("INTF.{n}"), true);
    }

    let mps = builder.db.terms.get("MAIN.S").unwrap().1.clone();
    builder.db.terms.insert("MAIN.NHOLE.S".to_string(), mps);
    let mut mpn = builder.db.terms.get("MAIN.N").unwrap().1.clone();
    for w in lv_bh_n {
        mpn.wires.insert(w, int::TermInfo::BlackHole);
    }
    builder.db.terms.insert("MAIN.NHOLE.N".to_string(), mpn);

    builder.build()
}

fn make_grid(rd: &Part) -> virtex5::Grid {
    let int = extract_int(rd, &["INT"], &[]);
    let columns = make_columns(rd, &int);
    let cols_io = get_cols_io(&columns);
    let reg_cfg = get_reg_cfg(rd, &int);
    virtex5::Grid {
        columns,
        cols_vbrk: get_cols_vbrk(rd, &int),
        cols_mgt_buf: get_cols_mgt_buf(rd, &int),
        col_hard: get_col_hard(rd, &int),
        cols_io,
        regs: (int.rows.len() / 20),
        reg_cfg,
        holes_ppc: get_holes_ppc(rd, &int),
    }
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let pos = s.find(|c: char| c.is_digit(10))?;
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(grid: &virtex5::Grid, pins: &[PkgPin]) -> Bond {
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
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                let mut exp_func = format!("IO_L{}{}", io.bbel / 2, ['N', 'P'][io.bbel as usize % 2]);
                if io.is_cc() {
                    exp_func += "_CC";
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
                match io.get_cfg() {
                    Some(CfgPin::Data(d)) => {
                        if d >= 16 {
                            write!(exp_func, "_A{}", d - 16).unwrap();
                        }
                        write!(exp_func, "_D{d}").unwrap();
                        if d < 3 {
                            write!(exp_func, "_FS{d}").unwrap();
                        }
                    }
                    Some(CfgPin::Addr(a)) => {
                        write!(exp_func, "_A{a}").unwrap();
                    }
                    Some(CfgPin::Rs(a)) => {
                        write!(exp_func, "_RS{a}").unwrap();
                    }
                    Some(CfgPin::CsoB) => exp_func += "_CSO_B",
                    Some(CfgPin::FweB) => exp_func += "_FWE_B",
                    Some(CfgPin::FoeB) => exp_func += "_FOE_B_MOSI",
                    Some(CfgPin::FcsB) => exp_func += "_FCS_B",
                    None => (),
                    _ => unreachable!(),
                }
                if let Some(sm) = io.sm_pair() {
                    write!(exp_func, "_SM{}{}", sm, ['N', 'P'][io.bbel as usize % 2]).unwrap();
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
            } else if let Some(&spin) = sm_lookup.get(pad) {
                let exp_func = match spin {
                    SysMonPin::VP => "VP_0",
                    SysMonPin::VN => "VN_0",
                    _ => unreachable!(),
                };
                if exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::SysMonByBank(0, spin)
            } else {
                println!("unk iopad {pad} {f}", f=pin.func);
                continue;
            }
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "RSVD" => BondPin::Rsvd, // ??? on TXT devices
                "RSVD_0" => BondPin::Rsvd, // actually VFS, R_FUSE
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
                "INIT_B_0" => BondPin::Cfg(CfgPin::InitB),
                "RDWR_B_0" => BondPin::Cfg(CfgPin::RdWrB),
                "CS_B_0" => BondPin::Cfg(CfgPin::CsiB),
                "D_IN_0" => BondPin::Cfg(CfgPin::Din),
                "D_OUT_BUSY_0" => BondPin::Cfg(CfgPin::Dout),
                "M0_0" => BondPin::Cfg(CfgPin::M0),
                "M1_0" => BondPin::Cfg(CfgPin::M1),
                "M2_0" => BondPin::Cfg(CfgPin::M2),
                "HSWAPEN_0" => BondPin::Cfg(CfgPin::HswapEn),
                "DXN_0" => BondPin::Dxn,
                "DXP_0" => BondPin::Dxp,
                "AVSS_0" => BondPin::SysMonByBank(0, SysMonPin::AVss),
                "AVDD_0" => BondPin::SysMonByBank(0, SysMonPin::AVdd),
                "VREFP_0" => BondPin::SysMonByBank(0, SysMonPin::VRefP),
                "VREFN_0" => BondPin::SysMonByBank(0, SysMonPin::VRefN),
                "MGTAVTTRXC" => BondPin::GtByRegion(1, GtRegionPin::AVttRxC),
                "MGTAVTTRXC_L" => BondPin::GtByRegion(0, GtRegionPin::AVttRxC),
                "MGTAVTTRXC_R" => BondPin::GtByRegion(1, GtRegionPin::AVttRxC),
                _ => if let Some((n, b)) = split_num(&pin.func) {
                    match n {
                        "VCCO_" => BondPin::VccO(b),
                        "MGTAVCC_" => BondPin::GtByBank(b, GtPin::AVcc, 0),
                        "MGTAVCCPLL_" => BondPin::GtByBank(b, GtPin::AVccPll, 0),
                        "MGTAVTTRX_" => BondPin::GtByBank(b, GtPin::VtRx, 0),
                        "MGTAVTTTX_" => BondPin::GtByBank(b, GtPin::VtTx, 0),
                        "MGTRREF_" => BondPin::GtByBank(b, GtPin::RRef, 0),
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
    (make_device(rd, geom::Grid::Virtex5(grid), bonds, BTreeSet::new()), Some(int_db))
}
