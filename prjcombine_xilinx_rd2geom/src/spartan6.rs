use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_xilinx_rawdump::{Part, Coord, PkgPin};
use prjcombine_xilinx_geom::{self as geom, BondPin, CfgPin, Bond, GtPin, DisabledPart, BelCoord};
use prjcombine_xilinx_geom::spartan6::{self, ColumnKind, ColumnIoKind, Gts, Mcb, McbIo};

use itertools::Itertools;

use crate::grid::{extract_int, find_columns, find_column, find_rows, find_row, find_tiles, IntGrid, PreDevice, make_device};

fn make_columns(rd: &Part, int: &IntGrid) -> Vec<ColumnKind> {
    let mut res: Vec<Option<ColumnKind>> = Vec::new();
    for _ in 0..int.cols.len() {
        res.push(None);
    }
    for c in find_columns(rd, &["CLEXL", "CLEXL_DUMMY"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::CleXL);
    }
    for c in find_columns(rd, &["CLEXM", "CLEXM_DUMMY"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::CleXM);
    }
    for c in find_columns(rd, &["BRAMSITE2", "BRAMSITE2_DUMMY"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Bram);
    }
    for c in find_columns(rd, &["MACCSITE2"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::Dsp);
    }
    for c in find_columns(rd, &["RIOI", "LIOI"]) {
        res[int.lookup_column(c - 1) as usize] = Some(ColumnKind::Io);
    }
    for c in find_columns(rd, &["CLKC"]) {
        res[int.lookup_column(c - 3) as usize] = Some(ColumnKind::CleClk);
    }
    for c in find_columns(rd, &["GTPDUAL_DSP_FEEDTHRU"]) {
        res[int.lookup_column(c - 2) as usize] = Some(ColumnKind::DspPlus);
    }
    res.into_iter().map(|x| x.unwrap()).collect()
}

fn get_cols_io(rd: &Part, int: &IntGrid, top: bool) -> Vec<ColumnIoKind> {
    int.cols.iter().map(|&x| {
        let co = Coord {
            x: x as u16 + 1,
            y: if top {rd.height - 3} else {2},
        };
        let ci = Coord {
            x: x as u16 + 1,
            y: if top {rd.height - 4} else {3},
        };
        let has_o = rd.tiles[&co].kind.ends_with("IOI_OUTER");
        let has_i = rd.tiles[&ci].kind.ends_with("IOI_INNER");
        match (has_o, has_i) {
            (false, false) => ColumnIoKind::None,
            (false, true) => ColumnIoKind::Inner,
            (true, false) => ColumnIoKind::Outer,
            (true, true) => ColumnIoKind::Both,
        }

    }).collect()
}

fn get_rows_io(rd: &Part, int: &IntGrid, right: bool) -> Vec<bool> {
    int.rows.iter().map(|&y| {
        let c = Coord {
            x: if right {rd.width - 4} else {3},
            y: y as u16,
        };
        matches!(&rd.tiles[&c].kind[..], "LIOI" | "RIOI" | "LIOI_BRK" | "RIOI_BRK")
    }).collect()
}

fn get_cols_clk_fold(rd: &Part, int: &IntGrid) -> Option<(u32, u32)> {
    let v: Vec<_> = find_columns(rd, &["DSP_HCLK_GCLK_FOLD"]).into_iter().map(|x| int.lookup_column(x - 2)).sorted().collect();
    match &v[..] {
        &[] => None,
        &[l, r] => Some((l, r)),
        _ => unreachable!(),
    }
}

fn get_cols_reg_buf(rd: &Part, int: &IntGrid) -> (u32, u32) {
    let l = if let Some(c) = find_column(rd, &["REGH_BRAM_FEEDTHRU_L_GCLK", "REGH_DSP_L"]) {
        int.lookup_column(c - 2)
    } else if let Some(c) = find_column(rd, &["REGH_CLEXM_INT_GCLKL"]) {
        int.lookup_column(c)
    } else {
        unreachable!()
    };
    let r = if let Some(c) = find_column(rd, &["REGH_BRAM_FEEDTHRU_R_GCLK", "REGH_DSP_R"]) {
        int.lookup_column(c - 2)
    } else if let Some(c) = find_column(rd, &["REGH_CLEXL_INT_CLK"]) {
        int.lookup_column(c)
    } else {
        unreachable!()
    };
    (l, r)
}

