//! Item-level reuse for `-Zpublic-api-hash`.
//!
//! A crate's public span hash changes whenever any of its span-sensitive items moves. A dependent
//! only copies the positions of the items it actually used: the MIR it instantiated, inlined or
//! evaluated, and the macros it expanded. `write_span_deps` records those items and their span
//! digests next to the dependent's `.rmeta`, and `check_span_reuse` (`-Zcheck-span-reuse`) tells
//! a build system whether they are still the same after a dependency was rebuilt with an unchanged
//! public API hash.
//!
//! A skipped dependent also keeps its cached warnings and its type debuginfo, which can show
//! positions of items that are not span-sensitive (a note pointing at a dependency's const or
//! type). Those are recorded as the leading lines of the dependency source file they lie in, so
//! the check also has to run when the dependency's public span hash did not change.

use std::fmt::Write as _;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustc_data_structures::fingerprint::Fingerprint;
use rustc_data_structures::fx::{FxHashMap, FxIndexMap, FxIndexSet};
use rustc_data_structures::stable_hash::StableHasher;
use rustc_data_structures::svh::Svh;
use rustc_hashes::Hash64;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::{OutFileName, OutputType};
use rustc_session::output::filename_for_metadata;
use rustc_span::def_id::{CrateNum, LOCAL_CRATE};
use rustc_span::{FileName, SourceFile, SourceFileHash};
use rustc_target::spec::Target;

use crate::creader::{CStore, MetadataLoader};
use crate::locator::{CrateFlavor, get_metadata_section};
use crate::rmeta::MetadataBlob;
use crate::rmeta::decoder::CrateMetadata;
use crate::rmeta::encoder::source_file_key;

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

/// The previous `.rdr-spans` file: object files that incremental compilation reused still contain
/// the positions its items had, although this session did not read them again. Its source file
/// entries are kept too, because the build system replays all diagnostics of the last build.
#[derive(Default)]
struct Previous {
    /// Stable crate id and local `DefPathHash` of each item.
    items: Vec<(u64, u64)>,
    /// Stable crate id, metadata name and line count of each source file entry.
    files: Vec<(u64, String, usize)>,
}

fn read_previous(path: &Path) -> Previous {
    let Ok(text) = std::fs::read_to_string(path) else { return Previous::default() };
    let mut previous = Previous::default();
    let mut krate = None;
    for line in text.lines() {
        if let Some(entry) = line.strip_prefix("file\t") {
            if let (Some(krate), Some(entry)) = (krate, FileEntry::parse(entry)) {
                previous.files.push((krate, entry.name.to_owned(), entry.lines));
            }
            continue;
        }
        let mut words = line.split(' ');
        match words.next() {
            Some("crate") => krate = words.nth(1).and_then(|id| u64::from_str_radix(id, 16).ok()),
            Some("item") => {
                if let (Some(krate), Some(Ok(hash))) =
                    (krate, words.next().map(|h| u64::from_str_radix(h, 16)))
                {
                    previous.items.push((krate, hash));
                }
            }
            _ => {}
        }
    }
    previous
}

/// A dependency source file that diagnostics (or type debuginfo) of the dependent show positions
/// in: `file\t<metadata name>\t<source hash>\t<local path>\t<lines>\t<prefix hash>`. The
/// shown positions all lie in the first `lines` lines, so the dependent stays valid as long as
/// those lines are unchanged, whatever happens below them.
struct FileEntry<'a> {
    name: &'a str,
    src_hash: &'a str,
    local_path: &'a str,
    lines: usize,
    prefix_hash: &'a str,
}

impl<'a> FileEntry<'a> {
    fn parse(entry: &'a str) -> Option<Self> {
        let [name, src_hash, local_path, lines, prefix_hash] =
            entry.split('\t').collect::<Vec<_>>()[..]
        else {
            return None;
        };
        Some(FileEntry { name, src_hash, local_path, lines: lines.parse().ok()?, prefix_hash })
    }
}

