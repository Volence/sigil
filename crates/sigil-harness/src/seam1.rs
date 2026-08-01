//! seam-1 — the resident-sound-blob NATIVE LINK, as a reusable library.
//!
//! The five sound `.emp` files (z80_sound_driver / sound_sequencer / sound_sfx /
//! sound_fm / sound_psg) link as ONE native Z80 module set at VMA `$0000` / LMA
//! `$3DE` (plain) / `$3E2` (debug), blob order driver→sequencer→sfx→fm→psg. The 47
//! intra-blob cross-file `extern proc` references resolve INTERNALLY against the
//! sibling sections' `pub proc` exports.
//!
//! Two consumers share this machinery: the whole-ROM gates
//! (`seam1_native_link.rs`) and the `emit_sound_blob` bin that produces the
//! canonical build inputs asl consumes after the twins are deleted (Option A —
//! sigil emits the blob bytes + the exported-symbol contract; asl packs the
//! not-yet-flipped remainder). The emit is byte-DETERMINISTIC from the tracked
//! `.emp` sources + the sigil toolchain version, and the canonical CRC bar is what
//! proves it — the blob is provenance-tracked exactly like the ROMs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sigil_frontend_emp::ast;
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SectionPlacement, SymbolTable};

/// The plain-shape blob length (`Z80_SOUND_SIZE`, s4.lst) — 6172 B.
pub const BLOB_LEN_PLAIN: usize = 0x181C;
/// The debug blob length = plain + $7E (the sequencer's 16 `if DEBUG==1` bodies).
pub const BLOB_LEN_DEBUG: usize = 0x181C + 0x7E;

/// The blob's LMA base = `Z80_Sound_Start` = `BootData + 54`. SHAPE-DEPENDENT: the
/// debug shape grows +4 UPSTREAM of BootData (boot `__DEBUG__` content), so the
/// whole blob slides to `$3E2` in debug — VERIFIED from s4.lst / s4.debug.lst.
pub fn blob_lma(debug: bool) -> u32 {
    if debug {
        0x3E2
    } else {
        0x3DE
    }
}

/// The exported-symbol CONTRACT: the sequencer opcode handlers the banked
/// `seq_opcode_tab.emp` references via `dc.w <handler>` (26 distinct labels,
/// 36 table slots — several ops share a handler). `emit_seq_opcode_tab` resolves
/// these VMAs in-link off `native_sound_blob().symbols` (no AS syms file is
/// emitted — the whole banked seq table is native). All 26 live in
/// `sound_sequencer.emp`, so their VMAs are read off that section's labels.
pub const HANDLER_SYMBOLS: &[&str] = &[
    "Seq_BadOpcode",
    "Seq_Op_Dac",
    "Seq_Op_Detune",
    "Seq_Op_End",
    "Seq_Op_Jump",
    "Seq_Op_Lfo",
    "Seq_Op_LoopPoint",
    "Seq_Op_Macro",
    "Seq_Op_ModSet",
    "Seq_Op_NoteDur",
    "Seq_Op_NoteFill",
    "Seq_Op_NoteRaw",
    "Seq_Op_OpBias",
    "Seq_Op_Pan",
    "Seq_Op_Patch",
    "Seq_Op_PitchEnv",
    "Seq_Op_Porta",
    "Seq_Op_PsgEnv",
    "Seq_Op_PsgNoise",
    "Seq_Op_RegDelta",
    "Seq_Op_RegWrite",
    "Seq_Op_RepeatEnd",
    "Seq_Op_RepeatStart",
    "Seq_Op_SpinRev",
    "Seq_Op_Tempo",
    "Seq_Op_Vol",
];

struct FileSpec {
    rel_path: &'static str,
    section: &'static str,
    vma_plain: u32,
    vma_debug: u32,
    /// The NAMES of the sound constants this resident module folds — the `-D` env
    /// the emit seeds. The VALUES are resolved from the `sound_constants.emp`
    /// authority (or the small `seam_emit_config` list for the genuinely-external
    /// game/data config) by [`resolve_consts`]. No value is hand-maintained here.
    const_names: fn() -> &'static [&'static str],
}

fn file_specs() -> Vec<FileSpec> {
    vec![
        FileSpec {
            rel_path: "engine/sound/z80_sound_driver.emp",
            section: "z80_sound_driver",
            vma_plain: 0x0000,
            vma_debug: 0x0000,
            const_names: driver_const_names,
        },
        FileSpec {
            rel_path: "engine/sound/sound_sequencer.emp",
            section: "sound_sequencer",
            vma_plain: 0x0565,
            vma_debug: 0x0565,
            const_names: sequencer_const_names,
        },
        FileSpec {
            rel_path: "engine/sound/sound_sfx.emp",
            section: "sound_sfx",
            vma_plain: 0x0CD7,
            vma_debug: 0x0D55,
            const_names: sfx_const_names,
        },
        FileSpec {
            rel_path: "engine/sound/sound_fm.emp",
            section: "sound_fm",
            vma_plain: 0x12C3,
            vma_debug: 0x1341,
            const_names: fm_const_names,
        },
        FileSpec {
            rel_path: "engine/sound/sound_psg.emp",
            section: "sound_psg",
            vma_plain: 0x1660,
            vma_debug: 0x16DE,
            const_names: psg_const_names,
        },
    ]
}

/// The BANKED `$8000`-window symbols (LUTs / opcode + win tables) — genuinely
/// external seam-2 data, shape-INVARIANT. Supplied as equ carriers for the
/// STANDALONE blob (`native_sound_blob`); in the whole-ROM link they instead
/// resolve against the AS side. `DacSampleTable` is a driver `-D`, not here.
fn banked_carriers() -> Vec<(&'static str, i64)> {
    vec![
        ("SeqOpcodeTable", 0x856D),
        ("SfxBlobWinTab", 0x845F),
        ("FmPitchTableZ", 0x8000),
        ("LogVolumeLutZ", 0x817C),
        ("CarrierMaskTableZ", 0x827C),
        ("SndDefaultPitchTable", 0x8357),
        ("PsgDivisorTableZ", 0x80BE),
        ("PsgVolEnv_Ids", 0x8284),
        ("PsgVolEnv_Ptrs", 0x828F),
        ("FmVolEnv_Ids", 0x8335),
        ("FmVolEnv_Ptrs", 0x8338),
    ]
}

