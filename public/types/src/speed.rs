use std::collections::BTreeMap;

use jzon::JsonValue;
use serde::{Deserialize, Serialize};

use crate::units::{Resistance, Scalar, Temperature, Time, Voltage};

/// A simple propagation delay, with minimum and maximum value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimeRange {
    pub min: Time,
    pub max: Time,
}

impl std::fmt::Display for TimeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.min, self.max)
    }
}

/// An unateness-aware delay through binate combinational logic.
///
/// The `rise_to_rise` field describes the input-to-output delay when a rising edge
/// on the input causes a rising edge on the output.  Likewise, the `rise_to_fall` field
/// describes the delay when a rising edge on the input causes a falling edge on the output,
/// and so on.  Each of the four fields is a range with a minimum and maximum value.
///
/// This is the most generic unateness-aware delay type, covering all four combinations.
/// There are other types which should be used in specific circumstances:
///
/// 1. When the path is positive-unate (ie. a rising edge on the input can only cause a rising edge
///    on the output, and a falling edge on the input can only cause a falling edge on the output),
///    the `rise_to_fall` and `fall_to_rise` fields would not be applicable, and
///    the [`DelayRfUnate`] type should be used instead.  This applies to both routing and
///    some kinds of combinational logic (AND gates, OR gates, MAJ gates, …).  The field
///    correspondence is as follows:
///    - `rise_to_rise` corresponds to [`DelayRfUnate`]'s `rise` field
///    - `fall_to_fall` corresponds to [`DelayRfUnate`]'s `fall` field
/// 2. Likewise, when the path is negative-uante (a rising edge on the input can only cause
///    a falling edge on the output, and vice versa), the `rise_to_rise` and `fall_to_fall` are
///    not applicable, and the [`DelayRfUnate`] type should also be used instead.
///    This applies to both routing (when inverting muxes are involved) and some kinds of
///    combinational logic (NOT gates, NAND gates, …).  The field correspondence is as follows:
///    - `fall_to_rise` corresponds to [`DelayRfUnate`]'s `rise` field
///    - `rise_to_fall` corresponds to [`DelayRfUnate`]'s `fall` field
/// 3. When the path is a posedge clock-to-out, the `fall_to_*` fields are not applicable.
///    The same applies for a negedge clock-to-out and `rise_to_*` fields.  In these cases,
///    the [`DelayRfFromEdge`] type should be used instead.  This type can also be used
///    for asynchronous resets with a configurable reset value.
/// 4. When the path is a reset-to-out with a constant reset value, only one of the four
///    fields is applicable.  In this case, a simple [`Delay`] should be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelayRfBinate {
    pub rise_to_rise: Time,
    pub rise_to_fall: Time,
    pub fall_to_rise: Time,
    pub fall_to_fall: Time,
}

impl std::fmt::Display for DelayRfBinate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "++{} +-{} -+{} --{}",
            self.rise_to_rise, self.rise_to_fall, self.fall_to_rise, self.fall_to_fall
        )
    }
}

/// A version of [`DelayRfBinate`] with min and max values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelayRfBinateRange {
    pub rise_to_rise: TimeRange,
    pub rise_to_fall: TimeRange,
    pub fall_to_rise: TimeRange,
    pub fall_to_fall: TimeRange,
}

impl std::fmt::Display for DelayRfBinateRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "r-r {} r-f {} f-r {} f-f {}",
            self.rise_to_rise, self.rise_to_fall, self.fall_to_rise, self.fall_to_fall
        )
    }
}

/// An unateness-aware delay through unate combinational logic or routing.
///
/// The `rise` field describes the input-to-output delay for a rising edge on the output,
/// and the `fall` field describes the input-to-output delay for a falling edge on the output.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelayRfUnate {
    pub rise: Time,
    pub fall: Time,
}

impl std::fmt::Display for DelayRfUnate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{} -{}", self.rise, self.fall)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelayRfUnateRange {
    pub rise: TimeRange,
    pub fall: TimeRange,
}

impl std::fmt::Display for DelayRfUnateRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r {} f {}", self.rise, self.fall)
    }
}

/// A simple setup-hold constraint.
///
/// This constraint describes a relation between a data input and a clock input.
/// Assume there's an active clock edge at time `t`.  To avoid metastability and ensure the correct
/// value is stored into the register, the data input must not change within the time window
/// described by `[t - setup, t + hold]`.
///
/// It is possible that either `setup` or `hold` is negative.  However, `setup + hold` must never
/// be negative.
///
/// While registers with `setup + hold` of exactly 0 are not physically possible, some speed
/// databases describe such registers anyway.  This can happen in one of two cases:
///
/// - the source speed data is very crude (this generally applies to CPLDs and other old devices)
/// - the `setup + hold` window is actually non-zero, but small enough that it is covered by
///   routing delay range of whatever muxes are feeding the clock and data inputs
///
/// This value type is used when the timing model is not unateness-aware.  For a unate-aware
/// constraint, the [`SetupHoldRf`] type should be used instead.
///
/// Note that both `setup` and `hold` are simple `Time` values, not ranges — there is no use for
/// a range, as only the `max` value would be actually meaningful for the setup and hold checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SetupHold {
    pub setup: Time,
    pub hold: Time,
}

