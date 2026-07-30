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

/// The exported-symbol CONTRACT: the sequencer opcode handlers the surviving
/// banked `seq_opcode_tab.asm` references via `dw <handler>` (26 distinct labels,
/// 36 table slots — several ops share a handler). This is the EXACT set the
/// generated `z80_sound_syms.asm` carries; a surviving AS consumer that references
/// any OTHER blob label fails LOUDLY at asl time (undefined symbol). All 26 live in
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
    consts: fn() -> Vec<(&'static str, i64)>,
}

fn file_specs() -> Vec<FileSpec> {
    vec![
        FileSpec {
            rel_path: "engine/sound/z80_sound_driver.emp",
            section: "z80_sound_driver",
            vma_plain: 0x0000,
            vma_debug: 0x0000,
            consts: driver_consts,
        },
        FileSpec {
            rel_path: "engine/sound/sound_sequencer.emp",
            section: "sound_sequencer",
            vma_plain: 0x0565,
            vma_debug: 0x0565,
            consts: sequencer_consts,
        },
        FileSpec {
            rel_path: "engine/sound/sound_sfx.emp",
            section: "sound_sfx",
            vma_plain: 0x0CD7,
            vma_debug: 0x0D55,
            consts: sfx_consts,
        },
        FileSpec {
            rel_path: "engine/sound/sound_fm.emp",
            section: "sound_fm",
            vma_plain: 0x12C3,
            vma_debug: 0x1341,
            consts: fm_consts,
        },
        FileSpec {
            rel_path: "engine/sound/sound_psg.emp",
            section: "sound_psg",
            vma_plain: 0x1660,
            vma_debug: 0x16DE,
            consts: psg_consts,
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
    let mut defines: Vec<(String, i128)> = (spec.consts)()
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

/// Render the generated `z80_sound_syms.asm` text: the exported-symbol contract as
/// per-shape `equ`s under `ifdef __DEBUG__`. GENERATED — never hand-edited.
pub fn render_syms_asm(aeon: &Path) -> String {
    let plain = handler_symbols(aeon, false);
    let debug = handler_symbols(aeon, true);
    let mut s = String::new();
    s.push_str("; z80_sound_syms.asm — GENERATED by `sigil emit_sound_blob`; DO NOT EDIT.\n");
    s.push_str("; The exported-symbol contract of the natively-linked resident sound blob:\n");
    s.push_str("; the sequencer opcode-handler VMAs the banked seq_opcode_tab.asm references.\n");
    s.push_str("; Per shape (the sequencer's `if DEBUG==1` growth re-bases handlers after it).\n");
    s.push_str("    ifdef __DEBUG__\n");
    for (name, addr) in &debug {
        s.push_str(&format!("{name} = ${addr:X}\n"));
    }
    s.push_str("    else\n");
    for (name, addr) in &plain {
        s.push_str(&format!("{name} = ${addr:X}\n"));
    }
    s.push_str("    endif\n");
    s
}

/// Emit the seam-1 build inputs to `out_dir`: `z80_sound_blob.bin` (plain),
/// `z80_sound_blob_debug.bin` (debug), and `z80_sound_syms.asm` (the contract).
/// Deterministic from the `.emp` sources + the toolchain version.
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

    let syms = render_syms_asm(aeon);
    let p = out_dir.join("z80_sound_syms.asm");
    std::fs::write(&p, syms).map_err(|e| format!("write {}: {e}", p.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// The five per-file const seams (the sound_constants.asm equs each file folds).
// ---------------------------------------------------------------------------

fn driver_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("CHROUTE_COUNT", 0xb), ("CHROUTE_DAC", 0xa), ("CHROUTE_FM6", 0x5),
        ("CHROUTE_PSG1", 0x6), ("DAC_SAMPLE_COUNT", 0xa), ("DacSampleTable", 0x85ad),
        ("DacSample_ds_length", 0x5), ("DacSample_ds_ptr", 0x3), ("SCF_ACTIVE", 0x1),
        ("SCF_IS_DAC", 0x10), ("SCF_IS_FM", 0x4), ("SCF_IS_PSG", 0x8), ("SCF_KEYED_B", 0x1),
        ("SHC_CMD_HI", 0x1), ("SHC_CMD_LO", 0x2), ("SHC_LEN", 0x5), ("SHC_MOD_HI", 0x3),
        ("SHC_MOD_LO", 0x4), ("SHC_ROUTE", 0x0), ("SH_CHANNELS", 0x6), ("SH_CHCOUNT", 0x3),
        ("SH_F_FM6_ADAPTIVE", 0x4), ("SH_PITCHTAB_HI", 0x4), ("SH_PITCHTAB_LO", 0x5),
        ("SH_TEMPO", 0x1), ("SH_TEMPO_MOD", 0x2), ("SND_ALIVE_MARKER", 0x5a),
        ("SND_CTRL_DMA_ACTIVE", 0x1f04), ("SND_CUR_BANK", 0x18f3), ("SND_DAC_PHASE", 0x18f0),
        ("SND_ENGINE_TABLE_BANK", 0xb), ("SND_FADE_CMD_IN", 0x2), ("SND_FADE_DELAY", 0x1),
        ("SND_FADE_DELAY_CTR", 0x1cce), ("SND_FADE_DIRTY", 0x1ccf), ("SND_FADE_SILENCE", 0x7f),
        ("SND_FADE_TARGET", 0x1ccd), ("SND_FM6_ADAPTIVE", 0x18fc),
        ("SND_FM6_CHAN_PTR", 0x18fa), ("SND_FM_KEYON_OPMASK", 0xf0),
        ("SND_MASTER_FADE", 0x1ccc), ("SND_MUSIC_PARAM_BANK", 0x1ca6),
        ("SND_MUSIC_PARAM_FLAGS", 0x1ca9), ("SND_MUSIC_PARAM_PATCHPTR", 0x1caa),
        ("SND_MUSIC_PARAM_PTR", 0x1ca7), ("SND_MUSIC_STOP", 0xff), ("SND_REG_DAC_DATA", 0x2a),
        ("SND_REG_DAC_ENABLE", 0x2b), ("SND_REG_KEY_ONOFF", 0x28), ("SND_REG_LFO", 0x22),
        ("SND_REG_LR_AMS_FMS", 0xb4), ("SND_REG_TIMER_A_HI", 0x24),
        ("SND_REG_TIMER_A_LO", 0x25), ("SND_REG_TIMER_CTRL", 0x27), ("SND_REQ_FADE", 0x1f05),
        ("SND_REQ_MUSIC", 0x1f02), ("SND_REQ_PING", 0x1f00), ("SND_REQ_SAMPLE", 0x1f01),
        ("SND_REQ_SFX", 0x1f03), ("SND_REQ_TEMPO", 0x1f06), ("SND_RING_BASE", 0x1900),
        ("SND_RING_LEAD_PRIME", 0x80), ("SND_RING_LEAD_TARGET", 0xc8), ("SND_RING_PAGE", 0x19),
        ("SND_RING_RD", 0x18f4), ("SND_RING_WR", 0x18f5), ("SND_ROM_BANK", 0x18f2),
        ("SND_ROM_LEN", 0x18f8), ("SND_ROM_PTR", 0x18f6), ("SND_SEQ_ACTIVE", 0x1a04),
        ("SND_SEQ_BADOP", 0x1a05), ("SND_SEQ_BASE", 0x1a00), ("SND_SEQ_CHANNELS", 0x1a08),
        ("SND_SEQ_CHCOUNT", 0x1a01), ("SND_SEQ_END", 0x1c9c), ("SND_SEQ_PATCHTAB", 0x1a02),
        ("SND_SEQ_TEMPO", 0x1a00), ("SND_SEQ_TEMPO_MOD", 0x1a07), ("SND_SEQ_TRACE_WR", 0x1a06),
        ("SND_SFX_QUEUE_CNT", 0x1ee4), ("SND_SONG_BANK", 0x18f1), ("SND_STACK_TOP", 0x1ffe),
        ("SND_STAT_ACK_COUNT", 0x1f12), ("SND_STAT_ALIVE", 0x1f10),
        ("SND_STAT_DAC_ACTIVE", 0x1f14), ("SND_STAT_PING_ECHO", 0x1f11),
        ("SND_STAT_TICK", 0x1f13), ("SND_TEMPO_BASE", 0x1cd2), ("SND_TEMPO_CUR", 0x1cd0),
        ("SND_TEMPO_RESTORE", 0xff), ("SND_TEMPO_TARGET", 0x1cd1),
        ("SND_TIMERA_CTRL_PROGRAM", 0x5), ("SND_TIMERA_CTRL_REARM", 0x15),
        ("SND_TIMERA_N", 0x89), ("SND_TIMERA_OVF_MASK", 0x1), ("SND_Z80_BANKREG", 0x6000),
        ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001), ("SND_Z80_YM_A2", 0x4002),
        ("SND_Z80_YM_A3", 0x4003), ("SeqChannel_len", 0x3c), ("Snd_PitchTabPtr", 0x1ca3),
        ("Snd_SongBase", 0x1ca1), ("Snd_SpindashRev", 0x1ca5), ("sc_detune", 0x3a),
        ("sc_dur_count", 0x4), ("sc_dur_default", 0x5), ("sc_flags", 0xa),
        ("sc_last_patch", 0x7), ("sc_macro_active", 0x3b), ("sc_mod_ctrl", 0x2a),
        ("sc_mod_ptr", 0x2), ("sc_noise_mode", 0x39), ("sc_note", 0x9),
        ("sc_porta_accum", 0x20), ("sc_porta_incr", 0x22), ("sc_psgenv", 0x27),
        ("sc_psgenv_cur", 0x28), ("sc_psgenv_out", 0x29), ("sc_pt_count", 0x13),
        ("sc_route", 0xb), ("sc_stream_ptr", 0x0), ("sc_tempo_accum", 0x12),
        ("sc_tempo_mod", 0x11), ("sc_volume", 0x8),
    ]
}

fn sequencer_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("sc_stream_ptr", 0x00), ("sc_mod_ptr", 0x02), ("sc_dur_count", 0x04),
        ("sc_dur_default", 0x05), ("sc_patch", 0x06), ("sc_volume", 0x08),
        ("sc_note", 0x09), ("sc_flags", 0x0A), ("sc_route", 0x0B),
        ("sc_loop_ptr", 0x0C), ("sc_repeat_ptr", 0x0E), ("sc_repeat_count", 0x10),
        ("sc_tempo_mod", 0x11), ("sc_tempo_accum", 0x12), ("sc_pt_count", 0x13),
        ("sc_pt_cursor", 0x14), ("sc_points", 0x15), ("sc_transpose", 0x1A),
        ("sc_pan", 0x1B), ("sc_opbias", 0x1C), ("sc_porta_accum", 0x20),
        ("sc_porta_incr", 0x22), ("sc_last_pan", 0x24), ("sc_fill_master", 0x25),
        ("sc_fill_count", 0x26), ("sc_psgenv", 0x27), ("sc_psgenv_cur", 0x28),
        ("sc_psgenv_out", 0x29), ("sc_env", 0x27), ("sc_env_cur", 0x28),
        ("sc_env_out", 0x29), ("sc_mod_ctrl", 0x2A), ("sc_mod_wait", 0x2B),
        ("sc_mod_speed", 0x2C), ("sc_mod_delta", 0x2D), ("sc_mod_steps", 0x2E),
        ("sc_mod_speed_raw", 0x2F), ("sc_mod_step_raw", 0x30), ("sc_mod_wait_raw", 0x31),
        ("sc_mod_delta_raw", 0x32), ("sc_mod_accum", 0x33), ("sc_base_freq", 0x35),
        ("sc_last_freq", 0x37), ("sc_noise_mode", 0x39), ("sc_detune", 0x3A),
        ("sc_macro_active", 0x3B), ("sx_gain", 0x40), ("sx_extend", 0x42),
        ("SeqChannel_len", 0x3C),
        ("SCF_ACTIVE_B", 0), ("SCF_KEYED_B", 1), ("SCF_IS_FM_B", 2), ("SCF_IS_PSG_B", 3),
        ("SCF_REKEY_B", 5), ("SCF_SFX_OVERRIDE_B", 6), ("SCF_PITCH_CHROMATIC_B", 7),
        ("MEV_VOL", 0xE0), ("MEV_REST", 0x80), ("MEV_NOTE_BASE", 0x81),
        ("PsgVolEnvCtl_Loop", 0x80), ("PsgVolEnvCtl_Sustain", 0x81), ("PsgVolEnvCtl_Rest", 0x83),
        ("FmVolEnvCtl_Loop", 0x80), ("FmVolEnvCtl_Sustain", 0x81), ("FmVolEnvCtl_Rest", 0x83),
        ("TAG_MAC_NEXT", 0xE0), ("TAG_MAC_REG", 0xE1), ("TAG_MAC_LOOP", 0xE2), ("TAG_MAC_END", 0xE3),
        ("FNUM_LO", 0x284), ("FNUM_HI", 0x508),
        ("SEQEV_NOTEON", 1), ("SEQEV_NOTEOFF", 2), ("SEQEV_VOL", 3), ("SEQEV_PATCH", 4),
        ("SEQEV_DAC", 5), ("SEQEV_LOOP", 6), ("SEQEV_JUMP", 7), ("SEQEV_END", 8),
        ("SEQEV_RPT_START", 9), ("SEQEV_RPT_END", 10),
        ("SND_FM_TL_MAX", 0x7F), ("SND_PSG_SILENCE_T3", 0xDF), ("CHROUTE_PSGN", 9),
        ("SND_REG_LFO", 0x22), ("SND_REG_TIMER_A_HI", 0x24), ("SND_REG_TIMER_CTRL", 0x27),
        ("SND_REG_KEY_ONOFF", 0x28), ("SND_REG_DAC_DATA", 0x2A), ("SND_REG_DAC_ENABLE", 0x2B),
        ("SND_FADE_DELAY", 1), ("SND_FADE_STEP", 2), ("SND_SEQ_TRACE_LEN", 0x20),
        ("SND_Z80_PSG", 0x7F11), ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001),
        ("SND_STAT_TICK", 0x1F13), ("SND_SEQ_ACTIVE", 0x1A04), ("SND_SEQ_CHCOUNT", 0x1A01),
        ("SND_SEQ_CHANNELS", 0x1A08), ("SND_SEQ_BADOP", 0x1A05), ("SND_SEQ_TRACE", 0x1CAC),
        ("SND_SEQ_TRACE_WR", 0x1A06), ("SND_TEMPO_CUR", 0x1CD0), ("SND_TEMPO_TARGET", 0x1CD1),
        ("SND_TEMPO_BASE", 0x1CD2), ("SND_MASTER_FADE", 0x1CCC), ("SND_FADE_TARGET", 0x1CCD),
        ("SND_FADE_DELAY_CTR", 0x1CCE), ("SND_FADE_DIRTY", 0x1CCF),
        ("Snd_SongBase", 0x1CA1), ("Snd_SpindashRev", 0x1CA5),
    ]
}

