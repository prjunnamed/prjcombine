use bincode::{BorrowDecode, Decode, Encode};

use crate::{EntityBundleIndex, EntityBundleMap, EntityId};

impl<I: EntityId, T: Encode> Encode for EntityBundleMap<I, T> {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.bundles.len().encode(encoder)?;
        for (key, (idx, val)) in &self.bundles {
            let num = match idx {
                EntityBundleIndex::Single(_) => None,
                EntityBundleIndex::Array(range) => Some(range.len()),
            };
            num.encode(encoder)?;
            key.encode(encoder)?;
            val.encode(encoder)?;
        }
        Ok(())
    }
}

impl<I: EntityId, Context, T: Decode<Context>> Decode<Context> for EntityBundleMap<I, T> {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len = usize::decode(decoder)?;
        let mut res = Self::new();
        for _ in 0..len {
            let num: Option<usize> = Decode::decode(decoder)?;
            let key = String::decode(decoder)?;
            let val = T::decode(decoder)?;
            match num {
                None => {
                    if res.insert(key, val).is_none() {
                        return Err(bincode::error::DecodeError::Other("duplicate key"));
                    }
                }
                Some(num) => {
                    if res.insert_array(key, num, val).is_none() {
                        return Err(bincode::error::DecodeError::Other("duplicate key"));
                    }
                }
            }
        }
        Ok(res)
    }
}

impl<'de, I: EntityId, Context, T: BorrowDecode<'de, Context>> BorrowDecode<'de, Context>
    for EntityBundleMap<I, T>
{
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len = usize::decode(decoder)?;
        let mut res = Self::new();
        for _ in 0..len {
            let num: Option<usize> = BorrowDecode::borrow_decode(decoder)?;
            let key = String::borrow_decode(decoder)?;
            let val = T::borrow_decode(decoder)?;
            match num {
                None => {
                    if res.insert(key, val).is_none() {
                        return Err(bincode::error::DecodeError::Other("duplicate key"));
                    }
                }
                Some(num) => {
                    if res.insert_array(key, num, val).is_none() {
                        return Err(bincode::error::DecodeError::Other("duplicate key"));
                    }
                }
            }
        }
        Ok(res)
    }
}
