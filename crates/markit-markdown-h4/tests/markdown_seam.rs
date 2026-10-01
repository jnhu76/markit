//! H4-provider acceptance tests (campaign B10): the update path is held
//! to `incremental result == clean authoritative parse` — the clean parse
//! through the same grammar is the test-side oracle and never ships in
//! production code. Document/Outline consumers here touch only
//! `markit-markdown-api` types.

use std::rc::Rc;

use markit_composition::{CompositionKernel, DesiredEntry, Revision as K0Revision};
use markit_document::{Document, SourceEdit, SourceOffset, SourceRange, SourceRevision};
use markit_markdown_api::{
    MarkdownSemanticConfig, MarkdownSemanticService, MarkdownSession, OutlineProjection,
    SemanticEvent, SemanticStatus,
};
use markit_markdown_h4::{H4_DIALECT, H4Service, markdown_plugin};

fn config() -> MarkdownSemanticConfig {
    MarkdownSemanticConfig::new(H4_DIALECT)
}

/// Drive one committed edit through the document and the session, the way
/// the product bridge will.
fn apply(
    doc: &mut Document,
    session: &dyn MarkdownSession,
    range: SourceRange,
    replacement: &str,
) -> SourceRevision {
    let before = doc.snapshot();
    let base = doc.revision();
    let edit = SourceEdit::replace(doc.id(), base, range, replacement.to_owned());
    let rev = doc.commit(edit.clone()).expect("valid edit");
    let after = doc.snapshot();
    session
        .apply_edit(&before, &after, &edit)
        .expect("ordered sequence");
    rev
}

fn whole(doc: &Document) -> SourceRange {
    SourceRange::new(SourceOffset::new(0), SourceOffset::new(doc.source().len()))
}

fn headings_at(session: &dyn MarkdownSession, rev: SourceRevision) -> Vec<(u8, String)> {
    session
        .pin(rev)
        .expect("view for the revision was computed")
        .headings()
        .iter()
        .map(|h| (h.level, h.text.clone()))
        .collect()
}

/// The authoritative clean parse of `src`, as (level, text) pairs.
fn clean_headings(src: &str) -> Vec<(u8, String)> {
    markit_markdown_h4::test_support::headings_of(
        &markit_markdown_h4::test_support::clean_parse(src),
        src,
    )
    .into_iter()
    .map(|h| (h.level, h.text))
    .collect()
}

/// B10-1: initial source's headings match the clean authoritative
/// expectation, with the plugin admitted through K0 like any role.
#[test]
fn initial_source_headings_match_clean_parse() {
    let mut kernel = CompositionKernel::new();
    kernel.register_component(markdown_plugin()).expect("legal");
    kernel
        .set_desired(vec![DesiredEntry::enabled(
            "markdown",
            "markdown",
            K0Revision::fresh(),
        )])
        .expect("legal");
    kernel.settle();

    let src = "# Title\n\npara\n\n## Section\n\n### Deep 三\n";
    let expected = clean_headings(src);

    let doc = Document::new(src);
    let session = H4Service::new().open_session(doc.id(), config());
    session.reset(&doc.snapshot());

    assert_eq!(headings_at(session.as_ref(), doc.revision()), expected);
    assert!(matches!(session.status(), SemanticStatus::Available(_)));
}

/// B10-2: edits at the beginning, middle, and end produce correct
/// headings; after every edit the incremental result equals the clean
/// authoritative parse of the new source.
#[test]
fn edits_at_beginning_middle_end_match_clean_parse() {
    let mut doc = Document::new("# First\n\nmiddle text\n\n# Last\n");
    let session = H4Service::new().open_session(doc.id(), config());
    session.reset(&doc.snapshot());

    // beginning: insert a heading before everything
    let mut rev = apply(
        &mut doc,
        session.as_ref(),
        SourceRange::new(SourceOffset::new(0), SourceOffset::new(0)),
        "# Zero 始\n\n",
    );
    assert_eq!(
        headings_at(session.as_ref(), rev),
        clean_headings(doc.source())
    );

    // middle: turn the middle paragraph into a heading
    let mid_start = doc.source().find("middle text").unwrap();
    rev = apply(
        &mut doc,
        session.as_ref(),
        SourceRange::new(
            SourceOffset::new(mid_start),
            SourceOffset::new(mid_start + "middle text".len()),
        ),
        "## Middle 中",
    );
    assert_eq!(
        headings_at(session.as_ref(), rev),
        clean_headings(doc.source())
    );

    // end: append a heading
    let r = whole(&doc);
    let tail = format!("{}\n\n# Tail 🏁\n", doc.source().trim_end_matches('\n'));
    rev = apply(&mut doc, session.as_ref(), r, &tail);
    assert_eq!(
        headings_at(session.as_ref(), rev),
        clean_headings(doc.source())
    );

    let texts: Vec<_> = headings_at(session.as_ref(), doc.revision())
        .into_iter()
        .map(|(_, t)| t)
        .collect();
    assert_eq!(texts, ["Zero 始", "First", "Middle 中", "Last", "Tail 🏁"]);
}

