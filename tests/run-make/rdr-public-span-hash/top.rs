fn main() {
    std::panic::set_hook(Box::new(|info| {
        println!("panic at {}", info.location().unwrap());
    }));
    println!("{}", mid::report());
    let _ = std::panic::catch_unwind(mid::boom);
}
