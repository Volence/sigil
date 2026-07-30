//! seam-2 stage-2b OPTION Y — the CO-LINK PROBE (TDD, empirical).
//!
//! The design §2d claim: co-link `dac_sample_tab.emp` with `dac_samples.emp` and
//! the descriptor cells fold DIRECTLY from `bankid(Dac_Kick)`/`winptr(Dac_Kick)`/
//! the length — the 30 `SND_*` `-D` defines VANISH. The recon flagged three cells
//! whose cross-module resolution was UNPROVEN in the corpus:
//!   * `bankid(L)` / `winptr(L)` on a CROSS-MODULE label in a DATA cell (proven
//!     in the corpus only inside `ensure(...)` link-asserts, never emitted).
//!   * the LENGTH cell — `.len` is a COMPTIME property of a same-module `Value::Data`
//!     binding, so `Dac_Kick.len` cannot resolve in a module that does not own the
//!     blob. The recon's fallback: reference the `SND_*_LEN` equ (which folds
//!     same-module in `dac_samples.emp`) as a cross-module LINK symbol.
//!
//! This probe co-links the REAL `dac_samples.emp` (placed at the current baseline
//! $48000/$50000) with a SYNTHETIC one-descriptor head module and empirically
//! settles each mechanism BEFORE any `.asm` deletion. Every assert records a fact.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test seam2_colink_probe
//! ```

use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_frontend_emp::resolve::place_sections;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::Level;
use std::path::PathBuf;

fn aeon_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("AEON_DIR").unwrap_or_else(|_| "/home/volence/sonic_hacks/aeon".to_string()),
    )
}
fn strict_gate() -> bool {
    std::env::var("SIGIL_STRICT_GATE").is_ok()
}

/// Lower the REAL `dac_samples.emp` and return its sections (the two `bank:`
/// payloads + the module carrier holding the 30 `SND_*` `equ_syms`) — NOT yet
/// placed. The caller places all sections together with the probe head.
fn dac_samples_sections() -> Vec<Section> {
    let aeon = aeon_dir();
    let dir = aeon.join("games/sonic4/data/sound");
    let src = std::fs::read_to_string(dir.join("dac_samples.emp"))
        .unwrap_or_else(|e| panic!("read dac_samples.emp: {e}"));
    let (file, pdiags) = parse_str(&src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "dac_samples.emp parse: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: Some(dir),
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != Level::Error),
        "dac_samples.emp lower: {:?}",
        ldiags.iter().filter(|d| d.level == Level::Error).collect::<Vec<_>>()
    );
    module.sections
}

/// Lower a synthetic head module (source given) at `(cpu: z80, vma: $8000)`.
/// Returns the head section + any lower diagnostics (so the `.len` crux probe can
/// assert the error). The head is placed by the caller into a `head` region.
fn lower_head(src: &str) -> (Option<Section>, Vec<sigil_span::Diagnostic>) {
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "probe head parse: {pdiags:?}");
    let opts = LowerOptions {
        initial_cpu: Cpu::M68000,
        include_root: None,
        embed_base: None,
        defines: vec![],
    };
    let (module, ldiags) = lower_module(&file, &opts);
    let sec = module.sections.into_iter().find(|s| s.name == "probe_head");
    (sec, ldiags)
}

/// The current-baseline co-link map: dac banks at $48000/$50000 (as `emit_dac_banks`),
/// the module carrier in `text`, the probe head phased into the song bank at $5856D.
const MAP: &str = "\
fill = 0x00

[[region]]
name = \"text\"
lma_base = 0x0000
size = 0x10
kind = \"rom\"

[[region]]
name = \"dac_blip_bank\"
lma_base = 0x48000
size = 0x8000
kind = \"rom\"

[[region]]
name = \"dac_shared_bank\"
lma_base = 0x50000
size = 0x8000
kind = \"rom\"

[[region]]
name = \"probe_head\"
lma_base = 0x5856D
size = 0x100
kind = \"rom\"
";

/// Co-link `dac_samples.emp` + the probe head; return the linked probe-head bytes.
fn colink(head_src: &str) -> Result<Vec<u8>, Vec<sigil_span::Diagnostic>> {
    let mut sections = dac_samples_sections();
    let (head, ldiags) = lower_head(head_src);
    if ldiags.iter().any(|d| d.level == Level::Error) {
        return Err(ldiags);
    }
    sections.push(head.expect("probe head section"));
    let map = sigil_link::load_map(MAP).expect("map loads");
    let pd = place_sections(&mut sections, &map);
    assert!(pd.iter().all(|d| d.level != Level::Error), "place_sections: {pd:?}");
    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)?;
    let linked = sigil_link::link(&resolved, &SymbolTable::new())?;
    Ok(linked.section("probe_head").expect("linked probe_head").bytes.clone())
}

