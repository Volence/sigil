fn main() {
    // Track the whole vendored tree: emitting any `rerun-if-changed` disables
    // cargo's default whole-package tracking, so without this a vendored *header*
    // edit (shrink.h/format.h/divsufsort_config.h/...) would not trigger a rebuild.
    println!("cargo:rerun-if-changed=vendor");
    let mut b = cc::Build::new();
    b.include("vendor").include("vendor/libdivsufsort/include");
    for f in [
        "vendor/shrink.c",
        "vendor/matchfinder.c",
        "vendor/expand.c",
        "vendor/libdivsufsort/lib/divsufsort.c",
        "vendor/libdivsufsort/lib/divsufsort_utils.c",
        "vendor/libdivsufsort/lib/sssort.c",
        "vendor/libdivsufsort/lib/trsort.c",
    ] {
        b.file(f);
        println!("cargo:rerun-if-changed={f}");
    }
    b.warnings(false).compile("salvador");
}