/// Parse one resident `.emp` file, returning its AST + its directory (the include
/// root). Panics on a parse error — the blob is a hard build dependency.
fn parse_one(aeon: &Path, spec: &FileSpec) -> (ast::File, PathBuf) {
    let path = aeon.join(spec.rel_path);
    let dir = path.parent().expect("file has a parent dir").to_path_buf();
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {pdiags:?}",
        spec.rel_path
    );
    (file, dir)
}

/// A module's `invariant: preserves(...)` reglist segments (empty if none) — the
/// register units every proc in the module inherits (mirrors `lower/mod.rs`'s
/// `validate_module_invariants` reader). Unioned into an imported callee's stub so
/// a consumer credits the inherited-preserve (e.g. fm/psg's module `preserves(ix)`).
fn module_invariant_reglist(file: &ast::File) -> Vec<(String, Option<String>)> {
    for (name, expr) in &file.module.attrs {
        if name != "invariant" {
            continue;
        }
        if let ast::Expr::Call { callee, args, .. } = expr {
            if callee.segments.last().map(String::as_str) == Some("preserves") {
                return args
                    .iter()
                    .filter_map(|a| match &a.value {
                        ast::Expr::Path(p) if p.segments.len() == 1 => {
                            Some((p.segments[0].clone(), None))
                        }
                        _ => None,
                    })
                    .collect();
            }
        }
    }
    Vec::new()
}

/// The cross-module import-resolution table: every `pub proc` across the five
/// resident files → the `extern proc` contract stub a CONSUMER module needs to
/// credit a `call` to it. The stub's contract is DERIVED from the definition (its
/// declared clauses, its `preserves` UNIONED with its home module's `invariant`),
/// so there is no hand-written decl to drift (the seam retires the extern-decl-vs-def
/// hazard structurally). Emits nothing — an `extern proc` is a byte-neutral decl.
fn import_stub_table(aeon: &Path) -> BTreeMap<String, ast::ExternProcDecl> {
    let mut out = BTreeMap::new();
    for spec in &file_specs() {
        let (file, _dir) = parse_one(aeon, spec);
        let inv = module_invariant_reglist(&file);
        collect_pub_proc_stubs(&file.items, &inv, &mut out);
    }
    out
}

fn collect_pub_proc_stubs(
    items: &[ast::Item],
    inv: &[(String, Option<String>)],
    out: &mut BTreeMap<String, ast::ExternProcDecl>,
) {
    use sigil_frontend_emp::regfile::{expand_reglist, RegFile};
    let expand = |segs: &[(String, Option<String>)]| expand_reglist(segs, RegFile::Z80, |_| {});
    for it in items {
        match it {
            ast::Item::Proc(p) if p.public => {
                // The callee's PRESERVED units, resolved from its DEFINITION. A proc
                // with a complete `clobbers` contract preserves everything it does
                // NOT clobber-or-return (the honest-contract theorem — sound because
                // `[call.clobbers-incomplete]` verifies the corpus's clobbers are
                // complete); a proc with no `clobbers` clause falls back to its
                // declared preserves. The home module's `invariant` is unioned in.
                let declared = expand(&p.preserves);
                let inv_units = expand(inv);
                let mut units: std::collections::BTreeSet<String> = match &p.clobbers {
                    Some(clob) => {
                        let clobbered = expand(clob);
                        let produced = p.out.as_deref().map(expand).unwrap_or_default();
                        RegFile::Z80
                            .universe()
                            .into_iter()
                            .filter(|u| !clobbered.contains(u) && !produced.contains(u))
                            .collect()
                    }
                    None => std::collections::BTreeSet::new(),
                };
                units.extend(declared);
                units.extend(inv_units);
                let preserves: Vec<(String, Option<String>)> =
                    units.into_iter().map(|u| (u, None)).collect();
                let sig = ast::ProcSig {
                    params: p
                        .params
                        .iter()
                        .map(|(n, t, s)| (n.clone(), Some(t.clone()), *s))
                        .collect(),
                    // An extern decl carries preserves + out (the caller-consumed
                    // clauses); `clobbers` stays undeclared, matching the retired
                    // hand-written externs' shape.
                    clobbers: None,
                    preserves,
                    out: p.out.clone(),
                    out_flags: p.out_flags.clone(),
                    out_cond: p.out_cond.clone(),
                    out_types: p.out_types.clone(),
                };
                out.insert(
                    p.name.clone(),
                    ast::ExternProcDecl { public: false, name: p.name.clone(), sig, span: p.span },
                );
            }
            ast::Item::Section(sec) => collect_pub_proc_stubs(&sec.items, inv, out),
            _ => {}
        }
    }
}

/// The derived contract stubs for the `use engine.sound_*.{...}` imports a file
/// declares — prepended to the file's items so the Z80 callee-preserves oracle
/// credits its cross-module `call`s (the `extern proc` machinery, now DERIVED from
/// the sibling definitions rather than hand-declared).
fn use_import_stubs(
    items: &[ast::Item],
    table: &BTreeMap<String, ast::ExternProcDecl>,
    out: &mut Vec<ast::Item>,
) {
    for it in items {
        match it {
            ast::Item::Use(u) => match &u.names {
                ast::UseNames::List(names) => {
                    for n in names {
                        if let Some(stub) = table.get(n) {
                            out.push(ast::Item::ExternProc(stub.clone()));
                        }
                    }
                }
                ast::UseNames::Glob | ast::UseNames::Whole => {}
            },
            ast::Item::Section(sec) => use_import_stubs(&sec.items, table, out),
            _ => {}
        }
    }
}

