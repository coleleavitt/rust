#![crate_name = "dep"]
#![crate_type = "rlib"]

pub struct Counter {
    pub count: u32,
}

// SHIFT_ABOVE_CONST
pub const START: Counter = Counter { count: 0 };

// SHIFT_ABOVE_HANDLE
pub struct Handle {
    pub id: u64,
}

// SHIFT_BELOW
pub fn below() -> u32 {
    7
}
