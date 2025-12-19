use core::marker::PhantomData;
use std::fmt;

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

use super::EntityVec;
use crate::EntityId;

impl<I: EntityId, V: Serialize> Serialize for EntityVec<I, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for v in self.values() {
            seq.serialize_element(v)?;
        }
        seq.end()
    }
}

struct DeserializeVisitor<I: EntityId, V> {
    marker: PhantomData<fn() -> EntityVec<I, V>>,
}

impl<I: EntityId, V> DeserializeVisitor<I, V> {
    fn new() -> Self {
        DeserializeVisitor {
            marker: PhantomData,
        }
    }
}

impl<'de, I, V> Visitor<'de> for DeserializeVisitor<I, V>
where
    I: EntityId,
    V: Deserialize<'de>,
{
    type Value = EntityVec<I, V>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("entity vector")
    }

    fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut res = EntityVec::with_capacity(access.size_hint().unwrap_or(0));

        while let Some(value) = access.next_element()? {
            res.push(value);
        }

        Ok(res)
    }
}

impl<'de, I, V> Deserialize<'de> for EntityVec<I, V>
where
    I: EntityId,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(DeserializeVisitor::new())
    }
}