/// Lower one resident `.emp` file (const seam `-D` + the shape `DEBUG` flag, with an
/// optional single-symbol doctor), returning its single named section. The file's
/// `use` imports are resolved to derived contract stubs (`table`) and prepended, so
/// the cross-module `call`s' callee-preserves credit exactly as the retired
/// hand-written `extern proc` decls provided (byte-neutral — decls emit nothing).
fn lower_one(
    aeon: &Path,
    spec: &FileSpec,
    debug: bool,
    doctor: Option<(&str, i64)>,
    table: &BTreeMap<String, ast::ExternProcDecl>,
) -> Section {
    let (file, dir) = parse_one(aeon, spec);
    let mut imported: Vec<ast::Item> = Vec::new();
    use_import_stubs(&file.items, table, &mut imported);
    let file = if imported.is_empty() {
        file
    } else {
        ast::File {
            module: file.module.clone(),
            attrs: file.attrs.clone(),
            items: imported.into_iter().chain(file.items.iter().cloned()).collect(),
            docs: file.docs.clone(),
        }
    };
    let mut defines: Vec<(String, i128)> = resolve_consts(aeon, (spec.const_names)())
        .into_iter()
        .map(|(n, v)| {
            let v = match doctor {
                Some((dn, dv)) if dn == n => dv,
                _ => v,
            };
            (n.to_string(), v as i128)
        })
        .collect();
    defines.push(("DEBUG".to_string(), if debug { 1 } else { 0 }));
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir),
        embed_base: None,
        defines,
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} lower errors: {:?}",
        spec.rel_path,
        ldiags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    module
        .sections
        .into_iter()
        .find(|s| s.name == spec.section)
        .unwrap_or_else(|| panic!("{} did not emit section {}", spec.rel_path, spec.section))
}

/// The STANDALONE native-linked blob: bytes + the exported-symbol contract. Used by
/// the `emit_sound_blob` bin (Option A) and the whole-blob byte gate. The banked
/// `$8000` tables are supplied as carriers here (shape-invariant fixed values, so
/// the blob is self-contained); the intra-blob externs resolve internally.
pub struct NativeSoundBlob {
    /// The flattened blob bytes (driver→seq→sfx→fm→psg), `Z80_SOUND_SIZE` long.
    pub bytes: Vec<u8>,
    /// The exported-symbol contract: `(name, VMA)` for each `HANDLER_SYMBOLS` entry
    /// (the sequencer opcode handlers the banked table references), per shape.
    pub symbols: Vec<(String, u32)>,
}

/// Build the standalone native-linked blob for `debug`, returning bytes + symbols.
pub fn native_sound_blob(aeon: &Path, debug: bool) -> NativeSoundBlob {
    NativeSoundBlob { bytes: native_blob_doctored(aeon, debug, None), symbols: handler_symbols(aeon, debug) }
}

/// The 26 handler VMAs read off `sound_sequencer`'s labels (`vma_base + offset`),
/// per shape. `sound_sequencer` starts at `$0565` in BOTH shapes; its internal
/// `if DEBUG==1` growth re-bases the handlers AFTER a debug block, so the values are
/// shape-DEPENDENT and read from the lowered section directly.
fn handler_symbols(aeon: &Path, debug: bool) -> Vec<(String, u32)> {
    let specs = file_specs();
    let seq = specs.iter().find(|s| s.section == "sound_sequencer").expect("sequencer spec");
    let table = import_stub_table(aeon);
    let sec = lower_one(aeon, seq, debug, None, &table);
    let base = if debug { seq.vma_debug } else { seq.vma_plain };
    let mut out = Vec::new();
    for want in HANDLER_SYMBOLS {
        let label = sec
            .labels
            .iter()
            .find(|l| l.name == *want)
            .unwrap_or_else(|| panic!("sound_sequencer.emp must export handler `{want}`"));
        out.push((want.to_string(), base + label.offset));
    }
    out
}

/// The flattened standalone blob bytes with an optional single-symbol doctor
/// (const seam `-D` OR a banked carrier). Used by the byte gate + its t24 controls.
pub fn native_blob_doctored(aeon: &Path, debug: bool, doctor: Option<(&str, i64)>) -> Vec<u8> {
    let specs = file_specs();
    let table = import_stub_table(aeon);
    let mut sections: Vec<Section> = Vec::new();
    for spec in &specs {
        let mut sec = lower_one(aeon, spec, debug, doctor, &table);
        let vma = if debug { spec.vma_debug } else { spec.vma_plain };
        sec.vma_base = Some(vma);
        sec.lma = blob_lma(debug) + vma;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
        sections.push(sec);
    }

    // The banked $8000-window symbols as equ carriers at harness-private LMAs.
    let carrier_pairs: Vec<(String, String)> = banked_carriers()
        .into_iter()
        .map(|(n, v)| {
            let v = match doctor {
                Some((dn, dv)) if dn == n => dv,
                _ => v,
            };
            (n.to_string(), format!("${v:X}"))
        })
        .collect();
    let carrier_refs: Vec<(&str, &str)> =
        carrier_pairs.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
    let mut carriers = crate::test_support::assemble_equ_pairs(&carrier_refs);
    for (i, sec) in carriers.iter_mut().enumerate() {
        sec.lma = 0x0100_0000 + (i as u32) * 0x1000;
        sec.placement = SectionPlacement::Pinned;
        sec.group = None;
    }
    sections.extend(carriers);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));

    let mut out = Vec::new();
    for spec in &specs {
        let bytes = linked
            .section(spec.section)
            .unwrap_or_else(|| panic!("linked image missing section {}", spec.section))
            .bytes
            .clone();
        out.extend(bytes);
    }
    out
}

/// Emit the seam-1 build inputs to `out_dir`: `z80_sound_blob.bin` (plain) and
/// `z80_sound_blob_debug.bin` (debug). Deterministic from the `.emp` sources + the
/// toolchain version. (The old `z80_sound_syms.asm` handler-VMA contract file is no
/// longer emitted — the banked seq table is native and resolves those VMAs in-link,
/// and no AS consumer of them survives; kill-list row 92.)
pub fn emit_sound_blob(aeon: &Path, out_dir: &Path) -> Result<(), String> {
    let out_dir: PathBuf = out_dir.to_path_buf();
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;

    let plain = native_sound_blob(aeon, false);
    let debug = native_sound_blob(aeon, true);
    if plain.bytes.len() != BLOB_LEN_PLAIN {
        return Err(format!("plain blob is {} bytes, expected {BLOB_LEN_PLAIN}", plain.bytes.len()));
    }
    if debug.bytes.len() != BLOB_LEN_DEBUG {
        return Err(format!("debug blob is {} bytes, expected {BLOB_LEN_DEBUG}", debug.bytes.len()));
    }

    let write = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let p = out_dir.join(name);
        std::fs::write(&p, bytes).map_err(|e| format!("write {}: {e}", p.display()))
    };
    write("z80_sound_blob.bin", &plain.bytes)?;
    write("z80_sound_blob_debug.bin", &debug.bytes)?;
    Ok(())
}

