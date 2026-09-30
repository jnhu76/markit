//! Markit Composition Kernel (K0): the generic control-plane kernel.
//!
//! > **Kernel controls reachability, ownership and lifetime; it should not
//! > own application payloads.**
//!
//! Primitive budget is exactly five (design §D, §R):
//!
//! ```text
//! Context  Capability  Fiber  Effect  Reconcile
//! ```
//!
//! This crate is generic. It must not — and structurally cannot — know any
//! product, media, platform, or UI payload semantics: it depends on no
//! markit-* crate and nothing from the research area (see
//! `tests/dependency_firewall.rs` and the donor implementation ADR, D1).
//!
//! Semantic authority: the donor design documents
//! `composition-kernel-0-design.md` and
//! `composition-kernel-0-implementation-adr.md` in the qianqian donor
//! repository at the commit recorded in `PROVENANCE.md`. All `§…` section
//! citations in this crate refer to that donor design document; all `ADR
//! D…`/`B…` citations refer to that donor implementation ADR and its
//! reviews.
//!
//! Control plane is synchronous and serialized; the realtime firewall (§N)
//! forbids every kernel operation on the realtime hot path. Payload flows
//! through pre-bound data edges, never through the kernel.

mod capability;
mod component;
mod context;
mod desired;
mod diagnostic;
mod fiber;
mod kernel;

pub use capability::{Capability, CapabilityKey};
pub use component::{ActivationError, ComponentRegistrationError, ComponentSpec, Discharge};
pub use context::{ActivationCtx, Binding, ResolveError, TeardownCtx};
pub use desired::{CompositionError, CompositionErrors, DesiredEntry, Revision};
pub use diagnostic::{CompositionSnapshot, FiberDiagnostic, RelationDiagnostic};
pub use fiber::FiberState;
pub use kernel::{CompositionKernel, DisposeVerdict, EffectHandle, StepOutcome};
