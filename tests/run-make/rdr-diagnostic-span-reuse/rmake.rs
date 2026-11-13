// A build system that skips a dependent replays the dependent's cached warnings, and keeps its
// object files. `-Zpublic-api-hash` therefore records in `libmid.rdr-spans` the lines of dependency
// source files that the dependent's diagnostics (here a note pointing at `dep::START`) and, with
// `-Zdebug-info-type-line-numbers`, its type debuginfo (`dep::Handle`) show. `-Zcheck-span-reuse`
// must say no when lines above them move, and yes when only lines below them move; in every step
// the warnings and the debuginfo lines of `mid` must be what a from-scratch build produces.
//
//@ ignore-cross-compile

use run_make_support::{llvm_dwarfdump, rfs, rustc};

fn api_hash(rmeta: &str) -> String {
    let out = rustc().arg("-Zls=public_hash").input(rmeta).run().stdout_utf8();
    out.lines().find_map(|l| l.strip_prefix("Public hash:")).unwrap().trim().to_owned()
}

fn reusable(spans: &str) -> bool {
    let out = rustc().arg("-Zcheck-span-reuse").input(spans).run().stdout_utf8();
    match out.trim() {
        "span-reuse: yes" => true,
        no if no.starts_with("span-reuse: no") => false,
        other => panic!("unexpected output {other:?}"),
    }
}

fn build_dep(dir: &str) {
    rustc()
        .input("dep.rs")
        .edition("2021")
        .emit("metadata,link")
        .arg("-Zpublic-api-hash")
        .incremental(format!("{dir}/incr-dep"))
        .out_dir(dir)
        .run();
}

/// Builds `mid` and returns its warnings.
fn build_mid(dir: &str) -> String {
    rustc()
        .input("mid.rs")
        .edition("2021")
        .emit("metadata,link")
        .arg("-Zpublic-api-hash")
        .arg("-Cdebuginfo=2")
        .arg("-Zdebug-info-type-line-numbers")
        .incremental(format!("{dir}/incr-mid"))
        .out_dir(dir)
        .extern_("dep", format!("{dir}/libdep.rlib"))
        .run()
        .stderr_utf8()
}

fn decl_lines(dir: &str) -> Vec<String> {
    let out = llvm_dwarfdump().arg("--debug-info").input(format!("{dir}/libmid.rlib")).run();
    out.stdout_utf8().lines().filter(|l| l.contains("DW_AT_decl_line")).map(str::to_owned).collect()
}

/// The warnings and debuginfo lines of a from-scratch build of the current sources.
fn clean(tag: &str) -> (String, Vec<String>) {
    let dir = format!("clean-{tag}");
    rfs::create_dir(&dir);
    build_dep(&dir);
    let warnings = build_mid(&dir);
    (warnings, decl_lines(&dir))
}

fn shift(marker: &str) {
    let src = rfs::read_to_string("dep.rs");
    rfs::write("dep.rs", src.replacen(marker, &format!("{marker}\n\n\n"), 1));
}

/// Moves lines in `dep` and rebuilds it. If `-Zcheck-span-reuse` says `mid` is reusable, its
/// cached warnings and object files must equal a clean build's; otherwise they must not (the
/// check is needed), and `mid` is rebuilt. Returns whether `mid` was reusable.
fn step(cached: &mut String, marker: &str, tag: &str) -> bool {
    let api = api_hash("inc/libdep.rmeta");
    shift(marker);
    build_dep("inc");
    assert_eq!(api, api_hash("inc/libdep.rmeta"), "{tag}: public API hash changed");
    let reuse = reusable("inc/libmid.rdr-spans");
    let expected = clean(tag);
    let skipped = (cached.clone(), decl_lines("inc"));
    if reuse {
        assert_eq!(skipped, expected, "{tag}: reused `mid` differs from a clean build");
    } else {
        assert_ne!(skipped, expected, "{tag}: rebuild was not needed");
        *cached = build_mid("inc");
        assert_eq!((cached.clone(), decl_lines("inc")), expected, "{tag}");
    }
    reuse
}

fn main() {
    rfs::create_dir("inc");
    build_dep("inc");
    let mut cached = build_mid("inc");
    assert!(cached.contains("dep.rs:9:1"), "expected a note pointing into `dep`: {cached}");
    assert_eq!((cached.clone(), decl_lines("inc")), clean("0"));

    assert!(step(&mut cached, "// SHIFT_BELOW", "below"), "moving lines below forced a rebuild");
    assert!(!step(&mut cached, "// SHIFT_ABOVE_HANDLE", "debuginfo"), "moved type line reused");
    assert!(!step(&mut cached, "// SHIFT_ABOVE_CONST", "warning"), "moved warning note reused");
    assert!(step(&mut cached, "// SHIFT_BELOW", "below-again"), "moving lines below rebuilt");
}
