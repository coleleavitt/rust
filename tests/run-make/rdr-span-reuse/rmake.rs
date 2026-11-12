// Item-level reuse with `-Zpublic-api-hash`: a dependent writes the span-sensitive items of its
// dependencies that it used (`libmid.rdr-spans`), and `-Zcheck-span-reuse` tells whether those
// items kept their positions after a dependency changed. Moving an item `mid` does not use keeps
// `mid` reusable even though `dep`'s crate-level span hash changed; moving an item it uses does
// not. In every step `top` must print what a from-scratch build prints.
//
//@ ignore-cross-compile

use run_make_support::{rfs, run, rustc};

fn span_hash(rmeta: &str) -> String {
    let out = rustc().arg("-Zls=public_span_hash").input(rmeta).run().stdout_utf8();
    out.lines().find_map(|l| l.strip_prefix("Public span hash:")).unwrap().trim().to_owned()
}

fn reusable(spans: &str) -> bool {
    let out = rustc().arg("-Zcheck-span-reuse").input(spans).run().stdout_utf8();
    match out.trim() {
        "span-reuse: yes" => true,
        no if no.starts_with("span-reuse: no") => false,
        other => panic!("unexpected output {other:?}"),
    }
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
        .edition("2021")
        .arg("-Zpublic-api-hash")
        .incremental(format!("{dir}/incr-top"))
        .out_dir(dir)
        .extern_("mid", format!("{dir}/libmid.rlib"))
        .library_search_path(dir)
        .run();
    run(&format!("{dir}/top")).stdout_utf8()
}

fn clean_output(tag: &str) -> String {
    let dir = format!("clean-{tag}");
    rfs::create_dir(&dir);
    build_lib(&dir, "dep");
    build_lib(&dir, "mid");
    build_top(&dir)
}

fn shift(marker: &str) {
    let src = rfs::read_to_string("dep.rs");
    rfs::write("dep.rs", src.replacen(marker, &format!("{marker}\n\n\n"), 1));
}

/// Moves lines in `dep`, rebuilds it, rebuilds `mid` only if `-Zcheck-span-reuse` says so, and
/// relinks `top`. Returns whether `mid` was reusable.
fn step(marker: &str, tag: &str) -> bool {
    let before = span_hash("inc/libdep.rmeta");
    shift(marker);
    build_lib("inc", "dep");
    assert_ne!(before, span_hash("inc/libdep.rmeta"), "{tag}: crate span hash did not change");
    let reuse = reusable("inc/libmid.rdr-spans");
    if !reuse {
        build_lib("inc", "mid");
    }
    assert_eq!(build_top("inc"), clean_output(tag), "{tag}");
    reuse
}

fn main() {
    rfs::create_dir("inc");
    build_lib("inc", "dep");
    build_lib("inc", "mid");
    assert_eq!(build_top("inc"), clean_output("0"));

    assert!(step("// SHIFT_ABOVE_UNUSED", "unused"), "moving an unused item forced a rebuild");
    assert!(!step("// SHIFT_ABOVE_MACRO", "macro"), "moving a used macro allowed reuse");
    assert!(!step("use std::panic::Location;", "generic"), "moving a used fn allowed reuse");
}