/// Hashes the first `lines` lines of `src`, ignoring a byte order mark and the line ending style,
/// so that the source as read from disk and the normalized source of the source map agree.
fn prefix_hash(src: &str, lines: usize) -> String {
    let mut hasher = StableHasher::new();
    for line in src.strip_prefix('\u{feff}').unwrap_or(src).lines().take(lines) {
        line.hash(&mut hasher);
    }
    Svh::new(hasher.finish::<Fingerprint>()).to_hex()
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
    let previous = read_previous(&path);
    let mut items: FxIndexSet<(CrateNum, u64)> = previous
        .items
        .into_iter()
        .filter_map(|(krate, hash)| Some((*crate_by_id.get(&krate)?, hash)))
        .collect();
    let mut reads: Vec<_> = cstore.span_sensitive_reads.lock().iter().copied().collect();
    reads.sort_by_key(|def_id| tcx.def_path_hash(*def_id));
    items.extend(
        reads
            .into_iter()
            .map(|def_id| (def_id.krate, tcx.def_path_hash(def_id).local_hash().as_u64())),
    );

    // Dependency source files that cached diagnostics (or, with
    // `-Zdebug-info-type-line-numbers`, type debuginfo) show positions in, with the number of
    // lines that contain all of those positions.
    let source_map = tcx.sess.source_map();
    let mut files: FxIndexMap<(CrateNum, String), (Arc<SourceFile>, usize)> = Default::default();
    let mut add_file = |file: Arc<SourceFile>, lines: usize| {
        if file.cnum == LOCAL_CRATE || cstore.get_crate_data(file.cnum).root.rdr_hashes.is_none() {
            return;
        }
        let key = (file.cnum, source_file_key(&file).0);
        let entry = files.entry(key).or_insert((file, 0));
        entry.1 = entry.1.max(lines);
    };
    let mut spans = tcx.dcx().emitted_spans();
    spans.extend(cstore.def_span_reads.lock().iter().map(|&def_id| tcx.def_span(def_id)));
    for span in spans {
        if !span.is_dummy() {
            let hi = source_map.lookup_char_pos(span.hi());
            add_file(hi.file, hi.line);
        }
    }
    let dep_files: FxHashMap<(CrateNum, String), Arc<SourceFile>> = source_map
        .files()
        .iter()
        .map(|file| ((file.cnum, source_file_key(file).0), Arc::clone(file)))
        .collect();
    for (krate, name, lines) in previous.files {
        if let Some(&cnum) = crate_by_id.get(&krate)
            && let Some(file) = dep_files.get(&(cnum, name))
        {
            add_file(Arc::clone(file), lines);
        }
    }

    let mut out = String::new();
    for &cnum in tcx.crates(()) {
        let mut hashes: Vec<u64> =
            items.iter().filter(|(krate, _)| *krate == cnum).map(|&(_, hash)| hash).collect();
        let crate_files: Vec<&(Arc<SourceFile>, usize)> =
            files.iter().filter(|((krate, _), _)| *krate == cnum).map(|(_, file)| file).collect();
        if hashes.is_empty() && crate_files.is_empty() {
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
        for (file, lines) in crate_files {
            let (name, src_hash) = source_file_key(file);
            let local_path = match &file.name {
                FileName::Real(real) => real.local_path().map(|path| path.display().to_string()),
                _ => None,
            };
            // Without the source, only an unchanged file keeps the dependent valid.
            let prefix = source_map
                .ensure_source_file_source_present(file)
                .then(|| file.external_src.read().get_source().map(|src| prefix_hash(src, *lines)))
                .flatten();
            writeln!(
                out,
                "file\t{name}\t{src_hash}\t{}\t{lines}\t{}",
                local_path.as_deref().unwrap_or("-"),
                prefix.as_deref().unwrap_or("none"),
            )
            .unwrap();
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
    let text = std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let mut current: Option<(MetadataBlob, String)> = None;
    for line in text.lines() {
        let words: Vec<&str> = line.split(' ').collect();
        match words.as_slice() {
            ["crate", name, id, file] if !line.starts_with("file\t") => {
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
                    return Ok(Some(format!(
                        "crate `{name}` at {} changed identity",
                        file.display()
                    )));
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
            _ if let Some(entry) = line.strip_prefix("file\t").and_then(FileEntry::parse) => {
                let Some((blob, krate)) = &current else { return Err("file before crate".into()) };
                if !file_still_matches(blob, &entry) {
                    let name = entry.name;
                    return Ok(Some(format!("shown lines of `{name}` in `{krate}` changed")));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}

/// Whether the lines a dependent's diagnostics show from a dependency source file are the same in
/// the dependency on disk. The source is read from the recorded local path and must be the one the
/// dependency was built from.
fn file_still_matches(blob: &MetadataBlob, entry: &FileEntry<'_>) -> bool {
    let root = blob.get_root();
    let Some(now) = blob.rdr_source_file_hash(&root, entry.name) else { return false };
    if now == entry.src_hash {
        return true;
    }
    if entry.prefix_hash == "none" {
        return false;
    }
    let Some(kind) = now.split_once('=').and_then(|(kind, _)| kind.parse().ok()) else {
        return false;
    };
    let Ok(src) = std::fs::read_to_string(entry.local_path) else { return false };
    SourceFileHash::new_in_memory(kind, &src).to_string() == now
        && prefix_hash(&src, entry.lines) == entry.prefix_hash
}
