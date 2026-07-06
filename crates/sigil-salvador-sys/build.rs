fn main() {
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
