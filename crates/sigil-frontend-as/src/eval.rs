//! eval: the multi-pass driver — directive dispatch, expansion, fixpoint, emit.

use crate::Options;
use sigil_ir::Module;
use sigil_span::Diagnostic;

/// Run the front-end over `src`. Stub: returns an empty module (grown in later tasks).
pub fn run(_src: &str, _opts: &Options) -> Result<Module, Vec<Diagnostic>> {
    Ok(Module { sections: Vec::new() })
}
