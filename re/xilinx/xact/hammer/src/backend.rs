use std::{
    collections::{BTreeMap, HashMap, btree_map},
    fs::{File, read_to_string},
    io::Write,
    path::Path,
    process::Command,
};

use prjcombine_interconnect::grid::{BelCoord, ExpandedGrid, NodeLoc, WireCoord};
use prjcombine_re_fpga_hammer::{Diff, FeatureData, FpgaBackend, FuzzerInfo, State};
use prjcombine_re_hammer::{Backend, FuzzerId};
use prjcombine_re_xilinx_xact_geom::Device;
use prjcombine_re_xilinx_xact_naming::grid::{ExpandedGridNaming, PipCoords};
use prjcombine_types::{bitvec::BitVec, bsdata::TileBit};
use prjcombine_xc2000::expanded::ExpandedDevice;
use prjcombine_xilinx_bitstream::{BitPos, BitTile, Bitstream, BitstreamGeom, KeyData, parse};

use crate::lca::{Block, Design, Net};

pub struct XactBackend<'a> {
    pub debug: u8,
    pub xact_path: &'a Path,
    pub device: &'a Device,
    pub bs_geom: &'a BitstreamGeom,
    pub egrid: &'a ExpandedGrid<'a>,
    pub ngrid: &'a ExpandedGridNaming<'a>,
    pub edev: &'a ExpandedDevice<'a>,
}

