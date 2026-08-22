//! Markit product application.
//!
//! Default binary: the markit-core seam demo (no UI substrate; P0-03 lands the
//! first real GPUI slice). With the `g0-probe` feature, `--g0-probe` runs the
//! G0 baseline probe for the pinned GPUI revision (docs/product/g0-gpui-baseline.md).

mod core_demo;

#[cfg(feature = "g0-probe")]
mod g0_probe;

fn main() {
    let args: Vec<String> = std::env::args().collect();
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
    core_demo::run();
}
