# Provenance

These four 256-byte golden files:

- `rocking_a20_p64.bin`
- `ojz_calm_a96_p64.bin`
- `haze_a16_p64.bin`
- `shimmer_a8_p32.bin`

were copied verbatim from `crates/sigil-frontend-as/tests/vectors/sine_goldens/`.
They were originally captured from real `asl`/Aeon builds and are used as the
reference sine-table goldens for the Plan-5 T4 `as.*` capability gate.

They are inert in this crate until T4 wires up the corresponding `as.*` test
harness in `sigil-frontend-emp`; committed here early (as part of Task 0,
`sigil-salvador-sys`) purely to make them available without a second
round-trip through the vendored `.emp`/`asl` pipeline.
