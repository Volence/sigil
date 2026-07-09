fn main() {
    // Track the whole vendored tree: emitting any `rerun-if-changed`
    // disables cargo's default whole-package tracking, so without this a
    // vendored *header* edit (common.h, clowncommon/clowncommon.h, ...)
    // would not trigger a rebuild.
    println!("cargo:rerun-if-changed=vendor");

    cc::Build::new()
        .file("vendor/compress.c")
        .file("vendor/decompress.c")
        .file("vendor/common-internal.c")
        .include("vendor")
        .warnings(false)
        .compile("clownnemesis");
}
