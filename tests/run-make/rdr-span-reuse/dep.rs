#![crate_name = "dep"]
#![crate_type = "rlib"]

use std::panic::Location;

pub fn used_generic<T>() -> &'static Location<'static> {
    Location::caller()
}

// SHIFT_ABOVE_MACRO

#[macro_export]
macro_rules! here {
    () => {
        $crate::used_generic::<u8>()
    };
}

// SHIFT_ABOVE_UNUSED

pub fn unused_generic<T>() -> &'static Location<'static> {
    Location::caller()
}
