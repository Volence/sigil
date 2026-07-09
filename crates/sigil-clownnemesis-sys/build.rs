fn main() {
    // Track the whole vendored tree: emitting any `rerun-if-changed`
    // disables cargo's default whole-package tracking, so without this a
    // vendored *header* edit (common.h, clowncommon/clowncommon.h, ...)
    // would not trigger a rebuild.
    println!("cargo:rerun-if-changed=vendor");

    // Every compiled file here is verbatim vendored C (there is no
    // Sigil-authored C in this crate), so `warnings(false)` is scoped to
    // exactly the vendored code — upstream's warnings are not ours to fix.
    cc::Build::new()
        .file("vendor/compress.c")
        .file("vendor/decompress.c")
        .file("vendor/common-internal.c")
        .include("vendor")
        .warnings(false)
        .compile("clownnemesis");
}
