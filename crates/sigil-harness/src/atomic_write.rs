//! Installing a file by rename rather than by truncation.
//!
//! Every artifact sigil hands to a consumer goes through here. The consumer set is
//! no longer just the person who typed the command: other lanes poll sigil's outputs
//! while a build is running, and the two failure modes are not equally loud. A
//! MISSING ROM is loud, and a reader handles it. A TRUNCATED ROM is silent, and a
//! reader that loads one produces findings from half a build that are shaped exactly
//! like findings from a whole one.
//!
//! # What the guarantee is
//!
//! A reader that opens the destination path observes either the complete previous
//! contents or the complete new contents, never a prefix of either and never an
//! empty file. The bytes are written to a temporary beside the destination, flushed
//! with `sync_all`, and then moved onto the destination name with `rename`, which
//! within one filesystem is a single indivisible directory operation.
//!
//! The temporary is a SIBLING of the destination and not a file in the system
//! temporary directory. A rename is atomic only within one filesystem, and the one
//! directory guaranteed to share the destination's filesystem is the destination's
//! own.
//!
//! # What the guarantee is not
//!
//! * **It is not a validity claim.** A rename installs a complete file; whether
//!   that file's CONTENTS are correct is the caller's business. Installing a
//!   complete-but-wrong artifact atomically is still installing a wrong artifact.
//!
//! * **It is not crash consistency.** `sync_all` orders the data before the rename
//!   within this process, which a kill respects. A power loss may expose the
//!   directory entry against writes the kernel has not flushed.
//!
//! * **It is not multi-file atomicity.** Installing several artifacts is several
//!   renames, so a reader can observe a mixed set. A caller needing a set to land
//!   together needs a staging protocol, not this function.
//!
//! # The file-mode contract, which changed
//!
//! A rename replaces a NAME. Permission to do it comes from the WRITE BIT ON THE
//! CONTAINING DIRECTORY, and the destination file's own mode does not enter into it.
//! Truncation is the opposite: it opens the destination for writing, so the
//! destination's own mode governs.
//!
//! The consequence, stated plainly because someone is relying on the other answer:
//!
//! > **`chmod a-w <artifact>` does not prevent sigil from replacing that artifact.**
//! > Freezing an output by file mode alone has no effect on this path. To make an
//! > artifact unreplaceable, the CONTAINING DIRECTORY must also be write-denied.
//!
//! Measured on this machine (coreutils 9.11): renaming over a 0444 file in a
//! writable directory succeeds and changes the file's contents; with the directory
//! at 0555 as well it is refused with `EACCES`; and truncating that same 0444 file
//! in a writable directory is refused. So a file-mode freeze does stop a
//! truncating writer and does not stop this one.
//!
//! The destination's existing mode IS carried across the rename (see
//! [`write_atomic`]), so a file left at 0444 still reads 0444 afterwards. That
//! preservation is cosmetic with respect to blocking: it keeps a deliberately-set
//! mode from being silently reset to the process umask default, and it does not
//! restore the enforcement described above.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Distinguishes concurrent installs of the same destination from within one
/// process, which the pid alone cannot.
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Install `bytes` at `path` by rename, never by truncation.
///
/// On success the destination holds exactly `bytes` and no temporary remains. On
/// failure the destination is left exactly as it was, including the case where it
/// did not exist, and the temporary is removed; a caller that aborts on a write
/// error therefore leaves the previous artifact readable rather than a stub.
///
/// When the destination already exists its permission bits are applied to the
/// installed file, so a mode set deliberately on an artifact survives being
/// rewritten. A destination that does not yet exist gets the mode `File::create`
/// gives it, which is the process umask default.
///
/// Errors are rendered with the destination path, not the temporary, because the
/// temporary is an implementation detail no caller named. A caller that renders its
/// own message wants [`write_atomic_io`], which returns the cause alone.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_atomic_io(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
}

/// [`write_atomic`] returning the underlying failure rather than a rendered
/// sentence, for a caller that names the destination itself and would otherwise
/// print the path twice.
pub fn write_atomic_io(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} names no file", path.display()),
            )
        })?
        .to_string_lossy()
        .into_owned();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".{stem}.{}.{seq}.tmp", std::process::id()));

    let install = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        // Carry the destination's mode BEFORE the rename: afterwards the name refers
        // to the new inode and the old mode is unreachable. A destination that does
        // not exist, or whose mode cannot be read, simply leaves the temporary's own.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(path) {
                let mode = meta.permissions().mode();
                let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode));
            }
        }
        std::fs::rename(&tmp, path)
    };
    match install() {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// [`write_atomic`] for text, which is the shape every ledger and listing writer
/// holds its content in.
pub fn write_atomic_str(path: &Path, contents: &str) -> Result<(), String> {
    write_atomic(path, contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ordinary install: right bytes, right path, no temporary left beside it.
    #[test]
    fn installs_the_bytes_and_leaves_no_temporary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("rom.bin");
        write_atomic(&p, b"\x01\x02\x03").expect("install");
        assert_eq!(std::fs::read(&p).expect("read"), b"\x01\x02\x03");
        let leftovers: Vec<String> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .filter(|n| n != "rom.bin")
            .collect();
        assert!(leftovers.is_empty(), "temporary left behind: {leftovers:?}");
    }

    /// A second install replaces the contents wholesale rather than overlaying them,
    /// so a shorter artifact does not leave a tail of the longer one it replaced.
    #[test]
    fn replacing_a_longer_file_leaves_no_tail() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("rom.bin");
        write_atomic(&p, b"AAAAAAAAAA").expect("first");
        write_atomic(&p, b"BB").expect("second");
        assert_eq!(std::fs::read(&p).expect("read"), b"BB");
    }

    /// The improvement over truncation: a failed write leaves the PREVIOUS artifact
    /// complete and readable. The failure is forced by denying the directory, which
    /// is what `File::create` on the temporary needs, so no timing is involved.
    #[cfg(unix)]
    #[test]
    fn a_failed_write_leaves_the_previous_file_intact() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("rom.bin");
        write_atomic(&p, b"COMPLETE-OLD").expect("seed");

        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555))
            .expect("deny directory");
        let err = write_atomic(&p, b"NEW").expect_err("write must fail with the directory denied");
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755))
            .expect("restore directory");

        assert!(err.contains("rom.bin"), "error names the destination: {err}");
        assert_eq!(std::fs::read(&p).expect("read"), b"COMPLETE-OLD");
    }

    /// The destination's mode survives the rename, which `rename` alone does not do
    /// for you: the installed file is a different inode carrying the temporary's mode
    /// until this is applied.
    #[cfg(unix)]
    #[test]
    fn the_destinations_mode_survives_the_rename() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("rom.bin");
        write_atomic(&p, b"old").expect("seed");
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o444)).expect("chmod");

        write_atomic(&p, b"new").expect("install over a read-only destination");

        let mode = std::fs::metadata(&p).expect("stat").permissions().mode() & 0o777;
        assert_eq!(mode, 0o444, "destination mode was reset to the umask default");
        assert_eq!(std::fs::read(&p).expect("read"), b"new");
    }
}
