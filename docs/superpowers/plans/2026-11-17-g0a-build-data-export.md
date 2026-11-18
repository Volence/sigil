---
kind: plan
status: awaiting-owner
description: "Empyrean v2 step 0, G0a: Sigil writes one schema-checked <stem>.build.json per ROM (symbols for both CPUs with an address space, layouts, integer constants, a line table, bound by the ROM's SHA-256); the contract, a Python reader, the harness move to romSha256, and Oracle's binding and decoder gate."
---
# G0a Build Data Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Every dispatch carries the `dispatching-empyrean-agents` invariant block.

**Goal:** For a fresh s4 and demo build, Oracle loads `<stem>.build.json`, binds it by SHA-256, resolves 68000 and Z80 symbols by address space, checks its `Sst` decoder against the exported layout, and answers "which file and line is this PC?", with every gate below seen red first and the 7 shapes byte-identical.

**Architecture:** A pure exporter in `sigil-link::export` builds a `BuildExport` model from the placed sections, a `SpanLocator` trait, and side tables the two front ends hand it. `sigil-cli`'s `run_build_native` (the one place that holds the ROM as written and the `ChainedResolve`) calls it after the ROM is installed and writes the sidecar atomically. The JSON schema lives in `empyrean/contract/schema/build/`, vendored into sigil with a drift check. Consumers (Python reader, aeon harness, Oracle) read only that file.

**Tech Stack:** Rust (`sigil-*` crates, `serde_json` with `preserve_order`, `sha2 0.10`, both already in the workspace lock), JSON Schema 2020-12, Python 3 (`jsonschema` 4.26, `hashlib`), Oracle (`oracle-core`, `oracle-aether`).

