use std::collections::BTreeMap;

use jzon::JsonValue;
use serde::{Deserialize, Serialize};

/// A time-dimension value for speed data.  The unit is ps.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Time(pub f64);

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let mut a = self.0.to_bits();
        let mut b = other.0.to_bits();
        if (a & (1 << 63)) != 0 {
            a = !a;
        } else {
            a ^= 1 << 63;
        }
        if (b & (1 << 63)) != 0 {
            b = !b;
        } else {
            b ^= 1 << 63;
        }
        a.cmp(&b)
    }
}

impl std::ops::Sub for Time {
    type Output = Time;

    fn sub(self, rhs: Self) -> Self::Output {
        Time(self.0 - rhs.0)
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ps", self.0)
    }
}

/// A simple propagation delay, with minimum and maximum value.
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Delay {
    pub min: Time,
    pub max: Time,
}

impl std::fmt::Display for Delay {
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
    pub rise_to_rise: Delay,
    pub rise_to_fall: Delay,
    pub fall_to_rise: Delay,
    pub fall_to_fall: Delay,
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

/// An unateness-aware delay through unate combinational logic or routing.
///
/// The `rise` field describes the input-to-output delay for a rising edge on the output,
/// and the `fall` field describes the input-to-output delay for a falling edge on the output.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelayRfUnate {
    pub rise: Delay,
    pub fall: Delay,
}

impl std::fmt::Display for DelayRfUnate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{} -{}", self.rise, self.fall)
    }
}

/// An unateness-aware clock-to-out or reset-to-out delay.
///
/// The `rise` field describes the input-to-output delay for a rising edge on the output,
/// and the `fall` field describes the input-to-output delay for a falling edge on the output.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DelayRfFromEdge {
    pub rise: Delay,
    pub fall: Delay,
}

impl std::fmt::Display for DelayRfFromEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{} -{}", self.rise, self.fall)
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
        write!(f, "setuphold [{}, {}]", -self.setup.0, self.hold)
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
            "setuphold rf +[{}, {}] -[{}, {}]",
            -self.rise_setup.0, self.rise_hold, -self.fall_setup.0, self.fall_hold
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
        write!(f, "recrem [{}, {}]", -self.recovery.0, self.removal)
    }
}

/// A single speed value in the speed database.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SpeedVal {
    /// A simple non-unate delay.  See [`Delay`] for details.
    Delay(Delay),
    /// A binate combinational delay.  See [`DelayRfBinate`] for details.
    DelayRfBinate(DelayRfBinate),
    /// An unate combinational or routing delay.  See [`DelayRfUnate`] for details.
    DelayRfUnate(DelayRfUnate),
    /// An unate clock-to-out or reset-to-out delay.  See [`DelayRfFromEdge`] for details.
    DelayRfFromEdge(DelayRfFromEdge),
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
}

impl std::fmt::Display for SpeedVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeedVal::Delay(delay) => write!(f, "delay {delay}"),
            SpeedVal::DelayRfBinate(delay) => write!(f, "delay rf binate {delay}"),
            SpeedVal::DelayRfUnate(delay) => write!(f, "delay rf unate {delay}"),
            SpeedVal::DelayRfFromEdge(delay) => {
                write!(f, "delay rf from edge {delay}")
            }
            SpeedVal::SetupHold(setuphold) => write!(f, "{setuphold}"),
            SpeedVal::SetupHoldRf(setuphold) => write!(f, "{setuphold}"),
            SpeedVal::RecRem(recrem) => write!(f, "{recrem}"),
            SpeedVal::PulseWidth(time) => write!(f, "pulsewidth {time}"),
            SpeedVal::Period(time) => write!(f, "period {time}"),
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

impl From<Time> for JsonValue {
    fn from(value: Time) -> Self {
        value.0.into()
    }
}

impl From<Delay> for JsonValue {
    fn from(value: Delay) -> Self {
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
            SpeedVal::DelayRfBinate(delay) => jzon::object! {
                kind: "delay_rf_binate",
                rise_to_rise: delay.rise_to_rise,
                rise_to_fall: delay.rise_to_fall,
                fall_to_rise: delay.fall_to_rise,
                fall_to_fall: delay.fall_to_fall,
            },
            SpeedVal::DelayRfUnate(delay) => jzon::object! {
                kind: "delay_rf_unate",
                rise: delay.rise,
                fall: delay.fall,
            },
            SpeedVal::DelayRfFromEdge(delay) => jzon::object! {
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
