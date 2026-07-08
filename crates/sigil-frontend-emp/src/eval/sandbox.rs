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
        // Symlink containment (T5, closing T1-review finding #1): the LEXICAL
        // check above only catches a `..`-escape written directly in `path` —
        // it says nothing about a symlink INSIDE the root whose target points
        // OUTSIDE it (e.g. `root/link -> /etc/passwd`), since the lexical join
        // never touches the filesystem. If `resolved` exists on disk, resolve
        // it FOR REAL (`fs::canonicalize` follows every symlink component) and
        // re-check containment against the (already-canonical) `root`; a
        // resolved-but-escaping target is still `[sandbox.path-escape]`. If it
        // does NOT exist, this is the legitimate "missing file" case (e.g.
        // `embed` of a not-yet-created file) — canonicalize would itself fail
        // on a nonexistent path, so skip straight to returning the lexical
        // result and let the builtin's own read fail with its own diagnostic.
        //
        // TOCTOU: the builtin later reads the LEXICAL `resolved` path, so a
        // symlink swapped between this check and that read could bypass
        // containment. This is acceptable for a single-threaded, build-time
        // comptime evaluator with no concurrent adversary — it is not a
        // live-attacker sandbox; it stops an honest source tree from
        // accidentally (or a `..`/symlink-in-tree from casually) escaping root.
        if resolved.exists() {
            if let Ok(canon) = std::fs::canonicalize(&resolved) {
                if !canon.starts_with(&root) {
                    self.error(
                        span,
                        "[sandbox.path-escape] embed/import path must stay within the source directory",
                    );
                    return None;
                }
            }
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
                // A provisional here() gets the SPECIFIC D-H.2 steering message.
                v @ Value::LinkExpr(_) => {
                    self.reject_if_provisional(&v, arg.span);
                    Err(())
                }
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

    /// `zx0(data)` (Spec 2, Plan 5 — Task 3): ZX0-compresses a [`Value::Data`]
    /// at comptime and wraps it in the exact 4-byte header `aeon/build.sh`
    /// hand-emits (its ZX0-wrapping loop, around line 118): `[u16 BE
    /// uncompressed-size][0x00][0x02]` ++ the raw salvador stream —
    /// byte-identical to the ROM's compressed art blobs.
    ///
    /// Unlike `embed`/`import`, `zx0` reads no file itself — its input `Data`
    /// already carries its own capture edge from whatever `embed` produced
    /// it — so it needs no sandbox root and never calls `record_capture`.
    ///
    /// `data` is the sole positional argument; it must already be a
    /// [`Value::Data`] (typically `embed(...)`/`bytes([...])`, possibly built
    /// up via `++`). Split into two halves for testability: this fn only
    /// does argument arity/type checking, delegating the compress-and-wrap
    /// work to [`zx0_from_data`](Self::zx0_from_data), which a unit test can
    /// call directly with a hand-built `DataBuf` (e.g. an oversized input)
    /// without constructing AST argument nodes.
    pub(super) fn eval_zx0(&mut self, args: &[ast::Arg], span: Span, env: &mut Env) -> Value {
        if args.len() != 1 {
            self.error(span, format!("`zx0` expects exactly 1 argument, got {}", args.len()));
            return Value::Poison;
        }
        if args[0].name.is_some() {
            self.error(args[0].span, "`zx0` takes a positional argument");
        }
        let arg = self.eval_expr(&args[0].value, env);
        // A leaked return / abort from the argument belongs to the caller.
        if self.aborted || self.pending_return.is_some() {
            return Value::Poison;
        }
        let buf = match arg {
            Value::Data(buf) => buf,
            Value::Poison => return Value::Poison,
            other => {
                self.error(span, format!("[zx0.arg] zx0 expects a Data value, got {}", other.type_name()));
                return Value::Poison;
            }
        };
        self.zx0_from_data(buf, span)
    }

    /// The compress-and-wrap core of `zx0` (Task 3). See
    /// [`eval_zx0`](Self::eval_zx0)'s doc comment for why this is split out.
    ///
    /// Flattens `buf`'s cells to raw input bytes: `Cell::Bytes` extends
    /// directly; a width-1 `Cell::Scalar` contributes its one (range-checked)
    /// byte; a WIDER `Cell::Scalar` has no committed byte order yet (that's
    /// Plan 4's 68k-BE-vs-Z80-LE lowering decision) and a `Cell::SymRef`
    /// names an address not yet resolved (also Plan 4) — both are diagnostics
    /// here, never a panic, since `zx0` can only compress concrete bytes.
    /// Asserts the flattened length fits the wrapper's `u16` size field,
    /// compresses via [`sigil_salvador_sys::compress`], and prepends the
    /// 4-byte header ahead of the compressed stream.
    pub(crate) fn zx0_from_data(&mut self, buf: DataBuf, span: Span) -> Value {
        let mut input = Vec::with_capacity(buf.size);
        for cell in &buf.cells {
            match cell {
                Cell::Bytes(b) => input.extend_from_slice(b),
                Cell::Scalar { value, width: 1, .. } => {
                    // Mirrors `byte`/`bytes`'s accepted range: a signed or
                    // unsigned reading of one byte. Reuse the SAME constants so
                    // the two byte-domain sites cannot silently drift apart.
                    if !(super::builtins::BYTE_LO..=super::builtins::BYTE_HI).contains(value) {
                        self.error(
                            span,
                            format!("[zx0.byte-range] zx0 input byte {value} does not fit 8 bits"),
                        );
                        return Value::Poison;
                    }
                    input.push((*value & 0xFF) as u8);
                }
                Cell::Scalar { .. } => {
                    self.error(
                        span,
                        "[zx0.byte-order] zx0 input has a multi-byte scalar with no committed byte order — build it from raw bytes (embed/bytes)",
                    );
                    return Value::Poison;
                }
                Cell::SymRef { .. } => {
                    self.error(span, "[zx0.symbolic] zx0 input has an unresolved symbol reference");
                    return Value::Poison;
                }
                Cell::RelOffset { .. } => {
                    self.error(span, "[zx0.symbolic] zx0 input has an unresolved offset-table entry");
                    return Value::Poison;
                }
                Cell::Expr { .. } => {
                    self.error(span, "[zx0.symbolic] zx0 input has an unresolved link-expr value");
                    return Value::Poison;
                }
            }
        }
        if input.len() > 0xFFFF {
            self.error(
                span,
                format!(
                    "[zx0.too-large] zx0 input is {} bytes, exceeds the 65535-byte u16 size field",
                    input.len()
                ),
            );
            return Value::Poison;
        }
        let n = input.len();
        let compressed = sigil_salvador_sys::compress(&input);
        let mut out = vec![(n >> 8) as u8, (n & 0xFF) as u8, 0x00, 0x02];
        out.extend_from_slice(&compressed);
        let mut result = DataBuf::empty();
        result.push(Cell::Bytes(out));
        Value::Data(result)
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

    /// A scratch directory (named by `tag`, so distinct tests never collide)
    /// under this crate's own `target/` build directory, cleaned up by each
    /// test that creates one.
    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("sandbox-test-scratch")
            .join(tag);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    #[test]
    fn resolve_sandbox_path_normal_in_root_file_resolves() {
        let root = scratch_dir("normal");
        std::fs::write(root.join("real.bin"), b"hello").expect("write fixture");
        let mut ev = Evaluator::new();
        ev.set_include_root(root.clone());
        let got = ev.resolve_sandbox_path("real.bin", span());
        let canonical_root = std::fs::canonicalize(&root).unwrap();
        assert_eq!(got, Some(canonical_root.join("real.bin")));
        assert!(ev.diags.is_empty(), "unexpected diagnostics: {:?}", ev.diags);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[cfg(unix)]
    fn resolve_sandbox_path_rejects_symlink_escaping_root() {
        use std::os::unix::fs::symlink;

        // `outside/secret.bin` lives OUTSIDE the sandbox root; `root/link_name`
        // is a symlink pointing at it. The LEXICAL join of `link_name` onto
        // `root` never leaves `root` — only following the symlink for real
        // reveals the escape.
        let base = scratch_dir("symlink-escape");
        let root = base.join("root");
        let outside = base.join("outside");
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::create_dir_all(&outside).expect("create outside");
        let secret = outside.join("secret.bin");
        std::fs::write(&secret, b"outside bytes").expect("write secret");
        let link = root.join("link_name");
        symlink(&secret, &link).expect("create symlink");

        let mut ev = Evaluator::new();
        ev.set_include_root(root.clone());
        let got = ev.resolve_sandbox_path("link_name", span());
        assert!(got.is_none(), "expected the symlink escape to be rejected");
        assert!(
            ev.diags.iter().any(|d| d.message.contains("[sandbox.path-escape]")),
            "expected [sandbox.path-escape], got {:?}",
            ev.diags
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[cfg(unix)]
    fn resolve_sandbox_path_allows_symlink_staying_in_root() {
        use std::os::unix::fs::symlink;

        // A symlink INSIDE the root pointing at another file also INSIDE the
        // root must still resolve — the containment check must not reject
        // every symlink, only ones that escape.
        let root = scratch_dir("symlink-contained");
        let real = root.join("real.bin");
        std::fs::write(&real, b"contained bytes").expect("write real");
        let link = root.join("link_name");
        symlink(&real, &link).expect("create symlink");

        let mut ev = Evaluator::new();
        ev.set_include_root(root.clone());
        let got = ev.resolve_sandbox_path("link_name", span());
        assert!(got.is_some(), "expected the in-root symlink to resolve: {:?}", ev.diags);
        assert!(ev.diags.is_empty(), "unexpected diagnostics: {:?}", ev.diags);
        let _ = std::fs::remove_dir_all(&root);
    }

    // `zx0` unit tests: exercised via `zx0_from_data` directly (bypassing AST
    // argument construction) for the shapes that are impractical to author as
    // `.emp` source — an oversized input in particular. The realistic
    // wrapper/end-to-end paths are covered by
    // `tests/sandbox_zx0.rs`'s integration tests.

    #[test]
    fn zx0_from_data_too_large_input_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(vec![0u8; 0x10000])); // 65536 bytes — one past the u16 max.
        let result = ev.zx0_from_data(buf, span());
        assert_eq!(result, Value::Poison);
        assert!(ev.diags.iter().any(|d| d.message.contains("[zx0.too-large]")));
    }

    #[test]
    fn zx0_from_data_multibyte_scalar_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: 0x1234, width: 2, signed: false, le: false });
        let result = ev.zx0_from_data(buf, span());
        assert_eq!(result, Value::Poison);
        assert!(ev.diags.iter().any(|d| d.message.contains("[zx0.byte-order]")));
    }

    #[test]
    fn zx0_from_data_byte_range_errors() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: 999, width: 1, signed: false, le: false });
        let result = ev.zx0_from_data(buf, span());
        assert_eq!(result, Value::Poison);
        assert!(ev.diags.iter().any(|d| d.message.contains("[zx0.byte-range]")));
    }

    #[test]
    fn zx0_from_data_empty_input() {
        // n == 0: header is `[0,0,0,2]`, and salvador on empty input yields an
        // empty stream — matching what build.sh would emit for a 0-byte file.
        let mut ev = Evaluator::new();
        let result = ev.zx0_from_data(DataBuf::empty(), span());
        let expected_compressed = sigil_salvador_sys::compress(&[]);
        match result {
            Value::Data(out) => match &out.cells[0] {
                Cell::Bytes(b) => {
                    let mut expected = vec![0x00, 0x00, 0x00, 0x02];
                    expected.extend_from_slice(&expected_compressed);
                    assert_eq!(b, &expected);
                }
                other => panic!("expected Cell::Bytes, got {other:?}"),
            },
            other => panic!("expected Value::Data, got {other:?}"),
        }
        assert!(ev.diags.is_empty(), "empty input should not diagnose: {:?}", ev.diags);
    }

    #[test]
    fn zx0_from_data_max_u16_input() {
        // n == 65535: the ALLOWED boundary (65536 is the rejected one). The
        // size field is `[0xFF, 0xFF]` and no `[zx0.too-large]` fires.
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Bytes(vec![7u8; 0xFFFF]));
        let result = ev.zx0_from_data(buf, span());
        let expected_compressed = sigil_salvador_sys::compress(&[7u8; 0xFFFF]);
        match result {
            Value::Data(out) => match &out.cells[0] {
                Cell::Bytes(b) => {
                    assert_eq!(&b[..4], &[0xFF, 0xFF, 0x00, 0x02]);
                    assert_eq!(&b[4..], &expected_compressed[..]);
                }
                other => panic!("expected Cell::Bytes, got {other:?}"),
            },
            other => panic!("expected Value::Data, got {other:?}"),
        }
        assert!(ev.diags.iter().all(|d| !d.message.contains("[zx0.too-large]")));
    }

    #[test]
    fn zx0_from_data_single_byte_scalar_and_bytes_mix() {
        let mut ev = Evaluator::new();
        let mut buf = DataBuf::empty();
        buf.push(Cell::Scalar { value: 5, width: 1, signed: false, le: false });
        buf.push(Cell::Bytes(vec![5, 5, 5, 5]));
        let result = ev.zx0_from_data(buf, span());
        // 5 bytes of the same repeated value: header [0,5,0,2] then the raw
        // salvador stream for `[5,5,5,5,5]`.
        let expected_compressed = sigil_salvador_sys::compress(&[5, 5, 5, 5, 5]);
        match result {
            Value::Data(out) => {
                assert_eq!(out.cells.len(), 1);
                match &out.cells[0] {
                    Cell::Bytes(b) => {
                        let mut expected = vec![0x00, 0x05, 0x00, 0x02];
                        expected.extend_from_slice(&expected_compressed);
                        assert_eq!(b, &expected);
                    }
                    other => panic!("expected Cell::Bytes, got {other:?}"),
                }
            }
            other => panic!("expected Value::Data, got {other:?}"),
        }
    }
}
