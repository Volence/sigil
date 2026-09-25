//! `[sound.bank-id-vs-placement]`: the cartridge bank ids the sound emit bakes into
//! the resident Z80 blob and the `DacSampleTable` head, checked against the banks
//! as the build actually placed them.
//!
//! The emit (`seam1::emit_sound_blob`, `seam2::emit_dac_artifacts`) runs before the
//! chainer places anything. It folds each bank id from the `map.toml` anchors through
//! `seam2::sound_layout`, and the ids land as plain bytes: 8-bit `ld a,n` immediates in
//! the blob (`SND_ENGINE_TABLE_BANK`, `SFX_BLOB_BANK`) and the `ds_bank` byte of each
//! DAC descriptor. The chainer then places the banks by its own walk. A disagreement
//! between the two is a ROM that selects the wrong `$8000` window at run time, and the
//! bytes themselves are well formed, so nothing downstream of the emit sees it.
//!
//! [`validate_sound_bank_ids`] reads each baked byte out of the LINKED image (what
//! ships) and compares it with `seam2::bank_id_of` of the LMA its bank's label was
//! placed at. Where each id sits is derived, never listed: the emitters re-link with
//! the id's input moved and the offsets that changed are the sites
//! ([`crate::seam1::blob_bank_id_sites`], [`crate::seam2::dac_head_bank_id_sites`]).
//!
//! A shape that declares no sound (`GameProfile::sound_on == false`) has nothing to
//! check, and says so only after confirming that its layout really carries no resident
//! blob bytes and no `DacSampleTable`. A sound-on shape in which any label, any site
//! group, or any byte cannot be found is refused: an unmeasured id is not a matching
//! one.

use sigil_ir::Section;
use sigil_link::LinkedImage;
use std::path::Path;

/// The resident blob's bank-id consts, each an 8-bit operand of a `SetBank` call.
pub const BLOB_BANK_ID_CONSTS: &[&str] = &["SND_ENGINE_TABLE_BANK", "SFX_BLOB_BANK"];

/// The placed labels whose bank each blob const must select. `SND_ENGINE_TABLE_BANK`
/// banks the engine-table head section; `SFX_BLOB_BANK` banks both the id-to-blob
/// window table (in that head) and the SFX block itself, so its sites must equal the
/// id of each.
fn blob_const_bank_labels(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "SND_ENGINE_TABLE_BANK" => Some(&["SoundTablesZ80_Head"]),
        "SFX_BLOB_BANK" => Some(&["SfxBlobWinTab", "Sfx_33"]),
        _ => None,
    }
}

/// The resident blob's bracket labels in `engine/system/boot_data.emp`.
const BLOB_START: &str = "Z80_Sound_Start";
const BLOB_END: &str = "Z80_Sound_End";
/// The DAC descriptor head's label in `games/sonic4/data/sound/soundbankhead.emp`.
const DAC_HEAD: &str = "DacSampleTable";
/// The two DAC bank heads in `games/sonic4/data/sound/dac_banks.emp`.
const DAC_BLIP_BANK: &str = "Dac_Temp_Blip";
const DAC_SHARED_BANK: &str = "Dac_SharedBank_Start";

