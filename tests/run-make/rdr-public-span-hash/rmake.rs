// `-Zpublic-api-hash` splits what a dependent depends on into two hashes:
// `public_api_hash` (no source positions) and `public_span_hash` (the absolute positions that
// dependents copy into their own code: generic, `#[inline]` and const fn MIR, macro definitions).
// A dependent is rebuilt when either changes. This test edits `dep.rs` the way a user would,
// decides like a build system whether `mid` must be rebuilt, always relinks `top`, and checks that
// `top` prints exactly what a from-scratch build of all three crates prints: locations, panic
// messages and values.
//
//@ ignore-cross-compile
//@ needs-unwind

use run_make_support::{rfs, run, rustc};

fn hashes(rmeta: &str) -> (String, String) {
    let get = |kind: &str, prefix: &str| {
        let out = rustc().arg(format!("-Zls={kind}")).input(rmeta).run().stdout_utf8();
        out.lines().find_map(|l| l.strip_prefix(prefix)).unwrap().trim().to_owned()
    };
    (get("public_hash", "Public hash:"), get("public_span_hash", "Public span hash:"))
}

fn build_lib(dir: &str, name: &str) {
    let mut cmd = rustc();
    cmd.input(format!("{name}.rs"))
        .edition("2021")
        .emit("metadata,link")
        .arg("-Zpublic-api-hash")
        .incremental(format!("{dir}/incr-{name}"))
        .out_dir(dir);
    if name == "mid" {
        cmd.extern_("dep", format!("{dir}/libdep.rlib"));
    }
    cmd.run();
}

fn build_top(dir: &str) -> String {
    rustc()
        .input("top.rs")
        .crate_name("top")
        .edition("2021")
        .arg("-Zpublic-api-hash")
        .incremental(format!("{dir}/incr-top"))
        .out_dir(dir)
        .library_search_path(dir)
        .extern_("mid", format!("{dir}/libmid.rlib"))
        .run();
    run(&format!("{dir}/top")).stdout_utf8()
}

/// Builds all three crates in a fresh directory.
fn clean_output(tag: &str) -> String {
    let dir = format!("clean-{tag}");
    rfs::create_dir(&dir);
    build_lib(&dir, "dep");
    build_lib(&dir, "mid");
    build_top(&dir)
}

/// Rebuilds `dep` in the incremental directory, `mid` only if one of `dep`'s hashes changed, and
/// relinks `top`. Returns which hashes changed and what `top` printed.
fn incremental_output(prev: &mut (String, String)) -> (bool, bool, String) {
    build_lib("inc", "dep");
    let new = hashes("inc/libdep.rmeta");
    let (api_changed, span_changed) = (new.0 != prev.0, new.1 != prev.1);
    if api_changed || span_changed {
        build_lib("inc", "mid");
    }
    *prev = new;
    (api_changed, span_changed, build_top("inc"))
}

fn edit(from: &str, to: &str) {
    let src = rfs::read_to_string("dep.rs");
    assert!(src.contains(from));
    rfs::write("dep.rs", src.replacen(from, to, 1));
}

fn main() {
    rfs::create_dir("inc");
    build_lib("inc", "dep");
    build_lib("inc", "mid");
    let mut prev = hashes("inc/libdep.rmeta");
    assert_eq!(build_top("inc"), clean_output("0"));

    // A private edit in the crate root file keeps both hashes: `mid` is not rebuilt.
    edit("const PRIVATE_VALUE: u32 = 1;", "const PRIVATE_VALUE: u32 = 2;");
    let (api, span, out) = incremental_output(&mut prev);
    assert!(!api && !span, "private edit in lib.rs changed api={api} span={span}");
    assert_eq!(out, clean_output("1"));

    // Lines inserted above a non-generic function: its location comes from `dep`'s own object
    // code, so both hashes stay and relinking `top` is enough.
    edit("// SHIFT_ABOVE_PLAIN", "// SHIFT_ABOVE_PLAIN\n\n\n");
    let (api, span, out) = incremental_output(&mut prev);
    assert!(!api && !span, "line shift above a plain fn changed api={api} span={span}");
    assert_eq!(out, clean_output("2"));

    // Lines inserted above the generic and `#[inline]` functions: `mid` contains their code with
    // `dep`'s positions, so the span hash changes (and only it) and `mid` is rebuilt.
    edit("use std::panic::Location;", "use std::panic::Location;\n\n\n\n");
    let (api, span, out) = incremental_output(&mut prev);
    assert!(!api && span, "line shift above generic fns: api={api} span={span}");
    assert_eq!(out, clean_output("3"));
}
