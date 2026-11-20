//! The build directory: where a build writes what it generates, so that it never
//! writes into the tree it builds (G0b Task 6, spec §5.4, Review Focus 4).
//!
//! Each build gets a private directory, created fresh under the system temporary
//! directory and removed when the build ends. Its `generated/` subdirectory is the
//! build's `generated` root ([`sigil_span::roots`], G0b Task 9): the sound seams write
//! into its `sound/` subdirectory ([`GENERATED_SOUND`]), and aeon's sources read them as
//! `embed("generated/sound/<file>")`. The tree is only read.
//!
//! The root is a named root of the build's lowering options, not a mount at a tree path:
//! nothing is keyed to the checkout, and a build that names no generated root refuses a
//! `generated/` path by name.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The subdirectory of the generated root the sound seams write into. A generated
/// root holds one directory per subsystem, as G0a's export `files` table names the
/// `generated` root's rows (`sound/...`, `debug/...`).
pub const GENERATED_SOUND: &str = "sound";

/// A build's private directory, removed on drop.
#[derive(Debug)]
pub struct BuildDir {
    root: PathBuf,
    generated: PathBuf,
}

impl BuildDir {
    /// Create a fresh, empty build directory with its `generated/` subdirectory. The
    /// name carries the process id, a per-process counter and the time, and the
    /// directory is created with `create_dir`, so two builds never share one.
    pub fn create() -> Result<BuildDir, String> {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let name = format!("sigil-build-{}-{}-{nanos}", std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed));
        let parent = std::env::temp_dir();
        let root = parent.join(name);
        std::fs::create_dir(&root).map_err(|e| format!("create the build directory {}: {e}", root.display()))?;
        // Canonical, so the read-set recorder's keys and the sandbox's joins name it the
        // same way the placement of a read does. Owned from here on, so a failure below
        // removes what was created.
        let root = std::fs::canonicalize(&root).map_err(|e| {
            let _ = std::fs::remove_dir(&root);
            format!("resolve {}: {e}", root.display())
        })?;
        let dir = BuildDir { generated: root.join("generated"), root };
        std::fs::create_dir(&dir.generated)
            .map_err(|e| format!("create the build directory {}: {e}", dir.generated.display()))?;
        Ok(dir)
    }

    /// The directory itself.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// The build's `generated` root directory.
    pub fn generated(&self) -> &Path {
        &self.generated
    }

    /// The directory the sound seams write into: [`GENERATED_SOUND`] under the
    /// generated root, created on first ask.
    pub fn sound(&self) -> Result<PathBuf, String> {
        sound_dir(&self.generated)
    }
}

impl Drop for BuildDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The sound seams' directory under the generated root `generated`, created if absent.
pub fn sound_dir(generated: &Path) -> Result<PathBuf, String> {
    let dir = generated.join(GENERATED_SOUND);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Refuse a generated root inside the checkout it serves: a build writes nothing into
/// the tree it builds.
pub fn check_outside(checkout: &Path, generated: &Path) -> Result<(), String> {
    let tree = std::fs::canonicalize(checkout).map_err(|e| format!("resolve the checkout {}: {e}", checkout.display()))?;
    let dir = std::fs::canonicalize(generated).map_err(|e| format!("resolve the generated root {}: {e}", generated.display()))?;
    if dir.starts_with(&tree) {
        return Err(format!(
            "the generated root {} is inside the checkout {} it is for, and a build writes nothing into the tree it builds",
            dir.display(),
            tree.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_dir_is_fresh_outside_the_tree_and_removed_on_drop() {
        let a = BuildDir::create().unwrap();
        let b = BuildDir::create().unwrap();
        assert_ne!(a.path(), b.path());
        assert!(a.generated().is_dir());
        assert_eq!(std::fs::read_dir(a.generated()).unwrap().count(), 0);
        let path = a.path().to_path_buf();
        drop(a);
        assert!(!path.exists());
    }

    #[test]
    fn a_generated_root_inside_its_checkout_is_refused() {
        let outer = BuildDir::create().unwrap();
        let inner = outer.path().join("engine/sound/generated");
        std::fs::create_dir_all(&inner).unwrap();
        let err = check_outside(outer.path(), &inner).unwrap_err();
        assert!(err.contains("inside the checkout"), "{err}");
        let other = BuildDir::create().unwrap();
        check_outside(outer.path(), other.generated()).unwrap();
    }

    #[test]
    fn the_sound_seams_write_under_the_generated_root() {
        let d = BuildDir::create().unwrap();
        let s = d.sound().unwrap();
        assert_eq!(s, d.generated().join("sound"));
        assert!(s.is_dir());
    }
}