// ===========================================================================
// [call.clobbers-incomplete] — the transitive-clobbers-completeness diagnostic
// over the linked resident blob (seam-1 design §4 · the t37 demand).
// ===========================================================================

/// The report of `[call.clobbers-incomplete]` over the five linked resident sound
/// files: every proc's declared `clobbers` must be a SUPERSET of its transitive
/// effect (local writes ∪ reachable-callee clobbers − verified preserves). The
/// native link is the precondition that makes this computable — every callee body
/// is present in ONE module, so the reachable-callee union is finite and in-scope.
pub struct Z80ClobbersReport {
    /// Each `[call.clobbers-incomplete]` firing (an under-claimed proc/register).
    pub firings: Vec<sigil_frontend_emp::closure::Firing>,
    /// Direct callees named by some proc that resolve to no in-blob proc — the
    /// closure's holes (the OQ-4 scope check: a non-empty set past the local-label
    /// filter is a resident code-call into the banked/68k side, out of seam-1's
    /// closure). Local labels (`.`-prefixed) and hygiene symbols are filtered.
    pub unresolved_callees: std::collections::BTreeSet<String>,
    /// Instructions the per-proc eval DROPPED (unresolved mnemonic/operand). Must
    /// be 0 for the closure to be complete over the corpus.
    pub dropped: usize,
}

/// The opcode-dispatch SUB-MACHINE — the sequencer procs reached ONLY via the
/// `ex(sp),hl; ret` COMPUTED trampoline (`sound_sequencer.emp`'s
/// `[dispatch.trampoline: SeqOpcodeTable …]` site), threading `hl` as the stream
/// cursor. Their transitive clobbers depend on that un-traversable computed edge
/// (e.g. `Seq_Op_Patch` clobbers ix through it), so the direct-call closure CANNOT
/// soundly verify them — this is the design §4 face-4 / OQ-4 scope boundary. The set
/// is exactly the `Seq_Op_*` dispatch targets + the loop re-entry `Seq_ContinueFetch`
/// (`jp Seq_ContinueFetch` from a handler → `Sequencer_NextOpcode.fetch` → the
/// computed dispatch). It is DELIBERATELY the trampoline-only set: the `Seq_Hook*`
/// event helpers are straight-line `call/ret` (bounded, closure-verifiable) and ARE
/// checked in scope — an in-scope caller (`Sequencer_NextOpcode`) consumes them.
/// `[call.clobbers-incomplete]` reports the excluded set SEPARATELY; the external
/// entry `Sequencer_Channel` carries the honest broad clobbers the loop inflicts.
pub fn is_opcode_dispatch_proc(name: &str) -> bool {
    name.starts_with("Seq_Op_") || name == "Seq_ContinueFetch"
}

/// Run `[call.clobbers-incomplete]` over the resident blob for `debug`. The honest
/// corpus fires 0 IN SCOPE (the computed-dispatch sub-machine, [`is_opcode_dispatch_proc`],
/// is out of the sound direct-call closure and reported separately).
pub fn z80_clobbers_report(aeon: &Path, debug: bool) -> Z80ClobbersReport {
    z80_clobbers_report_doctored(aeon, debug, &[])
}

/// The IN-SCOPE firings of a report — every firing outside the computed-dispatch
/// sub-machine. The honest corpus's in-scope set is EMPTY; a non-empty one is a
/// real transitive clobbers under-claim to fix (or, for a doctored/RED run, the
/// injected one).
pub fn in_scope_firings(report: &Z80ClobbersReport) -> Vec<&sigil_frontend_emp::closure::Firing> {
    report.firings.iter().filter(|f| !is_opcode_dispatch_proc(&f.proc)).collect()
}

