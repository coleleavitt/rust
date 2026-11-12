// Two checkouts of one crate at different paths share one incremental directory. Each build runs
// in its checkout and remaps that checkout to `.` with `--remap-path-prefix`, as cargo's
// `trim-paths` does. The absolute FROM path must not drop the shared incremental cache:
// only what the remapping produces may be tracked (#132132).
//
//@ ignore-cross-compile

use run_make_support::{cwd, rfs, rustc};

fn build(checkout: &str, incr_state: &str) {
    let dir = cwd().join(checkout);
    rustc()
        .current_dir(&dir)
        .input("src/lib.rs")
        .crate_type("lib")
        .incremental(cwd().join("incr"))
        .arg(format!("--remap-path-prefix={}=.", dir.display()))
        .arg(format!("-Zassert-incr-state={incr_state}"))
        .run();
}

fn main() {
    for checkout in ["a", "b"] {
        rfs::create_dir_all(format!("{checkout}/src"));
        rfs::copy("lib.rs", format!("{checkout}/src/lib.rs"));
    }
    build("a", "not-loaded");
    build("b", "loaded");
    build("a", "loaded");
}
