# Buffer Types for embedded-io

[![CI](https://github.com/rmja/buffered-io/actions/workflows/ci.yaml/badge.svg)](https://github.com/rmja/buffered-io/actions/workflows/ci.yaml)
[![crates.io](https://img.shields.io/crates/v/buffered-io.svg)](https://crates.io/crates/buffered-io)
[![docs.rs](https://docs.rs/buffered-io/badge.svg)](https://docs.rs/buffered-io)

The `buffered-io` crate implements buffering for the `embedded-io`/`embedded-io-async` `Read` and `Write` traits.

## Example

```rust
tokio_test::block_on(async {
    use buffered_io::asynch::BufferedWrite;
    use embedded_io_async::Write;
    
    let uart_tx = Vec::new(); // The underlying uart peripheral implementing Write to where buffered bytes are written
    let mut write_buf = [0; 120];
    let mut buffering = BufferedWrite::new(uart_tx, &mut write_buf);
    buffering.write(b"hello").await.unwrap(); // This write is buffered
    buffering.write(b" ").await.unwrap(); // This write is also buffered
    buffering.write(b"world").await.unwrap(); // This write is also buffered
    buffering.flush().await.unwrap(); // The string "hello world" is written to uart in one write
})
```