/// [`z80_clobbers_report`] with per-proc `clobbers` OVERRIDES (name → reglist
/// segments) — the RED fixture / t24 non-vacuity injection: doctoring a proc's
/// declared clobbers to UNDER-claim a register a callee destroys must fire.
pub fn z80_clobbers_report_doctored(
    aeon: &Path,
    debug: bool,
    doctor_clobbers: &[(&str, Vec<(String, Option<String>)>)],
) -> Z80ClobbersReport {
    use sigil_frontend_emp::closure::{check_firings, compute_closure, ProcNode};
    use sigil_frontend_emp::eval::eval_proc_body_env;
    use sigil_frontend_emp::regfile::{expand_reglist, RegFile};
    use sigil_frontend_emp::value::{CodeItem, CodeOperand};
    use sigil_frontend_emp::z80_preserves::z80_written_registers;
    use std::collections::{BTreeMap, BTreeSet};

    let expand = |segs: &[(String, Option<String>)]| expand_reglist(segs, RegFile::Z80, |_| {});

    // The Sym target of a Z80 transfer instruction (`call`/`rst`/`jp`/`jr`/`djnz`,
    // conditional or not) — the last symbolic operand (a leading `Cc` is skipped).
    // A `.`-prefixed local label or a hygiene (`$`) symbol is NOT an inter-proc
    // edge; a computed `jp (hl)`/indirect has no `Sym` and returns `None`.
    fn transfer_target(mnemonic: &str, ops: &[CodeOperand]) -> Option<String> {
        if !matches!(mnemonic, "call" | "rst" | "jp" | "jr" | "djnz") {
            return None;
        }
        ops.iter().rev().find_map(|op| match op {
            CodeOperand::Sym(name)
                if !name.starts_with('.') && !name.contains('$') =>
            {
                Some(name.clone())
            }
            _ => None,
        })
    }

    fn collect_nodes(
        file: &ast::File,
        items: &[ast::Item],
        defines: &[(String, i128)],
        inv_units: &BTreeSet<String>,
        expand: &impl Fn(&[(String, Option<String>)]) -> BTreeSet<String>,
        doctor: &[(&str, Vec<(String, Option<String>)>)],
        counter: &mut u32,
        dropped_total: &mut usize,
        nodes: &mut BTreeMap<String, ProcNode>,
    ) {
        for it in items {
            match it {
                ast::Item::Proc(p) => {
                    let (buf, _diags, next, dropped) = eval_proc_body_env(
                        file, &p.name, &p.params, &p.body, p.span, *counter,
                        sigil_ir::backend::Cpu::Z80, defines, &[],
                        &sigil_frontend_emp::contract::InterfaceEnv::empty(),
                    );
                    *counter = next;
                    *dropped_total += dropped;
                    let mut local_writes = BTreeSet::new();
                    let mut direct_callees = Vec::new();
                    if let Some(buf) = &buf {
                        local_writes = z80_written_registers(buf);
                        for ci in &buf.items {
                            if let CodeItem::Instr { mnemonic, ops, .. } = ci {
                                if let Some(t) = transfer_target(mnemonic, ops) {
                                    direct_callees.push(t);
                                }
                            }
                        }
                    }
                    // A `falls_into T` proc physically flows into T with no transfer
                    // instruction, so `transfer_target` cannot see the edge — model it
                    // directly (T's effect becomes this proc's, like a tail transfer).
                    if let Some(t) = &p.falls_into {
                        direct_callees.push(t.clone());
                    }
                    let doctored = doctor.iter().find(|(n, _)| *n == p.name);
                    let declared_clobbers = match doctored {
                        Some((_, segs)) => expand(segs),
                        None => expand(p.clobbers.as_deref().unwrap_or(&[])),
                    };
                    let mut verified_preserves = expand(&p.preserves);
                    verified_preserves.extend(inv_units.iter().cloned());
                    nodes.insert(
                        p.name.clone(),
                        ProcNode {
                            local_writes,
                            direct_callees,
                            indirect_sites: Vec::new(),
                            is_extern: false,
                            declared_clobbers,
                            params: BTreeSet::new(),
                            out: expand(p.out.as_deref().unwrap_or(&[])),
                            has_clobber_contract: p.clobbers.is_some() || doctored.is_some(),
                            verified_preserves,
                        },
                    );
                }
                ast::Item::Section(sec) => collect_nodes(
                    file, &sec.items, defines, inv_units, expand, doctor, counter,
                    dropped_total, nodes,
                ),
                _ => {}
            }
        }
    }

    let mut nodes: BTreeMap<String, ProcNode> = BTreeMap::new();
    let mut counter: u32 = 0;
    let mut dropped = 0usize;
    for spec in &file_specs() {
        let (file, _dir) = parse_one(aeon, spec);
        let inv_units = expand(&module_invariant_reglist(&file));
        let mut defines: Vec<(String, i128)> = resolve_consts(aeon, (spec.const_names)())
            .into_iter()
            .map(|(n, v)| (n.to_string(), v as i128))
            .collect();
        for (n, v) in banked_carriers() {
            defines.push((n.to_string(), v as i128));
        }
        defines.push(("DEBUG".to_string(), if debug { 1 } else { 0 }));
        collect_nodes(
            &file, &file.items, &defines, &inv_units, &expand, doctor_clobbers,
            &mut counter, &mut dropped, &mut nodes,
        );
    }

    let closure = compute_closure(&nodes, &BTreeMap::new());
    let firings = check_firings(&nodes, &closure);
    Z80ClobbersReport { firings, unresolved_callees: closure.unresolved_callees, dropped }
}

// ===========================================================================
// The per-module const seams — NAMES here, VALUES from the authority.
//
// E2: the 399 hand-maintained `(name, value)` entries are gone. Each module's
// `-D` env is the same set of NAMES it always folded; every value is resolved
// from `engine/sound/sound_constants.emp` (the sole authority) through the shared
// `eval_all_pub_consts` path — so a contract edit propagates to the resident blob
// with nothing to drift. The handful of names the authority does not own (game /
// data config) resolve from `seam_emit_config`, each carrying its provenance.
// ===========================================================================

/// Evaluate a module's `pub const`s to `(name, value)`, panicking on any parse or
/// resolve error (the blob is a hard build dependency). Drives the full evaluator,
/// so `offsetof`/`sizeof`/derivation RHSs fold exactly as in the real build.
fn eval_pub_consts(path: &Path, aeon: &Path, defines: &[(String, i128)]) -> Vec<(String, i64)> {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} parse errors: {pdiags:?}",
        path.display()
    );
    let (vals, diags) = sigil_frontend_emp::eval::eval_all_pub_consts(&file, Some(aeon), defines);
    assert!(
        diags.iter().all(|d| d.level != sigil_span::Level::Error),
        "{} pub-const resolve errors: {:?}",
        path.display(),
        diags.iter().filter(|d| d.level == sigil_span::Level::Error).collect::<Vec<_>>()
    );
    vals
}

/// The sound-constants authority: every `pub const` of `sound_constants.emp`,
/// resolved once and memoized per aeon root. This is the SINGLE source the five
/// resident-module seams read their contract values from (spec §9 one-authority
/// rule — a focused module-eval reuse, drift structurally impossible). `Z80_RAM`
/// (the one external base, feeding `SND_Z80_BASE`) is itself sourced from
/// `engine/system/constants.emp` — its own sole authority — and seeded, since the
/// standalone eval does not follow the `use engine.constants` from disk.
pub(crate) fn sound_authority_consts(aeon: &Path) -> std::sync::Arc<BTreeMap<String, i64>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<PathBuf, std::sync::Arc<BTreeMap<String, i64>>>>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    if let Some(m) = cache.lock().unwrap().get(aeon) {
        return m.clone();
    }
    let z80_ram = eval_pub_consts(&aeon.join("engine/system/constants.emp"), aeon, &[])
        .into_iter()
        .find(|(n, _)| n == "Z80_RAM")
        .map(|(_, v)| v)
        .expect("engine.constants must define Z80_RAM (the SND_Z80_BASE base)");
    let map: BTreeMap<String, i64> = eval_pub_consts(
        &aeon.join("engine/sound/sound_constants.emp"),
        aeon,
        &[("Z80_RAM".to_string(), z80_ram as i128)],
    )
    .into_iter()
    .collect();
    let arc = std::sync::Arc::new(map);
    cache.lock().unwrap().insert(aeon.to_path_buf(), arc.clone());
    arc
}

