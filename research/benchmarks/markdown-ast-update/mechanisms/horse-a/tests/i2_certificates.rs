//! I2 T2d — persistent RestartCertificate semantics (spec §5.1, §6;
//! data-model §8; tasks #18–#23).
//!
//! Frozen rules under test:
//!
//! ```text
//! a certificate exists only at an INTERIOR Owner boundary, built only
//!   from the real I1 RootBlankEvent whose cut equals that boundary;
//! support = preceding LF + complete blank physical line, Owner-relative,
//!   never crossing the left Owner's coverage start;
//! certificate coordinates are Owner-relative (never document-absolute);
//! the EOF boundary (cut == source_len) is never certified — real EOF is
//!   a separate legal completion path and finish() manufactures nothing;
//! BOF blanks never certify (their cut precedes the first interior
//!   boundary); preceding_lf = None is legal only at true BOF;
//! fence-body blanks and quote-prefixed live blanks certify nothing.
//! ```

mod common;

use common::build;
use markit_mdbench_horse_a::{OwnerPayload, RestartSupport};

/// The E24 frozen example support (source `"ab\n\n"`: bytes 0 a, 1 b,
/// 2 LF, 3 LF; support = preceding LF 2 + blank line [3,4) = {2,3}).
fn e24_support() -> RestartSupport {
    RestartSupport {
        preceding_lf: Some(2),
        blank_line: 3..4,
    }
}

#[test]
fn e24_insertion_at_a_support_byte_touches_the_support() {
    // The frozen E24 case, byte-exact: insert [2,2) -> touches support
    // -> the certificate is invalid for reuse. Do not weaken this.
    let s = e24_support();
    assert!(
        s.touches(2, 2),
        "zero-length insertion at the preceding LF byte 2 must touch the support"
    );
    assert!(
        s.touches(3, 3),
        "zero-length insertion inside the blank line must touch the support"
    );
    // Non-support bytes stay untouched.
    assert!(!s.touches(0, 0));
    assert!(!s.touches(1, 1));
    assert!(
        !s.touches(4, 4),
        "the cut itself (EOF side) is not a support byte"
    );
    // Range edits: intersection semantics.
    assert!(
        !s.touches(0, 2),
        "an edit ending exactly at the support start does not touch it"
    );
    assert!(!s.touches(4, 5));
    assert!(s.touches(2, 3));
    assert!(s.touches(3, 4));
    assert!(s.touches(2, 4));
    assert!(s.touches(0, 4));
    assert!(s.touches(1, 3));
}

#[test]
fn e24_support_is_retained_as_a_real_interior_certificate() {
    // "ab\n\ncd\n": the blank [3,4) certifies the interior boundary 4
    // with exactly the E24 support, Owner-relative to base 0.
    let doc = build(b"ab\n\ncd\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    let cert = owners[0]
        .outgoing_restart
        .as_ref()
        .expect("the interior boundary 4 must be certified");
    assert_eq!(cert.support, e24_support());
    assert!(cert.support.touches(2, 2));
    assert!(owners[1].outgoing_restart.is_none());
}

#[test]
fn a_simple_root_blank_certifies_the_boundary_between_two_owners() {
    // "a\n\nb\n": blank [2,3), cut 3 == p1; preceding LF 1.
    let doc = build(b"a\n\nb\n");
    let owners = doc.owners.owners_in_order();
    let cert = owners[0].outgoing_restart.as_ref().expect("certified");
    assert_eq!(cert.support.preceding_lf, Some(1));
    assert_eq!(cert.support.blank_line, 2..3);
    assert!(owners[1].outgoing_restart.is_none());
}

#[test]
fn every_interior_boundary_between_many_owners_is_certified() {
    // I1-derived evidence for this source: barriers with cuts
    // [5, 11, 16, 27] and preceding LFs [3, 9, 14, 25]; five root blocks.
    let src: &[u8] = b"# h\n\npara\n\n> q\n\n```\nc\n```\n\n[l]: /d\n";
    let doc = build(src);
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 5);
    // OWNER-RELATIVE expectations: absolute (preceding LF, blank line)
    // minus the Owner's coverage base. Bases: 0, 5, 11, 16, 27.
    let expected: [(Option<usize>, std::ops::Range<usize>); 4] = [
        (Some(3), 4..5),   // base 0:  abs (3, [4,5))
        (Some(4), 5..6),   // base 5:  abs (9, [10,11))
        (Some(3), 4..5),   // base 11: abs (14, [15,16))
        (Some(9), 10..11), // base 16: abs (25, [26,27))
    ];
    for (i, (preceding, blank)) in expected.iter().enumerate() {
        let cert = owners[i]
            .outgoing_restart
            .as_ref()
            .unwrap_or_else(|| panic!("boundary after Owner {i} must be certified"));
        assert_eq!(cert.support.preceding_lf, *preceding);
        assert_eq!(cert.support.blank_line.clone(), blank.clone());
        // The support ends exactly at the Owner's outgoing boundary and
        // never crosses the Owner's coverage start (task #30).
        assert_eq!(cert.support.blank_line.end, owners[i].coverage_len);
    }
    // The last Owner's outgoing boundary is the EOF boundary: no
    // certificate (task #23).
    assert!(owners[4].outgoing_restart.is_none());
}

