use std::collections::{BTreeMap, BTreeSet, btree_map};

use prjcombine_re_sdf::{Cell, Edge, IoPath};
use prjcombine_siliconblue::chip::ChipKind;
use prjcombine_types::{
    speed::{
        DelayRfBinate, DelayRfUnate, DerateFactorTemperatureLinear,
        DerateFactorVoltageInvQuadratic, RecRem, SetupHoldRf, Speed, SpeedVal,
    },
    units::{Scalar, Temperature, Time, Voltage},
};

use crate::run::{Design, RunResult};

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
                assert_eq!(*entry.get(), val);
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

#[derive(Copy, Clone, Debug)]
struct DerateFactors {
    min: Scalar,
    typ: Scalar,
    max: Scalar,
}

fn convert_delay(del: prjcombine_re_sdf::Delay, der: DerateFactors) -> Time {
    let v0 = del.min / der.min;
    let v1 = del.typ / der.typ;
    let v2 = del.max / der.max;
    let mut scale = 1.0;
    let v = (v0.0.0 + v1.0.0 + v2.0.0) / 3.0;
    if v != 0.0 {
        while (v * scale).abs() < 100000.0 {
            scale *= 10.0;
        }
        while (v * scale).abs() >= 1000000.0 {
            scale /= 10.0;
        }
    }
    let vs = (v * scale).round();
    let v = vs / scale;
    // let v0s = v0.0.0 * scale;
    // let v1s = v1.0.0 * scale;
    // let v2s = v2.0.0 * scale;
    // if (vs - v0s).abs() >= 0.7 || (vs - v1s).abs() >= 0.7 || (vs - v2s).abs() >= 0.7 {
    //     println!("MEOW {v} {vs} {v0s} {v1s} {v2s}");
    // }
    Time(v.into())
}

fn convert_delay_rf_unate(iopath: &IoPath, der: DerateFactors) -> DelayRfUnate {
    DelayRfUnate {
        rise: convert_delay(iopath.del_rise, der),
        fall: convert_delay(iopath.del_fall, der),
    }
}

fn convert_delay_rf_binate(iopath: &IoPath, der: DerateFactors) -> DelayRfBinate {
    DelayRfBinate {
        rise_to_rise: convert_delay(iopath.del_rise, der),
        rise_to_fall: convert_delay(iopath.del_fall, der),
        fall_to_rise: convert_delay(iopath.del_rise, der),
        fall_to_fall: convert_delay(iopath.del_fall, der),
    }
}

