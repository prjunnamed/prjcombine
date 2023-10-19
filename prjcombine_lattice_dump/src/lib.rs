use ndarray::Array2;
use prjcombine_lattice_rawdump::{Tile, TileSite};
use regex::Regex;
use std::collections::{HashMap, HashSet};

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
            tile.as_mut().unwrap().sites.push(TileSite {
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
