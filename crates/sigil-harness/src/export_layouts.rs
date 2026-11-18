//! The build export's layouts and constants (G0a spec 4.5), from the `.emp` front end's
//! registry ([`sigil_frontend_emp::export_side::ProgramLayouts`]) mapped to the export
//! model's records.
//!
//! The registry lives in the front end and says what the language resolved; the records
//! are the linker crate's types, and the crate graph (`sigil-cli/tests/crate_graph.rs`)
//! keeps the front end off `sigil-link`, so the mapping is here, in the driver, as
//! `export_symbols` does for symbols. The front end's refusals map one to one onto
//! [`ExportError`]: `TwoLayouts`, `TwoConsts`, `ConstOutOfRange` and `Layout`.

use sigil_frontend_emp::export_side::{
    emp_consts, emp_layouts, EmpExportError, EmpField, EmpLayoutKind, ProgramLayouts,
};
use sigil_link::export::{ConstRecord, ExportError, FieldRecord, LayoutKind, LayoutRecord, OverlayRecord};

fn error(e: EmpExportError) -> ExportError {
    match e {
        EmpExportError::TwoLayouts { module, name } => ExportError::TwoLayouts { module, name },
        EmpExportError::TwoConsts { module, name } => ExportError::TwoConsts { module, name },
        EmpExportError::ConstOutOfRange { module, name, value } => {
            ExportError::ConstOutOfRange { module, name, value: value.to_string() }
        }
        EmpExportError::Layout { module, name, why } => ExportError::Layout { module, name, why },
    }
}

fn field(f: &EmpField) -> FieldRecord {
    FieldRecord { name: f.name.clone(), offset: f.offset, size: f.size, type_: f.ty.clone() }
}

/// The `.emp` program's layouts as export records. A `(module, name)` that resolved to
/// two layouts is [`ExportError::TwoLayouts`].
pub fn export_layouts(reg: &ProgramLayouts) -> Result<Vec<LayoutRecord>, ExportError> {
    Ok(emp_layouts(reg)
        .map_err(error)?
        .into_iter()
        .map(|l| LayoutRecord {
            name: l.name,
            module: l.module,
            kind: match l.kind {
                EmpLayoutKind::Struct => LayoutKind::Struct,
                EmpLayoutKind::Bitfield => LayoutKind::Bitfield,
                EmpLayoutKind::Enum => LayoutKind::Enum,
            },
            size: l.size,
            fields: l.fields.iter().map(field).collect(),
            overlays: l
                .overlays
                .iter()
                .map(|o| OverlayRecord {
                    name: o.name.clone(),
                    base: o.base.clone(),
                    offset: o.offset,
                    size: o.size,
                    fields: o.fields.iter().map(field).collect(),
                })
                .collect(),
        })
        .collect())
}

/// The `.emp` program's integer, boolean and enum constants as export records. A
/// `(module, name)` that resolved to two values is [`ExportError::TwoConsts`]; a value
/// past `i64` is [`ExportError::ConstOutOfRange`].
pub fn export_consts(reg: &ProgramLayouts) -> Result<Vec<ConstRecord>, ExportError> {
    Ok(emp_consts(reg)
        .map_err(error)?
        .into_iter()
        .map(|c| ConstRecord { name: c.name, module: c.module, type_: c.ty, value: c.value })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigil_frontend_emp::export_side::{EmpLayout, EmpLayoutKind};
    use sigil_frontend_emp::lower::LowerOptions;
    use sigil_frontend_emp::resolve::{build_program_recorded, manifest::Manifest};

    fn layout(size: u32) -> EmpLayout {
        EmpLayout {
            name: "Sst".into(),
            module: "engine.objects.sst".into(),
            kind: EmpLayoutKind::Struct,
            size,
            fields: Vec::new(),
            overlays: Vec::new(),
        }
    }

    #[test]
    fn two_layouts_reaches_the_export_as_its_own_error() {
        let mut reg = ProgramLayouts::default();
        reg.record_layout(layout(80));
        assert_eq!(export_layouts(&reg).unwrap().len(), 1);
        reg.record_layout(layout(82));
        assert_eq!(
            export_layouts(&reg),
            Err(ExportError::TwoLayouts { module: "engine.objects.sst".into(), name: "Sst".into() })
        );
    }

    #[test]
    fn a_constant_past_i64_reaches_the_export_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        for (rel, src) in [
            ("app/big.emp", "module app.big\npub const HUGE = 1 << 63\n"),
            ("app/main.emp", "module app.main\nuse app.big.{HUGE}\npub proc main () {\n    rts\n}\n"),
        ] {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, src).unwrap();
        }
        let (manifest, _) = Manifest::scan(dir.path());
        let opts = LowerOptions {
            initial_cpu: sigil_ir::backend::Cpu::M68000,
            include_root: None,
            embed_base: None,
            defines: vec![],
        };
        let built = build_program_recorded(&manifest, "app.main", None, &opts);
        let err = export_consts(&built.layouts).unwrap_err();
        assert_eq!(
            err,
            ExportError::ConstOutOfRange { module: "app.big".into(), name: "HUGE".into(), value: "9223372036854775808".into() }
        );
        assert!(err.to_string().contains("app.big::HUGE"), "{err}");
    }
}
