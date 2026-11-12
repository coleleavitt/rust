//! Item-level reuse for `-Zpublic-api-hash`.
//!
//! A crate's public span hash changes whenever any of its span-sensitive items moves. A dependent
//! only copies the positions of the items it actually used: the MIR it instantiated, inlined or
//! evaluated, and the macros it expanded. `write_span_deps` records those items and their span
//! digests next to the dependent's `.rmeta`, and `check_span_reuse` (`-Zcheck-span-reuse`) tells
//! a build system whether they are still the same after a dependency was rebuilt with an unchanged
//! public API hash.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use rustc_data_structures::fingerprint::Fingerprint;
use rustc_data_structures::fx::{FxHashMap, FxIndexSet};
use rustc_data_structures::svh::Svh;
use rustc_hashes::Hash64;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::{OutFileName, OutputType};
use rustc_session::output::filename_for_metadata;
use rustc_span::def_id::CrateNum;
use rustc_target::spec::Target;

use crate::creader::{CStore, MetadataLoader};
use crate::locator::{CrateFlavor, get_metadata_section};
use crate::rmeta::decoder::CrateMetadata;
use crate::rmeta::MetadataBlob;

/// Where the span deps of the local crate go: next to its `.rmeta`.
fn span_deps_path(tcx: TyCtxt<'_>) -> Option<PathBuf> {
    if !tcx.sess.opts.unstable_opts.public_api_hash
        || !tcx.sess.opts.output_types.contains_key(&OutputType::Metadata)
    {
        return None;
    }
    match filename_for_metadata(tcx.sess, tcx.output_filenames(())) {
        OutFileName::Real(path) => Some(path.with_extension("rdr-spans")),
        OutFileName::Stdout => None,
    }
}

/// The items of the previous `.rdr-spans` file: object files that incremental compilation reused
/// still contain the positions those items had, although this session did not read them again.
fn previous_items(path: &Path) -> Vec<(u64, u64)> {
    let Ok(text) = std::fs::read_to_string(path) else { return vec![] };
    let mut items = vec![];
    let mut krate = None;
    for line in text.lines() {
        let mut words = line.split(' ');
        match words.next() {
            Some("crate") => {
                krate = words.nth(1).and_then(|id| u64::from_str_radix(id, 16).ok())
            }
            Some("item") => {
                if let (Some(krate), Some(Ok(hash))) =
                    (krate, words.next().map(|h| u64::from_str_radix(h, 16)))
                {
                    items.push((krate, hash));
                }
            }
            _ => {}
        }
    }
    items
}

fn digest_text(digest: Option<Fingerprint>) -> String {
    match digest {
        Some(digest) => Svh::new(digest).to_hex(),
        None => "none".to_owned(),
    }
}

fn item_digest(cdata: &CrateMetadata, local_hash: u64) -> Option<Fingerprint> {
    let index = cdata.local_def_path_hash_to_def_index(Hash64::new(local_hash))?;
    cdata.root.rdr_hashes.as_ref()?.span.get(cdata, index)
}

/// Writes the dependencies' span-sensitive items this session used, with their current span
/// digests, to `<crate>.rdr-spans` next to the `.rmeta`.
pub fn write_span_deps(tcx: TyCtxt<'_>) {
    let Some(path) = span_deps_path(tcx) else { return };
    let cstore = CStore::from_tcx(tcx);

    let crate_by_id: FxHashMap<u64, CrateNum> =
        tcx.crates(()).iter().map(|&cnum| (tcx.stable_crate_id(cnum).as_u64(), cnum)).collect();
    let mut items: FxIndexSet<(CrateNum, u64)> = previous_items(&path)
        .into_iter()
        .filter_map(|(krate, hash)| Some((*crate_by_id.get(&krate)?, hash)))
        .collect();
    let mut reads: Vec<_> = cstore.span_sensitive_reads.lock().iter().copied().collect();
    reads.sort_by_key(|def_id| tcx.def_path_hash(*def_id));
    items.extend(reads.into_iter().map(|def_id| {
        (def_id.krate, tcx.def_path_hash(def_id).local_hash().as_u64())
    }));

    let mut out = String::new();
    for &cnum in tcx.crates(()) {
        let mut hashes: Vec<u64> =
            items.iter().filter(|(krate, _)| *krate == cnum).map(|&(_, hash)| hash).collect();
        if hashes.is_empty() {
            continue;
        }
        hashes.sort_unstable();
        let source = tcx.used_crate_source(cnum);
        let Some(file) = source.rmeta.as_ref().or(source.rlib.as_ref()) else { continue };
        let cdata = cstore.get_crate_data(cnum);
        writeln!(
            out,
            "crate {} {:016x} {}",
            tcx.crate_name(cnum),
            tcx.stable_crate_id(cnum).as_u64(),
            file.display()
        )
        .unwrap();
        for hash in hashes {
            writeln!(out, "item {hash:016x} {}", digest_text(item_digest(cdata, hash))).unwrap();
        }
    }
    if let Err(err) = std::fs::write(&path, out) {
        tcx.dcx().err(format!("failed to write `{}`: {err}", path.display()));
    }
}

/// Compares the span digests recorded in a `.rdr-spans` file with the dependencies on disk.
/// Returns `None` if the dependent can be reused, or the reason it must be rebuilt.
pub fn check_span_reuse(
    target: &Target,
    path: &Path,
    metadata_loader: &dyn MetadataLoader,
    cfg_version: &'static str,
) -> Result<Option<String>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let mut current: Option<(MetadataBlob, String)> = None;
    for line in text.lines() {
        let words: Vec<&str> = line.split(' ').collect();
        match words.as_slice() {
            ["crate", name, id, file] => {
                let file = Path::new(file);
                let flavor = if file.extension().is_some_and(|ext| ext == "rmeta") {
                    CrateFlavor::Rmeta
                } else {
                    CrateFlavor::Rlib
                };
                let blob =
                    get_metadata_section(target, flavor, file, metadata_loader, cfg_version, None)
                        .map_err(|err| format!("{}: {err:?}", file.display()))?;
                let found = format!("{:016x}", blob.get_root().stable_crate_id.as_u64());
                if found != *id {
                    return Ok(Some(format!("crate `{name}` at {} changed identity", file.display())));
                }
                current = Some((blob, name.to_string()));
            }
            ["item", hash, digest] => {
                let Some((blob, name)) = &current else { return Err("item before crate".into()) };
                let hash = u64::from_str_radix(hash, 16).map_err(|err| err.to_string())?;
                let root = blob.get_root();
                let now = blob
                    .local_def_path_hash_to_def_index(&root, Hash64::new(hash))
                    .and_then(|index| root.rdr_hashes.as_ref()?.span.get(blob, index));
                if digest_text(now) != *digest {
                    return Ok(Some(format!("item {hash:016x} of `{name}` moved")));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}