fn get_rows_midbuf(rd: &Part, int: &IntGrid) -> (u32, u32) {
    let b = int.lookup_row(find_row(rd, &["REG_V_MIDBUF_BOT"]).unwrap());
    let t = int.lookup_row(find_row(rd, &["REG_V_MIDBUF_TOP"]).unwrap());
    (b, t)
}

fn get_rows_hclkbuf(rd: &Part, int: &IntGrid) -> (u32, u32) {
    let b = int.lookup_row(find_row(rd, &["REG_V_HCLKBUF_BOT"]).unwrap());
    let t = int.lookup_row(find_row(rd, &["REG_V_HCLKBUF_TOP"]).unwrap());
    (b, t)
}

fn get_rows_bank_split(rd: &Part, int: &IntGrid) -> Option<(u32, u32)> {
    if let Some(x) = find_row(rd, &["MCB_CAP_INT_BRK"]) {
        let l = int.lookup_row(x);
        let r = int.lookup_row(x) - 1;
        Some((l, r))
    } else {
        None
    }
}

fn get_rows_bufio_split(rd: &Part, int: &IntGrid) -> (u32, u32) {
    let b = int.lookup_row_inter(find_row(rd, &["HCLK_IOIL_BOT_SPLIT"]).unwrap());
    let t = int.lookup_row_inter(find_row(rd, &["HCLK_IOIL_TOP_SPLIT"]).unwrap());
    (b, t)
}

fn get_gts(rd: &Part, int: &IntGrid) -> Gts {
    let vt: Vec<_> = find_columns(rd, &["GTPDUAL_TOP", "GTPDUAL_TOP_UNUSED"]).into_iter().map(|x| int.lookup_column(x - 2)).sorted().collect();
    let vb: Vec<_> = find_columns(rd, &["GTPDUAL_BOT", "GTPDUAL_BOT_UNUSED"]).into_iter().map(|x| int.lookup_column(x - 2)).sorted().collect();
    match (&vt[..], &vb[..]) {
        (&[], &[]) => Gts::None,
        (&[l], &[]) => Gts::Single(l),
        (&[l, r], &[]) => Gts::Double(l, r),
        (&[l, r], &[_, _]) => Gts::Quad(l, r),
        _ => unreachable!(),
    }
}

