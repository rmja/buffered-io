use embedded_io_async::{BufRead, Read, Write};

/// A buffered [`Read`]
///
/// The BufferedRead will read into the provided buffer to avoid small reads to the inner reader.
pub struct BufferedRead<'buf, T: Read> {
    inner: T,
    buf: &'buf mut [u8],
    offset: usize,
    available: usize,
}

impl<'buf, T: Read> BufferedRead<'buf, T> {
    pub fn new(inner: T, buf: &'buf mut [u8]) -> Self {
        Self {
            inner,
            buf,
            offset: 0,
            available: 0,
        }
    }
}

impl<T: Read> embedded_io::ErrorType for BufferedRead<'_, T> {
    type Error = T::Error;
}

impl<T: Read + Write> Write for BufferedRead<'_, T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(buf).await
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.inner.write_all(buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush().await
    }
}

impl<T: Read> Read for BufferedRead<'_, T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.available == 0 {
            if buf.len() >= self.buf.len() {
                // Fast path - bypass local buffer
                return self.inner.read(buf).await;
            }
            self.offset = 0;
            self.available = self.inner.read(self.buf).await?;
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
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        if self.available == 0 {
            self.offset = 0;
            self.available = self.inner.read(self.buf).await?;
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
    use super::*;

    #[tokio::test]
    async fn can_read_to_buffer() {
        let inner = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut buf = [0; 8];
        let mut buffered = BufferedRead::new(inner.as_slice(), &mut buf);

        let mut read_buf = [0; 2];
        assert_eq!(2, buffered.read(&mut read_buf).await.unwrap());
        assert_eq!(2, buffered.offset);
        assert_eq!(6, buffered.available);
        assert_eq!(&[1, 2], read_buf.as_slice());

        let mut read_buf = [0; 2];
        assert_eq!(2, buffered.read(&mut read_buf).await.unwrap());
        assert_eq!(4, buffered.offset);
        assert_eq!(4, buffered.available);
        assert_eq!(&[3, 4], read_buf.as_slice());

        let mut read_buf = [0; 8];
        assert_eq!(4, buffered.read(&mut read_buf).await.unwrap());
        assert_eq!(4, buffered.offset);
        assert_eq!(0, buffered.available);
        assert_eq!(&[5, 6, 7, 8], &read_buf[..4]);
    }

    #[tokio::test]
    async fn bypass_on_large_buf() {
        let inner = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut buf = [0; 8];
        let mut buffered = BufferedRead::new(inner.as_slice(), &mut buf);

        let mut read_buf = [0; 10];
        assert_eq!(10, buffered.read(&mut read_buf).await.unwrap());
        assert_eq!(0, buffered.offset);
        assert_eq!(0, buffered.available);
        assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], read_buf.as_slice());
    }

    #[tokio::test]
    async fn can_buf_read() {
        let inner = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut buf = [0; 8];
        let mut buffered = BufferedRead::new(inner.as_slice(), &mut buf);
        assert_eq!(0, buffered.offset);
        assert_eq!(0, buffered.available);

        assert_eq!(
            &[1, 2, 3, 4, 5, 6, 7, 8],
            buffered.fill_buf().await.unwrap()
        );
        assert_eq!(0, buffered.offset);
        assert_eq!(8, buffered.available);

        buffered.consume(2);
        assert_eq!(2, buffered.offset);
        assert_eq!(6, buffered.available);
        assert_eq!(&[3, 4, 5, 6, 7, 8], buffered.fill_buf().await.unwrap());

        buffered.consume(6);
        assert_eq!(8, buffered.offset);
        assert_eq!(0, buffered.available);

        assert_eq!(&[9, 10], buffered.fill_buf().await.unwrap());
        assert_eq!(0, buffered.offset);
        assert_eq!(2, buffered.available);

        buffered.consume(2);
        assert_eq!(2, buffered.offset);
        assert_eq!(0, buffered.available);
    }
}
