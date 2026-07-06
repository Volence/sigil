//! The capability sandbox (Spec 2, Plan 5 — Task 1): path resolution rooted
//! at a fixed `include_root` directory, and a capture ledger recording every
//! comptime file read. Shared infrastructure for `embed` (Task 1) and `import`
//! (Task 2, this file's other half) — both builtins need the SAME "stay inside
//! the source directory" guard and the SAME provenance record of what was read.
//! A later `zx0` builtin reuses the same infra again.
//!
//! **Task 1 scope:** `resolve_sandbox_path` (join + escape rejection) and
//! `record_capture` (append-only ledger), plus `eval_embed`. **Task 2 scope**
//! (this addition): `eval_import` — a comptime JSON/TOML file read mapped into
//! generic comptime [`Value`]s (D-P5.4), reusing both of Task 1's edges
//! unchanged. The public accessor over [`captures`](Evaluator::captures) and the
//! exhaustive path-escape/determinism tests are a LATER task — this file only
//! records edges.
use super::{Env, Evaluator};
use crate::ast;
use crate::value::{Cell, DataBuf, Value};
use sha2::{Digest, Sha256};
use sigil_span::Span;
use std::path::{Component, Path, PathBuf};

/// One recorded comptime file read (Task 1): the resolved path, its SHA-256
/// digest, and its byte length. A later task exposes this ledger publicly and
/// asserts hermeticity/determinism from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureEdge {
    /// The resolved (sandbox-root-joined, normalized) path that was read.
    pub path: PathBuf,
    /// SHA-256 digest of the file's exact bytes at read time.
    pub hash: [u8; 32],
    /// The file's byte length.
    pub len: u64,
}

impl<'a> Evaluator<'a> {
    /// Set the directory `embed`/`import` paths resolve against (Task 1). The
    /// [`layout::eval_data_with_root`](crate::layout::eval_data_with_root) seam
    /// calls this before resolving a data item, mirroring
    /// [`set_here_base`](Self::set_here_base)'s per-evaluation setter pattern.
    pub(crate) fn set_include_root(&mut self, root: PathBuf) {
        self.include_root = Some(root);
    }

