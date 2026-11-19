//! The deb2 appendix: the MD Debugger symbol table `convsym` appends after `EndOfRom`,
//! its placement precondition (the blob-end contract), the native header re-fix over
//! the appended file, and the presence controls.
//!
//! Moved from sigil-harness (`native.rs`) unchanged by G0b Task 4. Which shapes carry an
//! appendix, and the per-game size floor, stay there (`declares_error_handler_island`,
//! `SONIC4_APPENDIX_FLOOR` / `DEMO_APPENDIX_FLOOR`): they read the game profile.

use std::path::Path;

/// The convsym `as_lst` filter build.sh passes verbatim (`build.sh:170-171`).
/// `z[A-Z].+` currently matches zero Aeon labels; passed for parity.
pub const CONVSYM_FILTER: &str = "z[A-Z].+";

/// The deb2 appendix magic (MD-Debugger symbol table) — the FIRST TWO bytes only.
/// convsym writes `de b2` then DATA-DEPENDENT header bytes (observed `04 02` in the
/// shipped ROMs, `00 1a`/`00 06` elsewhere — they encode the packing/offset, NOT a
/// fixed magic), so the presence control asserts these two bytes + a size band, not
/// the four-byte constant design §3.3 wrongly proposed (§S1.4 note).
pub const DEB2_MAGIC: [u8; 2] = [0xDE, 0xB2];

/// The label the vendored MD Debugger v2.6 blob is emitted under
/// (`engine/debug/error_handler.emp`). Present in BOTH canonical shapes since the
/// crash-report ruling (owner-ruled 2026-08-04), so the blob-end guard below now binds
/// the SHIPPED release ROM too — it is more load-bearing than it was, not less. The
/// opt-in `lean` profile is the one shape that carries no island and no such label.
pub const ERROR_HANDLER_BLOB_LABEL: &str = "ErrorHandlerBlob";

/// The length, in bytes, of the vendored MD Debugger v2.6 blob — the opaque
/// `dc.l` transliteration in `engine/debug/error_handler.emp`, NOT the whole island
/// (that is the 0x15A of exception stubs + this 0xF56 blob = 0x10B0).
///
/// Load-bearing because the blob HARDCODES it: two PC-relative `lea`s baked into the
/// vendored bytes (`dc.l $43FA090A` at blob+0x64A and `dc.l $47FA0872` at blob+0x6E2)
/// both resolve to `ErrorHandlerBlob + 0xF56`, one byte past the blob's last byte, and
/// each follows with `cmpi.w #$DEB2,(a1)+`. Upstream's contract is "convsym appends the
/// symbol table immediately after me"; convsym appends at `EndOfRom`. So a shape that
/// carries the blob must satisfy `EndOfRom == ErrorHandlerBlob + 0xF56`.
pub const ERROR_HANDLER_BLOB_LEN: u32 = 0xF56;

/// HARD placement guard: for a shape that carries the MD Debugger blob, the deb2
/// appendix MUST start at exactly `ErrorHandlerBlob + ERROR_HANDLER_BLOB_LEN`, i.e. the
/// error_handler island must be the LAST byte-emitting section. `appendix_start` is
/// `EndOfRom` (the assembled image length; the ROM region is based at 0, the same
/// identity the positive control below already relies on).
///
/// `expect_island` says whether this shape carries the island AT ALL, and it must come
/// from somewhere upstream of `listing` — [`declares_error_handler_island`] for a
/// profile-driven caller. It is what makes the check non-vacuous: a listing WITHOUT the
/// blob label is a satisfied contract for the `lean` shape and a renamed label, a
/// re-layout that dropped the island, or a harvest that lost the symbol for every other
/// shape, and nothing in the listing distinguishes those. So both directions of the
/// mismatch are refusals — an expected island whose label never appeared, and an
/// unexpected island whose label did.
///
/// This fails the BUILD rather than warning: the failure it prevents is silent at
/// runtime (the ROM assembles, boots and crashes correctly — it just prints `<unknown>`
/// for every Offset/Caller), so a warning would be read past. It is the enforcement
/// arm of the INVARIANT declared in `games/<g>/map.toml` and documented in
/// `engine/debug/error_handler.emp`.
///
/// The per-shape half of the same contract — that the shapes DECLARING the island are
/// exactly the shapes whose listings define its label — is
/// `tests/error_handler_island_membership.rs`, because the `lean` shape never reaches
/// this function (its callers skip the appendix entirely).
pub fn check_error_handler_is_last(
    listing: &[sigil_link::ListingSymbol],
    appendix_start: usize,
    expect_island: bool,
) -> Result<(), String> {
    check_error_handler_is_last_with(ERROR_HANDLER_BLOB_LABEL, ERROR_HANDLER_BLOB_LEN, listing, appendix_start, expect_island)
}