fn get_mcbs(rd: &Part, int: &IntGrid) -> Vec<Mcb> {
    let mut res = Vec::new();
    #[allow(non_snake_case)]
    let P = |row, bel| McbIo { row, bel };
    for r in find_rows(rd, &["MCB_L", "MCB_DUMMY"]) {
        let row_mcb = int.lookup_row(r - 6);
        res.push(Mcb {
            row_mcb,
            row_mui: [
                row_mcb - 13,
                row_mcb - 16,
                row_mcb - 19,
                row_mcb - 22,
                row_mcb - 25,
                row_mcb - 28,
                row_mcb - 31,
                row_mcb - 34,
            ],
            iop_dq: [
                row_mcb - 20,
                row_mcb - 17,
                row_mcb - 10,
                row_mcb - 11,
                row_mcb - 23,
                row_mcb - 26,
                row_mcb - 32,
                row_mcb - 35,
            ],
            iop_dqs: [
                row_mcb - 14,
                row_mcb - 29,
            ],
            io_dm: [
                P(row_mcb - 9, 1),
                P(row_mcb - 9, 0),
            ],
            iop_clk: row_mcb - 3,
            io_addr: [
                P(row_mcb - 2, 0),
                P(row_mcb - 2, 1),
                P(row_mcb + 12, 1),
                P(row_mcb - 4, 0),
                P(row_mcb + 14, 1),
                P(row_mcb - 5, 0),
                P(row_mcb - 5, 1),
                P(row_mcb + 12, 0),
                P(row_mcb + 15, 0),
                P(row_mcb + 15, 1),
                P(row_mcb + 14, 0),
                P(row_mcb + 18, 1),
                P(row_mcb + 16, 1),
                P(row_mcb + 20, 0),
                P(row_mcb + 20, 1),
            ],
            io_ba: [
                P(row_mcb - 1, 0),
                P(row_mcb - 1, 1),
                P(row_mcb + 13, 1),
            ],
            io_ras: P(row_mcb - 6, 0),
            io_cas: P(row_mcb - 6, 1),
            io_we: P(row_mcb + 13, 0),
            io_odt: P(row_mcb - 4, 1),
            io_cke: P(row_mcb + 16, 0),
            io_reset: P(row_mcb + 18, 0),
        });
    }
    for r in find_rows(rd, &["MCB_L_BOT"]) {
        let row_mcb = int.lookup_row(r - 6);
        res.push(Mcb {
            row_mcb,
            row_mui: [
                row_mcb - 21,
                row_mcb - 24,
                row_mcb - 27,
                row_mcb - 30,
                row_mcb - 34,
                row_mcb - 37,
                row_mcb - 40,
                row_mcb - 43,
            ],
            iop_dq: [
                row_mcb - 28,
                row_mcb - 25,
                row_mcb - 18,
                row_mcb - 19,
                row_mcb - 31,
                row_mcb - 35,
                row_mcb - 41,
                row_mcb - 44,
            ],
            iop_dqs: [
                row_mcb - 22,
                row_mcb - 38,
            ],
            io_dm: [
                P(row_mcb - 17, 1),
                P(row_mcb - 17, 0),
            ],
            iop_clk: row_mcb - 8,
            io_addr: [
                P(row_mcb - 7, 0),
                P(row_mcb - 7, 1),
                P(row_mcb - 4, 1),
                P(row_mcb - 10, 0),
                P(row_mcb - 1, 1),
                P(row_mcb - 13, 0),
                P(row_mcb - 13, 1),
                P(row_mcb - 4, 0),
                P(row_mcb + 12, 0),
                P(row_mcb + 12, 1),
                P(row_mcb - 1, 0),
                P(row_mcb + 14, 1),
                P(row_mcb + 13, 1),
                P(row_mcb + 15, 0),
                P(row_mcb + 15, 1),
            ],
            io_ba: [
                P(row_mcb - 5, 0),
                P(row_mcb - 5, 1),
                P(row_mcb - 2, 1),
            ],
            io_ras: P(row_mcb - 14, 0),
            io_cas: P(row_mcb - 14, 1),
            io_we: P(row_mcb - 2, 0),
            io_odt: P(row_mcb - 10, 1),
            io_cke: P(row_mcb + 13, 0),
            io_reset: P(row_mcb + 14, 0),
        });
    }
    res.sort_by_key(|x| x.row_mcb);
    res
}

fn has_encrypt(rd: &Part) -> bool {
    for pins in rd.packages.values() {
        for pin in pins {
            if pin.func == "VBATT" {
                return true;
            }
        }
    }
    false
}

fn set_cfg(grid: &mut spartan6::Grid, cfg: CfgPin, coord: BelCoord) {
    let old = grid.cfg_io.insert(cfg, coord);
    assert!(old.is_none() || old == Some(coord));
}

