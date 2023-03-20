use bitvec::vec::BitVec;
use prjcombine_hammer::{Backend, FuzzerId};
use prjcombine_int::grid::ExpandedGrid;
use prjcombine_toolchain::Toolchain;
use prjcombine_virtex_bitstream::parse;
use prjcombine_virtex_bitstream::{BitPos, BitTile, Bitstream, BitstreamGeom};
use prjcombine_xdl::{run_bitgen, Design, Instance, Net, NetPin, NetType, Placement};
use prjcombine_xilinx_geom::{Device, ExpandedDevice, GeomDb};
use std::borrow::Cow;
use std::collections::{hash_map, HashMap};
use std::fmt::Write;

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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct FeatureBit {
    pub tile: usize,
    pub frame: usize,
    pub bit: usize,
}

#[derive(Clone, Debug)]
pub struct SimpleFeatureData {
    pub bits: Vec<HashMap<FeatureBit, bool>>,
    pub fuzzers: Vec<FuzzerId>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum PostProc {}

#[derive(Debug)]
pub struct State<'a> {
    simple_features: HashMap<SimpleFeatureId<'a>, SimpleFeatureData>,
}

fn xlat_bits(bits: &HashMap<BitPos, bool>, tiles: &[BitTile]) -> Option<HashMap<FeatureBit, bool>> {
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
    Some(res)
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
        for (k, v) in kv {
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
                            site,
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
                _ => (),
            }
        }
        for (k, v) in kv {
            match *k {
                Key::SiteAttr(site, attr) => match v {
                    Value::None => (),
                    Value::String(s) => {
                        let inst = insts.get_mut(site).unwrap();
                        inst.cfg
                            .push(vec![attr.to_string(), "".to_string(), s.to_string()]);
                    }
                    _ => unreachable!(),
                },
                Key::SitePin(site, pin) => match v {
                    Value::None | Value::Bool(false) => (),
                    Value::Bool(true) => {
                        let name = format!("pin__{site}__{pin}");
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
                let mut xbits = vec![];
                for bbits in &bits {
                    let Some(bbits) = xlat_bits(bbits, btiles) else {
                        eprintln!("failed to xlat bits {bits:?} for {f:?}");
                        return Some(vec![]);
                    };
                    xbits.push(bbits);
                }
                match state.simple_features.entry(sfid) {
                    hash_map::Entry::Occupied(mut e) => {
                        let v = e.get_mut();
                        if v.bits != xbits {
                            eprintln!(
                                "bits mismatch for {f:?}: {vbits:?} vs {xbits:?}",
                                vbits = v.bits
                            );
                            Some(v.fuzzers.clone())
                        } else {
                            v.fuzzers.push(fid);
                            None
                        }
                    }
                    hash_map::Entry::Vacant(e) => {
                        e.insert(SimpleFeatureData {
                            bits: xbits,
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
        }
    }
}