/// [`check_error_handler_is_last`] with the blob's label and length passed in: the new
/// driver reads both from the engine description (`engine.toml` `error_handler_blob`,
/// `error_handler_blob_len`) instead of this module's constants (G0b Task 5).
pub fn check_error_handler_is_last_with(
    blob_label: &str,
    blob_len: u32,
    listing: &[sigil_link::ListingSymbol],
    appendix_start: usize,
    expect_island: bool,
) -> Result<(), String> {
    // Same lookup idiom the layout walk uses for `EndOfRom` — match by name.
    let found = listing.iter().find(|l| l.name == blob_label);
    let blob = match (expect_island, found) {
        (false, None) => return Ok(()),
        (true, None) => {
            return Err(format!(
                "MDDBG island MEMBERSHIP violated: this shape DECLARES the error_handler \
                 island, but its listing defines no `{blob_label}` among \
                 {} symbol(s), so the blob-end contract has no subject and would have \
                 passed by having nothing to check. The island is either not placed in \
                 this shape (a registry or `order` change), its blob label was renamed in \
                 engine/debug/error_handler.emp, or the listing harvest dropped it. Every \
                 one of those ships a ROM whose crash screen prints `<unknown>` for every \
                 Offset/Caller with no other symptom.",
                listing.len(),
            ));
        }
        (false, Some(b)) => {
            return Err(format!(
                "MDDBG island MEMBERSHIP violated: this shape declares NO error_handler \
                 island, yet its listing defines `{blob_label}` at {:#x}. \
                 The island and the lean fault handler are the two arms of one shape \
                 gate and share a placement slot, so a shape carrying both has lost the \
                 split, and the deb2 appendix this shape is not supposed to need would \
                 land somewhere the blob's baked `lea` displacements do not point.",
                b.value,
            ));
        }
        (true, Some(b)) => b,
    };
    let expect = blob.value.wrapping_add(blob_len) as usize;
    if appendix_start == expect {
        return Ok(());
    }
    let drift = appendix_start as i64 - expect as i64;
    Err(format!(
        "MDDBG blob-end contract VIOLATED: the deb2 appendix starts at EndOfRom \
         {appendix_start:#x}, but `{blob_label}` ({:#x}) + blob length \
         {blob_len:#x} = {expect:#x}, a drift of {drift:+} byte(s). \
         The MD Debugger locates its symbol table with two `lea` displacements BAKED \
         into the vendored blob bytes, both pointing at blob end, so the appendix must \
         start exactly there. {} Fix the `order` list in games/<game>/map.toml so the \
         error_handler island (BusError../ErrorHandlerBlob) is the FINAL byte-emitting \
         section, immediately before EndOfRom, do not touch the blob bytes.",
        blob.value,
        if drift > 0 {
            "Some section is placed AFTER the blob; the blob's `cmpi.w #$DEB2,(a1)+` will \
             read those bytes instead of the table, fail, and every crash-screen \
             Offset/Caller line will silently print `<unknown>`."
        } else {
            "The appendix starts BEFORE blob end, the blob is truncated or \
             the blob length no longer matches the vendored binary."
        }
    ))
}

