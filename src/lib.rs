#![cfg_attr(not(test), no_std)]
#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]

#[cfg(feature = "async")]
pub mod asynch;
