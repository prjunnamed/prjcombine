use std::collections::{BTreeMap, BTreeSet, btree_map};

use prjcombine_re_sdf::{Cell, Edge, IoPath};
use prjcombine_siliconblue::chip::ChipKind;
use prjcombine_types::{
    speed::{
        DelayRfBinate, DelayRfUnate, DerateFactorTemperatureLinear,
        DerateFactorVoltageInvQuadratic, RecRem, ResistanceRf, SetupHoldRf, Speed, SpeedVal,
    },
    units::{Resistance, Scalar, Time, Voltage},
};
use unnamed_entity::EntityId;

use crate::run::{Design, InstId, Instance, RunResult};

#[derive(Debug, Default)]
pub struct SpeedCollector {
    pub db: Speed,
    pub wanted_keys: BTreeSet<String>,
}

impl SpeedCollector {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, key: impl Into<String>, val: SpeedVal) -> bool {
        match self.db.vals.entry(key.into()) {
            btree_map::Entry::Vacant(entry) => {
                entry.insert(val);
                true
            }
            btree_map::Entry::Occupied(entry) => {
                assert_eq!(*entry.get(), val, "mismatch for {key}", key = entry.key());
                false
            }
        }
    }

    pub fn merge(&mut self, other: &Speed) -> bool {
        let mut changed = false;
        for (k, &v) in &other.vals {
            changed |= self.insert(k, v);
        }
        changed
    }

    pub fn want(&mut self, key: impl Into<String>) {
        self.wanted_keys.insert(key.into());
    }
}

const ZERO: prjcombine_re_sdf::Delay = prjcombine_re_sdf::Delay {
    min: Time::ZERO,
    typ: Time::ZERO,
    max: Time::ZERO,
};

fn convert_delay(del: prjcombine_re_sdf::Delay) -> Time {
    assert_eq!(del.min, del.typ);
    assert_eq!(del.min, del.max);
    del.min
}

fn convert_delay_rf_unate(iopath: &IoPath) -> DelayRfUnate {
    DelayRfUnate {
        rise: convert_delay(iopath.del_rise),
        fall: convert_delay(iopath.del_fall),
    }
}

fn convert_delay_rf_binate(iopath: &IoPath) -> DelayRfBinate {
    DelayRfBinate {
        rise_to_rise: convert_delay(iopath.del_rise),
        rise_to_fall: convert_delay(iopath.del_fall),
        fall_to_rise: convert_delay(iopath.del_rise),
        fall_to_fall: convert_delay(iopath.del_fall),
    }
}

