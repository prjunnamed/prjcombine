use bitvec::vec::BitVec;
use prjcombine_collector::{Diff, FeatureData, FeatureId, State};
use prjcombine_hammer::{Backend, FuzzerId};
use prjcombine_int::db::{BelId, WireId};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, LayerId, NodeLoc, RowId};
use prjcombine_toolchain::Toolchain;
use prjcombine_types::tiledb::TileBit;
use prjcombine_virtex_bitstream::{parse, KeyData, KeyDataAes, KeyDataDes, KeySeq};
use prjcombine_virtex_bitstream::{BitPos, BitTile, Bitstream, BitstreamGeom};
use prjcombine_xdl::{run_bitgen, Design, Instance, Net, NetPin, NetPip, NetType, Pcf, Placement};
use prjcombine_xilinx_geom::{
    Bond, Device, ExpandedBond, ExpandedDevice, ExpandedNamedDevice, GeomDb,
};
use prjcombine_xilinx_naming::grid::ExpandedGridNaming;
use rand::prelude::*;
use std::collections::{hash_map, HashMap};
use std::fmt::{Debug, Write};

pub struct IseBackend<'a> {
    pub debug: u8,
    pub tc: &'a Toolchain,
    pub db: &'a GeomDb,
    pub device: &'a Device,
    pub bs_geom: &'a BitstreamGeom,
    pub egrid: &'a ExpandedGrid<'a>,
    pub ngrid: &'a ExpandedGridNaming<'a>,
    pub edev: &'a ExpandedDevice<'a>,
    pub endev: &'a ExpandedNamedDevice<'a>,
    pub ebonds: &'a HashMap<String, ExpandedBond<'a>>,
}