impl std::fmt::Display for SetupHold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "setup {} hold {}", self.setup, self.hold)
    }
}

/// A unateness-aware setup-hold constraint.
///
/// This is a unateness-aware version of [`SetupHold`].  Assume there's an active clock edge
/// at time `t`.  The constraint is:
///
/// - there must be no rising edge on the data input within the time window
///   `[t - setup_rise, t + hold_rise]`
/// - there must be no falling edge on the data input within the time window
///   `[t - setup_fall, t + hold_fall]`
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SetupHoldRf {
    pub rise_setup: Time,
    pub rise_hold: Time,
    pub fall_setup: Time,
    pub fall_hold: Time,
}

impl std::fmt::Display for SetupHoldRf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "r setup {} r hold {} f setup {} f hold {}",
            self.rise_setup, self.rise_hold, self.fall_setup, self.fall_hold
        )
    }
}

/// A recovery-removal constraint.
///
/// This constraint describes a relation between the deassertion of an asynchronous reset input
/// and a clock input.  Assume there is an active clock edge at time `t`.  Consider the time
/// window of `[t - recovery, t + removal]`.  The register operation is:
///
/// - if the reset is deasserted before the time window, the register operates normally and latches
///   the data input
/// - if the reset is deasserted after the time window, the register stays at the reset value until
///   the next active clock edge
/// - if the reset is deasserted within the time window, the result is nondeterministic, and
///   the register may enter a metastable state
///
/// Like [`SetupHold`], one of `recovery` or `removal` can be negative, but `recovery + removal`
/// must be non-negative.
///
/// This constraint is very similar to [`SetupHold`].  The core difference is that setup-hold
/// constraints apply to both rising and falling data changes, while recovery-removal constraints
/// only apply to reset deassertion, not assertion.
///
/// Since only one edge of reset and only one edge of clock are applicable, there is no unate-aware
/// variant of `RecRem`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecRem {
    pub recovery: Time,
    pub removal: Time,
}

impl std::fmt::Display for RecRem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "recovery {} removal {}", self.recovery, self.removal)
    }
}

/// A linear derating factor equation for temperature-based derating.
///
/// The factor is `a * t + b`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerateFactorTemperatureLinear {
    pub a: Scalar,
    pub b: Scalar,
}

impl DerateFactorTemperatureLinear {
    pub fn eval(self, t: Temperature) -> Scalar {
        self.a * t.0 + self.b
    }
}

impl std::fmt::Display for DerateFactorTemperatureLinear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({a}t + {b})", a = self.a, b = self.b)
    }
}

/// An inverse-quadratic derating factor equation for voltage-based derating.
///
/// The factor is `1 / (a * V * V + b + V + c)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerateFactorVoltageInvQuadratic {
    pub a: Scalar,
    pub b: Scalar,
    pub c: Scalar,
}

impl DerateFactorVoltageInvQuadratic {
    pub fn eval(self, v: Voltage) -> Scalar {
        Scalar(1.0) / (self.a * v.0 * v.0 + self.b * v.0 + self.c)
    }
}

impl std::fmt::Display for DerateFactorVoltageInvQuadratic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({a}V² + {b}V + {c})¯¹",
            a = self.a,
            b = self.b,
            c = self.c,
        )
    }
}

/// An unateness-aware resistance.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResistanceRf {
    pub rise: Resistance,
    pub fall: Resistance,
}

impl std::fmt::Display for ResistanceRf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{} -{}", self.rise, self.fall)
    }
}