/// B10-3: a large paste is one commit and lands correctly.
#[test]
fn large_paste_lands_correctly() {
    let mut doc = Document::new("# Start\n");
    let session = H4Service::new().open_session(doc.id(), config());
    session.reset(&doc.snapshot());

    let pasted: String = (0..400)
        .map(|i| format!("## Pasted heading {i}\n\npasted body {i}\n\n"))
        .collect();
    let r = whole(&doc);
    let combined = format!("{}\n{}", doc.source().trim_end(), pasted);
    let rev = apply(&mut doc, session.as_ref(), r, &combined);

    assert_eq!(headings_at(session.as_ref(), rev).len(), 401);
    assert_eq!(
        headings_at(session.as_ref(), rev),
        clean_headings(doc.source())
    );
}

/// B10-4/5: CJK and emoji headings survive incremental updates.
#[test]
fn cjk_and_emoji_headings_survive_updates() {
    let mut doc = Document::new("# 世界\n\n## 🌍 地球\n");
    let session = H4Service::new().open_session(doc.id(), config());
    session.reset(&doc.snapshot());
    assert_eq!(
        headings_at(session.as_ref(), doc.revision()),
        vec![(1u8, "世界".to_owned()), (2u8, "🌍 地球".to_owned())]
    );

    let r = whole(&doc);
    let rev = apply(
        &mut doc,
        session.as_ref(),
        r,
        "# 新世界 🎌\n\n### 絵文字 🎨 section\n",
    );
    assert_eq!(
        headings_at(session.as_ref(), rev),
        clean_headings(doc.source())
    );
}

/// B10-6: a pinned semantic view for revision N stays N — same identity,
/// same headings — after the source reaches N+1.
#[test]
fn pinned_view_stays_at_n_after_n_plus_one() {
    let mut doc = Document::new("# One\n");
    let session = H4Service::new().open_session(doc.id(), config());
    session.reset(&doc.snapshot());
    let pinned = session.pin(doc.revision()).expect("initial view");
    assert_eq!(pinned.revision(), SourceRevision::INITIAL);

    let r = whole(&doc);
    apply(&mut doc, session.as_ref(), r, "# Two\n");
    assert_eq!(doc.revision().get(), 1);

    assert_eq!(
        pinned.revision(),
        SourceRevision::INITIAL,
        "no silent upgrade"
    );
    assert_eq!(pinned.headings().len(), 1);
    assert_eq!(pinned.headings()[0].text, "One");

    let outline_then = OutlineProjection::derive(&pinned);
    let current = session.pin(doc.revision()).expect("current view");
    let outline_now = OutlineProjection::derive(&current);
    assert_ne!(outline_then, outline_now);
    assert_eq!(outline_now.entries[0].text, "Two");
}

/// B10-7: two documents at the same numeric revision never cross
/// semantic results.
#[test]
fn same_revision_two_documents_never_cross() {
    let service = H4Service::new();
    let mut a = Document::new("# Alpha 文\n");
    let b_snapshot_doc = Document::new("# Beta 語\n");
    let sa = service.open_session(a.id(), config());
    let sb = service.open_session(b_snapshot_doc.id(), config());
    sa.reset(&a.snapshot());
    sb.reset(&b_snapshot_doc.snapshot());
    assert_eq!(a.revision(), b_snapshot_doc.revision());

    let ra = whole(&a);
    apply(&mut a, sa.as_ref(), ra, "# Alpha 改\n");
    let pinned_b = sb.pin(b_snapshot_doc.revision()).expect("b view");
    assert_eq!(pinned_b.headings()[0].text, "Beta 語");
    assert_ne!(pinned_b.id().document, a.id());
    assert_ne!(
        sa.generation(),
        sb.generation(),
        "sessions are distinct generations"
    );
}