#[test]
fn a_blank_inside_a_fence_never_certifies() {
    let doc = build(b"```\n\nx\n```\n");
    for owner in doc.owners.owners_in_order() {
        assert!(
            owner.outgoing_restart.is_none(),
            "fence-body blank certified"
        );
    }
}

#[test]
fn a_quote_prefixed_blank_with_a_live_quote_never_certifies() {
    let doc = build(b"> a\n>\n> b\n");
    for owner in doc.owners.owners_in_order() {
        assert!(
            owner.outgoing_restart.is_none(),
            "live-quote blank certified"
        );
    }
}

#[test]
fn an_unprefixed_blank_that_really_closes_the_quote_certifies() {
    // "> a\n\nb\n": the unprefixed blank closes the quote back to root,
    // then B1 observes a real root blank (parser evidence, allowed).
    // "> a\n" [0,4), blank [4,5) cut 5, preceding LF 3, "b\n" [5,7).
    let doc = build(b"> a\n\nb\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    let cert = owners[0].outgoing_restart.as_ref().expect("certified");
    assert_eq!(cert.support.preceding_lf, Some(3));
    assert_eq!(cert.support.blank_line, 4..5);
    assert!(owners[1].outgoing_restart.is_none());
}

#[test]
fn list_closure_certifies_at_its_real_post_b1_state() {
    // "- a\n\n- b\n": B1 closes the item and the list back to root (D5
    // tight-only), so the blank really is a root barrier (I1 evidence:
    // line_start 4, cut 5, preceding 3).
    let doc = build(b"- a\n\n- b\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    let cert = owners[0].outgoing_restart.as_ref().expect("certified");
    assert_eq!(cert.support.preceding_lf, Some(3));
    assert_eq!(cert.support.blank_line, 4..5);
    assert!(owners[1].outgoing_restart.is_none());
}

#[test]
fn eof_without_a_trailing_lf_never_certifies() {
    // "a\n\nb": the last line has no LF, so there is no trailing blank
    // line at all; the only barrier is the interior one.
    let doc = build(b"a\n\nb");
    let owners = doc.owners.owners_in_order();
    assert!(
        owners[0].outgoing_restart.is_some(),
        "interior boundary 3 is certified"
    );
    assert!(owners[1].outgoing_restart.is_none());

    // "ab": a single line, no blank anywhere.
    let doc = build(b"ab");
    assert!(doc.owners.owners_in_order()[0].outgoing_restart.is_none());
}

#[test]
fn the_eof_boundary_is_never_certified_even_after_a_real_trailing_blank() {
    // "ab\n\ncd\n\n": the trailing blank [7,8) is real I1 evidence, but
    // its cut is the source end — the EOF boundary, which is a separate
    // legal completion path, not a restart-certified interior boundary
    // (data-model §8.3: the last Owner's EOF boundary never enters
    // subtree_has_safe; task #23: no EOF certificate from RanToEnd).
    let doc = build(b"ab\n\ncd\n\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    let cert = owners[0]
        .outgoing_restart
        .as_ref()
        .expect("interior boundary 4 certified");
    assert_eq!(cert.support.preceding_lf, Some(2));
    assert_eq!(cert.support.blank_line, 3..4);
    assert!(
        owners[1].outgoing_restart.is_none(),
        "the EOF boundary must not carry a certificate"
    );
    // subtree_has_safe therefore stays false when the only trailing
    // evidence is at EOF... but Owner 0's interior certificate still
    // makes the root aggregate true here; the EOF-side Owner contributes
    // nothing.
    let last = owners.last().unwrap();
    assert!(last.outgoing_restart.is_none());
}

#[test]
fn several_blanks_in_one_gap_produce_exactly_one_boundary_certificate() {
    // "a\n\n\nb\n": two transient events (cuts 3 and 4); only the one
    // whose cut equals the interior boundary 4 persists — the frozen
    // one-certificate-per-Owner-boundary mapping (task #19).
    let doc = build(b"a\n\n\nb\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    let cert = owners[0]
        .outgoing_restart
        .as_ref()
        .expect("boundary certificate");
    assert_eq!(
        cert.support.preceding_lf,
        Some(2),
        "the LAST blank's LF, not the first blank's"
    );
    assert_eq!(cert.support.blank_line, 3..4);
    assert!(owners[1].outgoing_restart.is_none());
}

#[test]
fn bof_blanks_are_real_evidence_but_never_boundary_certificates() {
    // "\na\n\nb\n": the BOF blank (cut 1, preceding_lf None) precedes the
    // first block; it is transient. The interior boundary 4 (p1 = 4)
    // carries the certificate: blank [3,4), preceding LF 2. BOF stays a
    // distinguished VIRTUAL restart authority — no certificate object is
    // needed for it (data-model §8.3).
    let doc = build(b"\na\n\nb\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 2);
    let cert = owners[0]
        .outgoing_restart
        .as_ref()
        .expect("boundary 4 certified");
    assert_eq!(
        cert.support.preceding_lf,
        Some(2),
        "the interior blank's LF, not the BOF blank's"
    );
    assert_eq!(cert.support.blank_line, 3..4);
    assert!(owners[1].outgoing_restart.is_none());

    // A blank-only document has no interior boundary at all: no
    // certificate anywhere.
    let doc = build(b"\n");
    let owners = doc.owners.owners_in_order();
    assert_eq!(owners.len(), 1);
    assert!(matches!(owners[0].payload, OwnerPayload::TriviaOnly));
    assert!(owners[0].outgoing_restart.is_none());
}

#[test]
fn every_persisted_certificate_maps_to_one_owner_boundary() {
    // Mapping invariant (task #30) over a mixed document: every
    // certificate's blank line ends exactly at its Owner's coverage end,
    // and no certificate exists inside a container or at EOF.
    let doc = build(b"# h\n\npara\n\n> q\n\n```\nc\n```\n\n[l]: /d\n\nafter\n");
    markit_mdbench_horse_a::validate_certificates(&doc)
        .expect("every certificate must map to one real Owner boundary");
    for (i, owner) in doc.owners.owners_in_order().iter().enumerate() {
        if let Some(cert) = &owner.outgoing_restart {
            assert_eq!(
                cert.support.blank_line.end, owner.coverage_len,
                "certificate {i} does not end at its Owner boundary"
            );
        }
    }
    // The last Owner is never certified (EOF boundary).
    assert!(doc
        .owners
        .owners_in_order()
        .last()
        .unwrap()
        .outgoing_restart
        .is_none());
}
