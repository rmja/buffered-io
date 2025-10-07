use embedded_io_async::{Read, Write};

use super::BufferedWrite;

impl<T: Read> Read for BufferedWrite<'_, T> {
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
        assert!(buffered > 0);

        let mut new_pos = self.pos;
        self.buf[new_pos..new_pos + buffered].copy_from_slice(&buf[..buffered]);
        new_pos += buffered;

        if new_pos < self.buf.len() {
            // The buffer to write could fit in the buffer
            self.pos = new_pos;
        } else {
            // The buffer is full
            let written = self.inner.write(self.buf).await?;

            // We only assign self.pos _after_ we are sure that the write has completed successfully
            if written < new_pos {
                // We only partially wrote the inner buffer
                self.buf.copy_within(written..new_pos, 0);
                self.pos = new_pos - written;
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
mod async_tests {
    use embedded_io::{Error, ErrorKind, ErrorType};
    use embedded_io_async::Write;

    use super::BufferedWrite;

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
    async fn large_write_when_not_empty_can_handle_write_errors() {
        let mut inner = UnstableWrite::default();
        inner.writeable.push(0); // Return error
        inner.writeable.push(8); // Write all bytes
        let mut buf = [0; 8];
        let mut buffered = BufferedWrite::new(&mut inner, &mut buf);

        assert_eq!(1, buffered.write(&[1]).await.unwrap());
        assert_eq!(1, buffered.pos);
        assert_eq!(0, buffered.inner.written.len());

        assert!(buffered.write(&[2, 3, 4, 5, 6, 7, 8]).await.is_err());

        assert_eq!(7, buffered.write(&[2, 3, 4, 5, 6, 7, 8]).await.unwrap());
        assert_eq!(0, buffered.pos);
        assert_eq!(8, buffered.inner.written.len());
    }

    #[derive(Default)]
    struct UnstableWrite {
        written: Vec<u8>,
        writes: usize,
        writeable: Vec<usize>,
    }

    #[derive(Debug)]
    struct UnstableError;

    impl core::fmt::Display for UnstableError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "UnstableError")
        }
    }

    impl std::error::Error for UnstableError {}

    impl Error for UnstableError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    impl ErrorType for UnstableWrite {
        type Error = UnstableError;
    }

    impl Write for UnstableWrite {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            let written = self.writeable[self.writes];
            self.writes += 1;
            if written > 0 {
                self.written.extend_from_slice(&buf[..written]);
                Ok(written)
            } else {
                Err(UnstableError)
            }
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
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
