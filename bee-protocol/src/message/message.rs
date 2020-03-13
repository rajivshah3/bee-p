use crate::message::MessageError;
use crate::message::{Header, HEADER_SIZE, HEADER_TYPE_SIZE};

use std::convert::TryInto;
use std::ops::Range;

pub(crate) trait Message {
    const ID: u8;

    fn size_range() -> Range<usize>;

    fn size(&self) -> usize;

    fn from_bytes(bytes: &[u8]) -> Result<Self, MessageError>
    where
        Self: std::marker::Sized;

    fn to_bytes(self, bytes: &mut [u8]);

    fn from_full_bytes(header: &Header, payload: &[u8]) -> Result<Self, MessageError>
    where
        Self: std::marker::Sized,
    {
        let payload_length = u16::from_be_bytes(
            header[HEADER_TYPE_SIZE..HEADER_SIZE]
                .try_into()
                .map_err(|_| MessageError::InvalidAdvertisedLengthBytes([header[1], header[2]]))?,
        );

        // TODO check message type

        if payload_length as usize != payload.len() {
            Err(MessageError::InvalidAdvertisedLength(
                payload_length as usize,
                payload.len(),
            ))?;
        }

        Self::from_bytes(payload)
    }

    fn into_full_bytes(self) -> Vec<u8>
    where
        Self: std::marker::Sized,
    {
        // TODO constant
        let size = self.size();
        let mut bytes = vec![0u8; HEADER_SIZE + size];

        bytes[0] = Self::ID;
        bytes[HEADER_TYPE_SIZE..HEADER_SIZE].copy_from_slice(&(size as u16).to_be_bytes());
        self.to_bytes(&mut bytes[HEADER_SIZE..]);

        bytes
    }
}
