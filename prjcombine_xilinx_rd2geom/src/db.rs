use prjcombine_entity::{EntityMap, EntitySet, EntityVec};
use prjcombine_xilinx_geom::pkg::Bond;
use prjcombine_xilinx_geom::{
    int, BondId, DevBondId, DevSpeedId, Device, DeviceBond, DeviceCombo, DisabledPart, ExtraDie,
    GeomDb, Grid, GridId, SlrId,
};
use prjcombine_xilinx_rawdump::Part;
use std::collections::{btree_map, BTreeMap, BTreeSet};

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

pub fn make_device_multi(
    rd: &Part,
    grids: EntityVec<SlrId, Grid>,
    grid_master: SlrId,
    extras: Vec<ExtraDie>,
    mut bonds: Vec<(String, Bond)>,
    disabled: BTreeSet<DisabledPart>,
) -> PreDevice {
    let mut speeds = EntitySet::new();
    bonds.sort_by(|x, y| x.0.cmp(&y.0));
    let bonds = EntityMap::<DevBondId, _, _>::from_iter(bonds);
    let combos = rd
        .combos
        .iter()
        .map(|c| DeviceCombo {
            name: c.name.clone(),
            devbond_idx: bonds.get(&c.package).unwrap().0,
            speed_idx: speeds.get_or_insert(&c.speed),
        })
        .collect();
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

pub fn make_device(
    rd: &Part,
    grid: Grid,
    bonds: Vec<(String, Bond)>,
    disabled: BTreeSet<DisabledPart>,
) -> PreDevice {
    let mut grids = EntityVec::new();
    let grid_master = grids.push(grid);
    make_device_multi(rd, grids, grid_master, vec![], bonds, disabled)
}

pub struct DbBuilder {
    grids: EntityVec<GridId, Grid>,
    bonds: EntityVec<BondId, Bond>,
    devices: Vec<Device>,
    ints: BTreeMap<String, int::IntDb>,
}

impl DbBuilder {
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
        let bonds = pre.bonds.into_map_values(|(name, b)| DeviceBond {
            name,
            bond: self.insert_bond(b),
        });
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
                    };
                }
                merge_dicts!(nodes);
                merge_dicts!(terms);
                merge_dicts!(intfs);
                for (_, k, v) in int.node_namings {
                    match x.node_namings.get_mut(&k) {
                        None => {
                            x.node_namings.insert(k, v);
                        }
                        Some((_, v2)) => {
                            for (kk, vv) in v.wires {
                                match v2.wires.get(&kk) {
                                    None => {
                                        v2.wires.insert(kk, vv);
                                    }
                                    Some(vv2) => {
                                        assert_eq!(&vv, vv2);
                                    }
                                }
                            }
                            assert_eq!(v.wire_bufs, v2.wire_bufs);
                            assert_eq!(v.ext_pips, v2.ext_pips);
                            assert_eq!(v.bels, v2.bels);
                        }
                    }
                }
                merge_dicts!(term_namings);
                for (_, k, v) in int.intf_namings {
                    match x.intf_namings.get_mut(&k) {
                        None => {
                            x.intf_namings.insert(k, v);
                        }
                        Some((_, v2)) => {
                            for (kk, vv) in v.wires_in {
                                match v2.wires_in.get(kk) {
                                    None => {
                                        v2.wires_in.insert(kk, vv);
                                    }
                                    Some(vv2) => {
                                        assert_eq!(&vv, vv2);
                                    }
                                }
                            }
                            for (kk, vv) in v.wires_out {
                                match v2.wires_out.get(kk) {
                                    None => {
                                        v2.wires_out.insert(kk, vv);
                                    }
                                    Some(vv2 @ int::IntfWireOutNaming::Buf(no, _)) => match vv {
                                        int::IntfWireOutNaming::Buf(_, _) => assert_eq!(&vv, vv2),
                                        int::IntfWireOutNaming::Simple(ono) => assert_eq!(&ono, no),
                                    },
                                    Some(vv2 @ int::IntfWireOutNaming::Simple(n)) => {
                                        if let int::IntfWireOutNaming::Buf(no, _) = &vv {
                                            assert_eq!(no, n);
                                            v2.wires_out.insert(kk, vv);
                                        } else {
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