/// THE MECHANISM PROBE — reference the `SND_*` cross-module as CO-LINKED EQUS, the
/// EXACT cells `dac_sample_tab.emp` already writes (`dc.b SND_KICK_BANK` / `dc.w
/// SND_KICK_PTR` / `dc.w SND_KICK_LEN`). The `SND_*` equs fold SAME-MODULE in
/// `dac_samples.emp` (`bankid`/`winptr`/`.len` — placement + comptime), so the head
/// only needs to RESOLVE them cross-module. This is the Option-Y route that leaves
/// the descriptor body byte-identical and drops the `-D` entirely.
///
/// The crux: `SND_KICK_BANK` is a width-1 `dc.b` cell whose equ is a LinkExpr
/// (`bankid(Dac_Kick)`, folded at link to a BYTE-sized value $0A). The row-1623
/// wall is about a 2-byte POINTER in a 1-byte cell; a link-folded small int is a
/// different thing — this probe settles whether `dc.b <link-folded-equ>` is accepted.
///
/// Expected (current baseline): Dac_Kick @ $50000 → BANK $0A, PTR $8000, LEN 1406
/// ($057E). Dac_Temp_Blip @ $48000 → BANK $09, PTR $8000, LEN 2880 ($0B40).
#[test]
fn colink_snd_equs_resolve_cross_module_in_dc_cells() {
    if !strict_gate() {
        eprintln!("skip seam2_colink_probe (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    // The EXACT cell forms dac_sample_tab.emp uses — bare SND_* names in dc cells.
    let head = "\
module engine.probe_head (cpu: z80)

section probe_head (cpu: z80, vma: $8000) {
    pub proc ProbeHead () clobbers() {
        dc.b    SND_KICK_BANK
        dc.w    SND_KICK_PTR
        dc.w    SND_KICK_LEN
        dc.b    SND_BLIP_BANK
        dc.w    SND_BLIP_PTR
        dc.w    SND_BLIP_LEN
    }
}
";
    let bytes = colink(head).expect("co-linked SND_* equs resolve in dc cells");
    assert_eq!(bytes.len(), 10, "1+2+2 + 1+2+2 = 10 bytes");
    // kick
    assert_eq!(bytes[0], 0x0A, "BANK: SND_KICK_BANK (dc.b, link-folded bankid) = $0A ($50000>>15)");
    assert_eq!(&bytes[1..3], &[0x00, 0x80], "PTR: SND_KICK_PTR (dc.w, link-folded winptr) = $8000 LE");
    assert_eq!(&bytes[3..5], &[0x7E, 0x05], "LEN: SND_KICK_LEN (dc.w equ) = 1406 ($057E) LE");
    // blip
    assert_eq!(bytes[5], 0x09, "BANK: SND_BLIP_BANK = $09 ($48000>>15)");
    assert_eq!(&bytes[6..8], &[0x00, 0x80], "PTR: SND_BLIP_PTR = $8000 LE");
    assert_eq!(&bytes[8..10], &[0x40, 0x0B], "LEN: SND_BLIP_LEN = 2880 ($0B40) LE");
}

/// PROBE D — the `.len` CRUX. `.len` is a comptime property of a same-module
/// `Value::Data`; `Dac_Kick.len` in a module that does not own the blob must NOT
/// silently resolve. This documents WHY the LEN cell references the equ, not `.len`.
#[test]
fn colink_cross_module_dot_len_is_unavailable() {
    if !strict_gate() {
        eprintln!("skip seam2_colink_probe (set SIGIL_STRICT_GATE=1 + AEON_DIR)");
        return;
    }
    let head = "\
module engine.probe_head (cpu: z80)

section probe_head (cpu: z80, vma: $8000) {
    pub proc ProbeHead () clobbers() {
        dc.w    Dac_Kick.len
    }
}
";
    let r = colink(head);
    assert!(
        r.is_err(),
        "PROBE D: cross-module `.len` must fail (comptime .len needs same-module Value::Data), got {r:?}"
    );
}
