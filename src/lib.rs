#![cfg_attr(not(test), no_std)]
#![allow(incomplete_features)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]

#[cfg(feature = "async")]
pub mod asynch;
