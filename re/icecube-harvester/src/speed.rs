use std::collections::{BTreeMap, BTreeSet, btree_map};

use prjcombine_re_sdf::{Cell, Edge, IoPath};
use prjcombine_siliconblue::chip::ChipKind;
use prjcombine_types::speed::{
    Delay, DelayRfBinate, DelayRfFromEdge, DelayRfUnate, RecRem, SetupHoldRf, Speed, SpeedVal, Time,
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
    min: Time(0.0),
    typ: Time(0.0),
    max: Time(0.0),
};

fn convert_delay(del: prjcombine_re_sdf::Delay) -> Delay {
    Delay {
        min: del.min,
        max: del.max,
    }
}

fn convert_delay_rf_unate(iopath: &IoPath) -> DelayRfUnate {
    DelayRfUnate {
        rise: convert_delay(iopath.del_rise),
        fall: convert_delay(iopath.del_fall),
    }
}

fn convert_delay_rf_from_edge(iopath: &IoPath) -> DelayRfFromEdge {
    DelayRfFromEdge {
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
    collector.insert(name, SpeedVal::DelayRfUnate(delay));
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
                    collector.insert(name, SpeedVal::DelayRfUnate(delay));
                } else {
                    let delay = convert_delay_rf_binate(path);
                    collector.insert(name, SpeedVal::DelayRfBinate(delay));
                }
            }
            Edge::Posedge(port_from) => {
                assert_eq!(port_from, "clk");
                assert_eq!(port_to, "lcout");
                let delay = convert_delay_rf_from_edge(path);
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
                data.0 = Some(delay.max);
            } else {
                data.1 = Some(delay.max);
            }
        }
        if let Some(hold) = sh.hold {
            let delay = convert_delay(hold);
            // sigh.
            let delay = std::cmp::max(delay.max, delay.min);
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
                            recovery: delay.max,
                            removal: Time(0.0),
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
    collector.insert("PLB:CARRY_INIT", SpeedVal::DelayRfUnate(delay));
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
    collector.insert("GB_FABRIC", SpeedVal::DelayRfUnate(delay));
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

fn collect_simple(collector: &mut SpeedCollector, kind: &str, cell: &Cell) {
    for path in &cell.iopath {
        // println!("IOPATH {kind} {path:?}");
        let Edge::Plain(port_to) = &path.port_to else {
            unreachable!()
        };
        let port_to = strip_index(port_to);
        let Edge::Posedge(port_from) = &path.port_from else {
            unreachable!()
        };
        let delay = convert_delay_rf_from_edge(path);
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
            let delay = convert_delay(setup);
            if is_rise {
                data.0 = Some(delay.max);
            } else {
                data.1 = Some(delay.max);
            }
        }
        if let Some(hold) = sh.hold {
            let delay = convert_delay(hold);
            // sigh.
            let delay = std::cmp::max(delay.max, delay.min);
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

pub fn want_speed_data(collector: &mut SpeedCollector, kind: ChipKind) {
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
}

pub fn get_speed_data(design: &Design, run: &RunResult) -> SpeedCollector {
    let mut res = SpeedCollector::new();
    for cell in run
        .sdf
        .cells_by_name
        .values()
        .chain(run.sdf.cells_by_type.values())
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
            "SB_RAM4K" => collect_simple(&mut res, "BRAM", cell),
            "SB_RAM40_4K" => collect_simple(&mut res, "BRAM", cell),
            "CascadeBuf" => collect_int(&mut res, "BRAM:CASCADE", cell),
            // SPRAM
            "SB_SPRAM256KA" => collect_simple(&mut res, "SPRAM", cell),
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
            "SB_LEDD_IP" => collect_simple(&mut res, "LEDD_IP", cell),
            "SB_LEDDA_IP" => collect_simple(&mut res, "LEDDA_IP", cell),
            "SB_IR_IP" => collect_simple(&mut res, "IR_IP", cell),
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
        SpeedVal::DelayRfFromEdge(DelayRfFromEdge { rise, fall }),
    );
    collector.db
}