/// The offsets at which `real` and `doc` differ, each required to hold `want_real` in
/// `real` and `want_doc` in `doc`. `doc` is the same artifact re-emitted with one bank
/// id's input moved, so these are the bytes that hold that id. Refused when the two
/// differ in length, when a changed byte holds anything other than the two ids (the
/// input feeds a byte the check cannot interpret), or when nothing changed (the id is
/// not measurable in this artifact).
pub(crate) fn diff_id_sites(
    real: &[u8],
    doc: &[u8],
    want_real: u8,
    want_doc: u8,
    what: &str,
) -> Result<Vec<u32>, String> {
    if want_real == want_doc {
        return Err(format!(
            "[sound.bank-id-vs-placement] {what}: the probe id equals the emitted id ${want_real:02X}, \
             so no site can be told apart"
        ));
    }
    if real.len() != doc.len() {
        return Err(format!(
            "[sound.bank-id-vs-placement] {what}: moving the bank changed the artifact's length \
             ({} to {} bytes), so its bank-id bytes cannot be located by offset",
            real.len(),
            doc.len()
        ));
    }
    let mut sites = Vec::new();
    for (off, (&r, &d)) in real.iter().zip(doc).enumerate() {
        if r == d {
            continue;
        }
        if r != want_real || d != want_doc {
            return Err(format!(
                "[sound.bank-id-vs-placement] {what}: moving the bank changed offset {off:#x} from \
                 ${r:02X} to ${d:02X}, but a bank-id byte would go from ${want_real:02X} to \
                 ${want_doc:02X}. The bank feeds a byte that is not its id, and this check cannot \
                 say what that byte holds"
            ));
        }
        sites.push(off as u32);
    }
    if sites.is_empty() {
        return Err(format!(
            "[sound.bank-id-vs-placement] {what}: no byte depends on this bank id, so the id \
             cannot be measured in this artifact (if the consumer was removed on purpose, \
             remove it from the check too)"
        ));
    }
    Ok(sites)
}

/// One set of baked bank-id bytes and the bank they must select.
#[derive(Debug, Clone)]
pub struct SiteGroup {
    /// The artifact, for the diagnostic (`resident Z80 blob (plain shape)`).
    pub artifact: String,
    /// The placed label the artifact starts at; each offset is relative to it.
    pub base_label: &'static str,
    /// What folded the byte, for the diagnostic (`SND_ENGINE_TABLE_BANK`).
    pub baked_as: String,
    /// The placed labels whose bank id every site must equal.
    pub bank_labels: &'static [&'static str],
    /// Artifact offsets of the bank-id bytes.
    pub offsets: Vec<u32>,
}

/// Compare every site of every group: the byte `read` returns at `placed(base_label) +
/// offset` must equal `bank_id_of(placed(bank_label))` for each of the group's bank
/// labels. Returns the number of bytes compared. Every mismatch is reported, not just
/// the first; a label `placed` cannot find, a byte `read` cannot find, or a group with
/// no offsets is refused.
pub fn check_site_groups(
    groups: &[SiteGroup],
    placed: &dyn Fn(&str) -> Option<u32>,
    read: &dyn Fn(u32) -> Option<u8>,
) -> Result<usize, String> {
    let need = |label: &str| -> Result<u32, String> {
        placed(label).ok_or_else(|| {
            format!(
                "[sound.bank-id-vs-placement] cannot measure: `{label}` is not placed in this \
                 sound-on build, so the bank ids baked against it cannot be checked"
            )
        })
    };
    let mut mismatches = Vec::new();
    let mut compared = 0usize;
    for g in groups {
        if g.offsets.is_empty() {
            return Err(format!(
                "[sound.bank-id-vs-placement] cannot measure: {} has no {} site to check",
                g.artifact, g.baked_as
            ));
        }
        let base = need(g.base_label)?;
        for &bank_label in g.bank_labels {
            let bank_lma = need(bank_label)?;
            let placed_id = crate::seam2::bank_id_of(bank_lma);
            for &off in &g.offsets {
                let at = base + off;
                let baked = read(at).ok_or_else(|| {
                    format!(
                        "[sound.bank-id-vs-placement] cannot measure: {} offset {off:#x} (ROM \
                         {at:#x}) is outside every linked section",
                        g.artifact
                    )
                })?;
                compared += 1;
                if u32::from(baked) != placed_id {
                    mismatches.push(format!(
                        "  {} offset {off:#x} (ROM {at:#x}) bakes {} = ${baked:02X}, but \
                         `{bank_label}` was placed at {bank_lma:#x}, bank ${placed_id:02X}",
                        g.artifact, g.baked_as
                    ));
                }
            }
        }
    }
    if mismatches.is_empty() {
        return Ok(compared);
    }
    Err(format!(
        "[sound.bank-id-vs-placement] {} baked sound bank id(s) disagree with where the build \
         placed the bank. The emit folded these ids from the map.toml `dac_banks`/`sound_bank` \
         anchors (seam2::sound_layout) before placement; the chainer then placed the banks \
         elsewhere, so the driver would select the wrong $8000 window and play from the wrong \
         bank:\n{}\nMake the map anchors and the placed banks agree; do not edit the ids.",
        mismatches.len(),
        mismatches.join("\n")
    ))
}