/// A single speed value in the speed database.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SpeedVal {
    /// A simple propagation delay.
    ///
    /// Used for:
    ///
    /// - routing delays
    /// - combinational logic delays
    /// - clock-to-out delays
    /// - reset-to-out delays
    ///
    /// This value type is used when the timing model is not unateness-aware,
    /// or when only a single unateness is involved (such as reset-to-out delays).
    /// For unate-aware delays, one of the `DelayRf*` types can be used instead:
    ///
    /// - [`DelayRfBinate`] for binate combinational logic delays
    /// - [`DelayRfUnate`] for routing delays and positive or negative unate combinational logic delays
    /// - [`DelayRfFromEdge`] for clock-to-out and reset-to-out delays
    Delay(Time),
    /// A propagation delay with min and max values.
    DelayRange(TimeRange),
    /// A binate combinational delay.  See [`DelayRfBinate`] for details.
    DelayRfBinate(DelayRfBinate),
    /// A binate combinational delay with min and max values.
    DelayRfBinateRange(DelayRfBinateRange),
    /// An unate combinational or routing delay.  See [`DelayRfUnate`] for details.
    DelayRfPosUnate(DelayRfUnate),
    /// An unate combinational delay with min and max values.
    DelayRfPosUnateRange(DelayRfUnateRange),
    /// An unate combinational or routing delay.  See [`DelayRfUnate`] for details.
    DelayRfNegUnate(DelayRfUnate),
    /// An unate combinational delay with min and max values.
    DelayRfNegUnateRange(DelayRfUnateRange),
    /// An unate clock-to-out or reset-to-out delay.
    DelayRfFromEdge(DelayRfUnate),
    /// An unate clock-to-out or reset-to-out delay with min and max values.
    DelayRfFromEdgeRange(DelayRfUnateRange),
    /// A simple non-unate setup-hold constraint.  See [`SetupHold`] for details.
    SetupHold(SetupHold),
    /// An unate setup-hold constraint.  See [`SetupHoldRf`] for details.
    SetupHoldRf(SetupHoldRf),
    /// A recovery-removal constraint.  See [`RecRem`] for details.
    RecRem(RecRem),
    /// A minimum pulse width constraint.
    ///
    /// This value can describe:
    /// - the minimum width of a clock high or low period to ensure correct FF operation
    /// - the minimum width of an active gate pulse to ensure the data input is stored in
    ///   a latch
    /// - the minimum width of an async reset pulse to ensure the register is reset
    /// - the minimum width of a pulse that is guaranteed to be propagated through a routing
    ///   or combinational path
    PulseWidth(Time),
    /// A minimum period constraint.
    ///
    /// This value describes the minimum clock period (time between active edges) needed to ensure
    /// the correct operation of an FF.  In addition to this constraint, there may also be more
    /// specific constraints on the width of the low and high clock periods, which would be
    /// described by separate [`SpeedVal::PulseWidth`] entries.
    Period(Time),
    /// A scalar value, for example a derating factor.
    Scalar(Scalar),
    DerateFactorTemperatureLinear(DerateFactorTemperatureLinear),
    DerateFactorVoltageInvQuadratic(DerateFactorVoltageInvQuadratic),
    Resistance(Resistance),
    ResistanceRf(ResistanceRf),
}

impl std::fmt::Display for SpeedVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeedVal::Delay(delay) => write!(f, "delay {delay}"),
            SpeedVal::DelayRange(delay) => write!(f, "delay {delay}"),
            SpeedVal::DelayRfBinate(delay) => write!(f, "delay rf binate {delay}"),
            SpeedVal::DelayRfBinateRange(delay) => write!(f, "delay rf binate {delay}"),
            SpeedVal::DelayRfPosUnate(delay) => write!(f, "delay rf pos unate {delay}"),
            SpeedVal::DelayRfPosUnateRange(delay) => write!(f, "delay rf pos unate {delay}"),
            SpeedVal::DelayRfNegUnate(delay) => write!(f, "delay rf neg unate {delay}"),
            SpeedVal::DelayRfNegUnateRange(delay) => write!(f, "delay rf neg unate {delay}"),
            SpeedVal::DelayRfFromEdge(delay) => {
                write!(f, "delay rf from edge {delay}")
            }
            SpeedVal::DelayRfFromEdgeRange(delay) => {
                write!(f, "delay rf from edge {delay}")
            }
            SpeedVal::SetupHold(setuphold) => write!(f, "{setuphold}"),
            SpeedVal::SetupHoldRf(setuphold) => write!(f, "{setuphold}"),
            SpeedVal::RecRem(recrem) => write!(f, "{recrem}"),
            SpeedVal::PulseWidth(time) => write!(f, "pulsewidth {time}"),
            SpeedVal::Period(time) => write!(f, "period {time}"),
            SpeedVal::Scalar(scalar) => write!(f, "scalar {scalar}"),
            SpeedVal::DerateFactorTemperatureLinear(eq) => {
                write!(f, "derate temperature linear {eq}")
            }
            SpeedVal::DerateFactorVoltageInvQuadratic(eq) => {
                write!(f, "derate voltage inverse quadratic {eq}")
            }
            SpeedVal::Resistance(res) => write!(f, "res {res}"),
            SpeedVal::ResistanceRf(res) => write!(f, "res rf {res}"),
        }
    }
}

