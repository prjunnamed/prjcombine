use bitvec::vec::BitVec;
use prjcombine_hammer::{Backend, FuzzerId};
use prjcombine_int::db::IntDb;
use prjcombine_int::grid::{ExpandedGrid, RowId};
use prjcombine_toolchain::Toolchain;
use prjcombine_virtex_bitstream::parse;
use prjcombine_virtex_bitstream::{BitPos, BitTile, Bitstream, BitstreamGeom};
use prjcombine_xdl::{run_bitgen, Design, Instance, Net, NetPin, NetPip, NetType, Placement};
use prjcombine_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use std::borrow::Cow;
use std::collections::{hash_map, HashMap};
use std::fmt::Write;

use crate::diff::Diff;

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
    SiteMode(&'a str),
    GlobalOpt(&'a str),
    SiteAttr(&'a str, &'a str),
    SitePin(&'a str, &'a str),
    Pip(&'a str, &'a str, &'a str),
    GlobalMutex(&'a str),
    RowMutex(&'a str, RowId),
    SiteMutex(&'a str, &'a str),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value<'a> {
    None,
    Bool(bool),
    String(Cow<'a, str>),
}

impl<'a> From<Option<core::convert::Infallible>> for Value<'a> {
    fn from(_: Option<core::convert::Infallible>) -> Self {
        Self::None
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(value.into())
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MultiValue {
    Lut,
    Hex(i32),
}

#[derive(Clone, Debug)]
pub enum FuzzerInfo<'a> {
    Simple(Vec<BitTile>, SimpleFeatureId<'a>),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SimpleFeatureId<'a> {
    pub tile: &'a str,
    pub bel: &'a str,
    pub attr: &'a str,
    pub val: &'a str,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FeatureBit {
    pub tile: usize,
    pub frame: usize,
    pub bit: usize,
}

#[derive(Clone, Debug)]
pub struct SimpleFeatureData {
    pub diffs: Vec<Diff>,
    pub fuzzers: Vec<FuzzerId>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum PostProc {}

#[derive(Debug)]
pub struct State<'a> {
    pub simple_features: HashMap<SimpleFeatureId<'a>, SimpleFeatureData>,
}

impl<'a> State<'a> {
    pub fn get_diffs<'b: 'a>(
        &mut self,
        tile: &'b str,
        bel: &'b str,
        attr: &'b str,
        val: &'b str,
    ) -> Vec<Diff> {
        self.simple_features
            .remove(&SimpleFeatureId {
                tile,
                bel,
                attr,
                val,
            })
            .unwrap()
            .diffs
    }

    pub fn get_diff(&mut self, tile: &'a str, bel: &'a str, attr: &'a str, val: &'a str) -> Diff {
        let mut res = self.get_diffs(tile, bel, attr, val);
        assert_eq!(res.len(), 1);
        res.pop().unwrap()
    }
}

fn xlat_diff(bits: &HashMap<BitPos, bool>, tiles: &[BitTile]) -> Option<Diff> {
    let mut res = HashMap::new();
    'bits: for (&k, &v) in bits {
        for (i, t) in tiles.iter().enumerate() {
            if let Some(xk) = t.xlat_pos(k) {
                res.insert(
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
        eprintln!("cannot xlat {k:?} in {tiles:?}");
        return None;
    }
    Some(Diff { bits: res })
}

impl<'a> IseBackend<'a> {
    pub fn intdb(&self) -> &'a IntDb {
        let kind = match self.edev {
            ExpandedDevice::Xc4k(_) => todo!(),
            ExpandedDevice::Xc5200(_) => todo!(),
            ExpandedDevice::Virtex(_) => "virtex",
            ExpandedDevice::Virtex2(edev) => {
                if edev.grid.kind.is_virtex2() {
                    "virtex2"
                } else {
                    "spartan3"
                }
            }
            ExpandedDevice::Spartan6(_) => "spartan6",
            ExpandedDevice::Virtex4(edev) => match edev.kind {
                prjcombine_virtex4::grid::GridKind::Virtex4 => "virtex4",
                prjcombine_virtex4::grid::GridKind::Virtex5 => "virtex5",
                prjcombine_virtex4::grid::GridKind::Virtex6 => "virtex6",
                prjcombine_virtex4::grid::GridKind::Virtex7 => "series7",
            },
            ExpandedDevice::Ultrascale(_) => todo!(),
            ExpandedDevice::Versal(_) => todo!(),
        };
        &self.db.ints[kind]
    }
}

impl<'a> Backend for IseBackend<'a> {
    type Key = Key<'a>;
    type Value = Value<'a>;
    type MultiValue = MultiValue;
    type Bitstream = Bitstream;
    type FuzzerInfo = FuzzerInfo<'a>;
    type PostProc = PostProc;
    type BitPos = BitPos;
    type State = State<'a>;

    fn postproc(
        &self,
        _state: &State<'a>,
        _bs: &mut Bitstream,
        pp: &PostProc,
        _kv: &HashMap<Key, Value>,
    ) -> bool {
        match *pp {
            // XXX
        }
    }

    fn make_state(&self) -> State<'a> {
        State {
            simple_features: HashMap::new(),
        }
    }

    fn bitgen(&self, kv: &HashMap<Key<'a>, Value<'a>>) -> Bitstream {
        let mut gopts = HashMap::new();
        let mut insts = HashMap::new();
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
                        let key = Key::SiteAttr(site, attr);
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
            match *k {
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
                    Value::None => (),
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
            match *k {
                Key::SiteAttr(site, attr) => match v {
                    Value::None => (),
                    Value::String(s) => {
                        if !s.is_empty() {
                            let inst = insts.get_mut(site).unwrap();
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
                        nets.insert(
                            name.clone(),
                            Net {
                                name,
                                typ: NetType::Plain,
                                inpins: vec![NetPin {
                                    inst_name: site.to_string(),
                                    pin: pin.to_string(),
                                }],
                                outpins: vec![],
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
        let combo = &self.device.combos[0];
        let xdl = Design {
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
        let bitdata = run_bitgen(self.tc, &xdl, &gopts).unwrap();
        parse(self.bs_geom, &bitdata)
    }

    fn return_fuzzer(
        &self,
        state: &mut State<'a>,
        f: &FuzzerInfo<'a>,
        fid: FuzzerId,
        bits: Vec<HashMap<BitPos, bool>>,
    ) -> Option<Vec<FuzzerId>> {
        match *f {
            FuzzerInfo::Simple(ref btiles, sfid) => {
                let mut xdiffs = vec![];
                for bbits in &bits {
                    let Some(diff) = xlat_diff(bbits, btiles) else {
                        eprintln!("failed to xlat bits {bits:?} for {f:?}");
                        return Some(vec![]);
                    };
                    xdiffs.push(diff);
                }
                match state.simple_features.entry(sfid) {
                    hash_map::Entry::Occupied(mut e) => {
                        let v = e.get_mut();
                        if v.diffs != xdiffs {
                            eprintln!(
                                "bits mismatch for {f:?}: {vbits:?} vs {xdiffs:?}",
                                vbits = v.diffs
                            );
                            Some(v.fuzzers.clone())
                        } else {
                            v.fuzzers.push(fid);
                            None
                        }
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(SimpleFeatureData {
                            diffs: xdiffs,
                            fuzzers: vec![fid],
                        });
                        None
                    }
                }
            }
        }
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
                Value::String(v.into())
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
                Value::String(v.into())
            }
        }
    }
}