**Spec:** `empyrean/docs/v2/specs/2026-11-17-G0a-build-data-export-design.md` (revision 2, owner answers 2026-11-18: Q1 yes, Q2 yes, Q3 keep Oracle's curated table and gate it, Q4 confirmed). Review: `empyrean/docs/v2/specs/reviews/2026-11-17-G0a-review.md`. Companion: `2026-11-17-G0b-game-as-data-driver-design.md`.

## Global Constraints

- **Not byte-changing.** No ROM byte moves in any shape. Every task ends with `scripts/step0_shapes_cmp.sh` printing `SHAPES 7/7 IDENTICAL` at aeon HEAD (s4, s4 debug, demo, demo debug, config A, config B, lean). A task that moves a byte STOPS: BLOCKED, owner call.
- **The `.lst` is not touched.** `sigil_link::emit_listing` and `lst_source_digest` are not edited; convsym builds ROM bytes from the listing (`native.rs` deb2 path).
- Format envelope verbatim: `"format": "empyrean.build"`, `"version": {"major": 1, "minor": 0}`. Consumers accept any equal or higher minor, ignore unknown keys, refuse an unknown major **by name before reading anything else**.
- Binding is the full SHA-256 of the ROM file as written (`rom.sha256`, lowercase hex, 64 chars). In results the field is named `romSha256`. A 16-digit prefix may be displayed, never compared.
- Write order: ROM installed → hash the bytes held in memory (`full`) → write the sidecar atomically (`sigil_harness::atomic_write::write_atomic`). Any failure after the ROM write and before the sidecar leaves no sidecar. A stale sidecar from an earlier build is deleted before the ROM is installed.
- Numbers are JSON integers, never hex strings. No absolute paths, no timestamps, no hostnames: file paths are relative to a named root, one of exactly `engine`, `game`, `generated`, `sigil`.
- Constants in 1.0 are integer, boolean and enum values only. No `regions`, no per-row columns, no expansion chain (all 1.1).
- The anti-goal binds: no new declaration form, attribute or type in the `.emp` language. `Frame::Template` and span rewriting are evaluator plumbing.
- Branches: sigil `parcel/g0a-build-export` (cut from sigil master at execution time, today `c08bf4ce`); empyrean `feat/g0a-build-contract` (from `main`); aeon `feat/g0a-romsha256` (from master, today `81efe8f4`); oracle `feat/g0a-build-export` (from `main`, today `dd09acb9`). Parallel work uses worktrees on `<branch>--tN` merged back. Never commit to a default branch.
- Sigil CI stays self-contained (owner, G0b Q2): tests needing aeon run in the landing bar (`scripts/landing-run.sh --aeon <provisioned ref>`); sigil carries a vendored copy of the schema and vectors.
- `SIGIL_BUILD`/`SIGIL_EMIT` point at THIS branch's own release build (`CARGO_TARGET_DIR=.target-land`). Missing env = BLOCKED.
- `sigil build` rewrites `aeon/engine/sound/generated/`: never on a live aeon tree; build in a run-unique copy.
- Gates follow invariant 8 (the RF protocol in `2026-11-16-sigil-soundness.md`): mutation shown on disk, red run, restore from a committed baseline, green run, all quoted in the task's note section.
- Never `mcp__oracle__*`. Headless Oracle tests that start their own process are allowed.
- Exact-path `git add`; `git show --stat` after each commit; the finding goes in the commit body. No Claude co-author trailer.
- Doc sync at landing: the parcel note `sigil/docs/superpowers/notes/2026-11-17-g0a-build-export.md`, `empyrean/docs/v2/tasks/G0a Line table + Z80 symbols now.md` (status, branch, Progress line with SHA), sigil `docs/QUEUE.md` rows.

## Review Focus

1. **A ROM rebuilt in place while an old sidecar survives.** `s4.bin` rebuilt with `-o` but the build fails at the sidecar step: Oracle must refuse the stale `s4.build.json`, never bind it. Test in Task 3 (`stale_sidecar_is_removed_before_install`) and Task 14 (`refuses_sidecar_for_other_rom`).
2. **The `org` back-patch** (`org Hdr / dc.b n / org End`): the patched bytes belong to the patching line, the earlier fill rows lose those bytes, and no byte is covered twice. Test in Task 6 (`org_backpatch_last_writer_wins`).
3. **Z80 banked addresses.** Two songs in different banks both at Z80 `$8000`: lookup by name gives each its own `lma`; reverse lookup by `(z80, $8000)` without a bank is ambiguous and must say so, not pick one. Tests in Task 9 (`banked_rows_carry_lma`) and Task 14 (`z80_reverse_lookup_needs_bank`).
4. **A template body expanded from two call sites.** A PC in either expansion must report the line a person wrote at the call site (outermost), never the template's body line, and the two expansions must report different lines. Test in Task 7 (`two_splices_report_their_own_call_lines`).
5. **A consumer meeting a newer minor with an unknown record kind** (a 1.1 file carrying `regions`) keeps working, and a `major: 2` file is refused naming the version. Vectors in Task 2 (`pass/minor-1-1-unknown-keys.json`, `red/major-2.json`); reader tests in Task 11 and Task 14.

## File map

| Repo | File | Responsibility |
|---|---|---|
| sigil | `crates/sigil-link/src/export/mod.rs` (create) | `BuildExport` model, `build_export`, `ExportError` |
| sigil | `crates/sigil-link/src/export/lines.rs` (create) | the line walk (Org replay, last-writer-wins), completeness |
| sigil | `crates/sigil-link/src/export/json.rs` (create) | deterministic JSON writer |
| sigil | `crates/sigil-link/src/export/locate.rs` (create) | `SpanLocator` trait, `FileRef`, `RootName` |
| sigil | `crates/sigil-link/Cargo.toml` | add `serde_json` (preserve_order), `sha2` |
| sigil | `crates/sigil-harness/src/native.rs` | `ChainSources` implements `SpanLocator`; `RomBuild` gains `export_inputs`; seam 1 hand-off |
| sigil | `crates/sigil-harness/src/seam1.rs` | `parse_file` with real ids; Z80 symbol + row side table |
| sigil | `crates/sigil-frontend-emp/src/export_side.rs` (create) | `.emp` symbol side table, layout and const export |
| sigil | `crates/sigil-frontend-emp/src/eval/asm.rs` | `Frame::Template` expansions at each splice site |
| sigil | `crates/sigil-span/src/lib.rs` | `Frame::Template(String)` variant |
| sigil | `crates/sigil-frontend-as/src/export_side.rs` (create) | AS symbol side table (label / equ) |
| sigil | `crates/sigil-span/src/read_set.rs` | `FileRead` gains `sha256: [u8; 32]` |
| sigil | `crates/sigil-cli/src/main.rs` | sidecar write in `run_build_native`; `--no-build-json` |
| sigil | `contract/build/` (create) | vendored schema + vectors + `SOURCE` (empyrean commit + sha256 of each file) |
| sigil | `crates/sigil-cli/tests/build_export_*.rs` (create) | gates 1-8 (sigil side) |
| empyrean | `contract/schema/build/empyrean.build-1.schema.json` (create) | the schema |
| empyrean | `contract/schema/build/vectors/{pass,red}/*.json` (create) | vectors |
| empyrean | `contract/schema/tests/validate_build_schema.py` (create) | vector runner |
| empyrean | `contract/protocol.md` | §4 address space, `romSha256` amendments |
| empyrean | `clients/python/empyrean_build.py` (create) | reader |
| aeon | `tools/leg_provenance.py`, `tools/leg_survey.py`, `tools/leg_run.py` | `romSha256` in place of CRC-32 |
| oracle | `crates/oracle-core/src/build_export.rs` (create) | load, bind, lookup by space, PC to line |
| oracle | `crates/oracle-aether/src/engine.rs`, `decoders.rs` | bus amendments; Sst decoder gate |

## Baseline to measure first (Task 0)

Before Task 1, the controller records in the parcel note, at sigil `parcel/g0a-build-export` base and aeon HEAD: the 7 shapes' CRC-32 and SHA-256 (`scripts/step0_shapes_cmp.sh`); `grep -c` of `SndDrv_SetBank|Psg_NoteOn|Seq_Op_Dac` in `s4.lst` (expected 0, the gate-3 red); the listing's symbol count per shape; the count of `reauthor_user_items` and `with`-bracket splice sites in `sigil-frontend-emp/src/eval/asm.rs` (today 5 `reauthor_user_items` call sites); the 3 residual `.asm` files. Re-verify each cited line in this plan at the base SHA and correct the plan in place if one moved (cite by symbol, not line, in code comments).

---

### Task 1: Spike: export model, JSON writer, size and load time

**Files:**
- Create: `crates/sigil-link/src/export/{mod.rs,json.rs,locate.rs}`
- Modify: `crates/sigil-link/src/lib.rs` (`pub mod export;`), `crates/sigil-link/Cargo.toml`
- Test: `crates/sigil-link/src/export/json.rs` (unit), `crates/sigil-cli/tests/build_export_spike.rs`

**Interfaces:**
- Produces:
  - `pub struct BuildExport { pub rom: RomRecord, pub inputs_sha256: [u8; 32], pub tool: ToolRecord, pub shape: ShapeRecord, pub spaces: Vec<SpaceRecord>, pub files: Vec<FileRecord>, pub symbols: Vec<SymbolRecord>, pub layouts: Vec<LayoutRecord>, pub consts: Vec<ConstRecord>, pub lines: LineTable }`
  - `pub struct RomRecord { pub path: String, pub size: u64, pub sha256: [u8; 32] }`
  - `pub enum Space { M68k, Z80 }` (serialises `"m68k"`, `"z80"`; closed enum)
  - `pub struct SymbolRecord { pub name: String, pub demangled: Option<String>, pub space: Option<Space>, pub vma: u32, pub lma: Option<u32>, pub kind: SymbolKind, pub size: Option<u32>, pub module: Option<String>, pub public: bool }`; `pub enum SymbolKind { Proc, Label, Data, Equ, Ram }` (closed)
  - `pub struct LineTable { pub sections: Vec<LineSection> }`, `pub struct LineSection { pub name: String, pub space: Space, pub rows: Vec<LineRow> }`, `pub struct LineRow { pub vma: u32, pub lma: u32, pub len: u32, pub file: u32 /* index into files */, pub line: u32, pub kind: RowKind }`, `pub enum RowKind { Code, Data, NoSource(NoSourceKind) }`, `pub enum NoSourceKind { Fill, OrgGap, Deb2Appendix, HeaderChecksum }`
  - `pub fn to_json(e: &BuildExport) -> String` (compact, keys in the order of §4.6 of the spec, final `\n`)
  - `pub fn sha256_hex(b: &[u8; 32]) -> String`
- The JSON for `lines` is columnar: `{"columns": ["vma","lma","len","file","line","kind"], "sections": {"<name>": {"space": "m68k", "rows": [[...], ...]}}}`; `kind` is one of `"code"`, `"data"`, `"fill"`, `"org-gap"`, `"deb2"`, `"checksum"`.

- [ ] **Step 1:** Write `to_json_is_deterministic_and_ordered`: build a two-symbol, one-section `BuildExport` literal; assert `to_json` twice gives equal strings, the string starts `{"format":"empyrean.build","version":{"major":1,"minor":0},"rom":`, contains no `0x`/`$` hex, and `serde_json::from_str` round-trips it.
- [ ] **Step 2:** Run `cargo test -p sigil-link export::json` → FAIL (module missing).
- [ ] **Step 3:** Implement the model and `to_json` with `serde_json::Map` built in fixed key order (no derive-order reliance). Symbols sorted by `(space, vma, name)`; files by `(root, path)`; layouts and consts by `(module, name)`; line rows by `lma`.
- [ ] **Step 4:** Spike behind an env flag `SIGIL_EXPORT_SPIKE=1` in `run_build_native`: emit only `rom`, `tool`, `shape`, symbols from `RomBuild.listing` (kind `Label`/`Equ`, no space), and line rows at section granularity. Build s4 and s4 debug in a run-unique aeon copy; record file size, `python3 -c 'import json,time;t=time.time();json.load(open(p));print(time.time()-t)'` load time, and symbol count in the parcel note with wall-clock uptime.
- [ ] **Step 5:** Tests pass; `SHAPES 7/7 IDENTICAL`; commit with the measured numbers in the body. STOP gate: if the full-export projection (rows ≈ fragments) exceeds 20 MB, or the Python load time exceeds 1 s, BLOCKED: owner call on format B (spec §4.1).

### Task 2: Schema 1.0, vectors, protocol amendments (empyrean)

**Files:**
- Create: `empyrean/contract/schema/build/empyrean.build-1.schema.json`, `.../vectors/pass/{s4-minimal,demo-minimal,minor-1-1-unknown-keys,z80-banked}.json`, `.../vectors/red/{major-2,missing-rom-sha256,sha256-uppercase,hex-string-number,absolute-path,unknown-space,timestamp-key-in-envelope,lines-kind-unknown}.json`, `contract/schema/tests/validate_build_schema.py`
- Modify: `empyrean/contract/protocol.md` (§4 symbol methods, `emulator/romReloaded` row at the `buildId` line, the `load_symbols` forward note, `status`)
- Create (sigil, same task, second commit): `sigil/contract/build/` copy + `SOURCE`

**Interfaces:**
- Produces: schema `$id` `https://empyrean.invalid/contract/build/1`; the envelope (`format`, `version`, `rom`, `inputsSha256`, `tool`, `shape`, `spaces`) has `additionalProperties: false`; every record payload is open. `rom.sha256` pattern `^[0-9a-f]{64}$`. `files[].path` pattern refuses a leading `/` and any `..` segment. `space` enum `["m68k","z80"]`.
- Protocol amendment (one, per owner Q1+Q2), text decided here:
  - `emulator/romReloaded`: `buildId` is replaced by `romSha256` (string, 64 lowercase hex), always present.
  - `emulator/status` result and every measurement result gain `romSha256`; a result after an in-memory patch carries `romPatched: true`.
  - `lookup_symbol`, `run_to`, `stopped`: optional `space` (`"m68k"` default, `"z80"`); a Z80 reverse lookup in `$8000-$FFFF` takes `bank` and is refused (`-32602`, `data.reason: "ambiguous-bank"`) without one when more than one bank matches.
  - `load_symbols` accepts a `.build.json` path; `binding` gains the value `bound-sha256`; a mismatch is refused (`-32602`, `data.binding: "mismatch"`). The forward note is struck and dated.

- [ ] **Step 1:** Write the vectors first and `validate_build_schema.py` (checks: the schema against the 2020-12 metaschema; every pass vector validates; every red vector fails **with the error path the vector's `expect` field names**, so a red vector failing for the wrong reason is itself a failure).
- [ ] **Step 2:** Run `python3 contract/schema/tests/validate_build_schema.py` → FAIL (schema missing).
- [ ] **Step 3:** Write the schema; amend `protocol.md`.
- [ ] **Step 4:** Validator green; also run the existing `validate_contract_schema.py` (the bus schema must take the amended fields: add pass vectors for `status` with `romSha256` and a `lookup_symbol` with `space`, a red vector for a bad `space`).
- [ ] **Step 5:** Commit on `feat/g0a-build-contract`. Then vendor into sigil `contract/build/` with `SOURCE` (empyrean SHA + sha256 per file) and a sigil test `vendored_schema_matches_source` that recomputes each hash (runs in CI); the landing bar adds `scripts/check-vendored-contract.sh` comparing against `$EMPYREAN_DIR` (loud SKIP is a failure in the landing bar). Commit on sigil.

### Task 3: ROM hash, write order, sidecar, inputs and per-file hashes

**Files:**
- Modify: `crates/sigil-span/src/read_set.rs` (`FileRead.sha256`), `crates/sigil-cli/src/main.rs` (`run_build_native`, `parse_build_args`)
- Test: `crates/sigil-cli/tests/build_export_binding.rs`

**Interfaces:**
- Consumes: Task 1 model.
- Produces: `fn sidecar_path(rom_out: &Path) -> PathBuf` (replace the final extension: `s4.bin` → `s4.build.json`, `s4.debug.bin` → `s4.debug.build.json`); `--no-build-json` flag; `pub fn inputs_sha256(tool: &ToolRecord, shape: &ShapeRecord, defines: &[(String, i128)], files: &[FileRecord]) -> [u8; 32]` (SHA-256 over a canonical newline-joined text, one line per item, files sorted). `FileRecord { root: RootName, path: String, lang: Lang, sha256: [u8;32], size: u64 }`, built from `read_set::snapshot()` with each read path mapped to a root (`engine/` → `engine`, `games/<g>/` → `game`, `engine/sound/generated/`, `engine/debug/generated/` → `generated`, sigil source → `sigil`); a read outside every root is an error naming the path.

- [ ] **Step 1:** Tests: `sidecar_beside_rom_names_its_sha256` (build demo in a fixture copy; sidecar `rom.sha256` equals `sha256(file bytes)`); `stale_sidecar_is_removed_before_install` (pre-create `r.build.json`, make the build fail after install via a test hook `SIGIL_TEST_FAIL_BEFORE_SIDECAR=1`, assert no sidecar exists); `no_build_json_writes_none`; `inputs_sha256_changes_with_one_source_byte` and `..._ignores_output_path`; `files_have_no_absolute_paths`.
- [ ] **Step 2:** Run → FAIL.
- [ ] **Step 3:** Implement. `FileRead.sha256` is computed at read time from the same bytes as the CRC (never a re-read). The sidecar is written only when `-o` is given. Hash `full` (the exact bytes passed to `install_artifact`).
- [ ] **Step 4:** Green; `SHAPES 7/7 IDENTICAL`; `.lst` byte-identical to base in all 7 shapes (the read-set change must not move the source digest: assert it in `lst_unchanged_by_export`).
- [ ] **Step 5:** Commit.

### Task 4: `SpanLocator`; `ChainSources` behind it; exporter entry

**Files:**
- Modify: `crates/sigil-link/src/export/locate.rs`, `crates/sigil-harness/src/native.rs` (`ChainSources`, `RomBuild`, `build_rom_chained_with_listing`), `crates/sigil-cli/src/main.rs`
- Test: `crates/sigil-harness/tests/export_locator.rs`

**Interfaces:**
- Produces:
  - `pub trait SpanLocator { fn outermost(&self, span: Span) -> Option<SourcePos>; fn file(&self, id: u32) -> &FileRef; }` with `pub struct SourcePos { pub file: u32, pub line: u32 }` (1-based line; `file` indexes `BuildExport.files`).
  - `impl SpanLocator for ChainSources` (follows `Expansion.call` to a file id, the walk `SourceMap::label` already does).
  - `RomBuild.export_inputs: ExportInputs` where `pub struct ExportInputs { pub sections: Vec<Section> /* placed, post-resolve */, pub locator: Box<dyn SpanLocator + Send>, pub side: SideTables }` and `pub struct SideTables { pub symbols: Vec<SymbolRecord>, pub layouts: Vec<LayoutRecord>, pub consts: Vec<ConstRecord>, pub z80: Option<Z80Bridge> }` (empty until Tasks 5, 8, 9).
  - `pub fn build_export(inputs: &ExportInputs, rom: &RomRecord, tool: ToolRecord, shape: ShapeRecord, files: Vec<FileRecord>) -> Result<BuildExport, ExportError>`.
- [ ] **Step 1:** Tests: `locator_resolves_an_as_macro_body_to_its_call_line` (AS fixture with a macro; the expansion's bytes locate to the invocation line) and `locator_resolves_plain_emp_line`.
- [ ] **Step 2:** FAIL. **Step 3:** implement; `run_build_native` calls `build_export` after `install_artifact` succeeds. **Step 4:** green; 7/7. **Step 5:** commit.

### Task 5: Front-end symbol side tables

**Files:**
- Create: `crates/sigil-frontend-emp/src/export_side.rs`, `crates/sigil-frontend-as/src/export_side.rs`
- Modify: `crates/sigil-harness/src/native.rs` (fill `SideTables.symbols`)
- Test: `crates/sigil-cli/tests/build_export_symbols.rs`

**Interfaces:**
- Consumes: the `.emp` front end's resolved program exactly as `native.rs` holds it when lowering (procedures, modules, `pub`, spans), and `ChainedResolve.resolved` for placement. This plan does not name that type; record the name used in the task's commit body.
- Produces: `pub fn emp_symbols(program: &<the resolved program>, placed: &[Section]) -> Vec<SymbolRecord>` (procs with `kind: Proc`, `size` = first byte to last emitted byte in the same section, `module`, `public`); `pub fn as_symbols(...) -> Vec<SymbolRecord>` (label or `Equ`). `name` is the listing's identifying spelling exactly (mangling kept), `demangled` optional. Equates have `space: None` unless the front end knows they are addresses (RAM labels in `Reserve` sections → `kind: Ram`, `space: M68k`).
- [ ] **Step 1:** Tests: `queue_dma_deferrable_is_a_proc_with_extent` (s4 ref tree: `QueueDMA_Deferrable` `kind: proc`, `size` > 0, `vma + size` ≤ next symbol in its section); `equates_have_no_space`; and gate 5 `every_listing_symbol_is_in_the_export_at_the_same_address` (all 7 shapes; planted: shift one export symbol's `vma` by 2 → red).
- [ ] **Steps 2-5:** FAIL; implement; green with RF for gate 5; 7/7; commit.

### Task 6: Line walk, `Org` replay, last-writer-wins, completeness

**Files:**
- Create: `crates/sigil-link/src/export/lines.rs`
- Test: same file (unit) + `crates/sigil-cli/tests/build_export_lines.rs`

**Interfaces:**
- Produces: `pub fn walk_lines(sections: &[Section], loc: &dyn SpanLocator, no_source: &[NoSourceRange]) -> Result<LineTable, ExportError>` with `pub struct NoSourceRange { pub lma: u32, pub len: u32, pub kind: NoSourceKind }`. Cursor replay uses the same rule as `Section::image_bytes` (an `Org` seeks backward or forward; a forward seek's gap is an `OrgGap` row). A byte written twice belongs to the later writer; rows are split so no two rows overlap. Adjacent rows with equal `(file, line, kind)` and contiguous addresses merge.
- `ExportError::Uncovered { section, lma, len }` and `ExportError::Overlap { lma }`.
- [ ] **Step 1:** Unit tests: `org_backpatch_last_writer_wins` (fragments: data 4 B at 0 from line 10, `Org` to 1, data 1 B from line 20, `Org` to 4; expect rows `[0,1)`→10, `[1,2)`→20, `[2,4)`→10); `forward_org_makes_a_gap_row`; `uncovered_byte_is_refused`. Integration gate 2 `every_rom_byte_is_covered_once` over all 7 shapes, where the no-source ranges are the deb2 appendix (`[len(rom_before_appendix), len(full))`), the folded header checksum (`$18E`, 2 B), and section fill; planted: drop one row → red; duplicate one over the header back-patch → red.
- [ ] **Steps 2-5:** FAIL; implement; RF for gate 2; 7/7; commit with row counts per shape in the body.

### Task 7: `.emp` template expansions

**Files:**
- Modify: `crates/sigil-span/src/lib.rs` (`Frame::Template(String)`), `crates/sigil-frontend-emp/src/eval/asm.rs` (every `reauthor_user_items` site and every `with`-bracket splice), `crates/sigil-frontend-emp/src/resolve/manifest.rs` (expansions reach the rebuilt `SourceIndex`), `crates/sigil-harness/src/native.rs` (`ChainSources` accepts expansion ids from both front ends)
- Test: `crates/sigil-cli/tests/build_export_templates.rs`

**Interfaces:**
- Consumes: `SourceMap::add_expansion(body, body_start, call, frame) -> SourceId`.
- Produces: at each splice, the spliced items' `Span.source` is rewritten to a fresh expansion id whose `call` is the splice site's span. No IR struct changes.
- [ ] **Step 1:** Tests: `template_bytes_report_the_call_line` (fixture: a comptime template spliced once; the row for its bytes carries the call line); `two_splices_report_their_own_call_lines` (Review Focus 4); `code_value_built_in_one_place_spliced_in_another_reports_the_splice` ; `splice_site_census_is_complete`: a test that greps `asm.rs` for `reauthor_user_items(` and `with`-splice entry points and asserts each is listed in a `SPLICE_SITES` const beside the rewrite helper, so a new splice site without attribution goes red (planted: add a dummy call → red).
- [ ] **Step 2:** FAIL. **Step 3:** implement. **Step 4:** green; **byte-identical proof is the point of this task**: 7/7, and every `.lst` byte-identical to base (span rewriting must not reach diagnostics that feed the listing). **Step 5:** commit. STOP gate: if any shape moves, BLOCKED (do not "fix" by narrowing the rewrite).

### Task 8: Layouts and constants

**Files:**
- Modify: `crates/sigil-frontend-emp/src/export_side.rs`
- Test: `crates/sigil-cli/tests/build_export_layouts.rs`

**Interfaces:**
- Consumes: `layout.rs` `Layout`, `FieldLayout`, `BitfieldLayout`, `OverlayInfo`; A4's definition-site resolution (`resolve::hygiene`), which computes each comptime item in its defining module.
- Produces: `pub fn emp_layouts(...) -> Result<Vec<LayoutRecord>, ExportError>`, `pub fn emp_consts(...) -> Result<Vec<ConstRecord>, ExportError>`. `LayoutRecord { name, module, kind: "struct"|"bitfield"|"enum", size, fields: [{name, offset, size, type}], overlays: [{name, base, offset, size, fields}] }`; `type` is the resolved spelling (`u16`, `i8`, `fx8.8`, `ptr<Sst>`, newtype name). `ConstRecord { name, module, type, value: i64 }` for integer, bool (0/1, `type: "bool"`) and enum values only. **Two-layouts refusal:** the exporter collects every resolved layout keyed by `(module, name)`; two different layouts for one key is `ExportError::TwoLayouts { module, name }` (A4 made this unreachable today; the refusal keeps it so).
- [ ] **Step 1:** Tests: `sst_is_exported_at_size_80` (`Sst`, `size: 80`, field `code_addr` at 0 size 2, the `$30` overlay present in `overlays`); `consts_are_integers_only` (no record whose source is `Data`/`Code`/table); `two_layouts_is_refused` (unit: feed two layouts for one key; planted mutation of the check → green, i.e. red-first shown).
- [ ] **Steps 2-5:** standard; 7/7; commit.

### Task 9: Z80 bridge

**Files:**
- Modify: `crates/sigil-harness/src/seam1.rs` (`parse_file` with each file's own `SourceId` from `FileSpec.rel_path`, both call sites now using `parse_str`), `crates/sigil-harness/src/native.rs` (`emit_generated_in` returns the seam's symbols and rows in process into `SideTables.z80`)
- Test: `crates/sigil-cli/tests/build_export_z80.rs`

**Interfaces:**
- Produces: `pub struct Z80Bridge { pub blob_lma: u32 /* where boot_data.emp embeds it */, pub symbols: Vec<SymbolRecord> /* space Z80 */, pub sections: Vec<LineSection> /* space Z80, rows rebased to ROM lma */ }`. Every Z80 symbol and row in `$8000-$FFFF` carries `lma`; the bank is `lma >> 15`. The bridge's rows replace the embedding line's single data row for the blob's byte range (the walk's completeness check still holds).
- [ ] **Step 1:** Tests: gate 3 `snddrv_setbank_resolves_in_z80_space` (s4 export: `SndDrv_SetBank` `space: z80`; also `Psg_NoteOn`, `Seq_Op_Dac`; red today, measured in Task 0); `banked_rows_carry_lma` (Review Focus 3); `z80_rows_locate_to_driver_files`; `sound_off_shapes_have_no_z80_rows` (config B / lean, where the driver is absent).
- [ ] **Steps 2-5:** standard; 7/7 (the `parse_file` change must not move a byte); commit.

### Task 10: Listing-equivalence, determinism and second-game gates

**Files:**
- Test: `crates/sigil-cli/tests/build_export_gates.rs` (landing-bar suite; `#[ignore]` with reason in CI, run by `landing-run.sh`)

- [ ] **Step 1:** Tests: gate 7 `five_builds_one_export_hash_per_shape` (7 shapes × 5 builds in run-unique copies); gate 8 `demo_export_validates_and_has_no_sonic_symbol_in_engine_records` (demo and demo debug validate against the vendored schema via `jsonschema`-equivalent check in Rust **or** by shelling to the vendored Python validator, decided by Task 2's runner; no symbol whose `module` starts `engine.` names a `games.sonic4` item; planted: inject a `games.sonic4.Sonic_Init` row into a demo export → red); gate 6 `every_export_validates_against_the_vendored_schema` (all 7 shapes).
- [ ] **Steps 2-5:** RF for gates 7 and 8; green; landing-run `RESULT GREEN` with aggregate totals; commit.

### Task 11: Python reader (empyrean), then the harness move (aeon)

**Files:**
- Create: `empyrean/clients/python/empyrean_build.py`, `empyrean/clients/python/test_empyrean_build.py`
- Modify (aeon `feat/g0a-romsha256`): `tools/leg_provenance.py` (`rom_provenance` returns `romSha256` from the sidecar, keeps `crc32` for one release as `crc32Legacy`), `tools/leg_survey.py` (one `romSha256` per ROM file, the CRC checks re-keyed), `tools/leg_run.py` (`--build-json` replaces `--lst`; `--lst` refused with a message naming the replacement), their tests; `docs/research/2026-09-25-perf-survey/leg_probe.py` stays (record).

**Interfaces:**
- Produces: `load(path: str, rom: bytes | None = None, *, unbound: bool = False) -> BuildExport`; raises `UnknownMajor(found: int)` before reading any other key, `NotBound(expected: str, found: str)` when `rom` is given and its SHA-256 differs; `unbound=True` is the only way to skip the check and the returned object's `.bound` is then `False`. `BuildExport.symbol(name, space="m68k") -> Symbol`, `.at(space, addr, bank=None) -> Symbol` (raises `AmbiguousBank`), `.line_at(space, addr, bank=None) -> tuple[str, int] | None`, `.layout(name)`, `.const(name)`.
- [ ] **Step 1:** Reader tests against Task 2's vectors (pass vectors load; `major-2` raises `UnknownMajor(2)`; `minor-1-1-unknown-keys` loads; a one-byte-patched ROM raises `NotBound`). Harness tests: `test_leg_provenance.py::test_rom_identity_is_sha256_from_sidecar`, `test_leg_survey.py::test_one_sha256_per_rom_file` (planted: two cells with different `romSha256` for one ROM → mismatch reported).
- [ ] **Steps 2-5:** standard. The E1 v1 reference keeps its recorded CRC-32 values (a record) and gains `romSha256` beside them only if re-recorded; do not rewrite records. Aeon green: `./build.sh`, `DEBUG=1 ./build.sh`, `./build.sh demo`, `DEBUG=1 ./build.sh demo`. Commit in each repo.

### Task 12: Sigil landing

- [ ] Final whole-branch review (fresh reviewer, no conclusions handed in) against the spec's §6 gates and this plan's Review Focus.
- [ ] `landing-run.sh` green with totals; 7/7; vendored contract matches empyrean `feat/g0a-build-contract` tip.
- [ ] Merge order (controller only, with the owner's OK on the exact push list): empyrean contract → sigil → aeon harness → oracle. Doc sync per Global Constraints.

### Task 13: Oracle consumption (oracle `feat/g0a-build-export`)

**Files:**
- Create: `crates/oracle-core/src/build_export.rs` (load with `serde_json`, version check, binding, lookup by space and bank, PC to line by binary search over `lma`-sorted rows)
- Modify: `crates/oracle-aether/src/engine.rs` (`load_symbols` accepts `.build.json`; `status`, `romReloaded` and measurement results carry `romSha256`; `space`/`bank` on `lookup_symbol`, `run_to`, `stopped`; a `source_line` answer on `stopped` when an export is bound), `crates/oracle-aether/src/decoders.rs` (gate)
- Test: `crates/oracle-core/tests/build_export.rs`, `crates/oracle-aether/tests/build_export_bus.rs`

**Interfaces:**
- Consumes: Task 2 protocol amendment (exact field names there); Task 1 JSON shape.
- Produces: `pub fn load_build_export(json: &str, image: &[u8]) -> Result<BuildExport, BindError>` with `BindError::{UnknownMajor(u64), Mismatch { expected: String, found: String }}`; the hash is of the image as loaded from the file, before pad, mirror or runtime patch; a later in-memory patch sets `romPatched: true` on results.
- [ ] **Step 1:** Tests: gate 1 `refuses_sidecar_for_other_rom` (s4 export beside `s4.debug.bin` → Mismatch; beside a one-byte-patched `s4.bin` → Mismatch; lean shape → binds, where the deb2 probe said indeterminate); `z80_reverse_lookup_needs_bank`; `pc_to_line_lands_on_outermost_line` (a PC inside a template expansion); gate 4 `aeon_sst_fields_agree_with_exported_layout`: each of the 29 `AEON_SST_FIELDS` entries matches the exported `Sst` field of that name at the same offset and size; the curated omissions (the `$30` overlay window, the `$4E` tail) are listed in the test as the §11.25 rule-3 exemptions and must still exist in the export (so a removed field also goes red); planted: export `Sst` at size `$52` → red.
- [ ] **Steps 2-5:** standard; Oracle's own suite green with aggregate totals; commit.

## Dependencies and parallelism

Task 0 → 1 → (2 ∥ 3) → 4 → (5 ∥ 6) → 7 → 8 → 9 → 10 → 11 → 12. Task 13 starts once Task 2 is committed and Task 6 has produced a real s4 export (it reads exports from the landing-bar fixtures). Task 7 and Task 9 are the least certain (spec §7 estimate: 3 to 4 weeks for Tasks 1-11 on the Sigil side, ESTIMATED); if either exceeds 5 working days, report BLOCKED-or-continue to the owner with what is left.

## Out of scope here (booked)

- Aurora consumption: its own plan in aurora, after landing (spec §7 item 12).
- Moving the post-build Python gates off the listing; deleting the `.lst`.
- Line stepping in Oracle (follow-on task); the 1.1 expansion chain and `regions`.
- G0b's comparison job uses this export from its first run (G0b plan).
