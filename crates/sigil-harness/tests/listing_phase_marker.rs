//! THE LISTING SAYS WHETHER AN ADDRESS IS A VMA OR AN LMA, ON EVERY SHIPPED SHAPE.
//!
//! Every address row in a sigil listing prints a VMA. For most symbols that is also
//! the LMA and nothing is lost; for a symbol in a PHASED section it is a bank-local
//! runtime address whose bytes are stored somewhere else entirely. The file used to
//! say nothing about which was which, so a consumer that needed the distinction had
//! to re-derive it from somewhere other than the listing, and both available routes
//! are wrong in a way that reads as a real answer:
//!
//!  * BY ADDRESS RANGE. The only signal a listing alone carries is the magnitude of
//!    the number, and a phased VMA is an ordinary-looking small address. A range
//!    derivation attempted on this tree returned 68000 engine routines at low ROM
//!    addresses (`Flush_VDP_Shadow`, `Set_VDP_Reg`, `VDP_Shadow_Init`) as its
//!    "phased" set. Every one of them is unphased.
//!  * BY RE-PARSING THE SOURCE. `aeon/tools/scene_spans.py::vma_phased_symbol_names`
//!    does this, and its own docstring records why it had to: it scans every `.emp`
//!    file for `section ... (vma: ...)` and collects the top-level `proc`/`data` names
//!    inside. That derivation is BOTH loose and tight against what a given listing
//!    actually contains: measured on the reference tree, 29 of its 36 names reach
//!    neither shipped listing, and it misses a phased interior label that the demo
//!    listing does carry.
//!
//! So this gate charges the ASSEMBLER with answering, from the section that declared
//! the phase, and charges every shipped shape's listing with carrying the answer.
//!
//! WHAT WOULD MAKE THIS GATE VACUOUS, and what stops it. A wiring loss that returned
//! `lma: None` everywhere would leave every shape at `PHASE COUNT 0`, which is a
//! legal listing and would satisfy any per-shape consistency check. So the walk also
//! requires that the shipped set is not UNIFORMLY empty: at least one shape must
//! report a real phased symbol, with its two addresses.

use sigil_harness::native::{self, GameProfile};
use sigil_harness::test_support::reference_tree_for_profile;
use sigil_link::ListingSymbol;

/// The `PHASE` rows of an emitted listing, as `(name, vma, lma)`.
fn emitted_phase_rows(listing: &[ListingSymbol]) -> Vec<(String, u32, u32)> {
    let text = sigil_link::emit_listing(listing);
    text.lines()
        .filter(|l| l.starts_with("PHASE ") && l.contains(" VMA "))
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let hex = |s: &str| u32::from_str_radix(s.trim_start_matches('$'), 16).unwrap();
            (f[1].to_string(), hex(f[3]), hex(f[5]))
        })
        .collect()
}

/// The `PHASE COUNT n` line, which is present on EVERY listing this emitter writes.
fn emitted_count(listing: &[ListingSymbol]) -> Option<usize> {
    sigil_link::emit_listing(listing)
        .lines()
        .find_map(|l| l.strip_prefix("PHASE COUNT "))
        .and_then(|n| n.parse().ok())
}

#[test]
fn every_shipped_shape_declares_which_of_its_addresses_are_phased() {
    let shapes: Vec<(&str, GameProfile)> = native::shipped_shapes();
    assert!(
        !shapes.is_empty(),
        "shipped_shapes() enumerated nothing; a walk over an empty set is green for \
         the one reason that proves nothing"
    );

    let mut faults: Vec<String> = Vec::new();
    let mut total_phased = 0usize;
    let mut report: Vec<String> = Vec::new();

    for (label, profile) in &shapes {
        let Some(aeon) = reference_tree_for_profile(profile) else { return };
        let listing = match native::build_rom_chained_with_listing(&aeon, profile) {
            Ok(b) => b.listing,
            Err(e) => {
                faults.push(format!("shape `{label}`: build failed, so nothing was measured: {e}"));
                continue;
            }
        };

        // 1. The count line is ALWAYS there. Its absence is what an older sigil looks
        //    like, and a reader must never have to tell that apart from "nothing was
        //    phased" by the same evidence.
        let Some(count) = emitted_count(&listing) else {
            faults.push(format!(
                "shape `{label}`: the listing carries no `PHASE COUNT` line. Without it \
                 a consumer cannot tell an unphased build from an assembler that does \
                 not know about phasing, which is the single bit this section exists \
                 to state"
            ));
            continue;
        };

        // 2. The count and the rows are one fact, stated once.
        let rows = emitted_phase_rows(&listing);
        if rows.len() != count {
            faults.push(format!(
                "shape `{label}`: `PHASE COUNT {count}` against {} emitted rows",
                rows.len()
            ));
        }

        // 3. A phased row means what it says: two DIFFERENT addresses, and the row's
        //    VMA is the same value the address views print for that symbol.
        for (name, vma, lma) in &rows {
            if vma == lma {
                faults.push(format!(
                    "shape `{label}`: `{name}` is listed as phased with VMA == LMA \
                     (${vma:08X}); a row that states no difference is noise a consumer \
                     would have to filter"
                ));
            }
            match listing.iter().find(|s| &s.name == name) {
                Some(s) if s.value != *vma => faults.push(format!(
                    "shape `{label}`: `{name}` has address-row value ${:08X} but phase-row \
                     VMA ${vma:08X}; the marker must describe the rows above it, never \
                     restate them differently",
                    s.value
                )),
                None => faults.push(format!(
                    "shape `{label}`: `{name}` has a phase row and no address row"
                )),
                _ => {}
            }
        }

        // 4. A RAM label is never phased. Aeon's RAM blocks reserve address space and
        //    place zero image bytes, so an LMA for one would name a storage address
        //    for bytes that were never stored.
        for s in listing.iter().filter(|s| s.value >= 0xFFFF_0000) {
            if s.lma.is_some() {
                faults.push(format!(
                    "shape `{label}`: RAM label `{}` (${:08X}) claims a storage address",
                    s.name, s.value
                ));
            }
        }

        // 5. An equate has a value, not storage.
        for s in listing.iter().filter(|s| s.is_equate && s.lma.is_some()) {
            faults.push(format!("shape `{label}`: equate `{}` claims a storage address", s.name));
        }

        total_phased += count;
        report.push(format!("{label}: PHASE COUNT {count} {rows:?}"));
    }

    // 6. NON-VACUITY, over the shipped set rather than per shape. Some shapes may
    //    legitimately be unphased; all of them being so is what a lost wiring looks
    //    like, and it would satisfy every check above.
    assert!(
        total_phased > 0 || !faults.is_empty(),
        "no shipped shape reported a single phased symbol. Every check above is \
         satisfied by a derivation that returns `None` for everything, so this is the \
         one that notices the wiring is gone.\nMeasured: {}",
        report.join("\n          ")
    );

    assert!(
        faults.is_empty(),
        "{} phase-marker fault(s):\n  {}\nMeasured: {}",
        faults.len(),
        faults.join("\n  "),
        report.join("\n          ")
    );
}
