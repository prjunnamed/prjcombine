use std::fmt;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

use crate::{EntityId, EntitySet};

impl<I, V, RS> Serialize for EntitySet<I, V, RS>
where
    I: EntityId,
    V: Serialize + Hash + Eq,
    RS: BuildHasher,
{
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

#[allow(clippy::type_complexity)]
struct DeserializeVisitor<I: EntityId, V: Hash + Eq, RS: BuildHasher> {
    marker: PhantomData<fn() -> EntitySet<I, V, RS>>,
}

impl<I, V, RS> DeserializeVisitor<I, V, RS>
where
    I: EntityId,
    V: Hash + Eq,
    RS: BuildHasher,
{
    fn new() -> Self {
        DeserializeVisitor {
            marker: PhantomData,
        }
    }
}

impl<'de, I, V, RS> Visitor<'de> for DeserializeVisitor<I, V, RS>
where
    I: EntityId,
    V: Deserialize<'de> + Hash + Eq,
    RS: Default + BuildHasher,
{
    type Value = EntitySet<I, V, RS>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("entity set")
    }

    fn visit_seq<S>(self, mut access: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut res = EntitySet::with_capacity_and_hasher(
            access.size_hint().unwrap_or(0),
            Default::default(),
        );

        while let Some(value) = access.next_element()? {
            res.insert(value);
        }

        Ok(res)
    }
}

impl<'de, I, V, RS> Deserialize<'de> for EntitySet<I, V, RS>
where
    I: EntityId,
    V: Deserialize<'de> + Hash + Eq,
    RS: Default + BuildHasher,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(DeserializeVisitor::new())
    }
}
