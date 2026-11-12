#![crate_name = "mid"]
#![crate_type = "rlib"]

pub fn report() -> String {
    format!(
        "generic {} | caller {} | plain {} | private {}",
        dep::generic_location::<u32>(),
        dep::caller_location(),
        dep::plain_location(),
        dep::uses_private(),
    )
}

pub fn boom() {
    dep::inline_boom()
}
