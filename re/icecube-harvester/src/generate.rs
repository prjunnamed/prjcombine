use std::collections::{BTreeMap, HashMap, HashSet};

use bitvec::prelude::*;
use prjcombine_interconnect::{
    db::PinDir,
    dir::{Dir, DirH},
    grid::{ColId, EdgeIoCoord, RowId, TileIobId},
};
use prjcombine_siliconblue::{
    bond::{Bond, BondPin},
    chip::{ChipKind, ExtraNodeIo, ExtraNodeLoc},
    expanded::ExpandedDevice,
};
use rand::prelude::*;
use unnamed_entity::EntityId;

use crate::{
    parts::Part,
    prims::Primitive,
    run::{Design, InstId, InstPin, InstPinSource, Instance, RawLoc},
    sites::SiteInfo,
};

pub struct GeneratorConfig<'a> {
    pub part: &'a Part,
    pub edev: &'a ExpandedDevice<'a>,
    pub bonds: &'a BTreeMap<&'static str, Bond>,
    pub plb_info: &'a [SiteInfo],
    pub bel_info: &'a BTreeMap<&'static str, Vec<SiteInfo>>,
    pub pkg_bel_info: &'a BTreeMap<(&'static str, &'static str), Vec<SiteInfo>>,
    pub allow_global: bool,
    pub rows_colbuf: Vec<(RowId, RowId, RowId)>,
    pub prims: &'a BTreeMap<&'static str, Primitive>,
    pub extra_node_locs: &'a BTreeMap<ExtraNodeLoc, Vec<RawLoc>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum LeftVcc {
    _1P5,
    _1P8,
    _2P5,
    _3P3,
}

struct Generator<'a> {
    rng: ThreadRng,
    cfg: &'a GeneratorConfig<'a>,
    design: Design,
    signals: Vec<(InstId, InstPin)>,
    unused_signals: HashSet<(InstId, InstPin)>,
    unused_io: Vec<EdgeIoCoord>,
    io_cs_used: HashSet<(ColId, RowId)>,
    io_map: HashMap<EdgeIoCoord, &'a str>,
    io_latch_ok: HashSet<Dir>,
    gb_net: [Option<(InstId, InstPin)>; 8],
    g2l_mask: u8,
    have_fixed_bram: bool,
    left_vcc: LeftVcc,
    vpp_2v5_to_1p8v: bool,
}