fn sfx_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("sc_stream_ptr", 0x00), ("sc_dur_count", 0x04), ("sc_dur_default", 0x05),
        ("sc_volume", 0x08), ("sc_note", 0x09), ("sc_flags", 0x0A), ("sc_route", 0x0B),
        ("sc_tempo_mod", 0x11), ("sc_tempo_accum", 0x12), ("sc_pt_count", 0x13),
        ("sc_last_pan", 0x24), ("sc_base_freq", 0x35), ("sc_noise_mode", 0x39),
        ("sx_priority", 0x39), ("sx_patch_base", 0x3B), ("sx_saved_route", 0x3D),
        ("sx_kind", 0x3F), ("sx_gain", 0x40), ("sx_duck", 0x41), ("sx_extend", 0x42),
        ("SeqChannel_len", 0x3C), ("SfxChannel_len", 0x44),
        ("SCF_ACTIVE_B", 0), ("SCF_KEYED_B", 1), ("SCF_IS_FM_B", 2), ("SCF_IS_PSG_B", 3),
        ("SCF_SFX_OVERRIDE_B", 6), ("SCF_PITCH_CHROMATIC", 0x80),
        ("SFXEL_NONE", 0), ("SFXEL_FM", 1), ("SFXEL_PSG", 2), ("SFXEL_NOISE", 3),
        ("SFXH_PRIORITY", 0), ("SFXH_FLAGS", 1), ("SFXH_CHCOUNT", 2), ("SFXH_GAIN", 3),
        ("SFXH_DUCK", 4), ("SFXH_CAP", 5), ("SFXH_CHANNELS", 8),
        ("SFXHC_ROUTE", 0), ("SFXHC_CMD_HI", 2), ("SFXHC_CMD_LO", 3),
        ("SFXHC_VOICE_HI", 4), ("SFXHC_VOICE_LO", 5), ("SFXHC_LEN", 6),
        ("SHF_CONTINUOUS_B", 0), ("SHF_CONTINUOUS", 1),
        ("CHROUTE_FM3", 2), ("CHROUTE_FM4", 3), ("CHROUTE_FM5", 4), ("CHROUTE_PSG1", 6),
        ("CHROUTE_PSG2", 7), ("CHROUTE_PSG3", 8), ("CHROUTE_PSGN", 9), ("CHROUTE_COUNT", 0x0B),
        ("SFX_VOICE_COUNT", 7), ("SFX_DUCK_RAMP_STEP", 4), ("SFX_EXTEND_FRAMES", 0x0A),
        ("SFX_QUEUE_DEPTH", 3), ("SFX_ID_BASE", 0x33), ("SFX_TABLE_LEN", 0x87),
        ("SFX_BLOB_BANK", 0x0B), ("SFXID_REV_LOOP", 0xAB),
        ("SND_SFX_CHANNELS", 0x1D00), ("SND_SFX_QUEUE", 0x1EDC), ("SND_SFX_QUEUE_CNT", 0x1EE4),
        ("SND_SFX_DUCK_LEVEL", 0x1EE5), ("SND_SFX_DUCK_TARGET", 0x1EE6), ("SND_REQ_BASE", 0x1F00),
        ("SND_SEQ_CHCOUNT", 0x1A01), ("SND_SEQ_ACTIVE", 0x1A04), ("SND_SEQ_CHANNELS", 0x1A08),
        ("Snd_SpindashRev", 0x1CA5), ("SND_Z80_PSG", 0x7F11), ("SND_PSG_SILENCE_T3", 0xDF),
    ]
}

