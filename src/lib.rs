#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
extern crate alloc;

pub mod contracts;
pub mod data_structures;

#[cfg(not(target_arch = "wasm32"))]
pub mod test_context;
