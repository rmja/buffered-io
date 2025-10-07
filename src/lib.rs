#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]

mod read;
mod write;

pub use read::BufferedRead;
pub use write::BufferedWrite;

/// Unable to bypass the current buffered reader or writer because there are buffered bytes.
#[derive(Debug)]
pub struct BypassError;
