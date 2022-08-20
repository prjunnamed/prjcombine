use std::collections::{BTreeMap, BTreeSet, HashMap};
use prjcombine_xilinx_rawdump::{Part, Coord, PkgPin};
use prjcombine_xilinx_geom::{self as geom, DisabledPart, CfgPin, Bond, BondPin, ColId, int, int::Dir};
use prjcombine_xilinx_geom::virtex::{self, GridKind};
use prjcombine_entity::EntityId;

use crate::grid::{extract_int, find_columns, IntGrid, PreDevice, make_device};
use crate::intb::IntBuilder;
use crate::verify::Verifier;

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

fn get_cols_bram(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    find_columns(rd, &["LBRAM", "RBRAM", "MBRAM", "MBRAMS2E"])
        .into_iter()
        .map(|r| int.lookup_column(r))
        .collect()
}

fn get_cols_clkv(rd: &Part, int: &IntGrid) -> Vec<(ColId, ColId)> {
    let mut cols_clkv: Vec<_> = find_columns(rd, &["GCLKV", "CLKV"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .collect();
    cols_clkv.insert(0, int.cols.first_id().unwrap() + 2);
    cols_clkv.push(int.cols.last_id().unwrap() - 1);
    let mut cols_brk: Vec<_> = find_columns(rd, &["GBRKV"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .collect();
    cols_brk.push(int.cols.next_id());
    assert_eq!(cols_clkv.len(), cols_brk.len());
    cols_clkv.into_iter().zip(cols_brk.into_iter()).collect()
}

fn add_disabled_dlls(disabled: &mut BTreeSet<DisabledPart>, rd: &Part) {
    let c = Coord {
        x: rd.width / 2,
        y: 0,
    };
    let t = &rd.tiles[&c];
    if rd.tile_kinds.key(t.kind) == "CLKB_2DLL" {
        disabled.insert(DisabledPart::VirtexPrimaryDlls);
    }
}

fn add_disabled_brams(disabled: &mut BTreeSet<DisabledPart>, rd: &Part, int: &IntGrid) {
    for c in find_columns(rd, &["MBRAMS2E"]) {
        disabled.insert(DisabledPart::VirtexBram(int.lookup_column(c)));
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
                            assert_eq!(coord.row, grid.row_mid());
                            continue;
                        }
                        "IO_TRDY" => {
                            assert_eq!(coord.bel, 1);
                            assert_eq!(coord.row, grid.row_mid() - 1);
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

    let mut bram_forbidden = Vec::new();
    let mut bram_bt_forbidden = Vec::new();
    let mut dll_forbidden = Vec::new();
    let mut bram_extra_pips = BTreeMap::new();

    let mut gclk = Vec::new();
    for i in 0..4 {
        let w = builder.wire(format!("GCLK{i}"), int::WireKind::ClkOut(i), &[
            format!("GCLK{i}"),
            format!("LEFT_GCLK{i}"),
            format!("RIGHT_GCLK{i}"),
            format!("BOT_HGCLK{i}"),
            format!("TOP_HGCLK{i}"),
            format!("LL_GCLK{i}"),
            format!("UL_GCLK{i}"),
            format!("BRAM_GCLKIN{i}"),
            format!("BRAM_BOT_GCLKE{i}"),
            format!("BRAM_TOP_GCLKE{i}"),
            format!("BRAM_BOTP_GCLK{i}"),
            format!("BRAM_TOPP_GCLK{i}"),
            format!("BRAM_BOTS_GCLK{i}"),
            format!("BRAM_TOPS_GCLK{i}"),
        ]);
        builder.extra_name_sub(format!("MBRAM_GCLKD{i}"), 0, w);
        builder.extra_name_sub(format!("MBRAM_GCLKA{i}"), 3, w);
        gclk.push(w);
        bram_forbidden.push(w);
        bram_bt_forbidden.push(w);
        dll_forbidden.push(w);
        builder.buf(w, format!("GCLK{i}.BUF"), &[
            format!("BOT_GCLK{i}"),
            format!("TOP_GCLK{i}"),
        ]);
    }

    let pci_ce = builder.wire("PCI_CE", int::WireKind::MultiBranch(Dir::S), &[
        "LEFT_PCI_CE",
        "RIGHT_PCI_CE",
        "LL_PCI_CE",
        "LR_PCI_CE",
        "UL_PCI_CE",
        "UR_PCI_CE",
    ]);
    builder.conn_branch(pci_ce, Dir::N, pci_ce);

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
        let w = builder.pip_branch(w, Dir::S, format!("SINGLE.N{i}"), &[
            format!("N{i}"),
            format!("BOT_N{i}"),
        ]);
        builder.buf(w, format!("SINGLE.N{i}.BUF"), &[
            format!("N_P{i}"),
            format!("BOT_N_BUF{i}"),
        ]);
    }

    let def_t = int::NodeTileId::from_idx(0);
    for name in ["ADDR", "DIN", "DOUT"] {
        let mut l = Vec::new();
        let mut ln = Vec::new();
        for i in 0..32 {
            let w = builder.mux_out(format!("BRAM.SINGLE.{name}{i}"), &[
                format!("BRAM_R{name}S{i}"),
            ]);
            let s = builder.branch(w, Dir::S, format!("BRAM.SINGLE.{name}{i}.S"), &[
                format!("BRAM_R{name}N{i}"),
            ]);
            bram_forbidden.push(s);
            let n = builder.branch(w, Dir::N, format!("BRAM.SINGLE.{name}{i}.n"), &[""]);
            l.push(w);
            ln.push(n);
        }
        for i in 0..32 {
            let si;
            if name == "ADDR" {
                si = i;
            } else {
                si = i & 0x10 | (i + 0xf) & 0xf;
            }
            bram_extra_pips.insert(((def_t, l[i]), (def_t, ln[si])), int::NodeExtPipNaming {
                tile: int::NodeRawTileId::from_idx(1),
                wire_to: format!("BRAM_R{name}N{i}"),
                wire_from: format!("BRAM_R{name}S{si}"),
            });
        }
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
        format!("BRAM_LV{i}"),
        format!("BRAM_BOT_RLV{ii}", ii = (i + 11) % 12),
        format!("BRAM_BOTP_RLV{ii}", ii = (i + 11) % 12),
        format!("BRAM_TOP_RLV{i}"),
        format!("BRAM_TOPP_RLV{i}"),
    ])).collect();
    for i in 0..12 {
        builder.conn_branch(lv[i], Dir::N, lv[(i + 11) % 12]);
        dll_forbidden.push(lv[i]);
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

    for ab in ['A', 'B'] {
        for i in 0..16 {
            builder.mux_out(format!("OUT.BRAM.DI{ab}{i}"), &[
                format!("BRAM_DI{ab}{i}"),
            ]);
        }
    }
    for ab in ['A', 'B'] {
        for i in 0..12 {
            builder.mux_out(format!("OUT.BRAM.ADDR{ab}{i}"), &[
                format!("BRAM_ADDR{ab}{i}"),
            ]);
        }
    }
    for name in ["CLK", "RST", "SEL", "WE"] {
        for ab in ['A', 'B'] {
            builder.mux_out(format!("OUT.BRAM.{name}{ab}"), &[
                format!("BRAM_{name}{ab}"),
                format!("MBRAM_{name}{ab}"),
            ]);
        }
    }

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
    for pin in ["RESET", "DRCK1", "DRCK2", "SHIFT", "TDI", "UPDATE", "SEL1", "SEL2"] {
        builder.logic_out(format!("OUT.BSCAN.{pin}"), &[
            format!("UL_{pin}"),
        ]);
    }

    for ab in ['A', 'B'] {
        for i in 0..16 {
            builder.logic_out(format!("OUT.BRAM.DO{ab}{i}"), &[
                format!("BRAM_DO{ab}{i}"),
            ]);
        }
    }

    for i in 0..2 {
        builder.mux_out(format!("CLK.IMUX.BUFGCE.CLK{i}"), &[
            format!("CLKB_GCLKBUF{i}_IN"),
            format!("CLKT_GCLKBUF{ii}_IN", ii = i + 2),
        ]);
    }
    for i in 0..2 {
        builder.mux_out(format!("CLK.IMUX.BUFGCE.CE{i}"), &[
            format!("CLKB_CE{i}"),
            format!("CLKT_CE{i}"),
        ]);
    }
    for i in 0..2 {
        builder.logic_out(format!("CLK.OUT.BUFGCE.O{i}"), &[
            format!("CLKB_GCLK{i}_PW"),
            format!("CLKT_GCLK{ii}_PW", ii = i + 2),
        ]);
    }
    let mut clkpad = Vec::new();
    for i in 0..2 {
        let w = builder.logic_out(format!("CLK.OUT.CLKPAD{i}"), &[
            format!("CLKB_CLKPAD{i}"),
            format!("CLKT_CLKPAD{i}"),
        ]);
        clkpad.push(w);
    }
    let mut iofb = Vec::new();
    for i in 0..2 {
        let w = builder.logic_out(format!("CLK.OUT.IOFB{i}"), &[
            format!("CLKB_IOFB{i}"),
            format!("CLKT_IOFB{i}"),
        ]);
        iofb.push(w);
    }
    for i in 1..4 {
        builder.mux_out(format!("PCI.IMUX.I{i}"), &[
            format!("CLKL_I{i}"),
            format!("CLKR_I{i}"),
        ]);
    }
    let mut dll_ins = Vec::new();
    let mut clkin = None;
    let mut clkfb = None;
    for name in [
        "CLKIN",
        "CLKFB",
        "RST",
    ] {
        let w = builder.mux_out(format!("DLL.IMUX.{name}"), &[
            format!("BRAM_BOT_{name}"),
            format!("BRAM_BOTP_{name}"),
            format!("BRAM_BOT_{name}_1"),
            format!("BRAM_TOP_{name}"),
            format!("BRAM_TOPP_{name}"),
            format!("BRAM_TOPS_{name}"),
        ]);
        builder.extra_name_sub(format!("CLKB_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKB_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKB_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKB_{name}R_1"), 4, w);
        builder.extra_name_sub(format!("CLKT_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKT_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKT_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKT_{name}R_1"), 4, w);
        dll_ins.push(w);
        bram_bt_forbidden.push(w);
        if name == "CLKIN" {
            clkin = Some(w);
        }
        if name == "CLKFB" {
            clkfb = Some(w);
        }
    }
    let clkin = clkin.unwrap();
    let clkfb = clkfb.unwrap();
    let mut clk2x = None;
    for name in [
        "CLK0",
        "CLK90",
        "CLK180",
        "CLK270",
        "CLK2X",
        "CLK2X90",
        "CLKDV",
        "LOCKED",
    ] {
        let w = builder.logic_out(format!("DLL.OUT.{name}"), &[""]);
        builder.extra_name_sub(format!("CLKB_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKB_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKB_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKB_{name}R_1"), 4, w);
        builder.extra_name_sub(format!("CLKT_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKT_{name}R"), 2, w);
        if name == "LOCKED" {
            builder.extra_name_sub(format!("CLKT_LOCK_TL_1"), 3, w);
        } else {
            builder.extra_name_sub(format!("CLKT_{name}L_1"), 3, w);
        }
        builder.extra_name_sub(format!("CLKT_{name}R_1"), 4, w);
        if name == "CLK2X" {
            clk2x = Some(w);
        }
    }
    let clk2x = clk2x.unwrap();

    builder.extract_main_passes();

    builder.node_type("CENTER", "CLB", "CLB");
    builder.node_type("LEFT", "IO.L", "IO.L");
    builder.node_type("LEFT_PCI_BOT", "IO.L", "IO.L");
    builder.node_type("LEFT_PCI_TOP", "IO.L", "IO.L");
    builder.node_type("RIGHT", "IO.R", "IO.R");
    builder.node_type("RIGHT_PCI_BOT", "IO.R", "IO.R");
    builder.node_type("RIGHT_PCI_TOP", "IO.R", "IO.R");
    builder.node_type("BOT", "IO.B", "IO.B");
    builder.node_type("BL_DLLIOB", "IO.B", "IO.B");
    builder.node_type("BR_DLLIOB", "IO.B", "IO.B");
    builder.node_type("TOP", "IO.T", "IO.T");
    builder.node_type("TL_DLLIOB", "IO.T", "IO.T");
    builder.node_type("TR_DLLIOB", "IO.T", "IO.T");
    builder.node_type("LL", "CNR.BL", "CNR.BL");
    builder.node_type("LR", "CNR.BR", "CNR.BR");
    builder.node_type("UL", "CNR.TL", "CNR.TL");
    builder.node_type("UR", "CNR.TR", "CNR.TR");

    for tkn in [
        "LBRAM", "RBRAM", "MBRAM",
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut x = xy.x - 1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&(x as i32)) {
                x -= 1;
            }
            let mut coords = Vec::new();
            for dy in 0..4 {
                coords.push(Coord { x: xy.x, y: xy.y + dy });
            }
            for dy in 0..4 {
                coords.push(Coord { x, y: xy.y + dy });
            }
            builder.extract_xnode(tkn, xy, &coords, tkn, &bram_forbidden);
        }
        if let Some((_, n)) = builder.db.nodes.get_mut(tkn) {
            let (_, naming) = builder.db.node_namings.get_mut(tkn).unwrap();
            for (&k, v) in &bram_extra_pips {
                let (wt, wf) = k;
                n.muxes.get_mut(&wt).unwrap().ins.insert(wf);
                naming.ext_pips.insert(k, v.clone());
            }
        }
    }

    for (tkn, node, naming) in [
        ("BRAM_BOT", "BRAM_BOT", "BRAM_BOT.BOT"),
        ("BRAM_BOT_GCLK", "BRAM_BOT", "BRAM_BOT.BOT"),
        ("LBRAM_BOTS_GCLK", "BRAM_BOT", "BRAM_BOT.BOT"),
        ("RBRAM_BOTS_GCLK", "BRAM_BOT", "BRAM_BOT.BOT"),
        ("LBRAM_BOTS", "BRAM_BOT", "BRAM_BOT.BOT"),
        ("RBRAM_BOTS", "BRAM_BOT", "BRAM_BOT.BOT"),
        ("BRAM_BOT_NOGCLK", "BRAM_BOT", "BRAM_BOT.BOTP"),
        ("BRAMS2E_BOT_NOGCLK", "BRAM_BOT", "BRAM_BOT.BOTP"),
        ("LBRAM_BOTP", "BRAM_BOT", "BRAM_BOT.BOTP"),
        ("RBRAM_BOTP", "BRAM_BOT", "BRAM_BOT.BOTP"),
        ("BRAM_TOP", "BRAM_TOP", "BRAM_TOP.TOP"),
        ("BRAM_TOP_GCLK", "BRAM_TOP", "BRAM_TOP.TOP"),
        ("LBRAM_TOPS_GCLK", "BRAM_TOP", "BRAM_TOP.TOP"),
        ("RBRAM_TOPS_GCLK", "BRAM_TOP", "BRAM_TOP.TOP"),
        ("LBRAM_TOPS", "BRAM_TOP", "BRAM_TOP.TOP"),
        ("RBRAM_TOPS", "BRAM_TOP", "BRAM_TOP.TOP"),
        ("BRAM_TOP_NOGCLK", "BRAM_TOP", "BRAM_TOP.TOPP"),
        ("BRAMS2E_TOP_NOGCLK", "BRAM_TOP", "BRAM_TOP.TOPP"),
        ("LBRAM_TOPP", "BRAM_TOP", "BRAM_TOP.TOPP"),
        ("RBRAM_TOPP", "BRAM_TOP", "BRAM_TOP.TOPP"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut x = xy.x - 1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&(x as i32)) {
                x -= 1;
            }
            let coords = [
                xy,
                Coord {x, y: xy.y },
            ];
            builder.extract_xnode(node, xy, &coords, naming, &bram_bt_forbidden);
        }
    }

    for (tkn, node, mut naming) in [
        ("BRAM_BOT", "DLL.BOT", ""),
        ("LBRAM_BOTS_GCLK", "DLLS.BOT", "DLLS.BL.GCLK"),
        ("RBRAM_BOTS_GCLK", "DLLS.BOT", "DLLS.BR.GCLK"),
        ("LBRAM_BOTS", "DLLS.BOT", "DLLS.BL"),
        ("RBRAM_BOTS", "DLLS.BOT", "DLLS.BR"),
        ("LBRAM_BOTP", "DLLP.BOT", "DLLP.BL"),
        ("RBRAM_BOTP", "DLLP.BOT", "DLLP.BR"),
        ("BRAM_TOP", "DLL.TOP", ""),
        ("LBRAM_TOPS_GCLK", "DLLS.TOP", "DLLS.TL.GCLK"),
        ("RBRAM_TOPS_GCLK", "DLLS.TOP", "DLLS.TR.GCLK"),
        ("LBRAM_TOPS", "DLLS.TOP", "DLLS.TL"),
        ("RBRAM_TOPS", "DLLS.TOP", "DLLS.TR"),
        ("LBRAM_TOPP", "DLLP.TOP", "DLLP.TL"),
        ("RBRAM_TOPP", "DLLP.TOP", "DLLP.TR"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            if rd.family == "virtex" {
                naming = match node {
                    "DLL.BOT" => if xy.x == 1 {"DLL.BL"} else {"DLL.BR"},
                    "DLL.TOP" => if xy.x == 1 {"DLL.TL"} else {"DLL.TR"},
                    _ => unreachable!(),
                };
            }
            let mut x = xy.x - 1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&(x as i32)) {
                x -= 1;
            }
            let coords = [
                xy,
                Coord {x, y: xy.y },
            ];
            builder.extract_xnode(node, xy, &coords, naming, &dll_forbidden);
        }
    }
    for (naming, mode, bt, lr) in [
        ("DLL.BL", '_', 'B', 'L'),
        ("DLL.BR", '_', 'B', 'R'),
        ("DLL.TL", '_', 'T', 'L'),
        ("DLL.TR", '_', 'T', 'R'),
        ("DLLP.BL", 'P', 'B', 'L'),
        ("DLLP.BR", 'P', 'B', 'R'),
        ("DLLP.TL", 'P', 'T', 'L'),
        ("DLLP.TR", 'P', 'T', 'R'),
        ("DLLS.BL", 'S', 'B', 'L'),
        ("DLLS.BR", 'S', 'B', 'R'),
        ("DLLS.TL", 'S', 'T', 'L'),
        ("DLLS.TR", 'S', 'T', 'R'),
        ("DLLS.BL.GCLK", 'S', 'B', 'L'),
        ("DLLS.BR.GCLK", 'S', 'B', 'R'),
        ("DLLS.TL.GCLK", 'S', 'T', 'L'),
        ("DLLS.TR.GCLK", 'S', 'T', 'R'),
    ] {
        if let Some((_, naming)) = builder.db.node_namings.get_mut(naming) {
            let xt = if mode == 'S' {"_1"} else {""};
            let tile = int::NodeRawTileId::from_idx(1);
            let t_dll = int::NodeTileId::from_idx(0);
            let t_clk = int::NodeTileId::from_idx(2);
            let t_dlls = int::NodeTileId::from_idx(3);
            let wt_clkin = format!("CLK{bt}_CLKIN{lr}{xt}");
            let wt_clkfb = format!("CLK{bt}_CLKFB{lr}{xt}");
            for i in 0..2 {
                naming.ext_pips.insert(((t_dll, clkin), (t_clk, clkpad[i])), int::NodeExtPipNaming {
                    tile,
                    wire_to: wt_clkin.clone(),
                    wire_from: format!("CLK{bt}_CLKPAD{i}"),
                });
                naming.ext_pips.insert(((t_dll, clkfb), (t_clk, clkpad[i])), int::NodeExtPipNaming {
                    tile,
                    wire_to: wt_clkfb.clone(),
                    wire_from: format!("CLK{bt}_CLKPAD{i}"),
                });
            }
            if mode != '_' {
                for i in 0..2 {
                    naming.ext_pips.insert(((t_dll, clkin), (t_clk, iofb[i])), int::NodeExtPipNaming {
                        tile,
                        wire_to: wt_clkin.clone(),
                        wire_from: format!("CLK{bt}_IOFB{i}"),
                    });
                    naming.ext_pips.insert(((t_dll, clkfb), (t_clk, iofb[i])), int::NodeExtPipNaming {
                        tile,
                        wire_to: wt_clkfb.clone(),
                        wire_from: format!("CLK{bt}_IOFB{i}"),
                    });
                }
                if mode == 'P' {
                    naming.ext_pips.insert(((t_dll, clkin), (t_dlls, clk2x)), int::NodeExtPipNaming {
                        tile,
                        wire_to: wt_clkin,
                        wire_from: format!("CLK{bt}_CLK2X{lr}_1"),
                    });
                } else {
                    naming.ext_pips.insert(((t_dll, clkfb), (t_dll, clk2x)), int::NodeExtPipNaming {
                        tile,
                        wire_to: wt_clkfb,
                        wire_from: format!("CLK{bt}_CLK2X{lr}_1"),
                    });
                }
            }
        }
    }
    for (node, mode) in [
        ("DLL.BOT", '_'),
        ("DLL.TOP", '_'),
        ("DLLP.BOT", 'P'),
        ("DLLP.TOP", 'P'),
        ("DLLS.BOT", 'S'),
        ("DLLS.TOP", 'S'),
    ] {
        if let Some((_, node)) = builder.db.nodes.get_mut(node) {
            let t_dll = int::NodeTileId::from_idx(0);
            let t_clk = int::NodeTileId::from_idx(2);
            let t_dlls = int::NodeTileId::from_idx(3);
            for i in 0..2 {
                node.muxes.get_mut(&(t_dll, clkin)).unwrap().ins.insert((t_clk, clkpad[i]));
                node.muxes.get_mut(&(t_dll, clkfb)).unwrap().ins.insert((t_clk, clkpad[i]));
            }
            if mode != '_' {
                for i in 0..2 {
                    node.muxes.get_mut(&(t_dll, clkin)).unwrap().ins.insert((t_clk, iofb[i]));
                    node.muxes.get_mut(&(t_dll, clkfb)).unwrap().ins.insert((t_clk, iofb[i]));
                }
                if mode == 'P' {
                    node.muxes.get_mut(&(t_dll, clkin)).unwrap().ins.insert((t_dlls, clk2x));
                } else {
                    node.muxes.get_mut(&(t_dll, clkfb)).unwrap().ins.insert((t_dll, clk2x));
                }
            }
        }
    }

    let forbidden: Vec<_> = dll_ins.iter().copied().chain(gclk.iter().copied()).collect();
    for tkn in [
        "CLKB", "CLKB_4DLL", "CLKB_2DLL",
        "CLKT", "CLKT_4DLL", "CLKT_2DLL",
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = Coord {
                x: xy.x + 1,
                y: xy.y,
            };
            let coords = if rd.family == "virtex" {
                vec![
                    int_xy,
                    Coord { x: 1, y: xy.y },
                    Coord { x: rd.width - 2, y: xy.y },
                ]
            } else {
                let botp: Vec<_> = find_columns(rd, &["LBRAM_BOTP", "LBRAMS2E_BOTP", "RBRAM_BOTP", "RBRAMS2E_BOTP", "BRAMS2E_BOT_NOGCLK"]).into_iter().collect();
                let bots: Vec<_> = find_columns(rd, &["LBRAM_BOTS", "LBRAM_BOTS_GCLK", "RBRAM_BOTS", "RBRAM_BOTS_GCLK"]).into_iter().collect();
                assert_eq!(botp.len(), 2);
                assert_eq!(bots.len(), 2);
                vec![
                    int_xy,
                    Coord { x: botp[0] as u16, y: xy.y },
                    Coord { x: botp[1] as u16, y: xy.y },
                    Coord { x: bots[0] as u16, y: xy.y },
                    Coord { x: bots[1] as u16, y: xy.y },
                ]
            };
            builder.extract_xnode(tkn, xy, &coords, tkn, &forbidden);
        }
    }

    for tkn in ["CLKL", "CLKR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = Coord {
                x: xy.x,
                y: xy.y + 1,
            };
            builder.extract_xnode(tkn, xy, &[int_xy], tkn, &[pci_ce]);
        }
    }

    builder.build()
}

fn make_grid(rd: &Part) -> (virtex::Grid, BTreeSet<DisabledPart>) {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &[
        "CENTER",
        "LBRAM",
        "RBRAM",
        "MBRAM",
        "MBRAMS2E",
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
        columns: int.cols.len(),
        cols_bram: get_cols_bram(&rd, &int),
        cols_clkv: get_cols_clkv(&rd, &int),
        rows: int.rows.len(),
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
    let eint = grid.expand_grid(&disabled, &int_db);
    let vrf = Verifier::new(rd, &eint);
    vrf.finish();
    (make_device(rd, geom::Grid::Virtex(grid), bonds, disabled), Some(int_db))
}
