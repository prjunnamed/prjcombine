use prjcombine_xilinx_rawdump::Part;
use prjcombine_xilinx_geom::{Grid, Bond, DeviceBond, Device, DisabledPart, GeomDb, DeviceCombo, ExtraDie};
use std::collections::{HashSet, HashMap, BTreeSet};
use itertools::Itertools;

pub struct PreDevice {
    pub name: String,
    pub grids: Vec<Grid>,
    pub grid_master: usize,
    pub extras: Vec<ExtraDie>,
    pub bonds: Vec<(String, Bond)>,
    pub speeds: Vec<String>,
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

pub fn make_device_multi(rd: &Part, grids: Vec<Grid>, grid_master: usize, extras: Vec<ExtraDie>, bonds: Vec<(String, Bond)>, disabled: BTreeSet<DisabledPart>) -> PreDevice {
    let speeds: Vec<_> = rd.combos.iter().map(|c| c.speed.clone()).unique().collect();
    let bonds_lut: HashMap<_, _> = bonds.iter().enumerate().map(|(i, b)| (b.0.clone(), i)).collect();
    let speeds_lut: HashMap<_, _> = speeds.iter().enumerate().map(|(i, s)| (s.clone(), i)).collect();
    let combos = rd.combos.iter().map(|c| DeviceCombo {
        name: c.name.clone(),
        devbond_idx: bonds_lut[&c.package],
        speed_idx: speeds_lut[&c.speed],
    }).collect();
    PreDevice {
        name: rd.part.clone(),
        grids,
        grid_master,
        extras,
        bonds,
        speeds,
        combos,
        disabled: disabled,
    }
}

pub fn make_device(rd: &Part, grid: Grid, bonds: Vec<(String, Bond)>, disabled: BTreeSet<DisabledPart>) -> PreDevice {
    make_device_multi(rd, vec![grid], 0, vec![], bonds, disabled)
}

pub struct GridBuilder {
    grids: Vec<Grid>,
    bonds: Vec<Bond>,
    devices: Vec<Device>,
}

impl GridBuilder {
    pub fn new() -> Self {
        Self {
            grids: Vec::new(),
            bonds: Vec::new(),
            devices: Vec::new(),
        }
    }

    pub fn insert_grid(&mut self, grid: Grid) -> usize {
        for (i, g) in self.grids.iter().enumerate() {
            if g == &grid {
                return i;
            }
        }
        let res = self.grids.len();
        self.grids.push(grid);
        res
    }

    pub fn insert_bond(&mut self, bond: Bond) -> usize {
        for (i, b) in self.bonds.iter().enumerate() {
            if b == &bond {
                return i;
            }
        }
        let res = self.bonds.len();
        self.bonds.push(bond);
        res
    }

    pub fn ingest(&mut self, pre: PreDevice) {
        let grids = pre.grids.into_iter().map(|x| self.insert_grid(x)).collect();
        let bonds = pre.bonds.into_iter().map(|(name, b)| DeviceBond { name, bond_idx: self.insert_bond(b) }).collect();
        self.devices.push(Device {
            name: pre.name,
            grids,
            grid_master: pre.grid_master,
            extras: pre.extras,
            bonds,
            speeds: pre.speeds,
            combos: pre.combos,
            disabled: pre.disabled,
        });
    }

    pub fn finish(self) -> GeomDb {
        GeomDb {
            grids: self.grids,
            bonds: self.bonds,
            devices: self.devices,
        }
    }
}

pub fn find_columns(rd: &Part, tts: &[&str]) -> HashSet<i32> {
    let mut res = HashSet::new();
    for &tt in tts {
        if let Some(tk) = rd.tile_kinds.get(tt) {
            for t in &tk.tiles {
                res.insert(t.x as i32);
            }
        }
    }
    res
}

pub fn find_column(rd: &Part, tts: &[&str]) -> Option<i32> {
    let res = find_columns(rd, tts);
    if res.len() > 1 {
        panic!("more than one column found for {:?}", tts);
    }
    res.into_iter().next()
}

pub fn find_rows(rd: &Part, tts: &[&str]) -> HashSet<i32> {
    let mut res = HashSet::new();
    for &tt in tts {
        if let Some(tk) = rd.tile_kinds.get(tt) {
            for t in &tk.tiles {
                res.insert(t.y as i32);
            }
        }
    }
    res
}

pub fn find_row(rd: &Part, tts: &[&str]) -> Option<i32> {
    let res = find_rows(rd, tts);
    if res.len() > 1 {
        panic!("more than one row found for {:?}", tts);
    }
    res.into_iter().next()
}

pub fn find_tiles(rd: &Part, tts: &[&str]) -> HashSet<(i32, i32)> {
    let mut res = HashSet::new();
    for &tt in tts {
        if let Some(tk) = rd.tile_kinds.get(tt) {
            for t in &tk.tiles {
                res.insert((t.x as i32, t.y as i32));
            }
        }
    }
    res
}

#[derive(Clone, Debug)]
pub struct IntGrid {
    pub cols: Vec<i32>,
    pub rows: Vec<i32>,
}

impl IntGrid {
    pub fn lookup_column(&self, col: i32) -> u32 {
        self.cols.binary_search(&col).unwrap() as u32
    }
    pub fn lookup_column_inter(&self, col: i32) -> u32 {
        self.cols.binary_search(&col).unwrap_err() as u32
    }
    pub fn lookup_row(&self, row: i32) -> u32 {
        self.rows.binary_search(&row).unwrap() as u32
    }
    pub fn lookup_row_inter(&self, row: i32) -> u32 {
        self.rows.binary_search(&row).unwrap_err() as u32
    }
}

#[derive(Clone, Debug, Copy)]
pub struct ExtraCol {
    pub tts: &'static [&'static str],
    pub dx: &'static [i32],
}

pub fn extract_int(rd: &Part, tts: &[&str], extra_cols: &[ExtraCol]) -> IntGrid {
    let mut cols = find_columns(rd, tts);
    let rows = find_rows(rd, tts);
    for ec in extra_cols {
        for c in find_columns(rd, ec.tts) {
            for d in ec.dx {
                cols.insert(c + d);
            }
        }
    }
    IntGrid {
        cols: cols.into_iter().sorted().collect(),
        rows: rows.into_iter().sorted().collect(),
    }
}
