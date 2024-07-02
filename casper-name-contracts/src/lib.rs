#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
extern crate alloc;

pub mod contracts;
pub mod data_structures;

#[cfg(test)]
pub mod test_context;
