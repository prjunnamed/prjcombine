use std::hash::{BuildHasher, Hash};

use bincode::{BorrowDecode, Decode, Encode};

use crate::{EntityId, EntityMap};

impl<I, K, V, RS> Encode for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Encode + Hash + Eq,
    V: Encode,
    RS: BuildHasher,
{
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.len().encode(encoder)?;
        for (_, key, val) in self {
            key.encode(encoder)?;
            val.encode(encoder)?;
        }
        Ok(())
    }
}

impl<I, K, V, RS, Context> Decode<Context> for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Decode<Context> + Hash + Eq,
    V: Decode<Context>,
    RS: Default + BuildHasher,
{
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len = usize::decode(decoder)?;
        let mut res = EntityMap::with_capacity_and_hasher(len, Default::default());
        for _ in 0..len {
            let key = K::decode(decoder)?;
            let val = V::decode(decoder)?;
            res.insert(key, val);
        }
        Ok(res)
    }
}

impl<'de, I, K, V, RS, Context> BorrowDecode<'de, Context> for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: BorrowDecode<'de, Context> + Hash + Eq,
    V: BorrowDecode<'de, Context>,
    RS: Default + BuildHasher,
{
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len = usize::borrow_decode(decoder)?;
        let mut res = EntityMap::with_capacity_and_hasher(len, Default::default());
        for _ in 0..len {
            let key = K::borrow_decode(decoder)?;
            let val = V::borrow_decode(decoder)?;
            res.insert(key, val);
        }
        Ok(res)
    }
}
