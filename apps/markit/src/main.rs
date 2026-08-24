//! Markit product application.
//!
//! Default (P0-03): the first real GPUI editor slice — keystroke ->
//! EditTransaction -> Document/MarkdownState -> visible styled Markdown.
//! `--core-demo` keeps the markit-core seam demo as a headless
//! diagnostic; `--g0-probe` (feature `g0-probe`) keeps the G0 baseline
//! probe as diagnostic instrumentation (docs/product/g0-gpui-baseline.md).

mod core_demo;

#[cfg(feature = "editor")]
mod editor_slice;

#[cfg(feature = "g0-probe")]
mod g0_probe;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--core-demo") {
        core_demo::run();
        return;
    }
    let wants_probe = args.iter().any(|a| a == "--g0-probe");
    #[cfg(feature = "g0-probe")]
    if wants_probe {
        g0_probe::run(&args);
        return;
    }
    #[cfg(not(feature = "g0-probe"))]
    if wants_probe {
        eprintln!("binary built without the g0-probe feature; rebuild with --features g0-probe");
        std::process::exit(2);
    }
    #[cfg(feature = "editor")]
    editor_slice::run();
    #[cfg(not(feature = "editor"))]
    {
        eprintln!("binary built without the editor feature; rebuild with --features editor");
        core_demo::run();
    }
}
