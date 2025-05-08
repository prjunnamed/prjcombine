use jzon::JsonValue;
use serde::{Deserialize, Serialize};

/// A f64 with proper equality and total ordering.
///
/// This is needed for speed data deduplication.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Scalar(pub f64);

impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Scalar {}

impl PartialOrd for Scalar {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Scalar {
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

impl std::fmt::Debug for Scalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl std::fmt::Display for Scalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::ops::Add for Scalar {
    type Output = Scalar;

    fn add(self, rhs: Self) -> Self::Output {
        Scalar(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Scalar {
    type Output = Scalar;

    fn sub(self, rhs: Self) -> Self::Output {
        Scalar(self.0 - rhs.0)
    }
}

impl std::ops::Mul for Scalar {
    type Output = Scalar;

    fn mul(self, rhs: Self) -> Self::Output {
        Scalar(self.0 * rhs.0)
    }
}

impl std::ops::Div for Scalar {
    type Output = Scalar;

    fn div(self, rhs: Self) -> Self::Output {
        Scalar(self.0 / rhs.0)
    }
}

impl std::ops::Neg for Scalar {
    type Output = Scalar;

    fn neg(self) -> Self::Output {
        Scalar(-self.0)
    }
}

impl From<f64> for Scalar {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<i32> for Scalar {
    fn from(value: i32) -> Self {
        Self(value.into())
    }
}

impl From<Scalar> for JsonValue {
    fn from(value: Scalar) -> Self {
        value.0.into()
    }
}

/// A time-dimension value for speed data.  The unit is ps.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Time(pub Scalar);

impl Time {
    pub const ZERO: Time = Time(Scalar(0.0));
}

impl std::ops::Sub for Time {
    type Output = Time;

    fn sub(self, rhs: Self) -> Self::Output {
        Time(self.0 - rhs.0)
    }
}

impl std::ops::Div<Scalar> for Time {
    type Output = Time;

    fn div(self, rhs: Scalar) -> Self::Output {
        Time(self.0 / rhs)
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ps", self.0)
    }
}

impl From<Time> for JsonValue {
    fn from(value: Time) -> Self {
        value.0.into()
    }
}

/// A temperature-dimension value for speed data.  The unit is °C.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Temperature(pub Scalar);

impl std::fmt::Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}°C", self.0)
    }
}

impl From<Temperature> for JsonValue {
    fn from(value: Temperature) -> Self {
        value.0.into()
    }
}

/// A voltage-dimension value for speed data.  The unit is V.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Voltage(pub Scalar);

impl std::fmt::Display for Voltage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}V", self.0)
    }
}

impl From<Voltage> for JsonValue {
    fn from(value: Voltage) -> Self {
        value.0.into()
    }
}

/// A resistance-dimension value for speed data.  The unit is Ω.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Resistance(pub Scalar);

impl std::fmt::Display for Resistance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}Ω", self.0)
    }
}

impl From<Resistance> for JsonValue {
    fn from(value: Resistance) -> Self {
        value.0.into()
    }
}

/// A capacitance-dimension value for speed data.  The unit is pF.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Capacitance(pub Scalar);

impl std::fmt::Display for Capacitance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}pF", self.0)
    }
}

impl From<Capacitance> for JsonValue {
    fn from(value: Capacitance) -> Self {
        value.0.into()
    }
}
