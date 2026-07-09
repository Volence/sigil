fn main() {
    // Track the whole vendored tree and the shim: emitting any
    // `rerun-if-changed` disables cargo's default whole-package tracking,
    // so without these a vendored *header* edit (kosinski.h, bitfield.h,
    // ...) or a shim.cpp edit would not trigger a rebuild.
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=src/shim.cpp");

    // Vendored C: warnings are upstream's business, silence them.
    cc::Build::new()
        .file("vendor/compressors/clownlzss.c")
        .include("vendor")
        .warnings(false)
        .compile("clownlzss_core");

    // Sigil-authored shim: compile WITH warnings visible (it must stay
    // clean; only the vendored file above is exempt).
    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .file("src/shim.cpp")
        .include("vendor")
        .compile("clownlzss_shim");
}
