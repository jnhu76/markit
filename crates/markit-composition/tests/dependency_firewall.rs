//! Dependency firewall (donor implementation ADR D1, adapted for Markit):
//! the generic kernel must not depend on any product crate, research
//! mechanism, or repository area outside this crate. Enforced at compile
//! time via the manifest, and mechanically via a source-tree scan.
//!
//! The deny-list vocabulary below (donor media-domain words, donor product
//! crate names, Markit research/UI mechanism names) intentionally appears
//! ONLY inside firewall deny-lists and in PROVENANCE.md — the scan targets
//! are production sources and the manifest, where a hit would be an active
//! code dependency or contamination, not donor documentation.

use markit_composition::{ComponentSpec, CompositionKernel, DesiredEntry, Revision};

/// The kernel's own Cargo.toml, embedded at compile time.
const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn kernel_crate_has_no_product_dependencies() {
    // The [dependencies] table must be literally empty: no product crate,
    // no research crate, no path dependency, nothing (donor ADR D1 — the
    // kernel is generic, std only).
    let after_deps = MANIFEST
        .split("[dependencies]")
        .nth(1)
        .expect("[dependencies] table present");
    let next_table = after_deps.find('[').unwrap_or(after_deps.len());
    for line in after_deps[..next_table].lines() {
        let line = line.split('#').next().unwrap().trim();
        assert!(
            line.is_empty(),
            "kernel manifest must keep an EMPTY [dependencies] table \
             (donor ADR D1): unexpected entry {line:?}"
        );
    }
    for forbidden in [
        "markit-core",
        "markit-common",
        "markit-utils",
        "markit-shared",
        "markit-markdown",
        "markit-workbench",
        "markit-document",
        "markit-platform",
        "[dependencies.markit",
        "[dependencies.qianqian",
    ] {
        assert!(
            !MANIFEST.contains(forbidden),
            "kernel manifest must not reference '{forbidden}': the generic kernel \
             depends on no product crate (dependency firewall; donor ADR D1)"
        );
    }
}

/// Repository-boundary firewall: production sources and the manifest must
/// carry no reference to the research area, donor product crates, or
/// Markdown/UI/platform mechanisms. PROVENANCE.md is deliberately exempt —
/// it is the donor provenance record and is expected to name the donor
/// repository and the excluded product domains.
#[test]
fn kernel_sources_reference_no_repository_boundaries() {
    const SOURCES: &[(&str, &str)] = &[
        ("lib.rs", include_str!("../src/lib.rs")),
        ("capability.rs", include_str!("../src/capability.rs")),
        ("component.rs", include_str!("../src/component.rs")),
        ("context.rs", include_str!("../src/context.rs")),
        ("desired.rs", include_str!("../src/desired.rs")),
        ("diagnostic.rs", include_str!("../src/diagnostic.rs")),
        ("fiber.rs", include_str!("../src/fiber.rs")),
        ("kernel.rs", include_str!("../src/kernel.rs")),
        ("kernel_verify.rs", include_str!("../src/kernel_verify.rs")),
        ("Cargo.toml", include_str!("../Cargo.toml")),
    ];
    // Case-sensitive tokens that must never appear in production sources:
    // the research area and its mechanisms, donor product crates and media
    // domain, and Markit UI/platform/Markdown surfaces.
    const FORBIDDEN: &[&str] = &[
        "research/",
        "benchmark",
        "Benchmark",
        "horse",
        "Horse",
        "H0",
        "H1",
        "H2",
        "H3",
        "H4",
        "GPUI",
        "Electron",
        "Mermaid",
        "KaTeX",
        "markdown",
        "Markdown",
        "parser",
        "qianqian-playback",
        "qianqian-app",
        "qianqian-audio",
        "qianqian-output",
        "qianqian-decode",
        "qianqian-songcore",
        "songcore",
        "playback",
        "Playback",
        "audio",
        "Audio",
        "PCM",
        "pcm",
        "WASAPI",
        "CoreAudio",
        "decoder",
        "Decoder",
        "track",
        "Track",
        "music",
        "Music",
        "song",
        "Song",
        "lyrics",
        "spectrum",
        "SinkSession",
        "PcmSink",
        "device session",
        "seek",
        "pause",
        "volume",
    ];
    for (file, src) in SOURCES {
        for token in FORBIDDEN {
            assert!(
                !src.contains(token),
                "kernel source {file} references '{token}' — a repository-boundary or \
                 domain-vocabulary violation (dependency firewall)"
            );
        }
    }
}

/// Stage-2 smoke proof (from the donor): a generic kernel that knows
/// nothing about any domain can host components end to end.
#[test]
fn minimal_kernel_hosts_an_anonymous_component() {
    let mut kernel = CompositionKernel::new();
    kernel
        .register_component(ComponentSpec::new("anonymous"))
        .expect("component registered");
    kernel
        .set_desired(vec![DesiredEntry::enabled(
            "a",
            "anonymous",
            Revision::new(1),
        )])
        .expect("legal desired composition");
    kernel.settle();

    let snap = kernel.snapshot();
    assert!(snap.quiet);
    assert_eq!(
        snap.fibers.get("a").map(|f| f.state),
        Some(markit_composition::FiberState::Active)
    );
}
