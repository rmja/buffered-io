use embedded_io_async::{Read, Write};

use super::BypassError;

/// A buffered [`Write`]
///
/// The BufferedWrite will write into the provided buffer to avoid small writes to the inner writer.
pub struct BufferedWrite<'buf, T: Write> {
    inner: T,
    buf: &'buf mut [u8],
    pos: usize,
}

impl<'buf, T: Write> BufferedWrite<'buf, T> {
    /// Create a new buffered writer
    pub fn new(inner: T, buf: &'buf mut [u8]) -> Self {
        Self { inner, buf, pos: 0 }
    }

    /// Create a new buffered writer with a pre-polulated buffer
    pub fn new_with_data(inner: T, buf: &'buf mut [u8], written: usize) -> Self {
        Self {
            inner,
            buf,
            pos: written,
        }
    }

    /// Get whether there are any bytes currently buffered
    pub fn is_empty(&self) -> bool {
        self.pos == 0
    }

    /// Get the number of bytes that are currently buffered but not yet written to the inner writer
    pub fn written(&self) -> usize {
        self.pos
    }

    /// Clear the currently buffered, written bytes
    pub fn clear(&mut self) {
        self.pos = 0;
    }

    /// Get the inner writer if there are no currently buffered, written bytes
    pub fn bypass(&mut self) -> Result<&mut T, BypassError> {
        match self.pos {
            0 => Ok(&mut self.inner),
            _ => Err(BypassError),
        }
    }

    /// Get the inner writer if there are no currently buffered, written bytes, and rent the buffer
    pub fn bypass_with_buf(&mut self) -> Result<(&mut T, &mut [u8]), BypassError> {
        match self.pos {
            0 => Ok((&mut self.inner, self.buf)),
            _ => Err(BypassError),
        }
    }

    /// Split the writer to get the inner components
    pub fn split(&mut self) -> (&mut T, &mut [u8], usize) {
        (&mut self.inner, self.buf, self.pos)
    }

    /// Release and get the inner writer
    pub fn release(self) -> T {
        self.inner
    }
}

impl<T: Write> embedded_io::ErrorType for BufferedWrite<'_, T> {
    type Error = T::Error;
}

impl<T: Read + Write> Read for BufferedWrite<'_, T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.inner.read(buf).await
    }

    async fn read_exact(
        &mut self,
        buf: &mut [u8],
    ) -> Result<(), embedded_io::ReadExactError<Self::Error>> {
        self.inner.read_exact(buf).await
    }
}

impl<T: Write> Write for BufferedWrite<'_, T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.pos == 0 && buf.len() >= self.buf.len() {
            // Fast path - nothing in buffer and the buffer to write is large
            return self.inner.write(buf).await;
        }

        let buffered = usize::min(buf.len(), self.buf.len() - self.pos);
        if buffered > 0 {
            self.buf[self.pos..self.pos + buffered].copy_from_slice(&buf[..buffered]);
            self.pos += buffered;
        }

        if self.pos == self.buf.len() {
            // The buffer is full
            let written = self.inner.write(self.buf).await?;
            if written < self.pos {
                self.buf.copy_within(written..self.pos, 0);
                self.pos -= written;
            } else {
                self.pos = 0;
            }
        }

        Ok(buffered)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        if self.pos > 0 {
            self.inner.write_all(&self.buf[..self.pos]).await?;
            self.pos = 0;
        }

        self.inner.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn can_append_to_buffer() {
        let mut inner = Vec::new();
        let mut buf = [0; 8];
        let mut buffered = BufferedWrite::new(&mut inner, &mut buf);

        assert_eq!(2, buffered.write(&[1, 2]).await.unwrap());
        assert_eq!(2, buffered.pos);
        assert_eq!(0, buffered.inner.len());

        assert_eq!(2, buffered.write(&[3, 4]).await.unwrap());
        assert_eq!(4, buffered.pos);
        assert_eq!(0, buffered.inner.len());

        assert_eq!(4, buffered.write(&[5, 6, 7, 8]).await.unwrap());
        assert_eq!(0, buffered.pos);
        assert_eq!(8, buffered.inner.len());
        assert_eq!(&[1, 2, 3, 4, 5, 6, 7, 8], buffered.inner.as_slice());
    }

    #[tokio::test]
    async fn bypass_large_write_when_empty() {
        let mut inner = Vec::new();
        let mut buf = [0; 8];
        let mut buffered = BufferedWrite::new(&mut inner, &mut buf);

        assert_eq!(8, buffered.write(&[1, 2, 3, 4, 5, 6, 7, 8]).await.unwrap());
        assert_eq!(0, buffered.pos);
        assert_eq!(8, buffered.inner.len());
    }

    #[tokio::test]
    async fn large_write_when_not_empty() {
        let mut inner = Vec::new();
        let mut buf = [0; 8];
        let mut buffered = BufferedWrite::new(&mut inner, &mut buf);

        assert_eq!(1, buffered.write(&[1]).await.unwrap());
        assert_eq!(1, buffered.pos);
        assert_eq!(0, buffered.inner.len());

        assert_eq!(7, buffered.write(&[2, 3, 4, 5, 6, 7, 8, 9]).await.unwrap());
        assert_eq!(0, buffered.pos);
        assert_eq!(8, buffered.inner.len());
    }

    #[tokio::test]
    async fn flush_clears_buffer() {
        let mut inner = Vec::new();
        let mut buf = [0; 8];
        let mut buffered = BufferedWrite::new(&mut inner, &mut buf);

        assert_eq!(2, buffered.write(&[1, 2]).await.unwrap());
        assert_eq!(2, buffered.pos);
        assert_eq!(0, buffered.inner.len());

        buffered.flush().await.unwrap();
        assert_eq!(0, buffered.pos);
        assert_eq!(2, buffered.inner.len());
    }
}
