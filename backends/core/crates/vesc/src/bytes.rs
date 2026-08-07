use super::ProtocolError;

pub struct ByteReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteReader<'a> {
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    /// Reads one byte.
    ///
    /// # Errors
    ///
    /// Rejects truncated input.
    pub fn u8(&mut self) -> Result<u8, ProtocolError> {
        let value = *self
            .bytes
            .get(self.offset)
            .ok_or(ProtocolError::TruncatedPayload)?;
        self.offset += 1;
        Ok(value)
    }

    /// Reads a big-endian unsigned 16-bit value.
    ///
    /// # Errors
    ///
    /// Rejects truncated input.
    pub fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_be_bytes([self.u8()?, self.u8()?]))
    }

    /// Reads a big-endian signed 16-bit value.
    ///
    /// # Errors
    ///
    /// Rejects truncated input.
    pub fn i16(&mut self) -> Result<i16, ProtocolError> {
        Ok(i16::from_be_bytes([self.u8()?, self.u8()?]))
    }

    /// Reads a big-endian signed 32-bit value.
    ///
    /// # Errors
    ///
    /// Rejects truncated input.
    pub fn i32(&mut self) -> Result<i32, ProtocolError> {
        Ok(i32::from_be_bytes([
            self.u8()?,
            self.u8()?,
            self.u8()?,
            self.u8()?,
        ]))
    }

    /// Returns an exact copied slice.
    ///
    /// # Errors
    ///
    /// Rejects truncated input and overflow.
    pub fn bytes(&mut self, count: usize) -> Result<Vec<u8>, ProtocolError> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(ProtocolError::TruncatedPayload)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ProtocolError::TruncatedPayload)?
            .to_vec();
        self.offset = end;
        Ok(value)
    }

    /// Reads bounded, zero-terminated UTF-8.
    ///
    /// # Errors
    ///
    /// Rejects absent terminators, truncation, and invalid UTF-8.
    pub fn terminated_utf8(&mut self, max_content: usize) -> Result<String, ProtocolError> {
        let search_end = self
            .offset
            .saturating_add(max_content.saturating_add(1))
            .min(self.bytes.len());
        let relative = self.bytes[self.offset..search_end]
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(ProtocolError::TruncatedPayload)?;
        let end = self.offset + relative;
        let value = std::str::from_utf8(&self.bytes[self.offset..end])
            .map_err(|_| ProtocolError::InvalidUtf8)?
            .to_owned();
        self.offset = end + 1;
        Ok(value)
    }
}

pub struct ByteWriter {
    bytes: Vec<u8>,
    maximum: usize,
}

impl ByteWriter {
    #[must_use]
    pub fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(maximum.min(256)),
            maximum,
        }
    }

    fn reserve(&self, count: usize) -> Result<(), ProtocolError> {
        if self.bytes.len().saturating_add(count) > self.maximum {
            Err(ProtocolError::PayloadTooLarge)
        } else {
            Ok(())
        }
    }

    /// Appends one byte within the configured bound.
    ///
    /// # Errors
    ///
    /// Rejects output exceeding the bound.
    pub fn put_u8(&mut self, value: u8) -> Result<(), ProtocolError> {
        self.reserve(1)?;
        self.bytes.push(value);
        Ok(())
    }

    /// Appends a big-endian unsigned 16-bit value.
    ///
    /// # Errors
    ///
    /// Rejects output exceeding the bound.
    pub fn put_u16(&mut self, value: u16) -> Result<(), ProtocolError> {
        self.put_bytes(&value.to_be_bytes())
    }

    /// Appends a big-endian signed 32-bit value.
    ///
    /// # Errors
    ///
    /// Rejects output exceeding the bound.
    pub fn put_i32(&mut self, value: i32) -> Result<(), ProtocolError> {
        self.put_bytes(&value.to_be_bytes())
    }

    /// Appends bytes within the configured bound.
    ///
    /// # Errors
    ///
    /// Rejects output exceeding the bound.
    pub fn put_bytes(&mut self, value: &[u8]) -> Result<(), ProtocolError> {
        self.reserve(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}
