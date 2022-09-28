#![allow(clippy::needless_range_loop)]
use ndarray::Array2;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub name: String,
    pub kind: String,
    pub width: usize,
    pub height: usize,
    pub x: usize,
    pub y: usize,
    pub sites: Vec<Site>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub struct Site {
    pub name: String,
    pub x: i32,
    pub y: i32,
}

pub fn parse_tiles(data: &str, arch: &str) -> Array2<Tile> {
    let tile_re = Regex::new(
        r"Tile ([A-Z0-9_]+) \((\d+), (\d+)\)  bitmap offset \((\d+), (\d+)\)  <([A-Z0-9_]+)>$",
    )
    .unwrap();
    let site_re = Regex::new(r"  ([A-Z0-9_]+) \((-?\d+), (-?\d+)\)$").unwrap();
    let mut tiles = vec![];
    let mut tile = None;
    for line in data.lines() {
        if let Some(cap) = tile_re.captures(line) {
            if let Some(tile) = tile {
                tiles.push(tile);
            }
            tile = Some(Tile {
                name: cap[6].to_string(),
                kind: cap[1].to_string(),
                height: cap[2].parse().unwrap(),
                width: cap[3].parse().unwrap(),
                x: cap[5].parse().unwrap(),
                y: cap[4].parse().unwrap(),
                sites: vec![],
            });
        } else if let Some(cap) = site_re.captures(line) {
            tile.as_mut().unwrap().sites.push(Site {
                name: cap[1].to_string(),
                x: cap[3].parse().unwrap(),
                y: cap[2].parse().unwrap(),
            });
        } else if line.starts_with("Tile") || line.starts_with("  ") {
            panic!("regex failed: {line:?}");
        } else {
            assert!(tile.is_none());
        }
    }
    if let Some(tile) = tile {
        tiles.push(tile);
    }
    let mut rows = vec![];
    let mut row = vec![];
    let mut lx = 0;
    for tile in tiles {
        if tile.x == 0 && lx != 0 {
            rows.push(row);
            row = vec![];
        }
        lx = tile.x;
        row.push(tile);
    }
    rows.push(row);
    match arch {
        "scm" => {
            let mut cswz = vec![];
            let row = &rows[rows.len() - 6];
            let mut cols_pclk = HashSet::new();
            let mut insert_pclk = HashMap::new();
            for (i, tile) in row.iter().enumerate() {
                if tile.kind == "PCLK" {
                    cols_pclk.insert(i);
                    let cp = tile.name.rfind('C').unwrap();
                    let c: usize = tile.name[cp + 1..].parse().unwrap();
                    insert_pclk.insert(c, i);
                }
            }
            for i in 0..rows[0].len() {
                if cols_pclk.contains(&i) {
                    continue;
                }
                if let Some(&idx) = insert_pclk.get(&cswz.len()) {
                    cswz.push(idx);
                }
                cswz.push(i);
            }
            let mut rswz = vec![];
            rswz.push(rows.len() - 1);
            let row_hiq = rows
                .iter()
                .position(|x| x.iter().any(|y| y.kind == "HALFMUXT"))
                .unwrap()
                + 5;
            for i in 0..(rows.len() - 2) {
                if i == row_hiq {
                    rswz.push(rows.len() - 2);
                }
                rswz.push(i);
            }
            Array2::from_shape_fn((rows.len(), rows[0].len()), |(r, c)| {
                rows[rswz[r]][cswz[c]].clone()
            })
        }
        "ecp5" => {
            let row = &rows[1];
            let cib_l_idx = row.iter().position(|x| x.name == "CIB_R1C1").unwrap();
            let mut swz = vec![];
            swz.push(0);
            let mut tap = HashMap::new();
            for i in 1..(cib_l_idx - 1) {
                let tile = &row[i];
                let c: usize = tile.name.strip_prefix("TAP_R1C").unwrap().parse().unwrap();
                tap.insert(c, i);
            }
            for i in cib_l_idx..row.len() {
                let tile = &row[i];
                let c: usize = tile.name.strip_prefix("CIB_R1C").unwrap().parse().unwrap();
                if let Some(&idx) = tap.get(&c) {
                    swz.push(idx);
                }
                swz.push(i);
            }
            swz.push(cib_l_idx - 1);
            Array2::from_shape_fn((rows.len(), rows[0].len()), |(r, c)| {
                rows[r][swz[c]].clone()
            })
        }
        "crosslink" => {
            let row = &rows[2];
            let cib_l_idx = row.iter().position(|x| x.name == "CIB_R1C1").unwrap();
            let mut swz = vec![];
            let mut tap = HashMap::new();
            for i in 0..(cib_l_idx) {
                let tile = &row[i];
                let c: usize = tile.name.strip_prefix("TAP_R1C").unwrap().parse().unwrap();
                tap.insert(c, i);
            }
            for i in cib_l_idx..row.len() {
                let tile = &row[i];
                let c: usize = tile.name.strip_prefix("CIB_R1C").unwrap().parse().unwrap();
                if let Some(&idx) = tap.get(&c) {
                    swz.push(idx);
                }
                swz.push(i);
            }
            Array2::from_shape_fn((rows.len(), rows[0].len()), |(r, c)| {
                rows[r][swz[c]].clone()
            })
        }
        "machxo2" => {
            let row_t = rows
                .iter()
                .position(|row| row.last().unwrap().kind == "CENTER_T")
                .unwrap();
            let mut cswz = vec![];
            let col_c = rows[row_t]
                .iter()
                .position(|x| x.kind.starts_with("PIC_T_DUMMY_VIQ"))
                .unwrap();
            for i in 0..(rows[0].len() - 1) {
                cswz.push(i);
                if i == col_c {
                    cswz.push(rows[0].len() - 1);
                }
            }
            let mut rswz = vec![];
            rswz.push(row_t);
            let mut row_ebr = row_t + 1;
            for i in 0..row_t {
                rswz.push(i);
                if rows[i]
                    .iter()
                    .any(|x| x.kind.starts_with("CIB_EBR") && !x.kind.contains("640"))
                {
                    rswz.push(row_ebr);
                    row_ebr += 1;
                }
            }
            Array2::from_shape_fn((rows.len(), rows[0].len()), |(r, c)| {
                rows[rswz[r]][cswz[c]].clone()
            })
        }
        "nx" => {
            let row = &rows[1];
            let cib_l_idx = row.iter().position(|x| x.name == "CIB_R1C1").unwrap();
            let mut swz = vec![];
            swz.push(0);
            let mut col_tap = 1;
            let row_s = rows
                .iter()
                .position(|row| row.iter().any(|x| x.kind.starts_with("SPINE")))
                .unwrap();
            let cols_tap: HashSet<_> = rows[row_s]
                .iter()
                .enumerate()
                .filter_map(|(i, x)| {
                    if x.kind.starts_with("SPINE") {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();
            for i in cib_l_idx..row.len() {
                swz.push(i);
                if cols_tap.contains(&i) {
                    swz.push(col_tap);
                    col_tap += 1;
                }
            }
            swz.push(cib_l_idx - 1);
            Array2::from_shape_fn((rows.len(), rows[0].len()), |(r, c)| {
                rows[r][swz[c]].clone()
            })
        }
        _ => Array2::from_shape_fn((rows.len(), rows[0].len()), |(r, c)| rows[r][c].clone()),
    }
}

const COLORS: &[(&str, (u8, u8, u8))] = &[
    ("int", (204, 204, 204)),
    ("int-buf", (102, 102, 255)),
    ("int-if", (153, 153, 255)),
    ("clb", (204, 255, 204)),
    ("clbm", (0, 255, 0)),
    ("io", (255, 153, 255)),
    ("io-spec", (204, 153, 204)),
    ("iob", (255, 51, 255)),
    ("iob-spec", (204, 51, 204)),
    ("iobank", (255, 0, 255)),
    ("gtp", (102, 0, 255)),
    ("gtx", (153, 0, 255)),
    ("gth", (153, 102, 255)),
    ("gty", (153, 153, 255)),
    ("gtm", (102, 0, 255)),
    ("gtclk", (204, 51, 204)),
    ("sysmon", (153, 0, 255)),
    ("hsdac", (102, 0, 255)),
    ("hsadc", (153, 0, 255)),
    ("cfg", (255, 102, 0)),
    ("bram", (0, 0, 255)),
    ("uram", (0, 0, 153)),
    ("dsp", (0, 255, 255)),
    ("clk-last-buf", (255, 255, 0)),
    ("clk-row-buf", (255, 204, 0)),
    ("clk-row", (255, 255, 204)),
    ("clk-spine-buf", (255, 153, 0)),
    ("clk-spine", (255, 255, 153)),
    ("clk-global-buf", (204, 102, 0)),
    ("pll", (102, 102, 0)),
    ("pll-alt", (153, 102, 0)),
    ("hardip", (255, 102, 102)),
    ("noc", (255, 204, 204)),
    ("crippled", (102, 102, 102)),
    ("clk-brk", (153, 153, 153)),
    ("vbrk", (102, 102, 102)),
    ("tterm", (0, 0, 0)),
    ("lterm", (0, 0, 0)),
    ("rterm", (0, 0, 0)),
    ("bterm", (0, 0, 0)),
];

#[derive(Copy, Clone)]
struct TileInfo(&'static str, &'static [&'static str]);

const XP_TILES: &[TileInfo] = &[
    TileInfo("PLC", &["clbm"]),
    TileInfo("FPLC", &["clb"]),
    TileInfo("EMB0", &["bram"]),
    TileInfo("EMB1", &["bram"]),
    TileInfo("PIC_L", &["io"]),
    TileInfo("PIC_L_6K_CONFIG", &["io-spec"]),
    TileInfo("PIC_L_A", &["io-spec"]),
    TileInfo("PIC_L_A_20K", &["io-spec"]),
    TileInfo("PIC_L_B", &["io-spec"]),
    TileInfo("PIC_L_B_20K", &["io-spec"]),
    TileInfo("PIC_LDQS", &["io-spec"]),
    TileInfo("PIC_R", &["io"]),
    TileInfo("PIC_R_3K_CONFIG", &["io-spec"]),
    TileInfo("PIC_R_A", &["io-spec"]),
    TileInfo("PIC_R_A_20K", &["io-spec"]),
    TileInfo("PIC_R_B", &["io-spec"]),
    TileInfo("PIC_R_B_20K", &["io-spec"]),
    TileInfo("PIC_RDQS", &["io-spec"]),
    TileInfo("PIC_B_NO_IO", &["io", "crippled"]),
    TileInfo("PIC_BL", &["io"]),
    TileInfo("PIC_BL_A", &["io-spec"]),
    TileInfo("PIC_BL_B", &["io-spec"]),
    TileInfo("PIC_BLDQS", &["io-spec"]),
    TileInfo("PIC_BR", &["io"]),
    TileInfo("PIC_BR_A", &["io-spec"]),
    TileInfo("PIC_BR_B", &["io-spec"]),
    TileInfo("PIC_BRDQS", &["io-spec"]),
    TileInfo("PIC_T_NO_IO", &["io", "crippled"]),
    TileInfo("PIC_TL", &["io"]),
    TileInfo("PIC_TL_A", &["io"]),
    TileInfo("PIC_TL_A_CFG", &["io-spec"]),
    TileInfo("PIC_TL_AB_CFG", &["io-spec"]),
    TileInfo("PIC_TL_A_ONLY_CFG", &["io-spec"]),
    TileInfo("PIC_TL_B", &["io-spec"]),
    TileInfo("PIC_TLDQS", &["io-spec"]),
    TileInfo("PIC_TR", &["io"]),
    TileInfo("PIC_TR_A", &["io-spec"]),
    TileInfo("PIC_TR_A_CFG", &["io-spec"]),
    TileInfo("PIC_TR_AB_CFG", &["io-spec"]),
    TileInfo("PIC_TR_A_ONLY_CFG", &["io-spec"]),
    TileInfo("PIC_TR_B", &["io-spec"]),
    TileInfo("PIC_TR_B_CFG", &["io-spec"]),
    TileInfo("PIC_TRDQS", &["io-spec"]),
    TileInfo("CIB_LL_COR_20K", &["int"]),
    TileInfo("CIB_LR_COR_20K", &["int"]),
    TileInfo("CIB_UL_COR_20K", &["int"]),
    TileInfo("CIB_UR_COR_20K", &["int"]),
    TileInfo("CIB_LL_COR", &["int"]),
    TileInfo("CIB_LR_COR", &["int"]),
    TileInfo("CIB_UL_COR", &["int"]),
    TileInfo("CIB_UR_COR", &["int"]),
    TileInfo("CIB_LL_COR_6K", &["int"]),
    TileInfo("CIB_UL_COR_6K", &["int"]),
    TileInfo("L_CIB_DUMMY", &["int"]),
    TileInfo("R_CIB_DUMMY", &["int"]),
    TileInfo("CIB_DUMMY", &["int"]),
    TileInfo("BDLL", &["pll-alt"]),
    TileInfo("TDLL", &["pll-alt"]),
    TileInfo("TDLL_15K", &["pll-alt"]),
    TileInfo("PLL3A", &["pll"]),
    TileInfo("PLL3B", &["pll"]),
    TileInfo("PLL3C", &["pll"]),
    TileInfo("PLL3D", &["pll"]),
    TileInfo("CIB_DCSPLL", &["pll"]),
    TileInfo("VIQ", &["clk-row"]),
    TileInfo("VIQ_EMB", &["clk-row"]),
    TileInfo("CLK0_6K", &["clk-row-buf"]),
    TileInfo("CLK1_6K", &["clk-row-buf"]),
    TileInfo("CLK4_6K", &["clk-row-buf"]),
    TileInfo("CLK5_6K", &["clk-row-buf"]),
    TileInfo("CLK6_6K", &["clk-row-buf"]),
    TileInfo("CLK7_6K", &["clk-row-buf"]),
    TileInfo("CLK8_6K", &["clk-row-buf"]),
    TileInfo("CLK9_6K", &["clk-row-buf"]),
    TileInfo("CLK10_6K", &["clk-row-buf"]),
    TileInfo("CLK11_6K", &["clk-row-buf"]),
    TileInfo("CLK12_6K", &["clk-row-buf"]),
    TileInfo("CLK13_6K", &["clk-row-buf"]),
    TileInfo("CLK14_6K", &["clk-row-buf"]),
    TileInfo("CLK15_6K", &["clk-row-buf"]),
    TileInfo("CLKS", &["clk-row-buf"]),
    TileInfo("CLK0", &["clk-row-buf"]),
    TileInfo("CLK1", &["clk-row-buf"]),
    TileInfo("CLK2", &["clk-row-buf"]),
    TileInfo("CLK3", &["clk-row-buf"]),
    TileInfo("CLK4", &["clk-row-buf"]),
    TileInfo("CLK5", &["clk-row-buf"]),
    TileInfo("CLK6", &["clk-row-buf"]),
    TileInfo("CLK7", &["clk-row-buf"]),
    TileInfo("CLK8", &["clk-row-buf"]),
    TileInfo("CLK9", &["clk-row-buf"]),
    TileInfo("CLK10", &["clk-row-buf"]),
    TileInfo("CLK11", &["clk-row-buf"]),
    TileInfo("CONFIG", &["cfg"]),
];

const ECP_TILES: &[TileInfo] = &[
    TileInfo("PLC", &["clbm"]),
    TileInfo("FPLC", &["clb"]),
    TileInfo("EMB0", &["bram"]),
    TileInfo("EMB1", &["bram"]),
    TileInfo("DSP_0", &["dsp"]),
    TileInfo("DSP_1", &["dsp"]),
    TileInfo("DSP_2", &["dsp"]),
    TileInfo("DSP_3", &["dsp"]),
    TileInfo("DSP_4", &["dsp"]),
    TileInfo("DSP_5", &["dsp"]),
    TileInfo("DSP_6", &["dsp"]),
    TileInfo("DSP_7", &["dsp"]),
    TileInfo("PIC_L", &["io"]),
    TileInfo("PIC_LDQS", &["io-spec"]),
    TileInfo("PIC_R", &["io"]),
    TileInfo("PIC_RDQS", &["io-spec"]),
    TileInfo("PIC_RA", &["io-spec"]),
    TileInfo("PIC_RB", &["io-spec"]),
    TileInfo("PIC_T", &["io"]),
    TileInfo("PIC_TDQS", &["io-spec"]),
    TileInfo("PIC_B", &["io"]),
    TileInfo("PIC_BDQS", &["io-spec"]),
    TileInfo("PIC_BAB1", &["io-spec"]),
    TileInfo("PIC_BAB2", &["io-spec"]),
    TileInfo("PIC_BB1", &["io-spec"]),
    TileInfo("PIC_BB2", &["io-spec"]),
    TileInfo("PIC_BB3", &["io-spec"]),
    TileInfo("PIC_BDQSB", &["io-spec"]),
    TileInfo("CIB_DUMMY", &["int"]),
    TileInfo("L_CIB_DUMMY", &["int"]),
    TileInfo("R_CIB_DUMMY", &["int"]),
    TileInfo("CIB_LL_COR", &["int"]),
    TileInfo("CIB_LR_COR", &["int"]),
    TileInfo("CIB_UL_COR", &["int"]),
    TileInfo("CIB_UR_COR", &["int"]),
    TileInfo("EMB_DUMMY", &["int"]),
    TileInfo("CFG_1", &["cfg"]),
    TileInfo("CFG_2", &["cfg"]),
    TileInfo("CFGG_1", &["cfg"]),
    TileInfo("CFGG_2", &["cfg"]),
    TileInfo("VIQ_PICB", &["pll-alt"]),
    TileInfo("VIQ_PICT", &["pll-alt"]),
    TileInfo("PLL3A_L", &["pll"]),
    TileInfo("PLL3B_L", &["pll"]),
    TileInfo("PLL3A_R", &["pll"]),
    TileInfo("PLL3B_R", &["pll"]),
    TileInfo("PLL_DUMMY_L", &["pll"]),
    TileInfo("PLL_DUMMY_R", &["pll"]),
    TileInfo("VIQ", &["clk-row"]),
    TileInfo("VIQ_EMB", &["clk-row"]),
    TileInfo("DSP_VIQ", &["clk-row"]),
    TileInfo("CLK0", &["clk-row-buf"]),
    TileInfo("CLK1", &["clk-row-buf"]),
    TileInfo("CLK2", &["clk-row-buf"]),
    TileInfo("CLK3", &["clk-row-buf"]),
    TileInfo("CLK4", &["clk-row-buf"]),
    TileInfo("CLK5", &["clk-row-buf"]),
    TileInfo("CLK6", &["clk-row-buf"]),
    TileInfo("CLK7", &["clk-row-buf"]),
    TileInfo("CLK8", &["clk-row-buf"]),
    TileInfo("CLK9", &["clk-row-buf"]),
    TileInfo("CLK10", &["clk-row-buf"]),
    TileInfo("CLK11", &["clk-row-buf"]),
];

const MACHXO_TILES: &[TileInfo] = &[
    TileInfo("PLC", &["clbm"]),
    TileInfo("FPLC", &["clb"]),
    TileInfo("PIC_L", &["io"]),
    TileInfo("PIC2_L", &["io"]),
    TileInfo("PIC4_L", &["io"]),
    TileInfo("PIC_L_GSR", &["io", "cfg"]),
    TileInfo("PIC_L_OSC", &["io", "cfg"]),
    TileInfo("PIC_L_ISP", &["io", "cfg"]),
    TileInfo("PIC2_L_GSR", &["io", "cfg"]),
    TileInfo("PIC2_L_OSC", &["io", "cfg"]),
    TileInfo("PIC2_L_ISP", &["io", "cfg"]),
    TileInfo("PIC2_L_EBR1K_0", &["io", "bram"]),
    TileInfo("PIC4_L_EBR1K_1", &["io", "bram"]),
    TileInfo("PIC4_L_EBR1K_2", &["io", "bram"]),
    TileInfo("PIC4_L_EBR1K_3", &["io", "bram"]),
    TileInfo("PIC4_L_EBR1K_4", &["io", "bram"]),
    TileInfo("PIC4_L_EBR1K_5", &["io", "bram"]),
    TileInfo("PIC4_L_EBR1K_6", &["io", "bram"]),
    TileInfo("LLC_EBR2K_0", &["bram"]),
    TileInfo("PIC2_L_EBR2K_1", &["io", "bram"]),
    TileInfo("PIC2_L_EBR2K_2", &["io", "bram"]),
    TileInfo("PIC2_L_EBR2K_3", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_4", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_5", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_6", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_7", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_8", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_9", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_10", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_11", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_12", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_13", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_14", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_15", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_16", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_17", &["io", "bram"]),
    TileInfo("PIC4_L_EBR2K_18", &["io", "bram"]),
    TileInfo("PIC2_L_EBR2K_19", &["io", "bram"]),
    TileInfo("ULC_EBR2K_20", &["bram"]),
    TileInfo("PIC2_L_PLL1K", &["io", "pll"]),
    TileInfo("PIC_R", &["io"]),
    TileInfo("PIC2_R", &["io"]),
    TileInfo("PIC2_R_LVDS", &["io-spec"]),
    TileInfo("PIC4_R", &["io"]),
    TileInfo("PIC4_B", &["io"]),
    TileInfo("PIC6_B", &["io"]),
    TileInfo("PIC4_T", &["io"]),
    TileInfo("PIC6_T", &["io"]),
    TileInfo("LLC", &["int"]),
    TileInfo("LRC", &["int"]),
    TileInfo("ULC", &["int"]),
    TileInfo("URC", &["int"]),
    TileInfo("LLC256", &["int"]),
    TileInfo("LRC256", &["int"]),
    TileInfo("ULC256", &["int"]),
    TileInfo("URC256", &["int"]),
    TileInfo("CLK_DUMMY", &["clk-row"]),
    TileInfo("CLK_DUMMY_PICB", &["clk-row"]),
    TileInfo("CLK_DUMMY_PICT", &["clk-row"]),
    TileInfo("CLK0", &["clk-row-buf"]),
    TileInfo("CLK1", &["clk-row-buf"]),
    TileInfo("CLK2", &["clk-row-buf"]),
    TileInfo("CLK3", &["clk-row-buf"]),
    TileInfo("CLK4", &["clk-row-buf"]),
    TileInfo("CLK5", &["clk-row-buf"]),
    TileInfo("CLK0_2K", &["clk-row-buf"]),
    TileInfo("CLK1_2K", &["clk-row-buf"]),
    TileInfo("CLK2_2K", &["clk-row-buf"]),
    TileInfo("CLK3_2K", &["clk-row-buf"]),
    TileInfo("CLK4_2K", &["clk-row-buf"]),
    TileInfo("CLK5_2K", &["clk-row-buf"]),
];

const SCM_TILES: &[TileInfo] = &[
    TileInfo("PLC", &["clbm"]),
    TileInfo("EMBL0", &["bram"]),
    TileInfo("EMBL1", &["bram"]),
    TileInfo("LMACO1", &["hardip"]),
    TileInfo("LMACO2", &["hardip"]),
    TileInfo("LMACO3", &["hardip"]),
    TileInfo("LMACO4", &["hardip"]),
    TileInfo("LMACO0", &["hardip"]),
    TileInfo("RMACO1", &["hardip"]),
    TileInfo("RMACO2", &["hardip"]),
    TileInfo("RMACO3", &["hardip"]),
    TileInfo("RMACO4", &["hardip"]),
    TileInfo("RMACO0", &["hardip"]),
    TileInfo("BBMS2", &["hardip"]),
    TileInfo("BBMS3", &["hardip"]),
    TileInfo("BBMS6", &["hardip"]),
    TileInfo("BBMS7", &["hardip"]),
    TileInfo("GENERIC", &["int"]),
    TileInfo("GENERIC_RNET", &["int"]),
    TileInfo("BLANKEMB", &["int"]),
    TileInfo("PICL0", &["io"]),
    TileInfo("PICL0B", &["io-spec"]),
    TileInfo("PICL1", &["io"]),
    TileInfo("PICL1B", &["io-spec"]),
    TileInfo("PICL2", &["io"]),
    TileInfo("PICL3", &["io"]),
    TileInfo("PICR0", &["io"]),
    TileInfo("PICR0B", &["io-spec"]),
    TileInfo("PICR1", &["io"]),
    TileInfo("PICR1B", &["io-spec"]),
    TileInfo("PICR2", &["io"]),
    TileInfo("PICR3", &["io"]),
    TileInfo("PIC4B0", &["io"]),
    TileInfo("PIC4B1", &["io"]),
    TileInfo("PIC4B1B", &["io-spec"]),
    TileInfo("PIC4B2", &["io"]),
    TileInfo("PIC4B3", &["io"]),
    TileInfo("PIC5B0", &["io"]),
    TileInfo("PIC5B1", &["io"]),
    TileInfo("PIC5B1B", &["io-spec"]),
    TileInfo("PIC5B2", &["io"]),
    TileInfo("PIC5B3", &["io"]),
    TileInfo("PICT0", &["io"]),
    TileInfo("PICT0B", &["io-spec"]),
    TileInfo("PICT1", &["io"]),
    TileInfo("PICT1B", &["io-spec"]),
    TileInfo("PICT2", &["io"]),
    TileInfo("PICT2B", &["io-spec"]),
    TileInfo("PICT3", &["io"]),
    TileInfo("LPCS0", &["gtp"]),
    TileInfo("LPCS1", &["gtp"]),
    TileInfo("LPCS2", &["gtp"]),
    TileInfo("LPCS3", &["gtp"]),
    TileInfo("LPCS4", &["gtp"]),
    TileInfo("LPCS5", &["gtp"]),
    TileInfo("LPCS6", &["gtp"]),
    TileInfo("RPCS0", &["gtp"]),
    TileInfo("RPCS1", &["gtp"]),
    TileInfo("RPCS2", &["gtp"]),
    TileInfo("RPCS3", &["gtp"]),
    TileInfo("RPCS4", &["gtp"]),
    TileInfo("RPCS5", &["gtp"]),
    TileInfo("RPCS6", &["gtp"]),
    TileInfo("SYS0", &["cfg"]),
    TileInfo("SYS1", &["cfg"]),
    TileInfo("SYS2", &["cfg"]),
    TileInfo("SYS3", &["cfg"]),
    TileInfo("SYS4", &["cfg"]),
    TileInfo("SYS5", &["cfg"]),
    TileInfo("SYS6", &["cfg"]),
    TileInfo("SYS7", &["cfg"]),
    TileInfo("SYS8", &["cfg"]),
    TileInfo("SYS9", &["cfg"]),
    TileInfo("SYS10", &["cfg"]),
    TileInfo("SYS11", &["cfg"]),
    TileInfo("LLC", &["pll"]),
    TileInfo("MIBLLC", &["pll"]),
    TileInfo("LRC", &["pll"]),
    TileInfo("MIBLRC", &["pll"]),
    TileInfo("ULC", &["pll", "cfg"]),
    TileInfo("URC", &["pll", "cfg"]),
    TileInfo("PCLK", &["clk-row"]),
    TileInfo("EMBPCLK", &["clk-row"]),
    TileInfo("PCLKB0", &["clk-row-buf"]),
    TileInfo("PCLKB1", &["clk-row-buf"]),
    TileInfo("VIQ", &["clk-spine"]),
    TileInfo("EMBVIQ", &["clk-spine"]),
    TileInfo("HALFMUXB", &["clk-spine-buf"]),
    TileInfo("HALFMUXT", &["clk-spine-buf"]),
    TileInfo("VIQEB", &["clk-spine-buf"]),
    TileInfo("VIQET", &["clk-spine-buf"]),
    TileInfo("MIBBVIQ", &["clk-spine"]),
    TileInfo("MIBTVIQ", &["clk-spine"]),
    TileInfo("HIQ", &["clk-spine"]),
    TileInfo("HIQPCLK", &["clk-spine"]),
    TileInfo("MIBTPCLK", &["clk-spine"]),
    TileInfo("MIBBPCLK", &["clk-spine"]),
    TileInfo("MIBBPCLK4", &["clk-spine-buf"]),
    TileInfo("MIBBPCLK5", &["clk-spine-buf"]),
    TileInfo("CENTER", &["clk-spine"]),
    TileInfo("HIQMIBL", &["clk-spine-buf"]),
    TileInfo("HIQEL1", &["clk-spine-buf"]),
    TileInfo("HIQEL2", &["clk-spine-buf"]),
    TileInfo("HIQMIBR", &["clk-spine-buf"]),
    TileInfo("HIQER1", &["clk-spine-buf"]),
    TileInfo("HIQER2", &["clk-spine-buf"]),
    TileInfo("BLANK", &[]),
    TileInfo("BLANKMIBLR", &[]),
    TileInfo("BLANKMIBT", &[]),
    TileInfo("BLANKMIBTMIB", &[]),
    TileInfo("BLANKMIBB", &[]),
    TileInfo("BLANKPCLK", &[]),
];

const XP2_TILES: &[TileInfo] = &[
    TileInfo("FPLC", &["clb"]),
    TileInfo("FPLC5KVIQ", &["clb", "clk-spine"]),
    TileInfo("PLC", &["clbm"]),
    TileInfo("PLC5KVIQ", &["clbm", "clk-spine"]),
    TileInfo("EMB0", &["bram"]),
    TileInfo("EMB1", &["bram"]),
    TileInfo("EMB2", &["bram"]),
    TileInfo("EMB2_5KVIQ", &["bram", "clk-spine"]),
    TileInfo("EMB2SPL", &["bram", "clk-spine"]),
    TileInfo("EMB2SPR", &["bram", "clk-spine"]),
    TileInfo("DSP_0", &["dsp"]),
    TileInfo("DSP_1", &["dsp"]),
    TileInfo("DSP_2", &["dsp"]),
    TileInfo("DSP_3", &["dsp"]),
    TileInfo("DSP_4", &["dsp"]),
    TileInfo("DSP_5", &["dsp"]),
    TileInfo("DSP_6", &["dsp"]),
    TileInfo("DSP_7", &["dsp"]),
    TileInfo("DSP_8", &["dsp"]),
    TileInfo("DSP_8_5KVIQ", &["dsp", "clk-spine"]),
    TileInfo("DSP_8SPL", &["dsp", "clk-spine"]),
    TileInfo("DSP_8SPR", &["dsp", "clk-spine"]),
    TileInfo("PIC_L", &["io"]),
    TileInfo("PIC_L_NOPIO", &["io", "crippled"]),
    TileInfo("PIC_LDQS", &["io-spec"]),
    TileInfo("PIC_LDQSM2", &["io-spec"]),
    TileInfo("PIC_LDQSM3", &["io-spec"]),
    TileInfo("PIC_R", &["io"]),
    TileInfo("PIC_R_NOPIO", &["io", "crippled"]),
    TileInfo("PIC_RDQS", &["io-spec"]),
    TileInfo("PIC_RDQSM2", &["io-spec"]),
    TileInfo("PIC_RDQSM3", &["io-spec"]),
    TileInfo("PIC_B", &["io"]),
    TileInfo("PIC_BSPL", &["io", "clk-spine"]),
    TileInfo("PIC_BSPR", &["io", "clk-spine"]),
    TileInfo("PIC_B5KVIQ", &["io", "clk-spine"]),
    TileInfo("PIC_BDQS", &["io-spec"]),
    TileInfo("PIC_BLPCLK", &["io-spec"]),
    TileInfo("PIC_BRPCLK", &["io-spec"]),
    TileInfo("PIC_B_NOPIO", &["io", "crippled"]),
    TileInfo("PIC_T", &["io"]),
    TileInfo("PIC_TSPL", &["io", "clk-spine"]),
    TileInfo("PIC_TSPR", &["io", "clk-spine"]),
    TileInfo("PIC_T5KVIQ", &["io", "clk-spine"]),
    TileInfo("PIC_TDQS", &["io-spec"]),
    TileInfo("PIC_TLPCLK", &["io-spec"]),
    TileInfo("PIC_TRPCLK", &["io-spec"]),
    TileInfo("PIC_T_NOPIO", &["io", "crippled"]),
    TileInfo("CIB_LL_COR", &["pll"]),
    TileInfo("CIB_LR_COR", &["pll"]),
    TileInfo("CIB_UL_COR", &["pll"]),
    TileInfo("CIB_UR_COR", &["pll"]),
    TileInfo("CIB_LLC_NOPLL", &["int"]),
    TileInfo("CIB_URC_NOPLL", &["int"]),
    TileInfo("LLM0", &["int"]),
    TileInfo("RLM0", &["int"]),
    TileInfo("LUM0", &["int"]),
    TileInfo("RUM0", &["int"]),
    TileInfo("HIQ_END0", &["pll-alt"]),
    TileInfo("HIQ_END1", &["pll-alt", "cfg"]),
    TileInfo("VIQ", &["clk-spine"]),
    TileInfo("VIQ_EMB", &["clk-spine"]),
    TileInfo("VIQ_DSP", &["clk-spine"]),
    TileInfo("VIQ_PICB", &["clk-spine"]),
    TileInfo("VIQ_PICT", &["clk-spine"]),
    TileInfo("VIQ_BOT0", &["clk-spine-buf"]),
    TileInfo("VIQ_BOT1", &["clk-spine-buf"]),
    TileInfo("VIQ_BOT2", &["clk-spine-buf"]),
    TileInfo("VIQ_BOT3", &["clk-spine-buf"]),
    TileInfo("VIQ_BOT4", &["clk-spine-buf"]),
    TileInfo("VIQ_BOT5", &["clk-spine-buf"]),
    TileInfo("VIQ_BOT6", &["clk-spine-buf"]),
    TileInfo("VIQ_BOT7", &["clk-spine-buf"]),
    TileInfo("VIQ_TOP0", &["clk-spine-buf"]),
    TileInfo("VIQ_TOP1", &["clk-spine-buf"]),
    TileInfo("VIQ_TOP2", &["clk-spine-buf"]),
    TileInfo("VIQ_TOP3", &["clk-spine-buf"]),
    TileInfo("VIQ_TOP4", &["clk-spine-buf"]),
    TileInfo("VIQ_TOP5", &["clk-spine-buf"]),
    TileInfo("VIQ_TOP6", &["clk-spine-buf"]),
    TileInfo("VIQ_TOP7", &["clk-spine-buf"]),
];

const ECP2_TILES: &[TileInfo] = &[
    TileInfo("FPLC", &["clb"]),
    TileInfo("PLC", &["clbm"]),
    TileInfo("EMB0", &["bram"]),
    TileInfo("EMB1", &["bram"]),
    TileInfo("EMB2", &["bram"]),
    TileInfo("EMB2SPL", &["bram", "clk-spine"]),
    TileInfo("EMB2SPR", &["bram", "clk-spine"]),
    TileInfo("DSP_0", &["dsp"]),
    TileInfo("DSP_1", &["dsp"]),
    TileInfo("DSP_2", &["dsp"]),
    TileInfo("DSP_3", &["dsp"]),
    TileInfo("DSP_4", &["dsp"]),
    TileInfo("DSP_5", &["dsp"]),
    TileInfo("DSP_6", &["dsp"]),
    TileInfo("DSP_7", &["dsp"]),
    TileInfo("DSP_8", &["dsp"]),
    TileInfo("DSP_8SPL", &["dsp", "clk-spine"]),
    TileInfo("DSP_8SPR", &["dsp", "clk-spine"]),
    TileInfo("CFG_1", &["cfg"]),
    TileInfo("CFG_2", &["cfg"]),
    TileInfo("PIC_L", &["io"]),
    TileInfo("PIC_LLPCLK", &["io-spec"]),
    TileInfo("PIC_LUPCLK", &["io-spec"]),
    TileInfo("PIC_LDQS", &["io-spec"]),
    TileInfo("PIC_LDQSM2", &["io-spec"]),
    TileInfo("PIC_LDQSM3", &["io-spec"]),
    TileInfo("PIC_R", &["io"]),
    TileInfo("PIC_RLPCLK", &["io-spec"]),
    TileInfo("PIC_RUPCLK", &["io-spec"]),
    TileInfo("PIC_RDQS", &["io-spec"]),
    TileInfo("PIC_RDQSM2", &["io-spec"]),
    TileInfo("PIC_RDQSM3", &["io-spec"]),
    TileInfo("PIC_RCPU", &["io-spec"]),
    TileInfo("PIC_B", &["io"]),
    TileInfo("PIC_BSPL", &["io", "clk-spine"]),
    TileInfo("PIC_BSPR", &["io", "clk-spine"]),
    TileInfo("PIC_BDQS", &["io-spec"]),
    TileInfo("PIC_BLPCLK", &["io-spec"]),
    TileInfo("PIC_BRPCLK", &["io-spec"]),
    TileInfo("PIC_T", &["io"]),
    TileInfo("PIC_TSPL", &["io", "clk-spine"]),
    TileInfo("PIC_TSPR", &["io", "clk-spine"]),
    TileInfo("PIC_TLPCLK", &["io-spec"]),
    TileInfo("PIC_TRPCLK", &["io-spec"]),
    TileInfo("SERDES0", &["gtp"]),
    TileInfo("SERDES1", &["gtp"]),
    TileInfo("SERDES2", &["gtp"]),
    TileInfo("SERDES3", &["gtp"]),
    TileInfo("SERDES4", &["gtp"]),
    TileInfo("SERDES5", &["gtp"]),
    TileInfo("SERDES6", &["gtp"]),
    TileInfo("SERDES7", &["gtp"]),
    TileInfo("SERDES8SPR", &["gtp", "clk-spine"]),
    TileInfo("SERDES9", &["gtp"]),
    TileInfo("SERDES10", &["gtp"]),
    TileInfo("SERDES11", &["gtp"]),
    TileInfo("SERDES12", &["gtp"]),
    TileInfo("SERDES13", &["gtp"]),
    TileInfo("SERDES14", &["gtp"]),
    TileInfo("SERDES15", &["gtp"]),
    TileInfo("SERDES16", &["gtp"]),
    TileInfo("SERDES17", &["gtp"]),
    TileInfo("SERDES18", &["gtp"]),
    TileInfo("SERDES19", &["gtp"]),
    TileInfo("SERDES20", &["gtp"]),
    TileInfo("SERDES21", &["gtp"]),
    TileInfo("SERDES22", &["gtp"]),
    TileInfo("SERDES23", &["gtp"]),
    TileInfo("SERDES24", &["gtp"]),
    TileInfo("SERDES25", &["gtp"]),
    TileInfo("SERDES26", &["gtp"]),
    TileInfo("SERDES0B", &["gtp"]),
    TileInfo("SERDES1B", &["gtp"]),
    TileInfo("SERDES2B", &["gtp"]),
    TileInfo("SERDES3B", &["gtp"]),
    TileInfo("SERDES4B", &["gtp"]),
    TileInfo("SERDES5B", &["gtp"]),
    TileInfo("SERDES6B", &["gtp"]),
    TileInfo("SERDES7B", &["gtp"]),
    TileInfo("SERDES8BSPR", &["gtp", "clk-spine"]),
    TileInfo("SERDES9B", &["gtp"]),
    TileInfo("SERDES10B", &["gtp"]),
    TileInfo("SERDES11B", &["gtp"]),
    TileInfo("SERDES12B", &["gtp"]),
    TileInfo("SERDES13B", &["gtp"]),
    TileInfo("SERDES14B", &["gtp"]),
    TileInfo("SERDES15B", &["gtp"]),
    TileInfo("SERDES16B", &["gtp"]),
    TileInfo("SERDES17B", &["gtp"]),
    TileInfo("SERDES18B", &["gtp"]),
    TileInfo("SERDES19B", &["gtp"]),
    TileInfo("SERDES20B", &["gtp"]),
    TileInfo("SERDES21B", &["gtp"]),
    TileInfo("SERDES22B", &["gtp"]),
    TileInfo("SERDES23B", &["gtp"]),
    TileInfo("SERDES24B", &["gtp"]),
    TileInfo("SERDES25B", &["gtp"]),
    TileInfo("SERDES26B", &["gtp"]),
    TileInfo("SERDES0L", &["gtp"]),
    TileInfo("SERDES1L", &["gtp"]),
    TileInfo("SERDES2L", &["gtp"]),
    TileInfo("SERDES3L", &["gtp"]),
    TileInfo("SERDES4L", &["gtp"]),
    TileInfo("SERDES5L", &["gtp"]),
    TileInfo("SERDES6L", &["gtp"]),
    TileInfo("SERDES7L", &["gtp"]),
    TileInfo("SERDES8L", &["gtp"]),
    TileInfo("SERDES9L", &["gtp"]),
    TileInfo("SERDES10L", &["gtp"]),
    TileInfo("SERDES11L", &["gtp"]),
    TileInfo("SERDES12L", &["gtp"]),
    TileInfo("SERDES13L", &["gtp"]),
    TileInfo("SERDES14L", &["gtp"]),
    TileInfo("SERDES15L", &["gtp"]),
    TileInfo("SERDES16L", &["gtp"]),
    TileInfo("SERDES17LSPL", &["gtp", "clk-spine"]),
    TileInfo("SERDES18L", &["gtp"]),
    TileInfo("SERDES19L", &["gtp"]),
    TileInfo("SERDES20L", &["gtp"]),
    TileInfo("SERDES21L", &["gtp"]),
    TileInfo("SERDES22L", &["gtp"]),
    TileInfo("SERDES23L", &["gtp"]),
    TileInfo("SERDES24L", &["gtp"]),
    TileInfo("SERDES25L", &["gtp"]),
    TileInfo("SERDES26L", &["gtp"]),
    TileInfo("SERDES0BL", &["gtp"]),
    TileInfo("SERDES1BL", &["gtp"]),
    TileInfo("SERDES2BL", &["gtp"]),
    TileInfo("SERDES3BL", &["gtp"]),
    TileInfo("SERDES4BL", &["gtp"]),
    TileInfo("SERDES5BL", &["gtp"]),
    TileInfo("SERDES6BL", &["gtp"]),
    TileInfo("SERDES7BL", &["gtp"]),
    TileInfo("SERDES8BL", &["gtp"]),
    TileInfo("SERDES9BL", &["gtp"]),
    TileInfo("SERDES10BL", &["gtp"]),
    TileInfo("SERDES11BL", &["gtp"]),
    TileInfo("SERDES12BL", &["gtp"]),
    TileInfo("SERDES13BL", &["gtp"]),
    TileInfo("SERDES14BL", &["gtp"]),
    TileInfo("SERDES15BL", &["gtp"]),
    TileInfo("SERDES16BL", &["gtp"]),
    TileInfo("SERDES17BLSPL", &["gtp", "clk-spine"]),
    TileInfo("SERDES18BL", &["gtp"]),
    TileInfo("SERDES19BL", &["gtp"]),
    TileInfo("SERDES20BL", &["gtp"]),
    TileInfo("SERDES21BL", &["gtp"]),
    TileInfo("SERDES22BL", &["gtp"]),
    TileInfo("SERDES23BL", &["gtp"]),
    TileInfo("SERDES24BL", &["gtp"]),
    TileInfo("SERDES25BL", &["gtp"]),
    TileInfo("SERDES26BL", &["gtp"]),
    TileInfo("PIC_L_DUMMY", &["int"]),
    TileInfo("PIC_R_DUMMY", &["int"]),
    TileInfo("PIC_B_DUMMY", &["int"]),
    TileInfo("PIC_T_DUMMY", &["int"]),
    TileInfo("EMB_DUMMY", &["int"]),
    TileInfo("CIB_LL_COR", &["int"]),
    TileInfo("CIB_LR_COR", &["int"]),
    TileInfo("CIB_UL_COR", &["int"]),
    TileInfo("CIB_UR_COR", &["int"]),
    TileInfo("CIB_LLS_COR", &["int"]),
    TileInfo("CIB_LRS_COR", &["int"]),
    TileInfo("CIB_ULS_COR", &["int"]),
    TileInfo("CIB_URS_COR", &["int"]),
    TileInfo("HIQ_END0", &["int"]),
    TileInfo("HIQ_END1", &["int"]),
    TileInfo("EL_EMB_DUMMY", &["int"]),
    TileInfo("ER_EMB_DUMMY", &["int"]),
    TileInfo("PLL_LA", &["pll"]),
    TileInfo("PLL_LB", &["pll"]),
    TileInfo("PLL_LC", &["pll"]),
    TileInfo("LLM0", &["pll"]),
    TileInfo("PLL_RA", &["pll"]),
    TileInfo("PLL_RB", &["pll"]),
    TileInfo("PLL_RC", &["pll"]),
    TileInfo("RLM0", &["pll"]),
    TileInfo("LUM0", &["pll-alt"]),
    TileInfo("RUM0", &["pll-alt"]),
    TileInfo("VIQ", &["clk-spine"]),
    TileInfo("VIQ_PICB", &["clk-spine"]),
    TileInfo("VIQ_PICBB", &["clk-spine"]),
    TileInfo("VIQ_PICBC", &["clk-spine"]),
    TileInfo("VIQ_PICT", &["clk-spine"]),
    TileInfo("VIQ_PICTB", &["clk-spine"]),
    TileInfo("VIQ_PICT2", &["clk-spine"]),
    TileInfo("VIQ_EMB", &["clk-spine"]),
    TileInfo("VIQ_EMBPLUS", &["clk-spine"]),
    TileInfo("VIQ_EMBPLUSA", &["clk-spine"]),
    TileInfo("VIQ_EMB2", &["clk-spine"]),
    TileInfo("VIQ_EMBPLUS3", &["clk-spine"]),
    TileInfo("VIQ_DSP", &["clk-spine"]),
    TileInfo("VIQ_DSPPLUS", &["clk-spine"]),
    TileInfo("VIQ_DSPPLUS2", &["clk-spine"]),
    TileInfo("VIQ_EMBPLUS2", &["clk-spine"]),
    TileInfo("VIQ_EMBPLUS1", &["clk-spine"]),
    TileInfo("VIQ_EMB1", &["clk-spine-buf"]),
    TileInfo("CLKA1", &["clk-spine-buf"]),
    TileInfo("CLKB1", &["clk-spine-buf"]),
    TileInfo("CLKC1", &["clk-spine-buf"]),
    TileInfo("CLKD1", &["clk-spine-buf"]),
    TileInfo("CLKE1", &["clk-spine-buf"]),
    TileInfo("CLKF1", &["clk-spine-buf"]),
    TileInfo("CLKG1", &["clk-spine-buf"]),
    TileInfo("VIQ_DSP1", &["clk-spine-buf"]),
    TileInfo("VIQ_DSP2", &["clk-spine-buf"]),
    TileInfo("CLKI1", &["clk-spine-buf"]),
    TileInfo("CLKJ1", &["clk-spine-buf"]),
    TileInfo("CLKK1", &["clk-spine-buf"]),
    TileInfo("CLKL1", &["clk-spine-buf"]),
    TileInfo("CLKM1", &["clk-spine-buf"]),
    TileInfo("CLKN1", &["clk-spine-buf"]),
    TileInfo("CLKO1", &["clk-spine-buf"]),
    TileInfo("CLKP1", &["clk-spine-buf"]),
    TileInfo("CLKA3", &["clk-spine-buf"]),
    TileInfo("CLKB3", &["clk-spine-buf"]),
    TileInfo("CLKC3", &["clk-spine-buf"]),
    TileInfo("CLKD3", &["clk-spine-buf"]),
    TileInfo("CLKE3", &["clk-spine-buf"]),
    TileInfo("CLKF3", &["clk-spine-buf"]),
    TileInfo("CLK_LCTR3", &["clk-spine-buf"]),
    TileInfo("CLK_UCTR3", &["clk-spine-buf"]),
    TileInfo("CLKI3", &["clk-spine-buf"]),
    TileInfo("CLKJ3", &["clk-spine-buf"]),
    TileInfo("CLKK3", &["clk-spine-buf"]),
    TileInfo("CLKL3", &["clk-spine-buf"]),
    TileInfo("CLKM3", &["clk-spine-buf"]),
    TileInfo("CLKN3", &["clk-spine-buf"]),
    TileInfo("CLKO3", &["clk-spine-buf"]),
    TileInfo("CLKP3", &["clk-spine-buf"]),
    TileInfo("CLKA2", &["clk-spine-buf"]),
    TileInfo("CLKB2", &["clk-spine-buf"]),
    TileInfo("CLKC2", &["clk-spine-buf"]),
    TileInfo("CLKD2", &["clk-spine-buf"]),
    TileInfo("CLKE2", &["clk-spine-buf"]),
    TileInfo("CLKF2", &["clk-spine-buf"]),
    TileInfo("CLK_LCTR2", &["clk-spine-buf"]),
    TileInfo("CLK_UCTR2", &["clk-spine-buf"]),
    TileInfo("CLKI2", &["clk-spine-buf"]),
    TileInfo("CLKJ2", &["clk-spine-buf"]),
    TileInfo("CLKK2", &["clk-spine-buf"]),
    TileInfo("CLKL2", &["clk-spine-buf"]),
    TileInfo("CLKM2", &["clk-spine-buf"]),
    TileInfo("CLKN2", &["clk-spine-buf"]),
    TileInfo("CLKO2", &["clk-spine-buf"]),
    TileInfo("CLKP2", &["clk-spine-buf"]),
    TileInfo("CLKA", &["clk-spine-buf"]),
    TileInfo("CLKB", &["clk-spine-buf"]),
    TileInfo("CLKC", &["clk-spine-buf"]),
    TileInfo("CLKD", &["clk-spine-buf"]),
    TileInfo("CLKE", &["clk-spine-buf"]),
    TileInfo("CLKF", &["clk-spine-buf"]),
    TileInfo("CLKG", &["clk-spine-buf"]),
    TileInfo("CLKH", &["clk-spine-buf"]),
    TileInfo("CLK_LCTR", &["clk-spine-buf"]),
    TileInfo("CLK_UCTR", &["clk-spine-buf"]),
    TileInfo("CLKI", &["clk-spine-buf"]),
    TileInfo("CLKJ", &["clk-spine-buf"]),
    TileInfo("CLKK", &["clk-spine-buf"]),
    TileInfo("CLKL", &["clk-spine-buf"]),
    TileInfo("CLKM", &["clk-spine-buf"]),
    TileInfo("CLKN", &["clk-spine-buf"]),
    TileInfo("CLKO", &["clk-spine-buf"]),
    TileInfo("CLKP", &["clk-spine-buf"]),
    TileInfo("VIQ_EMB0", &["clk-spine"]),
    TileInfo("VIQ_EMB3", &["clk-spine"]),
    TileInfo("VIQ_EMB4", &["clk-spine"]),
    TileInfo("VIQ_EMBA0", &["clk-spine"]),
    TileInfo("VIQ_EMBA0", &["clk-spine"]),
    TileInfo("VIQ_EMBA1", &["clk-spine"]),
    TileInfo("VIQ_EMBA2", &["clk-spine"]),
    TileInfo("VIQ_EMBA3", &["clk-spine"]),
    TileInfo("VIQ_EMBB0", &["clk-spine"]),
    TileInfo("VIQ_EMBB1", &["clk-spine"]),
    TileInfo("VIQ_EMBB2", &["clk-spine"]),
    TileInfo("VIQ_EMBB3", &["clk-spine-buf"]),
    TileInfo("VIQ_EMBB4", &["clk-spine"]),
    TileInfo("VIQ_EMBB5", &["clk-spine"]),
    TileInfo("VIQ_EMBB6", &["clk-spine"]),
    TileInfo("VIQ_EMBC0", &["clk-spine"]),
    TileInfo("VIQ_EMBC1", &["clk-spine"]),
    TileInfo("VIQ_EMBC2", &["clk-spine"]),
    TileInfo("VIQ_EMBC3", &["clk-spine-buf"]),
    TileInfo("VIQ_EMBC4", &["clk-spine"]),
    TileInfo("VIQ_EMBC5", &["clk-spine"]),
    TileInfo("VIQ_EMBC6", &["clk-spine"]),
    TileInfo("VIQ_EMBD0", &["clk-spine"]),
    TileInfo("VIQ_EMBD1", &["clk-spine"]),
    TileInfo("VIQ_EMBD2", &["clk-spine"]),
    TileInfo("VIQ_EMBD3", &["clk-spine-buf"]),
    TileInfo("VIQ_EMBD4", &["clk-spine"]),
    TileInfo("VIQ_EMBD5", &["clk-spine"]),
    TileInfo("VIQ_EMBD6", &["clk-spine"]),
    TileInfo("DUMMY_PLC", &[]),
    TileInfo("DUMMY_DUMMY_T", &[]),
    TileInfo("DUMMY_DUMMY_R", &[]),
];

const ECP3_TILES: &[TileInfo] = &[
    TileInfo("FPLC", &["clb"]),
    TileInfo("PLC", &["clbm"]),
    TileInfo("EMB0", &["bram"]),
    TileInfo("EMB1", &["bram"]),
    TileInfo("EMB2", &["bram"]),
    TileInfo("EMB0CR", &["bram", "clk-spine"]),
    TileInfo("EMB0CR_1", &["bram", "clk-spine"]),
    TileInfo("EMB0CR_2", &["bram", "clk-spine"]),
    TileInfo("EMB0SPR", &["bram", "clk-spine"]),
    TileInfo("EMB0VIQSPR", &["bram", "clk-spine"]),
    TileInfo("EMB0VIQSPR_PT", &["bram", "clk-spine"]),
    TileInfo("EMB2VIQSPL", &["bram", "clk-spine"]),
    TileInfo("EMB2SPL", &["bram", "clk-spine"]),
    TileInfo("EMB2CL", &["bram", "clk-spine"]),
    TileInfo("EMB2CL_1", &["bram", "clk-spine"]),
    TileInfo("EMB2CL_2", &["bram", "clk-spine"]),
    TileInfo("EMB0PT", &["bram"]),
    TileInfo("EMB0SPR_PT", &["bram", "clk-spine"]),
    TileInfo("EMB0PT_LATVIQR", &["bram", "clk-spine"]),
    TileInfo("EMB0PT_R", &["bram", "clk-spine"]),
    TileInfo("EMB1PT", &["bram"]),
    TileInfo("EMB1PT_L", &["bram", "clk-spine"]),
    TileInfo("EMB1PT_L_17K", &["bram", "clk-spine"]),
    TileInfo("EMB1SPR", &["bram", "clk-spine"]),
    TileInfo("EMB1SPL", &["bram", "clk-spine"]),
    TileInfo("EMB2PT", &["bram"]),
    TileInfo("EMB2PT_L", &["bram", "clk-spine"]),
    TileInfo("EMB2PT_L_17K", &["bram", "clk-spine"]),
    TileInfo("EMB2PT_RATVIQL", &["bram", "clk-spine"]),
    TileInfo("EMB2PT_LATVIQR", &["bram", "clk-spine"]),
    TileInfo("EMB2SPL_PT", &["bram", "clk-spine"]),
    TileInfo("EMB2VIQSPL_PT", &["bram", "clk-spine"]),
    TileInfo("CENTER0", &["clk-spine-buf"]),
    TileInfo("CENTER1", &["clk-spine-buf"]),
    TileInfo("CENTER2", &["clk-spine-buf"]),
    TileInfo("CENTER3", &["clk-spine-buf"]),
    TileInfo("CENTER4", &["clk-spine-buf"]),
    TileInfo("CENTER5", &["clk-spine-buf"]),
    TileInfo("CENTER0_1", &["clk-spine-buf"]),
    TileInfo("CENTER1_1", &["clk-spine-buf"]),
    TileInfo("CENTER2_1", &["clk-spine-buf"]),
    TileInfo("CENTER3_1", &["clk-spine-buf"]),
    TileInfo("CENTER4_1", &["clk-spine-buf"]),
    TileInfo("CENTER5_1", &["clk-spine-buf"]),
    TileInfo("CENTER0_2", &["clk-spine-buf"]),
    TileInfo("CENTER1_2", &["clk-spine-buf"]),
    TileInfo("CENTER2_2", &["clk-spine-buf"]),
    TileInfo("CENTER3_2", &["clk-spine-buf"]),
    TileInfo("CENTER4_2", &["clk-spine-buf"]),
    TileInfo("CENTER5_2", &["clk-spine-buf"]),
    TileInfo("DSP0", &["dsp"]),
    TileInfo("DSP1", &["dsp"]),
    TileInfo("DSP2", &["dsp"]),
    TileInfo("DSP3", &["dsp"]),
    TileInfo("DSP4", &["dsp"]),
    TileInfo("DSP5", &["dsp"]),
    TileInfo("DSP6", &["dsp"]),
    TileInfo("DSP7", &["dsp"]),
    TileInfo("DSP8", &["dsp"]),
    TileInfo("DSP0PT", &["dsp"]),
    TileInfo("DSP0SPR_PT", &["dsp", "clk-spine"]),
    TileInfo("DSP0VIQSPR_PT", &["dsp", "clk-spine"]),
    TileInfo("DSP1PT", &["dsp"]),
    TileInfo("DSP1PT_L", &["dsp", "clk-spine"]),
    TileInfo("DSP1PT_L_17K", &["dsp", "clk-spine"]),
    TileInfo("DSP1SPR_PT", &["dsp", "clk-spine"]),
    TileInfo("DSP2PT", &["dsp"]),
    TileInfo("DSP2PT_L", &["dsp", "clk-spine"]),
    TileInfo("DSP2PT_L_17K", &["dsp", "clk-spine"]),
    TileInfo("DSP2PT_LATVIQR", &["dsp", "clk-spine"]),
    TileInfo("DSP3PT", &["dsp"]),
    TileInfo("DSP3PT_LATVIQR", &["dsp", "clk-spine"]),
    TileInfo("DSP4PT", &["dsp"]),
    TileInfo("DSP5PT", &["dsp"]),
    TileInfo("DSP5PT_RATVIQL", &["dsp", "clk-spine"]),
    TileInfo("DSP6PT", &["dsp"]),
    TileInfo("DSP6PT_R", &["dsp", "clk-spine"]),
    TileInfo("DSP6PT_R_17K", &["dsp", "clk-spine"]),
    TileInfo("DSP7PT", &["dsp"]),
    TileInfo("DSP7SPL_PT", &["dsp", "clk-spine"]),
    TileInfo("DSP8PT", &["dsp"]),
    TileInfo("DSP8SPL_PT", &["dsp", "clk-spine"]),
    TileInfo("DSP8VIQSPL_PT", &["dsp", "clk-spine"]),
    TileInfo("CFG0", &["cfg"]),
    TileInfo("CFG1", &["cfg"]),
    TileInfo("CFG2", &["cfg"]),
    TileInfo("CFG3", &["cfg"]),
    TileInfo("CFG4", &["cfg"]),
    TileInfo("CFG5", &["cfg"]),
    TileInfo("CFG6", &["cfg"]),
    TileInfo("CFG7", &["cfg"]),
    TileInfo("CFG8", &["cfg"]),
    TileInfo("CFG9", &["cfg"]),
    TileInfo("CFG10", &["cfg"]),
    TileInfo("CFG11", &["cfg"]),
    TileInfo("CFG0PT", &["cfg"]),
    TileInfo("CFG1PT", &["cfg"]),
    TileInfo("CFG2PT", &["cfg"]),
    TileInfo("CFG3PT", &["cfg"]),
    TileInfo("CFG4PT", &["cfg"]),
    TileInfo("CFG5PT", &["cfg"]),
    TileInfo("CFG6PT", &["cfg"]),
    TileInfo("CFG7PT", &["cfg"]),
    TileInfo("CFG8PT", &["cfg"]),
    TileInfo("CFG9PT", &["cfg"]),
    TileInfo("CFG9PT_17K", &["cfg"]),
    TileInfo("CFG10PT", &["cfg"]),
    TileInfo("CFG11PT", &["cfg"]),
    TileInfo("GPLL_L1PT", &["pll"]),
    TileInfo("GPLL_L2PT", &["pll"]),
    TileInfo("GPLL_L3PT", &["pll"]),
    TileInfo("GPLL_L4PT", &["pll"]),
    TileInfo("GPLL_L5PT", &["pll"]),
    TileInfo("GPLL_L6PT", &["pll"]),
    TileInfo("GPLL_L7PT", &["pll"]),
    TileInfo("GPLL_L8PT", &["pll"]),
    TileInfo("GPLL_L9PT", &["pll"]),
    TileInfo("GPLL_L10PT", &["pll"]),
    TileInfo("GPLL_L11PT", &["pll"]),
    TileInfo("GPLL_L12PT", &["pll"]),
    TileInfo("GPLL_R1PT", &["pll"]),
    TileInfo("GPLL_R2PT", &["pll"]),
    TileInfo("GPLL_R3PT", &["pll"]),
    TileInfo("GPLL_R4PT", &["pll"]),
    TileInfo("GPLL_R5PT", &["pll"]),
    TileInfo("GPLL_R6PT", &["pll"]),
    TileInfo("GPLL_R7PT", &["pll"]),
    TileInfo("GPLL_R8PT", &["pll"]),
    TileInfo("GPLL_R9PT", &["pll"]),
    TileInfo("GPLL_R10PT", &["pll"]),
    TileInfo("GPLL_R11PT", &["pll"]),
    TileInfo("GPLL_R12PT", &["pll"]),
    TileInfo("GPLL_L1", &["pll"]),
    TileInfo("GPLL_L2", &["pll"]),
    TileInfo("GPLL_L2_HIQ", &["pll"]),
    TileInfo("GPLL_L3", &["pll"]),
    TileInfo("GPLL_L3_HIQ", &["pll"]),
    TileInfo("GPLL_L4", &["pll"]),
    TileInfo("GPLL_L5", &["pll"]),
    TileInfo("GPLL_L6", &["pll"]),
    TileInfo("GPLL_L7", &["pll"]),
    TileInfo("GPLL_L8", &["pll"]),
    TileInfo("GPLL_L9", &["pll"]),
    TileInfo("GPLL_L10", &["pll"]),
    TileInfo("GPLL_L11", &["pll"]),
    TileInfo("GPLL_L12", &["pll"]),
    TileInfo("GPLL_R1", &["pll"]),
    TileInfo("GPLL_R2", &["pll"]),
    TileInfo("GPLL_R2_HIQ", &["pll"]),
    TileInfo("GPLL_R3", &["pll"]),
    TileInfo("GPLL_R3_HIQ", &["pll"]),
    TileInfo("GPLL_R4", &["pll"]),
    TileInfo("GPLL_R5", &["pll"]),
    TileInfo("GPLL_R6", &["pll"]),
    TileInfo("GPLL_R7", &["pll"]),
    TileInfo("GPLL_R8", &["pll"]),
    TileInfo("GPLL_R9", &["pll"]),
    TileInfo("GPLL_R10", &["pll"]),
    TileInfo("GPLL_R11", &["pll"]),
    TileInfo("GPLL_R12", &["pll"]),
    TileInfo("GDLL_L0", &["pll-alt"]),
    TileInfo("GDLL_L1", &["pll-alt"]),
    TileInfo("GDLL_L2", &["pll-alt"]),
    TileInfo("GDLL_R0", &["pll-alt"]),
    TileInfo("GDLL_R1", &["pll-alt"]),
    TileInfo("GDLL_R2", &["pll-alt"]),
    TileInfo("PIC_L0", &["io"]),
    TileInfo("PIC_L1", &["io"]),
    TileInfo("PIC_L2", &["io"]),
    TileInfo("PIC_L0A", &["io"]),
    TileInfo("PIC_L1A", &["io"]),
    TileInfo("PIC_L2A", &["io"]),
    TileInfo("PIC_L0B", &["io"]),
    TileInfo("PIC_L1B", &["io"]),
    TileInfo("PIC_L2B", &["io"]),
    TileInfo("PIC_L0E", &["io"]),
    TileInfo("PIC_L1E", &["io"]),
    TileInfo("PIC_L2E", &["io"]),
    TileInfo("PIC_LDQS0A", &["io-spec"]),
    TileInfo("PIC_LDQS1A", &["io-spec"]),
    TileInfo("PIC_LDQS2A", &["io-spec"]),
    TileInfo("PIC_LDQS0AS", &["io-spec"]),
    TileInfo("PIC_LDQS1AS", &["io-spec"]),
    TileInfo("PIC_LDQS2AS", &["io-spec"]),
    TileInfo("PIC_LDQS0B", &["io-spec"]),
    TileInfo("PIC_LDQS1B", &["io-spec"]),
    TileInfo("PIC_LDQS2B", &["io-spec"]),
    TileInfo("PIC_LDQS0C", &["io-spec"]),
    TileInfo("PIC_LDQS1C", &["io-spec"]),
    TileInfo("PIC_LDQS2C", &["io-spec"]),
    TileInfo("PIC_LDQS0D", &["io-spec"]),
    TileInfo("PIC_LDQS1D", &["io-spec"]),
    TileInfo("PIC_LDQS2D", &["io-spec"]),
    TileInfo("PIC_LDQS0E", &["io-spec"]),
    TileInfo("PIC_LDQS1E", &["io-spec"]),
    TileInfo("PIC_LDQS2E", &["io-spec"]),
    TileInfo("PIC_LDQS0F", &["io-spec"]),
    TileInfo("PIC_LDQS1F", &["io-spec"]),
    TileInfo("PIC_LDQS2F", &["io-spec"]),
    TileInfo("PIC_L0DQBUF3E", &["io-spec"]),
    TileInfo("PICATEMBM1_L1DQBUF4E", &["io-spec"]),
    TileInfo("PICATEMB_L2EVREF", &["io-spec"]),
    TileInfo("PICATEMB_L2EPT", &["io-spec"]),
    TileInfo("PICATVREFL_L2EPT", &["io-spec"]),
    TileInfo("LLC2", &["io-spec"]),
    TileInfo("PICATPLLM2_L0DQBUF3E", &["io-spec"]),
    TileInfo("PICATPLLM1_L1DQBUF4E", &["io-spec"]),
    TileInfo("PICATPLL_L2E", &["io-spec"]),
    TileInfo("PIC_L0DQBUF3A", &["io-spec"]),
    TileInfo("PICATEMBM1_L1DQBUF4A", &["io-spec"]),
    TileInfo("PICATEMB_L2APT", &["io-spec"]),
    TileInfo("PICATEMB_L2A", &["io-spec"]),
    TileInfo("PICATVREFU_L2APT", &["io-spec"]),
    TileInfo("PICATVREFL_L2APT", &["io-spec"]),
    TileInfo("LLC0", &["io-spec"]),
    TileInfo("PICATPLLM2_L0DQBUF3A", &["io-spec"]),
    TileInfo("PICATPLLM1_L1DQBUF4A", &["io-spec"]),
    TileInfo("PICATPLL_L2A", &["io-spec"]),
    TileInfo("PICATPLL_L2APT", &["io-spec"]),
    TileInfo("PIC_L0DQBUF3B", &["io-spec"]),
    TileInfo("PICATEMBM1_L1DQBUF4B", &["io-spec"]),
    TileInfo("PICATEMB_L2BPT", &["io-spec"]),
    TileInfo("PICATEMB_L2B", &["io-spec"]),
    TileInfo("PICATVREFU_L2BPT", &["io-spec"]),
    TileInfo("PICATVREFL_L2BPT", &["io-spec"]),
    TileInfo("PICATDSP_L2B", &["io-spec"]),
    TileInfo("LLC1", &["io-spec"]),
    TileInfo("PICATPLLM2_L0DQBUF3B", &["io-spec"]),
    TileInfo("PICATPLLM1_L1DQBUF4B", &["io-spec"]),
    TileInfo("PICATPLL_L2B", &["io-spec"]),
    TileInfo("PICATPLL_L2BPT", &["io-spec"]),
    TileInfo("PIC_R0", &["io"]),
    TileInfo("PIC_R1", &["io"]),
    TileInfo("PIC_R2", &["io"]),
    TileInfo("PIC_RCPU0", &["io-spec"]),
    TileInfo("PIC_RCPU1", &["io-spec"]),
    TileInfo("PIC_RCPU2", &["io-spec"]),
    TileInfo("PIC_RCPU0C", &["io-spec"]),
    TileInfo("PIC_RCPU1C", &["io-spec"]),
    TileInfo("PIC_RCPU2C", &["io-spec"]),
    TileInfo("PICATEMBM1_RCPU1", &["io-spec"]),
    TileInfo("PICATEMB_RCPU2VREF", &["io-spec"]),
    TileInfo("PICATEMB_RCPU2PT", &["io-spec"]),
    TileInfo("PICATEMB_RCPU2", &["io-spec"]),
    TileInfo("PIC_RDQS0", &["io-spec"]),
    TileInfo("PIC_RDQS1", &["io-spec"]),
    TileInfo("PIC_RDQS2", &["io-spec"]),
    TileInfo("PIC_R3DQS0", &["io-spec"]),
    TileInfo("PIC_R3DQS1", &["io-spec"]),
    TileInfo("PIC_R3DQS2", &["io-spec"]),
    TileInfo("PIC_RDQS0C", &["io-spec"]),
    TileInfo("PIC_RDQS1C", &["io-spec"]),
    TileInfo("PIC_RDQS2C", &["io-spec"]),
    TileInfo("PIC_R0DQBUF3", &["io-spec"]),
    TileInfo("PICATEMBM1_R1DQBUF4", &["io-spec"]),
    TileInfo("PICATVREFL_R2PT", &["io-spec"]),
    TileInfo("PICATVREFU_R2PT", &["io-spec"]),
    TileInfo("PICATDSP_R2", &["io-spec"]),
    TileInfo("PICATEMB_R2", &["io-spec"]),
    TileInfo("LRC", &["io-spec"]),
    TileInfo("PICATPLLM2_R0DQBUF3", &["io-spec"]),
    TileInfo("PICATPLLM1_R1DQBUF4", &["io-spec"]),
    TileInfo("PICATPLL_R2", &["io-spec"]),
    TileInfo("PICATPLL_R2PT", &["io-spec"]),
    TileInfo("PIC_B0", &["io"]),
    TileInfo("PIC_B1", &["io"]),
    TileInfo("PIC_B2", &["io"]),
    TileInfo("PIC_T0", &["io"]),
    TileInfo("PIC_TSPR0", &["io", "clk-spine"]),
    TileInfo("PIC_TVIQSPR0", &["io", "clk-spine"]),
    TileInfo("PIC_T1", &["io"]),
    TileInfo("PIC_T2", &["io"]),
    TileInfo("PIC_TSPL2", &["io", "clk-spine"]),
    TileInfo("PIC_TVIQSPL2", &["io", "clk-spine"]),
    TileInfo("PIC_TCPU0", &["io-spec"]),
    TileInfo("PIC_TCPU1", &["io-spec"]),
    TileInfo("PIC_TCPU2", &["io-spec"]),
    TileInfo("PIC_TDQS0", &["io-spec"]),
    TileInfo("PIC_TDQS1", &["io-spec"]),
    TileInfo("PIC_TDQS2", &["io-spec"]),
    TileInfo("SERDES0", &["gtp"]),
    TileInfo("SERDES0SP", &["gtp", "clk-spine"]),
    TileInfo("SERDES0VIQ", &["gtp", "clk-spine"]),
    TileInfo("SERDES1", &["gtp"]),
    TileInfo("SERDES2", &["gtp"]),
    TileInfo("SERDES3", &["gtp"]),
    TileInfo("SERDES4", &["gtp"]),
    TileInfo("SERDES5", &["gtp"]),
    TileInfo("SERDES6", &["gtp"]),
    TileInfo("SERDES7", &["gtp"]),
    TileInfo("SERDES8", &["gtp"]),
    TileInfo("SERDES8SP", &["gtp", "clk-spine"]),
    TileInfo("SERDES9", &["gtp"]),
    TileInfo("SERDES9SP", &["gtp", "clk-spine"]),
    TileInfo("SERDES10", &["gtp"]),
    TileInfo("SERDES11", &["gtp"]),
    TileInfo("SERDES12", &["gtp"]),
    TileInfo("SERDES13", &["gtp"]),
    TileInfo("SERDES14", &["gtp"]),
    TileInfo("SERDES15", &["gtp"]),
    TileInfo("SERDES16", &["gtp"]),
    TileInfo("SERDES17SP", &["gtp", "clk-spine"]),
    TileInfo("SERDES18SP", &["gtp", "clk-spine"]),
    TileInfo("SERDES17VIQ", &["gtp", "clk-spine"]),
    TileInfo("SERDES18VIQ", &["gtp", "clk-spine"]),
    TileInfo("SERDES19", &["gtp"]),
    TileInfo("SERDES20", &["gtp"]),
    TileInfo("SERDES21", &["gtp"]),
    TileInfo("SERDES22", &["gtp"]),
    TileInfo("SERDES23", &["gtp"]),
    TileInfo("SERDES24", &["gtp"]),
    TileInfo("SERDES25", &["gtp"]),
    TileInfo("SERDES26", &["gtp"]),
    TileInfo("SERDES26SP", &["gtp", "clk-spine"]),
    TileInfo("SERDES27", &["gtp"]),
    TileInfo("SERDES27SP", &["gtp", "clk-spine"]),
    TileInfo("SERDES28", &["gtp"]),
    TileInfo("SERDES29", &["gtp"]),
    TileInfo("SERDES30", &["gtp"]),
    TileInfo("SERDES31", &["gtp"]),
    TileInfo("SERDES32", &["gtp"]),
    TileInfo("SERDES33", &["gtp"]),
    TileInfo("SERDES34", &["gtp"]),
    TileInfo("SERDES35", &["gtp"]),
    TileInfo("SERDES35SP", &["gtp", "clk-spine"]),
    TileInfo("SERDES35VIQ", &["gtp", "clk-spine"]),
    TileInfo("SERDESA0", &["gtp"]),
    TileInfo("SERDESA1", &["gtp"]),
    TileInfo("SERDESA2", &["gtp"]),
    TileInfo("SERDESA3", &["gtp"]),
    TileInfo("SERDESA4", &["gtp"]),
    TileInfo("SERDESA5", &["gtp"]),
    TileInfo("SERDESA6", &["gtp"]),
    TileInfo("SERDESA7", &["gtp"]),
    TileInfo("SERDESA8", &["gtp"]),
    TileInfo("SERDESA9", &["gtp"]),
    TileInfo("SERDESA10", &["gtp"]),
    TileInfo("SERDESA11", &["gtp"]),
    TileInfo("SERDESA12", &["gtp"]),
    TileInfo("SERDESA13", &["gtp"]),
    TileInfo("SERDESA14", &["gtp"]),
    TileInfo("SERDESA15", &["gtp"]),
    TileInfo("SERDESA16", &["gtp"]),
    TileInfo("SERDESA17", &["gtp"]),
    TileInfo("SERDESA18", &["gtp"]),
    TileInfo("SERDESA19", &["gtp"]),
    TileInfo("SERDESA20", &["gtp"]),
    TileInfo("SERDESA21", &["gtp"]),
    TileInfo("SERDESA22", &["gtp"]),
    TileInfo("SERDESA23", &["gtp"]),
    TileInfo("SERDESA24", &["gtp"]),
    TileInfo("SERDESA25", &["gtp"]),
    TileInfo("SERDESA26", &["gtp"]),
    TileInfo("SERDESA27", &["gtp"]),
    TileInfo("SERDESA28", &["gtp"]),
    TileInfo("SERDESA29", &["gtp"]),
    TileInfo("SERDESA30", &["gtp"]),
    TileInfo("SERDESA31", &["gtp"]),
    TileInfo("SERDESA32", &["gtp"]),
    TileInfo("SERDESA33", &["gtp"]),
    TileInfo("SERDESA34", &["gtp"]),
    TileInfo("SERDESA35", &["gtp"]),
    TileInfo("DUMMY_ASR", &["int"]),
    TileInfo("DUMMY_ASRSPL", &["int", "clk-spine"]),
    TileInfo("DUMMY_ASRSPR", &["int", "clk-spine"]),
    TileInfo("PIC_T_DUMMY", &["int"]),
    TileInfo("PIC_R_DUMMY", &["int"]),
    TileInfo("ULC0", &["int"]),
    TileInfo("ULC1", &["int"]),
    TileInfo("ULC2", &["int"]),
    TileInfo("URC", &["int"]),
    TileInfo("PIC_L_DUMMY_17", &["int"]),
    TileInfo("PIC_L_DUMMY", &["int"]),
    TileInfo("PIC_T_DUMMY_VREF", &["int"]),
    TileInfo("TWOROW_EMPTY_PLC", &[]),
    TileInfo("TWOROW_EMPTY_PIC", &[]),
    TileInfo("DUMMY_SERDES", &[]),
    TileInfo("DUMMY_SERDES_B", &[]),
];

const ECP4_TILES: &[TileInfo] = &[
    TileInfo("PLC", &["clbm"]),
    TileInfo("CIB_DSP", &["int"]),
    TileInfo("CIB_DSP_B", &["int"]),
    TileInfo("CIB_EBR0", &["int"]),
    TileInfo("CIB_EBR1", &["int"]),
    TileInfo("CIB_EBR5", &["int"]),
    TileInfo("CIB_EBR6", &["int"]),
    TileInfo("CIB_EBR0_B", &["int"]),
    TileInfo("CIB_EBR1_B", &["int"]),
    TileInfo("CIB_EBR5_B", &["int"]),
    TileInfo("CIB_EBR6_B", &["int"]),
    TileInfo("CIB_L", &["int"]),
    TileInfo("CIB_T", &["int"]),
    TileInfo("CIB_B", &["int"]),
    TileInfo("MIB_EBR0", &["bram"]),
    TileInfo("MIB_EBR1", &["bram"]),
    TileInfo("MIB_EBR2", &["bram"]),
    TileInfo("MIB_EBR3", &["bram"]),
    TileInfo("MIB_EBR4", &["bram"]),
    TileInfo("MIB_EBR5", &["bram"]),
    TileInfo("MIB_EBR6", &["bram"]),
    TileInfo("MIB_EBR7", &["bram"]),
    TileInfo("MIB_EBR8", &["bram"]),
    TileInfo("MIB_EBR5_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR6_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR7_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR8_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR8_UL", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR0_UR", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR0_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR1_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR2_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR3_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR0B", &["bram"]),
    TileInfo("MIB_EBR1B", &["bram"]),
    TileInfo("MIB_EBR2B", &["bram"]),
    TileInfo("MIB_EBR3B", &["bram"]),
    TileInfo("MIB_EBR4B", &["bram"]),
    TileInfo("MIB_EBR5B", &["bram"]),
    TileInfo("MIB_EBR6B", &["bram"]),
    TileInfo("MIB_EBR7B", &["bram"]),
    TileInfo("MIB_EBR8B", &["bram"]),
    TileInfo("MIB_EBR5B_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR6B_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR7B_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR8B_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR8_LL", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR0_LR", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR0B_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR1B_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR2B_SP", &["bram", "clk-spine"]),
    TileInfo("MIB_EBR3B_SP", &["bram", "clk-spine"]),
    TileInfo("DSP0", &["dsp"]),
    TileInfo("DSP1", &["dsp"]),
    TileInfo("DSP2", &["dsp"]),
    TileInfo("DSP3", &["dsp"]),
    TileInfo("DSP4", &["dsp"]),
    TileInfo("DSP5", &["dsp"]),
    TileInfo("DSP6", &["dsp"]),
    TileInfo("DSP7", &["dsp"]),
    TileInfo("DSP8", &["dsp"]),
    TileInfo("DSP0_B", &["dsp"]),
    TileInfo("DSP1_B", &["dsp"]),
    TileInfo("DSP2_B", &["dsp"]),
    TileInfo("DSP3_B", &["dsp"]),
    TileInfo("DSP4_B", &["dsp"]),
    TileInfo("DSP5_B", &["dsp"]),
    TileInfo("DSP6_B", &["dsp"]),
    TileInfo("DSP7_B", &["dsp"]),
    TileInfo("DSP8_B", &["dsp"]),
    TileInfo("MIB_L_PIC0A", &["io"]),
    TileInfo("MIB_L_PIC0B", &["io"]),
    TileInfo("MIB_L_PIC0B_BREF", &["io", "iob-spec"]),
    TileInfo("MIB_L_PIC0B_HIQ_U", &["io", "clk-spine"]),
    TileInfo("MIB_L_PIC0A_DQS1", &["io-spec"]),
    TileInfo("MIB_L_PIC0B_DQS0", &["io-spec"]),
    TileInfo("MIB_L_PIC0A_DQS2", &["io-spec"]),
    TileInfo("MIB_L_PIC0B_DQS3", &["io-spec"]),
    TileInfo("MIB_L_PIC0A_DQS3", &["io-spec"]),
    TileInfo("MIB_LS_PIC0B_E", &["io"]),
    TileInfo("MIB_LS_PIC0B_E_DQS2", &["io-spec"]),
    TileInfo("MIB_EBR_ENDD_UL", &["io"]),
    TileInfo("MIB_EBR_ENDC_UL", &["io"]),
    TileInfo("MIB_EBR_ENDB_UL", &["io"]),
    TileInfo("MIB_EBR_ENDA_UL", &["io"]),
    TileInfo("MIB_LS_PIC0B_D", &["io"]),
    TileInfo("MIB_LS_PIC0B_D_DQS0", &["io-spec"]),
    TileInfo("DSP_ENDB_UL", &["io"]),
    TileInfo("DSP_ENDA_UL", &["io"]),
    TileInfo("MIB_LS_PIC0A_D", &["io"]),
    TileInfo("DSP_ENDB_LL", &["io"]),
    TileInfo("DSP_ENDA_LL", &["io"]),
    TileInfo("MIB_LS_PIC0A_E", &["io"]),
    TileInfo("MIB_LS_PIC0A_E_DQS1", &["io-spec"]),
    TileInfo("MIB_LS_PIC0A_E_DQS2", &["io-spec"]),
    TileInfo("MIB_EBR_ENDD_LL", &["io"]),
    TileInfo("MIB_EBR_ENDC_LL", &["io"]),
    TileInfo("MIB_EBR_ENDB_LL", &["io"]),
    TileInfo("MIB_EBR_ENDA_LL", &["io"]),
    TileInfo("MIB_R_PIC0A", &["io"]),
    TileInfo("MIB_R_PIC0B", &["io"]),
    TileInfo("MIB_R_PIC0B_BREF", &["io", "iob-spec"]),
    TileInfo("MIB_R_PIC0B_HIQ_U", &["io", "clk-spine"]),
    TileInfo("MIB_R_PIC0A_DQS1", &["io-spec"]),
    TileInfo("MIB_R_PIC0B_DQS0", &["io-spec"]),
    TileInfo("MIB_R_PIC0A_DQS2", &["io-spec"]),
    TileInfo("MIB_R_PIC0B_DQS3", &["io-spec"]),
    TileInfo("MIB_R_PIC0A_DQS3", &["io-spec"]),
    TileInfo("MIB_RS_PIC0B_E", &["io"]),
    TileInfo("MIB_RS_PIC0B_E_DQS2", &["io-spec"]),
    TileInfo("MIB_EBR_ENDD_UR", &["io"]),
    TileInfo("MIB_EBR_ENDC_UR", &["io"]),
    TileInfo("MIB_EBR_ENDB_UR", &["io"]),
    TileInfo("MIB_EBR_ENDA_UR", &["io"]),
    TileInfo("MIB_RS_PIC0B_D", &["io"]),
    TileInfo("MIB_RS_PIC0B_D_DQS0", &["io-spec"]),
    TileInfo("DSP_ENDB_UR", &["io"]),
    TileInfo("DSP_ENDA_UR", &["io"]),
    TileInfo("MIB_RS_PIC0A_D", &["io"]),
    TileInfo("DSP_ENDB_LR", &["io"]),
    TileInfo("DSP_ENDA_LR", &["io"]),
    TileInfo("MIB_RS_PIC0A_E", &["io"]),
    TileInfo("MIB_RS_PIC0A_E_DQS1", &["io-spec"]),
    TileInfo("MIB_RS_PIC0A_E_DQS2", &["io-spec"]),
    TileInfo("MIB_EBR_ENDA_LR", &["io"]),
    TileInfo("MIB_EBR_ENDB_LR", &["io"]),
    TileInfo("MIB_EBR_ENDC_LR", &["io"]),
    TileInfo("MIB_EBR_ENDD_LR", &["io"]),
    TileInfo("MIB_T_PIC0A", &["io"]),
    TileInfo("MIB_T_PIC0B", &["io"]),
    TileInfo("MIB_T_PIC0B_DLLDEL_B1", &["io", "pll-alt"]),
    TileInfo("MIB_T_PIC0A_DLLDEL_B0", &["io", "pll-alt"]),
    TileInfo("MIB_T_PIC0B_DLLDEL_B1_BIG", &["io", "pll-alt"]),
    TileInfo("MIB_T_PIC0A_DLLDEL_B1_BIG", &["io", "pll-alt"]),
    TileInfo("MIB_T_PIC0A_DQS1", &["io-spec"]),
    TileInfo("MIB_T_PIC0B_DQS0", &["io-spec"]),
    TileInfo("MIB_BL_DUMMY_DLL0", &["pll-alt"]),
    TileInfo("MIB_BL_DUMMY_DLL1", &["pll-alt"]),
    TileInfo("MIB_BR_DUMMY_DLL0", &["pll-alt"]),
    TileInfo("MIB_BR_DUMMY_DLL1", &["pll-alt"]),
    TileInfo("MIB_TL_DUMMY_DLL0", &["pll-alt"]),
    TileInfo("MIB_TL_DUMMY_DLL1", &["pll-alt"]),
    TileInfo("MIB_TR_DUMMY_DLL0", &["pll-alt"]),
    TileInfo("MIB_TR_DUMMY_DLL1", &["pll-alt"]),
    TileInfo("MIB_TR_DUMMY_DLL1_BIG", &["pll-alt"]),
    TileInfo("MIB_L_DUMMY_PLL", &["pll"]),
    TileInfo("MIB_R_DUMMY_PLL", &["pll"]),
    TileInfo("MIB_T_DUMMY_IOS", &["cfg"]),
    TileInfo("MIB_T_DUMMY_IOS_BIG", &["cfg"]),
    TileInfo("MIB_T_DUMMY_DTR", &["cfg"]),
    TileInfo("MIB_B_DUMMY_EFB0", &["cfg"]),
    TileInfo("MIB_B_DUMMY_EFB1", &["cfg"]),
    TileInfo("MIB_B_DUMMY_ASB0", &["gtp"]),
    TileInfo("MIB_B_DUMMY_ASB1", &["gtp"]),
    TileInfo("MIB_B_DUMMY_ASB2", &["gtp"]),
    TileInfo("MIB_B_DUMMY_DTR", &["cfg"]),
    TileInfo("MIB_TL_DUMMY_IPO", &["cfg"]),
    TileInfo("MIB_TR_DUMMY_IPO", &["cfg"]),
    TileInfo("DSP_DUMMY_GSR", &["cfg"]),
    TileInfo("MIB_T_DUMMY_BREF0", &["iob-spec"]),
    TileInfo("MIB_T_DUMMY_BREF1", &["iob-spec"]),
    TileInfo("MIB_T_DUMMY_BREF1_BIG", &["iob-spec"]),
    TileInfo("MIB_T_DUMMY_BREF2_BIG", &["iob-spec"]),
    TileInfo("MIB_R_DUMMY_BREF4", &["iob-spec"]),
    TileInfo("MIB_R_DUMMY_BREF5", &["iob-spec"]),
    TileInfo("MIB_L_DUMMY_BREF6", &["iob-spec"]),
    TileInfo("MIB_L_DUMMY_BREF7", &["iob-spec"]),
    TileInfo("MIB_B_DUMMY_VIQ_L", &["clk-spine-buf"]),
    TileInfo("MIB_B_DUMMY_VIQ_R", &["clk-spine-buf"]),
    TileInfo("MIB_T_DUMMY_VIQ_L", &["clk-spine-buf"]),
    TileInfo("MIB_T_DUMMY_VIQ_R", &["clk-spine-buf"]),
    TileInfo("MIB_T_DUMMY_VIQ_L_BIG", &["clk-spine-buf"]),
    TileInfo("MIB_T_DUMMY_VIQ_R_BIG", &["clk-spine-buf"]),
    TileInfo("MIB_L_DUMMY_HIQ_L", &["clk-spine-buf"]),
    TileInfo("MIB_L_DUMMY_HIQ_U", &["clk-spine-buf"]),
    TileInfo("MIB_R_DUMMY_HIQ_L", &["clk-spine-buf"]),
    TileInfo("MIB_R_DUMMY_HIQ_U", &["clk-spine-buf"]),
    TileInfo("MIB_EBR_VIQ_UL", &["clk-spine-buf"]),
    TileInfo("MIB_EBR_VIQ_UR", &["clk-spine-buf"]),
    TileInfo("MIB_EBR_VIQ_LL", &["clk-spine-buf"]),
    TileInfo("MIB_EBR_VIQ_LR", &["clk-spine-buf"]),
    TileInfo("MIB_EBR_VIQ_UL_SP", &["clk-spine"]),
    TileInfo("MIB_EBR_VIQ_UR_SP", &["clk-spine"]),
    TileInfo("MIB_EBR_VIQ_LL_SP", &["clk-spine"]),
    TileInfo("MIB_EBR_VIQ_LR_SP", &["clk-spine"]),
    TileInfo("MIB_L_DUMMY", &[]),
    TileInfo("MIB_R_DUMMY", &[]),
    TileInfo("MIB_LS_DUMMY", &[]),
    TileInfo("MIB_RS_DUMMY", &[]),
    TileInfo("MIB_B_DUMMY", &[]),
    TileInfo("MIB_T_DUMMY", &[]),
    TileInfo("DSP_DUMMY", &[]),
    TileInfo("DUMMY_END_EBR", &[]),
    TileInfo("DUMMY_END_DSP", &[]),
    TileInfo("MIB_EBR_DUMMY", &[]),
    TileInfo("MIB_EBR_DUMMY_B", &[]),
    TileInfo("DUMMY_END_B", &[]),
    TileInfo("DUMMY_END_T", &[]),
];

const ECP5_TILES: &[TileInfo] = &[
    TileInfo("PLC2", &["clbm"]),
    TileInfo("CIB", &["int"]),
    TileInfo("CIB_EBR", &["int"]),
    TileInfo("CIB_DSP", &["int"]),
    TileInfo("CIB_LR", &["int"]),
    TileInfo("CIB_LR_S", &["int"]),
    TileInfo("CIB_EFB0", &["int"]),
    TileInfo("CIB_EFB1", &["int"]),
    TileInfo("CIB_PLL0", &["int"]),
    TileInfo("CIB_PLL1", &["int"]),
    TileInfo("CIB_PLL2", &["int"]),
    TileInfo("CIB_PLL3", &["int"]),
    TileInfo("CIB_DCU0", &["int"]),
    TileInfo("CIB_DCUA", &["int"]),
    TileInfo("CIB_DCUB", &["int"]),
    TileInfo("CIB_DCUC", &["int"]),
    TileInfo("CIB_DCUD", &["int"]),
    TileInfo("CIB_DCUF", &["int"]),
    TileInfo("CIB_DCU3", &["int"]),
    TileInfo("CIB_DCU2", &["int"]),
    TileInfo("CIB_DCUG", &["int"]),
    TileInfo("CIB_DCUH", &["int"]),
    TileInfo("CIB_DCUI", &["int"]),
    TileInfo("CIB_DCU1", &["int"]),
    TileInfo("VCIB_DCU0", &["int"]),
    TileInfo("VCIB_DCUA", &["int"]),
    TileInfo("VCIB_DCUB", &["int"]),
    TileInfo("VCIB_DCUC", &["int"]),
    TileInfo("VCIB_DCUD", &["int"]),
    TileInfo("VCIB_DCUF", &["int"]),
    TileInfo("VCIB_DCU3", &["int"]),
    TileInfo("VCIB_DCU2", &["int"]),
    TileInfo("VCIB_DCUG", &["int"]),
    TileInfo("VCIB_DCUH", &["int"]),
    TileInfo("VCIB_DCUI", &["int"]),
    TileInfo("VCIB_DCU1", &["int"]),
    TileInfo("PICL0", &["io"]),
    TileInfo("PICL1", &["io"]),
    TileInfo("PICL2", &["io"]),
    TileInfo("PICL1_DQS0", &["io-spec"]),
    TileInfo("PICL2_DQS1", &["io-spec"]),
    TileInfo("PICL0_DQS2", &["io-spec"]),
    TileInfo("PICL1_DQS3", &["io-spec"]),
    TileInfo("MIB_CIB_LR", &["io"]),
    TileInfo("MIB_CIB_LRC", &["io"]),
    TileInfo("MIB_CIB_LX", &["io"]),
    TileInfo("PICR0", &["io"]),
    TileInfo("PICR1", &["io"]),
    TileInfo("PICR2", &["io"]),
    TileInfo("PICR1_DQS0", &["io-spec"]),
    TileInfo("PICR2_DQS1", &["io-spec"]),
    TileInfo("PICR0_DQS2", &["io-spec"]),
    TileInfo("PICR1_DQS3", &["io-spec"]),
    TileInfo("MIB_CIB_LR_A", &["io"]),
    TileInfo("MIB_CIB_LRC_A", &["io"]),
    TileInfo("MIB_CIB_RX", &["io"]),
    TileInfo("PICB0", &["io"]),
    TileInfo("PICB1", &["io"]),
    TileInfo("EFB0_PICB0", &["io", "cfg"]),
    TileInfo("EFB1_PICB1", &["io", "cfg"]),
    TileInfo("EFB2_PICB0", &["io", "cfg"]),
    TileInfo("EFB3_PICB1", &["io", "cfg"]),
    TileInfo("SPICB0", &["io"]),
    TileInfo("PICT0", &["io"]),
    TileInfo("PICT1", &["io"]),
    TileInfo("PIOT0", &["iob"]),
    TileInfo("PIOT1", &["iob"]),
    TileInfo("BANKREF0", &["iob-spec"]),
    TileInfo("BANKREF1", &["iob-spec"]),
    TileInfo("BANKREF2", &["iob-spec"]),
    TileInfo("BANKREF2A", &["iob-spec"]),
    TileInfo("BANKREF3", &["iob-spec"]),
    TileInfo("BANKREF4", &["iob-spec"]),
    TileInfo("BANKREF6", &["iob-spec"]),
    TileInfo("BANKREF7", &["iob-spec"]),
    TileInfo("BANKREF7A", &["iob-spec"]),
    TileInfo("BANKREF8", &["iob-spec", "pll"]),
    TileInfo("DTR", &["cfg"]),
    TileInfo("POR", &["cfg"]),
    TileInfo("OSC", &["cfg"]),
    TileInfo("PLL0_LL", &["pll"]),
    TileInfo("PLL0_LR", &["pll"]),
    TileInfo("PLL1_LR", &["pll"]),
    TileInfo("PLL0_UL", &["pll"]),
    TileInfo("PLL1_UL", &["pll"]),
    TileInfo("PLL0_UR", &["pll"]),
    TileInfo("PLL1_UR", &["pll"]),
    TileInfo("TMID_0", &["pll-alt"]),
    TileInfo("TMID_1", &["pll-alt"]),
    TileInfo("BMID_0H", &["clk-spine-buf"]),
    TileInfo("BMID_2", &["clk-spine-buf"]),
    TileInfo("BMID_0V", &["clk-spine-buf"]),
    TileInfo("BMID_2V", &["clk-spine-buf"]),
    TileInfo("MIB_EBR0", &["bram"]),
    TileInfo("MIB_EBR1", &["bram"]),
    TileInfo("MIB_EBR2", &["bram"]),
    TileInfo("MIB_EBR3", &["bram"]),
    TileInfo("MIB_EBR4", &["bram"]),
    TileInfo("MIB_EBR5", &["bram"]),
    TileInfo("MIB_EBR6", &["bram"]),
    TileInfo("MIB_EBR7", &["bram"]),
    TileInfo("MIB_EBR8", &["bram"]),
    TileInfo("EBR_SPINE_LL2", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_LL1", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_LL0", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_LR0", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_LR1", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_LR2", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_UL2", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_UL1", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_UL0", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_UR0", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_UR1", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_SPINE_UR2", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_CMUX_LL_25K", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_CMUX_LR_25K", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_CMUX_LL", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_CMUX_LR", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_CMUX_UL", &["bram", "clk-spine-buf"]),
    TileInfo("EBR_CMUX_UR", &["bram", "clk-spine-buf"]),
    TileInfo("DDRDLL_LL", &["pll-alt"]),
    TileInfo("DDRDLL_LR", &["pll-alt"]),
    TileInfo("DDRDLL_UL", &["pll-alt"]),
    TileInfo("DDRDLL_UR", &["pll-alt"]),
    TileInfo("PVT_COUNT2", &["clk-spine-buf"]),
    TileInfo("EBR_SPINE_LL3", &["clk-spine"]),
    TileInfo("LMID_0", &["clk-spine-buf"]),
    TileInfo("ECLK_L", &["clk-spine-buf"]),
    TileInfo("RMID_0", &["clk-spine-buf"]),
    TileInfo("ECLK_R", &["clk-spine-buf"]),
    TileInfo("CMUX_LL_0", &["clk-spine-buf"]),
    TileInfo("CMUX_LR_0", &["clk-spine-buf"]),
    TileInfo("MIB_DSP0", &["dsp"]),
    TileInfo("MIB_DSP1", &["dsp"]),
    TileInfo("MIB_DSP2", &["dsp"]),
    TileInfo("MIB_DSP3", &["dsp"]),
    TileInfo("MIB_DSP4", &["dsp"]),
    TileInfo("MIB_DSP5", &["dsp"]),
    TileInfo("MIB_DSP6", &["dsp"]),
    TileInfo("MIB_DSP7", &["dsp"]),
    TileInfo("MIB_DSP8", &["dsp"]),
    TileInfo("DSP_SPINE_UL0", &["dsp", "clk-spine-buf"]),
    TileInfo("DSP_SPINE_UR0", &["dsp", "clk-spine-buf"]),
    TileInfo("DSP_SPINE_UR1", &["dsp", "clk-spine-buf"]),
    TileInfo("MIB2_DSP0", &["dsp"]),
    TileInfo("MIB2_DSP1", &["dsp"]),
    TileInfo("MIB2_DSP2", &["dsp"]),
    TileInfo("MIB2_DSP3", &["dsp"]),
    TileInfo("MIB2_DSP4", &["dsp"]),
    TileInfo("MIB2_DSP5", &["dsp"]),
    TileInfo("MIB2_DSP6", &["dsp"]),
    TileInfo("MIB2_DSP7", &["dsp"]),
    TileInfo("MIB2_DSP8", &["dsp"]),
    TileInfo("DDRDLL_ULA", &["pll-alt"]),
    TileInfo("DDRDLL_URA", &["pll-alt"]),
    TileInfo("DSP_SPINE_UL1", &["clk-spine-buf"]),
    TileInfo("CMUX_UL_0", &["clk-spine-buf"]),
    TileInfo("CMUX_UR_0", &["clk-spine-buf"]),
    TileInfo("DSP_CMUX_UL", &["clk-spine-buf"]),
    TileInfo("DSP_CMUX_UR", &["clk-spine-buf"]),
    TileInfo("VIQ_BUF", &["clk-spine-buf"]),
    TileInfo("TAP_DRIVE", &["clk-row"]),
    TileInfo("TAP_DRIVE_CIB", &["clk-row"]),
    TileInfo("DCU0", &["gtp"]),
    TileInfo("DCU1", &["gtp"]),
    TileInfo("DCU2", &["gtp"]),
    TileInfo("DCU3", &["gtp"]),
    TileInfo("DCU4", &["gtp"]),
    TileInfo("DCU5", &["gtp"]),
    TileInfo("DCU6", &["gtp"]),
    TileInfo("DCU7", &["gtp"]),
    TileInfo("DCU8", &["gtp"]),
    TileInfo("DUMMY_TILE_0", &[]),
    TileInfo("DUMMY_TILE_1", &[]),
    TileInfo("DUMMY_TILE_2", &[]),
    TileInfo("DUMMY_TILE_4", &[]),
    TileInfo("DUMMY_TILE_5", &[]),
    TileInfo("DUMMY_TILE_6", &[]),
    TileInfo("DUMMY_TILE_7", &[]),
    TileInfo("DUMMY_TILE_8", &[]),
    TileInfo("DUMMY_TILE_A", &[]),
    TileInfo("DUMMY_TILE_E", &[]),
    TileInfo("DUMMY_TILE_F", &[]),
    TileInfo("DUMMY_TILE_S", &[]),
    TileInfo("DUMMY_TILE_T", &[]),
];

const CROSSLINK_TILES: &[TileInfo] = &[
    TileInfo("PLC_CR", &["clbm"]),
    TileInfo("CIB_CR", &["int"]),
    TileInfo("CIB_T_CR", &["int"]),
    TileInfo("MIB_EBR0", &["bram"]),
    TileInfo("MIB_EBR1", &["bram"]),
    TileInfo("MIB_EBR2", &["bram"]),
    TileInfo("MIB_EBR3", &["bram"]),
    TileInfo("MIB_EBR4", &["bram"]),
    TileInfo("MIB_EBR5", &["bram"]),
    TileInfo("MIB_EBR6", &["bram"]),
    TileInfo("MIB_EBR7", &["bram"]),
    TileInfo("MIB_EBR8", &["bram"]),
    TileInfo("SPINE_L0", &["clk-spine-buf"]),
    TileInfo("SPINE_R0", &["clk-spine-buf"]),
    TileInfo("SPINE_R1", &["clk-spine-buf"]),
    TileInfo("CMUX_0", &["clk-spine-buf"]),
    TileInfo("CMUX_1", &["clk-spine-buf"]),
    TileInfo("GPIO", &["io"]),
    TileInfo("LVDS_0", &["io"]),
    TileInfo("LVDS_1", &["io"]),
    TileInfo("LVDS_2", &["io"]),
    TileInfo("LVDS_3", &["io"]),
    TileInfo("EFB0_LVDS_1", &["io", "cfg"]),
    TileInfo("EFB1_LVDS_2", &["io", "cfg"]),
    TileInfo("EFB2_LVDS_3", &["io", "cfg"]),
    TileInfo("BANK_DDR_L", &["pll-alt"]),
    TileInfo("BANK_DDR_R", &["pll-alt"]),
    TileInfo("PCLK_DLY_2", &["pll-alt"]),
    TileInfo("PCLK_DLY_1", &["pll-alt"]),
    TileInfo("ECLK_B", &["clk-spine-buf"]),
    TileInfo("BMID", &["clk-spine-buf"]),
    TileInfo("TMID", &["clk-spine-buf"]),
    TileInfo("MIB_OSC", &["cfg"]),
    TileInfo("MIB_PLL0", &["pll"]),
    TileInfo("MIB_PLL1", &["pll"]),
    TileInfo("MIB_L_MIPI10", &["gtp"]),
    TileInfo("MIB_L_MIPI11", &["gtp"]),
    TileInfo("MIB_L_MIPI12", &["gtp"]),
    TileInfo("MIB_L_MIPI13", &["gtp"]),
    TileInfo("MIB_R_MIPI10", &["gtp"]),
    TileInfo("MIB_R_MIPI11", &["gtp"]),
    TileInfo("MIB_R_MIPI12", &["gtp"]),
    TileInfo("MIB_R_MIPI13", &["gtp"]),
    TileInfo("MIB_I2C_0", &["hardip"]),
    TileInfo("MIB_I2C_1", &["hardip"]),
    TileInfo("TAP_PLC", &["clk-row"]),
    TileInfo("TAP_PLC_R", &["clk-row"]),
    TileInfo("TAP_CIB", &["clk-row"]),
    TileInfo("TAP_CIB_R", &["clk-row"]),
    TileInfo("TAP_CIB_T", &["clk-row"]),
    TileInfo("TAP_CIB_T_R", &["clk-row"]),
    TileInfo("DUMMY_2X1", &[]),
    TileInfo("DUMMY_2X2", &[]),
    TileInfo("DUMMY_106X1", &[]),
    TileInfo("DUMMY_106X2", &[]),
];

const MACHXO2_TILES: &[TileInfo] = &[
    TileInfo("PLC", &["clbm"]),
    TileInfo("CIB_EBR0", &["int"]),
    TileInfo("CIB_EBR1", &["int"]),
    TileInfo("CIB_EBR2", &["int"]),
    TileInfo("CIB_EBR0_10K", &["int"]),
    TileInfo("CIB_EBR1_10K", &["int"]),
    TileInfo("CIB_EBR2_10K", &["int"]),
    TileInfo("CIB_EBR0_640", &["int"]),
    TileInfo("CIB_EBR1_640", &["int"]),
    TileInfo("CIB_EBR2_640", &["int"]),
    TileInfo("CIB_EBR2_640_END", &["int"]),
    TileInfo("CIB_EBR_DUMMY", &["int"]),
    TileInfo("CIB_EBR_DUMMY_10K", &["int"]),
    TileInfo("CIB_PIC_B0", &["int"]),
    TileInfo("CIB_PIC_B0_256", &["int"]),
    TileInfo("CIB_PIC_BS0_256", &["int"]),
    TileInfo("CIB_PIC_B0_640", &["int"]),
    TileInfo("CIB_PIC_B_DUMMY", &["int"]),
    TileInfo("CIB_PIC_B_DUMMY_256", &["int"]),
    TileInfo("CIB_PIC_B_DUMMY_640", &["int"]),
    TileInfo("CIB_PIC_T_DUMMY", &["int"]),
    TileInfo("CIB_PIC_T0", &["int"]),
    TileInfo("CIB_PIC_TS0", &["int"]),
    TileInfo("CIB_CFG0", &["int"]),
    TileInfo("CIB_CFG1", &["int"]),
    TileInfo("CIB_CFG2", &["int"]),
    TileInfo("CIB_CFG3", &["int"]),
    TileInfo("CIB_HSE", &["int"]),
    TileInfo("PIC_L0_DUMMY", &["int"]),
    TileInfo("PIC_L0_DUMMY_256", &["clk-spine-buf"]),
    TileInfo("PIC_L1_DUMMY", &["int"]),
    TileInfo("PIC_L1_DUMMY_640", &["clk-spine-buf"]),
    TileInfo("PIC_L2_DUMMY", &["int"]),
    TileInfo("PIC_R0_DUMMY", &["int"]),
    TileInfo("PIC_R0_DUMMY_256", &["clk-spine-buf"]),
    TileInfo("PIC_R1_DUMMY", &["int"]),
    TileInfo("PIC_R1_DUMMY_640", &["clk-spine-buf"]),
    TileInfo("ULC0", &["int"]),
    TileInfo("ULC0_256", &["cfg"]),
    TileInfo("ULC1", &["int"]),
    TileInfo("ULC1_640", &["cfg"]),
    TileInfo("ULC2", &["int"]),
    TileInfo("URC0", &["int"]),
    TileInfo("URC1", &["int"]),
    TileInfo("LRC0", &["int"]),
    TileInfo("LRC1", &["int"]),
    TileInfo("LLC0", &["int"]),
    TileInfo("LLC1", &["pll-alt"]),
    TileInfo("LLC2", &["pll-alt"]),
    TileInfo("CIB_EBR0_END0", &["pll-alt"]),
    TileInfo("CIB_EBR0_END0_10K", &["pll-alt"]),
    TileInfo("CIB_EBR0_END0_DLL3", &["pll-alt"]),
    TileInfo("CIB_EBR0_END0_DLL5", &["pll-alt"]),
    TileInfo("CIB_EBR0_END1", &["pll-alt"]),
    TileInfo("CIB_EBR0_END2_DLL3", &["pll-alt"]),
    TileInfo("CIB_EBR0_END2_DLL45", &["pll-alt"]),
    TileInfo("CIB_EBR2_END1_SP", &["int"]),
    TileInfo("CIB_EBR2_END1_10K", &["pll-alt"]),
    TileInfo("CIB_EBR2_END1", &["pll-alt"]),
    TileInfo("CIB_EBR2_END0", &["pll-alt"]),
    TileInfo("CIB_EBR_DUMMY_END3", &["pll-alt"]),
    TileInfo("PIC_L0", &["io"]),
    TileInfo("PIC_L0_I3C", &["io-spec"]),
    TileInfo("PIC_LS0", &["io"]),
    TileInfo("PIC_L0_VREF3", &["io-spec"]),
    TileInfo("PIC_L0_VREF4", &["io-spec"]),
    TileInfo("PIC_L0_VREF5", &["io-spec"]),
    TileInfo("PIC_L1", &["io"]),
    TileInfo("PIC_L1_I3C", &["io-spec"]),
    TileInfo("PIC_L1_VREF3", &["io-spec"]),
    TileInfo("PIC_L1_VREF4", &["io-spec"]),
    TileInfo("PIC_L1_VREF5", &["io-spec"]),
    TileInfo("PIC_L2", &["io"]),
    TileInfo("PIC_L2_VREF4", &["io-spec"]),
    TileInfo("PIC_L2_VREF5", &["io-spec"]),
    TileInfo("PIC_L3", &["io"]),
    TileInfo("PIC_L3_VREF4", &["io-spec"]),
    TileInfo("PIC_L3_VREF5", &["io-spec"]),
    TileInfo("LLC0PIC_VREF3", &["io-spec"]),
    TileInfo("LLC0PIC_I3C_VREF3", &["io-spec"]),
    TileInfo("LLC3PIC_VREF3", &["io-spec"]),
    TileInfo("PIC_R0", &["io"]),
    TileInfo("PIC_R0_256", &["io"]),
    TileInfo("PIC_RS0", &["io"]),
    TileInfo("PIC_RS0_256", &["io"]),
    TileInfo("PIC_R1", &["io"]),
    TileInfo("PIC_R1_640", &["io"]),
    TileInfo("LRC1PIC2", &["io"]),
    TileInfo("PIC_B0", &["io"]),
    TileInfo("PIC_B0_256", &["io"]),
    TileInfo("PIC_BS0_256", &["io"]),
    TileInfo("PIC_B_DUMMY_VIQ", &["pll-alt"]),
    TileInfo("PIC_B_DUMMY_VIQ_VREF", &["pll-alt"]),
    TileInfo("PIC_T0", &["io"]),
    TileInfo("PIC_T0_256", &["io"]),
    TileInfo("PIC_TS0", &["io"]),
    TileInfo("DQSDLL_L", &["pll-alt"]),
    TileInfo("DQSDLL_R", &["pll-alt"]),
    TileInfo("GPLL_L0", &["pll"]),
    TileInfo("GPLL_R0", &["pll"]),
    TileInfo("CFG0", &["cfg"]),
    TileInfo("CFG0_ENDL", &["cfg"]),
    TileInfo("CFG1", &["cfg"]),
    TileInfo("CFG2", &["cfg"]),
    TileInfo("CFG3", &["cfg"]),
    TileInfo("PIC_T_DUMMY_OSC", &["cfg"]),
    TileInfo("PIC_T_DUMMY_VIQ", &["pll-alt"]),
    TileInfo("PIC_T_DUMMY_VIQ_256", &[]),
    TileInfo("URC0VREF", &["iob-spec"]),
    TileInfo("PIC_B_DUMMY_VREF", &["iob-spec"]),
    TileInfo("B_DUMMY_ENDR_VREF2", &["iob-spec"]),
    TileInfo("LLC0PIC", &["io-spec"]),
    TileInfo("LLC1PIC", &["io-spec"]),
    TileInfo("ULC3PIC", &["io-spec"]),
    TileInfo("URC1PIC", &["io-spec"]),
    TileInfo("LRC0PIC", &["io-spec"]),
    TileInfo("LRC1PIC1", &["io-spec"]),
    TileInfo("EBR0", &["bram"]),
    TileInfo("EBR1", &["bram"]),
    TileInfo("EBR2", &["bram"]),
    TileInfo("EBR0_END", &["bram"]),
    TileInfo("EBR2_END", &["bram"]),
    TileInfo("EBR0_10K", &["bram"]),
    TileInfo("EBR1_10K", &["bram"]),
    TileInfo("EBR2_10K", &["bram"]),
    TileInfo("EBR0_END_10K", &["bram"]),
    TileInfo("EBR2_END_10K", &["bram"]),
    TileInfo("EBR0_640", &["bram"]),
    TileInfo("EBR1_640", &["bram"]),
    TileInfo("EBR2_640", &["bram"]),
    TileInfo("EBR2_640_END", &["bram"]),
    TileInfo("CENTER_DUMMY", &["clk-spine"]),
    TileInfo("CENTER_EBR", &["clk-spine-buf"]),
    TileInfo("CENTER_EBR_SP", &["clk-spine"]),
    TileInfo("CENTER_EBR_CIB_SP", &["clk-spine"]),
    TileInfo("CENTER_EBR_CIB_4K", &["clk-spine-buf"]),
    TileInfo("CENTER_EBR_CIB_10K", &["clk-spine-buf"]),
    TileInfo("CENTER_EBR_CIB", &["clk-spine-buf"]),
    TileInfo("CENTER_B", &["clk-spine"]),
    TileInfo("CENTER_B_CIB", &["clk-spine"]),
    TileInfo("CENTER_T", &["clk-spine"]),
    TileInfo("CENTER_T_CIB", &["clk-spine"]),
    TileInfo("CENTER0", &["clk-spine-buf"]),
    TileInfo("CENTER1", &["clk-spine-buf"]),
    TileInfo("CENTER2", &["clk-spine-buf"]),
    TileInfo("CENTER3", &["clk-spine-buf"]),
    TileInfo("CENTER_B_CIB_256", &["clk-spine-buf"]),
    TileInfo("CENTER_T_CIB_256", &["clk-spine-buf"]),
    TileInfo("CENTER4_640", &["clk-spine-buf"]),
    TileInfo("CENTER4", &["clk-spine-buf"]),
    TileInfo("CENTER5", &["clk-spine-buf"]),
    TileInfo("CENTER6", &["clk-spine-buf"]),
    TileInfo("CENTER7", &["clk-spine-buf"]),
    TileInfo("CENTER8", &["clk-spine-buf"]),
    TileInfo("CENTER9", &["clk-spine-buf"]),
    TileInfo("CENTERA", &["clk-spine-buf"]),
    TileInfo("CENTERB", &["clk-spine-buf"]),
    TileInfo("CENTERC", &["clk-spine-buf"]),
    TileInfo("EBR_DUMMY", &[]),
    TileInfo("EBR_DUMMY_END", &[]),
    TileInfo("PIC_B_DUMMY", &[]),
    TileInfo("PIC_T_DUMMY", &[]),
    TileInfo("B_DUMMY_ENDL", &[]),
    TileInfo("B_DUMMY_ENDR", &[]),
    TileInfo("T_DUMMY_ENDR", &[]),
];

const NX_TILES: &[TileInfo] = &[
    TileInfo("PLC", &["clbm"]),
    TileInfo("CIB", &["int"]),
    TileInfo("CIB_T", &["int"]),
    TileInfo("CIB_LR", &["int"]),
    TileInfo("CIB_LR_A", &["int"]),
    TileInfo("LRAM_0_15K", &["uram"]), // right
    TileInfo("LRAM_1_15K", &["uram"]), // right
    TileInfo("LRAM_2_15K", &["uram"]), // left
    TileInfo("LRAM_3_15K", &["uram"]), // left
    TileInfo("LRAM_4_15K", &["uram"]), // left
    TileInfo("LRAM_0", &["uram"]),
    TileInfo("LRAM_1", &["uram"]),
    TileInfo("LRAM_2", &["uram"]),
    TileInfo("LRAM_3", &["uram"]),
    TileInfo("LRAM_4", &["uram"]),
    TileInfo("LRAM_5", &["uram"]),
    TileInfo("LRAM_6", &["uram"]),
    TileInfo("PCS_0", &["gtx"]),
    TileInfo("PCS_1", &["gtx"]),
    TileInfo("PCS_2", &["gtx"]),
    TileInfo("PCS_3", &["gtx"]),
    TileInfo("PCS_4", &["gtx"]),
    TileInfo("PCS_5", &["gtx"]),
    TileInfo("PCS_6", &["gtx"]),
    TileInfo("PCS_7", &["gtx"]),
    TileInfo("SERDES_REFCLK", &["gtclk"]),
    // ???
    TileInfo("RBB_0", &["cfg"]),
    TileInfo("RBB_1", &["cfg"]),
    TileInfo("RBB_2", &["cfg"]),
    TileInfo("RBB_3", &["cfg"]),
    TileInfo("RBB_4", &["cfg"]),
    TileInfo("RBB_5", &["cfg"]),
    TileInfo("RBB_6", &["cfg"]),
    TileInfo("RBB_7", &["cfg"]),
    TileInfo("RBB_8", &["cfg"]),
    TileInfo("RBB_9", &["cfg"]),
    TileInfo("RBB_10", &["cfg"]),
    TileInfo("RBB_11", &["cfg"]),
    TileInfo("RBB_12", &["cfg"]),
    TileInfo("RBB_13", &["cfg"]),
    TileInfo("RBB_14", &["cfg"]),
    TileInfo("RBB_15", &["cfg"]),
    TileInfo("CDR0", &["gtp"]),
    TileInfo("CDR1", &["gtp"]),
    TileInfo("CDR1_RBB_15", &["gtp", "cfg"]),
    TileInfo("CDR0_RBB_11", &["gtp", "cfg"]),
    TileInfo("CDR1_RBB_13", &["gtp", "cfg"]),
    TileInfo("RBB_0_15K", &["cfg"]),
    TileInfo("RBB_1_15K", &["cfg"]),
    TileInfo("RBB_2_15K", &["cfg"]),
    TileInfo("RBB_3_15K", &["cfg"]),
    TileInfo("RBB_4_15K", &["cfg"]),
    TileInfo("LMID_RBB_5_15K", &["cfg", "clk-spine-buf"]),
    TileInfo("LMID_RBB_7", &["cfg", "clk-spine-buf"]),
    TileInfo("RBB_6_15K", &["cfg"]),
    TileInfo("RBB_7_15K", &["cfg"]),
    TileInfo("RBB_8_15K", &["cfg"]),
    TileInfo("RBB_9_15K", &["cfg"]),
    TileInfo("RBB_10_15K", &["cfg"]),
    TileInfo("RBB_11_15K", &["cfg"]),
    TileInfo("RBB_12_15K", &["cfg"]),
    TileInfo("RBB_13_15K", &["cfg"]),
    TileInfo("RBB_14_15K", &["cfg"]),
    TileInfo("RBB_15_15K", &["cfg"]),
    TileInfo("LMID", &["clk-spine-buf"]),
    TileInfo("CLKBUF_L_30K", &["clk-spine-buf"]),
    TileInfo("PCIE_X1", &["hardip"]),                 // top
    TileInfo("PCIE_LL", &["hardip"]),                 // top
    TileInfo("CDR0_15K", &["gtp"]),                   // bot
    TileInfo("CDR1_15K", &["gtp"]),                   // bot
    TileInfo("PVTCAL18_15K", &["cfg"]),               // bot
    TileInfo("V51_15K", &["cfg"]),                    // bot
    TileInfo("DDR_OSC_L_15K", &["pll-alt"]),          // bot
    TileInfo("DOSCL_P18_V18", &["pll-alt"]),          // bot
    TileInfo("DOSCL_P18", &["pll-alt"]),              // bot
    TileInfo("DOSCL_P18_RBB_9", &["pll-alt", "cfg"]), // bot
    TileInfo("CLKBUF_L_15K", &["cfg", "clk-spine-buf"]),
    TileInfo("GPLL_ULC", &["pll"]),                             // top
    TileInfo("GPLL_URC", &["pll"]),                             // top
    TileInfo("MIPI_DPHY_0", &["gtp"]),                          // top
    TileInfo("MIPI_DPHY_0_15K", &["gtp"]),                      // left
    TileInfo("MIPI_DPHY_1", &["gtp"]),                          // top
    TileInfo("MIPI_DPHY_1_15K", &["gtp"]),                      // top
    TileInfo("DPHY_CLKMUX0", &["gtp"]),                         // top
    TileInfo("DPHY_CLKMUX0_15K", &["gtp"]),                     // top
    TileInfo("DPHY_CLKMUX1", &["gtp"]),                         // top
    TileInfo("DPHY_CLKMUX1_15K", &["gtp"]),                     // top
    TileInfo("BK0_15K", &["iob-spec"]),                         // top
    TileInfo("BANKREF0", &["iob-spec"]),                        // top
    TileInfo("BANKREF5", &["iob-spec"]),                        // top
    TileInfo("SYSIO_B0", &["io"]),                              // top
    TileInfo("SYSIO_B1", &["io"]),                              // top
    TileInfo("SYSIO_B2", &["io"]),                              // top
    TileInfo("SYSIO_B2_DED", &["io"]),                          // top
    TileInfo("SYSIO_B2_C", &["io"]),                            // top
    TileInfo("SYSIO_B2_REM", &["io"]),                          // top
    TileInfo("SYSIO_B3", &["io"]),                              // top
    TileInfo("SYSIO_B3_C", &["io"]),                            // top
    TileInfo("SYSIO_B3_REM", &["io"]),                          // top
    TileInfo("SYSIO_B4", &["io"]),                              // top
    TileInfo("SYSIO_B4_C", &["io"]),                            // top
    TileInfo("SYSIO_B4_REM", &["io"]),                          // top
    TileInfo("SYSIO_B5", &["io"]),                              // top
    TileInfo("SYSIO_B7", &["io"]),                              // top
    TileInfo("SYSIO_B7_C", &["io"]),                            // top
    TileInfo("SYSIO_B7_REM", &["io"]),                          // top
    TileInfo("SYSIO_B8", &["io"]),                              // top
    TileInfo("SYSIO_B8_C", &["io"]),                            // top
    TileInfo("SYSIO_B8_REM", &["io"]),                          // top
    TileInfo("SYSIO_B9", &["io"]),                              // top
    TileInfo("SYSIO_B9_C", &["io"]),                            // top
    TileInfo("SYSIO_B9_REM", &["io"]),                          // top
    TileInfo("SYSIO_B0_0", &["io"]),                            // top
    TileInfo("SYSIO_B0_0_15K", &["io"]),                        // top
    TileInfo("EFB_15K", &["cfg"]),                              // top
    TileInfo("EFB_0", &["cfg"]),                                // top
    TileInfo("PMU_15K", &["cfg"]),                              // top
    TileInfo("PMU", &["cfg"]),                                  // top
    TileInfo("OSC_15K", &["cfg", "pll-alt"]),                   // top
    TileInfo("EFB_1_OSC", &["cfg", "pll-alt"]),                 // top
    TileInfo("EFB_1", &["cfg"]),                                // top
    TileInfo("OSC", &["pll-alt"]),                              // top
    TileInfo("EFB_2", &["cfg"]),                                // top
    TileInfo("I2C_15K", &["hardip"]),                           // top
    TileInfo("I2C_EFB_3", &["cfg", "hardip"]),                  // top
    TileInfo("EFB_3", &["cfg"]),                                // top
    TileInfo("I2C", &["hardip"]),                               // top
    TileInfo("POR_15K", &["cfg"]),                              // top
    TileInfo("PVTCAL33_15K", &["cfg"]),                         // top
    TileInfo("PPT_QOUT_15K", &["cfg"]),                         // top
    TileInfo("IREF_15K", &["cfg"]),                             // right
    TileInfo("IREF_P33", &["cfg"]),                             // right
    TileInfo("POR", &["cfg"]),                                  // right
    TileInfo("ALU", &["hardip"]),                               // right
    TileInfo("BK1_15K", &["iob-spec"]),                         // right
    TileInfo("BANKREF1", &["iob-spec"]),                        // right
    TileInfo("BANKREF2", &["iob-spec"]),                        // right
    TileInfo("SYSIO_B1_0", &["io"]),                            // right
    TileInfo("SYSIO_B1_0_C", &["io"]),                          // right
    TileInfo("SYSIO_B1_0_REM", &["io"]),                        // right
    TileInfo("SYSIO_B1_0_15K", &["io"]),                        // right
    TileInfo("SYSIO_B1_1_15K", &["io"]),                        // right
    TileInfo("SYSIO_B1_DED_15K", &["io-spec"]),                 // right
    TileInfo("SYSIO_B1_DED", &["io-spec"]),                     // right
    TileInfo("PIC_B1_DED_15K", &["io-spec"]),                   // right
    TileInfo("RMID_PICB_DLY10", &["pll-alt", "clk-spine-buf"]), // right
    TileInfo("RMID_DLY20", &["pll-alt", "clk-spine-buf"]),      // right
    TileInfo("RMID", &["clk-spine-buf"]),                       // right
    TileInfo("DLY10_C_15K", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_10", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_12", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_20", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_22", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_30", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_31", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_32", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_50", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_52", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_60", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_62", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_70", &["pll-alt"]),                      // right
    TileInfo("PCLK_DLY_60_RBB_8", &["pll-alt", "cfg"]),         // left
    TileInfo("PCLK_DLY_70_RBB_6", &["pll-alt", "cfg"]),         // left
    TileInfo("PCLK_DLY_80_RBB_6", &["pll-alt", "cfg"]),         // left
    TileInfo("PCLK_DLY_81_RBB_8", &["pll-alt", "cfg"]),         // left
    TileInfo("SYSIO_B2_0", &["io"]),                            // right
    TileInfo("SYSIO_B2_0_C", &["io"]),                          // right
    TileInfo("SYSIO_B2_0_REM", &["io"]),                        // right
    TileInfo("SYSIO_B2_1", &["io"]),                            // right
    TileInfo("SYSIO_B2_1_V18_21", &["io-spec"]),
    TileInfo("SYSIO_B2_1_V18_22", &["io-spec"]),
    TileInfo("GPLL_LRC_15K", &["pll"]),          // bot
    TileInfo("GPLL_LRC", &["pll"]),              // right
    TileInfo("DDR_OSC_R_15K_V32", &["pll-alt"]), // bot
    TileInfo("DDR_OSC_R", &["pll-alt"]),         // right
    TileInfo("BK3_15K", &["iob-spec"]),          // bot
    TileInfo("BANKREF3", &["iob-spec"]),         // bot
    TileInfo("BANKREF3_DDR42", &["iob-spec"]),   // bot
    TileInfo("BANKREF4_V18", &["iob-spec"]),     // bot
    TileInfo("BANKREF4", &["iob-spec"]),         // bot
    TileInfo("BANKREF4_DLY42_DDR41", &["iob-spec", "pll-alt"]), // bot
    TileInfo("SYSIO_B3_0", &["io"]),             // bot
    TileInfo("SYSIO_B3_1", &["io"]),             // bot
    TileInfo("SYSIO_B3_0_ECLK_L", &["io-spec"]), // bot
    TileInfo("SYSIO_B3_1_DQS0", &["io-spec"]),   // bot
    TileInfo("SYSIO_B3_0_DQS1", &["io-spec"]),   // bot
    TileInfo("SYSIO_B3_1_DQS2", &["io-spec"]),   // bot
    TileInfo("SYSIO_B3_0_DQS3", &["io-spec"]),   // bot
    TileInfo("SYSIO_B3_1_DQS4", &["io-spec"]),   // bot
    TileInfo("SYSIO_B3_1_15K_DQS30", &["io-spec"]), // bot
    TileInfo("SYSIO_B3_1_15K_ECLK_R_DQS31", &["io-spec"]), // bot
    TileInfo("SYSIO_B3_0_15K_DQS32", &["io-spec"]), // bot
    TileInfo("SYSIO_B3_1_ECLK_R", &["io-spec"]), // bot
    TileInfo("SYSIO_B3_1_V18", &["io-spec"]),    // bot
    TileInfo("SYSIO_B3_1_V18_31", &["io-spec"]), // bot
    TileInfo("SYSIO_B3_1_V18_32", &["io-spec"]), // bot
    TileInfo("SYSIO_B3_0_DLY30_V18", &["io-spec", "pll-alt"]), // bot
    TileInfo("SYSIO_B3_1_DLY32", &["io-spec", "pll-alt"]), // bot
    TileInfo("DLY30", &["pll-alt"]),             // bot
    TileInfo("DLY32", &["pll-alt"]),             // bot
    TileInfo("DLY40", &["pll-alt"]),             // bot
    TileInfo("DLY50", &["pll-alt"]),             // bot
    TileInfo("DLY52", &["pll-alt"]),             // bot
    TileInfo("SYSIO_B4_0", &["io"]),             // bot
    TileInfo("SYSIO_B4_1", &["io"]),             // bot
    TileInfo("PIC_B4_0_15K", &["io"]),           // bot
    TileInfo("SYSIO_B4_0_DLY50", &["io", "pll-alt"]), // bot
    TileInfo("SYSIO_B4_1_DLY52", &["io", "pll-alt"]), // bot
    TileInfo("SYSIO_B4_0_DLY42", &["io", "pll-alt"]), // bot
    TileInfo("SYSIO_B4_1_DQS0", &["io-spec"]),   // bot
    TileInfo("SYSIO_B4_0_DQS1", &["io-spec"]),   // bot
    TileInfo("SYSIO_B4_1_DQS2", &["io-spec"]),   // bot
    TileInfo("SYSIO_B4_0_DQS3", &["io-spec"]),   // bot
    TileInfo("SYSIO_B4_1_DQS4", &["io-spec"]),   // bot
    TileInfo("SYSIO_B4_1_DQS0_MID", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_0_DQS1_MID", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_1_DQS2_MID", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_0_DQS3_MID", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_1_DQS4_MID", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_0_15K_V31", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_0_15K_BK4_V42", &["io-spec"]), // bot
    TileInfo("PIC_B4_1_15K_DLY40", &["io-spec", "pll-alt"]), // bot
    TileInfo("PIC_B4_0_15K_DLY32", &["io-spec", "pll-alt"]), // bot
    TileInfo("PIC_B4_1_15K_DLY30_DQS40", &["io-spec", "pll-alt"]), // bot
    TileInfo("SYSIO_B4_1_15K_DQS41", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_0_15K_DQS42", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_1_V18", &["io-spec"]),    // bot
    TileInfo("SYSIO_B4_1_V18_41", &["io-spec"]), // bot
    TileInfo("SYSIO_B4_1_V18_42", &["io-spec"]), // bot
    TileInfo("DDR40_ECLK_L", &["iob-spec"]),     // bot
    TileInfo("SYSIO_B5_0", &["io"]),             // bot
    TileInfo("SYSIO_B5_1", &["io"]),             // bot
    TileInfo("SYSIO_B5_1_ECLK_R", &["io-spec"]), // bot
    TileInfo("SYSIO_B5_1_V18", &["io-spec"]),    // bot
    TileInfo("SYSIO_B5_1_V18_51", &["io-spec"]), // bot
    TileInfo("SYSIO_B5_1_V18_52", &["io-spec"]), // bot
    TileInfo("SYSIO_B5_1_DQS0", &["io-spec"]),   // bot
    TileInfo("SYSIO_B5_0_DQS1", &["io-spec"]),   // bot
    TileInfo("SYSIO_B5_1_DQS2", &["io-spec"]),   // bot
    TileInfo("SYSIO_B5_0_DQS3", &["io-spec"]),   // bot
    TileInfo("SYSIO_B5_1_DQS4", &["io-spec"]),   // bot
    TileInfo("SYSIO_B5_1_15K_DQS50", &["io-spec"]), // bot
    TileInfo("SYSIO_B5_1_15K_DQS51", &["io-spec"]), // bot
    TileInfo("SYSIO_B5_0_15K_DQS52", &["io-spec"]), // bot
    TileInfo("SYSIO_B5_1_15K_ECLK_L_V52", &["io-spec"]), // bot
    TileInfo("IO_B4_0_15K_DLY52_BK5", &["io-spec", "pll-alt"]), // bot
    TileInfo("IO_B4_1_15K_DLY50", &["io-spec", "pll-alt"]), // bot
    TileInfo("IO_B4_0_15K_DLY42", &["io-spec", "pll-alt"]), // bot
    TileInfo("IO_B4_1_15K_V41", &["io-spec"]),   // bot
    TileInfo("GPLL_LLC", &["pll"]),              // left
    TileInfo("GPLL_LLC_15K", &["pll"]),          // left
    TileInfo("BANKREF5_V18_ECLK_L", &["iob-spec"]), // left
    TileInfo("BANKREF6", &["iob-spec"]),         // left
    TileInfo("BANKREF7", &["iob-spec"]),         // left
    TileInfo("BANKREF7_RBB_2", &["iob-spec", "cfg"]), // left
    TileInfo("BANKREF7_RBB_10", &["iob-spec", "cfg"]), // left
    TileInfo("BANKREF8_RBB_5", &["iob-spec", "cfg"]), // left
    TileInfo("BANKREF9_RBB_4", &["iob-spec", "cfg"]), // left
    TileInfo("SYSIO_B6_0", &["io"]),             // left
    TileInfo("SYSIO_B6_1", &["io"]),             // left
    TileInfo("SYSIO_B6_0_ECLK_L", &["io-spec"]), // bot
    TileInfo("SYSIO_B6_1_V18_61", &["io-spec"]), // bot
    TileInfo("SYSIO_B6_1_V18_62", &["io-spec"]), // bot
    TileInfo("SYSIO_B6_1_DQS0", &["io-spec"]),   // bot
    TileInfo("SYSIO_B6_0_DQS1", &["io-spec"]),   // bot
    TileInfo("SYSIO_B6_1_DQS2", &["io-spec"]),   // bot
    TileInfo("SYSIO_B6_0_DQS3", &["io-spec"]),   // bot
    TileInfo("SYSIO_B6_1_DQS4", &["io-spec"]),   // bot
    TileInfo("SYSIO_B6_0_C", &["io"]),           // left
    TileInfo("SYSIO_B6_0_REM", &["io"]),         // left
    TileInfo("SYSIO_B7_0", &["io"]),             // left
    TileInfo("SYSIO_B7_0_C", &["io"]),           // left
    TileInfo("SYSIO_B7_0_REM", &["io"]),         // left
    TileInfo("ADC", &["sysmon"]),                // right
    TileInfo("DSP_L_0", &["dsp"]),
    TileInfo("DSP_L_1", &["dsp"]),
    TileInfo("DSP_L_2", &["dsp"]),
    TileInfo("DSP_L_3", &["dsp"]),
    TileInfo("DSP_L_4", &["dsp"]),
    TileInfo("DSP_L_5", &["dsp"]),
    TileInfo("DSP_L_6", &["dsp"]),
    TileInfo("DSP_L_7", &["dsp"]),
    TileInfo("DSP_L_8", &["dsp"]),
    TileInfo("DSP_L_9", &["dsp"]),
    TileInfo("DSP_L_10", &["dsp"]),
    TileInfo("DSP_R_1", &["dsp"]),
    TileInfo("DSP_R_2", &["dsp"]),
    TileInfo("DSP_R_3", &["dsp"]),
    TileInfo("DSP_R_4", &["dsp"]),
    TileInfo("DSP_R_5", &["dsp"]),
    TileInfo("DSP_R_6", &["dsp"]),
    TileInfo("DSP_R_7", &["dsp"]),
    TileInfo("DSP_R_8", &["dsp"]),
    TileInfo("DSP_R_9", &["dsp"]),
    TileInfo("DSP_R_10", &["dsp"]),
    TileInfo("DSP_R_11", &["dsp"]),
    TileInfo("EBR_1", &["bram"]),
    TileInfo("EBR_2", &["bram"]),
    TileInfo("EBR_4", &["bram"]),
    TileInfo("EBR_5", &["bram"]),
    TileInfo("EBR_7", &["bram"]),
    TileInfo("EBR_8", &["bram"]),
    TileInfo("EBR_9", &["bram"]),
    TileInfo("EBR_10", &["bram"]),
    TileInfo("SPINE_L1", &["clk-spine-buf"]),
    TileInfo("SPINE_L0", &["clk-spine-buf"]),
    TileInfo("SPINE_R0", &["clk-spine-buf"]),
    TileInfo("SPINE_R1", &["clk-spine-buf"]),
    TileInfo("SPINE_LL2", &["clk-spine-buf"]),
    TileInfo("SPINE_LL1", &["clk-spine-buf"]),
    TileInfo("SPINE_LL0", &["clk-spine-buf"]),
    TileInfo("SPINE_LR0", &["clk-spine-buf"]),
    TileInfo("SPINE_LR1", &["clk-spine-buf"]),
    TileInfo("SPINE_LR2", &["clk-spine-buf"]),
    TileInfo("SPINE_LR3", &["clk-spine-buf"]),
    TileInfo("SPINE_UL2", &["clk-spine-buf"]),
    TileInfo("SPINE_UL1", &["clk-spine-buf"]),
    TileInfo("SPINE_UL0", &["clk-spine-buf"]),
    TileInfo("SPINE_UR0", &["clk-spine-buf"]),
    TileInfo("SPINE_UR1", &["clk-spine-buf"]),
    TileInfo("SPINE_UR2", &["clk-spine-buf"]),
    TileInfo("SPINE_UR3", &["clk-spine-buf"]),
    TileInfo("CMUX_0_TL", &["clk-spine-buf"]),
    TileInfo("CMUX_1_GSR_TR", &["clk-spine-buf"]),
    TileInfo("CMUX_1_GSR", &["clk-spine-buf"]),
    TileInfo("CMUX_0", &["clk-spine-buf"]),
    TileInfo("CMUX_1", &["clk-spine-buf"]),
    TileInfo("CMUX_2", &["clk-spine-buf"]),
    TileInfo("CMUX_3", &["clk-spine-buf"]),
    TileInfo("CMUX_2_TRUNK_LL", &["clk-spine-buf"]),
    TileInfo("CMUX_3_TRUNK_LR", &["clk-spine-buf"]),
    TileInfo("CMUX_4_TRUNK_UL", &["clk-spine-buf"]),
    TileInfo("CMUX_5_TRUNK_UR", &["clk-spine-buf"]),
    TileInfo("CMUX_6", &["clk-spine-buf"]),
    TileInfo("CMUX_7", &["clk-spine-buf"]),
    TileInfo("TMID_0", &["clk-spine-buf"]),
    TileInfo("TMID_1", &["clk-spine-buf"]),
    TileInfo("TMID_1_15K", &["clk-spine-buf"]),
    TileInfo("CLKBUF_T_15K", &["clk-spine-buf"]),
    TileInfo("ECLK_0", &["clk-spine-buf"]),
    TileInfo("BMID_0_ECLK_1", &["clk-spine-buf"]),
    TileInfo("BMID_1_ECLK_2", &["clk-spine-buf"]),
    TileInfo("ECLK_3", &["clk-spine-buf"]),
    TileInfo("ECLK_R", &["clk-spine-buf"]),
    TileInfo("TRUNK_L_EBR_10", &["bram", "clk-spine-buf"]),
    TileInfo("TRUNK_R", &["clk-spine-buf"]),
    TileInfo("TAP_PLC", &["clk-spine"]),
    TileInfo("TAP_PLC_1S", &["clk-spine"]),
    TileInfo("TAP_PLC_1S_L", &["clk-spine"]),
    TileInfo("TAP_CIB", &["clk-spine"]),
    TileInfo("TAP_CIB_1S", &["clk-spine"]),
    TileInfo("TAP_CIB_1S_L", &["clk-spine"]),
    TileInfo("TAP_CIBT", &["clk-spine"]),
    TileInfo("TAP_CIBT_1S", &["clk-spine"]),
    TileInfo("TAP_CIBT_1S_L", &["clk-spine"]),
    TileInfo("MIB_EBR_TAP", &["clk-spine"]),
    TileInfo("MIB_B_TAP", &["clk-spine"]),
    TileInfo("MIB_T_TAP", &["clk-spine"]),
    TileInfo("CIB_LR_B", &[]),
    TileInfo("MIB_LR", &[]),
    TileInfo("MIB_LR_T", &[]),
    TileInfo("MIB_LR_FA", &[]),
    TileInfo("MIB_LR_B", &[]),
    TileInfo("MIB_LR_B_FA", &[]),
    TileInfo("MIB_LR_C", &[]),
    TileInfo("MIB_LR_C_FA", &[]),
    TileInfo("MIB_LR_REM", &[]),
    TileInfo("MIB_LR_REM_FA", &[]),
    TileInfo("MIB_B", &[]),
    TileInfo("MIB_T", &[]),
    TileInfo("MIB_EBR", &[]),
    TileInfo("MIB_CNR_32", &[]),
    TileInfo("MIB_CNR_32_FA", &[]),
    TileInfo("MIB_CNR_32_FD", &[]),
    TileInfo("MIB_CNR_32_FAFD", &[]),
];

const UNK_TILES: &[TileInfo] = &[
    TileInfo("FPLC", &["clb"]),
    TileInfo("PLC", &["clbm"]),
    TileInfo("PLC2", &["clbm"]),
];

pub fn dump_html(
    file: &Path,
    arch: &str,
    part: &str,
    tiles: &Array2<Tile>,
) -> Result<(), Box<dyn Error>> {
    let mut f = File::create(file)?;
    writeln!(f, "<html><head><title>{part}</title><style>")?;
    f.write_all(r#"
        table { border-collapse: collapse }
        td { margin: 0px; padding: 5px; border: solid 1px black; position: relative; }
        td > div { position: absolute; top: 5px; left: 5px; background: black; color: white; display: none; z-index: 1; padding: 5px; }
        td:hover > div { display: block; }
        td._rpad { padding: 5px; padding-right:1000px; border: none; position: relative; }
        td._bpad { padding: 5px; padding-bottom:1000px; border: none; position: relative; }
        td._unk { animation: unknown 1s infinite; }
        @keyframes unknown {
            0% { background: white; }
            50% { background: red; }
            100% { background: white; }
        }
    "#.as_bytes())?;
    let tile_info = match arch {
        "xp" => XP_TILES,
        "ecp" => ECP_TILES,
        "machxo" => MACHXO_TILES,
        "scm" => SCM_TILES,
        "xp2" => XP2_TILES,
        "ecp2" => ECP2_TILES,
        "ecp3" => ECP3_TILES,
        "ecp4" => ECP4_TILES,
        "ecp5" => ECP5_TILES,
        "crosslink" => CROSSLINK_TILES,
        "machxo2" => MACHXO2_TILES,
        "nx" => NX_TILES,
        _ => UNK_TILES,
    };
    let tile_info_d = tile_info
        .iter()
        .map(|t| (t.0, t))
        .collect::<HashMap<_, _>>();
    // XXX colors
    let colors = COLORS.iter().copied().collect::<HashMap<_, _>>();
    for t in tile_info.iter() {
        let tname = t.0;
        let tcls = t.1;
        let mut cs = Vec::new();
        for cls in tcls {
            match colors.get(cls) {
                None => panic!("missing color {}", cls),
                Some(c) => cs.push(c),
            }
        }
        match cs.len() {
            0 => (),
            1 => writeln!(f, "td.{} {{ background: rgb({}, {}, {}); }}\n", tname, cs[0].0, cs[0].1, cs[0].2)?,
            2 => writeln!(f, "td.{} {{ background: linear-gradient(45deg, rgb({}, {}, {}) 40%, rgb({}, {}, {}) 60%); }}\n", tname, cs[0].0, cs[0].1, cs[0].2, cs[1].0, cs[1].1, cs[1].2)?,
            3 => writeln!(f, "td.{} {{ background: linear-gradient(45deg, rgb({}, {}, {}) 25%, rgb({}, {}, {}) 50%, rgb({}, {}, {}) 75%); }}\n", tname, cs[0].0, cs[0].1, cs[0].2, cs[1].0, cs[1].1, cs[1].2, cs[2].0, cs[2].1, cs[2].2)?,
            _ => panic!("too colorful {}", tname),
        }
    }
    writeln!(f, "</style></head><body><table><tr>")?;
    for row in tiles.rows() {
        for tile in &row {
            let cls = match tile_info_d.get(&tile.kind[..]) {
                None => {
                    println!("unknown tile {}", tile.kind);
                    "_unk"
                }
                Some(_) => &tile.kind,
            };
            writeln!(f, "<td class=\"{cls}\">")?;
            write!(
                f,
                "<div>{kind}<br/>{name}<br/>({w},&nbsp;{h})&nbsp;at&nbsp;({x},&nbsp;{y})",
                kind = tile.kind,
                name = tile.name,
                w = tile.width,
                h = tile.height,
                x = tile.x,
                y = tile.y
            )?;
            for site in &tile.sites {
                write!(
                    f,
                    "<br/>{name}&nbsp;({x},&nbsp;{y})",
                    name = site.name,
                    x = site.x,
                    y = site.y
                )?;
            }
            writeln!(f, "</div></td>")?;
        }
        writeln!(f, "<td class=\"_rpad\"></td>")?;
        writeln!(f, "</tr><tr>")?;
    }
    for _ in 0..tiles.dim().1 {
        writeln!(f, "<td class=\"_bpad\"></td>")?;
    }
    writeln!(f, "</tr></table></body></html>")?;
    Ok(())
}