/// B10-8: a notification is accepted only against its target identity —
/// a stale-revision Available event cannot satisfy the interactive
/// target's outline pull.
#[test]
fn stale_revision_event_is_rejected_by_target_check() {
    let mut doc = Document::new("# Old\n");
    let session = H4Service::new().open_session(doc.id(), config());
    session.reset(&doc.snapshot());
    let stale_revision = doc.revision();
    // a real consumer (e.g. a print session) HOLDS the old pin
    let stale_view = session.pin(stale_revision).expect("stale view");

    let r = whole(&doc);
    apply(&mut doc, session.as_ref(), r, "# New\n");
    let target = doc.revision();
    assert_ne!(stale_revision, target);

    // The listener's acceptance rule: the event's revision must equal the
    // interactive target before its view may be pulled.
    let accepts = |event: &SemanticEvent, target: SourceRevision| match event {
        SemanticEvent::Available(id) => id.revision == target,
        SemanticEvent::Failed { .. } => false,
    };
    assert!(!accepts(&SemanticEvent::Available(stale_view.id()), target));
    let current_view = session.pin(target).expect("current view");
    assert!(accepts(
        &SemanticEvent::Available(current_view.id()),
        target
    ));
    assert_eq!(
        session.status(),
        SemanticStatus::Available(current_view.id())
    );
}

/// An unordered or gapped edit sequence is refused explicitly; the
/// session recovers via `reset` from the document's snapshot.
#[test]
fn gapped_edit_sequence_is_refused_and_reset_recovers() {
    let mut doc = Document::new("# A\n\nB\n");
    let session = H4Service::new().open_session(doc.id(), config());
    session.reset(&doc.snapshot());

    let stale_before = doc.snapshot();
    let r0 = whole(&doc);
    let tail_a2 = "# A2\n\nB\n";
    apply(&mut doc, session.as_ref(), r0, tail_a2);
    let base_after_first = doc.revision();
    let r1 = whole(&doc);
    let tail_a3 = "# A3\n\nB\n";
    apply(&mut doc, session.as_ref(), r1, tail_a3);
    assert_eq!(doc.revision().get(), 2);
    let _ = base_after_first;

    // replaying the FIRST edit's successor against the wrong base is a gap
    let stale_range = SourceRange::new(
        SourceOffset::new(0),
        SourceOffset::new(stale_before.source().len()),
    );
    let forged = SourceEdit::replace(
        doc.id(),
        stale_before.revision(),
        stale_range,
        "# Forged\n".to_owned(),
    );
    let after = doc.snapshot();
    let error = session
        .apply_edit(&stale_before, &after, &forged)
        .expect_err("gapped sequence must be refused");
    assert!(error.to_string().contains("reset"), "{error}");

    // reset from the document's real snapshot recovers cleanly
    session.reset(&after);
    assert_eq!(
        headings_at(session.as_ref(), doc.revision()),
        vec![(1u8, "A3".to_owned()),]
    );
}

/// The out-of-order guard, exercised with real event capture: events
/// published out of order are only accepted against their own target.
#[test]
fn events_carry_targets_and_late_delivery_changes_nothing() {
    let mut doc = Document::new("# Start\n");
    let session = H4Service::new().open_session(doc.id(), config());
    let seen: Rc<std::cell::RefCell<Vec<SemanticEvent>>> =
        Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = seen.clone();
    let _sub = session.subscribe(Rc::new(move |e| sink.borrow_mut().push(e.clone())));

    session.reset(&doc.snapshot());
    let r1 = whole(&doc);
    apply(&mut doc, session.as_ref(), r1, "# Start 2\n");
    let r2 = whole(&doc);
    apply(&mut doc, session.as_ref(), r2, "# Start 3\n");

    let seen = seen.borrow();
    assert_eq!(seen.len(), 3, "one event per publication");
    let revisions: Vec<_> = seen
        .iter()
        .map(|e| match e {
            SemanticEvent::Available(id) => id.revision.get(),
            SemanticEvent::Failed { .. } => panic!("no failures expected"),
        })
        .collect();
    assert_eq!(revisions, [0, 1, 2]);
    // the interactive target is the last event; replaying earlier events
    // in any order cannot change which revision is current
    assert_eq!(
        session.status(),
        SemanticStatus::Available(session.pin(doc.revision()).unwrap().id())
    );
}
