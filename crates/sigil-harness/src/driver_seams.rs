//! The harness side of the new driver's [`sigil_build::driver::Seams`]: the sound seams
//! (seam 1's resident link, seam 2's fold, the bank id constants) and the build-data side
//! tables, which still live here. The old driver calls the same functions directly.

use std::any::Any;
use std::path::Path;

use sigil_build::driver::{Seams, SideInputs};
use sigil_build::map_placement::AnchorOverlay;
use sigil_ir::Section;
use sigil_link::export::SideTables;

/// The harness's seams, for `sigil_build::driver::BuildOptions::seams`.
pub struct HarnessSeams;

impl Seams for HarnessSeams {
    fn emit_sound(&self, checkout: &Path, overlay: Option<&AnchorOverlay>, debug: bool) -> Result<Box<dyn Any>, String> {
        Ok(Box::new(crate::native::emit_generated_in(checkout, overlay)?.for_shape(debug)))
    }

    fn validate_sound(
        &self,
        checkout: &Path,
        overlay: Option<&AnchorOverlay>,
        resolved: &[Section],
        linked: &sigil_link::LinkedImage,
        sound_on: bool,
        debug: bool,
    ) -> Result<(), String> {
        crate::sound_bank_ids::validate_sound_bank_ids_in(checkout, overlay, resolved, linked, sound_on, debug)?;
        if sound_on {
            let layout = crate::seam2::sound_layout_in(checkout, overlay)?;
            crate::native::check_sound_fold(resolved, Some(&layout.folded_heads(debug)))?;
        } else {
            crate::native::check_sound_fold(resolved, None)?;
        }
        Ok(())
    }

    fn side_tables(&self, i: SideInputs<'_>) -> Result<SideTables, String> {
        let symbols = crate::export_symbols::export_symbols(
            i.emp_symbols,
            i.as_symbols,
            i.define_rows,
            i.resolved,
            i.stubs,
            i.z80_windows,
        )?;
        let layouts = crate::export_layouts::export_layouts(i.emp_layouts).map_err(|e| format!("build export: {e}"))?;
        let consts = crate::export_layouts::export_consts(i.emp_layouts).map_err(|e| format!("build export: {e}"))?;
        let z80 = match i.resident {
            Some(any) => {
                let link = any
                    .downcast::<crate::seam1::ResidentLink>()
                    .map_err(|_| "build export: the resident handle is not seam 1's ResidentLink".to_string())?;
                let lma = crate::export_z80::embed_site(i.resolved, &link.bytes).map_err(|e| format!("build export: {e}"))?;
                let at = lma as usize..lma as usize + link.bytes.len();
                if i.rom.get(at.clone()) != Some(&link.bytes[..]) {
                    return Err(format!(
                        "build export: the ROM's bytes at {:#X}..{:#X} are not the resident driver blob seam 1 linked",
                        at.start, at.end
                    ));
                }
                Some(crate::export_z80::z80_bridge(&link, lma)?)
            }
            None => None,
        };
        Ok(SideTables { symbols, layouts, consts, code: i.code, z80 })
    }
}
