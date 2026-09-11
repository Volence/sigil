//! The `.lst` source digest for one native build: the files the build read, the module
//! scan's membership, the build configuration and the identity of the ROM it wrote,
//! shaped into a [`sigil_link::SourceDigest`] for [`sigil_link::emit_source_digest`].
//!
//! The rows come from the [`sigil_span::read_set`] snapshot and from nothing else. This
//! module never lists a file kind of its own, so it cannot fall behind what the build
//! opened. What it decides is how each recorded path is written (which root it lies
//! under, which origin it has), and when the snapshot is not a digest this build can
//! vouch for: a file read with two contents, a scan of a second tree, no scan at all, or
//! a scanned module with no read of it.

use sigil_link::{DigestOrigin, DigestPath, DigestRead, DigestRoot, SourceDigest};
use sigil_span::read_set::Snapshot;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// The root of the sigil checkout this crate was compiled from: the tree the assembler
/// reads its committed off-canonical size tables from at run time (`load_frozen_table`).
pub fn sigil_source_root() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = crate_dir.parent().and_then(Path::parent).unwrap_or(crate_dir);
    sigil_span::read_set::canonical(root)
}

/// What the assembler says it is: the facts `sigil --version` reports.
pub struct AssemblerIdentity<'a> {
    pub version: &'a str,
    pub revision: &'a str,
    pub tree_state: &'a str,
}

/// The build configuration a digest records.
pub struct DigestShape<'a> {
    pub target: &'a str,
    pub game: &'a str,
    pub debug: bool,
    pub extra_entries: &'a [String],
}

/// The roots a digest path may be relative to, in the order a path is tried.
struct Roots {
    aeon: PathBuf,
    sigil: PathBuf,
}

impl Roots {
    fn place(&self, path: &Path) -> Result<DigestPath, String> {
        let filesystem = PathBuf::from("/");
        for (root, base) in
            [(DigestRoot::Aeon, &self.aeon), (DigestRoot::Sigil, &self.sigil), (DigestRoot::Filesystem, &filesystem)]
        {
            if let Ok(rel) = path.strip_prefix(base) {
                return Ok(DigestPath { root, path: relative_text(rel, path)? });
            }
        }
        Err(format!("{} is not an absolute path, so it lies under no digest root", path.display()))
    }
}

/// `rel` as `/`-joined text, refusing a component the grammar cannot carry.
fn relative_text(rel: &Path, full: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(s) => parts.push(
                s.to_str().ok_or_else(|| format!("{} is not UTF-8, so the digest cannot name it", full.display()))?,
            ),
            other => {
                return Err(format!("{} has a `{other:?}` component under its root", full.display()));
            }
        }
    }
    Ok(parts.join("/"))
}

/// Where `-o` puts the ROM, as an absolute path with its directory's symlinks
/// resolved, so it is placed under the same roots the reads are.
fn output_location(output: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(output)
        .map_err(|e| format!("resolve the -o path {}: {e}", output.display()))?;
    match (absolute.parent(), absolute.file_name()) {
        (Some(dir), Some(name)) => match std::fs::canonicalize(dir) {
            Ok(dir) => Ok(dir.join(name)),
            Err(_) => Ok(absolute),
        },
        _ => Ok(absolute),
    }
}

/// The digest of one build, from the recorder's `snapshot` taken after the build's last
/// read. `rom` is the full shipped file exactly as it is about to be written to
/// `rom_output`.
pub fn source_digest(
    aeon: &Path,
    assembler: &AssemblerIdentity<'_>,
    shape: &DigestShape<'_>,
    defines: Vec<(String, i128)>,
    rom: &[u8],
    rom_output: Option<&Path>,
    snapshot: &Snapshot,
) -> Result<SourceDigest, String> {
    let aeon_root = std::fs::canonicalize(aeon)
        .map_err(|e| format!("resolve the aeon root {}: {e}", aeon.display()))?;
    let roots = Roots { aeon: aeon_root.clone(), sigil: sigil_source_root() };

    if !snapshot.conflicts.is_empty() {
        let lines: Vec<String> = snapshot.conflicts.iter().map(|c| format!("  {c}")).collect();
        return Err(format!(
            "the build's read set cannot be stated as one digest:\n{}",
            lines.join("\n")
        ));
    }

    // Exactly one module scan, of this tree.
    let mut scan = None;
    for s in &snapshot.scans {
        if s.root != aeon_root {
            return Err(format!(
                "the build scanned {} as well as its aeon root {}, and a digest names one tree",
                s.root.display(),
                aeon_root.display()
            ));
        }
        scan = Some(s);
    }
    let scan = scan.ok_or_else(|| {
        format!(
            "no module scan of {} was recorded, so the read set missed the walk every build performs",
            aeon_root.display()
        )
    })?;

    // Every scanned module was read, or a module could change without moving a row.
    let read_paths: BTreeSet<&Path> = snapshot.reads.iter().map(|r| r.path.as_path()).collect();
    let mut scanned = Vec::with_capacity(scan.files.len());
    for member in &scan.files {
        let on_disk = sigil_span::read_set::canonical(&aeon_root.join(member));
        if !read_paths.contains(on_disk.as_path()) {
            return Err(format!(
                "the module scan found {} and no read of it was recorded, so the digest would not \
                 name a module the build parsed",
                member.display()
            ));
        }
        scanned.push(relative_text(member, &aeon_root.join(member))?);
    }
    let (scan_files, scan_crc) = sigil_link::digest_scan_identity(&scanned);

    let mut reads = Vec::with_capacity(snapshot.reads.len());
    for r in &snapshot.reads {
        let file = roots.place(&r.path)?;
        let origin = if file.root != DigestRoot::Aeon {
            DigestOrigin::External
        } else if r.tool {
            DigestOrigin::Tool
        } else if r.generated {
            DigestOrigin::Generated
        } else {
            DigestOrigin::Source
        };
        reads.push(DigestRead { crc: r.crc, size: r.size, origin, file });
    }

    let rom_output = match rom_output {
        Some(p) => Some(roots.place(&output_location(p)?)?),
        None => None,
    };

    Ok(SourceDigest {
        assembler_version: assembler.version.to_string(),
        revision: assembler.revision.to_string(),
        tree_state: assembler.tree_state.to_string(),
        target: shape.target.to_string(),
        game: shape.game.to_string(),
        debug: shape.debug,
        extra_entries: shape.extra_entries.to_vec(),
        defines,
        scan_files,
        scan_crc,
        reads,
        rom_crc: sigil_span::read_set::crc32(rom),
        rom_size: rom.len() as u64,
        rom_output,
    })
}
