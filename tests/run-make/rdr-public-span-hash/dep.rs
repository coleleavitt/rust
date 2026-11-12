#![crate_name = "dep"]
#![crate_type = "rlib"]

use std::panic::Location;

/// Generic: dependents instantiate it, so its span positions end up in their code.
pub fn generic_location<T>() -> &'static Location<'static> {
    Location::caller()
}

#[inline]
pub fn inline_boom() {
    panic!("inline boom")
}

#[track_caller]
pub fn caller_location() -> &'static Location<'static> {
    Location::caller()
}

// SHIFT_ABOVE_PLAIN

pub fn plain_location() -> &'static Location<'static> {
    Location::caller()
}

fn private_helper() -> u32 {
    PRIVATE_VALUE
}

const PRIVATE_VALUE: u32 = 1;

pub fn uses_private() -> u32 {
    private_helper()
}
