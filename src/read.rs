#[cfg(feature = "async")]
mod asynch;

use embedded_io::{BufRead, ErrorType, Read, Write};

use super::BypassError;

/// A buffered [`Read`]
///
/// The BufferedRead will read into the provided buffer to avoid small reads to the inner reader.
pub struct BufferedRead<'buf, T> {
    inner: T,
    buf: &'buf mut [u8],
    offset: usize,
    available: usize,
}

impl<'buf, T> BufferedRead<'buf, T> {
    /// Create a new buffered reader
    pub fn new(inner: T, buf: &'buf mut [u8]) -> Self {
        Self {
            inner,
            buf,
            offset: 0,
            available: 0,
        }
    }

    /// Create a new buffered reader with the first `available` bytes readily available at `offset`.
    ///
    /// This is useful if for some reason the inner reader was previously consumed by a greedy reader
    /// in a way such that the BufferedRead must inherit these excess bytes.
    pub fn new_with_data(inner: T, buf: &'buf mut [u8], offset: usize, available: usize) -> Self {
        assert!(offset + available <= buf.len());
        Self {
            inner,
            buf,
            offset,
            available,
        }
    }

    /// Get whether there are any bytes readily available
    pub fn is_empty(&self) -> bool {
        self.available == 0
    }

    /// Get the number of bytes that are readily availbale
    pub fn available(&self) -> usize {
        self.available
    }

    /// Get the inner reader if there are no currently buffered, available bytes
    pub fn bypass(&mut self) -> Result<&mut T, BypassError> {
        match self.available {
            0 => Ok(&mut self.inner),
            _ => Err(BypassError),
        }
    }

    /// Release and get the inner reader
    pub fn release(self) -> T {
        self.inner
    }
}

impl<T: ErrorType> ErrorType for BufferedRead<'_, T> {
    type Error = T::Error;
}

impl<T: Read + Write> Write for BufferedRead<'_, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.inner.write_all(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

impl<T: Read> Read for BufferedRead<'_, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.available == 0 {
            if buf.len() >= self.buf.len() {
                // Fast path - bypass local buffer
                return self.inner.read(buf);
            }
            self.offset = 0;
            self.available = self.inner.read(self.buf)?;
        }

        let len = usize::min(self.available, buf.len());
        buf[..len].copy_from_slice(&self.buf[self.offset..self.offset + len]);
        if len < self.available {
            // There are still bytes left
            self.offset += len;
            self.available -= len;
        } else {
            // The buffer is drained
            self.available = 0;
        }

        Ok(len)
    }
}

impl<T: Read> BufRead for BufferedRead<'_, T> {
    fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        if self.available == 0 {
            self.offset = 0;
            self.available = self.inner.read(self.buf)?;
        }

        Ok(&self.buf[self.offset..self.offset + self.available])
    }

    fn consume(&mut self, amt: usize) {
        assert!(amt <= self.available);
        self.offset += amt;
        self.available -= amt;
    }
}

#[cfg(test)]
mod tests {
    use embedded_io::{BufRead, Read};

    use super::BufferedRead;

    #[test]
    fn can_read_to_buffer() {
        let inner = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut buf = [0; 8];
        let mut buffered = BufferedRead::new(inner.as_slice(), &mut buf);

        let mut read_buf = [0; 2];
        assert_eq!(2, buffered.read(&mut read_buf).unwrap());
        assert_eq!(2, buffered.offset);
        assert_eq!(6, buffered.available);
        assert_eq!(&[1, 2], read_buf.as_slice());

        let mut read_buf = [0; 2];
        assert_eq!(2, buffered.read(&mut read_buf).unwrap());
        assert_eq!(4, buffered.offset);
        assert_eq!(4, buffered.available);
        assert_eq!(&[3, 4], read_buf.as_slice());

        let mut read_buf = [0; 8];
        assert_eq!(4, buffered.read(&mut read_buf).unwrap());
        assert_eq!(4, buffered.offset);
        assert_eq!(0, buffered.available);
        assert_eq!(&[5, 6, 7, 8], &read_buf[..4]);
    }

    #[test]
    fn bypass_on_large_buf() {
        let inner = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut buf = [0; 8];
        let mut buffered = BufferedRead::new(inner.as_slice(), &mut buf);

        let mut read_buf = [0; 10];
        assert_eq!(10, buffered.read(&mut read_buf).unwrap());
        assert_eq!(0, buffered.offset);
        assert_eq!(0, buffered.available);
        assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], read_buf.as_slice());
    }

    #[test]
    fn can_buf_read() {
        let inner = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut buf = [0; 8];
        let mut buffered = BufferedRead::new(inner.as_slice(), &mut buf);
        assert_eq!(0, buffered.offset);
        assert_eq!(0, buffered.available);

        assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8], buffered.fill_buf().unwrap());
        assert_eq!(0, buffered.offset);
        assert_eq!(8, buffered.available);

        buffered.consume(2);
        assert_eq!(2, buffered.offset);
        assert_eq!(6, buffered.available);
        assert_eq!(&[3, 4, 5, 6, 7, 8], buffered.fill_buf().unwrap());

        buffered.consume(6);
        assert_eq!(8, buffered.offset);
        assert_eq!(0, buffered.available);

        assert_eq!(&[9, 10], buffered.fill_buf().unwrap());
        assert_eq!(0, buffered.offset);
        assert_eq!(2, buffered.available);

        buffered.consume(2);
        assert_eq!(2, buffered.offset);
        assert_eq!(0, buffered.available);
    }
}
