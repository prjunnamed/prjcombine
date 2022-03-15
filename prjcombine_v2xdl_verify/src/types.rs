use rand::rngs::SmallRng;

use rand::{SeedableRng, Rng};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BitVal {
    S0,
    S1,
}

#[derive(Clone, Debug)]
pub enum ParamVal {
    String(String),
    Int(i32),
    Float(f64),
    Bits(Vec<BitVal>),
}

#[derive(Clone, Debug)]
pub struct SrcInst {
    pub typ: String,
    pub name: String,
    pub attrs: Vec<(String, ParamVal)>,
    pub params: Vec<(String, ParamVal)>,
    pub pins: Vec<(String, Vec<String>)>,
}

impl SrcInst {
    pub fn new(ctx: &mut TestGenCtx, typ: &str) -> Self {
        Self {
            typ: typ.to_string(),
            name: format!("inst_{}", ctx.get_ctr()),
            attrs: Vec::new(),
            params: Vec::new(),
            pins: Vec::new(),
        }
    }
    pub fn connect(&mut self, name: &str, wire: &str) {
        self.pins.push((name.to_string(), vec![wire.to_string()]));
    }
    pub fn connect_bus(&mut self, name: &str, wires: &[String]) {
        self.pins.push((name.to_string(), wires.iter().map(|x| x.to_string()).collect()));
    }
    pub fn attr(&mut self, name: &str, val: ParamVal) {
        self.attrs.push((name.to_string(), val));
    }
    pub fn attr_str(&mut self, name: &str, val: &str) {
        self.attr(name, ParamVal::String(val.to_string()));
    }
    pub fn param(&mut self, name: &str, val: ParamVal) {
        self.params.push((name.to_string(), val));
    }
    pub fn param_bits(&mut self, name: &str, val: &[BitVal]) {
        self.param(name, ParamVal::Bits(val.to_vec()));
    }
    pub fn param_str(&mut self, name: &str, val: &str) {
        self.param(name, ParamVal::String(val.to_string()));
    }
    pub fn param_int(&mut self, name: &str, val: i32) {
        self.param(name, ParamVal::Int(val));
    }
}

#[derive(Clone, Debug)]
pub enum TgtConfigVal {
    Plain(String),
    Rom(u8, u64),
    Ram(u8, u64),
    Lut(u8, u64),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TgtPinDir {
    Input,
    Output,
}

#[derive(Clone, Debug)]
pub struct TgtInst {
    pub kind: Vec<String>,
    pub config: Vec<(String, String, TgtConfigVal, Option<String>)>,
    pub pins: Vec<(String, String, TgtPinDir)>,
    pub pin_ties: Vec<(String, bool)>,
    pub pin_dumout: Vec<String>,
}

impl TgtInst {
    pub fn new(kind: &[&str]) -> Self {
        TgtInst {
            kind: kind.iter().map(|x| x.to_string()).collect(),
            config: Vec::new(),
            pins: Vec::new(),
            pin_ties: Vec::new(),
            pin_dumout: Vec::new(),
        }
    }
    pub fn cfg(&mut self, name: &str, val: &str) {
        self.config.push((name.to_string(), "".to_string(), TgtConfigVal::Plain(val.to_string()), None));
    }
    pub fn cfg_int(&mut self, name: &str, val: i32) {
        self.config.push((name.to_string(), "".to_string(), TgtConfigVal::Plain(val.to_string()), None));
    }
    pub fn cond_cfg(&mut self, name: &str, val: &str, kind: &str) {
        self.config.push((name.to_string(), "".to_string(), TgtConfigVal::Plain(val.to_string()), Some(kind.to_string())));
    }
    pub fn bel(&mut self, name: &str, bel: &str, val: &str) {
        self.config.push((name.to_string(), bel.to_string(), TgtConfigVal::Plain(val.to_string()), None));
    }
    pub fn bel_rom(&mut self, name: &str, bel: &str, sz: u8, val: u64) {
        self.config.push((name.to_string(), bel.to_string(), TgtConfigVal::Rom(sz, val), None));
    }
    pub fn bel_ram(&mut self, name: &str, bel: &str, sz: u8, val: u64) {
        self.config.push((name.to_string(), bel.to_string(), TgtConfigVal::Ram(sz, val), None));
    }
    pub fn bel_lut(&mut self, name: &str, bel: &str, sz: u8, val: u64) {
        self.config.push((name.to_string(), bel.to_string(), TgtConfigVal::Lut(sz, val), None));
    }
    pub fn pin_in(&mut self, name: &str, net: &str) {
        self.pins.push((name.to_string(), net.to_string(), TgtPinDir::Input));
    }
    pub fn pin_in_inv(&mut self, name: &str, net: &str, inv: bool) {
        self.pin_in(name, net);
        self.cfg(&format!("{name}INV"), &if inv {format!("{name}_B")} else {format!("{name}")});
    }
    pub fn pin_out(&mut self, name: &str, net: &str) {
        self.pins.push((name.to_string(), net.to_string(), TgtPinDir::Output));
    }
    pub fn pin_tie(&mut self, name: &str, val: bool) {
        self.pin_ties.push((name.to_string(), val));
    }
    pub fn pin_tie_inv(&mut self, name: &str, val: bool, inv: bool) {
        self.pin_tie(name, val);
        self.cfg(&format!("{name}INV"), &if inv {format!("{name}_B")} else {format!("{name}")});
    }
    pub fn pin_dumout(&mut self, name: &str) {
        self.pin_dumout.push(name.to_string());
    }
}

#[derive(Clone, Debug)]
pub struct Test {
    pub name: String,
    pub part: String,
    pub src_wires: Vec<String>,
    pub src_ins: Vec<String>,
    pub src_outs: Vec<String>,
    pub src_insts: Vec<SrcInst>,
    pub tgt_insts: Vec<TgtInst>,
}

impl Test {
    pub fn new(name: &str, part: &str) -> Self {
        Test {
            name: name.to_string(),
            part: part.to_string(),
            src_wires: Vec::new(),
            src_ins: Vec::new(),
            src_outs: Vec::new(),
            src_insts: Vec::new(),
            tgt_insts: Vec::new(),
        }
    }

