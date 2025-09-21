#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Ergonomic Rust wrapper for the OpenAI API.
//!
//! This crate provides a type-safe, builder-pattern interface to interact with
//! `OpenAI` API endpoints, making it easy to integrate AI capabilities into
//! your Rust applications.

pub use bon;

/// Test utilities module (available for both unit and integration tests)
pub mod test_utils;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
