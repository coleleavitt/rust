#![crate_name = "mid"]
#![crate_type = "rlib"]

pub fn report() -> String {
    format!("{} {}", dep::used_generic::<u32>(), dep::here!())
}
