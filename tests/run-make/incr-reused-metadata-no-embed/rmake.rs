// An unchanged rebuild with `-Zembed-metadata=no` reuses the incremental metadata work product.
// The rlib must still get its metadata stub, or rustc fails to read it back.
//
//@ ignore-cross-compile

use run_make_support::rustc;

fn main() {
    for _ in 0..2 {
        rustc()
            .input("lib.rs")
            .crate_type("lib")
            .emit("metadata,link")
            .arg("-Zembed-metadata=no")
            .incremental("incr")
            .run();
    }
}
