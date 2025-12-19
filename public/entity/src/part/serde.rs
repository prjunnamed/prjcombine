use core::marker::PhantomData;
use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};

use super::EntityPartVec;
use crate::id::EntityId;

impl<I: EntityId + Serialize, V: Serialize> Serialize for EntityPartVec<I, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.iter().count()))?;
        for (k, v) in self {
            map.serialize_entry(&k, v)?;
        }
        map.end()
    }
}

struct DeserializeVisitor<I: EntityId, V> {
    marker: PhantomData<fn() -> EntityPartVec<I, V>>,
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
    I: EntityId + Deserialize<'de>,
    V: Deserialize<'de>,
{
    type Value = EntityPartVec<I, V>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("entity partial vector")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = EntityPartVec::with_capacity(access.size_hint().unwrap_or(0));

        while let Some((id, value)) = access.next_entry()? {
            map.insert(id, value);
        }

        Ok(map)
    }
}

impl<'de, I, V> Deserialize<'de> for EntityPartVec<I, V>
where
    I: EntityId + Deserialize<'de>,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DeserializeVisitor::new())
    }
}
