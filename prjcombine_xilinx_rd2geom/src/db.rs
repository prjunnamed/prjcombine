use prjcombine_entity::{EntityMap, EntitySet, EntityVec};
use prjcombine_int::db::{IntDb, IntfWireOutNaming};
use prjcombine_int::grid::DieId;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{
    Bond, BondId, DevBondId, DevSpeedId, Device, DeviceBond, DeviceCombo, DeviceNaming,
    DeviceNamingId, DisabledPart, ExtraDie, GeomDb, Grid, GridId,
};
use std::collections::{btree_map, BTreeMap, BTreeSet};

pub struct PreDevice {
    pub name: String,
    pub grids: EntityVec<DieId, Grid>,
    pub grid_master: DieId,
    pub extras: Vec<ExtraDie>,
    pub bonds: EntityVec<DevBondId, (String, Bond)>,
    pub speeds: EntityVec<DevSpeedId, String>,
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
    pub naming: DeviceNaming,
}

pub fn make_device_multi(
    rd: &Part,
    grids: EntityVec<DieId, Grid>,
    grid_master: DieId,
    extras: Vec<ExtraDie>,
    mut bonds: Vec<(String, Bond)>,
    disabled: BTreeSet<DisabledPart>,
    naming: DeviceNaming,
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
        naming,
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
    make_device_multi(
        rd,
        grids,
        grid_master,
        vec![],
        bonds,
        disabled,
        DeviceNaming::Dummy,
    )
}

pub struct DbBuilder {
    grids: EntityVec<GridId, Grid>,
    bonds: EntityVec<BondId, Bond>,
    dev_namings: EntityVec<DeviceNamingId, DeviceNaming>,
    devices: Vec<Device>,
    ints: BTreeMap<String, IntDb>,
}

impl DbBuilder {
    pub fn new() -> Self {
        Self {
            grids: EntityVec::new(),
            bonds: EntityVec::new(),
            dev_namings: EntityVec::new(),
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

    pub fn insert_dev_naming(&mut self, naming: DeviceNaming) -> DeviceNamingId {
        for (k, v) in self.dev_namings.iter() {
            if v == &naming {
                return k;
            }
        }
        self.dev_namings.push(naming)
    }

    pub fn ingest(&mut self, pre: PreDevice) {
        let grids = pre.grids.into_map_values(|x| self.insert_grid(x));
        let bonds = pre.bonds.into_map_values(|(name, b)| DeviceBond {
            name,
            bond: self.insert_bond(b),
        });
        let naming = self.insert_dev_naming(pre.naming);
        self.devices.push(Device {
            name: pre.name,
            grids,
            grid_master: pre.grid_master,
            extras: pre.extras,
            bonds,
            speeds: pre.speeds,
            combos: pre.combos,
            disabled: pre.disabled,
            naming,
        });
    }

    pub fn ingest_int(&mut self, int: IntDb) {
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
                            for (kk, vv) in v.intf_wires_in {
                                match v2.intf_wires_in.get(&kk) {
                                    None => {
                                        v2.intf_wires_in.insert(kk, vv);
                                    }
                                    Some(vv2) => {
                                        assert_eq!(&vv, vv2);
                                    }
                                }
                            }
                            for (kk, vv) in v.intf_wires_out {
                                match v2.intf_wires_out.get(&kk) {
                                    None => {
                                        v2.intf_wires_out.insert(kk, vv);
                                    }
                                    Some(vv2 @ IntfWireOutNaming::Buf(no, _)) => match vv {
                                        IntfWireOutNaming::Buf(_, _) => assert_eq!(&vv, vv2),
                                        IntfWireOutNaming::Simple(ono) => assert_eq!(&ono, no),
                                    },
                                    Some(vv2 @ IntfWireOutNaming::Simple(n)) => {
                                        if let IntfWireOutNaming::Buf(no, _) = &vv {
                                            assert_eq!(no, n);
                                            v2.intf_wires_out.insert(kk, vv);
                                        } else {
                                            assert_eq!(&vv, vv2);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                merge_dicts!(term_namings);
            }
        }
    }

    pub fn finish(self) -> GeomDb {
        GeomDb {
            grids: self.grids,
            bonds: self.bonds,
            dev_namings: self.dev_namings,
            devices: self.devices,
            ints: self.ints,
        }
    }
}