/// Given the assembled ROM + listing, write both to a temp dir, run
/// `convsym … -output deb2 -a` then `fixheader`, and return the full appended file.
/// Split out so the t24 doctored-listing negative control can feed a mutated set.
///
/// `expect_island` declares whether the shape this (rom, listing) pair came from
/// carries the MD Debugger island, and is the expectation
/// [`check_error_handler_is_last`] is judged against. A profile-driven caller derives
/// it with [`declares_error_handler_island`]; a synthetic probe feeding a listing it
/// built itself declares what that listing represents. It is a parameter rather than
/// an assumption because a listing with no blob label is a satisfied contract for one
/// shape and a silent runtime failure for every other, and this function cannot tell
/// them apart from the listing.
pub fn append_deb2_appendix(
    aeon: &Path,
    rom: &[u8],
    listing: &[sigil_link::ListingSymbol],
    debug: bool,
    min_appendix: usize,
    expect_island: bool,
) -> Result<Vec<u8>, String> {
    let facts = AppendixFacts {
        blob_label: ERROR_HANDLER_BLOB_LABEL,
        blob_len: ERROR_HANDLER_BLOB_LEN,
        convsym: aeon.join("tools").join("convsym"),
    };
    append_deb2_appendix_with(&facts, rom, listing, debug, min_appendix, expect_island)
}

/// The engine facts the appendix reads: the MD Debugger blob's label and length, and the
/// `convsym` binary. The old driver passes this module's constants and `<aeon>/tools/convsym`;
/// the new driver passes the engine description's values (G0b Task 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendixFacts<'a> {
    pub blob_label: &'a str,
    pub blob_len: u32,
    pub convsym: std::path::PathBuf,
}