impl std::fmt::Debug for XactBackend<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XactBackend")
            .field("device", &self.device)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Key<'a> {
    BlockBase(&'a str),
    BlockConfig(&'a str, String, String),
    BlockEquate(&'a str, String),
    BlockPin(&'a str, String),
    Pip(PipCoords),
    GlobalOpt(String),
    BelMutex(BelCoord, String),
    NodeMutex(WireCoord),
    GlobalMutex(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value<'a> {
    None,
    Bool(bool),
    String(String),
    FromPin(&'a str, String),
    IntWire(WireCoord),
    Lut(&'static [&'static str], BitVec),
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

impl From<bool> for Value<'_> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<WireCoord> for Value<'_> {
    fn from(value: WireCoord) -> Self {
        Self::IntWire(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum MultiValue {
    Lut(&'static [&'static str]),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum PostProc {}

impl<'a> Backend for XactBackend<'a> {
    type Key = Key<'a>;
    type Value = Value<'a>;
    type MultiValue = MultiValue;
    type Bitstream = Bitstream;
    type FuzzerInfo = FuzzerInfo<BitTile>;
    type PostProc = PostProc;
    type BitPos = BitPos;
    type State = State;

    fn make_state(&self) -> State {
        State::default()
    }

    fn assemble_multi(v: &MultiValue, b: &BitVec) -> Value<'a> {
        match v {
            MultiValue::Lut(inps) => Value::Lut(inps, b.clone()),
        }
    }

    fn bitgen(&self, kv: &HashMap<Key, Value>) -> Bitstream {
        let mut blocks = BTreeMap::new();
        let mut nets_pin = BTreeMap::new();
        let mut mbo = BTreeMap::new();
        let mut net_ctr = 0;
        for (k, v) in kv {
            match *k {
                Key::BlockBase(loc) => match v {
                    Value::None => (),
                    Value::String(val) => {
                        blocks
                            .entry(loc.to_string())
                            .or_insert_with(|| Block {
                                loc: loc.into(),
                                base: None,
                                cfg: BTreeMap::new(),
                                equate: vec![],
                            })
                            .base = Some(val.clone());
                    }
                    _ => unreachable!(),
                },
                Key::BlockConfig(loc, ref attr, ref val) => match v {
                    Value::None | Value::Bool(false) => (),
                    Value::Bool(true) => {
                        blocks
                            .entry(loc.to_string())
                            .or_insert_with(|| Block {
                                loc: loc.into(),
                                base: None,
                                cfg: BTreeMap::new(),
                                equate: vec![],
                            })
                            .cfg
                            .entry(attr.clone())
                            .or_default()
                            .push(val.clone());
                    }
                    _ => unreachable!(),
                },
                Key::BlockEquate(loc, ref attr) => match v {
                    Value::None => (),
                    Value::Lut(inps, bits) => {
                        let mut terms = vec![];
                        assert_eq!(bits.len(), 1 << inps.len());
                        for (idx, bval) in bits.iter().enumerate() {
                            if bval {
                                let mut term = vec![];
                                for (j, &inp) in inps.iter().enumerate() {
                                    if (idx & 1 << j) != 0 {
                                        term.push(inp.to_string());
                                    } else {
                                        term.push(format!("~{inp}"));
                                    }
                                }
                                let term = term.join("*");
                                terms.push(format!("({term})"));
                            }
                        }
                        let val = if terms.is_empty() {
                            "0".to_string()
                        } else {
                            terms.join("+")
                        };
                        blocks
                            .entry(loc.to_string())
                            .or_insert_with(|| Block {
                                loc: loc.into(),
                                base: None,
                                cfg: BTreeMap::new(),
                                equate: vec![],
                            })
                            .equate
                            .push((attr.clone(), val));
                    }
                    _ => unreachable!(),
                },
                Key::BlockPin(loc, ref pin) => match *v {
                    Value::None => (),
                    Value::Bool(false) => (),
                    Value::Bool(true) => {
                        nets_pin.entry((loc, pin.clone())).or_insert_with(|| {
                            net_ctr += 1;
                            Net {
                                name: format!("net{net_ctr}"),
                                pins: vec![(loc.into(), pin.clone())],
                                pips: vec![],
                            }
                        });
                    }
                    Value::FromPin(sloc, ref spin) => {
                        nets_pin
                            .entry((sloc, spin.clone()))
                            .or_insert_with(|| {
                                net_ctr += 1;
                                Net {
                                    name: format!("net{net_ctr}"),
                                    pins: vec![(sloc.into(), spin.clone())],
                                    pips: vec![],
                                }
                            })
                            .pins
                            .push((loc.into(), pin.clone()));
                    }
                    _ => unreachable!(),
                },
                Key::Pip(crd) => match *v {
                    Value::None => (),
                    Value::FromPin(loc, ref pin) => {
                        nets_pin
                            .entry((loc, pin.clone()))
                            .or_insert_with(|| {
                                net_ctr += 1;
                                Net {
                                    name: format!("net{net_ctr}"),
                                    pins: vec![(loc.into(), pin.clone())],
                                    pips: vec![],
                                }
                            })
                            .pips
                            .push(crd);
                    }
                    _ => unreachable!(),
                },
                Key::GlobalOpt(ref opt) => match v {
                    Value::None => (),
                    Value::String(val) => {
                        mbo.insert(opt.clone(), val.clone());
                    }
                    _ => unreachable!(),
                },
                _ => (),
            }
        }
        let nets = Vec::from_iter(nets_pin.into_values());
        let blocks = blocks.into_values().collect();
        let speed = match &self.device.name[..] {
            "xc2064" | "xc2018" => "-33",
            "xc2064l" | "xc2018l" => "-10",
            "xc3020" | "xc3030" | "xc3042" | "xc3064" | "xc3090" => "-50",
            "xc3020a" | "xc3030a" | "xc3042a" | "xc3064a" | "xc3090a" => "-7",
            "xc3020l" | "xc3030l" | "xc3042l" | "xc3064l" | "xc3090l" => "-8",
            _ => "-5",
        };
        let design = Design {
            part: self.device.name.clone(),
            package: self.device.bonds[0].name.clone(),
            speed: speed.into(),
            nets,
            blocks,
        };
        let dir = tempfile::Builder::new()
            .prefix("xact_makebits")
            .tempdir()
            .unwrap();
        let mut lca_file = File::create(dir.path().join("MEOW.LCA")).unwrap();
        design.write(&mut lca_file).unwrap();
        std::mem::drop(lca_file);
        let mut mbo_file = File::create(dir.path().join("MEOW.MBO")).unwrap();
        // sigh. these need to be first.
        for k in ["STARTUPCLK", "SYNCTODONE"] {
            if let Some(v) = mbo.remove(k) {
                writeln!(mbo_file, "Configure {k} {v}").unwrap();
            }
        }
        for (k, v) in mbo {
            writeln!(mbo_file, "Configure {k} {v}").unwrap();
        }
        std::mem::drop(mbo_file);
        let mut cmd = Command::new("dosbox");
        cmd.env("SDL_VIDEODRIVER", "dummy")
            .arg("-c")
            .arg(format!("mount c {}", self.xact_path.to_string_lossy()))
            .arg("-c")
            .arg(format!("mount d {}", dir.path().to_string_lossy()))
            .arg("-c")
            .arg(r"c:\xact\makebits -b -mbo=d:\meow.mbo d:\meow.lca > d:\meow.log")
            .arg("-c")
            .arg("exit");
        let status = cmd.output().unwrap();
        if !status.status.success() {
            println!("temp dir at {}", dir.path().to_string_lossy());
            std::mem::forget(dir);
            panic!("non-zero dosbox exit status");
        }
        let rbt = match read_to_string(dir.path().join("MEOW.RBT")) {
            Ok(rbt) => rbt,
            Err(_) => {
                let log_file = std::fs::read(dir.path().join("MEOW.LOG")).unwrap();
                let log_file = String::from_utf8_lossy(&log_file);
                print!("{log_file}");
                println!("temp dir at {}", dir.path().to_string_lossy());
                std::mem::forget(dir);
                panic!("FAILED TO GET BITS FILE");
            }
        };
        let mut data = vec![];
        let mut bitpos = 7;
        let mut byte: u8 = 0;
        let mut got_bits = false;
        for line in rbt.lines() {
            if line.starts_with(|c: char| c.is_alphabetic()) {
                assert!(!got_bits);
                continue;
            }
            got_bits = true;
            for c in line.trim().chars() {
                let bit = match c {
                    '0' => 0,
                    '1' => 1,
                    _ => panic!("weird char {c:?} in bitstream"),
                };
                byte |= bit << bitpos;
                if bitpos == 0 {
                    data.push(byte);
                    bitpos = 7;
                    byte = 0;
                } else {
                    bitpos -= 1;
                }
            }
        }
        assert_eq!(bitpos, 7);
        parse(self.bs_geom, &data, &KeyData::None)
    }

    fn diff(bs1: &Bitstream, bs2: &Bitstream) -> HashMap<BitPos, bool> {
        Bitstream::diff(bs1, bs2)
    }

    fn return_fuzzer(
        &self,
        state: &mut State,
        f: &Self::FuzzerInfo,
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
                    btree_map::Entry::Occupied(mut e) => {
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
                    btree_map::Entry::Vacant(e) => {
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

    fn postproc(
        &self,
        _state: &State,
        _bs: &mut Bitstream,
        pp: &PostProc,
        _kv: &HashMap<Key, Value>,
    ) -> bool {
        match *pp {}
    }
}

impl FpgaBackend for XactBackend<'_> {
    type BitTile = BitTile;

    fn node_bits(&self, nloc: NodeLoc) -> Vec<BitTile> {
        self.edev.tile_bits(nloc)
    }

    fn egrid(&self) -> &ExpandedGrid<'_> {
        self.egrid
    }
}