/// The SFX-bank id contract (`SFX_ID_BASE` / `SFX_COUNT` / `SFX_TABLE_LEN`),
/// resolved once and memoized per aeon root. Its AUTHORITY is `sfx_bank.emp` — the
/// module that owns the bank they count, where they are DERIVED from the `SfxTable`
/// rows (`SfxTable.min_key` / `.count` / `.len`), zero-byte and drift-proof from the
/// SFX set. Parcel F2 dissolved the hand-owned `config/sound_ids.asm` mirror + the
/// seam hardcodes into this single derived source (the sfx_bank sibling of
/// `sound_authority_consts`; a focused module-eval reuse). SHAPE-INVARIANT (the SFX
/// block content is identical plain/debug), so one eval serves both shapes.
pub(crate) fn sfx_bank_authority_consts(aeon: &Path) -> std::sync::Arc<BTreeMap<String, i64>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<PathBuf, std::sync::Arc<BTreeMap<String, i64>>>>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    if let Some(m) = cache.lock().unwrap().get(aeon) {
        return m.clone();
    }
    let map: BTreeMap<String, i64> =
        eval_pub_consts(&aeon.join("games/sonic4/data/sound/sfx/sfx_bank.emp"), aeon, &[])
            .into_iter()
            .collect();
    let arc = std::sync::Arc::new(map);
    cache.lock().unwrap().insert(aeon.to_path_buf(), arc.clone());
    arc
}

/// The seam const values NOT owned by `sound_constants.emp` — genuinely external
/// game / data config the resident-blob emit supplies, exactly as `main.asm` /
/// the generated vol-env data supply them in the mixed AS build. Each carries its
/// provenance. (Contrast the 384 contract values, which flow from the authority,
/// and the SFX-bank counts, which flow from `sfx_bank_authority_consts`.)
/// `DacSampleTable` is DERIVED from seam-2's single DAC head placement, not hand-pinned.
fn seam_emit_config() -> BTreeMap<&'static str, i64> {
    // The DAC descriptor head's $8000-window VMA (sound_bank.inc's driver
    // `-D DacSampleTable`), tied to seam-2's one DAC_SAMPLE_TAB placement so the
    // window address cannot drift from the bank it points at.
    let dac_sample_table = 0x8000
        + (crate::seam2::DAC_SAMPLE_TAB_LMA as i64 - crate::seam2::SOUND_TABLES_Z80_LMA as i64);
    BTreeMap::from([
        ("DacSampleTable", dac_sample_table),
        // Game config (games/sonic4/main.asm + config/sound_ids.asm + config/game.asm)
        // — the resident blob needs the game's bank/id layout to build; the AS side
        // gets these from main.asm. Not engine sound contract, so not in the authority.
        ("SND_ENGINE_TABLE_BANK", 0x0B), // main.asm: MovingTrucks_Bank_Start >> 15
        ("SFX_BLOB_BANK", 0x0B),         // main.asm: = SND_ENGINE_TABLE_BANK
        // SFX_ID_BASE / SFX_TABLE_LEN are NOT here (Parcel F2) — their authority is
        // sfx_bank.emp (SfxTable-derived), resolved via `sfx_bank_authority_consts`.
        ("SFXID_REV_LOOP", 0xAB),        // config/sound_ids.emp (= SFXID_SPINDASH)
        // Vol-env data config (engine/sound/sound_tables_z80.emp, GENERATED). The two
        // COUNTs have NO other definition in the tree (resident-blob-only). The six
        // control bytes mirror the generated data module — the residual 6-value seam
        // is gap-ledgered for a future harvest (make them pub there, or promote to the
        // authority and have the generator import — both touch the generator).
        ("FMVOLENV_COUNT", 3),
        ("PSGVOLENV_COUNT", 0x0B),
        ("FmVolEnvCtl_Loop", 0x80),
        ("FmVolEnvCtl_Sustain", 0x81),
        ("FmVolEnvCtl_Rest", 0x83),
        ("PsgVolEnvCtl_Loop", 0x80),
        ("PsgVolEnvCtl_Sustain", 0x81),
        ("PsgVolEnvCtl_Rest", 0x83),
    ])
}