    pub fn make_wire(&mut self, ctx: &mut TestGenCtx) -> String {
        let res = format!("net_{}", ctx.get_ctr());
        self.src_wires.push(res.clone());
        res
    }

    pub fn make_in(&mut self, ctx: &mut TestGenCtx) -> String {
        let res = format!("net_in_{}", ctx.get_ctr());
        self.src_ins.push(res.clone());
        res
    }

    pub fn make_in_inv(&mut self, ctx: &mut TestGenCtx) -> (String, String, bool) {
        let raw = self.make_in(ctx);
        let inv = ctx.rng.gen();
        if inv {
            let res = self.make_wire(ctx);
            let mut inst = SrcInst::new(ctx, "INV");
            inst.connect("I", &raw);
            inst.connect("O", &res);
            self.src_insts.push(inst);
            (res, raw, true)
        } else {
            (raw.clone(), raw, false)
        }
    }

    pub fn make_out(&mut self, ctx: &mut TestGenCtx) -> String {
        let res = format!("net_out_{}", ctx.get_ctr());
        self.src_outs.push(res.clone());
        res
    }

    pub fn make_bus(&mut self, ctx: &mut TestGenCtx, num: usize) -> Vec<String> {
        (0..num).map(|_| self.make_wire(ctx)).collect()
    }

    pub fn make_ins(&mut self, ctx: &mut TestGenCtx, num: usize) -> Vec<String> {
        (0..num).map(|_| self.make_in(ctx)).collect()
    }

    pub fn make_outs(&mut self, ctx: &mut TestGenCtx, num: usize) -> Vec<String> {
        (0..num).map(|_| self.make_out(ctx)).collect()
    }
}

pub struct TestGenCtx {
    ctr: u32,
    pub rng: SmallRng,
}

impl TestGenCtx {
    pub fn new() -> Self {
        Self {
            ctr: 0,
            rng: SmallRng::from_entropy(),
        }
    }
    pub fn get_ctr(&mut self) -> u32 {
        self.ctr += 1;
        self.ctr
    }
    pub fn gen_bits(&mut self, num: usize) -> Vec<BitVal> {
        (0..num).map(|_| if self.rng.gen() { BitVal::S1 } else { BitVal::S0 }).collect()
    }
    pub fn gen_name(&mut self) -> String {
        format!("uniq_{}", self.get_ctr())
    }
}
