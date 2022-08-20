use std::collections::{BTreeMap, BTreeSet, HashMap};
use prjcombine_xilinx_rawdump::{Part, PkgPin};
use prjcombine_xilinx_geom::{self as geom, Bond, BondPin, int, int::Dir};
use prjcombine_xilinx_geom::xc5200;

use crate::grid::{extract_int, PreDevice, make_device};
use crate::intb::IntBuilder;
use crate::verify::Verifier;

fn make_grid(rd: &Part) -> xc5200::Grid {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &[
        "CENTER",
        "LL",
        "LR",
        "UL",
        "UR",
    ], &[]);
    xc5200::Grid {
        columns: int.cols.len(),
        rows: int.rows.len(),
    }
}

fn make_bond(grid: &xc5200::Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                BondPin::IoByCoord(io)
            } else {
                println!("UNK PAD {}", pad);
                continue;
            }
        } else {
            println!("UNK FUNC {}", pin.func);
            continue;
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks: BTreeMap::new(),
    }
}

fn make_int_db(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("xc5200", rd);

    builder.wire("GND", int::WireKind::Tie0, &[
        "WIRE_PIN_GND_LEFT",
        "WIRE_PIN_GND_RIGHT",
        "WIRE_PIN_GND_BOT",
        "WIRE_PIN_GND_TOP",
        "WIRE_PIN_GND_BL",
        "WIRE_PIN_GND_BR",
        "WIRE_PIN_GNDSRC_TL",
        "WIRE_PIN_GND_SRC_TR",
    ]);

    for i in 0..24 {
        let w = builder.wire(format!("CLB.M{i}"), int::WireKind::PipOut, &[
            format!("WIRE_M{i}_CLB"),
        ]);
        builder.buf(w, format!("CLB.M{i}.BUF"), &[
            format!("WIRE_BUF{i}_CLB"),
        ]);
    }
    for i in 0..16 {
        let w = builder.wire(format!("IO.M{i}"), int::WireKind::PipOut, &[
            format!("WIRE_M{i}_LEFT"),
            format!("WIRE_M{i}_RIGHT"),
            format!("WIRE_M{i}_BOT"),
            format!("WIRE_M{i}_TOP"),
        ]);
        builder.buf(w, format!("IO.M{i}.BUF"), &[
            format!("WIRE_BUF{i}_LEFT"),
            format!("WIRE_BUF{i}_RIGHT"),
            format!("WIRE_BUF{i}_BOT"),
            format!("WIRE_BUF{i}_TOP"),
        ]);
    }

    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        let w = builder.wire(format!("SINGLE.E{i}"), int::WireKind::PipOut, &[
            format!("WIRE_E{i}_CLB"),
            format!("WIRE_E{i}_LEFT"),
        ]);
        builder.pip_branch(w, Dir::E, format!("SINGLE.W{i}"), &[
            format!("WIRE_W{i}_CLB"),
            format!("WIRE_W{i}_RIGHT"),
        ]);
    }
    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        let w = builder.wire(format!("SINGLE.S{i}"), int::WireKind::PipOut, &[
            format!("WIRE_S{i}_CLB"),
            format!("WIRE_S{i}_TOP"),
        ]);
        builder.pip_branch(w, Dir::S, format!("SINGLE.N{i}"), &[
            format!("WIRE_N{i}_CLB"),
            format!("WIRE_N{i}_BOT"),
        ]);
    }

    let mut cond_ll = Vec::new();
    let mut cond_lr = Vec::new();
    let mut cond_ul = Vec::new();
    let mut cond_ur = Vec::new();
    for i in 0..8 {
        let w_be = builder.wire(format!("IO.SINGLE.B.E{i}"), int::WireKind::PipOut, &[
            format!("WIRE_E{i}_BOT"),
        ]);
        let w_bw = builder.pip_branch(w_be, Dir::E, format!("IO.SINGLE.B.W{i}"), &[
            format!("WIRE_W{i}_BOT"),
            format!("WIRE_NW{i}_BR"),
        ]);
        let w_rn = builder.wire(format!("IO.SINGLE.R.N{i}"), int::WireKind::PipOut, &[
            format!("WIRE_N{i}_RIGHT"),
        ]);
        let w_rs = builder.pip_branch(w_rn, Dir::N, format!("IO.SINGLE.R.S{i}"), &[
            format!("WIRE_S{i}_RIGHT"),
            format!("WIRE_WS{i}_TR"),
        ]);
        let w_tw = builder.wire(format!("IO.SINGLE.T.W{i}"), int::WireKind::PipOut, &[
            format!("WIRE_W{i}_TOP"),
        ]);
        let w_te = builder.pip_branch(w_tw, Dir::W, format!("IO.SINGLE.T.E{i}"), &[
            format!("WIRE_E{i}_TOP"),
            format!("WIRE_SE{i}_TL"),
        ]);
        let w_ls = builder.wire(format!("IO.SINGLE.L.S{i}"), int::WireKind::PipOut, &[
            format!("WIRE_S{i}_LEFT"),
        ]);
        let w_ln = builder.pip_branch(w_ls, Dir::S, format!("IO.SINGLE.L.N{i}"), &[
            format!("WIRE_N{i}_LEFT"),
            format!("WIRE_EN{i}_BL"),
        ]);
        cond_ll.push((w_be, w_ln));
        cond_lr.push((w_rn, w_bw));
        cond_ul.push((w_ls, w_te));
        cond_ur.push((w_tw, w_rs));
    }

    for i in [0, 6] {
        let w = builder.wire(format!("DBL.H{i}.M"), int::WireKind::PipOut, &[
            format!("WIRE_DH{i}_CLB"),
            format!("WIRE_DH{i}_LEFT"),
            format!("WIRE_DH{i}_RIGHT"),
        ]);
        builder.pip_branch(w, Dir::W, format!("DBL.H{i}.W"), &[
            format!("WIRE_DE{i}_CLB"),
            format!("WIRE_DE{i}_LEFT"),
        ]);
        builder.pip_branch(w, Dir::E, format!("DBL.H{i}.E"), &[
            format!("WIRE_DW{i}_CLB"),
            format!("WIRE_DW{i}_RIGHT"),
        ]);
    }
    for i in [0, 6] {
        let w = builder.wire(format!("DBL.V{i}.M"), int::WireKind::PipOut, &[
            format!("WIRE_DV{i}_CLB"),
            format!("WIRE_DV{i}_BOT"),
            format!("WIRE_DV{i}_TOP"),
        ]);
        builder.pip_branch(w, Dir::S, format!("DBL.V{i}.S"), &[
            format!("WIRE_DN{i}_CLB"),
            format!("WIRE_DN{i}_BOT"),
        ]);
        builder.pip_branch(w, Dir::N, format!("DBL.V{i}.N"), &[
            format!("WIRE_DS{i}_CLB"),
            format!("WIRE_DS{i}_TOP"),
        ]);
    }

    for i in 0..8 {
        let w = builder.wire(format!("LONG.H{i}"), int::WireKind::MultiBranch(Dir::W), &[
            format!("WIRE_LH{i}_CLB"),
            format!("WIRE_LH{i}_LEFT"),
            format!("WIRE_LH{i}_RIGHT"),
            format!("WIRE_LH{i}_BOT"),
            format!("WIRE_LH{i}_TOP"),
            format!("WIRE_LH{i}_BL"),
            format!("WIRE_LH{i}_BR"),
            format!("WIRE_LH{i}_TL"),
            format!("WIRE_LH{i}_TR"),
        ]);
        builder.conn_branch(w, Dir::E, w);
    }
    for i in 0..8 {
        let w = builder.wire(format!("LONG.V{i}"), int::WireKind::MultiBranch(Dir::S), &[
            format!("WIRE_LV{i}_CLB"),
            format!("WIRE_LV{i}_LEFT"),
            format!("WIRE_LV{i}_RIGHT"),
            format!("WIRE_LV{i}_BOT"),
            format!("WIRE_LV{i}_TOP"),
            format!("WIRE_LV{i}_BL"),
            format!("WIRE_LV{i}_BR"),
            format!("WIRE_LV{i}_TL"),
            format!("WIRE_LV{i}_TR"),
        ]);
        builder.conn_branch(w, Dir::N, w);
    }

    let w = builder.wire("GLOBAL.L", int::WireKind::Branch(Dir::W), &[
        "WIRE_GH0_CLB",
        "WIRE_GH0_LEFT",
    ]);
    builder.conn_branch(w, Dir::E, w);
    let w = builder.wire("GLOBAL.R", int::WireKind::Branch(Dir::E), &[
        "WIRE_GH1_CLB",
        "WIRE_GH1_RIGHT",
    ]);
    builder.conn_branch(w, Dir::W, w);
    let w = builder.wire("GLOBAL.B", int::WireKind::Branch(Dir::S), &[
        "WIRE_GV0_CLB",
        "WIRE_GV0_BOT",
    ]);
    builder.conn_branch(w, Dir::N, w);
    let w = builder.wire("GLOBAL.T", int::WireKind::Branch(Dir::N), &[
        "WIRE_GV1_CLB",
        "WIRE_GV1_TOP",
    ]);
    builder.conn_branch(w, Dir::S, w);

    let w = builder.wire("GLOBAL.TL", int::WireKind::Branch(Dir::W), &[
        "WIRE_GTL_TOP",
        "WIRE_GTL_TL",
    ]);
    builder.conn_branch(w, Dir::E, w);
    let w = builder.wire("GLOBAL.BR", int::WireKind::Branch(Dir::E), &[
        "WIRE_GBR_BOT",
        "WIRE_GBR_BR",
    ]);
    builder.conn_branch(w, Dir::W, w);
    let w = builder.wire("GLOBAL.BL", int::WireKind::Branch(Dir::S), &[
        "WIRE_GBL_LEFT",
        "WIRE_GBL_BL",
    ]);
    builder.conn_branch(w, Dir::N, w);
    let w = builder.wire("GLOBAL.TR", int::WireKind::Branch(Dir::N), &[
        "WIRE_GTR_RIGHT",
        "WIRE_GTR_TR",
    ]);
    builder.conn_branch(w, Dir::S, w);

    for i in 0..8 {
        // only 4 of these outside CLB
        let w = builder.mux_out(format!("OMUX{i}"), &[
            format!("WIRE_OMUX{i}_CLB"),
            format!("WIRE_QIN{i}_LEFT"),
            format!("WIRE_QIN{i}_RIGHT"),
            format!("WIRE_QIN{i}_BOT"),
            format!("WIRE_QIN{i}_TOP"),
        ]);
        let w = builder.buf(w, format!("OMUX{i}.BUF"), &[
            format!("WIRE_Q{i}_CLB"),
            format!("WIRE_Q{i}_LEFT"),
            format!("WIRE_Q{i}_RIGHT"),
            format!("WIRE_Q{i}_BOT"),
            format!("WIRE_Q{i}_TOP"),
        ]);
        if i < 4 {
            builder.branch(w, Dir::W, format!("OMUX{i}.BUF.W"), &[
              format!("WIRE_QE{i}_CLB"),
              format!("WIRE_QE{i}_LEFT"),
            ]);
            builder.branch(w, Dir::E, format!("OMUX{i}.BUF.E"), &[
              format!("WIRE_QW{i}_CLB"),
              format!("WIRE_QW{i}_RIGHT"),
            ]);
            builder.branch(w, Dir::S, format!("OMUX{i}.BUF.S"), &[
              format!("WIRE_QN{i}_CLB"),
              format!("WIRE_QN{i}_BOT"),
            ]);
            builder.branch(w, Dir::N, format!("OMUX{i}.BUF.N"), &[
              format!("WIRE_QS{i}_CLB"),
              format!("WIRE_QS{i}_TOP"),
            ]);
        }
    }

    for i in 0..4 {
        for pin in ["X", "Q", "DO"] {
            builder.logic_out(format!("OUT.LC{i}.{pin}"), &[
                format!("WIRE_LC{i}_{pin}_CLB"),
            ]);
        }
    }
    for i in 0..4 {
        builder.logic_out(format!("OUT.TBUF{i}"), &[
            format!("WIRE_TQ{i}_CLB"),
            format!("WIRE_TQ{i}_LEFT"),
            format!("WIRE_TQ{i}_RIGHT"),
            format!("WIRE_TQ{i}_BOT"),
            format!("WIRE_TQ{i}_TOP"),
        ]);
    }
    builder.logic_out("OUT.PWRGND", &["WIRE_PWRGND_CLB"]);
    for i in 0..4 {
        builder.logic_out(format!("OUT.IO{i}.I"), &[
            format!("WIRE_PIN_IO{i}_I_LEFT"),
            format!("WIRE_PIN_IO{i}_I_RIGHT"),
            format!("WIRE_PIN_IO{i}_I_BOT"),
            format!("WIRE_PIN_IO{i}_I_TOP"),
        ]);
    }
    builder.logic_out("OUT.CLKIOB", &[
        "WIRE_PIN_CLKIOB_BL",
        "WIRE_PIN_CLKIOB_BR",
        "WIRE_PIN_CLKIOB_TL",
        "WIRE_PIN_CLKIOB_TR",
    ]);
    builder.logic_out("OUT.RDBK.RIP", &["WIRE_PIN_RIP_BL"]);
    builder.logic_out("OUT.RDBK.DATA", &["WIRE_PIN_DATA_BL"]);
    builder.logic_out("OUT.STARTUP.DONEIN", &["WIRE_PIN_DONEIN_BR"]);
    builder.logic_out("OUT.STARTUP.Q1Q4", &["WIRE_PIN_Q1Q4_BR"]);
    builder.logic_out("OUT.STARTUP.Q2", &["WIRE_PIN_Q2_BR"]);
    builder.logic_out("OUT.STARTUP.Q3", &["WIRE_PIN_Q3_BR"]);
    builder.logic_out("OUT.BSCAN.DRCK", &["WIRE_PIN_DRCK_TL"]);
    builder.logic_out("OUT.BSCAN.IDLE", &["WIRE_PIN_IDLE_TL"]);
    builder.logic_out("OUT.BSCAN.RESET", &["WIRE_PIN_RESET_TL"]);
    builder.logic_out("OUT.BSCAN.SEL1", &["WIRE_PIN_SEL1_TL"]);
    builder.logic_out("OUT.BSCAN.SEL2", &["WIRE_PIN_SEL2_TL"]);
    builder.logic_out("OUT.BSCAN.SHIFT", &["WIRE_PIN_SHIFT_TL"]);
    builder.logic_out("OUT.BSCAN.UPDATE", &["WIRE_PIN_UPDATE_TL"]);
    builder.logic_out("OUT.BSUPD", &["WIRE_PIN_BSUPD_TR"]);
    builder.logic_out("OUT.OSC.OSC1", &["WIRE_PIN_OSC1_TR"]);
    builder.logic_out("OUT.OSC.OSC2", &["WIRE_PIN_OSC2_TR"]);
    builder.logic_out("OUT.TOP.COUT", &["WIRE_COUT_TOP"]);

    for i in 0..4 {
        for pin in ["F1", "F2", "F3", "F4", "DI"] {
            builder.mux_out(format!("IMUX.LC{i}.{pin}"), &[
                format!("WIRE_PIN_LC{i}_{pin}_CLB"),
            ]);
        }
    }
    for pin in ["CE", "CLK", "RST"] {
        builder.mux_out(format!("IMUX.CLB.{pin}"), &[format!("WIRE_{pin}_CLB")]);
    }
    builder.mux_out("IMUX.TS", &[
        "WIRE_TS_CLB",
        "WIRE_TS_LEFT",
        "WIRE_TS_RIGHT",
        "WIRE_TS_BOT",
        "WIRE_TS_TOP",
    ]);
    builder.mux_out("IMUX.GIN", &[
        "WIRE_GIN_LEFT",
        "WIRE_GIN_RIGHT",
        "WIRE_GIN_BOT",
        "WIRE_GIN_TOP",
    ]);
    for i in 0..4 {
        for pin in ["T", "O"] {
            builder.mux_out(format!("IMUX.IO{i}.{pin}"), &[
                format!("WIRE_PIN_IO{i}_{pin}_LEFT"),
                format!("WIRE_PIN_IO{i}_{pin}_RIGHT"),
                format!("WIRE_PIN_IO{i}_{pin}_BOT"),
                format!("WIRE_PIN_IO{i}_{pin}_TOP"),
            ]);
        }
    }
    builder.mux_out("IMUX.RDBK.RCLK", &["WIRE_PIN_RCLK_BL"]);
    builder.mux_out("IMUX.RDBK.TRIG", &["WIRE_PIN_TRIG_BL"]);
    builder.mux_out("IMUX.STARTUP.SCLK", &["WIRE_PIN_SCLK_BR"]);
    builder.mux_out("IMUX.STARTUP.GRST", &["WIRE_PIN_GRST_BR"]);
    builder.mux_out("IMUX.STARTUP.GTS", &["WIRE_PIN_GTS_BR"]);
    builder.mux_out("IMUX.BSCAN.TDO1", &["WIRE_PIN_TDO1_TL"]);
    builder.mux_out("IMUX.BSCAN.TDO2", &["WIRE_PIN_TDO2_TL"]);
    builder.mux_out("IMUX.OSC.OCLK", &["WIRE_PIN_OCLK_TR"]);
    builder.mux_out("IMUX.BYPOSC.PUMP", &["WIRE_PIN_PUMP_TR"]);
    builder.mux_out("IMUX.BUFG", &[
        "WIRE_PIN_BUFGIN_BL",
        "WIRE_PIN_BUFGIN_BR",
        "WIRE_PIN_BUFGIN_TL",
        "WIRE_PIN_BUFGIN_TR",
    ]);
    let bot_cin = builder.mux_out("IMUX.BOT.CIN", &["WIRE_COUT_BOT"]);

    builder.extract_main_passes();

    builder.node_type("CENTER", "CLB", "CLB");
    builder.node_type("LEFT", "IO.L", "IO.L");
    builder.node_type("LEFTCLK", "IO.L", "IO.L");
    builder.node_type("RIGHT", "IO.R", "IO.R");
    builder.node_type("RIGHTCLK", "IO.R", "IO.R");
    builder.node_type("BOT", "IO.B", "IO.B");
    builder.node_type("BOTCLK", "IO.B", "IO.B");
    builder.node_type("TOP", "IO.T", "IO.T");
    builder.node_type("TOPCLK", "IO.T", "IO.T");
    builder.node_type("LL", "CNR.BL", "CNR.BL");
    builder.node_type("LR", "CNR.BR", "CNR.BR");
    builder.node_type("UL", "CNR.TL", "CNR.TL");
    builder.node_type("UR", "CNR.TR", "CNR.TR");

    let node_ll = builder.db.nodes.get("CNR.BL").unwrap().0;
    let node_lr = builder.db.nodes.get("CNR.BR").unwrap().0;
    let node_ul = builder.db.nodes.get("CNR.TL").unwrap().0;
    let node_ur = builder.db.nodes.get("CNR.TR").unwrap().0;
    for (a, b) in cond_ll {
        builder.db.wires[a].kind = int::WireKind::CondAlias(node_ll, b);
    }
    for (a, b) in cond_lr {
        builder.db.wires[a].kind = int::WireKind::CondAlias(node_lr, b);
    }
    for (a, b) in cond_ul {
        builder.db.wires[a].kind = int::WireKind::CondAlias(node_ul, b);
    }
    for (a, b) in cond_ur {
        builder.db.wires[a].kind = int::WireKind::CondAlias(node_ur, b);
    }
    let node_bot = builder.db.nodes.get_mut("IO.B").unwrap().1;
    for mux in node_bot.muxes.values_mut() {
        mux.ins.retain(|&x| x.1 != bot_cin);
    }

    for tkn in [
        "CLKV",
        "CLKB",
        "CLKT",
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_fwd_xy = builder.walk_to_int(xy, Dir::W).unwrap();
            let int_bwd_xy = builder.walk_to_int(xy, Dir::E).unwrap();
            builder.extract_pass_tile("LLH.W", Dir::W, int_bwd_xy, Some(xy), None, None, Some((tkn, tkn)), int_fwd_xy, &[]);
            builder.extract_pass_tile("LLH.E", Dir::E, int_fwd_xy, Some(xy), None, None, None, int_bwd_xy, &[]);
        }
    }

    for tkn in [
        "CLKH",
        "CLKL",
        "CLKR",
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_fwd_xy = builder.walk_to_int(xy, Dir::S).unwrap();
            let int_bwd_xy = builder.walk_to_int(xy, Dir::N).unwrap();
            builder.extract_pass_tile("LLV.S", Dir::S, int_bwd_xy, Some(xy), None, None, Some((tkn, tkn)), int_fwd_xy, &[]);
            builder.extract_pass_tile("LLV.N", Dir::N, int_fwd_xy, Some(xy), None, None, None, int_bwd_xy, &[]);
        }
    }

    builder.build()
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
    let vrf = Verifier::new(rd, &eint);
    vrf.finish();
    (make_device(rd, geom::Grid::Xc5200(grid), bonds, BTreeSet::new()), Some(int_db))
}
