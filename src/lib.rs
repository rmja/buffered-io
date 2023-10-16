#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![feature(impl_trait_projections)]
#![feature(async_fn_in_trait)]
#![allow(stable_features, unknown_lints, async_fn_in_trait)]

#[cfg(feature = "async")]
pub mod asynch;