fn handle_spec_io(rd: &Part, grid: &mut spartan6::Grid) {
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    let mut novref = BTreeSet::new();
    for pins in rd.packages.values() {
        for pin in pins {
            if let Some(ref pad) = pin.pad {
                if !pad.starts_with("PAD") {
                    continue;
                }
                let coord = io_lookup[pad];
                let mut is_vref = false;
                let mut f = pin.func.strip_prefix("IO_L").unwrap();
                f = &f[f.find('_').unwrap()+1..];
                if f.starts_with("GCLK") {
                    // ignore
                    f = &f[f.find('_').unwrap()+1..];
                }
                if f.starts_with("IRDY") || f.starts_with("TRDY") {
                    // ignore
                    f = &f[f.find('_').unwrap()+1..];
                }
                for (p, c) in [
                    ("M0_CMPMISO_", CfgPin::M0),
                    ("M1_", CfgPin::M1),
                    ("CCLK_", CfgPin::Cclk),
                    ("CSO_B_", CfgPin::CsoB),
                    ("INIT_B_", CfgPin::InitB),
                    ("RDWR_B_", CfgPin::RdWrB),
                    ("AWAKE_", CfgPin::Awake),
                    ("FCS_B_", CfgPin::FcsB),
                    ("FOE_B_", CfgPin::FoeB),
                    ("FWE_B_", CfgPin::FweB),
                    ("LDC_", CfgPin::Ldc(0)),
                    ("HDC_", CfgPin::Hdc),
                    ("DOUT_BUSY_", CfgPin::Dout),
                    ("D0_DIN_MISO_MISO1_", CfgPin::Data(0)),
                    ("D1_MISO2_", CfgPin::Data(1)),
                    ("D2_MISO3_", CfgPin::Data(2)),
                    ("MOSI_CSI_B_MISO0_", CfgPin::CsiB),
                    ("CMPCLK_", CfgPin::CmpClk),
                    ("CMPMOSI_", CfgPin::CmpMosi),
                    ("USERCCLK_", CfgPin::UserCclk),
                    ("HSWAPEN_", CfgPin::HswapEn),
                ] {
                    if let Some(nf) = f.strip_prefix(p) {
                        f = nf;
                        set_cfg(grid, c, coord);
                    }
                }
                if f.starts_with('A') {
                    let pos = f.find('_').unwrap();
                    let a = f[1..pos].parse().unwrap();
                    set_cfg(grid, CfgPin::Addr(a), coord);
                    f = &f[pos+1..];
                }
                if f.starts_with('D') {
                    let pos = f.find('_').unwrap();
                    let a = f[1..pos].parse().unwrap();
                    set_cfg(grid, CfgPin::Data(a), coord);
                    f = &f[pos+1..];
                }
                if f.starts_with("SCP") {
                    let pos = f.find('_').unwrap();
                    let a = f[3..pos].parse().unwrap();
                    set_cfg(grid, CfgPin::Scp(a), coord);
                    f = &f[pos+1..];
                }
                if let Some(nf) = f.strip_prefix("VREF_") {
                    f = nf;
                    is_vref = true;
                }
                if f.starts_with("M") {
                    let (col, mi) = match &f[0..2] {
                        "M1" => (grid.columns.len() as u32 - 1, 0),
                        "M3" => (0, 0),
                        "M4" => (0, 1),
                        "M5" => (grid.columns.len() as u32 - 1, 1),
                        _ => unreachable!(),
                    };
                    assert_eq!(coord.col, col);
                    let mcb = &grid.mcbs[mi];
                    let epos = f.find('_').unwrap();
                    let mf = &f[2..epos];
                    match mf {
                        "RASN" => {
                            assert_eq!(coord.row, mcb.io_ras.row);
                            assert_eq!(coord.bel, mcb.io_ras.bel);
                        }
                        "CASN" => {
                            assert_eq!(coord.row, mcb.io_cas.row);
                            assert_eq!(coord.bel, mcb.io_cas.bel);
                        }
                        "WE" => {
                            assert_eq!(coord.row, mcb.io_we.row);
                            assert_eq!(coord.bel, mcb.io_we.bel);
                        }
                        "ODT" => {
                            assert_eq!(coord.row, mcb.io_odt.row);
                            assert_eq!(coord.bel, mcb.io_odt.bel);
                        }
                        "CKE" => {
                            assert_eq!(coord.row, mcb.io_cke.row);
                            assert_eq!(coord.bel, mcb.io_cke.bel);
                        }
                        "RESET" => {
                            assert_eq!(coord.row, mcb.io_reset.row);
                            assert_eq!(coord.bel, mcb.io_reset.bel);
                        }
                        "LDM" => {
                            assert_eq!(coord.row, mcb.io_dm[0].row);
                            assert_eq!(coord.bel, mcb.io_dm[0].bel);
                        }
                        "UDM" => {
                            assert_eq!(coord.row, mcb.io_dm[1].row);
                            assert_eq!(coord.bel, mcb.io_dm[1].bel);
                        }
                        "LDQS" => {
                            assert_eq!(coord.row, mcb.iop_dqs[0]);
                            assert_eq!(coord.bel, 0);
                        }
                        "LDQSN" => {
                            assert_eq!(coord.row, mcb.iop_dqs[0]);
                            assert_eq!(coord.bel, 1);
                        }
                        "UDQS" => {
                            assert_eq!(coord.row, mcb.iop_dqs[1]);
                            assert_eq!(coord.bel, 0);
                        }
                        "UDQSN" => {
                            assert_eq!(coord.row, mcb.iop_dqs[1]);
                            assert_eq!(coord.bel, 1);
                        }
                        "CLK" => {
                            assert_eq!(coord.row, mcb.iop_clk);
                            assert_eq!(coord.bel, 0);
                        }
                        "CLKN" => {
                            assert_eq!(coord.row, mcb.iop_clk);
                            assert_eq!(coord.bel, 1);
                        }
                        _ => {
                            if mf.starts_with("A") {
                                let i: usize = mf[1..].parse().unwrap();
                                assert_eq!(coord.row, mcb.io_addr[i].row);
                                assert_eq!(coord.bel, mcb.io_addr[i].bel);
                            } else if mf.starts_with("BA") {
                                let i: usize = mf[2..].parse().unwrap();
                                assert_eq!(coord.row, mcb.io_ba[i].row);
                                assert_eq!(coord.bel, mcb.io_ba[i].bel);
                            } else if mf.starts_with("DQ") {
                                let i: usize = mf[2..].parse().unwrap();
                                assert_eq!(coord.row, mcb.iop_dq[i/2]);
                                assert_eq!(coord.bel, (i % 2) as u32);
                            } else {
                                println!("MCB {}", mf);
                            }
                        }
                    }
                    f = &f[epos+1..];
                }
                if !matches!(f, "0" | "1" | "2" | "3" | "4" | "5") {
                    println!("FUNC {f}");
                }
                if is_vref {
                    grid.vref.insert(coord);
                } else {
                    novref.insert(coord);
                }
            }
        }
    }
    for c in novref {
        assert!(!grid.vref.contains(&c));
    }
}