/// A string-keyed database of speed values, describing a particular speed grade of a device.
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct Speed {
    pub vals: BTreeMap<String, SpeedVal>,
}

impl Speed {
    pub fn new() -> Self {
        Default::default()
    }
}

impl From<TimeRange> for JsonValue {
    fn from(value: TimeRange) -> Self {
        jzon::object! {
            min: value.min,
            max: value.max,
        }
    }
}

impl From<SpeedVal> for JsonValue {
    fn from(value: SpeedVal) -> Self {
        match value {
            SpeedVal::Delay(delay) => jzon::object! {
                kind: "delay",
                value: delay,
            },
            SpeedVal::DelayRange(range) => jzon::object! {
                kind: "delay",
                value: range,
            },
            SpeedVal::DelayRfBinate(delay) => jzon::object! {
                kind: "delay_rf_binate",
                rise_to_rise: delay.rise_to_rise,
                rise_to_fall: delay.rise_to_fall,
                fall_to_rise: delay.fall_to_rise,
                fall_to_fall: delay.fall_to_fall,
            },
            SpeedVal::DelayRfBinateRange(delay) => jzon::object! {
                kind: "delay_rf_binate",
                rise_to_rise: delay.rise_to_rise,
                rise_to_fall: delay.rise_to_fall,
                fall_to_rise: delay.fall_to_rise,
                fall_to_fall: delay.fall_to_fall,
            },
            SpeedVal::DelayRfPosUnate(delay) => jzon::object! {
                kind: "delay_rf_pos_unate",
                rise: delay.rise,
                fall: delay.fall,
            },
            SpeedVal::DelayRfPosUnateRange(delay) => jzon::object! {
                kind: "delay_rf_pos_unate",
                rise: delay.rise,
                fall: delay.fall,
            },
            SpeedVal::DelayRfNegUnate(delay) => jzon::object! {
                kind: "delay_rf_neg_unate",
                rise: delay.rise,
                fall: delay.fall,
            },
            SpeedVal::DelayRfNegUnateRange(delay) => jzon::object! {
                kind: "delay_rf_neg_unate",
                rise: delay.rise,
                fall: delay.fall,
            },
            SpeedVal::DelayRfFromEdge(delay) => jzon::object! {
                kind: "delay_rf_from_edge",
                rise: delay.rise,
                fall: delay.fall,
            },
            SpeedVal::DelayRfFromEdgeRange(delay) => jzon::object! {
                kind: "delay_rf_from_edge",
                rise: delay.rise,
                fall: delay.fall,
            },
            SpeedVal::SetupHold(setuphold) => jzon::object! {
                kind: "setuphold",
                setup: setuphold.setup,
                hold: setuphold.hold,
            },
            SpeedVal::SetupHoldRf(setuphold) => jzon::object! {
                kind: "setuphold_rf",
                rise_setup: setuphold.rise_setup,
                fall_setup: setuphold.fall_setup,
                rise_hold: setuphold.rise_hold,
                fall_hold: setuphold.fall_hold,
            },
            SpeedVal::RecRem(recrem) => jzon::object! {
                kind: "recrem",
                recovery: recrem.recovery,
                removal: recrem.removal,
            },
            SpeedVal::PulseWidth(time) => jzon::object! {
                kind: "pulsewidth",
                value: time,
            },
            SpeedVal::Period(time) => jzon::object! {
                kind: "period",
                value: time,
            },
            SpeedVal::Scalar(scalar) => jzon::object! {
                kind: "scalar",
                value: scalar,
            },
            SpeedVal::DerateFactorTemperatureLinear(eq) => jzon::object! {
                kind: "derate_factor_temperature_linear",
                a: eq.a,
                b: eq.b,
            },
            SpeedVal::DerateFactorVoltageInvQuadratic(eq) => jzon::object! {
                kind: "derate_factor_voltage_inv_quadratic",
                a: eq.a,
                b: eq.b,
                c: eq.c,
            },
            SpeedVal::Resistance(res) => jzon::object! {
                kind: "resistance",
                value: res,
            },
            SpeedVal::ResistanceRf(res) => jzon::object! {
                kind: "resistance_rf",
                rise: res.rise,
                fall: res.fall,
            },
        }
    }
}

impl From<&Speed> for JsonValue {
    fn from(speed: &Speed) -> Self {
        jzon::object! {
            vals: jzon::object::Object::from_iter(speed.vals.iter().map(|(name, val)| {
                (name.as_str(), *val)
            })),
        }
    }
}

impl std::fmt::Display for Speed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (k, v) in &self.vals {
            writeln!(f, "\t{k:40}: {v}")?;
        }
        Ok(())
    }
}
