#![cfg_attr(not(test), no_std)]
#![allow(unknown_lints, async_fn_in_trait)]
#![allow(stable_features)]
#![feature(async_fn_in_trait)]

#[cfg(feature = "async")]
pub mod asynch;
