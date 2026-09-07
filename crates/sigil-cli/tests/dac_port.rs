//! Sound-migration T1: the REAL `dac_samples.emp` port, with DERIVED expectations.
//!
//! Compiles the ACTUAL ported file from aeon's tree,
//! `games/sonic4/data/sound/dac_samples.emp`, through the production
//! parse -> lower -> place -> resolve -> link pipeline, with `include_root` set to
//! the module's OWN directory (so `embed("dac/kick.pcm")` and
//! `embed("temp_blip.bin")` resolve), and the two `bank:` sections placed into
//! `--map` regions BY SECTION NAME at the LMAs `seam2::sound_layout` derives from
//! the live `games/sonic4/map.toml`.
//!
//! It pins two facts, and NEITHER expectation is typed here:
//!
//!   (a) PAYLOAD BYTES: each bank section's linked bytes are exactly the blobs the
//!       module declares for it, concatenated in declaration order, and nothing
//!       else.
//!   (b) EVERY `SND_*` equ: each `_BANK` / `_PTR` / `_LEN` folds to the value its
//!       own right-hand side names, `bankid(L)` / `winptr(L)` of the label's placed
//!       address and `B.len` of the blob's file length.
//!
//! ## Two sources, and which is authoritative where
//!
//! The expectations come from two sources that CAN disagree, so the test says
//! which one wins:
//!
//!   * the EMITTED ARTIFACT (the linked section image and its LMA) is authoritative
//!     for VALUES. Where a blob's bytes sit is found by searching the image for
//!     those bytes, and the pointer expectation is computed from that position.
//!     Aeon's LS-7 compared its DAC descriptors from the emitted artifact for the
//!     same reason: the source and the artifact are two things, and the artifact is
//!     what ships.
//!   * the SOURCE (`dac_samples.emp`) is authoritative for ORDER and NAMES only:
//!     which `data` lines a section holds and in what order, which blob const each
//!     binds, which `.pcm` file that const embeds, and which label or blob each
//!     `SND_*` equ names.
//!
//! The two are asserted to AGREE: the position at which a blob's bytes are found
//! must equal the position the declaration order predicts (the running sum of the
//! preceding blobs' lengths), and the image must end where the last declared blob
//! ends. A disagreement fails naming the blob; it is never resolved by preference.
//!
//! Every mismatch is COLLECTED and reported together, so a change that shifts
//! every pointer after one blob reports every affected row rather than the first.
//!
//! The drum list is read from the module, not typed here: aeon LS-7 (cbc023ff)
//! deleted two byte-identical `.pcm` files (DAC ids 5 and 6 alias ids 2 and 3
//! through `winptr`), and a typed nine-file list stopped describing the bank at
//! that commit while this gate's assert stopped at its first row.
//!
//! ## Falsification
//!
//! Perturbing the derivation's INPUT, the section LMA the pointer expectations are
//! computed from (`sec.lma + 2` in place of `sec.lma` in [`derive_layout`]), turns
//! every `SND_*_PTR` row red by name (`SND_KICK_PTR: expected 0x8002, got 0x8000`,
//! and so on down the bank) while the `_BANK` and `_LEN` rows stay green. The
//! fold under test is sigil's link-time `bankid()` / `winptr()` against the placed
//! address; the derivation is independent arithmetic over the artifact.

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_harness::seam2::sound_layout;
use sigil_harness::test_support::{parse_dac_declarations, reference_tree, DacDataLine, DacDeclarations};
use sigil_ir::backend::Cpu;
use sigil_ir::{Expr, Section, SymbolTable};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The aeon root the sound dir sits under (`games/sonic4/data/sound` is four levels
/// down), which is what `sound_layout` reads the map from.
fn aeon_root(sound_dir: &Path) -> PathBuf {
    sound_dir.ancestors().nth(4).expect("games/sonic4/data/sound has an aeon root").to_path_buf()
}

/// `bankid()`'s fold, spelled once: `(lma & $7F8000) >> 15`.
fn bankid(lma: u32) -> i64 {
    ((lma & 0x7F_8000) >> 15) as i64
}

/// `winptr()`'s fold, spelled once: `(lma & $7FFF) | $8000`.
fn winptr(lma: u32) -> i64 {
    ((lma & 0x7FFF) | 0x8000) as i64
}

/// REFERENCE-DEPENDENT: the sources live in the sibling aeon tree. Absent, both
/// tests SKIP green, unless `SIGIL_STRICT_GATE=1`, which makes absence a failure.
fn sound_dir() -> Option<PathBuf> {
    reference_tree(&[
        "games/sonic4/data/sound/dac_samples.emp",
        "games/sonic4/data/sound/temp_blip.bin",
    ])
    .map(|aeon| aeon.join("games/sonic4/data/sound"))
}

/// The two-bank map, at the bank LMAs the live `games/sonic4/map.toml` derives
/// (`seam2::sound_layout`: the `dac_banks` anchor plus one window, so a re-layout
/// moves this port with the game). Sections match regions BY NAME; the top-level
/// `equ`/`ensure` items land in the default `text` section, which needs its own
/// region (it emits ZERO bytes here, every SND_* is an equ, but `place_sections`
/// still requires a home for it). Region sizes are the $8000 window per bank;
/// `text` is nominal.
fn map_toml(aeon: &Path) -> String {
    let l = sound_layout(aeon).expect("sound_layout derives the DAC bank LMAs from map.toml");
    format!(
        "fill = 0x00\n\
         \n\
         [[region]]\n\
         name = \"text\"\n\
         lma_base = 0x0000\n\
         size = 0x10\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"dac_blip_bank\"\n\
         lma_base = 0x{:X}\n\
         size = 0x8000\n\
         kind = \"rom\"\n\
         \n\
         [[region]]\n\
         name = \"dac_shared_bank\"\n\
         lma_base = 0x{:X}\n\
         size = 0x8000\n\
         kind = \"rom\"\n",
        l.dac_blip_lma, l.dac_shared_lma
    )
}

/// Parse -> lower (with the sound-dir include-root) -> place into the map ->
/// resolve -> link. Returns the module's source text (the ORDER/NAMES source), the
/// placed+resolved sections (equ exprs folded to `Expr::Int`) and the linked image
/// (the VALUES source), asserting a clean pipeline at each stage.
fn compile_real_file(dir: &Path) -> (String, Vec<Section>, sigil_link::LinkedImage) {
    let emp_path = dir.join("dac_samples.emp");
    let src = std::fs::read_to_string(&emp_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", emp_path.display()));

    let (file, pdiags) = parse_str(&src);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "parse errors: {pdiags:?}"
    );

    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        // The module's OWN directory, so `embed("dac/kick.pcm")` and
        // `embed("temp_blip.bin")` resolve within the capability sandbox.
        include_root: Some(dir.to_path_buf()),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "lower errors (embed/ensure): {ldiags:?}"
    );

    let map = sigil_link::load_map(&map_toml(&aeon_root(dir))).expect("map must load");
    let mut sections = module.sections;
    let pdiags = place_sections(&mut sections, &map);
    assert!(
        pdiags.iter().all(|d| d.level != sigil_span::Level::Error),
        "place_sections errors (region-per-section): {pdiags:?}"
    );

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout failed (bank straddle / ensure?): {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link failed: {d:?}"));
    (src, resolved, linked)
}

/// Read a folded equ value from the resolved sections. After `resolve_layout`,
/// every equ's `expr` is `Expr::Int(v)` (the `bankid()`/`winptr()`/`.len` fold
/// against the FINAL placed addresses), so this is a direct read; a non-`Int`
/// expr (or a missing name) is a hard failure, not a silent skip.
fn equ_value(sections: &[Section], name: &str) -> i64 {
    for sec in sections {
        for eq in &sec.equ_syms {
            if eq.name == name {
                match &eq.expr {
                    Expr::Int(v) => return *v,
                    other => panic!("equ `{name}` did not fold to Int post-resolve: {other:?}"),
                }
            }
        }
    }
    panic!("equ `{name}` not found in any resolved section");
}

/// Every equ name the resolved sections carry that starts with `SND_`, so the
/// derivation can prove it covered them all rather than the ones its parser knew.
fn snd_equ_names(sections: &[Section]) -> Vec<String> {
    let mut names: Vec<String> = sections
        .iter()
        .flat_map(|s| s.equ_syms.iter().map(|e| e.name.clone()))
        .filter(|n| n.starts_with("SND_"))
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------------------
// The SOURCE half: order and names, read out of `dac_samples.emp` by
// `test_support::parse_dac_declarations` (section 7), shared with the seam-2
// bank-emit gate so the module is read one way.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// The ARTIFACT half: values, read off the linked image, checked against the order.
// ---------------------------------------------------------------------------

/// Where every declared label sits and how long every declared blob is, derived
/// from the linked image (values) and the declaration order (names), plus every
/// place the two disagree.
struct Layout {
    /// label -> placed address (section LMA + the offset its bytes were FOUND at).
    label_addr: BTreeMap<String, u32>,
    /// blob const -> length of the file it embeds.
    blob_len: BTreeMap<String, usize>,
    /// Every disagreement between the image and the declaration order, by name.
    faults: Vec<String>,
}

/// The first offset at or after `from` where `needle` occurs in `hay`.
fn find_at_or_after(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from + needle.len() > hay.len() {
        return None;
    }
    hay[from..].windows(needle.len()).position(|w| w == needle).map(|p| p + from)
}

fn derive_layout(dir: &Path, declared: &DacDeclarations, linked: &sigil_link::LinkedImage) -> Layout {
    let mut out = Layout { label_addr: BTreeMap::new(), blob_len: BTreeMap::new(), faults: Vec::new() };
    for (sec_name, lines) in &declared.sections {
        let sec = linked
            .section(sec_name)
            .unwrap_or_else(|| panic!("linked image must carry `{sec_name}` (the module declares it)"));
        // The pointer derivation's INPUT: the LMA the placer gave this section.
        let base = sec.lma;
        let mut cursor = 0usize;
        for DacDataLine { label, blob } in lines {
            let path = declared
                .embeds
                .get(blob)
                .unwrap_or_else(|| panic!("`{sec_name}`: data `{label}` binds `{blob}`, which no `const ... = embed(...)` declares"));
            let file = std::fs::read(dir.join(path))
                .unwrap_or_else(|e| panic!("read {path} (blob `{blob}`, label `{label}`): {e}"));
            assert!(!file.is_empty(), "{path} is empty, its position in the image is unmeasurable");
            // VALUES from the artifact: where these bytes actually are.
            let pos = match find_at_or_after(&sec.bytes, &file, cursor) {
                Some(p) if p == cursor => p,
                Some(p) => {
                    out.faults.push(format!(
                        "`{sec_name}`: `{label}` ({path}, {} B) found at image offset {p:#x}, but the \
                         declaration order predicts {cursor:#x}",
                        file.len()
                    ));
                    p
                }
                None => {
                    out.faults.push(format!(
                        "`{sec_name}`: `{label}` ({path}, {} B) is not in the image at or after offset \
                         {cursor:#x} (image is {} B)",
                        file.len(),
                        sec.bytes.len()
                    ));
                    cursor
                }
            };
            out.label_addr.insert(label.clone(), base + pos as u32);
            out.blob_len.insert(blob.clone(), file.len());
            cursor = pos + file.len();
        }
        if cursor != sec.bytes.len() {
            out.faults.push(format!(
                "`{sec_name}`: the declared blobs end at {cursor:#x} but the image is {:#x} B, the \
                 section holds bytes no `data` line declares",
                sec.bytes.len()
            ));
        }
    }
    out
}

/// (a) PAYLOAD BYTES: each bank section's linked bytes are exactly its declared
/// blobs, in declaration order, with nothing before, between or after them. This
/// is the agreement assertion between the two sources, run on its own so a payload
/// fault reads as one.
#[test]
fn dac_bank_payloads_match_declared_blobs() {
    let Some(dir) = sound_dir() else { return };
    let (src, _resolved, linked) = compile_real_file(&dir);
    let declared = parse_dac_declarations(&src);
    let layout = derive_layout(&dir, &declared, &linked);
    assert!(
        layout.faults.is_empty(),
        "{} payload disagreement(s) between dac_samples.emp and the linked image:\n  {}",
        layout.faults.len(),
        layout.faults.join("\n  ")
    );
    // The bank shapes the sound driver is built around: one blip alone in its
    // window, several drums sharing one window. Fewer than that and the fold is
    // not being exercised across a bank.
    let blip = &declared.sections["dac_blip_bank"];
    let shared = &declared.sections["dac_shared_bank"];
    assert_eq!(blip.len(), 1, "dac_blip_bank must hold exactly one blob: {blip:?}");
    assert!(shared.len() >= 2, "dac_shared_bank must hold at least two drums: {shared:?}");
}

/// (b) EVERY `SND_*` equ folds to what its own right-hand side names, against the
/// addresses the artifact shows and the lengths the files have. Every row is
/// checked and every mismatch reported; the set of equs checked is proven to be
/// the set the resolved module carries.
#[test]
fn snd_equ_values_match_derived_layout() {
    let Some(dir) = sound_dir() else { return };
    let (src, resolved, linked) = compile_real_file(&dir);
    let declared = parse_dac_declarations(&src);
    let layout = derive_layout(&dir, &declared, &linked);
    assert!(
        layout.faults.is_empty(),
        "the value source disagrees with the order source, so no pointer expectation is \
         trustworthy:\n  {}",
        layout.faults.join("\n  ")
    );
    let v = |name: &str| equ_value(&resolved, name);

    // The blip and drum banks must differ or the bankid fold is untestable.
    let l = sound_layout(&aeon_root(&dir)).expect("sound_layout");
    assert_ne!(bankid(l.dac_blip_lma), bankid(l.dac_shared_lma), "the blip and drum banks must differ");

    let addr_of = |label: &str| {
        *layout
            .label_addr
            .get(label)
            .unwrap_or_else(|| panic!("`{label}` is named by an SND_* equ but is not a `data` label of any section"))
    };
    let len_of = |blob: &str| {
        *layout
            .blob_len
            .get(blob)
            .unwrap_or_else(|| panic!("`{blob}` is named by an SND_*_LEN equ but no `data` line places it"))
    };

    let mut mismatches = Vec::new();
    let mut covered = Vec::new();
    for (base, t) in &declared.equs {
        let rows = [
            (format!("SND_{base}_BANK"), bankid(addr_of(t.bank_of.as_ref().unwrap()))),
            (format!("SND_{base}_PTR"), winptr(addr_of(t.ptr_of.as_ref().unwrap()))),
            (format!("SND_{base}_LEN"), len_of(t.len_of.as_ref().unwrap()) as i64),
        ];
        for (name, expected) in rows {
            let got = v(&name);
            if got != expected {
                mismatches.push(format!("{name}: expected {expected:#X}, got {got:#X}"));
            }
            covered.push(name);
        }
    }
    covered.sort();
    assert_eq!(
        covered,
        snd_equ_names(&resolved),
        "the SND_* equs the derivation checked are not the SND_* equs the module resolved"
    );
    assert!(
        mismatches.is_empty(),
        "{} of {} SND_* row(s) disagree with the derived layout:\n  {}",
        mismatches.len(),
        covered.len(),
        mismatches.join("\n  ")
    );
}
