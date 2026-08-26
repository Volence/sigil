//! Derived ROM layout (2026-08-26, decision d-7) — the shipped-shape invariants the
//! derived placement must keep, read off the SAME resolve the ROM comes from
//! (`native::resolve_frozen_layout`), never off a pin:
//!
//!   * the error_handler island is the LAST byte-emitting section, immediately before
//!     `EndOfRom` — the MD Debugger locates its symbol table with two `lea`
//!     displacements baked into the vendored blob, both pointing at blob end, so any
//!     section placed after the blob silently breaks every crash-screen symbol.
//!
//! ```text
//! SIGIL_STRICT_GATE=1 AEON_DIR=/path/to/aeon cargo test -p sigil-cli --test derived_layout
//! ```
use sigil_harness::native;
use sigil_harness::test_support::reference_tree_for_profile;

// The frozen resolve touches the shared engine/sound/generated dir.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// For `profile`: the section carrying `ErrorHandlerBlob` has the greatest base of every
/// byte-emitting ROM section, its end (base + blob offset + blob length) is exactly the
/// `EndOfRom` address, and nothing emits a byte at or past that end.
fn error_handler_is_the_last_emission(profile: native::GameProfile) {
    let Some(aeon) = reference_tree_for_profile(&profile) else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let layout = native::resolve_frozen_layout(&aeon, &profile).unwrap_or_else(|e| panic!("{e}"));

    let mut blob: Option<(u32, u32)> = None; // (section base, blob offset)
    let mut end_of_rom: Option<u32> = None;
    let mut emitters: Vec<(u32, u32, String)> = Vec::new(); // (base, end, name)
    for s in &layout {
        if s.vma_base.is_some_and(|v| v != s.lma && v >= 0x8000) && s.image_len() == 0 {
            continue;
        }
        for l in &s.labels {
            if l.name == native::ERROR_HANDLER_BLOB_LABEL {
                assert!(blob.is_none(), "two `{}` labels", native::ERROR_HANDLER_BLOB_LABEL);
                blob = Some((s.lma, l.offset));
            }
            if l.name == "EndOfRom" {
                end_of_rom = Some(s.lma + l.offset);
            }
        }
        let len = s.image_bytes().len() as u32;
        if len > 0 {
            emitters.push((s.lma, s.lma + len, s.name.clone()));
        }
    }
    // LOUD on unmeasurable: a shape without the blob or the terminus proves nothing.
    let (blob_base, blob_off) = blob.unwrap_or_else(|| {
        panic!("`{}` is not in this shape's layout — the invariant cannot be measured", native::ERROR_HANDLER_BLOB_LABEL)
    });
    let end_of_rom = end_of_rom.expect("`EndOfRom` is not in this shape's layout — the invariant cannot be measured");
    let blob_end = blob_base + blob_off + native::ERROR_HANDLER_BLOB_LEN;
    assert_eq!(blob_end, end_of_rom, "blob end {blob_end:#x} != EndOfRom {end_of_rom:#x}");
    let last = emitters.iter().max_by_key(|(base, _, _)| *base).expect("no byte-emitting section");
    assert_eq!(last.0, blob_base, "the highest-based emitter is `{}` at {:#x}, not the error_handler island at {blob_base:#x}", last.2, last.0);
    let trailing: Vec<_> = emitters.iter().filter(|(_, end, _)| *end > blob_end).collect();
    assert!(trailing.is_empty(), "section(s) emit past the blob end {blob_end:#x}: {trailing:?}");
}

#[test]
fn s4_debug_error_handler_is_the_last_emission() {
    error_handler_is_the_last_emission(native::sonic4_profile(true));
}

#[test]
fn s4_error_handler_is_the_last_emission() {
    error_handler_is_the_last_emission(native::sonic4_profile(false));
}
