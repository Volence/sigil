//! Spec 2 · Plan 6 — per-file byte-diff harness + `.asm`→`.emp` port proof.
//!
//! The capstone proof: take a REAL Aeon `.asm` data file, assemble it in
//! isolation through the AS front-end (the reference bytes), compile its `.emp`
//! port through the modern front-end (the candidate bytes), and assert the two
//! are **byte-identical**. Plus the mixed-build link seam (T4) — an emp section
//! and an AS section composing into one linked image through the shared symbol
//! table.
//!
//! Target: `song_drumtest.asm` (82 bytes, pure `dc.b`, even length so `align 2`
//! is a no-op). Verified to assemble standalone via `sigil-frontend-as` under
//! `Cpu::M68000` (the `$xx` hex form requires 68k mode; under Z80 `$` is the
//! location counter). `sfx_33.asm` (58 bytes) is the documented fallback; both
//! are vendored under `tests/vectors/ports/` verbatim so this harness is
//! hermetic (it does not reach into the sibling `aeon/` tree).

use sigil_frontend_as::{assemble, Options};
use sigil_frontend_emp::lower::{lower_module, LowerOptions};
use sigil_frontend_emp::parse_str;
use sigil_ir::backend::Cpu;
use sigil_ir::{Section, SymbolTable};
use sigil_span::{Diagnostic, Level};