impl std::fmt::Debug for IseBackend<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IseBackend")
            .field("device", &self.device)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Key<'a> {
    Package,
    SiteMode(&'a str),
    GlobalOpt(String),
    SiteAttr(&'a str, String),
    SitePin(&'a str, String),
    SitePinFrom(&'a str, String),
    Pip(&'a str, &'a str, &'a str),
    VccAux,
    AltVr,
    InternalVref(u32),
    DciCascade(u32),
    VccoSenseMode(u32),
    GlobalMutex(String),
    RowMutex(String, RowId),
    BelMutex((DieId, ColId, RowId, LayerId, BelId), String),
    NodeMutex((DieId, (ColId, RowId), WireId)),
    TileMutex(NodeLoc, String),
    IntMutex(DieId, ColId, RowId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PinFromKind {
    Iob,
    Bufg,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value<'a> {
    None,
    Bool(bool),
    String(String),
    U32(u32),
    PinFrom(PinFromKind),
    FromPin(&'a str, String),
    Bel(DieId, ColId, RowId, LayerId, BelId),
}

impl From<Option<core::convert::Infallible>> for Value<'_> {
    fn from(_: Option<core::convert::Infallible>) -> Self {
        Self::None
    }
}

impl<'a> From<&'a str> for Value<'_> {
    fn from(value: &'a str) -> Self {
        Self::String(value.into())
    }
}

impl<'a> From<&'a String> for Value<'_> {
    fn from(value: &'a String) -> Self {
        Self::String(value.clone())
    }
}

impl From<String> for Value<'_> {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<PinFromKind> for Value<'_> {
    fn from(value: PinFromKind) -> Self {
        Self::PinFrom(value)
    }
}

impl From<bool> for Value<'_> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<u32> for Value<'_> {
    fn from(value: u32) -> Self {
        Self::U32(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MultiValue {
    Lut,
    OldLut(char),
    Hex(i32),
    HexPrefix,
    Bin,
    Dec(i32),
}

#[derive(Clone, Debug)]
pub struct FuzzerFeature {
    pub id: FeatureId,
    pub tiles: Vec<BitTile>,
}

#[derive(Clone)]
pub struct FuzzerInfo {
    pub features: Vec<FuzzerFeature>,
}

impl Debug for FuzzerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.features[0].id)
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum PostProc {}

impl IseBackend<'_> {
    fn gen_key(&self, gopts: &mut HashMap<String, String>) -> KeyData {
        let mut rng = thread_rng();
        match self.edev {
            ExpandedDevice::Virtex2(_) => {
                let key_passes = rng.gen_range(1..=6);
                let start_key = rng.gen_range(0..(6 - key_passes + 1));
                let mut key = KeyDataDes {
                    key: rng.gen(),
                    keyseq: core::array::from_fn(|_| {
                        *[KeySeq::First, KeySeq::Middle, KeySeq::Last, KeySeq::Single]
                            .choose(&mut rng)
                            .unwrap()
                    }),
                };
                if key_passes == 1 {
                    key.keyseq[start_key] = KeySeq::Single;
                } else {
                    for i in start_key..(start_key + key_passes) {
                        key.keyseq[i] = KeySeq::Middle;
                    }
                    key.keyseq[start_key] = KeySeq::First;
                    key.keyseq[start_key + key_passes - 1] = KeySeq::Last;
                }
                for i in 0..6 {
                    gopts.insert(format!("KEY{i}"), hex::encode(key.key[i]));
                    gopts.insert(
                        format!("KEYSEQ{i}"),
                        match key.keyseq[i] {
                            KeySeq::First => "F",
                            KeySeq::Middle => "M",
                            KeySeq::Last => "L",
                            KeySeq::Single => "S",
                        }
                        .into(),
                    );
                }
                gopts.insert("STARTKEY".into(), start_key.to_string());
                gopts.insert("KEYPASSES".into(), key_passes.to_string());
                KeyData::Des(key)
            }
            ExpandedDevice::Spartan6(_) | ExpandedDevice::Virtex4(_) => {
                let key = KeyDataAes { key: rng.gen() };
                gopts.insert("KEY0".into(), hex::encode(key.key));
                KeyData::Aes(key)
            }
            _ => unreachable!(),
        }
    }
}

impl<'a> Backend for IseBackend<'a> {
    type Key = Key<'a>;
    type Value = Value<'a>;
    type MultiValue = MultiValue;
    type Bitstream = Bitstream;
    type FuzzerInfo = FuzzerInfo;
    type PostProc = PostProc;
    type BitPos = BitPos;
    type State = State;

    fn postproc(
        &self,
        _state: &State,
        _bs: &mut Bitstream,
        pp: &PostProc,
        _kv: &HashMap<Key<'a>, Value>,
    ) -> bool {
        match *pp {}
    }

    fn make_state(&self) -> State {
        State::default()
    }

    fn bitgen(&self, kv: &HashMap<Key, Value>) -> Bitstream {
        let mut gopts = HashMap::new();
        let mut insts: HashMap<String, Instance> = HashMap::new();
        let mut nets = HashMap::new();
        let orig_kv = kv;
        let mut kv = kv.clone();
        let mut single_pips = vec![];

        let combo = if let Some(Value::String(package)) = kv.get(&Key::Package) {
            'pkg: {
                for combo in &self.device.combos {
                    if self.device.bonds[combo.devbond_idx].name == *package {
                        break 'pkg combo;
                    }
                }
                panic!("pkg {package} not found");
            }
        } else {
            &self.device.combos[0]
        };

        let mut site_to_tile = HashMap::new();
        for nnode in self.ngrid.nodes.values() {
            if let Some(ref name) = nnode.tie_name {
                site_to_tile.insert(name.to_string(), nnode.names[nnode.tie_rt].to_string());
            }
            for (id, name) in &nnode.bels {
                let rt = self.ngrid.db.node_namings[nnode.naming].bels[id].tile;
                site_to_tile.insert(name.to_string(), nnode.names[rt].to_string());
            }
        }
        if let ExpandedNamedDevice::Virtex4(endev) = self.endev {
            for ngt in endev.gtz.values() {
                let Some(ngt) = ngt else { continue };
                site_to_tile.insert(ngt.bel.clone(), ngt.tile.clone());
            }
        }

        let mut site_to_place = HashMap::new();
        let bond = &self.db.bonds[self.device.bonds[combo.devbond_idx].bond];
        if let Bond::Xc2000(bond) = bond {
            let ExpandedNamedDevice::Xc2000(endev) = self.endev else {
                unreachable!()
            };
            for (k, v) in &bond.pins {
                if let prjcombine_xc2000::bond::BondPin::Io(io) = v {
                    let name = endev.get_io_name(*io);
                    site_to_place.insert(name.to_string(), k.to_string());
                }
            }
            for io in endev.grid.get_bonded_ios() {
                let name = endev.get_io_name(io);
                match site_to_place.entry(name.to_string()) {
                    hash_map::Entry::Occupied(_) => (),
                    hash_map::Entry::Vacant(e) => {
                        e.insert(format!("UNB{suf}", suf = name.strip_prefix("PAD").unwrap()));
                    }
                }
            }
        }

        // sigh. bitgen inserts nondeterministic defaults without this.
        for (k, v) in orig_kv {
            if let Key::SiteMode(site) = k {
                let Value::String(s) = v else {
                    continue;
                };
                if s == "BLOCKRAM" {
                    for attr in [
                        "INIT_00", "INIT_01", "INIT_02", "INIT_03", "INIT_04", "INIT_05",
                        "INIT_06", "INIT_07", "INIT_08", "INIT_09", "INIT_0a", "INIT_0b",
                        "INIT_0c", "INIT_0d", "INIT_0e", "INIT_0f",
                    ] {
                        let key = Key::SiteAttr(site, attr.into());
                        let zero =
                            "0000000000000000000000000000000000000000000000000000000000000000";
                        let entry = kv.entry(key).or_insert(zero.into());
                        if matches!(*entry, Value::None) {
                            *entry = zero.into();
                        }
                    }
                }
            }
        }
        let dummy_kind = match self.edev {
            ExpandedDevice::Virtex2(_) => Some("VCC"),
            ExpandedDevice::Spartan6(_) | ExpandedDevice::Virtex4(_) => Some("TIEOFF"),
            _ => None,
        };
        if let Some(dummy_kind) = dummy_kind {
            for nnode in self.ngrid.nodes.values() {
                if let Some(ref name) = nnode.tie_name {
                    insts.insert(
                        "DUMMY_INST".to_string(),
                        Instance {
                            name: "DUMMY_INST".to_string(),
                            kind: dummy_kind.to_string(),
                            placement: Placement::Placed {
                                tile: "DUMMY".to_string(),
                                site: name.to_string(),
                            },
                            cfg: vec![],
                        },
                    );
                    break;
                }
            }
        }

        let (dummy_inst_kind, dummy_inst_port) = match self.edev {
            ExpandedDevice::Xc2000(edev) => {
                if edev.grid.kind.is_xc4000() {
                    ("CLB", "K")
                } else {
                    ("LC5A", "CK")
                }
            }
            ExpandedDevice::Virtex(_) => ("SLICE", "CLK"),
            ExpandedDevice::Virtex2(edev) => (
                if edev.grid.kind.is_virtex2() {
                    "SLICE"
                } else {
                    "SLICEL"
                },
                "CLK",
            ),
            ExpandedDevice::Spartan6(_) => ("SLICEX", "CLK"),
            ExpandedDevice::Virtex4(_) => ("SLICEL", "CLK"),
            _ => unreachable!(),
        };
        insts.insert(
            "DUMMY_SINGLE_PIPS".to_string(),
            Instance {
                name: "DUMMY_SINGLE_PIPS".to_string(),
                kind: dummy_inst_kind.to_string(),
                placement: Placement::Unplaced,
                cfg: vec![],
            },
        );

        let mut pin_pips: HashMap<_, Vec<_>> = HashMap::new();

        for (k, v) in &kv {
            match k {
                Key::GlobalOpt(opt) => match v {
                    Value::None => (),
                    Value::String(s) => {
                        gopts.insert(opt.to_string(), s.to_string());
                    }
                    _ => unreachable!(),
                },
                Key::SiteMode(site) => match v {
                    Value::None => (),
                    Value::String(s) => {
                        if s.is_empty() {
                            continue;
                        }
                        insts.insert(
                            site.to_string(),
                            Instance {
                                name: site.to_string(),
                                kind: s.to_string(),
                                placement: Placement::Placed {
                                    site: match site_to_place.get(*site) {
                                        Some(place) => place.to_string(),
                                        None => site.to_string(),
                                    },
                                    tile: site_to_tile[*site].to_string(),
                                },
                                cfg: vec![],
                            },
                        );
                    }
                    _ => unreachable!(),
                },
                Key::Pip(tile, wa, wb) => match v {
                    Value::None | Value::Bool(false) => (),
                    Value::Bool(true) => single_pips.push(NetPip {
                        tile: tile.to_string(),
                        wire_from: wa.to_string(),
                        wire_to: wb.to_string(),
                        dir: prjcombine_xdl::PipDirection::UniBuf,
                    }),
                    Value::FromPin(site, pin) => {
                        pin_pips
                            .entry((*site, &pin[..]))
                            .or_default()
                            .push((*tile, *wa, *wb));
                    }
                    _ => unreachable!(),
                },
                _ => (),
            }
        }
        if !single_pips.is_empty() {
            nets.insert(
                "SINGLE_PIPS".to_string(),
                Net {
                    name: "SINGLE_PIPS".to_string(),
                    typ: NetType::Plain,
                    inpins: vec![NetPin {
                        inst_name: "DUMMY_SINGLE_PIPS".to_string(),
                        pin: dummy_inst_port.to_string(),
                    }],
                    outpins: vec![],
                    pips: single_pips,
                    cfg: vec![],
                },
            );
        }
        for (k, v) in &kv {
            match k {
                Key::SiteAttr(site, attr) => match v {
                    Value::None => (),
                    Value::String(s) => {
                        if !s.is_empty() {
                            let inst = insts.get_mut(&**site).unwrap();
                            if let Some(suf) = s.strip_prefix("#LUT:") {
                                inst.cfg.push(vec![
                                    attr.to_string(),
                                    "".to_string(),
                                    "#LUT".to_string(),
                                    suf.to_string(),
                                ]);
                            } else if let Some(suf) = s.strip_prefix("#RAM:") {
                                inst.cfg.push(vec![
                                    attr.to_string(),
                                    "".to_string(),
                                    "#RAM".to_string(),
                                    suf.to_string(),
                                ]);
                            } else {
                                inst.cfg.push(vec![
                                    attr.to_string(),
                                    "".to_string(),
                                    s.to_string(),
                                ]);
                            }
                        }
                    }
                    _ => unreachable!(),
                },
                Key::SitePin(site, pin) => match v {
                    Value::None | Value::Bool(false) => (),
                    Value::Bool(true) => {
                        let name = format!("SINGLE_PIN__{site}__{pin}");
                        let mut net = Net {
                            name: name.clone(),
                            typ: NetType::Plain,
                            inpins: vec![NetPin {
                                inst_name: site.to_string(),
                                pin: pin.to_string(),
                            }],
                            outpins: vec![],
                            pips: vec![],
                            cfg: vec![],
                        };
                        if let Some(pips) = pin_pips.get(&(*site, &pin[..])) {
                            for pip in pips {
                                net.pips.push(NetPip {
                                    tile: pip.0.to_string(),
                                    wire_from: pip.1.to_string(),
                                    wire_to: pip.2.to_string(),
                                    dir: prjcombine_xdl::PipDirection::UniBuf,
                                });
                            }
                        }
                        match kv.get(&Key::SitePinFrom(site, pin.clone())) {
                            None => (),
                            Some(Value::None) => (),
                            Some(Value::PinFrom(kind)) => {
                                let inst_name = format!("FAKEINST__{site}__{pin}");
                                let (fi_kind, fi_pin, fi_cfg) = match kind {
                                    PinFromKind::Iob => match self.edev {
                                        ExpandedDevice::Xc2000(_) => todo!(),
                                        ExpandedDevice::Virtex(_) => todo!(),
                                        ExpandedDevice::Virtex2(edev) => {
                                            let mut cfg = vec![("IMUX", "1")];
                                            if edev.grid.kind.is_spartan3a() {
                                                cfg.extend([
                                                    ("IBUF_DELAY_VALUE", "DLY0"),
                                                    ("DELAY_ADJ_ATTRBOX", "FIXED"),
                                                    ("SEL_MUX", "0"),
                                                ]);
                                            }
                                            ("IOB", "I", cfg)
                                        }
                                        ExpandedDevice::Spartan6(_) => {
                                            ("IOB", "I", vec![("IMUX", "I"), ("BYPASS_MUX", "I")])
                                        }
                                        ExpandedDevice::Virtex4(edev) => match edev.kind {
                                            prjcombine_virtex4::grid::GridKind::Virtex4 => {
                                                ("IOB", "I", vec![("INBUFUSED", "0")])
                                            }
                                            prjcombine_virtex4::grid::GridKind::Virtex5 => {
                                                ("IOB", "I", vec![("IMUX", "I")])
                                            }
                                            prjcombine_virtex4::grid::GridKind::Virtex6 => todo!(),
                                            prjcombine_virtex4::grid::GridKind::Virtex7 => todo!(),
                                        },
                                        _ => unreachable!(),
                                    },
                                    PinFromKind::Bufg => match self.edev {
                                        ExpandedDevice::Virtex2(_)
                                        | ExpandedDevice::Spartan6(_) => ("BUFGMUX", "O", vec![]),
                                        ExpandedDevice::Virtex4(_) => ("BUFGCTRL", "O", vec![]),
                                        _ => unreachable!(),
                                    },
                                };
                                insts.insert(
                                    inst_name.clone(),
                                    Instance {
                                        name: inst_name.clone(),
                                        kind: fi_kind.into(),
                                        placement: Placement::Unplaced,
                                        cfg: fi_cfg
                                            .iter()
                                            .map(|(a, b)| {
                                                vec![a.to_string(), "".to_string(), b.to_string()]
                                            })
                                            .collect(),
                                    },
                                );
                                net.outpins.push(NetPin {
                                    inst_name,
                                    pin: fi_pin.into(),
                                });
                            }
                            _ => unreachable!(),
                        }
                        nets.insert(name, net);
                    }
                    Value::FromPin(site_other, pin_other) => {
                        let name = format!("SINGLE_PIN__{site}__{pin}");
                        nets.insert(
                            name.clone(),
                            Net {
                                name,
                                typ: NetType::Plain,
                                inpins: vec![NetPin {
                                    inst_name: site_other.to_string(),
                                    pin: pin_other.to_string(),
                                }],
                                outpins: vec![NetPin {
                                    inst_name: site.to_string(),
                                    pin: pin.to_string(),
                                }],
                                pips: vec![],
                                cfg: vec![],
                            },
                        );
                    }
                    _ => unreachable!(),
                },
                _ => (),
            }
        }
        let part = match self.edev {
            ExpandedDevice::Xc2000(edev) => {
                if edev.grid.kind.is_xc4000() {
                    format!(
                        "{d}{p}{s}",
                        d = &self.device.name[2..],
                        p = self.device.bonds[combo.devbond_idx].name,
                        s = self.device.speeds[combo.speed_idx],
                    )
                } else {
                    format!(
                        "{d}{p}",
                        d = &self.device.name[2..],
                        p = self.device.bonds[combo.devbond_idx].name,
                    )
                }
            }

            _ => format!(
                "{d}{s}{p}",
                d = self.device.name,
                s = self.device.speeds[combo.speed_idx],
                p = self.device.bonds[combo.devbond_idx].name
            ),
        };
        let mut xdl = Design {
            name: "meow".to_string(),
            part,
            cfg: vec![],
            version: "v3.2".to_string(),
            instances: insts.into_values().collect(),
            nets: nets.into_values().collect(),
        };
        if let ExpandedDevice::Xc2000(edev) = self.edev {
            if !edev.grid.kind.is_xc4000() {
                xdl.version = "".to_string();
            }
        }
        xdl.instances.shuffle(&mut rand::thread_rng());
        let vccaux = if let Some(Value::String(val)) = kv.get(&Key::VccAux) {
            if val.is_empty() {
                None
            } else {
                Some(val.to_string())
            }
        } else {
            None
        };
        let altvr = kv.get(&Key::AltVr) == Some(&Value::Bool(true));
        let mut internal_vref = HashMap::new();
        let mut dci_cascade = HashMap::new();
        let mut vccosensemode = HashMap::new();
        for (k, v) in &kv {
            match k {
                Key::DciCascade(bank) => match v {
                    Value::U32(val) => {
                        dci_cascade.insert(*bank, *val);
                    }
                    Value::None => (),
                    _ => unreachable!(),
                },
                Key::InternalVref(bank) => match v {
                    Value::U32(val) => {
                        internal_vref.insert(*bank, *val);
                    }
                    Value::None => (),
                    _ => unreachable!(),
                },
                Key::VccoSenseMode(bank) => match v {
                    Value::String(val) => {
                        vccosensemode.insert(*bank, val.clone());
                    }
                    Value::None => (),
                    _ => unreachable!(),
                },
                _ => (),
            }
        }
        let pcf = Pcf {
            vccaux,
            internal_vref,
            dci_cascade,
            vccosensemode,
        };
        let mut key = KeyData::None;
        if let Some(encrypt) = gopts.get("ENCRYPT") {
            if encrypt == "YES" {
                key = self.gen_key(&mut gopts);
            }
        }
        if self.device.name.contains("7s15") || self.device.name.contains("7s6") {
            // frankenstein ISE breaks non-compressed non-debug bitstreams on those for some reason
            gopts.insert("COMPRESS".to_owned(), "".to_owned());
        }
        let bitdata = run_bitgen(self.tc, &xdl, &gopts, &pcf, altvr).unwrap();
        parse(self.bs_geom, &bitdata, &key)
    }

    fn return_fuzzer(
        &self,
        state: &mut State,
        f: &FuzzerInfo,
        fid: FuzzerId,
        bits: Vec<HashMap<BitPos, bool>>,
    ) -> Option<Vec<FuzzerId>> {
        let mut fdiffs: Vec<_> = f
            .features
            .iter()
            .map(|_| vec![Diff::default(); bits.len()])
            .collect();
        for (bitidx, bbits) in bits.iter().enumerate() {
            'bits: for (&k, &v) in bbits {
                for (fidx, feat) in f.features.iter().enumerate() {
                    for (i, t) in feat.tiles.iter().enumerate() {
                        if let Some(xk) = t.xlat_pos_rev(k) {
                            fdiffs[fidx][bitidx].bits.insert(
                                TileBit {
                                    tile: i,
                                    frame: xk.0,
                                    bit: xk.1,
                                },
                                v,
                            );
                            continue 'bits;
                        }
                    }
                }
                eprintln!("failed to xlat bit {k:?} [bits {bbits:?}] for {f:?}, candidates:");
                for feat in &f.features {
                    println!("{:?}: {:?}", feat.id, feat.tiles);
                }
                return Some(vec![]);
            }
        }
        for (feat, xdiffs) in f.features.iter().zip(fdiffs) {
            if self.debug >= 3 {
                eprintln!("RETURN {feat:?} {xdiffs:?}");
            }
            if feat.tiles.is_empty() {
                for diff in &xdiffs {
                    if !diff.bits.is_empty() {
                        eprintln!("null fuzzer {f:?} with bits: {xdiffs:?}");
                        return Some(vec![]);
                    }
                }
            } else {
                match state.features.entry(feat.id.clone()) {
                    hash_map::Entry::Occupied(mut e) => {
                        let v = e.get_mut();
                        if v.diffs != xdiffs {
                            eprintln!(
                                "bits mismatch for {f:?}/{fid:?}: {vbits:?} vs {xdiffs:?}",
                                fid = feat.id,
                                vbits = v.diffs
                            );
                            return Some(v.fuzzers.clone());
                        } else {
                            v.fuzzers.push(fid);
                        }
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(FeatureData {
                            diffs: xdiffs,
                            fuzzers: vec![fid],
                        });
                    }
                }
            }
        }
        None
    }

    fn diff(bs1: &Bitstream, bs2: &Bitstream) -> HashMap<BitPos, bool> {
        Bitstream::diff(bs1, bs2)
    }

    fn assemble_multi(mv: &MultiValue, y: &BitVec) -> Value<'a> {
        match *mv {
            MultiValue::Lut => {
                let mut v = "#LUT:0x".to_string();
                let nc = y.len() / 4;
                for i in 0..nc {
                    let mut c = 0;
                    for j in 0..4 {
                        if y[(nc - 1 - i) * 4 + j] {
                            c |= 1 << j;
                        }
                    }
                    write!(v, "{c:x}").unwrap();
                }
                Value::String(v)
            }
            MultiValue::OldLut(f) => {
                let mut v = format!("#LUT:{f}=0x");
                let nc = y.len() / 4;
                for i in 0..nc {
                    let mut c = 0;
                    for j in 0..4 {
                        if y[(nc - 1 - i) * 4 + j] {
                            c |= 1 << j;
                        }
                    }
                    write!(v, "{c:x}").unwrap();
                }
                Value::String(v)
            }
            MultiValue::Hex(delta) => {
                let mut y = y.clone();
                if delta != 0 {
                    y.push(false);
                }
                if delta > 0 {
                    for _ in 0..delta {
                        for mut bit in &mut y {
                            bit.set(!*bit);
                            if *bit {
                                break;
                            }
                        }
                    }
                }
                if delta < 0 {
                    for _ in 0..-delta {
                        for mut bit in &mut y {
                            bit.set(!*bit);
                            if !*bit {
                                break;
                            }
                        }
                    }
                }
                let mut v = String::new();
                let nc = y.len().div_ceil(4);
                for i in 0..nc {
                    let mut c = 0;
                    for j in 0..4 {
                        let bitidx = (nc - 1 - i) * 4 + j;
                        if bitidx < y.len() && y[bitidx] {
                            c |= 1 << j;
                        }
                    }
                    write!(v, "{c:x}").unwrap();
                }
                Value::String(v)
            }
            MultiValue::HexPrefix => {
                let mut v = "0x".to_string();
                let nc = y.len().div_ceil(4);
                for i in 0..nc {
                    let mut c = 0;
                    for j in 0..4 {
                        let bitidx = (nc - 1 - i) * 4 + j;
                        if bitidx < y.len() && y[bitidx] {
                            c |= 1 << j;
                        }
                    }
                    write!(v, "{c:x}").unwrap();
                }
                Value::String(v)
            }
            MultiValue::Bin => {
                let mut v = String::new();
                for bit in y.iter().rev() {
                    write!(v, "{}", if *bit { "1" } else { "0" }).unwrap();
                }
                Value::String(v)
            }
            MultiValue::Dec(delta) => {
                let mut val: u64 = 0;
                assert!(y.len() <= 64);
                for (i, v) in y.iter().enumerate() {
                    if *v {
                        val |= 1 << i;
                    }
                }
                val = val.checked_add_signed(delta.into()).unwrap();
                Value::String(format!("{}", val))
            }
        }
    }
}
