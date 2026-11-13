#![crate_name = "mid"]
#![crate_type = "rlib"]

pub fn reset() {
    dep::START.count = 1;
}

#[inline(never)]
pub fn handle(h: dep::Handle) -> u64 {
    h.id
}

pub fn below() -> u32 {
    dep::below()
}
