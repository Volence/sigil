//! The build directory: where a build writes what it generates, so that it never
//! writes into the tree it builds (G0b Task 6, spec §5.4, Review Focus 4).
//!
//! Each build gets a private directory, created fresh under the system temporary
//! directory and removed when the build ends. Its `generated/` subdirectory is the
//! build's `generated` root ([`sigil_frontend_emp::generated_root`]), mounted at
//! [`GENERATED_MOUNT`] in the tree: the sound seams write there, and an
//! `embed("engine/sound/generated/...")` reads there. The tree is only read.
//!
//! The mount path is an engine fact the seams already hold in Rust (they wrote into it
//! before), and it goes when G0c removes the seams. It is not a description field: the
//! descriptions carry nothing from the seams (spec §5.4).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sigil_frontend_emp::generated_root::{self, GeneratedRoot};

/// The tree path the generated root is mounted at, relative to the checkout: where
/// aeon's sources `embed` the sound seams' output from.
pub const GENERATED_MOUNT: &str = "engine/sound/generated";

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

    /// The generated root this build directory provides for `checkout`.
    pub fn root_for(&self, checkout: &Path) -> Result<GeneratedRoot, String> {
        generated_root_for(checkout, &self.generated)
    }
}

impl Drop for BuildDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The generated root that mounts `dir` at [`GENERATED_MOUNT`] in `checkout`.
pub fn generated_root_for(checkout: &Path, dir: &Path) -> Result<GeneratedRoot, String> {
    let tree = std::fs::canonicalize(checkout).map_err(|e| format!("resolve the checkout {}: {e}", checkout.display()))?;
    let dir = std::fs::canonicalize(dir).map_err(|e| format!("resolve the generated root {}: {e}", dir.display()))?;
    if dir.starts_with(&tree) {
        return Err(format!(
            "the generated root {} is inside the checkout {} it is for, and a build writes nothing into the tree it builds",
            dir.display(),
            tree.display()
        ));
    }
    Ok(GeneratedRoot { mount: tree.join(GENERATED_MOUNT), dir })
}

/// Run `f` with `checkout`'s generated root in force, handing it the generated
/// directory: the root already entered on this thread when it is `checkout`'s (a caller
/// that owns the build directory, such as `sigil build`, which also places the build's
/// reads with it), otherwise a fresh [`BuildDir`] that lives exactly as long as `f`.
pub fn with_generated_root<T, E: From<String>>(checkout: &Path, f: impl FnOnce(&Path) -> Result<T, E>) -> Result<T, E> {
    let tree = std::fs::canonicalize(checkout).map_err(|e| E::from(format!("resolve the checkout {}: {e}", checkout.display())))?;
    if let Some(current) = generated_root::current() {
        if current.mount == tree.join(GENERATED_MOUNT) {
            return f(&current.dir);
        }
    }
    let dir = BuildDir::create().map_err(E::from)?;
    let root = dir.root_for(checkout).map_err(E::from)?;
    // The directory handed to `f` is the one the root resolves embeds into, so the seams'
    // writes and the lowering's reads cannot name two places.
    let generated = root.dir.clone();
    let _root = generated_root::enter(root);
    f(&generated)
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
        let err = generated_root_for(outer.path(), &inner).unwrap_err();
        assert!(err.contains("inside the checkout"), "{err}");
    }

    #[test]
    fn with_generated_root_reuses_the_entered_root_for_its_checkout_only() {
        let tree = BuildDir::create().unwrap();
        let owned = BuildDir::create().unwrap();
        let _g = generated_root::enter(owned.root_for(tree.path()).unwrap());
        let seen = with_generated_root::<_, String>(tree.path(), |d| Ok(d.to_path_buf())).unwrap();
        assert_eq!(seen, owned.generated());
        let other = BuildDir::create().unwrap();
        let fresh = with_generated_root::<_, String>(other.path(), |d| {
            assert_eq!(generated_root::current().unwrap().dir, d);
            Ok(d.to_path_buf())
        })
        .unwrap();
        assert_ne!(fresh, owned.generated());
        assert!(!fresh.exists(), "a build directory made for one call is removed after it");
        assert_eq!(generated_root::current().unwrap().dir, owned.generated(), "the outer root is restored");
    }
}