fn make_grid(rd: &Part) -> (spartan6::Grid, BTreeSet<DisabledPart>) {
    let int = extract_int(rd, &[
        "INT",
        "INT_BRK",
        "INT_BRAM",
        "INT_BRAM_BRK",
        "IOI_INT",
        "LIOI_INT",
    ], &[]);
    let mut disabled = BTreeSet::new();
    if !find_tiles(rd, &["GTPDUAL_TOP_UNUSED"]).is_empty() {
        disabled.insert(DisabledPart::Spartan6Gtp);
    }
    if !find_tiles(rd, &["MCB_DUMMY"]).is_empty() {
        disabled.insert(DisabledPart::Spartan6Mcb);
    }
    for c in find_columns(rd, &["CLEXL_DUMMY", "CLEXM_DUMMY"]) {
        let c = int.lookup_column(c - 1);
        disabled.insert(DisabledPart::Spartan6ClbColumn(c));
    }
    for (c, r) in find_tiles(rd, &["BRAMSITE2_DUMMY"]) {
        let c = int.lookup_column(c - 2);
        let r = int.lookup_row(r) / 16;
        disabled.insert(DisabledPart::Spartan6BramRegion(c, r));
    }
    for (c, r) in find_tiles(rd, &["MACCSITE2_DUMMY"]) {
        let c = int.lookup_column(c - 2);
        let r = int.lookup_row(r) / 16;
        disabled.insert(DisabledPart::Spartan6DspRegion(c, r));
    }
    let columns = make_columns(rd, &int);
    let col_clk = columns.iter().position(|&x| x == ColumnKind::CleClk).unwrap() as u32;
    let mut grid = spartan6::Grid {
        columns,
        cols_bio: get_cols_io(rd, &int, false),
        cols_tio: get_cols_io(rd, &int, true),
        col_clk,
        cols_clk_fold: get_cols_clk_fold(rd, &int),
        cols_reg_buf: get_cols_reg_buf(rd, &int),
        rows: int.rows.len() as u32 / 16,
        rows_lio: get_rows_io(rd, &int, false),
        rows_rio: get_rows_io(rd, &int, true),
        rows_midbuf: get_rows_midbuf(rd, &int),
        rows_hclkbuf: get_rows_hclkbuf(rd, &int),
        rows_bank_split: get_rows_bank_split(rd, &int),
        rows_bufio_split: get_rows_bufio_split(rd, &int),
        gts: get_gts(rd, &int),
        mcbs: get_mcbs(rd, &int),
        vref: BTreeSet::new(),
        cfg_io: BTreeMap::new(),
        has_encrypt: has_encrypt(rd),
    };
    handle_spec_io(rd, &mut grid);
    (grid, disabled)
}