fn collect_int(collector: &mut SpeedCollector, name: &str, cell: &Cell, der: DerateFactors) {
    assert_eq!(cell.iopath.len(), 1);
    let iopath = &cell.iopath[0];
    assert_eq!(iopath.port_from, Edge::Plain("I".into()));
    assert_eq!(iopath.port_to, Edge::Plain("O".into()));
    let delay = convert_delay_rf_unate(iopath, der);
    collector.insert(name, SpeedVal::DelayRfPosUnate(delay));
    assert!(cell.ports.is_empty());
    assert!(cell.setuphold.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_lc(collector: &mut SpeedCollector, cell: &Cell, der: DerateFactors) {
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
                        let delay = convert_delay(path.del_rise, der);
                        collector.insert(format!("{name}:RISE"), SpeedVal::Delay(delay));
                    }
                    if path.del_fall != ZERO {
                        let delay = convert_delay(path.del_fall, der);
                        collector.insert(format!("{name}:FALL"), SpeedVal::Delay(delay));
                    }
                } else if port_to == "carryout" {
                    let delay = convert_delay_rf_unate(path, der);
                    collector.insert(name, SpeedVal::DelayRfPosUnate(delay));
                } else {
                    let delay = convert_delay_rf_binate(path, der);
                    collector.insert(name, SpeedVal::DelayRfBinate(delay));
                }
            }
            Edge::Posedge(port_from) => {
                assert_eq!(port_from, "clk");
                assert_eq!(port_to, "lcout");
                let delay = convert_delay_rf_unate(path, der);
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
            let delay = convert_delay(setup, der);
            if is_rise {
                data.0 = Some(delay);
            } else {
                data.1 = Some(delay);
            }
        }
        if let Some(hold) = sh.hold {
            let delay = convert_delay(hold, der);
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
                    let delay = convert_delay(recovery, der);
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

fn collect_carry_init(collector: &mut SpeedCollector, cell: &Cell, der: DerateFactors) {
    assert_eq!(cell.iopath.len(), 1);
    let iopath = &cell.iopath[0];
    assert_eq!(iopath.port_from, Edge::Plain("carryinitin".into()));
    assert_eq!(iopath.port_to, Edge::Plain("carryinitout".into()));
    let delay = convert_delay_rf_unate(iopath, der);
    collector.insert("PLB:CARRY_INIT", SpeedVal::DelayRfPosUnate(delay));
    assert!(cell.ports.is_empty());
    assert!(cell.setuphold.is_empty());
    assert!(cell.recrem.is_empty());
    assert!(cell.period.is_empty());
    assert!(cell.width.is_empty());
}

fn collect_gb_fabric(collector: &mut SpeedCollector, cell: &Cell, der: DerateFactors) {
    assert_eq!(cell.iopath.len(), 1);
    let iopath = &cell.iopath[0];
    assert_eq!(
        iopath.port_from,
        Edge::Plain("USERSIGNALTOGLOBALBUFFER".into())
    );
    assert_eq!(iopath.port_to, Edge::Plain("GLOBALBUFFEROUTPUT".into()));
    let delay = convert_delay_rf_unate(iopath, der);
    collector.insert("GB_FABRIC", SpeedVal::DelayRfPosUnate(delay));
    assert!(cell.ports.is_empty());
    assert!(cell.setuphold.is_empty());
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

fn collect_simple(collector: &mut SpeedCollector, kind: &str, cell: &Cell, der: DerateFactors) {
    for path in &cell.iopath {
        // println!("IOPATH {kind} {path:?}");
        let Edge::Plain(port_to) = &path.port_to else {
            unreachable!()
        };
        let port_to = strip_index(port_to);
        let Edge::Posedge(port_from) = &path.port_from else {
            unreachable!()
        };
        let delay = convert_delay_rf_unate(path, der);
        collector.insert(
            format!("{kind}:{port_from}_TO_{port_to}"),
            SpeedVal::DelayRfFromEdge(delay),
        );
    }
    let mut setuphold = BTreeMap::new();
    for sh in &cell.setuphold {
        // println!("SETUPHOLD {kind} {sh:?}");
        let Edge::Posedge(port_c) = &sh.edge_c else {
            unreachable!()
        };
        let (is_rise, port_d) = match &sh.edge_d {
            Edge::Posedge(port) => (true, port),
            Edge::Negedge(port) => (false, port),
            _ => unreachable!(),
        };
        let port_d = strip_index(port_d);
        let data = setuphold
            .entry((port_d, port_c))
            .or_insert((None, None, None, None));
        if let Some(setup) = sh.setup {
            let delay = convert_delay(setup, der);
            if is_rise {
                data.0 = Some(delay);
            } else {
                data.1 = Some(delay);
            }
        }
        if let Some(hold) = sh.hold {
            let delay = convert_delay(hold, der);
            if is_rise {
                data.2 = Some(delay);
            } else {
                data.3 = Some(delay);
            }
        }
    }
    for ((port_d, port_c), (sr, sf, hr, hf)) in setuphold {
        collector.insert(
            format!("{kind}:{port_d}_SETUPHOLD_{port_c}"),
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

    collector
}

fn get_der_factors(design: &Design) -> DerateFactors {
    // bug: without a project file, icecube always uses "L" grade for derating
    // factor computation (but loads the T speed file as appropriate).
    let speed = init_speed_data(design.kind, &design.device, "L");
    let factors = [
        ("BEST", 1.26, 0.0),
        ("TYP", 1.2, 25.0),
        ("WORST", 1.14, 85.0),
    ]
    .map(|(c, v, t)| {
        let pf = *speed
            .db
            .vals
            .get("DERATE_P")
            .unwrap_or_else(|| speed.db.vals.get(&format!("DERATE_P_{c}")).unwrap());
        let vf = speed.db.vals["DERATE_V"];
        let tf = *speed
            .db
            .vals
            .get("DERATE_T")
            .unwrap_or_else(|| speed.db.vals.get(&format!("DERATE_T_{c}")).unwrap());
        let SpeedVal::Scalar(pf) = pf else {
            unreachable!()
        };
        let SpeedVal::DerateFactorVoltageInvQuadratic(vf) = vf else {
            unreachable!()
        };
        let SpeedVal::DerateFactorTemperatureLinear(tf) = tf else {
            unreachable!()
        };
        let vf = vf.eval(Voltage(v.into()));
        let tf = tf.eval(Temperature(t.into()));
        pf * vf * tf
    });
    DerateFactors {
        min: factors[0],
        typ: factors[1],
        max: factors[2],
    }
}

pub fn get_speed_data(design: &Design, run: &RunResult) -> SpeedCollector {
    let der = get_der_factors(design);

    let mut res = SpeedCollector::new();
    for cell in run
        .sdf
        .cells_by_name
        .values()
        .chain(run.sdf.cells_by_type.values())
    {
        match cell.typ.as_str() {
            "InMux" => collect_int(&mut res, "INT:IMUX_LC", cell, der),
            "IoInMux" => collect_int(&mut res, "INT:IMUX_IO", cell, der),
            "ClkMux" => collect_int(&mut res, "INT:IMUX_CLK", cell, der),
            "CEMux" => collect_int(&mut res, "INT:IMUX_CE", cell, der),
            "SRMux" => collect_int(&mut res, "INT:IMUX_RST", cell, der),
            "LocalMux" => collect_int(&mut res, "INT:LOCAL", cell, der),
            "Glb2LocalMux" => collect_int(&mut res, "INT:GOUT", cell, der),
            "GlobalMux" => collect_int(&mut res, "INT:GLOBAL", cell, der),
            "Span12Mux_s0_v" => collect_int(&mut res, "INT:LONG_V_0", cell, der),
            "Span12Mux_s1_v" => collect_int(&mut res, "INT:LONG_V_1", cell, der),
            "Span12Mux_s2_v" => collect_int(&mut res, "INT:LONG_V_2", cell, der),
            "Span12Mux_s3_v" => collect_int(&mut res, "INT:LONG_V_3", cell, der),
            "Span12Mux_s4_v" => collect_int(&mut res, "INT:LONG_V_4", cell, der),
            "Span12Mux_s5_v" => collect_int(&mut res, "INT:LONG_V_5", cell, der),
            "Span12Mux_s6_v" => collect_int(&mut res, "INT:LONG_V_6", cell, der),
            "Span12Mux_s7_v" => collect_int(&mut res, "INT:LONG_V_7", cell, der),
            "Span12Mux_s8_v" => collect_int(&mut res, "INT:LONG_V_8", cell, der),
            "Span12Mux_s9_v" => collect_int(&mut res, "INT:LONG_V_9", cell, der),
            "Span12Mux_s10_v" => collect_int(&mut res, "INT:LONG_V_10", cell, der),
            "Span12Mux_s11_v" => collect_int(&mut res, "INT:LONG_V_11", cell, der),
            "Span12Mux_v" => collect_int(&mut res, "INT:LONG_V_12", cell, der),
            "Span12Mux_s0_h" => collect_int(&mut res, "INT:LONG_H_0", cell, der),
            "Span12Mux_s1_h" => collect_int(&mut res, "INT:LONG_H_1", cell, der),
            "Span12Mux_s2_h" => collect_int(&mut res, "INT:LONG_H_2", cell, der),
            "Span12Mux_s3_h" => collect_int(&mut res, "INT:LONG_H_3", cell, der),
            "Span12Mux_s4_h" => collect_int(&mut res, "INT:LONG_H_4", cell, der),
            "Span12Mux_s5_h" => collect_int(&mut res, "INT:LONG_H_5", cell, der),
            "Span12Mux_s6_h" => collect_int(&mut res, "INT:LONG_H_6", cell, der),
            "Span12Mux_s7_h" => collect_int(&mut res, "INT:LONG_H_7", cell, der),
            "Span12Mux_s8_h" => collect_int(&mut res, "INT:LONG_H_8", cell, der),
            "Span12Mux_s9_h" => collect_int(&mut res, "INT:LONG_H_9", cell, der),
            "Span12Mux_s10_h" => collect_int(&mut res, "INT:LONG_H_10", cell, der),
            "Span12Mux_s11_h" => collect_int(&mut res, "INT:LONG_H_11", cell, der),
            "Span12Mux_h" => collect_int(&mut res, "INT:LONG_H_12", cell, der),
            "Span12Mux" => collect_int(&mut res, "INT:LONG", cell, der),
            "Span4Mux_s0_v" => collect_int(&mut res, "INT:QUAD_V_0", cell, der),
            "Span4Mux_s1_v" => collect_int(&mut res, "INT:QUAD_V_1", cell, der),
            "Span4Mux_s2_v" => collect_int(&mut res, "INT:QUAD_V_2", cell, der),
            "Span4Mux_s3_v" => collect_int(&mut res, "INT:QUAD_V_3", cell, der),
            "Span4Mux_v" => collect_int(&mut res, "INT:QUAD_V_4", cell, der),
            "Span4Mux_s0_h" => collect_int(&mut res, "INT:QUAD_H_0", cell, der),
            "Span4Mux_s1_h" => collect_int(&mut res, "INT:QUAD_H_1", cell, der),
            "Span4Mux_s2_h" => collect_int(&mut res, "INT:QUAD_H_2", cell, der),
            "Span4Mux_s3_h" => collect_int(&mut res, "INT:QUAD_H_3", cell, der),
            "Span4Mux_h" => collect_int(&mut res, "INT:QUAD_H_4", cell, der),
            "Span4Mux" => collect_int(&mut res, "INT:QUAD", cell, der),
            "IoSpan4Mux" => collect_int(&mut res, "INT:QUAD_IO", cell, der),
            "Odrv4" => collect_int(&mut res, "INT:OUT_TO_QUAD", cell, der),
            "Odrv12" => collect_int(&mut res, "INT:OUT_TO_LONG", cell, der),
            "Sp12to4" => collect_int(&mut res, "INT:LONG_TO_QUAD", cell, der),

            // PLB
            "LogicCell2" | "LogicCell40" => collect_lc(&mut res, cell, der),
            "ICE_CARRY_IN_MUX" => collect_carry_init(&mut res, cell, der),

            // globals
            "ICE_GB" => collect_gb_fabric(&mut res, cell, der),

            // IO (ice65)
            "ICE_IO" | "ICE_GB_IO" => {
                // TODO
            }

            // IO (ice40)
            "IO_PAD" | "IO_PAD_I3C" | "IO_PAD_OD" => {
                // TODO
            }
            _ if cell.typ.starts_with("PRE_IO") => {
                // TODO
            }
            "SB_IO_OD" => {
                // TODO
            }

            // BRAM
            "SB_RAM4K" => collect_simple(&mut res, "BRAM", cell, der),
            "SB_RAM40_4K" => collect_simple(&mut res, "BRAM", cell, der),
            "CascadeBuf" => collect_int(&mut res, "BRAM:CASCADE", cell, der),
            // SPRAM
            "SB_SPRAM256KA" => collect_simple(&mut res, "SPRAM", cell, der),
            // hard logic
            "SB_SPI" => {
                // TODO
            }
            "SB_I2C" => {
                // TODO
            }
            "SB_I2C_FIFO" => {
                // TODO
            }
            "SB_LEDD_IP" => collect_simple(&mut res, "LEDD_IP", cell, der),
            "SB_LEDDA_IP" => collect_simple(&mut res, "LEDDA_IP", cell, der),
            "SB_IR_IP" => collect_simple(&mut res, "IR_IP", cell, der),
            "SB_FILTER_50NS" => {
                // TODO
            }

            // PLL (ice65)
            "SB_PLL_CORE" | "SB_PLL_PAD" | "SB_PLL_2_PAD" => {
                // TODO
            }
            // PLL (ice40)
            "SB_PLL40_CORE"
            | "SB_PLL40_2F_CORE"
            | "PLL40"
            | "PLL40_FEEDBACK_PATH_DELAY"
            | "PLL40_FEEDBACK_PATH_EXTERNAL"
            | "PLL40_FEEDBACK_PATH_PHASE_AND_DELAY"
            | "PLL40_FEEDBACK_PATH_SIMPLE"
            | "PLL40_2"
            | "PLL40_2_FEEDBACK_PATH_DELAY"
            | "PLL40_2_FEEDBACK_PATH_EXTERNAL"
            | "PLL40_2_FEEDBACK_PATH_PHASE_AND_DELAY"
            | "PLL40_2_FEEDBACK_PATH_SIMPLE"
            | "PLL40_2F"
            | "PLL40_2F_FEEDBACK_PATH_DELAY"
            | "PLL40_2F_FEEDBACK_PATH_EXTERNAL"
            | "PLL40_2F_FEEDBACK_PATH_PHASE_AND_DELAY"
            | "PLL40_2F_FEEDBACK_PATH_SIMPLE" => {
                // TODO
            }

            // LED drivers
            "SB_LED_DRV_CUR" => {
                // TODO
            }
            "SB_RGB_DRV" => {
                // TODO
            }
            "SB_IR_DRV" => {
                // TODO
            }
            "SB_RGBA_DRV" => {
                // TODO
            }
            "SB_IR400_DRV" => {
                // TODO
            }
            "SB_BARCODE_DRV" => {
                // TODO
            }
            "ICE_IR500_DRV" => {
                // TODO
            }

            // junk
            "LUT_MUX" | "ADTTRIBUF" | "GIOBUG" => {
                // TODO: junk?
            }
            "gio2CtrlBuf" | "CascadeMux" | "DummyBuf" | "INV" | "TRIBUF" | "MUX4" | "DL"
            | "sync_clk_enable" => {
                collect_null(cell);
            }

            _ => {
                println!("unknown cell: {}", cell.typ);
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
