use std::hash::{BuildHasher, Hash};

use bincode::{BorrowDecode, Decode, Encode};

use crate::{EntityId, EntitySet};

impl<I, V, RS> Encode for EntitySet<I, V, RS>
where
    I: EntityId,
    V: Encode + Hash + Eq,
    RS: BuildHasher,
{
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.len().encode(encoder)?;
        for val in self.values() {
            val.encode(encoder)?;
        }
        Ok(())
    }
}

impl<I, V, RS, Context> Decode<Context> for EntitySet<I, V, RS>
where
    I: EntityId,
    V: Decode<Context> + Hash + Eq,
    RS: Default + BuildHasher,
{
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len = usize::decode(decoder)?;
        let mut res = EntitySet::with_capacity_and_hasher(len, Default::default());
        for _ in 0..len {
            let item = V::decode(decoder)?;
            res.insert(item);
        }
        Ok(res)
    }
}

impl<'de, I, V, RS, Context> BorrowDecode<'de, Context> for EntitySet<I, V, RS>
where
    I: EntityId,
    V: BorrowDecode<'de, Context> + Hash + Eq,
    RS: Default + BuildHasher,
{
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len = usize::borrow_decode(decoder)?;
        let mut res = EntitySet::with_capacity_and_hasher(len, Default::default());
        for _ in 0..len {
            let item = V::borrow_decode(decoder)?;
            res.insert(item);
        }
        Ok(res)
    }
}