fn collect_int(collector: &mut SpeedCollector, name: &str, cell: &Cell) {
    assert_eq!(cell.iopath.len(), 1);
    let iopath = &cell.iopath[0];
    assert_eq!(iopath.port_from, Edge::Plain("I".into()));
    assert_eq!(iopath.port_to, Edge::Plain("O".into()));
    let delay = convert_delay_rf_unate(iopath);
    collector.insert(name, SpeedVal::DelayRfPosUnate(delay));
    assert!(cell.ports.is_empty());
    assert!(cell.setuphold.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_lc(collector: &mut SpeedCollector, cell: &Cell) {
    let mut setuphold = BTreeMap::new();
    for path in &cell.iopath {
        let Edge::Plain(port_to) = &path.port_to else {
            unreachable!()
        };
        match &path.port_from {
            Edge::Plain(port_from) => {
                let name = match (port_from.as_str(), port_to.as_str()) {
                    ("in0", "lcout") => "PLB:I0_TO_O",
                    ("in1", "lcout") => "PLB:I1_TO_O",
                    ("in2", "lcout") => "PLB:I2_TO_O",
                    ("in3", "lcout") => "PLB:I3_TO_O",
                    ("in0", "ltout") => "PLB:I0_TO_CASC",
                    ("in1", "ltout") => "PLB:I1_TO_CASC",
                    ("in2", "ltout") => "PLB:I2_TO_CASC",
                    ("in3", "ltout") => "PLB:I3_TO_CASC",
                    ("in1", "carryout") => "PLB:I1_TO_CO",
                    ("in2", "carryout") => "PLB:I2_TO_CO",
                    ("carryin", "carryout") => "PLB:CI_TO_CO",
                    ("sr", "lcout") => "PLB:RST_TO_O",
                    _ => panic!("unk path {port_from} {port_to}"),
                };
                if port_from == "sr" {
                    if path.del_rise != ZERO {
                        let delay = convert_delay(path.del_rise);
                        collector.insert(format!("{name}:RISE"), SpeedVal::Delay(delay));
                    }
                    if path.del_fall != ZERO {
                        let delay = convert_delay(path.del_fall);
                        collector.insert(format!("{name}:FALL"), SpeedVal::Delay(delay));
                    }
                } else if port_to == "carryout" {
                    let delay = convert_delay_rf_unate(path);
                    collector.insert(name, SpeedVal::DelayRfPosUnate(delay));
                } else {
                    let delay = convert_delay_rf_binate(path);
                    collector.insert(name, SpeedVal::DelayRfBinate(delay));
                }
            }
            Edge::Posedge(port_from) => {
                assert_eq!(port_from, "clk");
                assert_eq!(port_to, "lcout");
                let delay = convert_delay_rf_unate(path);
                collector.insert("PLB:CLK_TO_O", SpeedVal::DelayRfFromEdge(delay));
            }
            _ => unreachable!(),
        }
    }
    for sh in &cell.setuphold {
        let Edge::Posedge(port_c) = &sh.edge_c else {
            unreachable!()
        };
        assert_eq!(port_c, "clk");
        let (is_rise, port) = match &sh.edge_d {
            Edge::Posedge(port) => (true, port),
            Edge::Negedge(port) => (false, port),
            _ => unreachable!(),
        };
        let port = match port.as_str() {
            "in0" => "I0",
            "in1" => "I1",
            "in2" => "I2",
            "in3" => "I3",
            "ce" => "CE",
            "sr" => "RST",
            _ => unreachable!(),
        };
        let data = setuphold.entry(port).or_insert((None, None, None, None));
        if let Some(setup) = sh.setup {
            let delay = convert_delay(setup);
            if is_rise {
                data.0 = Some(delay);
            } else {
                data.1 = Some(delay);
            }
        }
        if let Some(hold) = sh.hold {
            let delay = convert_delay(hold);
            if is_rise {
                data.2 = Some(delay);
            } else {
                data.3 = Some(delay);
            }
        }
    }
    for (pin, (sr, sf, hr, hf)) in setuphold {
        collector.insert(
            format!("PLB:{pin}_SETUPHOLD_CLK"),
            SpeedVal::SetupHoldRf(SetupHoldRf {
                rise_setup: sr.unwrap(),
                rise_hold: hr.unwrap(),
                fall_setup: sf.unwrap(),
                fall_hold: hf.unwrap(),
            }),
        );
    }
    for recrem in &cell.recrem {
        let Edge::Posedge(port_c) = &recrem.edge_c else {
            unreachable!()
        };
        assert_eq!(port_c, "clk");
        if let Some(removal) = recrem.removal {
            assert_eq!(removal, ZERO);
        }
        if let Some(recovery) = recrem.recovery {
            match &recrem.edge_r {
                Edge::Negedge(port_r) => {
                    assert_eq!(port_r, "sr");
                    let delay = convert_delay(recovery);
                    collector.insert(
                        "PLB:RST_RECREM_CLK",
                        SpeedVal::RecRem(RecRem {
                            recovery: delay,
                            removal: Time::ZERO,
                        }),
                    );
                }
                Edge::Posedge(port_r) => {
                    assert_eq!(port_r, "sr");
                    assert_eq!(recovery, ZERO);
                }
                _ => unreachable!(),
            }
        }
    }
    assert!(cell.ports.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_carry_init(collector: &mut SpeedCollector, cell: &Cell) {
    assert_eq!(cell.iopath.len(), 1);
    let iopath = &cell.iopath[0];
    assert_eq!(iopath.port_from, Edge::Plain("carryinitin".into()));
    assert_eq!(iopath.port_to, Edge::Plain("carryinitout".into()));
    let delay = convert_delay_rf_unate(iopath);
    collector.insert("PLB:CARRY_INIT", SpeedVal::DelayRfPosUnate(delay));
    assert!(cell.ports.is_empty());
    assert!(cell.setuphold.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_gb_fabric(collector: &mut SpeedCollector, cell: &Cell) {
    assert_eq!(cell.iopath.len(), 1);
    let iopath = &cell.iopath[0];
    assert_eq!(
        iopath.port_from,
        Edge::Plain("USERSIGNALTOGLOBALBUFFER".into())
    );
    assert_eq!(iopath.port_to, Edge::Plain("GLOBALBUFFEROUTPUT".into()));
    let delay = convert_delay_rf_unate(iopath);
    collector.insert("GB_FABRIC", SpeedVal::DelayRfPosUnate(delay));
    assert!(cell.ports.is_empty());
    assert!(cell.setuphold.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_ice_io(collector: &mut SpeedCollector, cell: &Cell, design: &Design) {
    for path in &cell.iopath {
        let (is_edge, is_neg, port_from) = match path.port_from {
            Edge::Plain(ref port) => (false, false, port.as_str()),
            Edge::Posedge(ref port) => (true, false, port.as_str()),
            Edge::Negedge(ref port) => (true, true, port.as_str()),
        };
        let Edge::Plain(ref port_to) = path.port_to else {
            unreachable!()
        };
        let port_to = port_to.as_str();
        let (is_edge, key) = match (is_edge, is_neg, port_from, port_to) {
            (false, false, "PACKAGEPIN", "DIN0") => (false, "PAD_TO_DIN0"),
            (false, false, "PACKAGEPIN", "GLOBALBUFFEROUTPUT") => (false, "PAD_TO_GB"),
            (false, false, "LATCHINPUTVALUE", "DIN0") => (false, "LATCH_TO_DIN0"),
            (true, false, "INPUTCLK", "DIN0") => (true, "ICLK_P_TO_DIN0"),
            (true, true, "INPUTCLK", "DIN1") => (true, "ICLK_N_TO_DIN1"),
            (false, false, "DOUT0", "PACKAGEPIN") => (false, "DOUT0_TO_PAD"),
            (true, false, "OUTPUTCLK", "PACKAGEPIN") => (true, "OCLK_P_TO_PAD"),
            (true, true, "OUTPUTCLK", "PACKAGEPIN") => (true, "OCLK_N_TO_PAD"),
            (false, false, "OUTPUTENABLE", "PACKAGEPIN") => (false, "OE_TO_PAD_ON"),
            // BUG: icecube uses wrong edge here. but only on the T speed grade.
            (false, false, "INPUTCLK", "PACKAGEPIN") => (false, "OE_TO_PAD_OFF"),
            (false, false, "OUTPUTCLK", "PACKAGEPIN") => (true, "OCLK_P_TO_PAD_OE"),
            _ => {
                println!("unk IOPATH {path:?}");
                continue;
            }
        };
        let delay = convert_delay_rf_unate(path);
        if is_edge {
            collector.insert(format!("IO:{key}"), SpeedVal::DelayRfFromEdge(delay));
        } else {
            collector.insert(format!("IO:{key}"), SpeedVal::DelayRfPosUnate(delay));
            if design.speed == "L" && key == "OE_TO_PAD_ON" {
                collector.insert("IO:OE_TO_PAD_OFF", SpeedVal::DelayRfPosUnate(delay));
            }
        }
    }
    let mut setuphold = BTreeMap::new();
    for sh in &cell.setuphold {
        let (is_neg, port_c) = match sh.edge_c {
            Edge::Posedge(ref port) => (false, port.as_str()),
            Edge::Negedge(ref port) => (true, port.as_str()),
            _ => unreachable!(),
        };
        let (is_rise, port_d) = match sh.edge_d {
            Edge::Posedge(ref port) => (true, port.as_str()),
            Edge::Negedge(ref port) => (false, port.as_str()),
            _ => unreachable!(),
        };
        let key = match (port_d, is_neg, port_c) {
            ("CLOCKENABLE", false, "INPUTCLK") => "CE_SETUPHOLD_ICLK",
            ("CLOCKENABLE", false, "OUTPUTCLK") => "CE_SETUPHOLD_OCLK",
            ("PACKAGEPIN", false, "INPUTCLK") => "PAD_SETUPHOLD_ICLK_P",
            ("PACKAGEPIN", true, "INPUTCLK") => "PAD_SETUPHOLD_ICLK_N",
            ("DOUT0", false, "OUTPUTCLK") => "DOUT0_SETUPHOLD_OCLK_P",
            ("DOUT1", true, "OUTPUTCLK") => "DOUT1_SETUPHOLD_OCLK_N",
            ("OUTPUTENABLE", false, "OUTPUTCLK") => "OE_SETUPHOLD_OCLK_P",
            _ => unreachable!(),
        };
        let data = setuphold.entry(key).or_insert((None, None, None, None));
        if let Some(setup) = sh.setup {
            let delay = convert_delay(setup);
            if is_rise {
                data.0 = Some(delay);
            } else {
                data.1 = Some(delay);
            }
        }
        if let Some(hold) = sh.hold {
            let delay = convert_delay(hold);
            if is_rise {
                data.2 = Some(delay);
            } else {
                data.3 = Some(delay);
            }
        }
    }
    for (key, (sr, sf, hr, hf)) in setuphold {
        collector.insert(
            format!("IO:{key}"),
            SpeedVal::SetupHoldRf(SetupHoldRf {
                rise_setup: sr.unwrap(),
                rise_hold: hr.unwrap(),
                fall_setup: sf.unwrap(),
                fall_hold: hf.unwrap(),
            }),
        );
    }
    assert!(cell.ports.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_pre_io(collector: &mut SpeedCollector, cell: &Cell) {
    for path in &cell.iopath {
        let (is_edge, is_neg, port_from) = match path.port_from {
            Edge::Plain(ref port) => (false, false, port.as_str()),
            Edge::Posedge(ref port) => (true, false, port.as_str()),
            Edge::Negedge(ref port) => (true, true, port.as_str()),
        };
        let Edge::Plain(ref port_to) = path.port_to else {
            unreachable!()
        };
        let port_to = port_to.as_str();
        let (is_edge, key) = match (is_edge, is_neg, port_from, port_to) {
            (false, false, "PADIN", "DIN0") => (false, "PADIN_TO_DIN0"),
            (false, false, "PADSIGNALTOGLOBALBUFFER", "GLOBALBUFFEROUTPUT") => {
                (false, "PADIN_TO_GB")
            }
            (false, false, "LATCHINPUTVALUE", "DIN0") => (false, "LATCH_TO_DIN0"),
            (true, false, "INPUTCLK", "DIN0") => (true, "ICLK_P_TO_DIN0"),
            (true, true, "INPUTCLK", "DIN1") => (true, "ICLK_N_TO_DIN1"),
            (false, false, "DOUT0", "PADOUT") => (false, "DOUT0_TO_PADOUT"),
            (true, false, "OUTPUTCLK", "PADOUT") => (true, "OCLK_P_TO_PADOUT"),
            (true, true, "OUTPUTCLK", "PADOUT") => (true, "OCLK_N_TO_PADOUT"),
            (false, false, "OUTPUTENABLE", "PADOEN") => (false, "OE_TO_PADOEN"),
            (true, false, "OUTPUTCLK", "PADOEN") => (true, "OCLK_P_TO_PADOEN"),
            _ => {
                println!("unk IOPATH {path:?}");
                continue;
            }
        };
        let delay = convert_delay_rf_unate(path);
        if is_edge {
            collector.insert(format!("IO:{key}"), SpeedVal::DelayRfFromEdge(delay));
        } else {
            collector.insert(format!("IO:{key}"), SpeedVal::DelayRfPosUnate(delay));
        }
    }
    let mut setuphold = BTreeMap::new();
    for sh in &cell.setuphold {
        let (is_neg, port_c) = match sh.edge_c {
            Edge::Posedge(ref port) => (false, port.as_str()),
            Edge::Negedge(ref port) => (true, port.as_str()),
            _ => unreachable!(),
        };
        let (is_rise, port_d) = match sh.edge_d {
            Edge::Posedge(ref port) => (true, port.as_str()),
            Edge::Negedge(ref port) => (false, port.as_str()),
            _ => unreachable!(),
        };
        let key = match (port_d, is_neg, port_c) {
            ("CLOCKENABLE", false, "INPUTCLK") => "CE_SETUPHOLD_ICLK",
            ("CLOCKENABLE", false, "OUTPUTCLK") => "CE_SETUPHOLD_OCLK",
            ("PADIN", false, "INPUTCLK") => "PADIN_SETUPHOLD_ICLK_P",
            ("PADIN", true, "INPUTCLK") => "PADIN_SETUPHOLD_ICLK_N",
            ("DOUT0", false, "OUTPUTCLK") => "DOUT0_SETUPHOLD_OCLK_P",
            ("DOUT1", true, "OUTPUTCLK") => "DOUT1_SETUPHOLD_OCLK_N",
            ("OUTPUTENABLE", false, "OUTPUTCLK") => "OE_SETUPHOLD_OCLK_P",
            _ => {
                println!("unk SETUPHOLD {sh:?}");
                continue;
            }
        };
        let data = setuphold.entry(key).or_insert((None, None, None, None));
        if let Some(setup) = sh.setup {
            let delay = convert_delay(setup);
            if is_rise {
                data.0 = Some(delay);
            } else {
                data.1 = Some(delay);
            }
        }
        if let Some(hold) = sh.hold {
            let delay = convert_delay(hold);
            if is_rise {
                data.2 = Some(delay);
            } else {
                data.3 = Some(delay);
            }
        }
    }
    for (key, (sr, sf, hr, hf)) in setuphold {
        collector.insert(
            format!("IO:{key}"),
            SpeedVal::SetupHoldRf(SetupHoldRf {
                rise_setup: sr.unwrap(),
                rise_hold: hr.unwrap(),
                fall_setup: sf.unwrap(),
                fall_hold: hf.unwrap(),
            }),
        );
    }
    assert!(cell.ports.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_pll(collector: &mut SpeedCollector, cell: &Cell, inst: &Instance) {
    let feedback = inst.props["FEEDBACK_PATH"].as_str();
    let typ = cell.typ.as_str();
    for path in &cell.iopath {
        let Edge::Plain(ref port_to) = path.port_to else {
            unreachable!();
        };
        let Edge::Plain(ref port_from) = path.port_from else {
            let Edge::Negedge(ref port_from) = path.port_from else {
                unreachable!();
            };
            assert_eq!(port_from, "SCLK");
            assert_eq!(port_to, "SDO");
            continue;
        };
        let key = match (typ, port_from.as_str(), port_to.as_str()) {
            ("SB_PLL_CORE", "REFERENCECLK", "PLLOUTCORE") => {
                format!("PLL:REFERENCECLK_TO_PLLOUTCORE_{feedback}")
            }
            ("SB_PLL_CORE", "REFERENCECLK", "PLLOUTGLOBAL") => {
                format!("PLL:REFERENCECLK_TO_PLLOUTGLOBAL_{feedback}")
            }
            ("SB_PLL_PAD", "PACKAGEPIN", "PLLOUTCORE") => {
                format!("PLL:PAD_TO_PLLOUTCORE_{feedback}")
            }
            ("SB_PLL_PAD", "PACKAGEPIN", "PLLOUTGLOBAL") => {
                format!("PLL:PAD_TO_PLLOUTGLOBAL_{feedback}")
            }
            ("SB_PLL_2_PAD", "PACKAGEPIN", "PLLOUTCOREB") => {
                format!("PLL:PAD_TO_PLLOUTCORE_{feedback}")
            }
            ("SB_PLL_2_PAD", "PACKAGEPIN", "PLLOUTGLOBALB") => {
                format!("PLL:PAD_TO_PLLOUTGLOBAL_{feedback}")
            }
            ("SB_PLL_2_PAD", "PACKAGEPIN", "PLLOUTCOREA") => "IO:PAD_TO_DIN0".to_string(),
            ("SB_PLL_2_PAD", "PACKAGEPIN", "PLLOUTGLOBALA") => "IO:PAD_TO_GB".to_string(),

            ("SB_PLL40_CORE", "REFERENCECLK", "PLLOUTCORE") => {
                format!("PLL40_CORE:REFERENCECLK_TO_PLLOUTCORE_{feedback}")
            }
            ("SB_PLL40_CORE", "REFERENCECLK", "PLLOUTGLOBAL") => {
                format!("PLL40_CORE:REFERENCECLK_TO_PLLOUTGLOBAL_{feedback}")
            }
            ("SB_PLL40_2F_CORE", "REFERENCECLK", "PLLOUTCOREA") => {
                format!("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREA_{feedback}")
            }
            ("SB_PLL40_2F_CORE", "REFERENCECLK", "PLLOUTGLOBALA") => {
                format!("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALA_{feedback}")
            }
            ("SB_PLL40_2F_CORE", "REFERENCECLK", "PLLOUTCOREB") => {
                format!("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREB_{feedback}")
            }
            ("SB_PLL40_2F_CORE", "REFERENCECLK", "PLLOUTGLOBALB") => {
                format!("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALB_{feedback}")
            }

            ("PLL40", "PLLIN", "PLLOUTCORE") => {
                format!("PLL40_PAD:PLLIN_TO_PLLOUTCORE_{feedback}")
            }
            ("PLL40", "PLLIN", "PLLOUTGLOBAL") => {
                format!("PLL40_PAD:PLLIN_TO_PLLOUTGLOBAL_{feedback}")
            }
            ("PLL40_2", "PLLIN", "PLLOUTCOREA") => {
                format!("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREA_{feedback}")
            }
            ("PLL40_2", "PLLIN", "PLLOUTGLOBALA") => {
                format!("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALA_{feedback}")
            }
            ("PLL40_2", "PLLIN", "PLLOUTCOREB") => {
                format!("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREB_{feedback}")
            }
            ("PLL40_2", "PLLIN", "PLLOUTGLOBALB") => {
                format!("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALB_{feedback}")
            }
            ("PLL40_2F", "PLLIN", "PLLOUTCOREA") => {
                format!("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREA_{feedback}")
            }
            ("PLL40_2F", "PLLIN", "PLLOUTGLOBALA") => {
                format!("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALA_{feedback}")
            }
            ("PLL40_2F", "PLLIN", "PLLOUTCOREB") => {
                format!("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREB_{feedback}")
            }
            ("PLL40_2F", "PLLIN", "PLLOUTGLOBALB") => {
                format!("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALB_{feedback}")
            }

            _ => {
                println!("unk IOPATH {typ} {feedback} {path:?}");
                continue;
            }
        };
        let delay = convert_delay_rf_unate(path);
        collector.insert(key, SpeedVal::DelayRfPosUnate(delay));
    }
    for sh in &cell.setuphold {
        let Edge::Negedge(ref port_c) = sh.edge_c else {
            unreachable!();
        };
        assert_eq!(port_c, "SCLK");
        let (Edge::Negedge(ref port_d) | Edge::Posedge(ref port_d)) = sh.edge_d else {
            unreachable!();
        };
        assert_eq!(port_d, "SDI");
    }
    assert!(cell.ports.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn strip_index(name: &str) -> &str {
    if let Some((name, rest)) = name.split_once('[') {
        assert!(rest.ends_with(']'));
        name
    } else {
        name
    }
}

fn collect_simple(collector: &mut SpeedCollector, kind: &str, cell: &Cell) {
    for path in &cell.iopath {
        // println!("IOPATH {kind} {path:?}");
        let Edge::Plain(port_to) = &path.port_to else {
            unreachable!()
        };
        let port_to = strip_index(port_to);
        let (is_neg, port_from) = match path.port_from {
            Edge::Posedge(ref port_from) => (false, port_from),
            Edge::Negedge(ref port_from) => (true, port_from),
            _ => unreachable!(),
        };
        let delay = convert_delay_rf_unate(path);
        if matches!(port_from.as_str(), "SCKI" | "SCKO" | "SCLI" | "SCLO") {
            let pn = if is_neg { 'N' } else { 'P' };
            collector.insert(
                format!("{kind}:{port_from}_{pn}_TO_{port_to}"),
                SpeedVal::DelayRfFromEdge(delay),
            );
        } else {
            assert!(!is_neg);
            collector.insert(
                format!("{kind}:{port_from}_TO_{port_to}"),
                SpeedVal::DelayRfFromEdge(delay),
            );
        };
    }
    let mut setuphold = BTreeMap::new();
    for sh in &cell.setuphold {
        // println!("SETUPHOLD {kind} {sh:?}");
        let (is_neg, port_c) = match sh.edge_c {
            Edge::Posedge(ref port_c) => (false, port_c),
            Edge::Negedge(ref port_c) => (true, port_c),
            _ => unreachable!(),
        };
        let (is_rise, port_d) = match &sh.edge_d {
            Edge::Posedge(port) => (true, port),
            Edge::Negedge(port) => (false, port),
            _ => unreachable!(),
        };
        let port_d = strip_index(port_d);
        let data = setuphold
            .entry((port_d, is_neg, port_c))
            .or_insert((None, None, None, None));
        if let Some(setup) = sh.setup {
            let delay = convert_delay(setup);
            if is_rise {
                data.0 = Some(delay);
            } else {
                data.1 = Some(delay);
            }
        }
        if let Some(hold) = sh.hold {
            let delay = convert_delay(hold);
            if is_rise {
                data.2 = Some(delay);
            } else {
                data.3 = Some(delay);
            }
        }
    }
    for ((port_d, is_neg, port_c), (sr, sf, hr, hf)) in setuphold {
        let key = if matches!(port_c.as_str(), "SCKI" | "SCKO" | "SCLI" | "SCLO") {
            let pn = if is_neg { 'N' } else { 'P' };
            format!("{kind}:{port_d}_SETUPHOLD_{port_c}_{pn}")
        } else {
            assert!(!is_neg);
            format!("{kind}:{port_d}_SETUPHOLD_{port_c}")
        };
        collector.insert(
            key,
            SpeedVal::SetupHoldRf(SetupHoldRf {
                rise_setup: sr.unwrap(),
                rise_hold: hr.unwrap(),
                fall_setup: sf.unwrap(),
                fall_hold: hf.unwrap(),
            }),
        );
    }
    assert!(cell.ports.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_drv(collector: &mut SpeedCollector, kind: &str, cell: &Cell) {
    // NOTE: the EN-to-LED paths have two delays in the `.lib` file, `three_state_enable`
    // and `three_state_disable`.  They are just dumped as two IO paths between the same terminals
    // upon SDF conversion.  We undo this transformation, and merge them into a single struct.
    // Since the output in question is open drain, we model output enable as a falling edge,
    // and output disable as a rising edge.
    let mut assembly = BTreeMap::new();
    for path in &cell.iopath {
        // println!("IOPATH {kind} {path:?}");
        let Edge::Plain(port_to) = &path.port_to else {
            unreachable!()
        };
        let Edge::Plain(port_from) = &path.port_from else {
            unreachable!()
        };
        let delay = convert_delay_rf_unate(path);
        let key = format!("{kind}:{port_from}_TO_{port_to}");
        if port_from.contains("EN") && kind != "LED_DRV_CUR" {
            assert_eq!(delay.rise, delay.fall);
            let delay = delay.rise;
            match assembly.entry(key) {
                btree_map::Entry::Vacant(entry) => {
                    entry.insert(delay);
                }
                btree_map::Entry::Occupied(entry) => {
                    let (key, fall) = entry.remove_entry();
                    let delay = DelayRfUnate { rise: delay, fall };
                    collector.insert(key, SpeedVal::DelayRfNegUnate(delay));
                }
            }
        } else {
            collector.insert(key, SpeedVal::DelayRfPosUnate(delay));
        }
    }
}

fn collect_filter(collector: &mut SpeedCollector, cell: &Cell) {
    assert_eq!(cell.iopath.len(), 1);
    let iopath = &cell.iopath[0];
    assert_eq!(iopath.port_from, Edge::Plain("FILTERIN".into()));
    assert_eq!(iopath.port_to, Edge::Plain("FILTEROUT".into()));
    let delay = convert_delay_rf_unate(iopath);
    collector.insert("FILTER:IN_TO_OUT", SpeedVal::DelayRfPosUnate(delay));
    assert!(cell.ports.is_empty());
    assert!(cell.setuphold.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_null(cell: &Cell) {
    for iopath in &cell.iopath {
        assert_eq!(iopath.del_rise, ZERO);
        assert_eq!(iopath.del_fall, ZERO);
    }
    assert!(cell.ports.is_empty());
    for sh in &cell.setuphold {
        if let Some(d) = sh.hold {
            assert_eq!(d, ZERO);
        }
        if let Some(d) = sh.setup {
            assert_eq!(d, ZERO);
        }
    }
    for rr in &cell.recrem {
        if let Some(d) = rr.recovery {
            assert_eq!(d, ZERO);
        }
        if let Some(d) = rr.removal {
            assert_eq!(d, ZERO);
        }
    }
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn insert_iob_data(
    collector: &mut SpeedCollector,
    prefix: &str,
    del_out: (f64, f64),
    res_out: (f64, f64),
    del_oe: (f64, f64),
    del_in: (f64, f64),
) {
    collector.insert(
        format!("{prefix}:PAD_TO_PADIN"),
        SpeedVal::DelayRfPosUnate(DelayRfUnate {
            rise: Time(del_in.0.into()),
            fall: Time(del_in.1.into()),
        }),
    );
    collector.insert(
        format!("{prefix}:PADOUT_TO_PAD"),
        SpeedVal::DelayRfPosUnate(DelayRfUnate {
            rise: Time(del_out.0.into()),
            fall: Time(del_out.1.into()),
        }),
    );
    collector.insert(
        format!("{prefix}:PADOUT_TO_PAD_RES"),
        SpeedVal::ResistanceRf(ResistanceRf {
            rise: Resistance(res_out.0.into()),
            fall: Resistance(res_out.1.into()),
        }),
    );
    collector.insert(
        format!("{prefix}:PADOEN_TO_PAD"),
        SpeedVal::DelayRfPosUnate(DelayRfUnate {
            rise: Time(del_oe.0.into()),
            fall: Time(del_oe.1.into()),
        }),
    );
}

pub fn init_speed_data(kind: ChipKind, part: &str, grade: &str) -> SpeedCollector {
    let mut collector = SpeedCollector::new();

    collector.want("DERATE_V");
    if kind.is_ice65() {
        collector.want("DERATE_P_WORST");
        collector.want("DERATE_P_TYP");
        collector.want("DERATE_P_BEST");
        if grade == "L" {
            collector.want("DERATE_T_WORST");
            collector.want("DERATE_T_TYP");
            collector.want("DERATE_T_BEST");
            collector.insert("DERATE_P_WORST", SpeedVal::Scalar(1.075.into()));
            collector.insert("DERATE_P_TYP", SpeedVal::Scalar(1.0.into()));
            collector.insert("DERATE_P_BEST", SpeedVal::Scalar(0.5.into()));
            collector.insert(
                "DERATE_V",
                SpeedVal::DerateFactorVoltageInvQuadratic(DerateFactorVoltageInvQuadratic {
                    a: 0.548.into(),
                    b: 1.1588.into(),
                    c: (-1.1768).into(),
                }),
            );
            collector.insert(
                "DERATE_T_WORST",
                SpeedVal::DerateFactorTemperatureLinear(DerateFactorTemperatureLinear {
                    a: 0.00021.into(),
                    b: 0.994.into(),
                }),
            );
            collector.insert(
                "DERATE_T_TYP",
                SpeedVal::DerateFactorTemperatureLinear(DerateFactorTemperatureLinear {
                    a: 0.000414.into(),
                    b: 0.989.into(),
                }),
            );
            collector.insert(
                "DERATE_T_BEST",
                SpeedVal::DerateFactorTemperatureLinear(DerateFactorTemperatureLinear {
                    a: 0.000552.into(),
                    b: 0.986.into(),
                }),
            );
        } else {
            collector.want("DERATE_T");
            collector.insert("DERATE_P_WORST", SpeedVal::Scalar(1.095.into()));
            collector.insert("DERATE_P_TYP", SpeedVal::Scalar(1.0.into()));
            collector.insert("DERATE_P_BEST", SpeedVal::Scalar(0.5.into()));
            collector.insert(
                "DERATE_V",
                SpeedVal::DerateFactorVoltageInvQuadratic(DerateFactorVoltageInvQuadratic {
                    a: 0.0216.into(),
                    b: 1.7748.into(),
                    c: (-1.1641).into(),
                }),
            );
            collector.insert(
                "DERATE_T",
                SpeedVal::DerateFactorTemperatureLinear(DerateFactorTemperatureLinear {
                    a: 0.0006.into(),
                    b: 0.985.into(),
                }),
            );
        }
    } else {
        collector.want("DERATE_T");
        let derate_v_lp = DerateFactorVoltageInvQuadratic {
            a: 0.337.into(),
            b: 1.304.into(),
            c: (-1.052).into(),
        };
        let derate_v_lp_12 = derate_v_lp.eval(Voltage(1.2.into()));
        let derate_p = |f: f64| Scalar(f) / Scalar(1.327) * Scalar(0.85) / derate_v_lp_12;
        if part.starts_with("iCE40HX") {
            collector.want("DERATE_P");
            collector.insert("DERATE_P", SpeedVal::Scalar(derate_p(0.973)));
            collector.insert(
                "DERATE_V",
                SpeedVal::DerateFactorVoltageInvQuadratic(DerateFactorVoltageInvQuadratic {
                    a: (-0.135).into(),
                    b: 2.013.into(),
                    c: (-1.223).into(),
                }),
            );
            collector.insert(
                "DERATE_T",
                SpeedVal::DerateFactorTemperatureLinear(DerateFactorTemperatureLinear {
                    a: 0.0001722.into(),
                    b: 0.996.into(),
                }),
            );
        } else {
            collector.want("DERATE_P_WORST");
            collector.want("DERATE_P_TYP");
            collector.want("DERATE_P_BEST");
            if part.starts_with("iCE40LP") {
                collector.insert("DERATE_P_WORST", SpeedVal::Scalar(derate_p(1.421)));
                collector.insert("DERATE_P_TYP", SpeedVal::Scalar(derate_p(1.327)));
                collector.insert("DERATE_P_BEST", SpeedVal::Scalar(derate_p(1.149)));
            } else {
                collector.insert("DERATE_P_WORST", SpeedVal::Scalar(1.164.into()));
                collector.insert("DERATE_P_TYP", SpeedVal::Scalar(0.858.into()));
                collector.insert("DERATE_P_BEST", SpeedVal::Scalar(0.552.into()));
            }
            collector.insert(
                "DERATE_V",
                SpeedVal::DerateFactorVoltageInvQuadratic(derate_v_lp),
            );
            collector.insert(
                "DERATE_T",
                SpeedVal::DerateFactorTemperatureLinear(DerateFactorTemperatureLinear {
                    a: (-0.00012).into(),
                    b: 1.003.into(),
                }),
            );
        }
    }

    // interconnect
    collector.want("INT:IMUX_LC");
    collector.want("INT:IMUX_IO");
    collector.want("INT:IMUX_CLK");
    collector.want("INT:IMUX_CE");
    collector.want("INT:IMUX_RST");
    collector.want("INT:LOCAL");
    collector.want("INT:GOUT");
    collector.want("INT:GLOBAL");
    // TODO: wtf?
    collector.want("INT:QUAD");
    collector.want("INT:LONG");
    for i in 0..=4 {
        collector.want(format!("INT:QUAD_V_{i}"));
        collector.want(format!("INT:QUAD_H_{i}"));
    }
    for i in 0..=12 {
        collector.want(format!("INT:LONG_V_{i}"));
        collector.want(format!("INT:LONG_H_{i}"));
    }
    collector.want("INT:QUAD_IO");
    collector.want("INT:OUT_TO_QUAD");
    collector.want("INT:OUT_TO_LONG");
    collector.want("INT:LONG_TO_QUAD");

    collector.want("GB_FABRIC");

    // PLB
    collector.want("PLB:CARRY_INIT");
    collector.want("PLB:I0_TO_O");
    collector.want("PLB:I1_TO_O");
    collector.want("PLB:I2_TO_O");
    collector.want("PLB:I3_TO_O");
    if kind.is_ice40() {
        collector.want("PLB:I0_TO_CASC");
        collector.want("PLB:I1_TO_CASC");
        collector.want("PLB:I2_TO_CASC");
        collector.want("PLB:I3_TO_CASC");
    }
    collector.want("PLB:I1_TO_CO");
    collector.want("PLB:I2_TO_CO");
    collector.want("PLB:CI_TO_CO");
    collector.want("PLB:CLK_TO_O");
    // these two are merged later
    collector.want("PLB:RST_TO_O:RISE");
    collector.want("PLB:RST_TO_O:FALL");
    collector.want("PLB:I0_SETUPHOLD_CLK");
    collector.want("PLB:I1_SETUPHOLD_CLK");
    collector.want("PLB:I2_SETUPHOLD_CLK");
    collector.want("PLB:I3_SETUPHOLD_CLK");
    collector.want("PLB:CE_SETUPHOLD_CLK");
    collector.want("PLB:RST_SETUPHOLD_CLK");
    collector.want("PLB:RST_RECREM_CLK");

    // IO and PLL
    if kind.is_ice65() {
        collector.want("IO:CE_SETUPHOLD_ICLK");
        collector.want("IO:CE_SETUPHOLD_OCLK");

        collector.want("IO:PAD_TO_DIN0");
        collector.want("IO:PAD_TO_GB");
        collector.want("IO:LATCH_TO_DIN0");
        collector.want("IO:PAD_SETUPHOLD_ICLK_P");
        collector.want("IO:PAD_SETUPHOLD_ICLK_N");
        collector.want("IO:ICLK_P_TO_DIN0");
        collector.want("IO:ICLK_N_TO_DIN1");

        collector.want("IO:DOUT0_TO_PAD");
        collector.want("IO:DOUT0_SETUPHOLD_OCLK_P");
        collector.want("IO:DOUT1_SETUPHOLD_OCLK_N");
        collector.want("IO:OCLK_P_TO_PAD");
        collector.want("IO:OCLK_N_TO_PAD");

        collector.want("IO:OE_TO_PAD_ON");
        collector.want("IO:OE_TO_PAD_OFF");
        collector.want("IO:OE_SETUPHOLD_OCLK_P");
        collector.want("IO:OCLK_P_TO_PAD_OE");

        if kind == ChipKind::Ice65P04 {
            collector.want("PLL:REFERENCECLK_TO_PLLOUTCORE_SIMPLE");
            collector.want("PLL:REFERENCECLK_TO_PLLOUTCORE_DELAY");
            collector.want("PLL:REFERENCECLK_TO_PLLOUTCORE_PHASE_AND_DELAY");
            collector.want("PLL:REFERENCECLK_TO_PLLOUTCORE_EXTERNAL");
            collector.want("PLL:REFERENCECLK_TO_PLLOUTGLOBAL_SIMPLE");
            collector.want("PLL:REFERENCECLK_TO_PLLOUTGLOBAL_DELAY");
            collector.want("PLL:REFERENCECLK_TO_PLLOUTGLOBAL_PHASE_AND_DELAY");
            collector.want("PLL:REFERENCECLK_TO_PLLOUTGLOBAL_EXTERNAL");

            collector.want("PLL:PAD_TO_PLLOUTCORE_SIMPLE");
            collector.want("PLL:PAD_TO_PLLOUTCORE_DELAY");
            collector.want("PLL:PAD_TO_PLLOUTCORE_PHASE_AND_DELAY");
            collector.want("PLL:PAD_TO_PLLOUTCORE_EXTERNAL");
            collector.want("PLL:PAD_TO_PLLOUTGLOBAL_SIMPLE");
            collector.want("PLL:PAD_TO_PLLOUTGLOBAL_DELAY");
            collector.want("PLL:PAD_TO_PLLOUTGLOBAL_PHASE_AND_DELAY");
            collector.want("PLL:PAD_TO_PLLOUTGLOBAL_EXTERNAL");
        }
    } else {
        collector.want("IO:CE_SETUPHOLD_ICLK");
        collector.want("IO:CE_SETUPHOLD_OCLK");

        collector.want("IO:PADIN_TO_DIN0");
        collector.want("IO:PADIN_TO_GB");
        collector.want("IO:LATCH_TO_DIN0");
        collector.want("IO:PADIN_SETUPHOLD_ICLK_P");
        collector.want("IO:PADIN_SETUPHOLD_ICLK_N");
        collector.want("IO:ICLK_P_TO_DIN0");
        collector.want("IO:ICLK_N_TO_DIN1");

        collector.want("IO:DOUT0_TO_PADOUT");
        collector.want("IO:DOUT0_SETUPHOLD_OCLK_P");
        collector.want("IO:DOUT1_SETUPHOLD_OCLK_N");
        collector.want("IO:OCLK_P_TO_PADOUT");
        collector.want("IO:OCLK_N_TO_PADOUT");

        collector.want("IO:OE_TO_PADOEN");
        collector.want("IO:OE_SETUPHOLD_OCLK_P");
        collector.want("IO:OCLK_P_TO_PADOEN");

        for v in ["1.8", "2.5", "3.3"] {
            collector.want(format!("IOB_{v}:PAD_TO_PADIN"));
            collector.want(format!("IOB_{v}:PADOUT_TO_PAD"));
            collector.want(format!("IOB_{v}:PADOUT_TO_PAD_RES"));
            collector.want(format!("IOB_{v}:PADOEN_TO_PAD"));
        }

        if kind == ChipKind::Ice40R04 {
            insert_iob_data(
                &mut collector,
                "IOB_3.3",
                (1941.5 - 353.0, 1973.2 - 353.0),
                (35.0, 38.0),
                (2537.0, 2347.0),
                (485.0, 755.0),
            );
            insert_iob_data(
                &mut collector,
                "IOB_2.5",
                (1941.5, 1973.2),
                (35.0, 38.0),
                (2890.0, 2700.0),
                (830.0, 1100.0),
            );
            insert_iob_data(
                &mut collector,
                "IOB_1.8",
                (1941.5 + 1191.0, 1973.2 + 1191.0),
                (35.0, 38.0),
                (4081.0, 3891.0),
                (2021.0, 2291.0),
            );
        } else {
            insert_iob_data(
                &mut collector,
                "IOB_3.3",
                (1634.0, 1768.0),
                (28.0, 32.0),
                (1768.0, 1634.0),
                (510.0, 460.0),
            );
            insert_iob_data(
                &mut collector,
                "IOB_2.5",
                (1941.5, 1973.2),
                (35.0, 38.0),
                (1973.0, 1942.0),
                (590.0, 540.0),
            );
            insert_iob_data(
                &mut collector,
                "IOB_1.8",
                (2824.0, 2707.0),
                (51.0, 53.0),
                (2707.0, 2824.0),
                (850.0, 770.0),
            );
        }

        if kind.has_actual_io_we() {
            for v in ["1.8", "2.5", "3.3"] {
                collector.want(format!("IOB_W_{v}:PAD_TO_PADIN"));
                collector.want(format!("IOB_W_{v}:PADOUT_TO_PAD"));
                collector.want(format!("IOB_W_{v}:PADOUT_TO_PAD_RES"));
                collector.want(format!("IOB_W_{v}:PADOEN_TO_PAD"));
            }
            insert_iob_data(
                &mut collector,
                "IOB_W_3.3",
                (1634.0, 1768.0),
                (28.0, 32.0),
                (1681.0, 1617.0),
                (510.0, 460.0),
            );
            insert_iob_data(
                &mut collector,
                "IOB_W_2.5",
                (1941.5, 1973.2),
                (35.0, 38.0),
                (1902.0, 1990.0),
                (590.0, 540.0),
            );
            insert_iob_data(
                &mut collector,
                "IOB_W_1.8",
                (2824.0, 2707.0),
                (51.0, 53.0),
                (2631.0, 2965.0),
                (850.0, 770.0),
            );
        }

        if !kind.has_io_we() {
            for v in ["1.8", "2.5", "3.3"] {
                collector.want(format!("IOB_OD_{v}:PAD_TO_PADIN"));
                collector.want(format!("IOB_OD_{v}:PADOUT_TO_PAD"));
                collector.want(format!("IOB_OD_{v}:PADOUT_TO_PAD_RES"));
                collector.want(format!("IOB_OD_{v}:PADOEN_TO_PAD"));
            }
            insert_iob_data(
                &mut collector,
                "IOB_OD_3.3",
                (f64::INFINITY, 1768.0),
                (f64::INFINITY, 32.0),
                (2510.0, 2350.0),
                (720.0, 960.0),
            );
            insert_iob_data(
                &mut collector,
                "IOB_OD_2.5",
                (f64::INFINITY, 1973.2),
                (f64::INFINITY, 38.0),
                (2890.0, 2700.0),
                (830.0, 1100.0),
            );
            insert_iob_data(
                &mut collector,
                "IOB_OD_1.8",
                (f64::INFINITY, 2707.0),
                (f64::INFINITY, 53.0),
                (4080.0, 3810.0),
                (1180.0, 1560.0),
            );
        }

        if kind != ChipKind::Ice40P03 && part != "iCE40HX640" {
            collector.want("PLL40_CORE:REFERENCECLK_TO_PLLOUTCORE_SIMPLE");
            collector.want("PLL40_CORE:REFERENCECLK_TO_PLLOUTCORE_DELAY");
            collector.want("PLL40_CORE:REFERENCECLK_TO_PLLOUTCORE_PHASE_AND_DELAY");
            collector.want("PLL40_CORE:REFERENCECLK_TO_PLLOUTCORE_EXTERNAL");
            collector.want("PLL40_CORE:REFERENCECLK_TO_PLLOUTGLOBAL_SIMPLE");
            collector.want("PLL40_CORE:REFERENCECLK_TO_PLLOUTGLOBAL_DELAY");
            collector.want("PLL40_CORE:REFERENCECLK_TO_PLLOUTGLOBAL_PHASE_AND_DELAY");
            collector.want("PLL40_CORE:REFERENCECLK_TO_PLLOUTGLOBAL_EXTERNAL");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREA_SIMPLE");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREA_DELAY");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREA_PHASE_AND_DELAY");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREA_EXTERNAL");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALA_SIMPLE");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALA_DELAY");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALA_PHASE_AND_DELAY");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALA_EXTERNAL");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREB_SIMPLE");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREB_DELAY");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREB_PHASE_AND_DELAY");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTCOREB_EXTERNAL");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALB_SIMPLE");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALB_DELAY");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALB_PHASE_AND_DELAY");
            collector.want("PLL40_2F_CORE:REFERENCECLK_TO_PLLOUTGLOBALB_EXTERNAL");
            collector.want("PLL40_PAD:PLLIN_TO_PLLOUTCORE_SIMPLE");
            collector.want("PLL40_PAD:PLLIN_TO_PLLOUTCORE_DELAY");
            collector.want("PLL40_PAD:PLLIN_TO_PLLOUTCORE_PHASE_AND_DELAY");
            collector.want("PLL40_PAD:PLLIN_TO_PLLOUTCORE_EXTERNAL");
            collector.want("PLL40_PAD:PLLIN_TO_PLLOUTGLOBAL_SIMPLE");
            collector.want("PLL40_PAD:PLLIN_TO_PLLOUTGLOBAL_DELAY");
            collector.want("PLL40_PAD:PLLIN_TO_PLLOUTGLOBAL_PHASE_AND_DELAY");
            collector.want("PLL40_PAD:PLLIN_TO_PLLOUTGLOBAL_EXTERNAL");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREA_SIMPLE");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREA_DELAY");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREA_PHASE_AND_DELAY");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREA_EXTERNAL");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALA_SIMPLE");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALA_DELAY");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALA_PHASE_AND_DELAY");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALA_EXTERNAL");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREB_SIMPLE");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREB_DELAY");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREB_PHASE_AND_DELAY");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTCOREB_EXTERNAL");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALB_SIMPLE");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALB_DELAY");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALB_PHASE_AND_DELAY");
            collector.want("PLL40_2_PAD:PLLIN_TO_PLLOUTGLOBALB_EXTERNAL");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREA_SIMPLE");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREA_DELAY");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREA_PHASE_AND_DELAY");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREA_EXTERNAL");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALA_SIMPLE");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALA_DELAY");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALA_PHASE_AND_DELAY");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALA_EXTERNAL");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREB_SIMPLE");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREB_DELAY");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREB_PHASE_AND_DELAY");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTCOREB_EXTERNAL");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALB_SIMPLE");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALB_DELAY");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALB_PHASE_AND_DELAY");
            collector.want("PLL40_2F_PAD:PLLIN_TO_PLLOUTGLOBALB_EXTERNAL");
        }
    }

    // BRAM
    if kind != ChipKind::Ice40P03 {
        collector.want("BRAM:RCLK_TO_RDATA");
        collector.want("BRAM:RADDR_SETUPHOLD_RCLK");
        collector.want("BRAM:RE_SETUPHOLD_RCLK");
        collector.want("BRAM:RCLKE_SETUPHOLD_RCLK");
        collector.want("BRAM:WADDR_SETUPHOLD_WCLK");
        collector.want("BRAM:WDATA_SETUPHOLD_WCLK");
        collector.want("BRAM:MASK_SETUPHOLD_WCLK");
        collector.want("BRAM:WE_SETUPHOLD_WCLK");
        collector.want("BRAM:WCLKE_SETUPHOLD_WCLK");
        if kind.is_ice40() {
            collector.want("BRAM:CASCADE");
        }
    }

    // SPI, I2C
    if matches!(
        kind,
        ChipKind::Ice40R04 | ChipKind::Ice40T04 | ChipKind::Ice40T05
    ) {
        for pin in [
            "SBADRI0", "SBADRI1", "SBADRI2", "SBADRI3", "SBADRI4", "SBADRI5", "SBADRI6", "SBADRI7",
            "SBDATI0", "SBDATI1", "SBDATI2", "SBDATI3", "SBDATI4", "SBDATI5", "SBDATI6", "SBDATI7",
            "SBRWI", "SBSTBI",
        ] {
            collector.want(format!("SPI:{pin}_SETUPHOLD_SBCLKI"));
        }
        for pin in [
            "SBACKO", "SBDATO0", "SBDATO1", "SBDATO2", "SBDATO3", "SBDATO4", "SBDATO5", "SBDATO6",
            "SBDATO7", "SPIIRQ", "SPIWKUP",
        ] {
            collector.want(format!("SPI:SBCLKI_TO_{pin}"));
        }
        collector.want("SPI:SCKI_N_TO_SO");
        collector.want("SPI:SCKI_P_TO_SO");
        collector.want("SPI:SCKI_P_TO_SOE");
        collector.want("SPI:SCKO_N_TO_MO");
        collector.want("SPI:SCKO_P_TO_MO");
        collector.want("SPI:SCKO_P_TO_MOE");
        collector.want("SPI:SCKO_P_TO_MCSNO0");
        collector.want("SPI:SCKO_P_TO_MCSNO1");
        collector.want("SPI:SCKO_P_TO_MCSNO2");
        collector.want("SPI:SCKO_P_TO_MCSNO3");
        collector.want("SPI:SCKO_P_TO_MCSNOE0");
        collector.want("SPI:SCKO_P_TO_MCSNOE1");
        collector.want("SPI:SCKO_P_TO_MCSNOE2");
        collector.want("SPI:SCKO_P_TO_MCSNOE3");
        collector.want("SPI:MI_SETUPHOLD_SCKO_N");
        collector.want("SPI:MI_SETUPHOLD_SCKO_P");
        collector.want("SPI:SCSNI_SETUPHOLD_SCKI_N");
        collector.want("SPI:SCSNI_SETUPHOLD_SCKI_P");
        collector.want("SPI:SI_SETUPHOLD_SCKI_N");
        collector.want("SPI:SI_SETUPHOLD_SCKI_P");

        for pin in [
            "SBADRI0", "SBADRI1", "SBADRI2", "SBADRI3", "SBADRI4", "SBADRI5", "SBADRI6", "SBADRI7",
            "SBDATI0", "SBDATI1", "SBDATI2", "SBDATI3", "SBDATI4", "SBDATI5", "SBDATI6", "SBDATI7",
            "SBRWI", "SBSTBI",
        ] {
            collector.want(format!("I2C:{pin}_SETUPHOLD_SBCLKI"));
        }
        for pin in [
            "SBACKO", "SBDATO0", "SBDATO1", "SBDATO2", "SBDATO3", "SBDATO4", "SBDATO5", "SBDATO6",
            "SBDATO7", "I2CIRQ", "I2CWKUP",
        ] {
            collector.want(format!("I2C:SBCLKI_TO_{pin}"));
        }
        collector.want("I2C:SCLI_N_TO_SDAO");
        collector.want("I2C:SCLI_N_TO_SDAOE");
        collector.want("I2C:SCLI_P_TO_SDAO");
        collector.want("I2C:SCLI_P_TO_SDAOE");
        collector.want("I2C:SCLO_N_TO_SDAO");
        collector.want("I2C:SCLO_N_TO_SDAOE");
        collector.want("I2C:SCLO_P_TO_SDAO");
        collector.want("I2C:SCLO_P_TO_SDAOE");
        collector.want("I2C:SDAI_SETUPHOLD_SCLI_P");
        collector.want("I2C:SDAI_SETUPHOLD_SCLO_P");
    }
    if kind == ChipKind::Ice40T01 {
        for pin in [
            "ADRI0", "ADRI1", "ADRI2", "ADRI3", "CSI", "DATI0", "DATI1", "DATI2", "DATI3", "DATI4",
            "DATI5", "DATI6", "DATI7", "DATI8", "DATI9", "FIFORST", "STBI", "WEI",
        ] {
            collector.want(format!("I2C_FIFO:{pin}_SETUPHOLD_CLKI"));
        }
        for pin in [
            "ACKO",
            "DATO0",
            "DATO1",
            "DATO2",
            "DATO3",
            "DATO4",
            "DATO5",
            "DATO6",
            "DATO7",
            "DATO8",
            "DATO9",
            "RXFIFOAFULL",
            "RXFIFOEMPTY",
            "RXFIFOFULL",
            "SCLO",
            "SCLOE",
            "SRWO",
            "TXFIFOAEMPTY",
            "TXFIFOEMPTY",
            "TXFIFOFULL",
        ] {
            collector.want(format!("I2C_FIFO:CLKI_TO_{pin}"));
        }
        collector.want("I2C_FIFO:SCLI_N_TO_SDAO");
        collector.want("I2C_FIFO:SCLI_N_TO_SDAOE");
        collector.want("I2C_FIFO:SCLI_P_TO_SDAO");
        collector.want("I2C_FIFO:SCLI_P_TO_SDAOE");
        collector.want("I2C_FIFO:SCLO_N_TO_SDAO");
        collector.want("I2C_FIFO:SCLO_N_TO_SDAOE");
        collector.want("I2C_FIFO:SCLO_P_TO_SDAO");
        collector.want("I2C_FIFO:SCLO_P_TO_SDAOE");
        collector.want("I2C_FIFO:SDAI_SETUPHOLD_SCLI_P");
        collector.want("I2C_FIFO:SDAI_SETUPHOLD_SCLO_P");
    }

    // LEDD_IP
    if kind == ChipKind::Ice40T04 {
        collector.want("LEDD_IP:LEDDCLK_TO_LEDDON");
        collector.want("LEDD_IP:LEDDCLK_TO_PWMOUT0");
        collector.want("LEDD_IP:LEDDCLK_TO_PWMOUT1");
        collector.want("LEDD_IP:LEDDCLK_TO_PWMOUT2");
        for i in 0..4 {
            collector.want(format!("LEDD_IP:LEDDADDR{i}_SETUPHOLD_LEDDCLK"));
        }
        for i in 0..8 {
            collector.want(format!("LEDD_IP:LEDDDAT{i}_SETUPHOLD_LEDDCLK"));
        }
        collector.want("LEDD_IP:LEDDCS_SETUPHOLD_LEDDCLK");
        collector.want("LEDD_IP:LEDDDEN_SETUPHOLD_LEDDCLK");
        collector.want("LEDD_IP:LEDDEXE_SETUPHOLD_LEDDCLK");
    }

    // LEDDA_IP
    if matches!(kind, ChipKind::Ice40T01 | ChipKind::Ice40T05) {
        collector.want("LEDDA_IP:LEDDCLK_TO_LEDDON");
        collector.want("LEDDA_IP:LEDDCLK_TO_PWMOUT0");
        collector.want("LEDDA_IP:LEDDCLK_TO_PWMOUT1");
        collector.want("LEDDA_IP:LEDDCLK_TO_PWMOUT2");
        for i in 0..4 {
            collector.want(format!("LEDDA_IP:LEDDADDR{i}_SETUPHOLD_LEDDCLK"));
        }
        for i in 0..8 {
            collector.want(format!("LEDDA_IP:LEDDDAT{i}_SETUPHOLD_LEDDCLK"));
        }
        collector.want("LEDDA_IP:LEDDCS_SETUPHOLD_LEDDCLK");
        collector.want("LEDDA_IP:LEDDDEN_SETUPHOLD_LEDDCLK");
        collector.want("LEDDA_IP:LEDDEXE_SETUPHOLD_LEDDCLK");
    }

    // IR_IP
    if kind == ChipKind::Ice40T01 {
        collector.want("IR_IP:CLKI_TO_BUSY");
        collector.want("IR_IP:CLKI_TO_DRDY");
        collector.want("IR_IP:CLKI_TO_ERR");
        collector.want("IR_IP:CLKI_TO_IROUT");
        for i in 0..8 {
            collector.want(format!("IR_IP:CLKI_TO_RDATA{i}"));
        }
        for i in 0..4 {
            collector.want(format!("IR_IP:ADRI{i}_SETUPHOLD_CLKI"));
        }
        for i in 0..8 {
            collector.want(format!("IR_IP:WDATA{i}_SETUPHOLD_CLKI"));
        }
        collector.want("IR_IP:CSI_SETUPHOLD_CLKI");
        collector.want("IR_IP:DENI_SETUPHOLD_CLKI");
        collector.want("IR_IP:EXE_SETUPHOLD_CLKI");
        collector.want("IR_IP:LEARN_SETUPHOLD_CLKI");
        collector.want("IR_IP:WEI_SETUPHOLD_CLKI");
    }

    // SPRAM
    if kind == ChipKind::Ice40T05 {
        collector.want("SPRAM:CLOCK_TO_DATAOUT");
        collector.want("SPRAM:SLEEP_TO_DATAOUT");
        collector.want("SPRAM:SLEEP_SETUPHOLD_CLOCK");
        collector.want("SPRAM:STANDBY_SETUPHOLD_CLOCK");
        collector.want("SPRAM:DATAIN_SETUPHOLD_CLOCK");
        collector.want("SPRAM:ADDRESS_SETUPHOLD_CLOCK");
        collector.want("SPRAM:CHIPSELECT_SETUPHOLD_CLOCK");
        collector.want("SPRAM:MASKWREN_SETUPHOLD_CLOCK");
        collector.want("SPRAM:WREN_SETUPHOLD_CLOCK");
    }

    // FILTER
    if kind == ChipKind::Ice40T05 {
        collector.want("FILTER:IN_TO_OUT");
    }

    // DRV
    if kind == ChipKind::Ice40T04 {
        collector.want("LED_DRV_CUR:EN_TO_LEDPU");
        collector.want("IR_DRV:IRPWM_TO_IRLED");
        collector.want("RGB_DRV:RGB0PWM_TO_RGB0");
        collector.want("RGB_DRV:RGB1PWM_TO_RGB1");
        collector.want("RGB_DRV:RGB2PWM_TO_RGB2");
    }
    if matches!(kind, ChipKind::Ice40T05 | ChipKind::Ice40T01) {
        collector.want("RGBA_DRV:RGBLEDEN_TO_RGB0");
        collector.want("RGBA_DRV:RGBLEDEN_TO_RGB1");
        collector.want("RGBA_DRV:RGBLEDEN_TO_RGB2");
        collector.want("RGBA_DRV:RGB0PWM_TO_RGB0");
        collector.want("RGBA_DRV:RGB1PWM_TO_RGB1");
        collector.want("RGBA_DRV:RGB2PWM_TO_RGB2");
    }
    if kind == ChipKind::Ice40T01 {
        collector.want("IR400_DRV:IRLEDEN_TO_IRLED");
        collector.want("IR400_DRV:IRPWM_TO_IRLED");
        collector.want("BARCODE_DRV:BARCODEEN_TO_BARCODE");
        collector.want("BARCODE_DRV:BARCODEPWM_TO_BARCODE");
        collector.want("IR500_DRV:IRLEDEN_TO_IRLED1");
        collector.want("IR500_DRV:IRPWM_TO_IRLED1");
        collector.want("IR500_DRV:IRLEDEN2_TO_IRLED2");
        collector.want("IR500_DRV:IRPWM2_TO_IRLED2");
    }

    collector
}

