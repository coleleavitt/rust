// A public constant whose value comes from a private constant. Changing only the private
// constant changes the value that dependents inline, so the public API hash must change even
// though no public source text, span or type changed.
//
//@ ignore-cross-compile

use run_make_support::{rfs, rustc};

fn public_hash() -> String {
    rustc().input("dep.rs").arg("-Zpublic-api-hash").run();
    let listing = rustc().arg("-Zls=public_hash").input("libdep.rlib").run().stdout_utf8();
    listing.lines().find_map(|line| line.strip_prefix("Public hash:")).unwrap().trim().to_owned()
}

fn main() {
    let before = public_hash();
    rfs::write("private.rs", "pub(crate) const V: u32 = 2;\n");
    let after = public_hash();
    assert_ne!(before, after, "public hash did not change when the value of a public const changed");
}