    /// Resolve a comptime file-read `path` (from `embed`/`import`) against the
    /// sandbox root, rejecting anything that would read outside the source
    /// directory.
    ///
    /// - No `include_root` set → `[sandbox.no-root]` (a comptime file read
    ///   needs a root to resolve against).
    /// - An absolute `path` → `[sandbox.path-escape]`.
    /// - A relative `path` whose `..` components walk back past the root, once
    ///   normalized lexically (without touching the filesystem, since the
    ///   target may not exist — e.g. the "missing file" case) →
    ///   `[sandbox.path-escape]`.
    ///
    /// On success, returns the joined + normalized absolute path (the root
    /// itself canonicalized, so a root reached via a symlink still contains
    /// every path resolved against it).
    pub(crate) fn resolve_sandbox_path(&mut self, path: &str, span: Span) -> Option<PathBuf> {
        let Some(root) = self.include_root.clone() else {
            self.error(
                span,
                "[sandbox.no-root] embed/import needs a source directory to resolve paths against",
            );
            return None;
        };
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            self.error(
                span,
                "[sandbox.path-escape] embed/import path must stay within the source directory",
            );
            return None;
        }
        // Canonicalize the root (resolving symlinks) so every "starts_with"
        // check below compares like-for-like; fall back to the given root if it
        // cannot be canonicalized (e.g. does not exist), which just means the
        // subsequent file read will fail with its own diagnostic instead.
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        // Join `candidate` onto `root`, resolving `.`/`..` components LEXICALLY
        // (no filesystem access — `embed`'s "missing file" case must still hit
        // this path cleanly). A `..` that would pop above `root` is rejected as
        // an escape; `Component::Normal` segments are pushed as-is.
        let mut resolved = root.clone();
        for comp in candidate.components() {
            match comp {
                Component::ParentDir => {
                    if !resolved.pop() || !resolved.starts_with(&root) {
                        self.error(
                            span,
                            "[sandbox.path-escape] embed/import path must stay within the source directory",
                        );
                        return None;
                    }
                }
                Component::CurDir => {}
                Component::Normal(seg) => resolved.push(seg),
                // `candidate` was already checked non-absolute, so `RootDir`/
                // `Prefix` components cannot appear.
                Component::RootDir | Component::Prefix(_) => {}
            }
        }
        if !resolved.starts_with(&root) {
            self.error(
                span,
                "[sandbox.path-escape] embed/import path must stay within the source directory",
            );
            return None;
        }
        Some(resolved)
    }

    /// Record a comptime file read in the capture ledger (Task 1): appends a
    /// [`CaptureEdge`] with `path`'s SHA-256 digest and byte length. Called
    /// after a successful read, before the bytes are sliced/consumed, so the
    /// ledger records the file's FULL contents regardless of any `skip`/`len`.
    pub(crate) fn record_capture(&mut self, path: &Path, bytes: &[u8]) {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash: [u8; 32] = hasher.finalize().into();
        self.captures.push(CaptureEdge { path: path.to_path_buf(), hash, len: bytes.len() as u64 });
    }

    /// `embed(path, skip: N, len: M)` (Task 1): reads a file at comptime,
    /// within the capability sandbox, and yields its bytes as a
    /// [`Value::Data`] — `BINCLUDE` parity with slicing.
    ///
    /// `path` is the first positional argument (a string); `skip` (default 0)
    /// and `len` (default: the rest of the file past `skip`) are optional named
    /// non-negative integer arguments. Any other named argument, a missing/
    /// non-string path, a sandbox-escaping path, an unreadable file, or a
    /// `skip`/`len` past the end of the file are all diagnostics that poison
    /// the result — never a panic.
    pub(super) fn eval_embed(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let mut path_arg: Option<&ast::Arg> = None;
        let mut skip_arg: Option<&ast::Arg> = None;
        let mut len_arg: Option<&ast::Arg> = None;
        for arg in args {
            match arg.name.as_deref() {
                None => {
                    if path_arg.is_some() {
                        self.error(arg.span, "`embed` takes exactly one positional path argument");
                    } else {
                        path_arg = Some(arg);
                    }
                }
                Some("skip") => {
                    if skip_arg.is_some() {
                        self.error(arg.span, "`skip` given more than once");
                    }
                    skip_arg = Some(arg);
                }
                Some("len") => {
                    if len_arg.is_some() {
                        self.error(arg.span, "`len` given more than once");
                    }
                    len_arg = Some(arg);
                }
                Some(other) => {
                    self.error(arg.span, format!("unknown named argument `{other}` to `embed`"));
                }
            }
        }
        let Some(path_arg) = path_arg else {
            self.error(span, "`embed` requires a path argument");
            return Value::Poison;
        };
        let path_val = self.eval_expr(&path_arg.value, env);
        // A leaked return / abort from the path argument belongs to the caller.
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        let path = match path_val {
            Value::Str(s) => s,
            Value::Poison => return Value::Poison,
            other => {
                self.error(
                    path_arg.span,
                    format!("`embed` path must be a string, got {}", other.type_name()),
                );
                return Value::Poison;
            }
        };
        let skip = match self.eval_embed_uint_arg(skip_arg, "skip", env) {
            Ok(n) => n,
            Err(()) => return Value::Poison,
        };
        let len_opt = match len_arg {
            None => None,
            Some(_) => match self.eval_embed_uint_arg(len_arg, "len", env) {
                Ok(n) => Some(n),
                Err(()) => return Value::Poison,
            },
        };
        let Some(resolved) = self.resolve_sandbox_path(&path, path_arg.span) else {
            return Value::Poison;
        };
        let bytes = match std::fs::read(&resolved) {
            Ok(b) => b,
            Err(_) => {
                self.error(span, format!("[embed.read] cannot read {path}"));
                return Value::Poison;
            }
        };
        self.record_capture(&resolved, &bytes);
        let file_len = bytes.len() as u64;
        if skip > file_len {
            self.error(
                span,
                format!("[embed.range] embed skip={skip} exceeds file length {file_len}"),
            );
            return Value::Poison;
        }
        let len = len_opt.unwrap_or(file_len - skip);
        if len > file_len - skip {
            self.error(
                span,
                format!("[embed.range] embed slice skip={skip} len={len} exceeds file length {file_len}"),
            );
            return Value::Poison;
        }
        let start = skip as usize;
        let end = start + len as usize;
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(bytes[start..end].to_vec()));
        Value::Data(buf)
    }

    /// Evaluate an optional named non-negative-integer argument to `embed`
    /// (`skip`/`len`). Returns `Ok(0)` when `arg` is `None` (the caller supplies
    /// the right default for `skip` vs `len` — this helper just does the shared
    /// eval-and-check). `Err(())` signals an already-diagnosed failure
    /// (non-integer, negative, or a leaked return/abort) the caller should
    /// propagate as `Poison`.
    fn eval_embed_uint_arg(
        &mut self,
        arg: Option<&ast::Arg>,
        label: &str,
        env: &mut Env,
    ) -> Result<u64, ()> {
        let Some(arg) = arg else {
            return Ok(0);
        };
        let v = self.eval_expr(&arg.value, env);
        if self.aborted || self.pending_return.is_some() {
            return Err(());
        }
        match v.as_stored_int() {
            Some(n) => match u64::try_from(n) {
                Ok(u) => Ok(u),
                Err(_) => {
                    self.error(
                        arg.span,
                        format!("`{label}` must be a non-negative value that fits u64, got {n}"),
                    );
                    Err(())
                }
            },
            None => match v {
                Value::Poison => Err(()),
                other => {
                    self.error(arg.span, format!("`{label}` must be an integer, got {}", other.type_name()));
                    Err(())
                }
            },
        }
    }

    /// `import(path)` (Task 2, D-P5.4): reads a JSON or TOML file at comptime,
    /// within the SAME capability sandbox as `embed`, and maps it into generic
    /// comptime `Value`s rather than raw `Data` bytes:
    ///
    /// | source shape          | `Value`                                          |
    /// |------------------------|--------------------------------------------------|
    /// | object / table          | `Struct { ty_name: "<import>", fields }` (key order preserved) |
    /// | array                    | `Array`                                          |
    /// | integral number          | `Int`                                            |
    /// | fractional number        | `Float`                                          |
    /// | string                   | `Str`                                            |
    /// | bool                     | `Bool`                                           |
    /// | JSON `null`              | `Unit`                                           |
    /// | TOML `Datetime`          | `[import.unsupported]` + `Poison` (no comptime equivalent) |
    ///
    /// A returned `Value::Struct` deliberately carries NO real type identity —
    /// `ty_name` is always the placeholder `"<import>"`. `lower_struct`
    /// (`emit.rs`) already matches struct fields BY NAME against the layout of
    /// whatever `Ty` the surrounding `data` item declares, ignoring `ty_name`
    /// entirely, so a typed `data P: Point = import("p.json")` lowers correctly
    /// against `Point`'s layout with no additional wiring here — and the SAME
    /// `lower_struct` now shape-checks (D-P5.4) that the value's keys exactly
    /// match the declared fields, so a mismatched import is a diagnostic rather
    /// than a silent mis-size.
    ///
    /// The file format is dispatched on the path's EXTENSION (case-insensitive):
    /// `.json` → `serde_json`, `.toml` → `toml`; anything else is
    /// `[import.format]`. `path` is the only argument (any named argument is
    /// unknown-argument diagnostic, matching `embed`'s pattern for `skip`/`len`
    /// but with nothing to name here). A sandbox-escaping path, an unreadable
    /// file, or a parse error are all diagnostics that poison the result — never
    /// a panic.
    pub(super) fn eval_import(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        let mut path_arg: Option<&ast::Arg> = None;
        for arg in args {
            match arg.name.as_deref() {
                None => {
                    if path_arg.is_some() {
                        self.error(arg.span, "`import` takes exactly one positional path argument");
                    } else {
                        path_arg = Some(arg);
                    }
                }
                Some(other) => {
                    self.error(arg.span, format!("unknown named argument `{other}` to `import`"));
                }
            }
        }
        let Some(path_arg) = path_arg else {
            self.error(span, "`import` requires a path argument");
            return Value::Poison;
        };
        let path_val = self.eval_expr(&path_arg.value, env);
        // A leaked return / abort from the path argument belongs to the caller.
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        let path = match path_val {
            Value::Str(s) => s,
            Value::Poison => return Value::Poison,
            other => {
                self.error(
                    path_arg.span,
                    format!("`import` path must be a string, got {}", other.type_name()),
                );
                return Value::Poison;
            }
        };
        let Some(resolved) = self.resolve_sandbox_path(&path, path_arg.span) else {
            return Value::Poison;
        };
        // Dispatch on the EXTENSION before touching the file: an unsupported
        // extension is diagnosed without needing a successful read.
        let ext = resolved
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        if !matches!(ext.as_deref(), Some("json") | Some("toml")) {
            self.error(span, "[import.format] import needs a .json or .toml file");
            return Value::Poison;
        }
        let bytes = match std::fs::read(&resolved) {
            Ok(b) => b,
            Err(_) => {
                self.error(span, format!("[import.read] cannot read {path}"));
                return Value::Poison;
            }
        };
        self.record_capture(&resolved, &bytes);
        match ext.as_deref() {
            Some("json") => match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(v) => Self::json_to_value(&v),
                Err(e) => {
                    self.error(span, format!("[import.parse] {path}: {e}"));
                    Value::Poison
                }
            },
            Some("toml") => {
                let text = match std::str::from_utf8(&bytes) {
                    Ok(t) => t,
                    Err(e) => {
                        self.error(span, format!("[import.parse] {path}: {e}"));
                        return Value::Poison;
                    }
                };
                match text.parse::<toml::Value>() {
                    Ok(v) => self.toml_to_value(&v, span),
                    Err(e) => {
                        self.error(span, format!("[import.parse] {path}: {e}"));
                        Value::Poison
                    }
                }
            }
            // The extension check above already restricted `ext` to these two.
            _ => unreachable!("extension already checked to be json or toml"),
        }
    }

    /// Map a `serde_json::Value` into a comptime [`Value`] (Task 2, D-P5.4).
    /// JSON key order is preserved (the crate's `preserve_order` feature backs
    /// `serde_json::Map` with an `IndexMap`) — chosen over sorting so a struct's
    /// declared field order isn't required to match a lexical key order, and so
    /// re-running `import` on an unchanged file is visibly stable/deterministic
    /// against the SOURCE file's own key order, not an incidental sort.
    fn json_to_value(v: &serde_json::Value) -> Value {
        match v {
            serde_json::Value::Null => Value::Unit,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i as i128)
                } else if let Some(u) = n.as_u64() {
                    Value::Int(u as i128)
                } else {
                    // Neither an i64 nor a u64 representation: a fractional
                    // number, or an integer wider than 64 bits (serde_json
                    // itself only models integers up to u64/i64) — either way
                    // there is no exact `Int` to give, so fall back to `f64`.
                    Value::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            serde_json::Value::String(s) => Value::Str(s.clone()),
            serde_json::Value::Array(arr) => Value::Array(arr.iter().map(Self::json_to_value).collect()),
            serde_json::Value::Object(map) => Value::Struct {
                ty_name: "<import>".to_string(),
                fields: map.iter().map(|(k, v)| (k.clone(), Self::json_to_value(v))).collect(),
            },
        }
    }

    /// Map a `toml::Value` into a comptime [`Value`] (Task 2, D-P5.4). Same
    /// key-order-preserving choice as [`json_to_value`](Self::json_to_value)
    /// (backed by the crate's `preserve_order` feature on the `toml` dep). A
    /// `Datetime` has no comptime equivalent — `[import.unsupported]` +
    /// `Poison` for that value (the surrounding object/array still builds
    /// around it; only that one field/element is poisoned).
    fn toml_to_value(&mut self, v: &toml::Value, span: Span) -> Value {
        match v {
            toml::Value::String(s) => Value::Str(s.clone()),
            toml::Value::Integer(i) => Value::Int(*i as i128),
            toml::Value::Float(f) => Value::Float(*f),
            toml::Value::Boolean(b) => Value::Bool(*b),
            toml::Value::Datetime(_) => {
                self.error(span, "[import.unsupported] TOML datetime not supported");
                Value::Poison
            }
            toml::Value::Array(arr) => {
                Value::Array(arr.iter().map(|e| self.toml_to_value(e, span)).collect())
            }
            toml::Value::Table(map) => Value::Struct {
                ty_name: "<import>".to_string(),
                fields: map.iter().map(|(k, v)| (k.clone(), self.toml_to_value(v, span))).collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Evaluator;

    fn span() -> Span {
        Span { source: sigil_span::SourceId(0), start: 0, end: 0 }
    }

    #[test]
    fn record_capture_appends_one_edge() {
        let mut ev = Evaluator::new();
        assert!(ev.captures.is_empty());
        let path = PathBuf::from("some/file.bin");
        ev.record_capture(&path, b"hello");
        assert_eq!(ev.captures.len(), 1);
        let edge = &ev.captures[0];
        assert_eq!(edge.path, path);
        assert_eq!(edge.len, 5);
        // SHA-256("hello"), a fixed known digest — pins the hash function/input.
        assert_eq!(
            edge.hash,
            [
                0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e, 0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9,
                0xe2, 0x9e, 0x1b, 0x16, 0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e, 0x73, 0x04, 0x33, 0x62,
                0x93, 0x8b, 0x98, 0x24,
            ]
        );
    }

    #[test]
    fn resolve_sandbox_path_without_root_errors() {
        let mut ev = Evaluator::new();
        let got = ev.resolve_sandbox_path("x.bin", span());
        assert!(got.is_none());
        assert!(ev.diags.iter().any(|d| d.message.contains("[sandbox.no-root]")));
    }

    #[test]
    fn resolve_sandbox_path_rejects_absolute() {
        let mut ev = Evaluator::new();
        ev.set_include_root(PathBuf::from("/tmp"));
        let got = ev.resolve_sandbox_path("/etc/passwd", span());
        assert!(got.is_none());
        assert!(ev.diags.iter().any(|d| d.message.contains("[sandbox.path-escape]")));
    }

    #[test]
    fn resolve_sandbox_path_rejects_dotdot_escape() {
        let mut ev = Evaluator::new();
        ev.set_include_root(PathBuf::from("/tmp"));
        let got = ev.resolve_sandbox_path("../etc/passwd", span());
        assert!(got.is_none());
        assert!(ev.diags.iter().any(|d| d.message.contains("[sandbox.path-escape]")));
    }
}