/// The LMA of `label` in the resolved layout (LMA, not VMA: the sound heads are
/// phased to `$8000`, and a bank id is a property of where the bytes sit in ROM).
fn placed_lma(resolved: &[Section], label: &str) -> Option<u32> {
    resolved.iter().find_map(|sec| {
        sec.labels.iter().find(|l| l.name == label).map(|l| sec.lma + l.offset)
    })
}

/// The byte the linked image holds at ROM address `lma`.
fn linked_byte(linked: &LinkedImage, lma: u32) -> Option<u8> {
    linked.sections.iter().find_map(|s| {
        let off = lma.checked_sub(s.lma)? as usize;
        s.bytes.get(off).copied()
    })
}

/// `[sound.bank-id-vs-placement]` over one built shape. Returns the number of baked
/// bytes compared, which is zero only for a shape that declares no sound and whose
/// layout confirms it. See the module doc for what is compared and how the sites and
/// the expected ids are derived.
pub fn validate_sound_bank_ids(
    aeon: &Path,
    resolved: &[Section],
    linked: &LinkedImage,
    sound_on: bool,
    debug: bool,
) -> Result<usize, String> {
    let placed = |label: &str| placed_lma(resolved, label);
    if !sound_on {
        let blob_len = match (placed(BLOB_START), placed(BLOB_END)) {
            (Some(s), Some(e)) => e.saturating_sub(s),
            _ => 0,
        };
        if blob_len != 0 || placed(DAC_HEAD).is_some() {
            return Err(format!(
                "[sound.bank-id-vs-placement] this shape declares sound off, yet its layout \
                 carries sound bank-id bytes (resident blob {blob_len} bytes, `{DAC_HEAD}` \
                 {}). Their ids would go unchecked; the shape's sound flag and its registry \
                 disagree",
                if placed(DAC_HEAD).is_some() { "placed" } else { "absent" }
            ));
        }
        return Ok(0);
    }

    let crate::seam1::BlobBankIdSites { by_const: blob_sites, blob_len } =
        crate::seam1::blob_bank_id_sites(aeon, debug)?;
    let placed_blob_len = match (placed(BLOB_START), placed(BLOB_END)) {
        (Some(s), Some(e)) => e.saturating_sub(s),
        _ => {
            return Err(format!(
                "[sound.bank-id-vs-placement] cannot measure: `{BLOB_START}`/`{BLOB_END}` are not \
                 both placed in this sound-on build"
            ))
        }
    };
    if placed_blob_len as usize != blob_len {
        return Err(format!(
            "[sound.bank-id-vs-placement] cannot measure: the placed resident blob spans \
             {placed_blob_len} bytes but the emitted blob is {blob_len}, so its bank-id offsets \
             do not locate bytes in the ROM"
        ));
    }
    let shape = if debug { "debug" } else { "plain" };
    let mut groups = Vec::new();
    for (name, offsets) in blob_sites {
        let bank_labels = blob_const_bank_labels(name).ok_or_else(|| {
            format!("[sound.bank-id-vs-placement] blob const {name} names no bank label to check")
        })?;
        groups.push(SiteGroup {
            artifact: format!("resident Z80 blob ({shape} shape)"),
            base_label: BLOB_START,
            baked_as: name.to_string(),
            bank_labels,
            offsets,
        });
    }
    let dac = crate::seam2::dac_head_bank_id_sites(aeon)?;
    groups.push(SiteGroup {
        artifact: "DacSampleTable head".to_string(),
        base_label: DAC_HEAD,
        baked_as: "ds_bank (dac_blip_bank)".to_string(),
        bank_labels: &[DAC_BLIP_BANK],
        offsets: dac.blip,
    });
    groups.push(SiteGroup {
        artifact: "DacSampleTable head".to_string(),
        base_label: DAC_HEAD,
        baked_as: "ds_bank (dac_shared_bank)".to_string(),
        bank_labels: &[DAC_SHARED_BANK],
        offsets: dac.shared,
    });
    check_site_groups(&groups, &placed, &|lma| linked_byte(linked, lma))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(offsets: Vec<u32>) -> SiteGroup {
        SiteGroup {
            artifact: "blob".into(),
            base_label: "Base",
            baked_as: "ID".into(),
            bank_labels: &["Bank"],
            offsets,
        }
    }

    /// A ROM of `len` zero bytes with `id` written at each of `at`.
    fn rom(len: usize, at: &[u32], id: u8) -> Vec<u8> {
        let mut r = vec![0u8; len];
        for &a in at {
            r[a as usize] = id;
        }
        r
    }

    const BASE: u32 = 0x100;
    const BANK: u32 = 0xB8000;

    fn placed(l: &str) -> Option<u32> {
        match l {
            "Base" => Some(BASE),
            "Bank" => Some(BANK),
            _ => None,
        }
    }

    #[test]
    fn matching_ids_pass_and_count_every_byte() {
        let id = crate::seam2::bank_id_of(BANK) as u8;
        let r = rom(0x200, &[BASE + 2, BASE + 9], id);
        let n = check_site_groups(&[group(vec![2, 9])], &placed, &|a| r.get(a as usize).copied());
        assert_eq!(n, Ok(2));
    }

    #[test]
    fn a_moved_bank_is_refused_naming_baked_and_placed() {
        let stale = crate::seam2::bank_id_of(BANK - 0x10000) as u8;
        let placed_id = crate::seam2::bank_id_of(BANK);
        assert_ne!(u32::from(stale), placed_id);
        let r = rom(0x200, &[BASE + 2], stale);
        let e = check_site_groups(&[group(vec![2])], &placed, &|a| r.get(a as usize).copied())
            .unwrap_err();
        assert!(e.contains("[sound.bank-id-vs-placement]"), "{e}");
        assert!(e.contains(&format!("= ${stale:02X}")), "{e}");
        assert!(e.contains(&format!("bank ${placed_id:02X}")), "{e}");
        assert!(e.contains("`Bank`"), "{e}");
    }

    #[test]
    fn an_unplaced_label_or_empty_group_or_unreadable_byte_is_refused() {
        let r = rom(0x200, &[], 0);
        let read = |a: u32| r.get(a as usize).copied();
        let mut g = group(vec![2]);
        g.bank_labels = &["Missing"];
        assert!(check_site_groups(&[g], &placed, &read).unwrap_err().contains("cannot measure"));
        assert!(check_site_groups(&[group(vec![])], &placed, &read)
            .unwrap_err()
            .contains("cannot measure"));
        assert!(check_site_groups(&[group(vec![0x400])], &placed, &read)
            .unwrap_err()
            .contains("cannot measure"));
    }

    #[test]
    fn diff_sites_are_the_changed_id_bytes_only() {
        let real = [0x3E, 0x17, 0xCD, 0x00, 0x3E, 0x17];
        let doc = [0x3E, 0xE8, 0xCD, 0x00, 0x3E, 0xE8];
        assert_eq!(diff_id_sites(&real, &doc, 0x17, 0xE8, "t"), Ok(vec![1, 5]));
        // A changed byte that is not the id itself cannot be interpreted.
        let odd = [0x3E, 0xE8, 0xCD, 0x01, 0x3E, 0xE8];
        assert!(diff_id_sites(&real, &odd, 0x17, 0xE8, "t").is_err());
        // Nothing changed: the id is not measurable here.
        assert!(diff_id_sites(&real, &real, 0x17, 0xE8, "t").unwrap_err().contains("no byte"));
        // A length change means offsets no longer line up.
        assert!(diff_id_sites(&real, &doc[..5], 0x17, 0xE8, "t").is_err());
    }
}