/// Resolve a module's const NAMES to `(name, value)` — authority first, then the
/// emit-config fallback. A name in neither is a loud panic (a seam name with no
/// home is a build error, never a silent wrong byte).
fn resolve_consts(aeon: &Path, names: &[&'static str]) -> Vec<(&'static str, i64)> {
    let auth = sound_authority_consts(aeon);
    let sfx = sfx_bank_authority_consts(aeon);
    let cfg = seam_emit_config();
    names
        .iter()
        .map(|&n| {
            let v = auth
                .get(n)
                .copied()
                .or_else(|| sfx.get(n).copied())
                .or_else(|| cfg.get(n).copied())
                .unwrap_or_else(|| {
                    panic!(
                        "seam-1 const `{n}` is in none of the sound_constants.emp authority, \
                         the sfx_bank.emp authority, nor the emit-config list"
                    )
                });
            (n, v)
        })
        .collect()
}

/// The `z80_sound_driver.emp` const-name seam (values from the authority via `resolve_consts`).
fn driver_const_names() -> &'static [&'static str] {
    &[
        "CHROUTE_COUNT", "CHROUTE_DAC", "CHROUTE_FM6", "CHROUTE_PSG1",
        "DAC_SAMPLE_COUNT", "DacSampleTable", "DacSample_ds_length", "DacSample_ds_ptr",
        "SCF_ACTIVE", "SCF_IS_DAC", "SCF_IS_FM", "SCF_IS_PSG",
        "SCF_KEYED_B", "SHC_CMD_HI", "SHC_CMD_LO", "SHC_LEN",
        "SHC_MOD_HI", "SHC_MOD_LO", "SHC_ROUTE", "SH_CHANNELS",
        "SH_CHCOUNT", "SH_F_FM6_ADAPTIVE", "SH_PITCHTAB_HI", "SH_PITCHTAB_LO",
        "SH_TEMPO", "SH_TEMPO_MOD", "SND_ALIVE_MARKER", "SND_CTRL_DMA_ACTIVE",
        "SND_CUR_BANK", "SND_DAC_PHASE", "SND_ENGINE_TABLE_BANK", "SND_FADE_CMD_IN",
        "SND_FADE_DELAY", "SND_FADE_DELAY_CTR", "SND_FADE_DIRTY", "SND_FADE_SILENCE",
        "SND_FADE_TARGET", "SND_FM6_ADAPTIVE", "SND_FM6_CHAN_PTR", "SND_FM_KEYON_OPMASK",
        "SND_MASTER_FADE", "SND_MUSIC_PARAM_BANK", "SND_MUSIC_PARAM_FLAGS", "SND_MUSIC_PARAM_PATCHPTR",
        "SND_MUSIC_PARAM_PTR", "SND_MUSIC_STOP", "SND_REG_DAC_DATA", "SND_REG_DAC_ENABLE",
        "SND_REG_KEY_ONOFF", "SND_REG_LFO", "SND_REG_LR_AMS_FMS", "SND_REG_TIMER_A_HI",
        "SND_REG_TIMER_A_LO", "SND_REG_TIMER_CTRL", "SND_REQ_FADE", "SND_REQ_MUSIC",
        "SND_REQ_PING", "SND_REQ_SAMPLE", "SND_REQ_SFX", "SND_REQ_TEMPO",
        "SND_RING_BASE", "SND_RING_LEAD_PRIME", "SND_RING_LEAD_TARGET", "SND_RING_PAGE",
        "SND_RING_RD", "SND_RING_WR", "SND_ROM_BANK", "SND_ROM_LEN",
        "SND_ROM_PTR", "SND_SEQ_ACTIVE", "SND_SEQ_BADOP", "SND_SEQ_BASE",
        "SND_SEQ_CHANNELS", "SND_SEQ_CHCOUNT", "SND_SEQ_END", "SND_SEQ_PATCHTAB",
        "SND_SEQ_TEMPO", "SND_SEQ_TEMPO_MOD", "SND_SEQ_TRACE_WR", "SND_SFX_QUEUE_CNT",
        "SND_SONG_BANK", "SND_STACK_TOP", "SND_STAT_ACK_COUNT", "SND_STAT_ALIVE",
        "SND_STAT_DAC_ACTIVE", "SND_STAT_PING_ECHO", "SND_STAT_TICK", "SND_TEMPO_BASE",
        "SND_TEMPO_CUR", "SND_TEMPO_RESTORE", "SND_TEMPO_TARGET", "SND_TIMERA_CTRL_PROGRAM",
        "SND_TIMERA_CTRL_REARM", "SND_TIMERA_N", "SND_TIMERA_OVF_MASK", "SND_Z80_BANKREG",
        "SND_Z80_YM_A0", "SND_Z80_YM_A1", "SND_Z80_YM_A2", "SND_Z80_YM_A3",
        "SeqChannel_len", "Snd_PitchTabPtr", "Snd_SongBase", "Snd_SpindashRev",
        "sc_detune", "sc_dur_count", "sc_dur_default", "sc_flags",
        "sc_last_patch", "sc_macro_active", "sc_mod_ctrl", "sc_mod_ptr",
        "sc_noise_mode", "sc_note", "sc_porta_accum", "sc_porta_incr",
        "sc_psgenv", "sc_psgenv_cur", "sc_psgenv_out", "sc_pt_count",
        "sc_route", "sc_stream_ptr", "sc_tempo_accum", "sc_tempo_mod",
        "sc_volume",
    ]
}

/// The `sound_sequencer.emp` const-name seam (values from the authority via `resolve_consts`).
fn sequencer_const_names() -> &'static [&'static str] {
    &[
        "sc_stream_ptr", "sc_mod_ptr", "sc_dur_count", "sc_dur_default",
        "sc_patch", "sc_volume", "sc_note", "sc_flags",
        "sc_route", "sc_loop_ptr", "sc_repeat_ptr", "sc_repeat_count",
        "sc_tempo_mod", "sc_tempo_accum", "sc_pt_count", "sc_pt_cursor",
        "sc_points", "sc_transpose", "sc_pan", "sc_opbias",
        "sc_porta_accum", "sc_porta_incr", "sc_last_pan", "sc_fill_master",
        "sc_fill_count", "sc_psgenv", "sc_psgenv_cur", "sc_psgenv_out",
        "sc_env", "sc_env_cur", "sc_env_out", "sc_mod_ctrl",
        "sc_mod_wait", "sc_mod_speed", "sc_mod_delta", "sc_mod_steps",
        "sc_mod_speed_raw", "sc_mod_step_raw", "sc_mod_wait_raw", "sc_mod_delta_raw",
        "sc_mod_accum", "sc_base_freq", "sc_last_freq", "sc_noise_mode",
        "sc_detune", "sc_macro_active", "sx_gain", "sx_extend",
        "SeqChannel_len", "SCF_ACTIVE_B", "SCF_KEYED_B", "SCF_IS_FM_B",
        "SCF_IS_PSG_B", "SCF_REKEY_B", "SCF_SFX_OVERRIDE_B", "SCF_PITCH_CHROMATIC_B",
        "MEV_VOL", "MEV_REST", "MEV_NOTE_BASE", "PsgVolEnvCtl_Loop",
        "PsgVolEnvCtl_Sustain", "PsgVolEnvCtl_Rest", "FmVolEnvCtl_Loop", "FmVolEnvCtl_Sustain",
        "FmVolEnvCtl_Rest", "TAG_MAC_NEXT", "TAG_MAC_REG", "TAG_MAC_LOOP",
        "TAG_MAC_END", "FNUM_LO", "FNUM_HI", "SEQEV_NOTEON",
        "SEQEV_NOTEOFF", "SEQEV_VOL", "SEQEV_PATCH", "SEQEV_DAC",
        "SEQEV_LOOP", "SEQEV_JUMP", "SEQEV_END", "SEQEV_RPT_START",
        "SEQEV_RPT_END", "SND_FM_TL_MAX", "SND_PSG_SILENCE_T3", "CHROUTE_PSGN",
        "SND_REG_LFO", "SND_REG_TIMER_A_HI", "SND_REG_TIMER_CTRL", "SND_REG_KEY_ONOFF",
        "SND_REG_DAC_DATA", "SND_REG_DAC_ENABLE", "SND_FADE_DELAY", "SND_FADE_STEP",
        "SND_SEQ_TRACE_LEN", "SND_Z80_PSG", "SND_Z80_YM_A0", "SND_Z80_YM_A1",
        "SND_STAT_TICK", "SND_SEQ_ACTIVE", "SND_SEQ_CHCOUNT", "SND_SEQ_CHANNELS",
        "SND_SEQ_BADOP", "SND_SEQ_TRACE", "SND_SEQ_TRACE_WR", "SND_TEMPO_CUR",
        "SND_TEMPO_TARGET", "SND_TEMPO_BASE", "SND_MASTER_FADE", "SND_FADE_TARGET",
        "SND_FADE_DELAY_CTR", "SND_FADE_DIRTY", "Snd_SongBase", "Snd_SpindashRev",
    ]
}