pub fn get_speed_data(design: &Design, run: &RunResult) -> SpeedCollector {
    let mut res = SpeedCollector::new();
    for (inst, cell) in run
        .sdf
        .cells_by_name
        .iter()
        .map(|(key, val)| (Some(key.as_str()), val))
        .chain(run.sdf.cells_by_type.values().map(|val| (None, val)))
    {
        match cell.typ.as_str() {
            "InMux" => collect_int(&mut res, "INT:IMUX_LC", cell),
            "IoInMux" => collect_int(&mut res, "INT:IMUX_IO", cell),
            "ClkMux" => collect_int(&mut res, "INT:IMUX_CLK", cell),
            "CEMux" => collect_int(&mut res, "INT:IMUX_CE", cell),
            "SRMux" => collect_int(&mut res, "INT:IMUX_RST", cell),
            "LocalMux" => collect_int(&mut res, "INT:LOCAL", cell),
            "Glb2LocalMux" => collect_int(&mut res, "INT:GOUT", cell),
            "GlobalMux" => collect_int(&mut res, "INT:GLOBAL", cell),
            "Span12Mux_s0_v" => collect_int(&mut res, "INT:LONG_V_0", cell),
            "Span12Mux_s1_v" => collect_int(&mut res, "INT:LONG_V_1", cell),
            "Span12Mux_s2_v" => collect_int(&mut res, "INT:LONG_V_2", cell),
            "Span12Mux_s3_v" => collect_int(&mut res, "INT:LONG_V_3", cell),
            "Span12Mux_s4_v" => collect_int(&mut res, "INT:LONG_V_4", cell),
            "Span12Mux_s5_v" => collect_int(&mut res, "INT:LONG_V_5", cell),
            "Span12Mux_s6_v" => collect_int(&mut res, "INT:LONG_V_6", cell),
            "Span12Mux_s7_v" => collect_int(&mut res, "INT:LONG_V_7", cell),
            "Span12Mux_s8_v" => collect_int(&mut res, "INT:LONG_V_8", cell),
            "Span12Mux_s9_v" => collect_int(&mut res, "INT:LONG_V_9", cell),
            "Span12Mux_s10_v" => collect_int(&mut res, "INT:LONG_V_10", cell),
            "Span12Mux_s11_v" => collect_int(&mut res, "INT:LONG_V_11", cell),
            "Span12Mux_v" => collect_int(&mut res, "INT:LONG_V_12", cell),
            "Span12Mux_s0_h" => collect_int(&mut res, "INT:LONG_H_0", cell),
            "Span12Mux_s1_h" => collect_int(&mut res, "INT:LONG_H_1", cell),
            "Span12Mux_s2_h" => collect_int(&mut res, "INT:LONG_H_2", cell),
            "Span12Mux_s3_h" => collect_int(&mut res, "INT:LONG_H_3", cell),
            "Span12Mux_s4_h" => collect_int(&mut res, "INT:LONG_H_4", cell),
            "Span12Mux_s5_h" => collect_int(&mut res, "INT:LONG_H_5", cell),
            "Span12Mux_s6_h" => collect_int(&mut res, "INT:LONG_H_6", cell),
            "Span12Mux_s7_h" => collect_int(&mut res, "INT:LONG_H_7", cell),
            "Span12Mux_s8_h" => collect_int(&mut res, "INT:LONG_H_8", cell),
            "Span12Mux_s9_h" => collect_int(&mut res, "INT:LONG_H_9", cell),
            "Span12Mux_s10_h" => collect_int(&mut res, "INT:LONG_H_10", cell),
            "Span12Mux_s11_h" => collect_int(&mut res, "INT:LONG_H_11", cell),
            "Span12Mux_h" => collect_int(&mut res, "INT:LONG_H_12", cell),
            "Span12Mux" => collect_int(&mut res, "INT:LONG", cell),
            "Span4Mux_s0_v" => collect_int(&mut res, "INT:QUAD_V_0", cell),
            "Span4Mux_s1_v" => collect_int(&mut res, "INT:QUAD_V_1", cell),
            "Span4Mux_s2_v" => collect_int(&mut res, "INT:QUAD_V_2", cell),
            "Span4Mux_s3_v" => collect_int(&mut res, "INT:QUAD_V_3", cell),
            "Span4Mux_v" => collect_int(&mut res, "INT:QUAD_V_4", cell),
            "Span4Mux_s0_h" => collect_int(&mut res, "INT:QUAD_H_0", cell),
            "Span4Mux_s1_h" => collect_int(&mut res, "INT:QUAD_H_1", cell),
            "Span4Mux_s2_h" => collect_int(&mut res, "INT:QUAD_H_2", cell),
            "Span4Mux_s3_h" => collect_int(&mut res, "INT:QUAD_H_3", cell),
            "Span4Mux_h" => collect_int(&mut res, "INT:QUAD_H_4", cell),
            "Span4Mux" => collect_int(&mut res, "INT:QUAD", cell),
            "IoSpan4Mux" => collect_int(&mut res, "INT:QUAD_IO", cell),
            "Odrv4" => collect_int(&mut res, "INT:OUT_TO_QUAD", cell),
            "Odrv12" => collect_int(&mut res, "INT:OUT_TO_LONG", cell),
            "Sp12to4" => collect_int(&mut res, "INT:LONG_TO_QUAD", cell),

            // PLB
            "LogicCell2" | "LogicCell40" => collect_lc(&mut res, cell),
            "ICE_CARRY_IN_MUX" => collect_carry_init(&mut res, cell),

            // globals
            "ICE_GB" => collect_gb_fabric(&mut res, cell),

            // IO (ice65)
            "ICE_IO" | "ICE_GB_IO" => collect_ice_io(&mut res, cell, design),

            // IO (ice40)
            "IO_PAD" | "IO_PAD_I3C" | "IO_PAD_OD" => {
                // (ignored)
            }
            _ if cell.typ.starts_with("PRE_IO") => collect_pre_io(&mut res, cell),
            "SB_IO_OD" => {
                // (ignored)
            }

            // BRAM
            "SB_RAM4K" => collect_simple(&mut res, "BRAM", cell),
            "SB_RAM40_4K" => collect_simple(&mut res, "BRAM", cell),
            "CascadeBuf" => collect_int(&mut res, "BRAM:CASCADE", cell),
            // SPRAM
            "SB_SPRAM256KA" => collect_simple(&mut res, "SPRAM", cell),
            // hard logic
            "SB_SPI" => collect_simple(&mut res, "SPI", cell),
            "SB_I2C" | "SB_I2C_FIFO" => {
                let inst =
                    InstId::from_idx(inst.unwrap().strip_prefix('i').unwrap().parse().unwrap());
                let inst = &design.insts[inst];
                if let Some(val) = inst.props.get("SDA_INPUT_DELAYED") {
                    if val == "1" {
                        continue;
                    }
                }
                if let Some(val) = inst.props.get("SDA_OUTPUT_DELAYED") {
                    if val == "1" {
                        continue;
                    }
                }
                collect_simple(&mut res, &cell.typ[3..], cell);
            }
            "SB_LEDD_IP" => collect_simple(&mut res, "LEDD_IP", cell),
            "SB_LEDDA_IP" => collect_simple(&mut res, "LEDDA_IP", cell),
            "SB_IR_IP" => collect_simple(&mut res, "IR_IP", cell),
            "SB_FILTER_50NS" => collect_filter(&mut res, cell),

            // PLL
            "SB_PLL_CORE" | "SB_PLL_PAD" | "SB_PLL_2_PAD" | "SB_PLL40_CORE"
            | "SB_PLL40_2F_CORE" | "PLL40" | "PLL40_2" | "PLL40_2F" => {
                let inst = if cell.typ.starts_with("PLL40") {
                    if inst.is_none() {
                        continue;
                    }
                    InstId::from_idx(
                        inst.unwrap()
                            .strip_prefix('i')
                            .unwrap()
                            .strip_suffix("_pll")
                            .unwrap()
                            .parse()
                            .unwrap(),
                    )
                } else {
                    InstId::from_idx(inst.unwrap().strip_prefix('i').unwrap().parse().unwrap())
                };
                collect_pll(&mut res, cell, &design.insts[inst]);
            }
            "PLL40_FEEDBACK_PATH_DELAY"
            | "PLL40_FEEDBACK_PATH_EXTERNAL"
            | "PLL40_FEEDBACK_PATH_PHASE_AND_DELAY"
            | "PLL40_FEEDBACK_PATH_SIMPLE"
            | "PLL40_2_FEEDBACK_PATH_DELAY"
            | "PLL40_2_FEEDBACK_PATH_EXTERNAL"
            | "PLL40_2_FEEDBACK_PATH_PHASE_AND_DELAY"
            | "PLL40_2_FEEDBACK_PATH_SIMPLE"
            | "PLL40_2F_FEEDBACK_PATH_DELAY"
            | "PLL40_2F_FEEDBACK_PATH_EXTERNAL"
            | "PLL40_2F_FEEDBACK_PATH_PHASE_AND_DELAY"
            | "PLL40_2F_FEEDBACK_PATH_SIMPLE" => {
                // (ignore)
            }

            // LED drivers
            "SB_LED_DRV_CUR" => collect_drv(&mut res, "LED_DRV_CUR", cell),
            "SB_RGB_DRV" => collect_drv(&mut res, "RGB_DRV", cell),
            "SB_IR_DRV" => collect_drv(&mut res, "IR_DRV", cell),
            "SB_RGBA_DRV" => collect_drv(&mut res, "RGBA_DRV", cell),
            "SB_IR400_DRV" => collect_drv(&mut res, "IR400_DRV", cell),
            "SB_BARCODE_DRV" => collect_drv(&mut res, "BARCODE_DRV", cell),
            "ICE_IR500_DRV" => collect_drv(&mut res, "IR500_DRV", cell),

            // junk
            "LUT_MUX" | "ADTTRIBUF" | "GIOBUG" => {
                // TODO: junk?
            }
            "gio2CtrlBuf" | "CascadeMux" | "DummyBuf" | "INV" | "TRIBUF" | "MUX4" | "DL"
            | "sync_clk_enable" => {
                collect_null(cell);
            }

            _ => {
                println!("unknown cell: {typ} {inst:?}", typ = cell.typ);
                for path in &cell.iopath {
                    println!("  IOPATH {path:?}");
                }
                for port in &cell.ports {
                    println!("  PORT {port:?}");
                }
                for setuphold in &cell.setuphold {
                    println!("  SETUPHOLD {setuphold:?}");
                }
                for recrem in &cell.recrem {
                    println!("  RECREM {recrem:?}");
                }
                for period in &cell.period {
                    println!("  PERIOD {period:?}");
                }
                for width in &cell.width {
                    println!("  WIDTH {width:?}");
                }
            }
        }
    }
    res
}

pub fn finish_speed(mut collector: SpeedCollector) -> Speed {
    for key in collector.db.vals.keys() {
        if !collector.wanted_keys.contains(key) {
            println!("KEY {key} NOT WANTED?!?");
        }
    }
    let SpeedVal::Delay(rise) = collector.db.vals.remove("PLB:RST_TO_O:RISE").unwrap() else {
        unreachable!()
    };
    let SpeedVal::Delay(fall) = collector.db.vals.remove("PLB:RST_TO_O:FALL").unwrap() else {
        unreachable!()
    };
    collector.insert(
        "PLB:RST_TO_O",
        SpeedVal::DelayRfFromEdge(DelayRfUnate { rise, fall }),
    );
    collector.db
}