fn fm_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("sc_route", 0x0B), ("sc_patch", 0x06), ("sc_pan", 0x1B), ("sc_transpose", 0x1A),
        ("sc_detune", 0x3A), ("sc_base_freq", 0x35), ("sc_last_freq", 0x37),
        ("sc_porta_accum", 0x20), ("sc_porta_incr", 0x22), ("sc_opbias", 0x1C),
        ("sc_flags", 0x0A), ("sc_fill_master", 0x25), ("sc_fill_count", 0x26),
        ("sc_env_cur", 0x28), ("sc_env_out", 0x29), ("sx_gain", 0x40), ("sx_patch_base", 0x3B),
        ("SND_REG_DAC_DATA", 0x2A), ("SND_REG_ALG_FB", 0xB0), ("SND_REG_LR_AMS_FMS", 0xB4),
        ("SND_REG_OP_DT_MUL", 0x30), ("SND_REG_OP_TL", 0x40), ("SND_REG_OP_RS_AR", 0x50),
        ("SND_REG_OP_AM_D1R", 0x60), ("SND_REG_OP_D2R", 0x70), ("SND_REG_OP_D1L_RR", 0x80),
        ("SND_REG_OP_SSG_EG", 0x90), ("SND_REG_FNUM_HI", 0xA4), ("SND_REG_FNUM_LO", 0xA0),
        ("SND_REG_KEY_ONOFF", 0x28),
        ("SND_Z80_YM_A0", 0x4000), ("SND_Z80_YM_A1", 0x4001),
        ("SND_Z80_YM_A2", 0x4002), ("SND_Z80_YM_A3", 0x4003),
        ("SND_SFX_BASE", 0x1D00), ("SND_FM_TL_MAX", 0x7F), ("SND_FM_KEYON_OPMASK", 0xF0),
        ("CHROUTE_FM6", 5), ("FmPatch_len", 0x20), ("FmPatch_fp_tl", 6),
        ("SCF_KEYED_B", 1),
        ("REGDELTA_OP_MASK", 3), ("REGDELTA_GROUP_MASK", 0x0F),
        ("REGDELTA_GROUP_COUNT", 6), ("REGDELTA_GROUP_SHIFT", 2),
        ("PITCHTAB_MAX_IDX", 0x83), ("PITCHTAB_COUNT", 0x84), ("FMPITCH_MAX_IDX", 0x5E),
        ("FNUM_HI", 0x508), ("FNUM_LO", 0x284),
        ("SND_STAT_DAC_ACTIVE", 0x1F14), ("SND_MASTER_FADE", 0x1CCC),
        ("SND_SFX_DUCK_LEVEL", 0x1EE5), ("SND_SEQ_PATCHTAB", 0x1A02),
        ("Snd_PitchTabPtr", 0x1CA3),
        ("SND_FM_SCRATCH", 0x1C9C), ("SND_FM_SCRATCH_LEN", 5),
    ]
}