/// The `sound_sfx.emp` const-name seam (values from the authority via `resolve_consts`).
fn sfx_const_names() -> &'static [&'static str] {
    &[
        "sc_stream_ptr", "sc_dur_count", "sc_dur_default", "sc_volume",
        "sc_note", "sc_flags", "sc_route", "sc_tempo_mod",
        "sc_tempo_accum", "sc_pt_count", "sc_last_pan", "sc_base_freq",
        "sc_noise_mode", "sx_priority", "sx_patch_base", "sx_saved_route",
        "sx_kind", "sx_gain", "sx_duck", "sx_extend",
        "SeqChannel_len", "SfxChannel_len", "SCF_ACTIVE_B", "SCF_KEYED_B",
        "SCF_IS_FM_B", "SCF_IS_PSG_B", "SCF_SFX_OVERRIDE_B", "SCF_PITCH_CHROMATIC",
        "SFXEL_NONE", "SFXEL_FM", "SFXEL_PSG", "SFXEL_NOISE",
        "SFXH_PRIORITY", "SFXH_FLAGS", "SFXH_CHCOUNT", "SFXH_GAIN",
        "SFXH_DUCK", "SFXH_CAP", "SFXH_CHANNELS", "SFXHC_ROUTE",
        "SFXHC_CMD_HI", "SFXHC_CMD_LO", "SFXHC_VOICE_HI", "SFXHC_VOICE_LO",
        "SFXHC_LEN", "SHF_CONTINUOUS_B", "SHF_CONTINUOUS", "CHROUTE_FM3",
        "CHROUTE_FM4", "CHROUTE_FM5", "CHROUTE_PSG1", "CHROUTE_PSG2",
        "CHROUTE_PSG3", "CHROUTE_PSGN", "CHROUTE_COUNT", "SFX_VOICE_COUNT",
        "SFX_DUCK_RAMP_STEP", "SFX_EXTEND_FRAMES", "SFX_QUEUE_DEPTH", "SFX_ID_BASE",
        "SFX_TABLE_LEN", "SFX_BLOB_BANK", "SFXID_REV_LOOP", "SND_SFX_CHANNELS",
        "SND_SFX_QUEUE", "SND_SFX_QUEUE_CNT", "SND_SFX_DUCK_LEVEL", "SND_SFX_DUCK_TARGET",
        "SND_REQ_BASE", "SND_SEQ_CHCOUNT", "SND_SEQ_ACTIVE", "SND_SEQ_CHANNELS",
        "Snd_SpindashRev", "SND_Z80_PSG", "SND_PSG_SILENCE_T3",
    ]
}

/// The `sound_fm.emp` const-name seam (values from the authority via `resolve_consts`).
fn fm_const_names() -> &'static [&'static str] {
    &[
        "sc_route", "sc_patch", "sc_pan", "sc_transpose",
        "sc_detune", "sc_base_freq", "sc_last_freq", "sc_porta_accum",
        "sc_porta_incr", "sc_opbias", "sc_flags", "sc_fill_master",
        "sc_fill_count", "sc_env_cur", "sc_env_out", "sx_gain",
        "sx_patch_base", "SND_REG_DAC_DATA", "SND_REG_ALG_FB", "SND_REG_LR_AMS_FMS",
        "SND_REG_OP_DT_MUL", "SND_REG_OP_TL", "SND_REG_OP_RS_AR", "SND_REG_OP_AM_D1R",
        "SND_REG_OP_D2R", "SND_REG_OP_D1L_RR", "SND_REG_OP_SSG_EG", "SND_REG_FNUM_HI",
        "SND_REG_FNUM_LO", "SND_REG_KEY_ONOFF", "SND_Z80_YM_A0", "SND_Z80_YM_A1",
        "SND_Z80_YM_A2", "SND_Z80_YM_A3", "SND_SFX_BASE", "SND_FM_TL_MAX",
        "SND_FM_KEYON_OPMASK", "CHROUTE_FM6", "FmPatch_len", "FmPatch_fp_tl",
        "SCF_KEYED_B", "REGDELTA_OP_MASK", "REGDELTA_GROUP_MASK", "REGDELTA_GROUP_COUNT",
        "REGDELTA_GROUP_SHIFT", "PITCHTAB_MAX_IDX", "PITCHTAB_COUNT", "FMPITCH_MAX_IDX",
        "FNUM_HI", "FNUM_LO", "SND_STAT_DAC_ACTIVE", "SND_MASTER_FADE",
        "SND_SFX_DUCK_LEVEL", "SND_SEQ_PATCHTAB", "Snd_PitchTabPtr", "SND_FM_SCRATCH",
        "SND_FM_SCRATCH_LEN",
    ]
}

/// The `sound_psg.emp` const-name seam (values from the authority via `resolve_consts`).
fn psg_const_names() -> &'static [&'static str] {
    &[
        "sc_route", "sc_flags", "sc_volume", "sc_psgenv_cur",
        "sc_psgenv_out", "sc_porta_accum", "sc_porta_incr", "sc_base_freq",
        "sc_last_freq", "sc_noise_mode", "sc_detune", "sx_gain",
        "CHROUTE_PSG1", "CHROUTE_PSGN", "SCF_KEYED_B", "SND_FM_TL_MAX",
        "SND_PSG_ATTEN_SILENT", "SND_Z80_PSG", "SND_PSG_VOL_LATCH", "SND_PSG_TONE_LATCH",
        "SND_PSG_SILENCE_N", "SND_PSG_SILENCE_T1", "SND_PSG_SILENCE_T2", "SND_PSG_SILENCE_T3",
        "SND_PSG_NOISE_VOL", "SND_PSG_NOISE_CTRL", "SND_MASTER_FADE", "SND_SFX_DUCK_LEVEL",
        "PSGVOLENV_COUNT", "FMVOLENV_COUNT",
    ]
}
