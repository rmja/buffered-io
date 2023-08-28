# Buffer Types for embedded-io

[![CI](https://github.com/rmja/buffered-io/actions/workflows/ci.yaml/badge.svg)](https://github.com/rmja/buffered-io/actions/workflows/ci.yaml)
[![crates.io](https://img.shields.io/crates/v/buffered-io.svg)](https://crates.io/crates/buffered-io)
[![docs.rs](https://docs.rs/buffered-io/badge.svg)](https://docs.rs/buffered-io)

The `buffered-io` crate implements buffering for the `embedded-io`/`embedded-io-async` `Read` and `Write` traits.

## Example

```rust
let uart_tx = ...;
let mut write_buf = [0; 120];
let buffering = BufferedWrite::new(uart_tx, &mut write_buf);
buffering.write(b"hello").await?; // This write is buffered
buffering.write(b" ").await?; // This write is also buffered
buffering.write(b"world").await?; // This write is also buffered
buffering.flush().await?; // The string "hello world" is written to uart in one write
```