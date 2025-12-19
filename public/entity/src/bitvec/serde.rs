use core::marker::PhantomData;
use std::fmt;

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

use crate::{EntityBitVec, EntityId};

impl<I: EntityId> Serialize for EntityBitVec<I> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for v in self.values() {
            seq.serialize_element(&*v)?;
        }
        seq.end()
    }
}

struct DeserializeVisitor<I: EntityId> {
    marker: PhantomData<fn() -> EntityBitVec<I>>,
}

impl<I: EntityId> DeserializeVisitor<I> {
    fn new() -> Self {
        DeserializeVisitor {
            marker: PhantomData,
        }
    }
}

impl<'de, I> Visitor<'de> for DeserializeVisitor<I>
where
    I: EntityId,
{
    type Value = EntityBitVec<I>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("entity vector")
    }

    fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut res = EntityBitVec::with_capacity(access.size_hint().unwrap_or(0));

        while let Some(value) = access.next_element()? {
            res.push(value);
        }

        Ok(res)
    }
}

impl<'de, I> Deserialize<'de> for EntityBitVec<I>
where
    I: EntityId,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(DeserializeVisitor::new())
    }
}
