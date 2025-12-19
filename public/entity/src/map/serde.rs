use core::hash::{BuildHasher, Hash};
use std::fmt;
use std::marker::PhantomData;

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

use crate::{EntityId, EntityMap};

impl<I, K, V, RS> Serialize for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Serialize + Hash + Eq,
    V: Serialize,
    RS: BuildHasher,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for (_, k, v) in self {
            seq.serialize_element(&(k, v))?;
        }
        seq.end()
    }
}

#[allow(clippy::type_complexity)]
struct DeserializeVisitor<I: EntityId, K: Hash + Eq, V, RS: BuildHasher> {
    marker: PhantomData<fn() -> EntityMap<I, K, V, RS>>,
}

impl<I, K, V, RS> DeserializeVisitor<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher,
{
    fn new() -> Self {
        DeserializeVisitor {
            marker: PhantomData,
        }
    }
}

impl<'de, I, K, V, RS> Visitor<'de> for DeserializeVisitor<I, K, V, RS>
where
    I: EntityId,
    K: Deserialize<'de> + Hash + Eq,
    V: Deserialize<'de>,
    RS: Default + BuildHasher,
{
    type Value = EntityMap<I, K, V, RS>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("entity map")
    }

    fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut res = EntityMap::with_capacity_and_hasher(
            access.size_hint().unwrap_or(0),
            Default::default(),
        );

        while let Some((key, value)) = access.next_element()? {
            res.insert(key, value);
        }

        Ok(res)
    }
}

impl<'de, I, K, V, RS> Deserialize<'de> for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Deserialize<'de> + Hash + Eq,
    V: Deserialize<'de>,
    RS: Default + BuildHasher,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(DeserializeVisitor::new())
    }
}
