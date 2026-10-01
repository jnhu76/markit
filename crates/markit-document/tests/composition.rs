//! The `document` composition role: an independent document
//! capability/lifecycle admitted through K0 like any other plugin, while
//! the concrete documents it creates stay plain domain objects whose hot
//! path never re-enters the kernel after binding.

use std::cell::RefCell;
use std::rc::Rc;

use markit_composition::{
    ActivationError, ComponentSpec, CompositionKernel, DesiredEntry, DisposeVerdict, Revision,
};
use markit_document::{
    DocumentStoreCapability, SourceEdit, SourceOffset, SourceRange, document_plugin,
};

/// The plugin provides the stable document capability; a consumer resolves
/// it once during activation and then drives real documents through the
/// already-bound face — no per-edit composition dispatch.
#[test]
fn document_plugin_binds_through_k0_and_documents_flow_through_the_bound_face() {
    let mut kernel = CompositionKernel::new();
    kernel
        .register_component(document_plugin())
        .expect("the document role registers once");

    let observed: Rc<RefCell<Option<(u64, u64, String)>>> = Rc::new(RefCell::new(None));
    let probe = {
        let observed = observed.clone();
        ComponentSpec::new("document_probe")
            .requires::<DocumentStoreCapability>()
            .on_activate(move |ctx| {
                let store = ctx
                    .resolve::<DocumentStoreCapability>()
                    .map_err(|e| ActivationError::new(format!("{e:?}")))?;
                let store = store.service();

                let id = store.create("# Tdoc\n");
                let mut doc = store.get_mut(id).expect("just created");
                let base = doc.revision();
                let rev = doc
                    .commit(SourceEdit::replace(
                        id,
                        base,
                        SourceRange::new(SourceOffset::new(2), SourceOffset::new(6)),
                        "Bound".to_owned(),
                    ))
                    .map_err(|e| ActivationError::new(format!("{e:?}")))?;
                *observed.borrow_mut() = Some((id.get(), rev.get(), doc.source().to_owned()));
                Ok(())
            })
    };
    kernel.register_component(probe).expect("probe registers");

    kernel
        .set_desired(vec![
            DesiredEntry::enabled("document", "document", Revision::fresh()),
            DesiredEntry::enabled("probe", "document_probe", Revision::fresh()),
        ])
        .expect("legal composition");
    kernel.settle();

    let observed = observed.borrow().clone().expect("the probe committed through the bound document face");
    assert_eq!(observed, (1, 1, "# Bound\n".to_owned()));
}

/// Closing the store retires with the fiber; disposal is clean.
#[test]
fn document_plugin_disposal_is_clean() {
    let mut kernel = CompositionKernel::new();
    kernel.register_component(document_plugin()).expect("legal");
    kernel
        .set_desired(vec![DesiredEntry::enabled(
            "document",
            "document",
            Revision::fresh(),
        )])
        .expect("legal");
    kernel.settle();

    assert_eq!(kernel.dispose_root(), DisposeVerdict::Discharged);
}
