//! The build's `generated` root: a directory OUTSIDE the source tree that holds the
//! files the build itself generates, mounted at one tree path so that an `embed` of a
//! path under the mount reads the generated directory and never the tree.
//!
//! Why it exists (G0b Task 6, spec §5.4): aeon's sources embed the sound seams' output
//! by tree path (`embed("engine/sound/generated/z80_sound_blob.bin")`). Before the root
//! existed the seams wrote those files INTO the tree on every build, `--check`
//! included, so a build mutated the tree it was building. With the root, the seams
//! write into the build's private directory and an embed under the mount resolves
//! there. A file missing from the directory is a missing file (`[embed.not-found]`
//! naming the generated path), never a fall-back to whatever an earlier process left
//! in the tree.
//!
//! # Scope
//!
//! The root is a property of ONE build, entered with [`enter`] for the duration of
//! that build on the thread running it. It is thread-scoped rather than an option on
//! every lowering entry point because the sandbox is reached through many of them (data
//! items, constant folding, layout registration), and it travels to the comptime
//! evaluation thread through [`crate::eval::run_on_eval_stack`], the one place this
//! crate spawns a thread. Two builds on two threads of one process each see their own
//! root, so concurrent builds of one tree (with different anchor overlays, say) cannot
//! read each other's files.
//!
//! # Who enters it
//!
//! The guarantee holds only while a root is in force: with none, the sandbox resolves a
//! mount path against the tree like any other path (this crate does not know which
//! tree paths a project mounts). So every build entry point enters one:
//! `sigil build` (both drivers, `--check` included), the harness's chained and frozen
//! resolves, `native::build_emp` (always a fresh root, with the sound artifacts emitted
//! into it), and the harness's focused lowerings (`harvest_*`, `name_env_report`), which
//! reuse an enclosing build's root or else get an empty one, so a mount read there
//! fails loudly. What still lowers with no root: single-module port tests that
//! provision the tree's mount themselves first (`native::ensure_generated`, which writes
//! the tree on purpose), and the generic `sigil emp --root` path, which builds no aeon
//! sound. Interim: G0b's two-root resolution (Task 9) names roots in
//! the lowering options, and this scope folds into it there (queue row
//! `G0B-GENERATED-ROOT-SCOPE`).

use std::cell::RefCell;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A generated root: the directory holding the generated files, and the tree path it
/// is mounted at. Both absolute and canonical, so they compare against the sandbox's
/// canonical joins and the read-set recorder's canonical keys.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedRoot {
    /// The tree path the root is mounted at (`<tree>/engine/sound/generated`).
    pub mount: PathBuf,
    /// The directory the files are in, outside the tree.
    pub dir: PathBuf,
}

impl GeneratedRoot {
    /// The generated file a tree path under the mount names, or `None` for a path
    /// outside the mount.
    pub fn redirect(&self, tree_path: &Path) -> Option<PathBuf> {
        let rest = tree_path.strip_prefix(&self.mount).ok()?;
        Some(self.dir.join(rest))
    }

    /// The tree path a generated file is mounted at, or `None` for a path outside the
    /// generated directory: the inverse of [`redirect`](Self::redirect), for writing a
    /// read of the generated directory under the name the source used for it.
    pub fn mounted(&self, physical: &Path) -> Option<PathBuf> {
        let rest = physical.strip_prefix(&self.dir).ok()?;
        Some(self.mount.join(rest))
    }
}

thread_local! {
    static CURRENT: RefCell<Option<Arc<GeneratedRoot>>> = const { RefCell::new(None) };
}

/// The generated root entered on this thread, if any.
pub fn current() -> Option<Arc<GeneratedRoot>> {
    CURRENT.with(|c| c.borrow().clone())
}

/// The scope of an [`enter`]: leaving it restores the root that was in force before.
/// Not `Send`: it restores the thread it was created on.
#[must_use = "the generated root is in force only while this guard lives"]
pub struct Entered {
    prev: Option<Arc<GeneratedRoot>>,
    _thread: PhantomData<*const ()>,
}

impl Drop for Entered {
    fn drop(&mut self) {
        let prev = self.prev.take();
        CURRENT.with(|c| *c.borrow_mut() = prev);
    }
}

fn set(root: Option<Arc<GeneratedRoot>>) -> Entered {
    let prev = CURRENT.with(|c| std::mem::replace(&mut *c.borrow_mut(), root));
    Entered { prev, _thread: PhantomData }
}

/// Put `root` in force on this thread until the returned guard drops.
pub fn enter(root: GeneratedRoot) -> Entered {
    set(Some(Arc::new(root)))
}

/// Carry a root captured on the spawning thread into a spawned one.
pub(crate) fn inherit(root: Option<Arc<GeneratedRoot>>) -> Entered {
    set(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> GeneratedRoot {
        GeneratedRoot { mount: PathBuf::from("/t/engine/sound/generated"), dir: PathBuf::from("/b/generated") }
    }

    #[test]
    fn redirect_and_mounted_are_inverse_and_only_under_their_prefix() {
        let r = root();
        assert_eq!(r.redirect(Path::new("/t/engine/sound/generated/x.bin")), Some(PathBuf::from("/b/generated/x.bin")));
        assert_eq!(r.mounted(Path::new("/b/generated/x.bin")), Some(PathBuf::from("/t/engine/sound/generated/x.bin")));
        assert_eq!(r.redirect(Path::new("/t/engine/sound/x.bin")), None);
        assert_eq!(r.redirect(Path::new("/t/engine/sound/generatedx/x.bin")), None);
        assert_eq!(r.mounted(Path::new("/b/other/x.bin")), None);
    }

    #[test]
    fn a_scope_restores_the_root_it_replaced() {
        assert_eq!(current(), None);
        let outer = enter(root());
        let inner_root = GeneratedRoot { mount: PathBuf::from("/u"), dir: PathBuf::from("/c") };
        {
            let _inner = enter(inner_root.clone());
            assert_eq!(current().as_deref(), Some(&inner_root));
        }
        assert_eq!(current().as_deref(), Some(&root()));
        drop(outer);
        assert_eq!(current(), None);
    }

    #[test]
    fn the_root_reaches_the_comptime_evaluation_thread() {
        let _g = enter(root());
        let seen = crate::eval::run_on_eval_stack(current);
        assert_eq!(seen.as_deref(), Some(&root()));
        let other = std::thread::spawn(current).join().unwrap();
        assert_eq!(other, None, "a thread nobody handed the root to must not see it");
    }
}
