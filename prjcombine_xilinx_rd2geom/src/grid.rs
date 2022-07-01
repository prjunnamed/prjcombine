use prjcombine_xilinx_rawdump::Part;
use prjcombine_xilinx_geom::{Grid, Bond, DeviceBond, Device, DisabledPart, GeomDb, DeviceCombo, ExtraDie, BondId, GridId, DevBondId, DevSpeedId, ColId, RowId, SlrId, int};
use prjcombine_entity::{EntityVec, EntitySet, EntityMap};
use std::collections::{BTreeSet, BTreeMap, btree_map};

pub struct PreDevice {
    pub name: String,
    pub grids: EntityVec<SlrId, Grid>,
    pub grid_master: SlrId,
    pub extras: Vec<ExtraDie>,
    pub bonds: EntityVec<DevBondId, (String, Bond)>,
    pub speeds: EntityVec<DevSpeedId, String>,
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
}

pub fn make_device_multi(rd: &Part, grids: EntityVec<SlrId, Grid>, grid_master: SlrId, extras: Vec<ExtraDie>, mut bonds: Vec<(String, Bond)>, disabled: BTreeSet<DisabledPart>) -> PreDevice {
    let mut speeds = EntitySet::new();
    bonds.sort_by(|x, y| x.0.cmp(&y.0));
    let bonds = EntityMap::<DevBondId, _, _>::from_iter(bonds);
    let combos = rd.combos.iter().map(|c| DeviceCombo {
        name: c.name.clone(),
        devbond_idx: bonds.get(&c.package).unwrap().0,
        speed_idx: speeds.insert(c.speed.clone()).0,
    }).collect();
    PreDevice {
        name: rd.part.clone(),
        grids,
        grid_master,
        extras,
        bonds: bonds.into_vec(),
        speeds: speeds.into_vec(),
        combos,
        disabled,
    }
}

pub fn make_device(rd: &Part, grid: Grid, bonds: Vec<(String, Bond)>, disabled: BTreeSet<DisabledPart>) -> PreDevice {
    let mut grids = EntityVec::new();
    let grid_master = grids.push(grid);
    make_device_multi(rd, grids, grid_master, vec![], bonds, disabled)
}

pub struct GridBuilder {
    grids: EntityVec<GridId, Grid>,
    bonds: EntityVec<BondId, Bond>,
    devices: Vec<Device>,
    ints: BTreeMap<String, int::IntDb>,
}

impl GridBuilder {
    pub fn new() -> Self {
        Self {
            grids: EntityVec::new(),
            bonds: EntityVec::new(),
            devices: Vec::new(),
            ints: BTreeMap::new(),
        }
    }

    pub fn insert_grid(&mut self, grid: Grid) -> GridId {
        for (i, g) in self.grids.iter() {
            if g == &grid {
                return i;
            }
        }
        self.grids.push(grid)
    }

    pub fn insert_bond(&mut self, bond: Bond) -> BondId {
        for (k, v) in self.bonds.iter() {
            if v == &bond {
                return k;
            }
        }
        self.bonds.push(bond)
    }

