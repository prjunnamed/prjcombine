use std::collections::BTreeMap;
use std::fmt::Write;

use prjcombine_types::speed::{Speed, SpeedVal};

use crate::DocgenContext;

pub struct SpeedData<'a> {
    pub names: Vec<String>,
    pub speed: &'a Speed,
}

pub fn gen_speed<'a>(ctx: &mut DocgenContext, tag: &str, speeds: &[SpeedData<'a>]) {
    let mut kv = BTreeMap::new();
    let mut add_kv = |idx: usize, k: &'a str, sk: &'static str, v: String| {
        let vals = kv
            .entry(k)
            .or_insert(BTreeMap::new())
            .entry(sk)
            .or_insert_with(|| vec![None; speeds.len()]);
        vals[idx] = Some(v);
    };
    for (idx, ds) in speeds.iter().enumerate() {
        for (k, &v) in &ds.speed.vals {
            match v {
                SpeedVal::Delay(delay) => {
                    add_kv(idx, k, "", delay.to_string());
                }
                SpeedVal::DelayRange(delay) => {
                    add_kv(idx, k, "", delay.to_string());
                }
                SpeedVal::DelayRfBinate(delay) => {
                    add_kv(idx, k, "rise-to-rise", delay.rise_to_rise.to_string());
                    add_kv(idx, k, "rise-to-fall", delay.rise_to_fall.to_string());
                    add_kv(idx, k, "fall-to-rise", delay.fall_to_rise.to_string());
                    add_kv(idx, k, "fall-to-fall", delay.fall_to_fall.to_string());
                }
                SpeedVal::DelayRfBinateRange(delay) => {
                    add_kv(idx, k, "rise-to-rise", delay.rise_to_rise.to_string());
                    add_kv(idx, k, "rise-to-fall", delay.rise_to_fall.to_string());
                    add_kv(idx, k, "fall-to-rise", delay.fall_to_rise.to_string());
                    add_kv(idx, k, "fall-to-fall", delay.fall_to_fall.to_string());
                }
                SpeedVal::DelayRfPosUnate(delay) => {
                    add_kv(idx, k, "rise-to-rise", delay.rise.to_string());
                    add_kv(idx, k, "fall-to-fall", delay.fall.to_string());
                }
                SpeedVal::DelayRfPosUnateRange(delay) => {
                    add_kv(idx, k, "rise-to-rise", delay.rise.to_string());
                    add_kv(idx, k, "fall-to-fall", delay.fall.to_string());
                }
                SpeedVal::DelayRfNegUnate(delay) => {
                    add_kv(idx, k, "fall-to-rise", delay.rise.to_string());
                    add_kv(idx, k, "rise-to-fall", delay.fall.to_string());
                }
                SpeedVal::DelayRfNegUnateRange(delay) => {
                    add_kv(idx, k, "fall-to-rise", delay.rise.to_string());
                    add_kv(idx, k, "rise-to-fall", delay.fall.to_string());
                }
                SpeedVal::DelayRfFromEdge(delay) => {
                    add_kv(idx, k, "rise", delay.rise.to_string());
                    add_kv(idx, k, "fall", delay.fall.to_string());
                }
                SpeedVal::DelayRfFromEdgeRange(delay) => {
                    add_kv(idx, k, "rise", delay.rise.to_string());
                    add_kv(idx, k, "fall", delay.fall.to_string());
                }
                SpeedVal::SetupHold(sh) => {
                    add_kv(idx, k, "setup", sh.setup.to_string());
                    add_kv(idx, k, "hold", sh.hold.to_string());
                }
                SpeedVal::SetupHoldRf(sh) => {
                    add_kv(idx, k, "rise setup", sh.rise_setup.to_string());
                    add_kv(idx, k, "rise hold", sh.rise_hold.to_string());
                    add_kv(idx, k, "fall setup", sh.fall_setup.to_string());
                    add_kv(idx, k, "fall hold", sh.fall_hold.to_string());
                }
                SpeedVal::RecRem(recrem) => {
                    add_kv(idx, k, "recovery", recrem.recovery.to_string());
                    add_kv(idx, k, "removal", recrem.removal.to_string());
                }
                SpeedVal::PulseWidth(time) => {
                    add_kv(idx, k, "", time.to_string());
                }
                SpeedVal::Period(time) => {
                    add_kv(idx, k, "", time.to_string());
                }
                SpeedVal::Scalar(scalar) => {
                    add_kv(idx, k, "", scalar.to_string());
                }
                SpeedVal::DerateFactorTemperatureLinear(eq) => {
                    add_kv(idx, k, "a", eq.a.to_string());
                    add_kv(idx, k, "b", eq.b.to_string());
                }
                SpeedVal::DerateFactorVoltageInvQuadratic(eq) => {
                    add_kv(idx, k, "a", eq.a.to_string());
                    add_kv(idx, k, "b", eq.b.to_string());
                    add_kv(idx, k, "c", eq.c.to_string());
                }
                SpeedVal::Resistance(res) => {
                    add_kv(idx, k, "", res.to_string());
                }
                SpeedVal::ResistanceRf(res) => {
                    add_kv(idx, k, "rise", res.rise.to_string());
                    add_kv(idx, k, "fall", res.fall.to_string());
                }
            }
        }
    }
    let mut buf = String::new();
    writeln!(buf, r#"<div class="table-wrapper"><table>"#).unwrap();
    writeln!(buf, r#"<caption>{tag} speed data</caption>"#).unwrap();
    writeln!(buf, r#"<thead>"#).unwrap();
    writeln!(buf, r#"<tr>"#).unwrap();
    writeln!(buf, r#"<th colspan="2">Key</th>"#).unwrap();
    for ds in speeds {
        writeln!(buf, r#"<th>"#).unwrap();
        let mut first = true;
        for name in &ds.names {
            if !first {
                writeln!(buf, r#"<br>"#).unwrap();
            }
            writeln!(buf, r#"{name}"#).unwrap();
            first = false;
        }
        writeln!(buf, r#"</th>"#).unwrap();
    }
    writeln!(buf, r#"</tr>"#).unwrap();
    writeln!(buf, r#"</thead>"#).unwrap();
    writeln!(buf, r#"<tbody>"#).unwrap();
    for (&k, skv) in &kv {
        let mut key_first = true;
        for (&sk, vals) in skv {
            writeln!(buf, r#"<tr>"#).unwrap();
            if key_first {
                key_first = false;
                if skv.len() == 1 && sk.is_empty() {
                    writeln!(buf, r#"<td colspan="2">{k}</td>"#).unwrap();
                } else {
                    writeln!(
                        buf,
                        r#"<td rowspan="{rs}">{k}</td><td>{sk}</td>"#,
                        rs = skv.len()
                    )
                    .unwrap();
                }
            } else {
                writeln!(buf, r#"<td>{sk}</td>"#).unwrap();
            }
            for val in vals {
                if let Some(val) = val {
                    writeln!(buf, r#"<td>{val}</td>"#).unwrap();
                } else {
                    writeln!(buf, r#"<td>-</td>"#).unwrap();
                }
            }
            writeln!(buf, r#"</tr>"#).unwrap();
        }
    }
    writeln!(buf, r#"</tbody>"#).unwrap();
    writeln!(buf, r#"</table></div>"#).unwrap();
    ctx.items.insert(format!("speed-{tag}"), buf);
}
