use bincode::{BorrowDecode, Decode, Encode, de::read::Reader, enc::write::Writer};

use crate::EntityId;

use super::EntityBitVec;

impl<I: EntityId> Encode for EntityBitVec<I> {
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

impl<I: EntityId, Context> Decode<Context> for EntityBitVec<I> {
    fn decode<D: bincode::de::Decoder<Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len: usize = Decode::decode(decoder)?;
        let mut buf = vec![0u8; len.div_ceil(8)];
        decoder.reader().read(&mut buf)?;
        Ok(EntityBitVec::from_bytes(&buf, len))
    }
}

impl<'de, I: EntityId, Context> BorrowDecode<'de, Context> for EntityBitVec<I> {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let len: usize = BorrowDecode::borrow_decode(decoder)?;
        let mut buf = vec![0u8; len.div_ceil(8)];
        decoder.reader().read(&mut buf)?;
        Ok(EntityBitVec::from_bytes(&buf, len))
    }
}