    pub fn ingest(&mut self, pre: PreDevice) {
        let grids = pre.grids.into_map_values(|x| self.insert_grid(x));
        let bonds = pre.bonds.into_map_values(|(name, b)| DeviceBond { name, bond: self.insert_bond(b) });
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

    pub fn ingest_int(&mut self, int: int::IntDb) {
        match self.ints.entry(int.name.clone()) {
            btree_map::Entry::Vacant(x) => {
                x.insert(int);
            }
            btree_map::Entry::Occupied(mut x) => {
                let x = x.get_mut();
                assert_eq!(x.wires, int.wires);
                macro_rules! merge_dicts {
                    ($f:ident) => {
                        for (_, k, v) in int.$f {
                            match x.$f.get(&k) {
                                None => {
                                    x.$f.insert(k, v);
                                }
                                Some((_, v2)) => {
                                    if v != *v2 {
                                        println!("FAIL at {}", k);
                                    }
                                    assert_eq!(&v, v2);
                                }
                            }
                        }
                    }
                }
                merge_dicts!(nodes);
                merge_dicts!(terms);
                merge_dicts!(intfs);
                merge_dicts!(bels);
                for (_, k, v) in int.namings {
                    match x.namings.get_mut(&k) {
                        None => {
                            x.namings.insert(k, v);
                        }
                        Some((_, v2)) => {
                            for (kk, vv) in v {
                                match v2.get(kk) {
                                    None => {
                                        v2.insert(kk, vv);
                                    }
                                    Some(vv2) => {
                                        assert_eq!(&vv, vv2);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn finish(self) -> GeomDb {
        GeomDb {
            grids: self.grids,
            bonds: self.bonds,
            devices: self.devices,
            ints: self.ints,
        }
    }
}

pub fn find_columns(rd: &Part, tts: &[&str]) -> BTreeSet<i32> {
    let mut res = BTreeSet::new();
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

pub fn find_rows(rd: &Part, tts: &[&str]) -> BTreeSet<i32> {
    let mut res = BTreeSet::new();
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

pub fn find_tiles(rd: &Part, tts: &[&str]) -> BTreeSet<(i32, i32)> {
    let mut res = BTreeSet::new();
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
pub struct IntGrid<'a> {
    pub rd: &'a Part,
    pub cols: EntityVec<ColId, i32>,
    pub rows: EntityVec<RowId, i32>,
    pub slr_start: u16,
    pub slr_end: u16,
}

impl IntGrid<'_> {
    pub fn lookup_column(&self, col: i32) -> ColId {
        self.cols.binary_search(&col).unwrap()
    }

    pub fn lookup_column_inter(&self, col: i32) -> ColId {
        self.cols.binary_search(&col).unwrap_err()
    }

    pub fn lookup_row(&self, row: i32) -> RowId {
        self.rows.binary_search(&row).unwrap()
    }

    pub fn lookup_row_inter(&self, row: i32) -> RowId {
        self.rows.binary_search(&row).unwrap_err()
    }

    pub fn find_tiles(&self, tts: &[&str]) -> BTreeSet<(i32, i32)> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            if let Some(tk) = self.rd.tile_kinds.get(tt) {
                for t in &tk.tiles {
                    if (self.slr_start..self.slr_end).contains(&t.y) {
                        res.insert((t.x as i32, t.y as i32));
                    }
                }
            }
        }
        res
    }

    pub fn find_rows(&self, tts: &[&str]) -> BTreeSet<i32> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            if let Some(tk) = self.rd.tile_kinds.get(tt) {
                for t in &tk.tiles {
                    if (self.slr_start..self.slr_end).contains(&t.y) {
                        res.insert(t.y as i32);
                    }
                }
            }
        }
        res
    }

    pub fn find_row(&self, tts: &[&str]) -> Option<i32> {
        let res = self.find_rows(tts);
        if res.len() > 1 {
            panic!("more than one row found for {:?}", tts);
        }
        res.into_iter().next()
    }

    pub fn find_columns(&self, tts: &[&str]) -> BTreeSet<i32> {
        let mut res = BTreeSet::new();
        for &tt in tts {
            if let Some(tk) = self.rd.tile_kinds.get(tt) {
                for t in &tk.tiles {
                    if (self.slr_start..self.slr_end).contains(&t.y) {
                        res.insert(t.x as i32);
                    }
                }
            }
        }
        res
    }

    pub fn find_column(&self, tts: &[&str]) -> Option<i32> {
        let res = self.find_columns(tts);
        if res.len() > 1 {
            panic!("more than one column found for {:?}", tts);
        }
        res.into_iter().next()
    }
}

#[derive(Clone, Debug, Copy)]
pub struct ExtraCol {
    pub tts: &'static [&'static str],
    pub dx: &'static [i32],
}

pub fn extract_int<'a>(rd: &'a Part, tts: &[&str], extra_cols: &[ExtraCol]) -> IntGrid<'a> {
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
        rd,
        cols: cols.into_iter().collect(),
        rows: rows.into_iter().collect(),
        slr_start: 0,
        slr_end: rd.height,
    }
}

pub fn extract_int_slr<'a>(rd: &'a Part, tts: &[&str], extra_cols: &[ExtraCol], slr_start: u16, slr_end: u16) -> IntGrid<'a> {
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
        rd,
        cols: cols.into_iter().collect(),
        rows: rows.into_iter().filter(|&x| (slr_start..slr_end).contains(&(x as u16))).collect(),
        slr_start,
        slr_end,
    }
}
