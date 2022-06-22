use std::collections::{BTreeMap, BTreeSet, HashMap};
use prjcombine_xilinx_rawdump::{Part, Coord, PkgPin};
use prjcombine_xilinx_geom::{self as geom, DisabledPart, CfgPin, Bond, BondPin, int, int::Dir};
use prjcombine_xilinx_geom::virtex::{self, GridKind};

use itertools::Itertools;

use crate::grid::{extract_int, find_columns, IntGrid, PreDevice, make_device};
use crate::intb::IntBuilder;

fn get_kind(rd: &Part) -> GridKind {
    match &rd.family[..] {
        "virtex" | "spartan2" => GridKind::Virtex,
        "virtexe" | "spartan2e" => if find_columns(rd, &["MBRAM"]).contains(&6) {
            GridKind::VirtexEM
        } else {
            GridKind::VirtexE
        },
        _ => panic!("unknown family {}", rd.family),
    }
}

fn get_cols_bram(rd: &Part, int: &IntGrid) -> Vec<u32> {
    find_columns(rd, &["LBRAM", "RBRAM", "MBRAM", "MBRAMS2E"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .sorted()
        .collect()
}

fn get_cols_clkv(rd: &Part, int: &IntGrid) -> Vec<(u32, u32)> {
    let cols_clkv: Vec<_> = find_columns(rd, &["LBRAM", "RBRAM", "GCLKV", "CLKV"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .sorted()
        .collect();
    let mut cols_brk: Vec<_> = find_columns(rd, &["GBRKV"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .sorted()
        .collect();
    cols_brk.push(int.cols.len() as u32);
    assert_eq!(cols_clkv.len(), cols_brk.len());
    cols_clkv.into_iter().zip(cols_brk.into_iter()).collect()
}

fn add_disabled_dlls(disabled: &mut BTreeSet<DisabledPart>, rd: &Part) {
    let c = Coord {
        x: rd.width / 2,
        y: 0,
    };
    let t = &rd.tiles[&c];
    if t.kind == "CLKB_2DLL" {
        disabled.insert(DisabledPart::VirtexPrimaryDlls);
    }
}

fn add_disabled_brams(disabled: &mut BTreeSet<DisabledPart>, rd: &Part, int: &IntGrid) {
    for c in find_columns(rd, &["MBRAMS2E"]) {
        disabled.insert(DisabledPart::VirtexBram(int.lookup_column_inter(c)));
    }
}

fn handle_spec_io(rd: &Part, grid: &mut virtex::Grid) {
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    let mut novref = BTreeSet::new();
    for pins in rd.packages.values() {
        for pin in pins {
            if let Some(ref pad) = pin.pad {
                if pad.starts_with("GCLK") {
                    continue;
                }
                let coord = io_lookup[pad];
                let mut func = &pin.func[..];
                if let Some(pos) = func.find("_L") {
                    func = &func[..pos];
                }
                if func.starts_with("IO_VREF_") {
                    grid.vref.insert(coord);
                } else {
                    novref.insert(coord);
                    let cfg = match func {
                        "IO" => continue,
                        "IO_DIN_D0" => CfgPin::Data(0),
                        "IO_D1" => CfgPin::Data(1),
                        "IO_D2" => CfgPin::Data(2),
                        "IO_D3" => CfgPin::Data(3),
                        "IO_D4" => CfgPin::Data(4),
                        "IO_D5" => CfgPin::Data(5),
                        "IO_D6" => CfgPin::Data(6),
                        "IO_D7" => CfgPin::Data(7),
                        "IO_CS" => CfgPin::CsiB,
                        "IO_INIT" => CfgPin::InitB,
                        "IO_WRITE" => CfgPin::RdWrB,
                        "IO_DOUT_BUSY" => CfgPin::Dout,
                        "IO_IRDY" => {
                            assert_eq!(coord.bel, 3);
                            assert_eq!(coord.row, grid.rows / 2);
                            continue;
                        }
                        "IO_TRDY" => {
                            assert_eq!(coord.bel, 1);
                            assert_eq!(coord.row, grid.rows / 2 - 1);
                            continue;
                        }
                        _ => panic!("UNK FUNC {func} {coord:?}"),
                    };
                    let old = grid.cfg_io.insert(cfg, coord);
                    assert!(old.is_none() || old == Some(coord));
                }
            }
        }
    }
    for c in novref {
        assert!(!grid.vref.contains(&c));
    }
}

fn make_int_db(rd: &Part) -> int::IntDb {
    let mut builder = IntBuilder::new("virtex", rd);
    builder.node_type("CENTER", "CLB", "NODE.CLB");
    builder.node_type("LEFT", "IO.L", "NODE.IO.L");
    builder.node_type("LEFT_PCI_BOT", "IO.L", "NODE.IO.L");
    builder.node_type("LEFT_PCI_TOP", "IO.L", "NODE.IO.L");
    builder.node_type("RIGHT", "IO.R", "NODE.IO.R");
    builder.node_type("RIGHT_PCI_BOT", "IO.R", "NODE.IO.R");
    builder.node_type("RIGHT_PCI_TOP", "IO.R", "NODE.IO.R");
    builder.node_type("BOT", "IO.B", "NODE.IO.B");
    builder.node_type("BL_DLLIOB", "IO.B", "NODE.IO.B");
    builder.node_type("BR_DLLIOB", "IO.B", "NODE.IO.B");
    builder.node_type("TOP", "IO.T", "NODE.IO.T");
    builder.node_type("TL_DLLIOB", "IO.T", "NODE.IO.T");
    builder.node_type("TR_DLLIOB", "IO.T", "NODE.IO.T");
    builder.node_type("LL", "CNR.BL", "NODE.CNR.BL");
    builder.node_type("LR", "CNR.BR", "NODE.CNR.BR");
    builder.node_type("UL", "CNR.TL", "NODE.CNR.TL");
    builder.node_type("UR", "CNR.TR", "NODE.CNR.TR");

    for i in 0..4 {
        let w = builder.wire(format!("GCLK{i}"), int::WireKind::ClkOut(i), &[
            format!("GCLK{i}"),
            format!("LEFT_GCLK{i}"),
            format!("RIGHT_GCLK{i}"),
            format!("BOT_HGCLK{i}"),
            format!("TOP_HGCLK{i}"),
            format!("LL_GCLK{i}"),
            format!("UL_GCLK{i}"),
        ]);
        builder.buf(w, format!("GCLK{i}.BUF"), &[
            format!("BOT_GCLK{i}"),
            format!("TOP_GCLK{i}"),
        ]);
    }
    builder.wire("PCI_CE", int::WireKind::ClkOut(4), &[
        "LEFT_PCI_CE",
        "RIGHT_PCI_CE",
        "LL_PCI_CE",
        "LR_PCI_CE",
        "UL_PCI_CE",
        "UR_PCI_CE",
    ]);

    for i in 0..24 {
        let w = builder.wire(format!("SINGLE.E{i}"), int::WireKind::PipOut, &[
            format!("E{i}"),
            format!("LEFT_E{i}"),
        ]);
        builder.buf(w, format!("SINGLE.E{i}.BUF"), &[
            format!("E_P{i}"),
            format!("LEFT_E_BUF{i}"),
        ]);
        let w = builder.pip_branch(w, Dir::E, format!("SINGLE.W{i}"), &[
            format!("W{i}"),
            format!("RIGHT_W{i}"),
        ]);
        builder.buf(w, format!("SINGLE.W{i}.BUF"), &[
            format!("W_P{i}"),
            format!("RIGHT_W_BUF{i}"),
        ]);
    }
    for i in 0..24 {
        let w = builder.wire(format!("SINGLE.S{i}"), int::WireKind::PipOut, &[
            format!("S{i}"),
            format!("TOP_S{i}"),
        ]);
        builder.buf(w, format!("SINGLE.S{i}.BUF"), &[
            format!("S_P{i}"),
            format!("TOP_S_BUF{i}"),
        ]);
        let w = builder.pip_branch(w, Dir::N, format!("SINGLE.N{i}"), &[
            format!("N{i}"),
            format!("BOT_N{i}"),
        ]);
        builder.buf(w, format!("SINGLE.N{i}.BUF"), &[
            format!("N_P{i}"),
            format!("BOT_N_BUF{i}"),
        ]);
    }

    let hexnames = |pref, i| [
        format!("{pref}{i}"),
        format!("LEFT_{pref}{i}"),
        format!("RIGHT_{pref}{i}"),
        format!("TOP_{pref}{i}"),
        format!("BOT_{pref}{i}"),
        format!("LL_{pref}{i}"),
        format!("LR_{pref}{i}"),
        format!("UL_{pref}{i}"),
        format!("UR_{pref}{i}"),
    ];
    let hexnames_hc = |pref, i| [
        format!("{pref}{i}"),
        format!("LEFT_{pref}{i}"),
        format!("RIGHT_{pref}{i}"),
    ];
    let hexnames_hio = |pref, i| [
        format!("TOP_{pref}{i}"),
        format!("BOT_{pref}{i}"),
        format!("LL_{pref}{i}"),
        format!("LR_{pref}{i}"),
        format!("UL_{pref}{i}"),
        format!("UR_{pref}{i}"),
    ];
    for i in 0..4 {
        let m = builder.multi_out(format!("HEX.H{i}.3"), &hexnames("H6M", i));
        let b = builder.multi_branch(m, Dir::W, format!("HEX.H{i}.2"), &hexnames("H6B", i));
        let a = builder.multi_branch(b, Dir::W, format!("HEX.H{i}.1"), &hexnames("H6A", i));
        let e = builder.multi_branch(a, Dir::W, format!("HEX.H{i}.0"), &hexnames("H6E", i));
        let c = builder.multi_branch(m, Dir::E, format!("HEX.H{i}.4"), &hexnames("H6C", i));
        let d = builder.multi_branch(c, Dir::E, format!("HEX.H{i}.5"), &hexnames("H6D", i));
        let w = builder.multi_branch(d, Dir::E, format!("HEX.H{i}.6"), &hexnames("H6W", i));
        builder.buf(e, format!("HEX.H{i}.0.BUF"), &hexnames("H6E_BUF", i));
        builder.buf(a, format!("HEX.H{i}.1.BUF"), &hexnames("H6A_BUF", i));
        builder.buf(b, format!("HEX.H{i}.2.BUF"), &hexnames("H6B_BUF", i));
        builder.buf(m, format!("HEX.H{i}.3.BUF"), &hexnames("H6M_BUF", i));
        builder.buf(c, format!("HEX.H{i}.4.BUF"), &hexnames("H6C_BUF", i));
        builder.buf(d, format!("HEX.H{i}.5.BUF"), &hexnames("H6D_BUF", i));
        builder.buf(w, format!("HEX.H{i}.6.BUF"), &hexnames("H6W_BUF", i));
    }
    for i in 4..6 {
        let m = builder.multi_out(format!("HEX.H{i}.3"), &hexnames_hio("H6M", i));
        let b = builder.multi_branch(m, Dir::W, format!("HEX.H{i}.2"), &hexnames_hio("H6B", i));
        let a = builder.multi_branch(b, Dir::W, format!("HEX.H{i}.1"), &hexnames_hio("H6A", i));
        builder.multi_branch(a, Dir::W, format!("HEX.H{i}.0"), &hexnames_hio("H6E", i));
        let c = builder.multi_branch(m, Dir::E, format!("HEX.H{i}.4"), &hexnames_hio("H6C", i));
        let d = builder.multi_branch(c, Dir::E, format!("HEX.H{i}.5"), &hexnames_hio("H6D", i));
        builder.multi_branch(d, Dir::E, format!("HEX.H{i}.6"), &hexnames_hio("H6W", i));
    }
    for i in 0..4 {
        let ii = 4 + i * 2;
        let w = builder.mux_out(format!("HEX.W{i}.6"), &hexnames_hc("H6W", ii));
        let d = builder.branch(w, Dir::W, format!("HEX.W{i}.5"), &hexnames_hc("H6D", ii));
        let c = builder.branch(d, Dir::W, format!("HEX.W{i}.4"), &hexnames_hc("H6C", ii));
        let m = builder.branch(c, Dir::W, format!("HEX.W{i}.3"), &hexnames_hc("H6M", ii));
        let b = builder.branch(m, Dir::W, format!("HEX.W{i}.2"), &hexnames_hc("H6B", ii));
        let a = builder.branch(b, Dir::W, format!("HEX.W{i}.1"), &hexnames_hc("H6A", ii));
        builder.branch(a, Dir::W, format!("HEX.W{i}.0"), &hexnames_hc("H6E", ii));
    }
    for i in 0..4 {
        let ii = 5 + i * 2;
        let e = builder.mux_out(format!("HEX.E{i}.0"), &hexnames_hc("H6E", ii));
        let a = builder.branch(e, Dir::E, format!("HEX.E{i}.1"), &hexnames_hc("H6A", ii));
        let b = builder.branch(a, Dir::E, format!("HEX.E{i}.2"), &hexnames_hc("H6B", ii));
        let m = builder.branch(b, Dir::E, format!("HEX.E{i}.3"), &hexnames_hc("H6M", ii));
        let c = builder.branch(m, Dir::E, format!("HEX.E{i}.4"), &hexnames_hc("H6C", ii));
        let d = builder.branch(c, Dir::E, format!("HEX.E{i}.5"), &hexnames_hc("H6D", ii));
        builder.branch(d, Dir::E, format!("HEX.E{i}.6"), &hexnames_hc("H6W", ii));
    }
    for i in 0..4 {
        let m = builder.multi_out(format!("HEX.V{i}.3"), &hexnames("V6M", i));
        let b = builder.branch(m, Dir::S, format!("HEX.V{i}.2"), &hexnames("V6B", i));
        let a = builder.branch(b, Dir::S, format!("HEX.V{i}.1"), &hexnames("V6A", i));
        let n = builder.branch(a, Dir::S, format!("HEX.V{i}.0"), &hexnames("V6N", i));
        let c = builder.branch(m, Dir::N, format!("HEX.V{i}.4"), &hexnames("V6C", i));
        let d = builder.branch(c, Dir::N, format!("HEX.V{i}.5"), &hexnames("V6D", i));
        let s = builder.branch(d, Dir::N, format!("HEX.V{i}.6"), &hexnames("V6S", i));
        builder.buf(n, format!("HEX.V{i}.0.BUF"), &hexnames("V6N_BUF", i));
        builder.buf(a, format!("HEX.V{i}.1.BUF"), &hexnames("V6A_BUF", i));
        builder.buf(b, format!("HEX.V{i}.2.BUF"), &hexnames("V6B_BUF", i));
        builder.buf(m, format!("HEX.V{i}.3.BUF"), &hexnames("V6M_BUF", i));
        builder.buf(c, format!("HEX.V{i}.4.BUF"), &hexnames("V6C_BUF", i));
        builder.buf(d, format!("HEX.V{i}.5.BUF"), &hexnames("V6D_BUF", i));
        builder.buf(s, format!("HEX.V{i}.6.BUF"), &hexnames("V6S_BUF", i));
    }
    for i in 0..4 {
        let ii = 4 + i * 2;
        let s = builder.mux_out(format!("HEX.S{i}.6"), &hexnames("V6S", ii));
        let d = builder.branch(s, Dir::S, format!("HEX.S{i}.5"), &hexnames("V6D", ii));
        let c = builder.branch(d, Dir::S, format!("HEX.S{i}.4"), &hexnames("V6C", ii));
        let m = builder.branch(c, Dir::S, format!("HEX.S{i}.3"), &hexnames("V6M", ii));
        let b = builder.branch(m, Dir::S, format!("HEX.S{i}.2"), &hexnames("V6B", ii));
        let a = builder.branch(b, Dir::S, format!("HEX.S{i}.1"), &hexnames("V6A", ii));
        builder.branch(a, Dir::S, format!("HEX.S{i}.0"), &hexnames("V6N", ii));
    }
    for i in 0..4 {
        let ii = 5 + i * 2;
        let n = builder.mux_out(format!("HEX.N{i}.0"), &hexnames("V6N", ii));
        let a = builder.branch(n, Dir::N, format!("HEX.N{i}.1"), &hexnames("V6A", ii));
        let b = builder.branch(a, Dir::N, format!("HEX.N{i}.2"), &hexnames("V6B", ii));
        let m = builder.branch(b, Dir::N, format!("HEX.N{i}.3"), &hexnames("V6M", ii));
        let c = builder.branch(m, Dir::N, format!("HEX.N{i}.4"), &hexnames("V6C", ii));
        let d = builder.branch(c, Dir::N, format!("HEX.N{i}.5"), &hexnames("V6D", ii));
        builder.branch(d, Dir::N, format!("HEX.N{i}.6"), &hexnames("V6S", ii));
    }

    let lh: Vec<_> = (0..12).map(|i| builder.wire(format!("LH.{i}"), int::WireKind::MultiBranch(Dir::W), &[
        format!("LH{i}"),
        format!("LEFT_LH{i}"),
        format!("RIGHT_LH{i}"),
        format!("BOT_LH{i}"),
        format!("TOP_LH{i}"),
        format!("LL_LH{i}"),
        format!("LR_LH{i}"),
        format!("UL_LH{i}"),
        format!("UR_LH{i}"),
    ])).collect();
    for i in 0..12 {
        builder.conn_branch(lh[i], Dir::E, lh[(i + 11) % 12]);
    }
    builder.buf(lh[0], "LH.0.FAKE", &["TOP_FAKE_LH0", "BOT_FAKE_LH0"]);
    builder.buf(lh[6], "LH.6.FAKE", &["TOP_FAKE_LH6", "BOT_FAKE_LH6"]);

    let lv: Vec<_> = (0..12).map(|i| builder.wire(format!("LV.{i}"), int::WireKind::MultiBranch(Dir::S), &[
        format!("LV{i}"),
        format!("LEFT_LV{i}"),
        format!("RIGHT_LV{i}"),
        format!("BOT_LV{i}"),
        format!("TOP_LV{i}"),
        format!("LL_LV{i}"),
        format!("LR_LV{i}"),
        format!("UL_LV{i}"),
        format!("UR_LV{i}"),
    ])).collect();
    for i in 0..12 {
        builder.conn_branch(lv[i], Dir::N, lv[(i + 11) % 12]);
    }

    for i in 0..2 {
        for pin in ["CLK", "SR", "CE", "BX", "BY"] { 
            builder.mux_out(format!("IMUX.S{i}.{pin}"), &[
                format!("S{i}_{pin}_B"),
            ]);
        }
        for fg in ['F', 'G'] {
            for j in 1..5 {
                builder.mux_out(format!("IMUX.S{i}.{fg}{j}"), &[
                    format!("S{i}_{fg}_B{j}"),
                ]);
            }
        }
    }
    for i in 0..2 {
        builder.mux_out(format!("IMUX.TBUF{i}.T"), &[
            format!("TS_B{i}"),
            format!("LEFT_TS{i}_B"),
            format!("RIGHT_TS{i}_B"),
        ]);
        builder.mux_out(format!("IMUX.TBUF{i}.I"), &[
            format!("T_IN{i}"),
            format!("LEFT_TI{i}_B"),
            format!("RIGHT_TI{i}_B"),
        ]);
    }
    for i in 0..4 {
        for pin in ["CLK", "SR", "ICE", "OCE", "TCE", "O", "T"] {
            let np = if pin == "SR" {"SR_B"} else {pin};
            builder.mux_out(format!("IMUX.IO{i}.{pin}"), &[
                format!("LEFT_{np}{i}"),
                format!("RIGHT_{np}{i}"),
                format!("BOT_{np}{i}"),
                format!("TOP_{np}{i}"),
            ]);
        }
    }
    builder.mux_out("IMUX.CAP.CLK", &["LL_CAPTURE_CLK"]);
    builder.mux_out("IMUX.CAP.CAP", &["LL_CAP"]);
    builder.mux_out("IMUX.STARTUP.CLK", &["UL_STARTUP_CLK"]);
    builder.mux_out("IMUX.STARTUP.GSR", &["UL_GSR"]);
    builder.mux_out("IMUX.STARTUP.GTS", &["UL_GTS"]);
    builder.mux_out("IMUX.STARTUP.GWE", &["UL_GWE"]);
    builder.mux_out("IMUX.BSCAN.TDO1", &["UL_TDO1"]);
    builder.mux_out("IMUX.BSCAN.TDO2", &["UL_TDO2"]);

    for i in 0..8 {
        let w = builder.mux_out(format!("OMUX{i}"), &[
            format!("OUT{i}"),
            format!("LEFT_OUT{i}"),
            format!("RIGHT_OUT{i}"),
        ]);
        if matches!(i, 0 | 1) {
            builder.branch(w, Dir::E, format!("OMUX{i}.W"), &[
                format!("OUT_W{i}"),
                format!("RIGHT_OUT_W{i}"),
            ]);
        }
        if matches!(i, 6 | 7) {
            builder.branch(w, Dir::W, format!("OMUX{i}.E"), &[
                format!("OUT_E{i}"),
                format!("LEFT_OUT_E{i}"),
            ]);
        }
    }

    for i in 0..2 {
        for pin in ["X", "Y", "XQ", "YQ", "XB", "YB"] {
            builder.logic_out(format!("OUT.S{i}.{pin}"), &[
                format!("S{i}_{pin}"),
            ]);
        }
    }
    builder.logic_out("OUT.TBUF", &[
        "TBUFO",
    ]);
    for i in 0..4 {
        builder.logic_out(format!("OUT.TBUF.L{i}"), &[
            format!("LEFT_TBUFO{i}"),
        ]);
    }
    for i in 0..4 {
        builder.logic_out(format!("OUT.TBUF.R{i}"), &[
            format!("RIGHT_TBUFO{i}"),
        ]);
    }
    for i in 0..4 {
        for pin in ["I", "IQ"] {
            builder.logic_out(format!("OUT.IO{i}.{pin}"), &[
                format!("LEFT_{pin}{i}"),
                format!("RIGHT_{pin}{i}"),
                format!("BOT_{pin}{i}"),
                format!("TOP_{pin}{i}"),
            ]);
        }
    }
    for i in 0..2 {
        for pin in ["X", "Y", "XQ", "YQ", "XB", "YB"] {
            builder.logic_out(format!("OUT.S{i}.{pin}"), &[
                format!("S{i}_{pin}"),
            ]);
        }
    }
    for pin in ["RESET", "DRCK1", "DRCK2", "SHIFT", "TDI", "UPDATE", "SEL1", "SEL2"] {
        builder.logic_out(format!("OUT.BSCAN.{pin}"), &[
            format!("UL_{pin}"),
        ]);
    }

    builder.extract_nodes();

    // XXX BRAM
    // XXX CLK[BT]
    // XXX DLLs
    // XXX CLK[LR]

    builder.build()
}

fn make_grid(rd: &Part) -> (virtex::Grid, BTreeSet<DisabledPart>) {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &[
        "CENTER",
        "LL",
        "LR",
        "UL",
        "UR",
    ], &[]);
    let kind = get_kind(rd);
    let mut disabled = BTreeSet::new();
    add_disabled_dlls(&mut disabled, rd);
    add_disabled_brams(&mut disabled, rd, &int);
    let mut grid = virtex::Grid {
        kind,
        columns: int.cols.len() as u32,
        cols_bram: get_cols_bram(&rd, &int),
        cols_clkv: get_cols_clkv(&rd, &int),
        rows: int.rows.len() as u32,
        vref: BTreeSet::new(),
        cfg_io: BTreeMap::new(),
    };
    handle_spec_io(rd, &mut grid);
    (grid, disabled)
}

fn make_bond(grid: &virtex::Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, (io.coord, io.bank)))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if pad.starts_with("GCLKPAD") {
                let bank = match &pad[..] {
                    "GCLKPAD0" => 4,
                    "GCLKPAD1" => 5,
                    "GCLKPAD2" => 1,
                    "GCLKPAD3" => 0,
                    _ => panic!("unknown pad {}", pad),
                };
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByBank(bank, 0)
            } else {
                let (coord, bank) = io_lookup[pad];
                assert_eq!(pin.vref_bank, Some(bank));
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByCoord(coord)
            }
        } else if pin.func.starts_with("VCCO_") {
            let bank = pin.func[5..].parse().unwrap();
            BondPin::VccO(bank)
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VCCO" => BondPin::VccO(0),
                "TCK" => BondPin::Cfg(CfgPin::Tck),
                "TDI" => BondPin::Cfg(CfgPin::Tdi),
                "TDO" => BondPin::Cfg(CfgPin::Tdo),
                "TMS" => BondPin::Cfg(CfgPin::Tms),
                "CCLK" => BondPin::Cfg(CfgPin::Cclk),
                "DONE" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM" => BondPin::Cfg(CfgPin::ProgB),
                "M0" => BondPin::Cfg(CfgPin::M0),
                "M1" => BondPin::Cfg(CfgPin::M1),
                "M2" => BondPin::Cfg(CfgPin::M2),
                "DXN" => BondPin::Dxn,
                "DXP" => BondPin::Dxp,
                _ => panic!("UNK FUNC {}", pin.func),
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks,
    }
}

pub fn ingest(rd: &Part) -> (PreDevice, Option<int::IntDb>) {
    let (grid, disabled) = make_grid(rd);
    let int_db = make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(&grid, pins),
        ));
    }
    (make_device(rd, geom::Grid::Virtex(grid), bonds, disabled), Some(int_db))
}
