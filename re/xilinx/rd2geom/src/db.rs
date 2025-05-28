use prjcombine_interconnect::db::IntDb;
use prjcombine_interconnect::grid::DieId;
use prjcombine_re_xilinx_geom::{
    Bond, BondId, Chip, ChipId, DevBondId, DevSpeedId, Device, DeviceBond, DeviceCombo,
    DeviceNaming, DeviceNamingId, DisabledPart, GeomDb, Interposer, InterposerId,
};
use prjcombine_re_xilinx_naming::db::{IntfWireOutNaming, NamingDb};
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_virtex4::gtz::GtzDb;
use std::collections::{BTreeMap, BTreeSet, btree_map};
use unnamed_entity::{EntityMap, EntitySet, EntityVec};

pub struct PreDevice {
    pub name: String,
    pub grids: EntityVec<DieId, Chip>,
    pub interposer: Interposer,
    pub bonds: EntityVec<DevBondId, (String, Bond)>,
    pub speeds: EntityVec<DevSpeedId, String>,
    pub combos: Vec<DeviceCombo>,
    pub disabled: BTreeSet<DisabledPart>,
    pub naming: DeviceNaming,
    pub intdb_name: String,
    pub intdb: IntDb,
    pub ndb: NamingDb,
    pub gtz: GtzDb,
}

#[allow(clippy::too_many_arguments)]
pub fn make_device_multi(
    rd: &Part,
    grids: EntityVec<DieId, Chip>,
    interposer: Interposer,
    gtz: GtzDb,
    mut bonds: Vec<(String, Bond)>,
    disabled: BTreeSet<DisabledPart>,
    naming: DeviceNaming,
    intdb_name: impl Into<String>,
    intdb: IntDb,
    ndb: NamingDb,
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
        interposer,
        gtz,
        bonds: bonds.into_vec(),
        speeds: speeds.into_vec(),
        combos,
        disabled,
        naming,
        intdb_name: intdb_name.into(),
        intdb,
        ndb,
    }
}

pub fn make_device(
    rd: &Part,
    grid: Chip,
    bonds: Vec<(String, Bond)>,
    disabled: BTreeSet<DisabledPart>,
    intdb_name: impl Into<String>,
    intdb: IntDb,
    ndb: NamingDb,
) -> PreDevice {
    let mut grids = EntityVec::new();
    grids.push(grid);
    make_device_multi(
        rd,
        grids,
        Interposer::None,
        GtzDb::default(),
        bonds,
        disabled,
        DeviceNaming::Dummy,
        intdb_name,
        intdb,
        ndb,
    )
}

pub struct DbBuilder {
    grids: EntityVec<ChipId, Chip>,
    bonds: EntityVec<BondId, Bond>,
    interposers: EntityVec<InterposerId, Interposer>,
    dev_namings: EntityVec<DeviceNamingId, DeviceNaming>,
    devices: Vec<Device>,
    ints: BTreeMap<String, IntDb>,
    namings: BTreeMap<String, NamingDb>,
    gtz: GtzDb,
}

impl DbBuilder {
    pub fn new() -> Self {
        Self {
            grids: EntityVec::new(),
            bonds: EntityVec::new(),
            interposers: EntityVec::new(),
            dev_namings: EntityVec::new(),
            devices: Vec::new(),
            ints: BTreeMap::new(),
            namings: BTreeMap::new(),
            gtz: GtzDb::default(),
        }
    }

    pub fn insert_grid(&mut self, grid: Chip) -> ChipId {
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

    pub fn insert_interposer(&mut self, interposer: Interposer) -> InterposerId {
        for (k, v) in self.interposers.iter() {
            if v == &interposer {
                return k;
            }
        }
        self.interposers.push(interposer)
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
        let interposer = self.insert_interposer(pre.interposer);
        self.devices.push(Device {
            name: pre.name,
            chips: grids,
            interposer,
            bonds,
            speeds: pre.speeds,
            combos: pre.combos,
            disabled: pre.disabled,
            naming,
        });
        for (_, name, gtz) in pre.gtz.gtz {
            if let Some(ogtz) = self.gtz.gtz.get(&name) {
                assert_eq!(ogtz.1, &gtz);
            } else {
                self.gtz.gtz.insert(name, gtz);
            }
        }
        self.ingest_int(pre.intdb_name, pre.intdb, pre.ndb);
    }

    fn ingest_int(&mut self, name: String, int: IntDb, naming: NamingDb) {
        match self.ints.entry(name.clone()) {
            btree_map::Entry::Vacant(x) => {
                x.insert(int);
            }
            btree_map::Entry::Occupied(mut tgt) => {
                let tgt = tgt.get_mut();
                assert_eq!(tgt.wires, int.wires);
                for (_, k, v) in int.tile_classes {
                    match tgt.tile_classes.get(&k) {
                        None => {
                            tgt.tile_classes.insert(k, v);
                        }
                        Some((_, v2)) => {
                            if v != *v2 {
                                println!("FAIL at {k}");
                            }
                            assert_eq!(&v, v2);
                        }
                    }
                }
                for (_, k, v) in int.conn_classes {
                    match tgt.conn_classes.get(&k) {
                        None => {
                            tgt.conn_classes.insert(k, v);
                        }
                        Some((_, v2)) => {
                            if v != *v2 {
                                println!("FAIL at {k}");
                            }
                            assert_eq!(&v, v2);
                        }
                    }
                }
            }
        }
        match self.namings.entry(name.clone()) {
            btree_map::Entry::Vacant(x) => {
                x.insert(naming);
            }
            btree_map::Entry::Occupied(mut tgt) => {
                let tgt = tgt.get_mut();
                for (_, k, v) in naming.tile_class_namings {
                    match tgt.tile_class_namings.get_mut(&k) {
                        None => {
                            tgt.tile_class_namings.insert(k, v);
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
                                    Some(vv2 @ IntfWireOutNaming::Buf { name_out, .. }) => match vv
                                    {
                                        IntfWireOutNaming::Buf { .. } => {
                                            assert_eq!(&vv, vv2)
                                        }
                                        IntfWireOutNaming::Simple { name } => {
                                            assert_eq!(name_out, &name)
                                        }
                                    },
                                    Some(vv2 @ IntfWireOutNaming::Simple { name }) => {
                                        if let IntfWireOutNaming::Buf { name_out, .. } = &vv {
                                            assert_eq!(name_out, name);
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
                for (_, k, v) in naming.conn_class_namings {
                    match tgt.conn_class_namings.get(&k) {
                        None => {
                            tgt.conn_class_namings.insert(k, v);
                        }
                        Some((_, v2)) => {
                            if v != *v2 {
                                println!("FAIL at {k}");
                            }
                            assert_eq!(&v, v2);
                        }
                    }
                }
            }
        }
    }

    pub fn finish(self) -> GeomDb {
        GeomDb {
            chips: self.grids,
            bonds: self.bonds,
            interposers: self.interposers,
            dev_namings: self.dev_namings,
            devices: self.devices,
            ints: self.ints,
            namings: self.namings,
            gtz: self.gtz,
        }
    }
}