fn split_num(s: &str) -> Option<(&str, u32)> {
    let mut pos = s.find(|c: char| c.is_digit(10))?;
    if let Some(upos) = s.find('_') {
        pos = upos + 1;
    }
    let n = s[pos..].parse().ok()?;
    Some((&s[..pos], n))
}

fn make_bond(grid: &spartan6::Grid, disabled: &BTreeSet<DisabledPart>, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, (io.coord, io.bank)))
        .collect();
    let gt_lookup: HashMap<_, _> = grid
        .get_gt(disabled)
        .into_iter()
        .flat_map(|gt| gt.get_pads().into_iter().map(move |(name, func, pin, idx)| (name, (func, gt.bank, pin, idx))))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&(coord, bank)) = io_lookup.get(pad) {
                //assert_eq!(pin.vref_bank, Some(bank));
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByCoord(coord)
            } else if let Some(&(ref exp_func, bank, gpin, idx)) = gt_lookup.get(pad) {
                if *exp_func != pin.func {
                    println!("pad {pad} got {f} exp {exp_func}", f=pin.func);
                }
                BondPin::GtByBank(bank, gpin, idx)
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
                "VBATT" => BondPin::VccBatt,
                "VFS" => BondPin::Vfs,
                "RFUSE" => BondPin::RFuse,
                "TCK" => BondPin::Cfg(CfgPin::Tck),
                "TDI" => BondPin::Cfg(CfgPin::Tdi),
                "TDO" => BondPin::Cfg(CfgPin::Tdo),
                "TMS" => BondPin::Cfg(CfgPin::Tms),
                "CMPCS_B_2" => BondPin::Cfg(CfgPin::CmpCsB),
                "DONE_2" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM_B_2" => BondPin::Cfg(CfgPin::ProgB),
                "SUSPEND" => BondPin::Cfg(CfgPin::Suspend),
                _ => if let Some((n, b)) = split_num(&pin.func) {
                    match n {
                        "VCCO_" => BondPin::VccO(b),
                        "MGTAVCC_" => BondPin::GtByBank(b, GtPin::AVcc, 0),
                        "MGTAVCCPLL0_" => BondPin::GtByBank(b, GtPin::AVccPll, 0),
                        "MGTAVCCPLL1_" => BondPin::GtByBank(b, GtPin::AVccPll, 1),
                        "MGTAVTTRX_" => BondPin::GtByBank(b, GtPin::VtRx, 0),
                        "MGTAVTTTX_" => BondPin::GtByBank(b, GtPin::VtTx, 0),
                        "MGTRREF_" => BondPin::GtByBank(b, GtPin::RRef, 0),
                        "MGTAVTTRCAL_" => BondPin::GtByBank(b, GtPin::AVttRCal, 0),
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
        io_banks,
    }
}

pub fn ingest(rd: &Part) -> PreDevice {
    let (grid, disabled) = make_grid(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(&grid, &disabled, pins),
        ));
    }
    make_device(rd, geom::Grid::Spartan6(grid), bonds, disabled)
}