/// Assemble a single `.asm` source string through the AS front-end in isolation
/// (68k mode — the ports are 68k `dc.b` data), link with an empty external
/// table, and flatten to the reference bytes. Panics with the AS diagnostics on
/// failure (the ports are self-contained: no external symbols).
fn as_reference(asm: &str) -> Vec<u8> {
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    let module = assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble failed: {d:?}"));
    let linked = sigil_link::link(&module.sections, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("AS link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Compile a `.emp` source string through the modern front-end to its flat
/// linked image — the same pipeline the `sigil emp` CLI runs (parse →
/// `lower_module` → `resolve_layout` → `link` → `flatten`), with no sandbox root
/// (these ports use no `embed`/`import`). Panics on any `Error`-level
/// diagnostic.
fn emp_candidate(emp: &str) -> Vec<u8> {
    let (file, pdiags) = parse_str(emp);
    assert!(
        pdiags.iter().all(|d| d.level != Level::Error),
        "emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != Level::Error),
        "emp lower errors: {ldiags:?}"
    );
    let resolved = sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("emp resolve_layout failed: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("emp link failed: {d:?}"));
    sigil_link::flatten(&linked, 0x00)
}

/// Like [`emp_candidate`], but for the offsets-totality proof (Spec 2, Plan 7
/// backlog #3, Task 8): the `RelWord16Be` signed-word-range check lives in
/// `sigil_link::link`, so a genuinely overflowing offset table is a
/// `resolve_layout`/`link`-stage `Err`, not a panic. Parse and lower are still
/// expected to succeed (an offsets overflow is a LINK-time fact — nothing
/// upstream can see it), so those two stages still assert clean and panic with
/// their diagnostics on failure, exactly like `emp_candidate`; only the
/// `resolve_layout`/`link` seam returns its diagnostics to the caller instead
/// of unwrapping them. Returns an empty `Vec` if compilation fully succeeds
/// (so a test asserting a specific error message still fails informatively
/// rather than panicking here).
fn emp_link_diags(emp: &str) -> Vec<Diagnostic> {
    let (file, pdiags) = parse_str(emp);
    assert!(
        pdiags.iter().all(|d| d.level != Level::Error),
        "emp parse errors: {pdiags:?}"
    );
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None };
    let (module, ldiags) = lower_module(&file, &opts);
    assert!(
        ldiags.iter().all(|d| d.level != Level::Error),
        "emp lower errors: {ldiags:?}"
    );
    let resolved = match sigil_link::resolve_layout(&module.sections, &SymbolTable::new(), true) {
        Ok(sections) => sections,
        Err(diags) => return diags,
    };
    match sigil_link::link(&resolved, &SymbolTable::new()) {
        Ok(_) => Vec::new(),
        Err(diags) => diags,
    }
}

/// Assert two byte streams are identical, reporting the first differing offset
/// (and a short context window) on failure — the per-file byte-diff contract.
fn assert_byte_identical(reference: &[u8], candidate: &[u8], what: &str) {
    if reference == candidate {
        return;
    }
    let n = reference.len().min(candidate.len());
    if let Some(i) = (0..n).find(|&i| reference[i] != candidate[i]) {
        panic!(
            "{what}: first byte diff at offset {i:#x}: reference {:#04x} != candidate {:#04x}\n\
             reference[{i:#x}..] = {:02X?}\n candidate[{i:#x}..] = {:02X?}",
            reference[i],
            candidate[i],
            &reference[i..(i + 8).min(reference.len())],
            &candidate[i..(i + 8).min(candidate.len())],
        );
    }
    panic!(
        "{what}: lengths differ — reference {} bytes, candidate {} bytes (common prefix matches)",
        reference.len(),
        candidate.len()
    );
}

const DRUMTEST_ASM: &str = include_str!("vectors/ports/song_drumtest.asm");
const DRUMTEST_EMP: &str = include_str!("vectors/ports/song_drumtest.emp");

/// T1 — the AS reference side assembles standalone. Records the target choice:
/// `song_drumtest.asm` assembles in isolation to exactly its 82 source bytes
/// (the emitted image is the literal `dc.b` stream; `align 2` on an even length
/// is a no-op). This is the reference the emp port must reproduce byte-for-byte.
#[test]
fn as_reference_assembles_drumtest_standalone() {
    let bytes = as_reference(DRUMTEST_ASM);
    assert_eq!(bytes.len(), 82, "song_drumtest assembles to 82 bytes");
    // Spot-check the header + tail so a silent AS regression can't pass this.
    assert_eq!(&bytes[..4], &[0x07, 0x80, 0x00, 0x05]);
    assert_eq!(&bytes[80..], &[0x80, 0xEF]);
}

/// T1 — the harness pipeline is wired end-to-end: `emp_candidate` compiles a
/// trivial inline `[u8; N]` module to exactly its literal bytes, and
/// `assert_byte_identical` accepts an exact match. Proves both harness halves
/// before the real port lands (T2), so a T2 diff failure is unambiguously the
/// port, never the harness.
#[test]
fn harness_pipeline_roundtrips_inline_bytes() {
    let bytes = emp_candidate("module t\ndata X: [u8; 3] = [$AA, $BB, $CC]\n");
    assert_byte_identical(&[0xAA, 0xBB, 0xCC], &bytes, "harness self-test");
}

/// T2 — THE CAPSTONE. The `.emp` port of `song_drumtest.asm` compiles through
/// the modern front-end to bytes **byte-identical** to the AS-assembled
/// original. This is Plan 6's core acceptance criterion: a real Aeon data file,
/// ported and byte-exact.
#[test]
fn emp_port_matches_as_reference() {
    let reference = as_reference(DRUMTEST_ASM);
    let candidate = emp_candidate(DRUMTEST_EMP);
    assert_byte_identical(&reference, &candidate, "song_drumtest port");
}

/// T3 — `@as_compat` is proven **byte-neutral on the data path** (D-P6.3). The
/// port ships with `@as_compat`; stripping that one attribute line must not
/// change a single emitted byte (data emission is already AS-faithful — the
/// attribute's only effect is silencing modernization lints, of which a
/// data-only module has none). Both variants must also equal the AS reference.
#[test]
fn as_compat_is_byte_neutral_on_data() {
    assert!(
        DRUMTEST_EMP.contains("@as_compat"),
        "precondition: the port declares @as_compat"
    );
    // Strip exactly the `@as_compat` attribute line to build the no-compat twin
    // (prose comments mention the word, so filter the standalone attribute line,
    // not every occurrence of the substring).
    let without: String = DRUMTEST_EMP
        .lines()
        .filter(|l| l.trim() != "@as_compat")
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !without.lines().any(|l| l.trim() == "@as_compat"),
        "the twin has no @as_compat attribute line"
    );

    let with_compat = emp_candidate(DRUMTEST_EMP);
    let no_compat = emp_candidate(&without);
    assert_byte_identical(&with_compat, &no_compat, "@as_compat byte-neutrality");

    // And both still match the AS reference (byte-neutral means byte-exact).
    let reference = as_reference(DRUMTEST_ASM);
    assert_byte_identical(&reference, &with_compat, "with @as_compat vs AS");
    assert_byte_identical(&reference, &no_compat, "without @as_compat vs AS");
}

// ---------------------------------------------------------------------------
// T4 — mixed-build link seam (D-P6.2): an emp section and an AS section compose
// into ONE linked image through the shared flat symbol table. No new link
// infra — concat the two `Vec<Section>` and `resolve_layout` + `link` once.
// ---------------------------------------------------------------------------

/// The `Vec<Section>` an emp module lowers to (no sandbox root, 68k initial).
fn emp_sections(emp: &str) -> Vec<Section> {
    let (file, pdiags) = parse_str(emp);
    assert!(pdiags.iter().all(|d| d.level != Level::Error), "emp parse: {pdiags:?}");
    let (module, ldiags) = lower_module(&file, &LowerOptions { initial_cpu: Cpu::M68000, include_root: None });
    assert!(ldiags.iter().all(|d| d.level != Level::Error), "emp lower: {ldiags:?}");
    module.sections
}

/// The `Vec<Section>` an AS source assembles to (68k — pointer tables are 68k).
fn as_sections(asm: &str) -> Vec<Section> {
    let opts = Options { initial_cpu: Cpu::M68000, ..Options::default() };
    assemble(asm, &opts).unwrap_or_else(|d| panic!("AS assemble: {d:?}")).sections
}

/// T4 — cross-seam symbol resolution. An emp section defines the ported symbol
/// `Song_DrumTest` at VMA $8000; a synthetic AS consumer (`dc.l Song_DrumTest`,
/// a pointer-table entry — the real consumer shape) references it. Concatenated
/// and linked ONCE, the AS fixup resolves through the shared table to the emp
/// symbol's VMA: $00008000, big-endian.
#[test]
fn mixed_build_cross_seam_symbol_resolves() {
    // emp defines the symbol at an explicit, distinctive VMA.
    let emp = "module seam.payload\n\
               section payload (cpu: m68000, vma: $8000) {\n\
                 data Song_DrumTest: [u8; 4] = [$07, $80, $00, $05]\n\
               }\n";
    // AS consumer references it (unresolved in-file → a link-time fixup).
    let asm = "Consumer:\n\tdc.l Song_DrumTest\n";

    let mut sections = emp_sections(emp);
    sections.extend(as_sections(asm));
    // Mirror production `build_program`'s no-map tail: two independently-lowered
    // modules each stamp their first section `Pinned` at lma 0, so they must be
    // placed sequentially to distinct LMAs before the link-time placement pass
    // (R7p.4 now flags two Pinned-at-0 sections as colliding pins). The cross-seam
    // symbol resolves from `payload`'s VMA ($8000), so its LMA is irrelevant here.
    sigil_frontend_emp::resolve::place_sequential(&mut sections, 0);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let linked = sigil_link::link(&resolved, &SymbolTable::new())
        .unwrap_or_else(|d| panic!("link across the seam failed: {d:?}"));

    // The AS consumer lands in the auto-named `sec0` section; its 4 bytes are the
    // resolved pointer to the emp-defined `Song_DrumTest` ($8000), big-endian.
    let consumer = linked.section("sec0").expect("AS consumer section `sec0`");
    assert_eq!(consumer.bytes, vec![0x00, 0x00, 0x80, 0x00], "cross-seam pointer resolves to $8000");
}

/// T4 negative — a cross-section name collision between an emp-defined and an
/// AS-defined symbol of the SAME name is a hard link `Error`. The shared symbol
/// table admits exactly one definer per name regardless of producing front-end.
#[test]
fn mixed_build_cross_seam_name_collision_errors() {
    let emp = "module seam.payload\n\
               section payload (cpu: m68000, vma: $8000) {\n\
                 data Song_DrumTest: [u8; 2] = [$07, $80]\n\
               }\n";
    // The AS side ALSO defines `Song_DrumTest` — a genuine collision.
    let asm = "Song_DrumTest:\n\tdc.b $00\n";

    let mut sections = emp_sections(emp);
    sections.extend(as_sections(asm));
    // Place sequentially (as production does) so the two Pinned-at-0 first
    // sections don't trip R7p.4's colliding-pins check before we reach the
    // intended cross-seam name-collision error at `link`.
    sigil_frontend_emp::resolve::place_sequential(&mut sections, 0);

    let resolved = sigil_link::resolve_layout(&sections, &SymbolTable::new(), true)
        .unwrap_or_else(|d| panic!("resolve_layout: {d:?}"));
    let err = sigil_link::link(&resolved, &SymbolTable::new())
        .expect_err("a cross-seam name collision must be a hard link error");
    assert!(
        err.iter().any(|d| d.level == Level::Error
            && d.message.contains("Song_DrumTest")
            && d.message.contains("redefined")),
        "expected a `Song_DrumTest redefined` error, got: {err:?}"
    );
}

/// Plan 7 backlog #3 (Task 6) — the FORWARD direction of an `offsets` block:
/// it emits one `dc.w target - base` word per member and defines its base label
/// at the table's first byte. Three offset words (6 bytes) precede three
/// one-byte data items, so `frame0`@6, `frame1`@7, `frame2`@8, and the words
/// resolve to `frame{n} - Map` = 6, 7, 8 (signed word, big-endian).
#[test]
fn offsets_forward_emits_word_offsets() {
    let emp = "module m\n\
               section s (cpu: m68000, vma: $000000) {\n\
                 offsets Map { F0: frame0, F1: frame1, F2: frame2 }\n\
                 data frame0: [u8; 1] = [$11]\n\
                 data frame1: [u8; 1] = [$22]\n\
                 data frame2: [u8; 1] = [$33]\n\
               }\n";
    let bytes = emp_candidate(emp);
    assert_eq!(bytes, vec![0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x11, 0x22, 0x33]);
}

/// Plan 7 backlog #3 (Task 7) — byte-diff cross-check of the `offsets` FORWARD
/// direction against the AS front-end's OWN (independent) computation of
/// `dc.w Target-Base`.
///
/// A throwaway probe (since deleted) confirmed `directive_dc_w`'s
/// `Target-Base` operand folds cleanly here: on the converged pass both `Map`
/// and each `F{n}` label are already in the seeded env, so `self.fold(&qe)`
/// resolves the whole subtraction to a concrete `Fold::Value` — no
/// `Fixup`/poison path is exercised, and no "unresolved word expression" error
/// fires. So the AS reference below is a genuine second, independent
/// computation of the same offsets (not a hand-computed golden standing in for
/// one), which is what makes this a real cross-check rather than a tautology.
///
/// Layout (both sides): `Map` labels the table's first byte (address 0). Three
/// `dc.w` entries = 6 bytes occupy addresses 0..6, so `F0`@6, `F1`@7, `F2`@8 —
/// matching `offsets_forward_emits_word_offsets` above bit-for-bit, so the AS
/// and emp sides describe the identical layout.
#[test]
fn offsets_byte_identical_to_as_reference() {
    let asm = "Map:\n\
               \tdc.w F0-Map, F1-Map, F2-Map\n\
               F0:\n\
               \tdc.b $11\n\
               F1:\n\
               \tdc.b $22\n\
               F2:\n\
               \tdc.b $33\n";
    let reference = as_reference(asm);
    // Sanity-check the independent AS computation against the hand worked-out
    // layout before using it as the byte-diff golden.
    assert_eq!(reference, vec![0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x11, 0x22, 0x33]);

    let emp = "module m\n\
               section s (cpu: m68000, vma: $000000) {\n\
                 offsets Map { F0: frame0, F1: frame1, F2: frame2 }\n\
                 data frame0: [u8; 1] = [$11]\n\
                 data frame1: [u8; 1] = [$22]\n\
                 data frame2: [u8; 1] = [$33]\n\
               }\n";
    let candidate = emp_candidate(emp);
    assert_byte_identical(&reference, &candidate, "offsets vs AS dc.w Target-Base");
}

/// Plan 7 backlog #3 (Task 7) — the NEGATIVE-offset case: a member whose
/// target is defined BEFORE the offsets block's own base label, so
/// `target - base` is negative and must round-trip through two's-complement
/// as a signed 16-bit word.
///
/// Layout: `Zero` (2 bytes, `$99,$88`) sits at addresses 0..2. `Map` (the
/// offsets base) labels the table's first byte, right after `Zero`, at
/// address 2 — deliberately EVEN, so `directive_dc_w`'s own word-alignment
/// padding (it pads to an even address before emitting, independent of any
/// `padding off` convention) is a no-op and doesn't perturb the layout being
/// tested. The lone member's target is `Zero`, so the word is
/// `Zero - Map = 0 - 2 = -2`, which as a big-endian `i16` is `0xFF 0xFE`. A
/// trailing `Pad` byte (`$00`) at address 4 proves nothing after the table got
/// perturbed (no implicit padding under the emp/AS `padding off` convention).
#[test]
fn offsets_negative_forward_offset_byte_identical_to_as_reference() {
    let asm = "Zero:\n\
               \tdc.b $99, $88\n\
               Map:\n\
               \tdc.w Zero-Map\n\
               Pad:\n\
               \tdc.b $00\n";
    let reference = as_reference(asm);
    assert_eq!(reference, vec![0x99, 0x88, 0xFF, 0xFE, 0x00]);

    let emp = "module m\n\
               section s (cpu: m68000, vma: $000000) {\n\
                 data zero: [u8; 2] = [$99, $88]\n\
                 offsets Map { Z: zero }\n\
                 data pad: [u8; 1] = [$00]\n\
               }\n";
    let candidate = emp_candidate(emp);
    assert_byte_identical(&reference, &candidate, "offsets negative offset vs AS dc.w Target-Base");
}

/// Plan 7 backlog #3 (Task 8) — TOTALITY: an offset that overflows the signed
/// 16-bit word range is a COMPILE ERROR, not a silently-wrapped/truncated
/// value. `RelWord16Be` accepts `target - base` in `-$8000..=$7FFF`; this test
/// forces `target - base` to `$8002` (well past `+$7FFF`) by inserting a
/// 32768-byte data run (`pad`) between the offsets block's own base label
/// (`Tbl`, at offset 0) and its target (`far`).
///
/// The 32768-byte run is built from a range (`0..32768`) mapped to the byte
/// `0` — `(0..32768).map(|_| 0)` does not parse directly (a parenthesized
/// receiver is not a path — method calls require a path/const receiver, see
/// `eval_builtins.rs`'s equivalent workaround), so the range and the mapped
/// array are bound to consts first, then referenced by name from the `data`
/// item — there is no array-repeat LITERAL syntax in the language, but this
/// comptime `map` over a `Range` is the ergonomic equivalent and comfortably
/// inside the 5,000,000-step comptime budget.
///
/// Layout: `Tbl` (the offsets base) at offset 0, its own 2-byte word at
/// offset 0..2, `pad` at offset 2..32770, `far` at offset 32770. So
/// `far - Tbl = 32770 = $8002`, past `+$7FFF` by 3 — a genuine overflow, not
/// an off-by-one artifact of the boundary itself.
#[test]
fn offsets_overflow_is_a_compile_error() {
    let emp = "module m\n\
               const PadRange = 0..32768\n\
               const PadArr = PadRange.map(|_| 0)\n\
               section s (cpu: m68000, vma: $000000) {\n\
                 offsets Tbl { Far: far }\n\
                 data pad: [u8; 32768] = PadArr\n\
                 data far: [u8; 1] = [$99]\n\
               }\n";
    let diags = emp_link_diags(emp);
    assert!(
        diags.iter().any(|d| d.message.contains("signed-word range")),
        "expected a signed-word-range diagnostic, got: {diags:?}"
    );
}

/// Plan 7 backlog #3 (Task 8) — an `offsets` REVERSE ordinal (`Map.Seed`) is a
/// plain comptime int usable anywhere one is expected, including as an
/// ordinary emitted data byte — not merely inside a `const` expression (which
/// `eval_offsets.rs` already covers). `Map` declares three members in order
/// (`Idle`=0, `Shoot`=1, `Seed`=2); `Id` emits `Map.Seed` (2) as a `[u8; 1]`
/// data byte, landing as the image's LAST byte after the 3 offset words (6B)
/// and the 3 one-byte targets (3B).
#[test]
fn offsets_ordinal_usable_as_byte() {
    let emp = "module m\n\
               section s (cpu: m68000, vma: $000000) {\n\
                 offsets Map { Idle: a, Shoot: b, Seed: c }\n\
                 data a: [u8; 1] = [$11]\n\
                 data b: [u8; 1] = [$22]\n\
                 data c: [u8; 1] = [$33]\n\
                 data Id: [u8; 1] = [Map.Seed]\n\
               }\n";
    let bytes = emp_candidate(emp);
    // 3 offset words (6B) + a,b,c (3B) + Id (1B == Map.Seed == 2).
    assert_eq!(
        bytes,
        vec![0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x11, 0x22, 0x33, 0x02]
    );
    assert_eq!(bytes.last(), Some(&0x02));
}

/// Plan 7 backlog #3 (Task 8) — the documented `examples/offset_table.emp`
/// (both `offsets` directions, in the house style of `examples/pitcher_plant
/// .emp`) actually compiles end-to-end through the full modern pipeline, not
/// merely parses. Mirrors how `song_drumtest.emp` is pulled in above
/// (`include_str!`, relative to this test file).
#[test]
fn example_offset_table_compiles() {
    let src = include_str!("../../../examples/offset_table.emp");
    let bytes = emp_candidate(src);
    assert!(!bytes.is_empty());
    // Spot-check the documented layout so a silent regression can't pass this:
    // 3 offset words (6B) + 3 one-byte targets + CurrentState + StateCount.
    assert_eq!(
        bytes,
        vec![0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00, 0x01, 0x02, 0x00, 0x03]
    );
}

// ---- Plan 7 #5: item-position guards + (max_size:) end-to-end ------------

/// Guards and a passing `(max_size:)` emit ZERO bytes — the linked image is
/// byte-identical to the same program with them removed (D5.2/D5.4).
#[test]
fn guards_are_byte_neutral_end_to_end() {
    let with_guards = "module m\n\
        const N = 4\n\
        ensure(N == 4, \"objs {N}\")\n\
        data A (max_size: 2): [u8;2] = [1,2]\n\
        ensure(2 > 1, \"still ok\")\n\
        data B: [u8;2] = [3,4]\n";
    let without = "module m\n\
        data A: [u8;2] = [1,2]\n\
        data B: [u8;2] = [3,4]\n";
    assert_eq!(
        emp_candidate(with_guards),
        emp_candidate(without),
        "guards + passing max_size must be byte-neutral"
    );
    // And the shared payload is exactly what we wrote.
    assert_eq!(emp_candidate(without), vec![1, 2, 3, 4]);
}

/// Two real aeon guard SHAPES ported to item position: a divisibility `ensure`
/// (the `if cond / error` class) and an `ensure_fatal(here() <= limit, …)` (the
/// `if * > X / fatal` class) inside a `vma:` section where the position guard
/// passes. Both compile end-to-end and the section's data lands intact.
#[test]
fn aeon_shaped_guard_ports() {
    let src = "module m\n\
        const PERIOD = 64\n\
        ensure(256 % PERIOD == 0, \"256 must be divisible by {PERIOD}\")\n\
        section blk (vma: $7000) {\n\
        data pad: [u8; 4] = [$AA, $BB, $CC, $DD]\n\
        ensure_fatal(here() <= $8000, \"must fit under the $8000 window\")\n\
        }\n";
    let bytes = emp_candidate(src);
    assert_eq!(bytes, vec![0xAA, 0xBB, 0xCC, 0xDD], "section data intact, guards zero-byte");
}

/// Plan 7 #5 — the documented `examples/guards.emp` (item-position guards +
/// `(max_size:)`, all passing) compiles end-to-end. Mirrors
/// `example_offset_table_compiles`; pins the byte layout so a silent regression
/// (or a guard newly emitting bytes) can't slip through.
#[test]
fn example_guards_compiles() {
    let src = include_str!("../../../examples/guards.emp");
    let bytes = emp_candidate(src);
    // 3 offset words (dc.w target-base = 6,7,8) + 3 state bytes + 4 anim frames.
    assert_eq!(
        bytes,
        vec![0x00, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00, 0x01, 0x02, 0x10, 0x20, 0x30, 0x40]
    );
}

/// Plan 7 #6 Part A — the documented `examples/sst_overlay.emp` (SST overlay +
/// field-access-as-displacement, D6.A) compiles end-to-end with zero errors.
/// This is the compiles-today counterpart to the still-blocked
/// `examples/pitcher_plant.emp`; `emp_candidate` already asserts no Error-level
/// parse/lower diagnostics, so a clean run here proves the whole exhibit lowers.
///
/// Bytes hand-derived from the struct/overlay layout (mirrors
/// crates/sigil-frontend-emp/tests/overlay.rs's `SST` const: `Sst` is $50 bytes,
/// `x_pos` at $10, `y_vel` at $1A, `sst_custom` (34 bytes) at $2E) and standard
/// 68000 opcode encodings already proven byte-exact in that file:
///   proc tick (a0: *Sst):
///     subq.b  #1, timer(a0)   -> timer is PlantV's first overlay field, overlay-
///                                 relative 0, so in-memory offset = window $2E.
///                                 SUBQ.B #1,(d16,A0) = 0x5328, ext $002E.
///     move.w  x_pos(a0), d0   -> x_pos is a DIRECT struct field at $10.
///                                 MOVE.W (d16,A0),D0 = 0x3028, ext $0010.
///     move.w  y_vel(a0), d1   -> y_vel is a DIRECT struct field at $1A, dest D1.
///                                 MOVE.W (d16,A0),D1 = 0x3228, ext $001A.
///     move.b  charge(a0), d2  -> charge follows timer (u8) in the overlay, so its
///                                 overlay-relative offset is 1 -> in-memory $2F.
///                                 Reading 1 of its 2 bytes is legal (narrower than
///                                 field). MOVE.B (d16,A0),D2 = 0x1428, ext $002F.
///     rts                     -> 0x4E75.
///   proc peek ():
///     tst.b   PlantV.timer(a1) -> qualified access on an UNTYPED a1, same $2E
///                                 in-memory offset as the bare form above.
///                                 TST.B (d16,A1) = 0x4A29, ext $002E.
///     rts                     -> 0x4E75.
#[test]
fn example_sst_overlay_compiles() {
    let src = include_str!("../../../examples/sst_overlay.emp");
    let bytes = emp_candidate(src);
    assert_eq!(
        bytes,
        vec![
            0x53, 0x28, 0x00, 0x2E, // subq.b #1, timer(a0)   == $2E(a0)
            0x30, 0x28, 0x00, 0x10, // move.w x_pos(a0), d0   == $10(a0)
            0x32, 0x28, 0x00, 0x1A, // move.w y_vel(a0), d1   == $1A(a0)
            0x14, 0x28, 0x00, 0x2F, // move.b charge(a0), d2  == $2F(a0)
            0x4E, 0x75, // rts
            0x4A, 0x29, 0x00, 0x2E, // tst.b PlantV.timer(a1) == $2E(a1)
            0x4E, 0x75, // rts
        ]
    );
}

/// `examples/dispatch.emp` — Spec 2, Plan 7 backlog #6, Part B (D6.B) worked
/// exhibit: the SAME three procs (`Init`/`Wait`/`Shoot`) dispatched through
/// BOTH shipped encodings, `word_offsets` then `long_ptrs`, each followed by a
/// routine-byte-idiom `data` item consuming a pre-scaled ordinal. Declaration
/// order is tables-then-procs (both tables before any proc), so every
/// `word_offsets` delta is a small POSITIVE forward offset — the idiomatic
/// S3K spelling — and every `long_ptrs` entry is a forward absolute pointer.
///
/// Layout (origin 0, harness flattens at `0x00`):
///   Routines (word_offsets, 3 members × 2 bytes)   @ $00, 6 bytes
///   initial_routine: [u8;1] = [Routines.Init]      @ $06, 1 byte
///   wait_routine:     [u8;1] = [Routines.Wait]      @ $07, 1 byte
///   PtrRoutines (long_ptrs, 3 members × 4 bytes)    @ $08, 12 bytes
///   ptr_wait_routine: [u8;1] = [PtrRoutines.Wait]   @ $14, 1 byte
///   proc Init  (moveq #0,d0 ; rts)                  @ $15, 4 bytes
///   proc Wait  (move.w #64,d1 ; rts)                @ $19, 6 bytes
///   proc Shoot (moveq #1,d0 ; rts)                  @ $1F, 4 bytes
///   total: $23 (35) bytes
///
/// Target addresses: Init=$15, Wait=$19, Shoot=$1F.
///
/// `Routines` (word_offsets, base $00, ordinals ×2):
///   dc.w Init  - Routines = $15 - $00 = $0015
///   dc.w Wait  - Routines = $19 - $00 = $0019
///   dc.w Shoot - Routines = $1F - $00 = $001F
///   Routines.Init = ordinal 0 * 2 = 0; Routines.Wait = ordinal 1 * 2 = 2.
///
/// `PtrRoutines` (long_ptrs, absolute, ordinals ×4):
///   dc.l Init  = $00000015
///   dc.l Wait  = $00000019
///   dc.l Shoot = $0000001F
///   PtrRoutines.Wait = ordinal 1 * 4 = 4.
///
/// Instruction encodings (proven in dispatch.rs / lower_corpus.rs /
/// lower_proc.rs / m68k.rs): `moveq #n,d0` = `70 nn` (quick-family, `0111
/// rrr0 dddddddd`, reg=d0); `move.w #64,d1` = MOVE word-size (size bits
/// `11`), dest D1/Dn mode (`dst_mode=000,dst_reg=001`), src `#imm` mode
/// (`111,100`) => word `0011 001 000 111 100` = `0x323C`, extension word
/// `$0040`; `rts` = `4E 75`.
#[test]
fn example_dispatch_compiles() {
    let src = include_str!("../../../examples/dispatch.emp");
    let bytes = emp_candidate(src);
    assert_eq!(
        bytes,
        vec![
            // Routines (word_offsets): dc.w Init-Routines, Wait-Routines, Shoot-Routines
            0x00, 0x15, 0x00, 0x19, 0x00, 0x1F,
            // initial_routine: [Routines.Init] = 0
            0x00,
            // wait_routine: [Routines.Wait] = 2
            0x02,
            // PtrRoutines (long_ptrs): dc.l Init, Wait, Shoot
            0x00, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x1F,
            // ptr_wait_routine: [PtrRoutines.Wait] = 4
            0x04,
            // proc Init: moveq #0,d0 ; rts
            0x70, 0x00, 0x4E, 0x75,
            // proc Wait: move.w #64,d1 ; rts
            0x32, 0x3C, 0x00, 0x40, 0x4E, 0x75,
            // proc Shoot: moveq #1,d0 ; rts
            0x70, 0x01, 0x4E, 0x75,
        ]
    );
}

/// `examples/reach_branches.emp` — Spec 2, Plan 7 backlog #8 worked exhibit:
/// every PC-relative relaxation-ladder rung this branch ships, byte-exact.
/// Compiles with ZERO diagnostics (no `@as_compat`, so Part C's unsized-Bcc
/// relaxation is live; every proc terminates cleanly, so no
/// `[proc.undeclared-fallthrough]` noise either) — `emp_candidate` already
/// asserts no Error-level parse/lower diagnostics, and this test additionally
/// re-parses to assert there are no diagnostics AT ALL.
///
/// Every byte below is hand-derived from the file's own structure, independent
/// of the compiler (the uniform 68000 rule used throughout: for BOTH `.s` and
/// `.w` encodings, `disp = target - (site + 2)`, where `site` is the
/// branch/call instruction's own start address — the 68000 PC used for the
/// displacement add is always op+2 regardless of the final instruction
/// length). Addresses accumulate strictly in file order (no reordering, no
/// alignment padding — every item here is already even-sized).
///
/// ```text
/// addr  bytes                  item
/// 0x00  60 02                  proc near_forward: jbra .fwd -> bra.s
///                                 disp = 4 - (0+2) = 2
/// 0x02  4E 71                  nop (filler; keeps .fwd off disp-0)
/// 0x04  4E 75                  .fwd: rts
/// 0x06  4E 71                  proc near_backward: .back: nop
/// 0x08  60 FC                  jbra .back -> bra.s, disp = 6 - (8+2) = -4 = 0xFC
/// 0x0A  4E 75                  proc TableGuard: rts (named landing pad)
/// 0x0C  60 00 00 CA            proc cross_table: jbra AfterTable -> bra.w
///                                 (rung 0 hypothetical disp = 200, > 127, excluded)
///                                 disp = 216 - (0x0C+2) = 202 = 0x00CA
/// 0x10  <200 bytes>            DataTable: 8-entry pseudo-sine cycle x25
/// 0xD8  4E 75                  proc AfterTable: rts
/// 0xDA  61 04                  proc call_helper: jbsr Helper -> bsr.s
///                                 disp = 224 - (0xDA+2) = 4
/// 0xDC  4E 71                  nop (filler; keeps Helper off disp-0)
/// 0xDE  4E 75                  call_helper's own rts
/// 0xE0  4E 75                  proc Helper: rts
/// 0xE2  66 02                  proc unsized_near: bne .n1 -> bne.s (cc=6)
///                                 disp = 0xE6 - (0xE2+2) = 2
/// 0xE4  4E 71                  nop
/// 0xE6  4E 75                  .n1: rts
/// 0xE8  62 02                  proc unsized_near_hi: bhi .n2 -> bhi.s (cc=2)
///                                 disp = 0xEC - (0xE8+2) = 2
/// 0xEA  4E 71                  nop
/// 0xEC  4E 75                  .n2: rts
/// 0xEE  62 00 FF 1A            proc unsized_far: bhi TableGuard -> bhi.w
///                                 (backward across DataTable, out of i8 range)
///                                 disp = 0x0A - (0xEE+2) = -230 = 0xFF1A
/// 0xF2  4E 75                  unsized_far's own rts
/// 0xF4  60 00 00 02            proc disp0_escape: jbra .imm
///                                 rung 0 hypothetical: target = 0xF4+2 = 0xF6,
///                                 disp = 0xF6-(0xF4+2) = 0 -> EXCLUDED (0x00 byte
///                                 is the word-form escape, unencodable)
///                                 rung 1: disp = 0xF8 - (0xF4+2) = 2 -> bra.w
/// 0xF8  4E 75                  .imm: rts
/// total: 0xFA (250) bytes
/// ```
#[test]
fn example_reach_branches_compiles_byte_exact() {
    let src = include_str!("../../../examples/reach_branches.emp");

    // Zero diagnostics at all (not just zero errors) — a clean teaching exhibit.
    let (file, pdiags) = parse_str(src);
    assert!(pdiags.is_empty(), "reach_branches parse diagnostics: {pdiags:?}");
    let opts = LowerOptions { initial_cpu: Cpu::M68000, include_root: None };
    let (_module, ldiags) = lower_module(&file, &opts);
    assert!(ldiags.is_empty(), "reach_branches lower diagnostics: {ldiags:?}");

    let bytes = emp_candidate(src);

    let mut want = Vec::new();
    want.extend_from_slice(&[0x60, 0x02]); // jbra .fwd -> bra.s disp 2
    want.extend_from_slice(&[0x4E, 0x71]); // nop
    want.extend_from_slice(&[0x4E, 0x75]); // .fwd: rts
    want.extend_from_slice(&[0x4E, 0x71]); // .back: nop
    want.extend_from_slice(&[0x60, 0xFC]); // jbra .back -> bra.s disp -4
    want.extend_from_slice(&[0x4E, 0x75]); // TableGuard: rts
    want.extend_from_slice(&[0x60, 0x00, 0x00, 0xCA]); // jbra AfterTable -> bra.w disp 202
    for _ in 0..25 {
        want.extend_from_slice(&[0x00, 0x2D, 0x40, 0x2D, 0x00, 0xD3, 0xC0, 0xD3]); // DataTable cycle
    }
    want.extend_from_slice(&[0x4E, 0x75]); // AfterTable: rts
    want.extend_from_slice(&[0x61, 0x04]); // jbsr Helper -> bsr.s disp 4
    want.extend_from_slice(&[0x4E, 0x71]); // nop
    want.extend_from_slice(&[0x4E, 0x75]); // call_helper's own rts
    want.extend_from_slice(&[0x4E, 0x75]); // Helper: rts
    want.extend_from_slice(&[0x66, 0x02]); // bne .n1 -> bne.s disp 2
    want.extend_from_slice(&[0x4E, 0x71]); // nop
    want.extend_from_slice(&[0x4E, 0x75]); // .n1: rts
    want.extend_from_slice(&[0x62, 0x02]); // bhi .n2 -> bhi.s disp 2
    want.extend_from_slice(&[0x4E, 0x71]); // nop
    want.extend_from_slice(&[0x4E, 0x75]); // .n2: rts
    want.extend_from_slice(&[0x62, 0x00, 0xFF, 0x1A]); // bhi TableGuard -> bhi.w disp -230
    want.extend_from_slice(&[0x4E, 0x75]); // unsized_far's own rts
    want.extend_from_slice(&[0x60, 0x00, 0x00, 0x02]); // jbra .imm -> bra.w disp 2 (disp-0 excluded at rung 0)
    want.extend_from_slice(&[0x4E, 0x75]); // .imm: rts

    assert_eq!(want.len(), 250, "hand-derived expectation totals 250 bytes");
    assert_byte_identical(&want, &bytes, "reach_branches exhibit");
}

// ---- Plan 7 #6 audit fix: nested `section {}` is rejected loudly ---------

/// A `section {}` nested inside another `section {}` used to be silently
/// dropped by `lower_section_items` (no `Item::Section` arm there) — losing
/// data bytes, an `ensure_fatal` guard, AND an over-capacity `(max_size:)`
/// check all at once. It must now be rejected at PARSE time with
/// `[section.nested]`, never reaching lowering.
#[test]
fn nested_section_with_guards_and_capacity_is_rejected_at_parse_not_silently_dropped() {
    let src = "module m\n\
        section outer {\n\
        section inner {\n\
        ensure_fatal(false, \"x\")\n\
        data T (max_size: 1): [u8; 4] = [1, 2, 3, 4]\n\
        }\n\
        }\n";
    let (_file, diags) = parse_str(src);
    assert!(
        diags.iter().any(|d| d.message.contains("[section.nested]")),
        "want [section.nested], got: {diags:?}"
    );
}