impl Generator<'_> {
    fn add_out_raw(&mut self, iid: InstId, pin: InstPin) {
        let sig = (iid, pin);
        self.signals.push(sig.clone());
        self.unused_signals.insert(sig);
    }

    fn add_out(&mut self, iid: InstId, pin: &'static str) {
        self.add_out_raw(iid, InstPin::Simple(pin.into()));
    }

    fn add_out_indexed(&mut self, iid: InstId, pin: &'static str, index: usize) {
        self.add_out_raw(iid, InstPin::Indexed(pin.into(), index));
    }

    fn get_inps(&mut self, num: usize) -> Vec<(InstId, InstPin)> {
        let res = Vec::from_iter(self.signals.choose_multiple(&mut self.rng, num).cloned());
        for sig in &res {
            self.unused_signals.remove(sig);
        }
        res
    }

    fn get_maybe_global(&mut self, is_plb: bool, mask: u8) -> (InstId, InstPin) {
        let idx = self.rng.random_range(0..8);
        let globals_ready = !self.design.kind.has_colbuf() || !self.cfg.rows_colbuf.is_empty();
        if ((mask >> idx) & 1) == 0
            || !self.cfg.allow_global
            || (!is_plb && !globals_ready)
            || self.rng.random_bool(0.5)
            || self.gb_net[idx].is_none()
        {
            return self.get_inps(1).pop().unwrap();
        }
        if globals_ready {
            self.gb_net[idx].clone().unwrap()
        } else {
            self.gb_net[idx].take().unwrap()
        }
    }

    fn get_unused_sig(&mut self) -> (InstId, InstPin) {
        let sig = self.unused_signals.iter().next().unwrap().clone();
        self.unused_signals.remove(&sig);
        sig
    }

    fn emit_dummy_lut(&mut self) {
        let mut inst = Instance::new("SB_LUT4");
        inst.prop("LUT_INIT", "16'h0000");
        let iid = self.design.insts.push(inst);
        self.add_out(iid, "O");
    }

    fn emit_gb(&mut self, index: usize) {
        let mut inst = Instance::new("SB_GB");
        let (src_inst, src_pin) = self.signals.choose(&mut self.rng).unwrap().clone();
        inst.connect("USER_SIGNAL_TO_GLOBAL_BUFFER", src_inst, src_pin);
        let iid = self.design.insts.push(inst);
        self.gb_net[index] = Some((iid, InstPin::Simple("GLOBAL_BUFFER_OUTPUT".into())));
    }

    fn emit_io(&mut self) -> usize {
        let crd = self.unused_io.pop().unwrap();
        let is_od = self.cfg.edev.chip.io_od.contains(&crd);
        let mut global_idx = None;
        for (&loc, node) in &self.cfg.edev.chip.extra_nodes {
            if let ExtraNodeLoc::GbIo(idx) = loc {
                if node.io[&ExtraNodeIo::GbIn] == crd {
                    global_idx = Some(idx);
                }
            }
        }
        if !self.cfg.allow_global {
            global_idx = None;
        }
        if let Some(idx) = global_idx {
            if self.gb_net[idx].is_some() {
                global_idx = None;
            }
        }
        if self.rng.random() {
            global_idx = None;
        }
        let is_i3c = self
            .cfg
            .edev
            .chip
            .extra_nodes
            .contains_key(&ExtraNodeLoc::IoI3c(crd))
            && global_idx.is_none()
            && self.cfg.allow_global
            && self.rng.random();
        let mut lvds = self.cfg.allow_global
            && self.rng.random()
            && !is_od
            && !is_i3c
            && self.cfg.edev.chip.io_has_lvds(crd);
        if crd.edge() == Dir::W
            && self.cfg.part.kind.has_vref()
            && !matches!(self.left_vcc, LeftVcc::_1P8 | LeftVcc::_2P5)
        {
            lvds = false;
        }
        if lvds {
            let other = crd.with_iob(TileIobId::from_idx(crd.iob().to_idx() ^ 1));
            let other_idx = self.unused_io.iter().position(|x| *x == other);
            if let Some(other_idx) = other_idx {
                self.unused_io.swap_remove(other_idx);
            } else {
                lvds = false;
            }
        }
        let pad = self.io_map[&crd];
        let package_pin = if is_od { "PACKAGEPIN" } else { "PACKAGE_PIN" };
        let mut io = Instance::new(if global_idx.is_some() {
            "SB_GB_IO"
        } else if is_i3c {
            "SB_IO_I3C"
        } else if is_od {
            "SB_IO_OD"
        } else {
            "SB_IO"
        });
        io.io
            .insert(InstPin::Simple(package_pin.into()), pad.to_string());
        io.top_port(package_pin);

        if !is_od {
            if lvds {
                io.prop("PULLUP", "1'b0");
            } else if is_i3c {
                io.prop(
                    "WEAK_PULLUP",
                    if self.rng.random() { "1'b1" } else { "1'b0" },
                );
                if self.rng.random() {
                    io.prop("PULLUP", "1'b1");
                    io.prop(
                        "PULLUP_RESISTOR",
                        ["3P3K", "6P8K", "10K"].choose(&mut self.rng).unwrap(),
                    );
                } else {
                    io.prop("PULLUP", "1'b0");
                }
            } else if self.cfg.allow_global {
                if matches!(self.cfg.part.kind, ChipKind::Ice40T01 | ChipKind::Ice40T05)
                    && self.rng.random()
                    && global_idx.is_none()
                {
                    io.prop("PULLUP", "1'b1");
                    io.prop(
                        "PULLUP_RESISTOR",
                        ["3P3K", "6P8K", "10K", "100K"]
                            .choose(&mut self.rng)
                            .unwrap(),
                    );
                } else {
                    io.prop("PULLUP", ["1'b0", "1'b1"].choose(&mut self.rng).unwrap());
                }
            } else {
                io.prop("PULLUP", "1'b1");
            }
        }
        let mut pin_type = bitvec![0; 6];
        for i in 0..6 {
            if self.rng.random_bool(0.5) {
                pin_type.set(i, true);
            }
        }
        if lvds {
            let iostd = if crd.edge() == Dir::W
                && self.cfg.part.kind.has_vref()
                && self.left_vcc == LeftVcc::_1P8
            {
                "SB_SUBLVDS_INPUT"
            } else {
                "SB_LVDS_INPUT"
            };
            io.prop("IO_STANDARD", iostd);
        } else if crd.edge() == Dir::W && self.cfg.part.kind.has_vref() {
            let iostds = match self.left_vcc {
                LeftVcc::_1P5 => ["SB_LVCMOS15_4", "SB_LVCMOS15_2"].as_slice(),
                LeftVcc::_1P8 => [
                    "SB_LVCMOS18_10",
                    "SB_LVCMOS18_8",
                    "SB_LVCMOS18_4",
                    "SB_LVCMOS18_2",
                    "SB_SSTL18_FULL",
                    "SB_SSTL18_HALF",
                    "SB_MDDR10",
                    "SB_MDDR8",
                    "SB_MDDR4",
                    "SB_MDDR2",
                ]
                .as_slice(),
                LeftVcc::_2P5 => [
                    "SB_LVCMOS25_16",
                    "SB_LVCMOS25_12",
                    "SB_LVCMOS25_8",
                    "SB_LVCMOS25_4",
                    "SB_SSTL2_CLASS_2",
                    "SB_SSTL2_CLASS_1",
                ]
                .as_slice(),
                LeftVcc::_3P3 => ["SB_LVCMOS33_8"].as_slice(),
            };
            let iostd = iostds.choose(&mut self.rng).unwrap();
            io.prop("IO_STANDARD", iostd);
        } else if !is_od {
            io.prop("IO_STANDARD", "SB_LVCMOS");
        }

        let inps = self.get_inps(6);

        let num_inps = self.rng.random_range(0..=3);
        let in_pins = if is_od {
            ["DOUT0", "DOUT1", "OUTPUTENABLE"]
        } else {
            ["D_OUT_0", "D_OUT_1", "OUTPUT_ENABLE"]
        };
        for &pin in in_pins.choose_multiple(&mut self.rng, num_inps) {
            let (sinst, spin) = inps.choose(&mut self.rng).unwrap().clone();
            io.connect(pin, sinst, spin);
            if pin.ends_with("ENABLE") {
                pin_type.set(5, true);
            }
        }
        if is_i3c {
            for pin in ["PU_ENB", "WEAK_PU_ENB"] {
                if self.rng.random() {
                    let (sinst, spin) = self.get_inps(1).pop().unwrap();
                    io.connect(pin, sinst, spin);
                }
            }
        }
        let (_, (col, row), _) = self.cfg.edev.chip.get_io_loc(crd);
        if self.rng.random_bool(0.5) && !self.io_cs_used.contains(&(col, row)) {
            self.io_cs_used.insert((col, row));
            let shared_in_pins = if is_od {
                ["INPUTCLK", "OUTPUTCLK", "CLOCKENABLE"]
            } else {
                ["INPUT_CLK", "OUTPUT_CLK", "CLOCK_ENABLE"]
            };
            if self.rng.random() {
                for pin in shared_in_pins {
                    if self.rng.random_bool(0.5) {
                        let (sinst, spin) = inps.choose(&mut self.rng).unwrap().clone();
                        io.connect(pin, sinst, spin);
                    }
                }
            } else {
                for pin in shared_in_pins {
                    let mask = if pin.ends_with("CLK") {
                        0xff
                    } else if matches!(self.cfg.part.kind, ChipKind::Ice65L04 | ChipKind::Ice65P04)
                    {
                        0x55
                    } else {
                        0xaa
                    };
                    let (sinst, spin) = self.get_maybe_global(false, mask);
                    io.connect(pin, sinst, spin);
                }
            }
            if !pin_type[0] || (pin_type[4] && pin_type[5]) || pin_type[2] || !pin_type[3] {
                io.prop_bin(
                    "NEG_TRIGGER",
                    &BitVec::from_iter([self.rng.random_bool(0.5)]),
                );
            }
        } else {
            pin_type.set(0, true);
            pin_type.set(2, false);
            pin_type.set(3, true);
            if pin_type[4] && pin_type[5] {
                pin_type.set(4, false);
            }
        }
        if !pin_type[4] && !pin_type[5] {
            pin_type.set(4, true);
        }
        if lvds {
            pin_type.set(1, false);
        }

        if pin_type[1] && self.rng.random_bool(0.5) && self.io_latch_ok.contains(&crd.edge()) {
            self.io_latch_ok.remove(&crd.edge());
            let (sinst, spin) = inps.choose(&mut self.rng).unwrap().clone();
            io.connect(
                if is_od {
                    "LATCHINPUTVALUE"
                } else {
                    "LATCH_INPUT_VALUE"
                },
                sinst,
                spin,
            );
        }

        io.prop_bin("PIN_TYPE", &pin_type);

        let iid = self.design.insts.push(io);
        if self.cfg.allow_global {
            if self.rng.random_bool(0.5) {
                self.add_out(iid, if is_od { "DIN0" } else { "D_IN_0" });
            }
            if self.rng.random_bool(0.5) && !pin_type[0] {
                self.add_out(iid, if is_od { "DIN1" } else { "D_IN_1" });
            }
        }
        if let Some(idx) = global_idx {
            self.gb_net[idx] = Some((iid, InstPin::Simple("GLOBAL_BUFFER_OUTPUT".into())));
        }
        if lvds { 2 } else { 1 }
    }

    fn emit_lut(&mut self) {
        let mut inst = Instance::new("SB_LUT4");
        let mut num_inps = *[1, 2, 3, 4].choose(&mut self.rng).unwrap();
        let has_ff = self.rng.random_bool(0.3);
        if has_ff {
            num_inps = 1;
        }
        let inps = if num_inps == 4 {
            let lut_init: u16 = self.rng.random();
            inst.prop("LUT_INIT", &format!("16'h{lut_init:04x}"));
            self.get_inps(num_inps)
        } else {
            // we can't figure out the swizzle unless we have 4 different inputs, so just 0 it.
            inst.prop("LUT_INIT", "16'h0000");
            Vec::from_iter((0..num_inps).map(|_| self.get_maybe_global(false, self.g2l_mask)))
        };
        for (i, (src_site, src_pin)) in inps.into_iter().enumerate() {
            inst.connect(["I0", "I1", "I2", "I3"][i], src_site, src_pin);
        }
        let iid = self.design.insts.push(inst);
        if has_ff {
            let mut kind = "SB_DFF".to_string();
            let has_en = self.rng.random_bool(0.5);
            let has_sr = self.rng.random_bool(0.5);
            let sr_sync = self.rng.random_bool(0.5);
            let sr_val = self.rng.random_bool(0.5);
            if self.rng.random_bool(0.5) {
                kind.push('N');
            }
            if has_en {
                kind.push('E');
            }
            if has_sr {
                if sr_sync {
                    kind.push('S');
                }
                if sr_val {
                    kind.push('S');
                } else {
                    kind.push('R');
                }
            }
            let mut ff = Instance::new(&kind);
            ff.connect("D", iid, InstPin::Simple("O".into()));
            if self.rng.random_bool(0.8) {
                let (ssite, spin) = self.get_maybe_global(true, 0xff);
                ff.connect("C", ssite, spin);
            }
            if has_en && self.rng.random_bool(0.8) {
                let (ssite, spin) = self.get_maybe_global(true, 0xaa);
                ff.connect("E", ssite, spin);
            }
            if has_sr && self.rng.random_bool(0.8) {
                let (ssite, spin) = self.get_maybe_global(true, 0x55);
                ff.connect(if sr_val { "S" } else { "R" }, ssite, spin);
            }
            let fid = self.design.insts.push(ff);
            self.add_out(fid, "Q");
        } else {
            self.add_out(iid, "O");
        }
    }

    fn emit_carry(&mut self) {
        let num = self.rng.random_range(2..16);
        let mut chain = None;
        for idx in 0..num {
            let mut do_lut = self.rng.random_bool(0.1);
            if idx == 0 {
                do_lut = false;
            }
            if idx == num - 1 {
                do_lut = true;
            }
            let inps = if do_lut {
                let inps = self.get_inps(3);
                let mut lut = Instance::new("SB_LUT4");
                lut.loc = Some(RawLoc {
                    x: 2,
                    y: 3 + idx / 8,
                    bel: idx % 8,
                });
                let mut lut_init: u16 = self.rng.random();
                while lut_init == 0 {
                    lut_init = self.rng.random();
                }
                lut.prop("LUT_INIT", &format!("16'h{lut_init:04x}"));
                for (i, &(src_site, ref src_pin)) in inps.iter().enumerate() {
                    lut.connect(["I0", "I1", "I2", "I3"][i], src_site, src_pin.clone());
                }
                let (src_site, src_pin) = chain.clone().unwrap();
                lut.connect("I3", src_site, src_pin);
                let iid = self.design.insts.push(lut);
                self.add_out(iid, "O");
                inps[1..].to_vec()
            } else {
                self.get_inps(2)
            };
            if idx != num - 1 {
                let mut carry = Instance::new("SB_CARRY");
                carry.loc = Some(RawLoc {
                    x: 2,
                    y: 3 + idx / 8,
                    bel: idx % 8,
                });
                let (src_site, src_pin) = inps[0].clone();
                carry.connect("I0", src_site, src_pin);
                let (src_site, src_pin) = inps[1].clone();
                carry.connect("I1", src_site, src_pin);
                if idx == 0 {
                    carry.pins.insert(
                        InstPin::Simple("CI".into()),
                        if self.rng.random() {
                            InstPinSource::Gnd
                        } else {
                            InstPinSource::Vcc
                        },
                    );
                } else {
                    let (src_site, src_pin) = chain.clone().unwrap();
                    carry.connect("CI", src_site, src_pin);
                }
                let iid = self.design.insts.push(carry);
                chain = Some((iid, InstPin::Simple("CO".into())));
            }
        }
    }

    fn emit_bram(&mut self) {
        let (kind, addr_bits) = if self.cfg.part.kind.is_ice65() {
            ("SB_RAM4K", 8)
        } else {
            ("SB_RAM40_4K", 11)
        };
        let mut kind = kind.to_string();
        let nr = self.rng.random_bool(0.5);
        let nw = self.rng.random_bool(0.5);
        if nr {
            kind.push_str("NR");
        }
        if nw {
            kind.push_str("NW");
        }
        let mut inst = Instance::new(&kind);
        for i in 0..16 {
            let mut val = "256'h".to_string();
            for _ in 0..64 {
                val.push_str(&format!("{:x}", self.rng.random_range(0..16)));
            }
            inst.prop(&format!("INIT_{i:X}"), &val);
        }
        let mut write_mode = 0;
        if self.cfg.part.kind.is_ice40() && self.rng.random_bool(0.2) {
            write_mode = self.rng.random_range(0..4);
            let read_mode = self.rng.random_range(0..4);
            inst.prop("READ_MODE", &read_mode.to_string());
            inst.prop("WRITE_MODE", &write_mode.to_string());
        }
        let mut last = None;
        let mut limit = if self.rng.random_bool(0.5) {
            self.rng.random_range(1..4)
        } else {
            self.rng.random_range(0..28)
        };
        let mut pins = vec![];
        let ouroboros = write_mode == 0 && self.rng.random_bool(0.2);
        for (pin, num) in [
            ("WDATA", 16),
            ("MASK", 16),
            ("WADDR", addr_bits),
            ("RADDR", addr_bits),
        ] {
            if num == 16 && write_mode != 0 {
                continue;
            }
            if num == 16 && ouroboros {
                continue;
            }
            for i in 0..num {
                pins.push((InstPin::Indexed(pin.into(), i), self.g2l_mask));
            }
        }
        for (pin, mask) in [
            (if nr { "RCLKN" } else { "RCLK" }, 0xff),
            ("RCLKE", 0xaa),
            ("RE", 0x55),
            (if nw { "WCLKN" } else { "WCLK" }, 0xff),
            ("WCLKE", 0xaa),
            ("WE", 0x55),
        ] {
            pins.push((InstPin::Simple(pin.into()), mask));
        }
        pins.shuffle(&mut self.rng);
        for (pin, mask) in pins {
            if self.rng.random_bool(0.5) && limit != 0 {
                let mut kill_last = false;
                let (src_site, src_pin) = if self.rng.random_bool(0.5) && last.is_some() {
                    last.clone().unwrap()
                } else {
                    if self.rng.random() {
                        kill_last = true;
                        self.get_maybe_global(false, mask)
                    } else {
                        self.get_inps(1).pop().unwrap()
                    }
                };
                match pin {
                    InstPin::Simple(pin) => inst.connect(&pin, src_site, src_pin.clone()),
                    InstPin::Indexed(pin, idx) => {
                        inst.connect_idx(&pin, idx, src_site, src_pin.clone())
                    }
                }
                limit -= 1;
                if kill_last {
                    last = None;
                } else {
                    last = Some((src_site, src_pin));
                }
            }
        }
        let iid = self.design.insts.push(inst);
        for i in 0..16 {
            self.add_out_indexed(iid, "RDATA", i);
        }
        if ouroboros {
            for pin in ["WDATA", "MASK"] {
                for idx in 0..16 {
                    if self.rng.random_bool(0.5) {
                        self.design.insts[iid].connect_idx(
                            pin,
                            idx,
                            iid,
                            InstPin::Indexed("RDATA".into(), self.rng.random_range(0..16)),
                        );
                    }
                }
            }
        }
    }

    fn emit_bram_pair(&mut self) {
        let (kind, addr_bits) = if self.cfg.part.kind.is_ice65() {
            ("SB_RAM4K", 8)
        } else {
            ("SB_RAM40_4K", 11)
        };
        let mut kind = kind.to_string();
        let nr = self.rng.random_bool(0.5);
        let nw = self.rng.random_bool(0.5);
        if nr {
            kind.push_str("NR");
        }
        if nw {
            kind.push_str("NW");
        }
        let mut inst_a = Instance::new(&kind);
        let mut inst_b = Instance::new(&kind);
        if !self.have_fixed_bram {
            let col = *self.cfg.edev.chip.cols_bram.iter().next().unwrap();
            let x = col.to_idx() as u32;
            inst_a.loc = Some(RawLoc { x, y: 1, bel: 0 });
            inst_b.loc = Some(RawLoc { x, y: 3, bel: 0 });
            self.have_fixed_bram = true;
        }
        for (pin, num) in [("WADDR", addr_bits), ("RADDR", addr_bits)] {
            let same = self.rng.random_bool(0.5);
            for idx in 0..num {
                if same {
                    let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                    inst_a.connect_idx(pin, idx, src_site, src_pin.clone());
                    inst_b.connect_idx(pin, idx, src_site, src_pin);
                } else {
                    let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                    inst_a.connect_idx(pin, idx, src_site, src_pin);
                    let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                    inst_b.connect_idx(pin, idx, src_site, src_pin);
                }
            }
        }
        for pin in [
            if nr { "RCLKN" } else { "RCLK" },
            "RCLKE",
            "RE",
            if nw { "WCLKN" } else { "WCLK" },
            "WCLKE",
            "WE",
        ] {
            let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
            inst_a.connect(pin, src_site, src_pin);
            let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
            inst_b.connect(pin, src_site, src_pin);
        }
        let iid = self.design.insts.push(inst_a);
        for i in 0..16 {
            self.add_out_indexed(iid, "RDATA", i);
        }
        let iid = self.design.insts.push(inst_b);
        for i in 0..16 {
            self.add_out_indexed(iid, "RDATA", i);
        }
    }

    fn emit_warmboot(&mut self) {
        let mut inst = Instance::new("SB_WARMBOOT");
        for pin in ["S0", "S1", "BOOT"] {
            if self.rng.random() {
                let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                inst.connect(pin, src_site, src_pin);
            }
        }
        self.design.insts.push(inst);
    }

    fn emit_simple_ip(&mut self, kind: &str) {
        if self.rng.random() {
            return;
        }
        let prim = &self.cfg.prims[kind];
        let mut inst = Instance::new(kind);
        let mut outps = vec![];
        for (&pin, pin_data) in &prim.pins {
            if pin_data.is_pad || pin == "LEDDRST" || (kind == "SB_IR_IP" && pin == "RST") {
                continue;
            }
            if let Some(width) = pin_data.len {
                for idx in 0..width {
                    if pin_data.dir == PinDir::Input {
                        if self.rng.random() {
                            let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                            inst.connect_idx(pin, idx, src_site, src_pin);
                        }
                    } else {
                        outps.push(InstPin::Indexed(pin.into(), idx));
                    }
                }
            } else {
                if pin_data.dir == PinDir::Input {
                    if self.rng.random() {
                        let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                        inst.connect(pin, src_site, src_pin);
                    }
                } else {
                    outps.push(InstPin::Simple(pin.into()));
                }
            }
        }
        let inst = self.design.insts.push(inst);
        let num_outps = self.rng.random_range(1..=outps.len());
        for outp in outps.choose_multiple(&mut self.rng, num_outps) {
            self.add_out_raw(inst, outp.clone());
        }
    }

    fn emit_osc(&mut self, kind: &str) {
        // TODO: remove condition
        if self.rng.random() || self.cfg.edev.chip.kind == ChipKind::Ice40R04 {
            return;
        }
        let prim = &self.cfg.prims[kind];
        let mut inst = Instance::new(kind);
        let mut outp = None;
        for (&pin, pin_data) in &prim.pins {
            if let Some(width) = pin_data.len {
                for idx in 0..width {
                    if pin_data.dir == PinDir::Input {
                        if self.rng.random() {
                            let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                            inst.connect_idx(pin, idx, src_site, src_pin);
                        }
                    } else {
                        unreachable!();
                    }
                }
            } else {
                if pin_data.dir == PinDir::Input {
                    if self.rng.random() {
                        let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                        inst.connect(pin, src_site, src_pin);
                    }
                } else {
                    assert!(outp.is_none());
                    outp = Some(pin);
                }
            }
        }
        let outp = outp.unwrap();
        if kind == "SB_HFOSC" {
            inst.prop_bin_str(
                "CLKHF_DIV",
                &BitVec::from_iter([self.rng.random::<bool>(), self.rng.random()]),
            );
        }
        let global_idx = match kind {
            "SB_HSOSC" | "SB_HFOSC" => 4,
            "SB_LSOSC" | "SB_LFOSC" => 5,
            _ => unreachable!(),
        };
        let route_through_fabric = self.rng.random() && self.gb_net[global_idx].is_none();
        if route_through_fabric {
            inst.prop("ROUTE_THROUGH_FABRIC", "1");
        }
        let inst = self.design.insts.push(inst);
        if route_through_fabric {
            self.add_out(inst, outp);
        } else {
            self.gb_net[global_idx] = Some((inst, InstPin::Simple(outp.into())));
        }
    }

    fn emit_spram(&mut self, side: DirH) {
        let kind = "SB_SPRAM256KA";
        let prim = &self.cfg.prims[kind];
        let mut outps = [vec![], vec![]];
        let mut insts = [
            if self.rng.random() {
                Some(Instance::new(kind))
            } else {
                None
            },
            if self.rng.random() {
                Some(Instance::new(kind))
            } else {
                None
            },
        ];
        if insts[0].is_none() && insts[1].is_none() {
            return;
        }
        for (&pin, pin_data) in &prim.pins {
            if pin_data.is_pad || pin == "LEDDRST" || (kind == "SB_IR_IP" && pin == "RST") {
                continue;
            }
            if matches!(pin, "RDMARGIN" | "RDMARGINEN" | "TEST") {
                if let Some(width) = pin_data.len {
                    for idx in 0..width {
                        if self.rng.random() {
                            let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                            for inst in insts.iter_mut().flatten() {
                                inst.connect_idx(pin, idx, src_site, src_pin.clone());
                            }
                        }
                    }
                } else {
                    if self.rng.random() {
                        let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                        for inst in insts.iter_mut().flatten() {
                            inst.connect(pin, src_site, src_pin.clone());
                        }
                    }
                }
            } else {
                for ii in 0..2 {
                    let Some(ref mut inst) = insts[ii] else {
                        continue;
                    };
                    if let Some(width) = pin_data.len {
                        for idx in 0..width {
                            if pin_data.dir == PinDir::Input {
                                if self.rng.random() {
                                    let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                                    inst.connect_idx(pin, idx, src_site, src_pin);
                                }
                            } else {
                                outps[ii].push(InstPin::Indexed(pin.into(), idx));
                            }
                        }
                    } else {
                        if pin_data.dir == PinDir::Input {
                            if self.rng.random() || pin == "POWEROFF" {
                                let (src_site, src_pin) = self.get_inps(1).pop().unwrap();
                                inst.connect(pin, src_site, src_pin);
                            }
                        } else {
                            outps[ii].push(InstPin::Simple(pin.into()));
                        }
                    }
                }
            }
        }
        for (ii, inst) in insts.into_iter().enumerate() {
            let Some(mut inst) = inst else { continue };
            inst.loc = Some(self.cfg.extra_node_locs[&ExtraNodeLoc::SpramPair(side)][ii]);
            let inst = self.design.insts.push(inst);
            let outps = std::mem::take(&mut outps[ii]);
            let num_outps = self.rng.random_range(1..=outps.len());
            for outp in outps.choose_multiple(&mut self.rng, num_outps) {
                self.add_out_raw(inst, outp.clone());
            }
        }
    }

    fn reduce_sigs(&mut self) {
        while self.unused_signals.len() >= 4 {
            let mut inst = Instance::new("SB_LUT4");
            inst.prop("LUT_INIT", "16'h0000");
            for i in 0..4 {
                let (src_site, src_pin) = self.get_unused_sig();
                inst.connect(["I0", "I1", "I2", "I3"][i], src_site, src_pin);
            }
            let iid = self.design.insts.push(inst);
            self.add_out(iid, "O");
        }
    }

    fn final_output(&mut self) {
        while !self.unused_signals.is_empty() {
            let crd = self.unused_io.pop().unwrap();
            let is_od = self.cfg.edev.chip.io_od.contains(&crd);
            let pad = self.io_map[&crd];
            let package_pin = if is_od { "PACKAGEPIN" } else { "PACKAGE_PIN" };
            let mut io = Instance::new(if is_od { "SB_IO_OD" } else { "SB_IO" });
            if !is_od {
                if crd.edge() == Dir::W && self.cfg.part.kind.has_vref() {
                    let iostd = match self.left_vcc {
                        LeftVcc::_1P5 => "SB_LVCMOS15_4",
                        LeftVcc::_1P8 => "SB_LVCMOS18_10",
                        LeftVcc::_2P5 => "SB_LVCMOS25_16",
                        LeftVcc::_3P3 => "SB_LVCMOS33_8",
                    };
                    io.prop("IO_STANDARD", iostd);
                } else {
                    io.prop("IO_STANDARD", "SB_LVCMOS");
                }
            }
            io.io
                .insert(InstPin::Simple(package_pin.into()), pad.to_string());
            if !is_od {
                io.prop("PULLUP", "1'b1");
            }
            io.top_port(package_pin);
            let (dout0, dout1) = if is_od {
                ("DOUT0", "DOUT1")
            } else {
                ("D_OUT_0", "D_OUT_1")
            };
            let (sinst, spin) = self.get_unused_sig();
            if self.rng.random_bool(0.5) {
                io.connect(dout0, sinst, spin.clone());
            } else if self.rng.random_bool(0.5) {
                io.connect(dout1, sinst, spin.clone());
            } else {
                io.connect(dout0, sinst, spin.clone());
                io.connect(dout1, sinst, spin.clone());
            }
            self.design.insts.push(io);
        }
    }

    fn generate(&mut self) {
        if self.vpp_2v5_to_1p8v {
            self.design
                .props
                .insert("VPP_2V5_TO_1P8V".into(), "1".into());
        }

        for _ in 0..20 {
            self.emit_dummy_lut();
        }
        if self.cfg.allow_global {
            for i in 0..8 {
                if self.rng.random_bool(0.5) {
                    self.emit_gb(i);
                }
            }
        }
        let mut actual_ios = self
            .rng
            .random_range(1..=self.cfg.pkg_bel_info[&(self.design.package, "SB_IO")].len() - 4);
        let mut actual_lcs = self.rng.random_range(4..=self.cfg.plb_info.len());
        let mut actual_brams = 0;
        if self.cfg.part.kind != ChipKind::Ice40P03 {
            let kind = if self.cfg.part.kind.is_ice65() {
                "SB_RAM4K"
            } else {
                "SB_RAM40_4K"
            };
            actual_brams = self.rng.random_range(2..=self.cfg.bel_info[kind].len());
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Thing {
            Io,
            Lut,
            Bram,
            LeddIp,
            LeddaIp,
            IrIp,
            Spram(DirH),
            LsOsc,
            HsOsc,
            LfOsc,
            HfOsc,
        }
        let mut things = vec![];
        for _ in 0..actual_ios {
            things.push(Thing::Io);
        }
        for _ in 0..actual_brams {
            things.push(Thing::Bram);
        }
        for _ in 0..actual_lcs {
            things.push(Thing::Lut);
        }
        for &key in self.cfg.edev.chip.extra_nodes.keys() {
            match key {
                // ExtraNodeLoc::Pll(dir_v) => todo!(),
                // ExtraNodeLoc::Spi(dir_h) => todo!(),
                // ExtraNodeLoc::I2c(dir_h) => todo!(),
                // ExtraNodeLoc::I2cFifo(dir_h) => todo!(),
                ExtraNodeLoc::LsOsc => {
                    things.push(Thing::LsOsc);
                }
                ExtraNodeLoc::HsOsc => {
                    things.push(Thing::HsOsc);
                }
                ExtraNodeLoc::LfOsc => {
                    things.push(Thing::LfOsc);
                }
                ExtraNodeLoc::HfOsc => {
                    things.push(Thing::HfOsc);
                }
                // ExtraNodeLoc::IrDrv => todo!(),
                // ExtraNodeLoc::RgbDrv => todo!(),
                // ExtraNodeLoc::BarcodeDrv => todo!(),
                // ExtraNodeLoc::Ir400Drv => todo!(),
                // ExtraNodeLoc::RgbaDrv => todo!(),
                // ExtraNodeLoc::LedDrvCur => todo!(),
                ExtraNodeLoc::LeddIp => {
                    things.push(Thing::LeddIp);
                }
                ExtraNodeLoc::LeddaIp => {
                    things.push(Thing::LeddaIp);
                }
                ExtraNodeLoc::IrIp => {
                    things.push(Thing::IrIp);
                }
                // ExtraNodeLoc::Mac16(col, row) => todo!(),
                ExtraNodeLoc::SpramPair(side) => {
                    things.push(Thing::Spram(side));
                }
                _ => (),
            }
        }
        things.shuffle(&mut self.rng);

        if self.rng.random_bool(0.5) {
            self.emit_carry();
        }
        for thing in things {
            match thing {
                Thing::Io => {
                    if actual_ios >= 6 {
                        actual_ios -= self.emit_io();
                    }
                }
                Thing::Lut => {
                    if actual_lcs != 0 {
                        self.emit_lut();
                        actual_lcs -= 1;
                    }
                }
                Thing::Bram => {
                    if actual_brams != 0 {
                        if actual_brams >= 2 && self.rng.random_bool(0.1) {
                            self.emit_bram_pair();
                            actual_brams -= 2;
                        } else {
                            self.emit_bram();
                            actual_brams -= 1;
                        }
                    }
                }
                Thing::LsOsc => {
                    self.emit_osc("SB_LSOSC");
                }
                Thing::HsOsc => {
                    self.emit_osc("SB_HSOSC");
                }
                Thing::LfOsc => {
                    self.emit_osc("SB_LFOSC");
                }
                Thing::HfOsc => {
                    self.emit_osc("SB_HFOSC");
                }
                Thing::LeddIp => {
                    self.emit_simple_ip("SB_LEDD_IP");
                }
                Thing::LeddaIp => {
                    self.emit_simple_ip("SB_LEDDA_IP");
                }
                Thing::IrIp => {
                    self.emit_simple_ip("SB_IR_IP");
                }
                Thing::Spram(side) => {
                    self.emit_spram(side);
                }
            }
        }

        if self
            .cfg
            .edev
            .chip
            .extra_nodes
            .contains_key(&ExtraNodeLoc::Warmboot)
            && self.rng.random()
        {
            self.emit_warmboot();
        }

        self.reduce_sigs();
        self.final_output();
    }
}

