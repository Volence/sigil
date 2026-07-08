//! Cross-module resolution driver (Spec 2 §3): gather modules, resolve
//! `use`/prelude names, place items, and produce one linkable Vec<Section>.
pub mod imports;
pub mod manifest;
