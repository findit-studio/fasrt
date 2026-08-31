//! A blazing fast, zero-copy subtitle parser and writer for SRT, WebVTT and
//! ASS/SSA in Rust.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![deny(missing_docs)]
#![allow(unused_extern_crates)]

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc as std;

#[cfg(feature = "std")]
extern crate std;

/// ASS/SSA subtitle parser and writer.
pub mod ass;

/// SRT subtitle parser.
pub mod srt;

/// WebVTT subtitle parser and writer.
pub mod vtt;

/// The types module contains the public types used by this crate.
pub mod types;

/// The error types.
pub mod error;

/// Utility functions
pub mod utils;
