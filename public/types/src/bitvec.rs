#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct BitVec {
    inner: ::bitvec::vec::BitVec,
}

impl BitVec {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity(len: usize) -> Self {
        Self {
            inner: ::bitvec::vec::BitVec::with_capacity(len),
        }
    }

    pub fn repeat(bit: bool, len: usize) -> Self {
        Self {
            inner: ::bitvec::vec::BitVec::repeat(bit, len),
        }
    }

    pub fn iter(&self) -> Iter {
        self.into_iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn all(&self) -> bool {
        self.inner.all()
    }

    pub fn any(&self) -> bool {
        self.inner.any()
    }

    pub fn set(&mut self, index: usize, value: bool) {
        self.inner.set(index, value);
    }

    pub fn push(&mut self, bit: bool) {
        self.inner.push(bit);
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        self.inner.swap(a, b);
    }

    pub fn as_u32(&self) -> u32 {
        assert!(self.len() <= 32);
        let mut res = 0;
        for (i, bit) in self.iter().enumerate() {
            res |= u32::from(bit) << i
        }
        res
    }

    pub fn as_u64(&self) -> u64 {
        assert!(self.len() <= 64);
        let mut res = 0;
        for (i, bit) in self.iter().enumerate() {
            res |= u64::from(bit) << i
        }
        res
    }

    pub fn slice(&self, range: impl std::ops::RangeBounds<usize>) -> BitVec {
        BitVec {
            inner: match (range.start_bound(), range.end_bound()) {
                (std::ops::Bound::Included(&s), std::ops::Bound::Included(&e)) => {
                    self.inner[s..=e].to_bitvec()
                }
                (std::ops::Bound::Included(&s), std::ops::Bound::Excluded(&e)) => {
                    self.inner[s..e].to_bitvec()
                }
                (std::ops::Bound::Included(&s), std::ops::Bound::Unbounded) => {
                    self.inner[s..].to_bitvec()
                }
                (std::ops::Bound::Unbounded, std::ops::Bound::Included(&e)) => {
                    self.inner[..=e].to_bitvec()
                }
                (std::ops::Bound::Unbounded, std::ops::Bound::Excluded(&e)) => {
                    self.inner[..e].to_bitvec()
                }
                (std::ops::Bound::Unbounded, std::ops::Bound::Unbounded) => {
                    self.inner[..].to_bitvec()
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = vec![0u8; self.len().div_ceil(8)];
        for (i, bit) in self.iter().enumerate() {
            res[i / 8] |= u8::from(bit) << (i % 8);
        }
        res
    }

    pub fn from_bytes(buf: &[u8], len: usize) -> Self {
        assert_eq!(buf.len(), len.div_ceil(8));
        let mut res = BitVec::repeat(false, len);
        for i in 0..len {
            res.set(i, (buf[i / 8] & (1 << (i % 8))) != 0);
        }
        res
    }
}

impl std::ops::BitAndAssign<&BitVec> for BitVec {
    fn bitand_assign(&mut self, rhs: &BitVec) {
        self.inner &= &rhs.inner;
    }
}

impl std::ops::BitOrAssign<&BitVec> for BitVec {
    fn bitor_assign(&mut self, rhs: &BitVec) {
        self.inner |= &rhs.inner;
    }
}

impl std::ops::BitXorAssign<&BitVec> for BitVec {
    fn bitxor_assign(&mut self, rhs: &BitVec) {
        self.inner ^= &rhs.inner;
    }
}

impl std::fmt::Display for BitVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for bit in self.iter().rev() {
            write!(f, "{}", usize::from(bit))?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for BitVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl std::ops::Index<usize> for BitVec {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

impl IntoIterator for BitVec {
    type Item = bool;

    type IntoIter = ::bitvec::vec::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

pub struct Iter<'a> {
    inner: ::bitvec::slice::Iter<'a, usize, ::bitvec::order::Lsb0>,
}

impl Iterator for Iter<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|bit| *bit)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|bit| *bit)
    }
}

impl ExactSizeIterator for Iter<'_> {}

impl<'a> IntoIterator for &'a BitVec {
    type Item = bool;

    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: self.inner.iter(),
        }
    }
}

impl FromIterator<bool> for BitVec {
    fn from_iter<T: IntoIterator<Item = bool>>(iter: T) -> Self {
        BitVec {
            inner: FromIterator::from_iter(iter),
        }
    }
}

impl Extend<bool> for BitVec {
    fn extend<T: IntoIterator<Item = bool>>(&mut self, iter: T) {
        self.inner.extend(iter);
    }
}

mod bincode {
    use bincode::{de::read::Reader, enc::write::Writer, Decode, BorrowDecode, Encode};

    use super::BitVec;

    impl Encode for BitVec {
        fn encode<E: bincode::enc::Encoder>(
            &self,
            encoder: &mut E,
        ) -> Result<(), bincode::error::EncodeError> {
            self.len().encode(encoder)?;
            let buf = self.to_bytes();
            encoder.writer().write(&buf)?;
            Ok(())
        }
    }

    impl<Context> Decode<Context> for BitVec {
        fn decode<D: bincode::de::Decoder<Context = Context>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            let len: usize = Decode::decode(decoder)?;
            let mut buf = vec![0u8; len.div_ceil(8)];
            decoder.reader().read(&mut buf)?;
            Ok(BitVec::from_bytes(&buf, len))
        }
    }

    impl<'de, Context> BorrowDecode<'de, Context> for BitVec {
        fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            let len: usize = BorrowDecode::borrow_decode(decoder)?;
            let mut buf = vec![0u8; len.div_ceil(8)];
            decoder.reader().read(&mut buf)?;
            Ok(BitVec::from_bytes(&buf, len))
        }
    }
}

mod serde {
    use serde::{
        Deserialize, Deserializer, Serialize,
        de::{SeqAccess, Visitor},
        ser::SerializeSeq,
    };

    use super::BitVec;

    impl Serialize for BitVec {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(self.len()))?;
            for element in self {
                seq.serialize_element(&element)?;
            }
            seq.end()
        }
    }

    #[allow(clippy::type_complexity)]
    struct DeserializeVisitor;

    impl<'de> Visitor<'de> for DeserializeVisitor {
        type Value = BitVec;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("bitvec")
        }

        fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let mut res = BitVec::with_capacity(access.size_hint().unwrap_or(0));

            while let Some(value) = access.next_element()? {
                res.push(value);
            }

            Ok(res)
        }
    }

    impl<'de> Deserialize<'de> for BitVec {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_seq(DeserializeVisitor)
        }
    }
}

mod jzon {
    use jzon::JsonValue;

    use super::BitVec;

    impl From<&BitVec> for JsonValue {
        fn from(value: &BitVec) -> Self {
            jzon::Array::from_iter(value.iter().map(JsonValue::from)).into()
        }
    }
}

#[macro_export]
macro_rules! __bit_to_bool {
    (0) => {
        false
    };
    (1) => {
        true
    };
}

#[macro_export]
macro_rules! bits {
    ($item:tt; $num:expr) => {
        $crate::bitvec::BitVec::repeat($crate::__bit_to_bool!($item), $num)
    };
    ($item:tt $(,)?) => {
        $crate::bitvec::BitVec::from_iter([$crate::__bit_to_bool!($item)])
    };
    ($head:tt, $($item:tt),+ $(,)?) => {
        $crate::bitvec::BitVec::from_iter([
            $crate::__bit_to_bool!($head), $($crate::__bit_to_bool!($item)),+
        ])
    };
    () => { $crate::bitvec::BitVec::new() };
}