/// [`append_deb2_appendix`] with the engine facts passed in ([`AppendixFacts`]).
pub fn append_deb2_appendix_with(
    facts: &AppendixFacts<'_>,
    rom: &[u8],
    listing: &[sigil_link::ListingSymbol],
    debug: bool,
    min_appendix: usize,
    expect_island: bool,
) -> Result<Vec<u8>, String> {
    // PLACEMENT PRECONDITION (checked BEFORE shelling convsym — the contract is about
    // where the appendix will land, so a violation is knowable from the layout alone).
    check_error_handler_is_last_with(facts.blob_label, facts.blob_len, listing, rom.len(), expect_island)?;

    let convsym = facts.convsym.clone();
    if !convsym.exists() {
        return Err(format!("convsym not found at {}", convsym.display()));
    }

    // A unique temp dir (pid + shape + a monotonic counter) so parallel builds
    // never share a `.bin`/`.lst`.
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "sigil_s14_{}_{}_{}",
        std::process::id(),
        if debug { "debug" } else { "plain" },
        seq
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bin = dir.join("rom.bin");
    let lst = dir.join("rom.lst");
    std::fs::write(&bin, rom).map_err(|e| e.to_string())?;
    // Demangle the sigil-canonical `.emp` locals into `Parent.local` + drop compiler
    // plumbing (Stage-3 P2b, OQ-B) so the source-meaningful locals SURVIVE convsym's
    // `as_lst` name parser (which rejects the mangled `$` form) and reach the deb2
    // appendix. ROM-byte-neutral: the appendix is post-`EndOfRom` symbol data only.
    //
    // EQUATES ARE DROPPED HERE, at the deb2 boundary, and the drop is STRUCTURAL —
    // not a reliance on convsym's `as_lst` reader taking only `C` rows. deb2 is the
    // MD Debugger's ADDRESS table (it answers "what code is at this PC?"), and an
    // equate is a value, so an equate has nothing to say to it. Making the exclusion
    // explicit is also what keeps every shipped ROM BYTE-IDENTICAL across this
    // parcel: the appendix is part of the ROM, so one leaked row would grow it and
    // move every golden.
    let addresses: Vec<sigil_link::ListingSymbol> =
        listing.iter().filter(|s| !s.is_equate).cloned().collect();
    let deb2_listing = sigil_link::demangle_symbols(&addresses);
    std::fs::write(&lst, sigil_link::emit_listing(&deb2_listing)).map_err(|e| e.to_string())?;

    // convsym: append the deb2 table (build.sh:170-171 flags verbatim).
    let out = sigil_span::read_set::tool_command(&convsym)
        .map_err(|e| format!("read convsym {}: {e}", convsym.display()))?
        .arg(&lst)
        .arg(&bin)
        .args(["-input", "as_lst", "-range", "0", "FFFFFF", "-exclude", "-filter"])
        .arg(CONVSYM_FILTER)
        .arg("-a")
        .output()
        .map_err(|e| format!("spawn convsym: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "convsym failed (rc {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    // read-set: not a build input. This is convsym's output read back, the build's
    // own product, and the ROM identity in the digest covers it.
    let mut full = std::fs::read(&bin).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&dir);

    // Re-fix the Sega header over the APPENDED file (was `tools/fixheader`, retired at
    // Stage-3 P4d — a native fold, verified byte-identical to fixheader's output):
    //   (1) the ROM-end pointer at $1A4 (4 bytes BE) = the last byte's address (len-1);
    //   (2) the checksum at $18E over [$200, len) — AFTER (1), since $1A4 is in range.
    if full.len() >= 0x1A8 {
        let end = (full.len() as u32).wrapping_sub(1);
        full[0x1A4..0x1A8].copy_from_slice(&end.to_be_bytes());
        sigil_link::apply_header_checksum(&mut full);
    }

    // POSITIVE CONTROL (assert-PRESENCE, §S1.4 / condition 2b): the deb2 magic MUST
    // sit at EndOfRom and the appendix MUST be non-trivial — a silent convsym
    // failure (`2>/dev/null || true` in build.sh) yields an appendix-LESS ROM, which
    // this rejects as a HARD error rather than a smaller "valid" file.
    let eor = rom.len();
    if full.len() <= eor {
        return Err(format!(
            "deb2 appendix MISSING: full file {} <= assembled {} (convsym silently produced no append)",
            full.len(),
            eor
        ));
    }
    if full[eor..eor + 2] != DEB2_MAGIC {
        return Err(format!(
            "deb2 magic absent at EndOfRom {eor:#x}: found {:02X?}",
            &full[eor..eor + 2.min(full.len() - eor)]
        ));
    }
    let appendix = full.len() - eor;
    // Size band: the sigil-canonical appendix size is target-dependent (sonic4/config
    // ~11 KB+, demo ~7 KB); `min_appendix` is the per-target floor. A wildly-out-of-band
    // size means the symbol set collapsed or exploded (the t24 presence control).
    if !(min_appendix..=0x10000).contains(&appendix) {
        return Err(format!(
            "deb2 appendix size {appendix:#x} out of the expected band ({min_appendix:#x}..0x10000)"
        ));
    }
    Ok(full)
}

/// Run the REAL `tools/convsym … -output log` over sigil's listing — the functional
/// consumer path (condition 2c): the same parser that packs the deb2 table, asked
/// to resolve names→addresses. Returns the `HEXADDR: Name` map. A load-bearing
/// symbol resolving to its known address here is proof the appendix carries it
/// correctly; a doctored `.lst` address is detected (t24).
pub fn convsym_resolve(aeon: &Path, listing: &[sigil_link::ListingSymbol]) -> Result<std::collections::HashMap<String, u32>, String> {
    let convsym = aeon.join("tools/convsym");
    let dir = std::env::temp_dir().join(format!("sigil_s14_log_{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let lst = dir.join("resolve.lst");
    std::fs::write(&lst, sigil_link::emit_listing(listing)).map_err(|e| e.to_string())?;
    let out = sigil_span::read_set::tool_command(&convsym)
        .map_err(|e| format!("read convsym {}: {e}", convsym.display()))?
        .arg(&lst)
        .arg("-")
        .args(["-input", "as_lst", "-output", "log", "-range", "0", "FFFFFF", "-exclude", "-filter"])
        .arg(CONVSYM_FILTER)
        .output()
        .map_err(|e| format!("spawn convsym -output log: {e}"))?;
    let _ = std::fs::remove_file(&lst);
    if !out.status.success() {
        return Err(format!("convsym -output log rc {:?}", out.status.code()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        // `HEXADDR: Name`
        if let Some((addr, name)) = line.split_once(": ") {
            if let Ok(v) = u32::from_str_radix(addr.trim(), 16) {
                map.insert(name.trim().to_string(), v);
            }
        }
    }
    Ok(map)
}
