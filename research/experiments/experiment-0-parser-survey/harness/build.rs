// Research-only build: compiles the vendored MD4C 0.5.3 C sources
// (vendor/md4c/, ISC license, provenance in vendor/md4c/PROVENANCE.md)
// into the survey harness for the M0-B full-parse baseline.

fn main() {
    let mut build = cc::Build::new();
    build
        .file("vendor/md4c/md4c.c")
        .file("vendor/md4c/entity.c")
        .include("vendor/md4c");
    build.compile("md4c");
    println!("cargo:rerun-if-changed=vendor/md4c/md4c.c");
    println!("cargo:rerun-if-changed=vendor/md4c/entity.c");
    println!("cargo:rerun-if-changed=vendor/md4c/md4c.h");
}