fn psg_consts() -> Vec<(&'static str, i64)> {
    vec![
        ("sc_route", 0x0B), ("sc_flags", 0x0A), ("sc_volume", 0x08),
        ("sc_psgenv_cur", 0x28), ("sc_psgenv_out", 0x29), ("sc_porta_accum", 0x20),
        ("sc_porta_incr", 0x22), ("sc_base_freq", 0x35), ("sc_last_freq", 0x37),
        ("sc_noise_mode", 0x39), ("sc_detune", 0x3A), ("sx_gain", 0x40),
        ("CHROUTE_PSG1", 6), ("CHROUTE_PSGN", 9), ("SCF_KEYED_B", 1),
        ("SND_FM_TL_MAX", 0x7F), ("SND_PSG_ATTEN_SILENT", 0x0F), ("SND_Z80_PSG", 0x7F11),
        ("SND_PSG_VOL_LATCH", 0x90), ("SND_PSG_TONE_LATCH", 0x80), ("SND_PSG_SILENCE_N", 0xFF),
        ("SND_PSG_SILENCE_T1", 0x9F), ("SND_PSG_SILENCE_T2", 0xBF), ("SND_PSG_SILENCE_T3", 0xDF),
        ("SND_PSG_NOISE_VOL", 0xF0), ("SND_PSG_NOISE_CTRL", 0xE0),
        ("SND_MASTER_FADE", 0x1CCC), ("SND_SFX_DUCK_LEVEL", 0x1EE5),
        ("PSGVOLENV_COUNT", 0x0B), ("FMVOLENV_COUNT", 3),
    ]
}
