use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::int::{
    BelInfo, BelNaming, BelPin, BelPinNaming, Dir, IntDb, NodeExtPipNaming, NodeRawTileId,
    NodeTileId, PinDir, WireKind,
};
use prjcombine_xilinx_rawdump::{Coord, Part};
use std::collections::BTreeMap;

use crate::grid::find_columns;
use crate::intb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("virtex", rd);

    let mut bram_forbidden = Vec::new();
    let mut bram_bt_forbidden = Vec::new();
    let mut dll_forbidden = Vec::new();
    let mut bram_extra_pips = BTreeMap::new();

    let mut gclk = Vec::new();
    for i in 0..4 {
        let w = builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[
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
            ],
        );
        builder.extra_name_sub(format!("MBRAM_GCLKD{i}"), 0, w);
        builder.extra_name_sub(format!("MBRAM_GCLKA{i}"), 3, w);
        builder.extra_name_sub(format!("BRAM_BOT_VGCLK{i}"), 2, w);
        builder.extra_name_sub(format!("BRAM_TOP_VGCLK{i}"), 2, w);
        gclk.push(w);
        bram_forbidden.push(w);
        bram_bt_forbidden.push(w);
        dll_forbidden.push(w);
        builder.buf(
            w,
            format!("GCLK{i}.BUF"),
            &[format!("BOT_GCLK{i}"), format!("TOP_GCLK{i}")],
        );
    }

    let pci_ce = builder.wire(
        "PCI_CE",
        WireKind::MultiBranch(Dir::S),
        &[
            "LEFT_PCI_CE",
            "RIGHT_PCI_CE",
            "LL_PCI_CE",
            "LR_PCI_CE",
            "UL_PCI_CE",
            "UR_PCI_CE",
        ],
    );
    builder.conn_branch(pci_ce, Dir::N, pci_ce);

    for i in 0..24 {
        let w = builder.wire(
            format!("SINGLE.E{i}"),
            WireKind::PipOut,
            &[format!("E{i}"), format!("LEFT_E{i}")],
        );
        builder.buf(
            w,
            format!("SINGLE.E{i}.BUF"),
            &[format!("E_P{i}"), format!("LEFT_E_BUF{i}")],
        );
        let w = builder.pip_branch(
            w,
            Dir::E,
            format!("SINGLE.W{i}"),
            &[format!("W{i}"), format!("RIGHT_W{i}")],
        );
        builder.buf(
            w,
            format!("SINGLE.W{i}.BUF"),
            &[format!("W_P{i}"), format!("RIGHT_W_BUF{i}")],
        );
    }
    for i in 0..24 {
        let w = builder.wire(
            format!("SINGLE.S{i}"),
            WireKind::PipOut,
            &[format!("S{i}"), format!("TOP_S{i}")],
        );
        builder.buf(
            w,
            format!("SINGLE.S{i}.BUF"),
            &[format!("S_P{i}"), format!("TOP_S_BUF{i}")],
        );
        let w = builder.pip_branch(
            w,
            Dir::S,
            format!("SINGLE.N{i}"),
            &[format!("N{i}"), format!("BOT_N{i}")],
        );
        builder.buf(
            w,
            format!("SINGLE.N{i}.BUF"),
            &[format!("N_P{i}"), format!("BOT_N_BUF{i}")],
        );
    }

    let def_t = NodeTileId::from_idx(0);
    for name in ["ADDR", "DIN", "DOUT"] {
        let mut l = Vec::new();
        let mut ln = Vec::new();
        for i in 0..32 {
            let w = builder.mux_out(
                format!("BRAM.SINGLE.{name}{i}"),
                &[format!("BRAM_R{name}S{i}")],
            );
            let s = builder.branch(
                w,
                Dir::S,
                format!("BRAM.SINGLE.{name}{i}.S"),
                &[format!("BRAM_R{name}N{i}")],
            );
            bram_forbidden.push(s);
            let n = builder.branch(w, Dir::N, format!("BRAM.SINGLE.{name}{i}.n"), &[""]);
            l.push(w);
            ln.push(n);
        }
        for i in 0..32 {
            let si = if name == "ADDR" {
                i
            } else {
                i & 0x10 | (i + 0xf) & 0xf
            };
            bram_extra_pips.insert(
                ((def_t, l[i]), (def_t, ln[si])),
                NodeExtPipNaming {
                    tile: NodeRawTileId::from_idx(1),
                    wire_to: format!("BRAM_R{name}N{i}"),
                    wire_from: format!("BRAM_R{name}S{si}"),
                },
            );
        }
    }

    let hexnames = |pref, i| {
        [
            format!("{pref}{i}"),
            format!("LEFT_{pref}{i}"),
            format!("RIGHT_{pref}{i}"),
            format!("TOP_{pref}{i}"),
            format!("BOT_{pref}{i}"),
            format!("LL_{pref}{i}"),
            format!("LR_{pref}{i}"),
            format!("UL_{pref}{i}"),
            format!("UR_{pref}{i}"),
        ]
    };
    let hexnames_hc = |pref, i| {
        [
            format!("{pref}{i}"),
            format!("LEFT_{pref}{i}"),
            format!("RIGHT_{pref}{i}"),
        ]
    };
    let hexnames_hio = |pref, i| {
        [
            format!("TOP_{pref}{i}"),
            format!("BOT_{pref}{i}"),
            format!("LL_{pref}{i}"),
            format!("LR_{pref}{i}"),
            format!("UL_{pref}{i}"),
            format!("UR_{pref}{i}"),
        ]
    };
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

    let lh: Vec<_> = (0..12)
        .map(|i| {
            builder.wire(
                format!("LH.{i}"),
                WireKind::MultiBranch(Dir::W),
                &[
                    format!("LH{i}"),
                    format!("LEFT_LH{i}"),
                    format!("RIGHT_LH{i}"),
                    format!("BOT_LH{i}"),
                    format!("TOP_LH{i}"),
                    format!("LL_LH{i}"),
                    format!("LR_LH{i}"),
                    format!("UL_LH{i}"),
                    format!("UR_LH{i}"),
                ],
            )
        })
        .collect();
    for i in 0..12 {
        builder.conn_branch(lh[i], Dir::E, lh[(i + 11) % 12]);
    }
    builder.buf(lh[0], "LH.0.FAKE", &["TOP_FAKE_LH0", "BOT_FAKE_LH0"]);
    builder.buf(lh[6], "LH.6.FAKE", &["TOP_FAKE_LH6", "BOT_FAKE_LH6"]);

    let lv: Vec<_> = (0..12)
        .map(|i| {
            builder.wire(
                format!("LV.{i}"),
                WireKind::MultiBranch(Dir::S),
                &[
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
                ],
            )
        })
        .collect();
    for i in 0..12 {
        builder.conn_branch(lv[i], Dir::N, lv[(i + 11) % 12]);
        dll_forbidden.push(lv[i]);
    }

    for i in 0..2 {
        for pin in ["CLK", "SR", "CE", "BX", "BY"] {
            builder.mux_out(format!("IMUX.S{i}.{pin}"), &[format!("S{i}_{pin}_B")]);
        }
        for fg in ['F', 'G'] {
            for j in 1..5 {
                builder.mux_out(format!("IMUX.S{i}.{fg}{j}"), &[format!("S{i}_{fg}_B{j}")]);
            }
        }
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.TBUF{i}.T"),
            &[
                format!("TS_B{i}"),
                format!("LEFT_TS{i}_B"),
                format!("RIGHT_TS{i}_B"),
            ],
        );
        builder.mux_out(
            format!("IMUX.TBUF{i}.I"),
            &[
                format!("T_IN{i}"),
                format!("LEFT_TI{i}_B"),
                format!("RIGHT_TI{i}_B"),
            ],
        );
    }
    for i in 0..4 {
        for pin in ["CLK", "SR", "ICE", "OCE", "TCE", "O", "T"] {
            let np = if pin == "SR" { "SR_B" } else { pin };
            builder.mux_out(
                format!("IMUX.IO{i}.{pin}"),
                &[
                    format!("LEFT_{np}{i}"),
                    format!("RIGHT_{np}{i}"),
                    format!("BOT_{np}{i}"),
                    format!("TOP_{np}{i}"),
                ],
            );
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
            builder.mux_out(format!("IMUX.BRAM.DI{ab}{i}"), &[format!("BRAM_DI{ab}{i}")]);
        }
    }
    for ab in ['A', 'B'] {
        for i in 0..12 {
            builder.mux_out(
                format!("IMUX.BRAM.ADDR{ab}{i}"),
                &[format!("BRAM_ADDR{ab}{i}")],
            );
        }
    }
    for name in ["CLK", "RST", "SEL", "WE"] {
        for ab in ['A', 'B'] {
            builder.mux_out(
                format!("IMUX.BRAM.{name}{ab}"),
                &[format!("BRAM_{name}{ab}"), format!("MBRAM_{name}{ab}")],
            );
        }
    }

    for i in 0..8 {
        let w = builder.mux_out(
            format!("OMUX{i}"),
            &[
                format!("OUT{i}"),
                format!("LEFT_OUT{i}"),
                format!("RIGHT_OUT{i}"),
            ],
        );
        if matches!(i, 0 | 1) {
            builder.branch(
                w,
                Dir::E,
                format!("OMUX{i}.W"),
                &[format!("OUT_W{i}"), format!("RIGHT_OUT_W{i}")],
            );
        }
        if matches!(i, 6 | 7) {
            builder.branch(
                w,
                Dir::W,
                format!("OMUX{i}.E"),
                &[format!("OUT_E{i}"), format!("LEFT_OUT_E{i}")],
            );
        }
    }

    for i in 0..2 {
        for pin in ["X", "Y", "XQ", "YQ", "XB", "YB"] {
            builder.logic_out(format!("OUT.S{i}.{pin}"), &[format!("S{i}_{pin}")]);
        }
    }
    builder.logic_out("OUT.TBUF", &["TBUFO"]);
    for i in 0..4 {
        builder.logic_out(format!("OUT.TBUF.L{i}"), &[format!("LEFT_TBUFO{i}")]);
    }
    for i in 0..4 {
        builder.logic_out(format!("OUT.TBUF.R{i}"), &[format!("RIGHT_TBUFO{i}")]);
    }
    for i in 0..4 {
        for pin in ["I", "IQ"] {
            builder.logic_out(
                format!("OUT.IO{i}.{pin}"),
                &[
                    format!("LEFT_{pin}{i}"),
                    format!("RIGHT_{pin}{i}"),
                    format!("BOT_{pin}{i}"),
                    format!("TOP_{pin}{i}"),
                ],
            );
        }
    }
    for pin in [
        "RESET", "DRCK1", "DRCK2", "SHIFT", "TDI", "UPDATE", "SEL1", "SEL2",
    ] {
        builder.logic_out(format!("OUT.BSCAN.{pin}"), &[format!("UL_{pin}")]);
    }

    for ab in ['A', 'B'] {
        for i in 0..16 {
            builder.logic_out(format!("OUT.BRAM.DO{ab}{i}"), &[format!("BRAM_DO{ab}{i}")]);
        }
    }

    for i in 0..2 {
        builder.mux_out(
            format!("CLK.IMUX.BUFGCE.CLK{i}"),
            &[
                format!("CLKB_GCLKBUF{i}_IN"),
                format!("CLKT_GCLKBUF{ii}_IN", ii = i + 2),
            ],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("CLK.IMUX.BUFGCE.CE{i}"),
            &[format!("CLKB_CE{i}"), format!("CLKT_CE{i}")],
        );
    }
    for i in 0..2 {
        builder.logic_out(
            format!("CLK.OUT.BUFGCE.O{i}"),
            &[
                format!("CLKB_GCLK{i}_PW"),
                format!("CLKT_GCLK{ii}_PW", ii = i + 2),
            ],
        );
    }
    let mut clkpad = Vec::new();
    for i in 0..2 {
        let w = builder.logic_out(
            format!("CLK.OUT.CLKPAD{i}"),
            &[format!("CLKB_CLKPAD{i}"), format!("CLKT_CLKPAD{i}")],
        );
        clkpad.push(w);
    }
    let mut iofb = Vec::new();
    for i in 0..2 {
        let w = builder.logic_out(
            format!("CLK.OUT.IOFB{i}"),
            &[format!("CLKB_IOFB{i}"), format!("CLKT_IOFB{i}")],
        );
        iofb.push(w);
    }
    for i in 1..4 {
        builder.mux_out(
            format!("PCI.IMUX.I{i}"),
            &[format!("CLKL_I{i}"), format!("CLKR_I{i}")],
        );
    }
    let mut dll_pins = BTreeMap::new();
    let mut dll_ins = Vec::new();
    let mut clkin = None;
    let mut clkfb = None;
    for name in ["CLKIN", "CLKFB", "RST"] {
        let w = builder.mux_out(
            format!("DLL.IMUX.{name}"),
            &[
                format!("BRAM_BOT_{name}"),
                format!("BRAM_BOTP_{name}"),
                format!("BRAM_BOT_{name}_1"),
                format!("BRAM_TOP_{name}"),
                format!("BRAM_TOPP_{name}"),
                format!("BRAM_TOPS_{name}"),
            ],
        );
        builder.extra_name_sub(format!("CLKB_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKB_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKB_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKB_{name}R_1"), 4, w);
        builder.extra_name_sub(format!("CLKT_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKT_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKT_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKT_{name}R_1"), 4, w);
        dll_ins.push(w);
        dll_pins.insert(
            name.to_string(),
            BelPin {
                wire: (NodeTileId::from_idx(0), w),
                dir: PinDir::Input,
            },
        );
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
        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X90", "CLKDV", "LOCKED",
    ] {
        let w = builder.logic_out(format!("DLL.OUT.{name}"), &[""]);
        builder.extra_name_sub(format!("CLKB_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKB_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKB_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKB_{name}R_1"), 4, w);
        builder.extra_name_sub(format!("CLKT_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKT_{name}R"), 2, w);
        if name == "LOCKED" {
            builder.extra_name_sub("CLKT_LOCK_TL_1", 3, w);
        } else {
            builder.extra_name_sub(format!("CLKT_{name}L_1"), 3, w);
        }
        builder.extra_name_sub(format!("CLKT_{name}R_1"), 4, w);
        if name == "CLK2X" {
            clk2x = Some(w);
        }
        dll_pins.insert(
            name.to_string(),
            BelPin {
                wire: (NodeTileId::from_idx(0), w),
                dir: PinDir::Output,
            },
        );
    }
    let clk2x = clk2x.unwrap();

    builder.extract_main_passes();

    let slice_name_only = ["F5IN", "F5", "CIN", "COUT"];

    builder.extract_node(
        "CENTER",
        "CLB",
        "CLB",
        &[
            builder
                .bel_indexed("SLICE0", "SLICE", 0)
                .pins_name_only(&slice_name_only)
                .pin_name_only("COUT", 1),
            builder
                .bel_indexed("SLICE1", "SLICE", 1)
                .pins_name_only(&slice_name_only)
                .pin_name_only("COUT", 1),
            builder
                .bel_indexed("TBUF0", "TBUF", 0)
                .pins_name_only(&["O"]),
            builder
                .bel_indexed("TBUF1", "TBUF", 1)
                .pins_name_only(&["O"]),
            builder
                .bel_virtual("TBUS")
                .extra_wire("BUS0", &["TBUF0"])
                .extra_wire("BUS1", &["TBUF1"])
                .extra_wire("BUS2", &["TBUF2"])
                .extra_wire("BUS3", &["TBUF3"])
                .extra_wire("BUS3_E", &["TBUF_STUB3"])
                .extra_int_out("OUT", &["TBUFO"]),
        ],
    );

    let bels_left = [
        builder.bel_indexed("IOB0", "IOB", 0),
        builder
            .bel_indexed("IOB1", "IOB", 1)
            .extra_wire_force("PCI", "LEFT_PCI_BOT_PCI1"),
        builder.bel_indexed("IOB2", "IOB", 2),
        builder
            .bel_indexed("IOB3", "IOB", 3)
            .extra_wire_force("PCI", "LEFT_PCI_TOP_PCI3"),
        builder
            .bel_indexed("TBUF0", "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed("TBUF1", "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual("TBUS")
            .extra_int_out("BUS0", &["LEFT_TBUFO2"])
            .extra_int_out("BUS1", &["LEFT_TBUFO3"])
            .extra_int_out("BUS2", &["LEFT_TBUFO0"])
            .extra_int_out("BUS3", &["LEFT_TBUFO1"])
            .extra_wire("BUS3_E", &["LEFT_TBUF1_STUB"]),
    ];
    builder.extract_node("LEFT", "IO.L", "IO.L", &bels_left);
    builder.extract_node("LEFT_PCI_BOT", "IO.L", "IO.L", &bels_left);
    builder.extract_node("LEFT_PCI_TOP", "IO.L", "IO.L", &bels_left);

    let bels_right = [
        builder.bel_indexed("IOB0", "IOB", 0),
        builder
            .bel_indexed("IOB1", "IOB", 1)
            .extra_wire_force("PCI", "RIGHT_PCI_BOT_PCI1"),
        builder.bel_indexed("IOB2", "IOB", 2),
        builder
            .bel_indexed("IOB3", "IOB", 3)
            .extra_wire_force("PCI", "RIGHT_PCI_TOP_PCI3"),
        builder
            .bel_indexed("TBUF0", "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed("TBUF1", "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual("TBUS")
            .extra_int_out("BUS0", &["RIGHT_TBUFO2"])
            .extra_int_out("BUS1", &["RIGHT_TBUFO3"])
            .extra_int_out("BUS2", &["RIGHT_TBUFO0"])
            .extra_int_out("BUS3", &["RIGHT_TBUFO1"]),
    ];
    builder.extract_node("RIGHT", "IO.R", "IO.R", &bels_right);
    builder.extract_node("RIGHT_PCI_BOT", "IO.R", "IO.R", &bels_right);
    builder.extract_node("RIGHT_PCI_TOP", "IO.R", "IO.R", &bels_right);

    let bels_bot = [
        builder.bel_indexed("IOB0", "IOB", 0),
        builder
            .bel_indexed("IOB1", "IOB", 1)
            .extra_wire_force("DLLFB", "BL_DLLIOB_IOFB"),
        builder
            .bel_indexed("IOB2", "IOB", 2)
            .extra_wire_force("DLLFB", "BR_DLLIOB_IOFB"),
        builder.bel_indexed("IOB3", "IOB", 3),
    ];
    builder.extract_node("BOT", "IO.B", "IO.B", &bels_bot);
    builder.extract_node("BL_DLLIOB", "IO.B", "IO.B", &bels_bot);
    builder.extract_node("BR_DLLIOB", "IO.B", "IO.B", &bels_bot);

    let bels_top = [
        builder.bel_indexed("IOB0", "IOB", 0),
        builder
            .bel_indexed("IOB1", "IOB", 1)
            .extra_wire_force("DLLFB", "TL_DLLIOB_IOFB"),
        builder
            .bel_indexed("IOB2", "IOB", 2)
            .extra_wire_force("DLLFB", "TR_DLLIOB_IOFB"),
        builder.bel_indexed("IOB3", "IOB", 3),
    ];
    builder.extract_node("TOP", "IO.T", "IO.T", &bels_top);
    builder.extract_node("TL_DLLIOB", "IO.T", "IO.T", &bels_top);
    builder.extract_node("TR_DLLIOB", "IO.T", "IO.T", &bels_top);

    builder.extract_node(
        "LL",
        "CNR.BL",
        "CNR.BL",
        &[builder.bel_single("CAPTURE", "CAPTURE")],
    );
    builder.extract_node("LR", "CNR.BR", "CNR.BR", &[]);
    builder.extract_node(
        "UL",
        "CNR.TL",
        "CNR.TL",
        &[
            builder.bel_single("STARTUP", "STARTUP"),
            builder.bel_single("BSCAN", "BSCAN"),
        ],
    );
    builder.extract_node("UR", "CNR.TR", "CNR.TR", &[]);

    for tkn in ["LBRAM", "RBRAM", "MBRAM"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut x = xy.x - 1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&(x as i32)) {
                x -= 1;
            }
            let mut coords = Vec::new();
            for dy in 0..4 {
                coords.push(Coord {
                    x: xy.x,
                    y: xy.y + dy,
                });
            }
            for dy in 0..4 {
                coords.push(Coord { x, y: xy.y + dy });
            }
            builder.extract_xnode(
                tkn,
                xy,
                &[],
                &coords,
                tkn,
                &[builder.bel_single("BRAM", "BLOCKRAM")],
                &bram_forbidden,
            );
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
            let coords = [xy, Coord { x, y: xy.y }];
            builder.extract_xnode(node, xy, &[], &coords, naming, &[], &bram_bt_forbidden);
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
                    "DLL.BOT" => {
                        if xy.x == 1 {
                            "DLL.BL"
                        } else {
                            "DLL.BR"
                        }
                    }
                    "DLL.TOP" => {
                        if xy.x == 1 {
                            "DLL.TL"
                        } else {
                            "DLL.TR"
                        }
                    }
                    _ => unreachable!(),
                };
            }
            let mut x = xy.x - 1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&(x as i32)) {
                x -= 1;
            }
            let coords = [xy, Coord { x, y: xy.y }];
            builder.extract_xnode(node, xy, &[], &coords, naming, &[], &dll_forbidden);
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
            let xt = if mode == 'S' { "_1" } else { "" };
            let tile = NodeRawTileId::from_idx(1);
            let t_dll = NodeTileId::from_idx(0);
            let t_clk = NodeTileId::from_idx(2);
            let t_dlls = NodeTileId::from_idx(3);
            let wt_clkin = format!("CLK{bt}_CLKIN{lr}{xt}");
            let wt_clkfb = format!("CLK{bt}_CLKFB{lr}{xt}");
            for i in 0..2 {
                naming.ext_pips.insert(
                    ((t_dll, clkin), (t_clk, clkpad[i])),
                    NodeExtPipNaming {
                        tile,
                        wire_to: wt_clkin.clone(),
                        wire_from: format!("CLK{bt}_CLKPAD{i}"),
                    },
                );
                naming.ext_pips.insert(
                    ((t_dll, clkfb), (t_clk, clkpad[i])),
                    NodeExtPipNaming {
                        tile,
                        wire_to: wt_clkfb.clone(),
                        wire_from: format!("CLK{bt}_CLKPAD{i}"),
                    },
                );
            }
            if mode != '_' {
                for i in 0..2 {
                    naming.ext_pips.insert(
                        ((t_dll, clkin), (t_clk, iofb[i])),
                        NodeExtPipNaming {
                            tile,
                            wire_to: wt_clkin.clone(),
                            wire_from: format!("CLK{bt}_IOFB{i}"),
                        },
                    );
                    naming.ext_pips.insert(
                        ((t_dll, clkfb), (t_clk, iofb[i])),
                        NodeExtPipNaming {
                            tile,
                            wire_to: wt_clkfb.clone(),
                            wire_from: format!("CLK{bt}_IOFB{i}"),
                        },
                    );
                }
                if mode == 'P' {
                    naming.ext_pips.insert(
                        ((t_dll, clkin), (t_dlls, clk2x)),
                        NodeExtPipNaming {
                            tile,
                            wire_to: wt_clkin,
                            wire_from: format!("CLK{bt}_CLK2X{lr}_1"),
                        },
                    );
                } else {
                    naming.ext_pips.insert(
                        ((t_dll, clkfb), (t_dll, clk2x)),
                        NodeExtPipNaming {
                            tile,
                            wire_to: wt_clkfb,
                            wire_from: format!("CLK{bt}_CLK2X{lr}_1"),
                        },
                    );
                }
            }
            let pins = dll_pins
                .keys()
                .map(|k| {
                    let mut name = format!("CLK{bt}_{k}{lr}{xt}");
                    if bt == 'T' && lr == 'L' && mode != '_' && k == "RST" {
                        if mode == 'S' {
                            name = "CLKT_RSTL".to_string();
                        } else {
                            name = "CLKT_RSTL_1".to_string();
                        }
                    }
                    if bt == 'T' && lr == 'L' && mode == 'S' && k == "LOCKED" {
                        name = "CLKT_LOCK_TL_1".to_string();
                    }
                    (
                        k.clone(),
                        BelPinNaming {
                            name: name.clone(),
                            name_far: name,
                            pips: Vec::new(),
                        },
                    )
                })
                .collect();
            naming.bels.push(BelNaming {
                tile: NodeRawTileId::from_idx(1),
                pins,
            });
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
            let t_dll = NodeTileId::from_idx(0);
            let t_clk = NodeTileId::from_idx(2);
            let t_dlls = NodeTileId::from_idx(3);
            for i in 0..2 {
                node.muxes
                    .get_mut(&(t_dll, clkin))
                    .unwrap()
                    .ins
                    .insert((t_clk, clkpad[i]));
                node.muxes
                    .get_mut(&(t_dll, clkfb))
                    .unwrap()
                    .ins
                    .insert((t_clk, clkpad[i]));
            }
            if mode != '_' {
                for i in 0..2 {
                    node.muxes
                        .get_mut(&(t_dll, clkin))
                        .unwrap()
                        .ins
                        .insert((t_clk, iofb[i]));
                    node.muxes
                        .get_mut(&(t_dll, clkfb))
                        .unwrap()
                        .ins
                        .insert((t_clk, iofb[i]));
                }
                if mode == 'P' {
                    node.muxes
                        .get_mut(&(t_dll, clkin))
                        .unwrap()
                        .ins
                        .insert((t_dlls, clk2x));
                } else {
                    node.muxes
                        .get_mut(&(t_dll, clkfb))
                        .unwrap()
                        .ins
                        .insert((t_dll, clk2x));
                }
            }
            node.bels.insert(
                "DLL".to_string(),
                BelInfo {
                    pins: dll_pins.clone(),
                },
            );
        }
    }

    let forbidden: Vec<_> = dll_ins
        .iter()
        .copied()
        .chain(gclk.iter().copied())
        .collect();
    for tkn in [
        "CLKB",
        "CLKB_4DLL",
        "CLKB_2DLL",
        "CLKT",
        "CLKT_4DLL",
        "CLKT_2DLL",
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
                    Coord {
                        x: rd.width - 2,
                        y: xy.y,
                    },
                ]
            } else {
                let botp: Vec<_> = find_columns(
                    rd,
                    &[
                        "LBRAM_BOTP",
                        "LBRAMS2E_BOTP",
                        "RBRAM_BOTP",
                        "RBRAMS2E_BOTP",
                        "BRAMS2E_BOT_NOGCLK",
                    ],
                )
                .into_iter()
                .collect();
                let bots: Vec<_> = find_columns(
                    rd,
                    &[
                        "LBRAM_BOTS",
                        "LBRAM_BOTS_GCLK",
                        "RBRAM_BOTS",
                        "RBRAM_BOTS_GCLK",
                    ],
                )
                .into_iter()
                .collect();
                assert_eq!(botp.len(), 2);
                assert_eq!(bots.len(), 2);
                vec![
                    int_xy,
                    Coord {
                        x: botp[0] as u16,
                        y: xy.y,
                    },
                    Coord {
                        x: botp[1] as u16,
                        y: xy.y,
                    },
                    Coord {
                        x: bots[0] as u16,
                        y: xy.y,
                    },
                    Coord {
                        x: bots[1] as u16,
                        y: xy.y,
                    },
                ]
            };
            let mut bels = vec![
                builder.bel_indexed("GCLKIOB0", "GCLKIOB", 0),
                builder.bel_indexed("GCLKIOB1", "GCLKIOB", 1),
                builder.bel_indexed("BUFG0", "GCLK", 0)
                    .extra_wire("OUT.GLOBAL", &["CLKB_GCLK0", "CLKT_GCLK2"]),
                builder.bel_indexed("BUFG1", "GCLK", 1)
                    .extra_wire("OUT.GLOBAL", &["CLKB_GCLK1", "CLKT_GCLK3"]),
            ];
            if rd.family != "virtex" {
                bels.push(
                    builder
                        .bel_virtual("IOFB0")
                        .extra_int_out("O", &["CLKB_IOFB0", "CLKT_IOFB0"]),
                );
                bels.push(
                    builder
                        .bel_virtual("IOFB1")
                        .extra_int_out("O", &["CLKB_IOFB1", "CLKT_IOFB1"]),
                );
            }
            builder.extract_xnode(tkn, xy, &[], &coords, tkn, &bels, &forbidden);
        }
    }

    for tkn in ["CLKL", "CLKR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = Coord {
                x: xy.x,
                y: xy.y + 1,
            };
            builder.extract_xnode(
                tkn,
                xy,
                &[],
                &[int_xy],
                tkn,
                &[builder
                    .bel_single("PCILOGIC", "PCILOGIC")
                    .pin_name_only("IRDY", 1)
                    .pin_name_only("TRDY", 1)],
                &[pci_ce],
            );
        }
    }

    for &xy in rd.tiles_by_kind_name("CLKC") {
        builder.extract_xnode_bels(
            "CLKC",
            xy,
            &[],
            &[xy],
            "CLKC",
            &[
                builder.bel_virtual("CLKC")
                    .extra_wire("IN0", &["CLKC_GCLK0"])
                    .extra_wire("IN1", &["CLKC_GCLK1"])
                    .extra_wire("IN2", &["CLKC_GCLK2"])
                    .extra_wire("IN3", &["CLKC_GCLK3"])
                    .extra_wire("OUT0", &["CLKC_HGCLK0"])
                    .extra_wire("OUT1", &["CLKC_HGCLK1"])
                    .extra_wire("OUT2", &["CLKC_HGCLK2"])
                    .extra_wire("OUT3", &["CLKC_HGCLK3"]),
                builder.bel_virtual("GCLKC")
                    .extra_wire("IN0", &["CLKC_HGCLK0"])
                    .extra_wire("IN1", &["CLKC_HGCLK1"])
                    .extra_wire("IN2", &["CLKC_HGCLK2"])
                    .extra_wire("IN3", &["CLKC_HGCLK3"])
                    .extra_wire("OUT0", &["CLKC_VGCLK0"])
                    .extra_wire("OUT1", &["CLKC_VGCLK1"])
                    .extra_wire("OUT2", &["CLKC_VGCLK2"])
                    .extra_wire("OUT3", &["CLKC_VGCLK3"]),
            ],
        );
    }

    for &xy in rd.tiles_by_kind_name("GCLKC") {
        builder.extract_xnode_bels(
            "GCLKC",
            xy,
            &[],
            &[xy],
            "GCLKC",
            &[
                builder.bel_virtual("GCLKC")
                    .extra_wire_force("IN0", "GCLKC_HGCLK0")
                    .extra_wire_force("IN1", "GCLKC_HGCLK1")
                    .extra_wire_force("IN2", "GCLKC_HGCLK2")
                    .extra_wire_force("IN3", "GCLKC_HGCLK3")
                    .extra_wire_force("OUT0", "GCLKC_VGCLK0")
                    .extra_wire_force("OUT1", "GCLKC_VGCLK1")
                    .extra_wire_force("OUT2", "GCLKC_VGCLK2")
                    .extra_wire_force("OUT3", "GCLKC_VGCLK3"),
            ],
        );
    }

    for &xy in rd.tiles_by_kind_name("BRAM_CLKH") {
        builder.extract_xnode_bels(
            "BRAM_CLKH",
            xy,
            &[],
            &[xy],
            "BRAM_CLKH",
            &[
                builder.bel_virtual("BRAM_CLKH")
                    .extra_wire_force("IN0", "BRAM_CLKH_GCLK0")
                    .extra_wire_force("IN1", "BRAM_CLKH_GCLK1")
                    .extra_wire_force("IN2", "BRAM_CLKH_GCLK2")
                    .extra_wire_force("IN3", "BRAM_CLKH_GCLK3")
                    .extra_int_out_force("OUT0", (NodeTileId::from_idx(0), gclk[0]), "BRAM_CLKH_VGCLK0")
                    .extra_int_out_force("OUT1", (NodeTileId::from_idx(0), gclk[1]), "BRAM_CLKH_VGCLK1")
                    .extra_int_out_force("OUT2", (NodeTileId::from_idx(0), gclk[2]), "BRAM_CLKH_VGCLK2")
                    .extra_int_out_force("OUT3", (NodeTileId::from_idx(0), gclk[3]), "BRAM_CLKH_VGCLK3")
            ],
        );
    }

    for (tkn, naming) in [
        ("CLKV", "CLKV.CLKV"),
        ("CLKB", "CLKV.CLKB"),
        ("CLKB_4DLL", "CLKV.CLKB"),
        ("CLKB_2DLL", "CLKV.CLKB"),
        ("CLKT", "CLKV.CLKT"),
        ("CLKT_4DLL", "CLKV.CLKT"),
        ("CLKT_2DLL", "CLKV.CLKT"),
        ("GCLKV", "CLKV.GCLKV"),
        ("GCLKB", "CLKV.GCLKB"),
        ("GCLKT", "CLKV.GCLKT"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy_l = builder.walk_to_int(xy, Dir::W).unwrap();
            let int_xy_r = builder.walk_to_int(xy, Dir::E).unwrap();
            let mut bel = builder.bel_virtual("CLKV");
            for i in 0..4 {
                bel = bel.extra_int_out(format!("OUT_L{i}"), &[
                    format!("GCLKV_BUFL{i}"),
                    format!("CLKV_GCLK_BUFL{i}"),
                    format!("GCLKB_GCLKW{i}"),
                    format!("GCLKT_GCLKW{i}"),
                    format!("CLKB_HGCLK_W{i}"),
                    format!("CLKT_HGCLK_W{i}"),
                ]);
                bel = bel.extra_int_out(format!("OUT_R{i}"), &[
                    format!("GCLKV_BUFR{i}"),
                    format!("CLKV_GCLK_BUFR{i}"),
                    format!("GCLKB_GCLKE{i}"),
                    format!("GCLKT_GCLKE{i}"),
                    format!("CLKB_HGCLK_E{i}"),
                    format!("CLKT_HGCLK_E{i}"),
                ]);
                bel = bel.extra_wire(format!("IN{i}"), &[
                    format!("GCLKV_GCLK_B{i}"),
                    format!("CLKV_VGCLK{i}"),
                    format!("GCLKB_VGCLK{i}"),
                    format!("GCLKT_VGCLK{i}"),
                    format!("CLKB_VGCLK{i}"),
                    format!("CLKT_VGCLK{i}"),
                ]);
            }
            builder.extract_xnode_bels(
                "CLKV",
                xy,
                &[],
                &[int_xy_l, int_xy_r],
                naming,
                &[bel],
            );
        }
    }

    for (tkn, naming) in [
        ("LBRAM", "CLKV_BRAM.L"),
        ("RBRAM", "CLKV_BRAM.R"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut bel = builder.bel_virtual("CLKV_BRAM");
            let mut coords = vec![xy];
            for i in 0..4 {
                bel = bel.extra_int_in(format!("IN{i}"), &[
                    format!("BRAM_GCLKIN{i}"),
                ]);
            }
            for (i, l) in ['D', 'C', 'B', 'A'].into_iter().enumerate() {
                for j in 0..4 {
                    bel = bel.extra_int_out(format!("OUT_L{i}_{j}"), &[
                        format!("LBRAM_GCLK_IOB{l}{j}"),
                        format!("RBRAM_GCLK_CLB{l}{j}"),
                    ]);
                    bel = bel.extra_int_out(format!("OUT_R{i}_{j}"), &[
                        format!("LBRAM_GCLK_CLB{l}{j}"),
                        format!("RBRAM_GCLK_IOB{l}{j}"),
                    ]);
                }
            }
            for i in 0..4 {
                coords.push(Coord {
                    x: xy.x - 1,
                    y: xy.y + i,
                });
            }
            for i in 0..4 {
                coords.push(Coord {
                    x: xy.x + 1,
                    y: xy.y + i,
                });
            }
            builder.extract_xnode_bels(
                "CLKV_BRAM",
                xy,
                &[],
                &coords,
                naming,
                &[bel],
            );
        }
    }

    for (tkn, kind) in [
        ("BRAM_BOT", "CLKV_BRAM_BOT"),
        ("BRAM_BOT_GCLK", "CLKV_BRAM_BOT"),
        ("LBRAM_BOTS_GCLK", "CLKV_BRAM_BOT"),
        ("RBRAM_BOTS_GCLK", "CLKV_BRAM_BOT"),
        ("BRAM_TOP", "CLKV_BRAM_TOP"),
        ("BRAM_TOP_GCLK", "CLKV_BRAM_TOP"),
        ("LBRAM_TOPS_GCLK", "CLKV_BRAM_TOP"),
        ("RBRAM_TOPS_GCLK", "CLKV_BRAM_TOP"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy_l = builder.walk_to_int(xy, Dir::W).unwrap();
            let mut bel = builder.bel_virtual(kind);
            for i in 0..4 {
                bel = bel.extra_int_out(format!("OUT_L{i}"), &[
                    format!("BRAM_BOT_GCLKW{i}"),
                    format!("BRAM_TOP_GCLKW{i}"),
                ]);
                bel = bel.extra_int_out(format!("OUT_R{i}"), &[
                    format!("BRAM_BOT_GCLKE{i}"),
                    format!("BRAM_TOP_GCLKE{i}"),
                ]);
                bel = bel.extra_int_in(format!("IN{i}"), &[
                    format!("BRAM_BOT_VGCLK{i}"),
                    format!("BRAM_TOP_VGCLK{i}"),
                ]);
            }
            let bram_xy = xy; // dummy position
            builder.extract_xnode_bels(
                kind,
                xy,
                &[],
                &[xy, int_xy_l, bram_xy],
                kind,
                &[bel],
            );
        }
    }

    builder.build()
}