pub fn generate(cfg: &GeneratorConfig) -> Design {
    let mut rng = rand::rng();
    let mut design = Design {
        kind: cfg.part.kind,
        device: cfg.part.name,
        package: cfg.part.packages.choose(&mut rng).unwrap(),
        speed: cfg.part.speeds.choose(&mut rng).unwrap(),
        temp: cfg.part.temps.choose(&mut rng).unwrap(),
        insts: Default::default(),
        keep_tmp: false,
        opts: vec![],
        props: Default::default(),
    };
    if cfg.part.kind != ChipKind::Ice40T04 {
        design.opts.push(
            ["--frequency low", "--frequency medium", "--frequency high"]
                .choose(&mut rng)
                .unwrap()
                .to_string(),
        );
    }

    let mut unused_io = vec![];
    let mut io_map = HashMap::new();
    let bond = &cfg.bonds[design.package];
    for (pad, &pin) in &bond.pins {
        let (BondPin::Io(crd) | BondPin::IoCDone(crd)) = pin else {
            continue;
        };
        if !cfg.allow_global && cfg.part.kind.has_vref() && crd.edge() == Dir::W {
            continue;
        }
        io_map.insert(crd, pad.as_str());
        unused_io.push(crd);
    }
    unused_io.shuffle(&mut rng);
    let mut io_latch_ok = HashSet::new();
    for dir in Dir::DIRS {
        if rng.random_bool(0.8) {
            io_latch_ok.insert(dir);
        }
    }
    let mut g2l_mask = 0;
    for _ in 0..4 {
        g2l_mask |= 1 << rng.random_range(0..8);
    }
    let left_vcc = *[LeftVcc::_1P5, LeftVcc::_1P8, LeftVcc::_2P5, LeftVcc::_3P3]
        .choose(&mut rng)
        .unwrap();
    let vpp_2v5_to_1p8v = cfg.edev.chip.kind.is_ultra() && rng.random();
    let mut generator = Generator {
        cfg,
        rng,
        design,
        signals: Default::default(),
        unused_signals: Default::default(),
        unused_io,
        io_cs_used: HashSet::new(),
        io_map,
        io_latch_ok,
        gb_net: [const { None }; 8],
        g2l_mask,
        have_fixed_bram: false,
        left_vcc,
        vpp_2v5_to_1p8v,
    };
    generator.generate();
    generator.design
}
