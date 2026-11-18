//! The exporter's entry: what a driver hands over after a build (`ExportInputs`),
//! and `build_export`, which turns it plus the ROM's identity into a `BuildExport`.
//! Driver-independent on purpose: the driver supplies placed sections, a span
//! locator and its side tables; nothing here knows a front end.

use super::locate::SpanLocator;
use super::{
    BuildExport, ConstRecord, ExportError, FileRecord, LayoutRecord, LineTable, RomRecord, ShapeRecord, Space,
    SpaceRecord, SymbolRecord, ToolRecord,
};
use sigil_ir::Section;

/// The Z80 sound driver's bridge facts (bank windows). Empty until the task that
/// fills it (G0a Task 9).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Z80Bridge {}

/// Tables the front ends know and the placed sections do not. Empty fields are legal
/// until the tasks that fill them land.
#[derive(Clone, Debug, Default)]
pub struct SideTables {
    pub symbols: Vec<SymbolRecord>,
    pub layouts: Vec<LayoutRecord>,
    pub consts: Vec<ConstRecord>,
    pub z80: Option<Z80Bridge>,
}

/// Everything a build keeps for its export, taken before the sections are consumed.
pub struct ExportInputs {
    /// The placed, post-resolve sections the ROM was linked from.
    pub sections: Vec<Section>,
    /// Maps those sections' spans to source lines.
    pub locator: Box<dyn SpanLocator + Send>,
    pub side: SideTables,
}

/// Assemble the export for `rom` (the bytes as installed).
///
/// `inputs_sha256` is left zero: it is a hash of the build's read set, which the
/// driver owns and this entry does not see, so the caller sets it before rendering.
pub fn build_export(
    inputs: &ExportInputs,
    rom: &RomRecord,
    tool: ToolRecord,
    shape: ShapeRecord,
    files: Vec<FileRecord>,
) -> Result<BuildExport, ExportError> {
    Ok(BuildExport {
        rom: rom.clone(),
        inputs_sha256: [0; 32],
        tool,
        shape,
        spaces: vec![
            SpaceRecord { id: Space::M68k, addr_bits: 24 },
            SpaceRecord { id: Space::Z80, addr_bits: 16 },
        ],
        files,
        symbols: inputs.side.symbols.clone(),
        layouts: inputs.side.layouts.clone(),
        consts: inputs.side.consts.clone(),
        lines: LineTable::default(),
    })
}
