#![crate_name = "dep"]
#![crate_type = "rlib"]

mod private;
mod public;

pub use public::C;
