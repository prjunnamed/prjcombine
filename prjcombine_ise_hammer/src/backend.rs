use bitvec::vec::BitVec;
use prjcombine_hammer::{Backend, FuzzerId};
use prjcombine_int::db::{BelId, WireId};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, LayerId, RowId};
use prjcombine_toolchain::Toolchain;
use prjcombine_virtex_bitstream::parse;
use prjcombine_virtex_bitstream::{BitPos, BitTile, Bitstream, BitstreamGeom};
use prjcombine_xdl::{run_bitgen, Design, Instance, Net, NetPin, NetPip, NetType, Pcf, Placement};
use prjcombine_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use rand::prelude::*;
use std::collections::{hash_map, HashMap};
use std::fmt::{Debug, Write};

use crate::diff::Diff;
use crate::fgen::Loc;

pub struct IseBackend<'a> {
    pub tc: &'a Toolchain,
    pub db: &'a GeomDb,
    pub device: &'a Device,
    pub bs_geom: &'a BitstreamGeom,
    pub egrid: &'a ExpandedGrid<'a>,
    pub edev: &'a ExpandedDevice<'a>,
}

impl<'a> std::fmt::Debug for IseBackend<'a> {
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
    GlobalMutex(String),
    RowMutex(String, RowId),
    BelMutex((DieId, ColId, RowId, LayerId, BelId), String),
    NodeMutex((DieId, (ColId, RowId), WireId)),
    TileMutex(Loc, String),
    IntMutex(DieId, ColId, RowId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PinFromKind {
    Iob,
    Bufg,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value {
    None,
    Bool(bool),
    String(String),
    PinFrom(PinFromKind),
    Bel(DieId, ColId, RowId, LayerId, BelId),
}

impl From<Option<core::convert::Infallible>> for Value {
    fn from(_: Option<core::convert::Infallible>) -> Self {
        Self::None
    }
}

impl<'a> From<&'a str> for Value {
    fn from(value: &'a str) -> Self {
        Self::String(value.into())
    }
}

impl<'a> From<&'a String> for Value {
    fn from(value: &'a String) -> Self {
        Self::String(value.clone())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<PinFromKind> for Value {
    fn from(value: PinFromKind) -> Self {
        Self::PinFrom(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MultiValue {
    Lut,
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

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct FeatureId {
    pub tile: String,
    pub bel: String,
    pub attr: String,
    pub val: String,
}

impl Debug for FeatureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}:{}", self.tile, self.bel, self.attr, self.val)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FeatureBit {
    pub tile: usize,
    pub frame: usize,
    pub bit: usize,
}

impl core::fmt::Debug for FeatureBit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.tile, self.frame, self.bit)
    }
}

#[derive(Clone, Debug)]
pub struct SimpleFeatureData {
    pub diffs: Vec<Diff>,
    pub fuzzers: Vec<FuzzerId>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum PostProc {}

#[derive(Debug)]
pub struct State {
    pub simple_features: HashMap<FeatureId, SimpleFeatureData>,
}

impl State {
    pub fn get_diffs(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Vec<Diff> {
        let tile = tile.into();
        let bel = bel.into();
        let attr = attr.into();
        let val = val.into();
        let id = FeatureId {
            tile,
            bel,
            attr,
            val,
        };
        self.simple_features
            .remove(&id)
            .unwrap_or_else(|| {
                panic!(
                    "NO DIFF: {tile} {bel} {attr} {val}",
                    tile = id.tile,
                    bel = id.bel,
                    attr = id.attr,
                    val = id.val
                )
            })
            .diffs
    }

    pub fn get_diff(
        &mut self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> Diff {
        let mut res = self.get_diffs(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        res.pop().unwrap()
    }

    pub fn peek_diffs(
        &self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> &Vec<Diff> {
        let tile = tile.into();
        let bel = bel.into();
        let attr = attr.into();
        let val = val.into();
        let id = FeatureId {
            tile,
            bel,
            attr,
            val,
        };
        &self
            .simple_features
            .get(&id)
            .unwrap_or_else(|| {
                panic!(
                    "NO DIFF: {tile} {bel} {attr} {val}",
                    tile = id.tile,
                    bel = id.bel,
                    attr = id.attr,
                    val = id.val
                )
            })
            .diffs
    }

    pub fn peek_diff(
        &self,
        tile: impl Into<String>,
        bel: impl Into<String>,
        attr: impl Into<String>,
        val: impl Into<String>,
    ) -> &Diff {
        let res = self.peek_diffs(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        &res[0]
    }
}

impl<'a> Backend for IseBackend<'a> {
    type Key = Key<'a>;
    type Value = Value;
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
        match *pp {
            // XXX
        }
    }

    fn make_state(&self) -> State {
        State {
            simple_features: HashMap::new(),
        }
    }

    fn bitgen(&self, kv: &HashMap<Key, Value>) -> Bitstream {
        let mut gopts = HashMap::new();
        let mut insts: HashMap<String, Instance> = HashMap::new();
        let mut nets = HashMap::new();
        let orig_kv = kv;
        let mut kv = kv.clone();
        let mut single_pips = vec![];
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

        insts.insert(
            "DUMMY_SINGLE_PIPS".to_string(),
            Instance {
                name: "DUMMY_SINGLE_PIPS".to_string(),
                kind: match self.edev {
                    ExpandedDevice::Xc4k(_) => todo!(),
                    ExpandedDevice::Xc5200(_) => todo!(),
                    ExpandedDevice::Virtex(_) => "SLICE",
                    ExpandedDevice::Virtex2(edev) => {
                        if edev.grid.kind.is_virtex2() {
                            "SLICE"
                        } else {
                            "SLICEL"
                        }
                    }
                    ExpandedDevice::Spartan6(_) => "SLICEX",
                    ExpandedDevice::Virtex4(_) => "SLICEL",
                    ExpandedDevice::Ultrascale(_) => unreachable!(),
                    ExpandedDevice::Versal(_) => unreachable!(),
                }
                .to_string(),
                placement: Placement::Unplaced,
                cfg: vec![],
            },
        );

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
                        insts.insert(
                            site.to_string(),
                            Instance {
                                name: site.to_string(),
                                kind: s.to_string(),
                                placement: Placement::Placed {
                                    site: site.to_string(),
                                    tile: "meow".to_string(),
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
                        pin: "CLK".to_string(),
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
                            inst.cfg
                                .push(vec![attr.to_string(), "".to_string(), s.to_string()]);
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
                        match kv.get(&Key::SitePinFrom(site, pin.clone())) {
                            None => (),
                            Some(Value::None) => (),
                            Some(Value::PinFrom(kind)) => {
                                let inst_name = format!("FAKEINST__{site}__{pin}");
                                let (fi_kind, fi_pin, fi_cfg) = match kind {
                                    PinFromKind::Iob => match self.edev {
                                        ExpandedDevice::Xc4k(_) => todo!(),
                                        ExpandedDevice::Xc5200(_) => todo!(),
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
                                        ExpandedDevice::Spartan6(_) => todo!(),
                                        ExpandedDevice::Virtex4(_) => {
                                            ("IOB", "I", vec![("INBUFUSED", "0")])
                                        }
                                        ExpandedDevice::Ultrascale(_) => todo!(),
                                        ExpandedDevice::Versal(_) => todo!(),
                                    },
                                    PinFromKind::Bufg => match self.edev {
                                        ExpandedDevice::Xc4k(_) => todo!(),
                                        ExpandedDevice::Xc5200(_) => todo!(),
                                        ExpandedDevice::Virtex(_) => todo!(),
                                        ExpandedDevice::Virtex2(_) => ("BUFGMUX", "O", vec![]),
                                        ExpandedDevice::Spartan6(_) => todo!(),
                                        ExpandedDevice::Virtex4(_) => ("BUFGCTRL", "O", vec![]),
                                        ExpandedDevice::Ultrascale(_) => todo!(),
                                        ExpandedDevice::Versal(_) => todo!(),
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
                    _ => unreachable!(),
                },
                _ => (),
            }
        }
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
        let mut xdl = Design {
            name: "meow".to_string(),
            part: format!(
                "{d}{s}{p}",
                d = self.device.name,
                s = self.device.speeds[combo.speed_idx],
                p = self.device.bonds[combo.devbond_idx].name
            ),
            cfg: vec![],
            version: "v3.2".to_string(),
            instances: insts.into_values().collect(),
            nets: nets.into_values().collect(),
        };
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
        let pcf = Pcf { vccaux };
        let bitdata = run_bitgen(self.tc, &xdl, &gopts, &pcf, altvr).unwrap();
        parse(self.bs_geom, &bitdata)
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
                                FeatureBit {
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
            if feat.tiles.is_empty() {
                for diff in &xdiffs {
                    if !diff.bits.is_empty() {
                        eprintln!("null fuzzer {f:?} with bits: {xdiffs:?}");
                        return Some(vec![]);
                    }
                }
            } else {
                match state.simple_features.entry(feat.id.clone()) {
                    hash_map::Entry::Occupied(mut e) => {
                        let v = e.get_mut();
                        if v.diffs != xdiffs {
                            eprintln!(
                                "bits mismatch for {f:?}: {vbits:?} vs {xdiffs:?}",
                                vbits = v.diffs
                            );
                            return Some(v.fuzzers.clone());
                        } else {
                            v.fuzzers.push(fid);
                        }
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(SimpleFeatureData {
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

    fn assemble_multi(mv: &MultiValue, y: &BitVec) -> Value {
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
