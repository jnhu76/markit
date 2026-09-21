//! Constructor/restore regression tests for the A6 repair set (PR #39):
//!
//! 1. the exact RESTORE inverse must restore replace-edits byte-exactly
//!    (G1-TABLE-DELIM-BREAK previously lost one byte);
//! 2. header-pipe removal must select a cell-separator pipe, not a
//!    leading/trailing boundary pipe whose removal keeps the table;
//! 3. list-item indent requires an adjacent same-indent sibling item
//!    (G0 §7: tight-only lists; non-adjacent items never nest);
//! 4. link-destination break inserts a space that decomposes the link.

use markit_mdbench_semantics::{
    g0::parse_g0, profile_g0, profile_g1, RecognitionStatus, SyntaxKind, G0_GRAMMAR_ID,
    G1_GRAMMAR_ID,
};
use markit_mdbench_workload_freeze::editors;

fn g0_fact_of(source: &str, kind: SyntaxKind) -> markit_mdbench_semantics::SyntaxFact {
    let profile = profile_g0(source);
    profile
        .syntax_facts
        .iter()
        .find(|f| f.syntax_kind == kind && f.recognition_status == RecognitionStatus::Recognized)
        .expect("fact of kind")
        .clone()
}

fn g1_fact_of(source: &str, kind: SyntaxKind) -> markit_mdbench_semantics::SyntaxFact {
    let profile = profile_g1(source);
    profile
        .syntax_facts
        .iter()
        .find(|f| f.syntax_kind == kind && f.recognition_status == RecognitionStatus::Recognized)
        .expect("fact of kind")
        .clone()
}

/// The exact RESTORE inverse of any edit (delete / insert / replace)
/// must reproduce the pre source byte-for-byte.
#[test]
fn exact_inverse_is_byte_exact_for_all_edit_shapes() {
    // Replace edit (G1 delimiter '-' -> 'n').
    let table = "| h1 | h2 |\n|---|---|\n| a | b |\n";
    let fact = g1_fact_of(table, SyntaxKind::Table);
    let brk = editors::construct_table_delim_break(table, &fact).expect("break");
    let s1 = brk.edit.apply(table).expect("apply");
    assert_ne!(s1, table);
    let removed = editors::slice(table, brk.edit.edit_start, brk.edit.edit_end);
    let restore = editors::construct_exact_reinsert(
        &s1,
        &removed,
        brk.edit.edit_start,
        brk.edit.edit_start + brk.edit.inserted_text.len() as u64,
        "ordinary_table",
    );
    assert_eq!(restore.edit.apply(&s1).expect("apply"), table);

    // Pure insert edit (list indent), on the second (sibling-adjacent) item.
    let list = "- a\n- b\n";
    let fact = profile_g0(list)
        .syntax_facts
        .iter()
        .find(|f| f.syntax_kind == SyntaxKind::ListItem && f.occurrence == 1)
        .expect("second item")
        .clone();
    let indent = editors::construct_list_item_indent(list, &fact).expect("indent");
    let s1 = indent.edit.apply(list).expect("apply");
    let removed = editors::slice(list, indent.edit.edit_start, indent.edit.edit_end);
    let restore = editors::construct_exact_reinsert(
        &s1,
        &removed,
        indent.edit.edit_start,
        indent.edit.edit_start + indent.edit.inserted_text.len() as u64,
        "list_item",
    );
    assert_eq!(restore.edit.apply(&s1).expect("apply"), list);

    // Pure delete edit (fence closer removal).
    let fenced = "para\n\n```rust\nlet x = 1;\n```\n\nafter\n";
    let fact = g0_fact_of(fenced, SyntaxKind::CodeBlockFenced);
    let brk = editors::construct_fence_closer_remove(fenced, &fact).expect("remove");
    let s1 = brk.edit.apply(fenced).expect("apply");
    let removed = editors::slice(fenced, brk.edit.edit_start, brk.edit.edit_end);
    let restore = editors::construct_exact_reinsert(
        &s1,
        &removed,
        brk.edit.edit_start,
        brk.edit.edit_start + brk.edit.inserted_text.len() as u64,
        "top_level_fence",
    );
    assert_eq!(restore.edit.apply(&s1).expect("apply"), fenced);
}

/// Removing a boundary pipe must not be selected; a separator pipe must.
#[test]
fn header_pipe_remove_selects_separator_only() {
    // `| a | b |`: pipes at 0 (leading), interior, last (trailing).
    let table = "| a | b |\n|---|---|\n| 1 | 2 |\n";
    let fact = g1_fact_of(table, SyntaxKind::Table);
    let edit = editors::construct_table_header_pipe_remove(table, &fact).expect("interior");
    // The interior pipe of the header row (byte 4), not 0 and not 7.
    assert_eq!(edit.edit.edit_start, 4);
    let post = edit.edit.apply(table).expect("apply");
    assert!(post.starts_with("| a  b |"));

    // `| a |`: only boundary pipes -> no separator, constructor rejects.
    let one_cell = "| a |\n|---|\n| 1 |\n";
    let fact = g1_fact_of(one_cell, SyntaxKind::Table);
    assert!(editors::construct_table_header_pipe_remove(one_cell, &fact).is_err());

    // `a | b |`: trailing boundary only; the first pipe is a separator.
    let no_leading = "a | b |\n--- | ---|\n1 | 2 |\n";
    let fact = g1_fact_of(no_leading, SyntaxKind::Table);
    let edit = editors::construct_table_header_pipe_remove(no_leading, &fact).expect("interior");
    assert_eq!(edit.edit.edit_start, 2);
}

/// Indenting requires an adjacent same-indent sibling item line.
#[test]
fn list_indent_requires_adjacent_sibling() {
    let adjacent = "- a\n- b\n";
    let facts = profile_g0(adjacent)
        .syntax_facts
        .iter()
        .filter(|f| {
            f.syntax_kind == SyntaxKind::ListItem
                && f.recognition_status == RecognitionStatus::Recognized
        })
        .cloned()
        .collect::<Vec<_>>();
    // First item has no preceding sibling; second item does.
    let first = facts
        .iter()
        .find(|f| f.occurrence == 0)
        .expect("first item");
    assert!(editors::construct_list_item_indent(adjacent, first).is_err());
    let second = facts
        .iter()
        .find(|f| f.occurrence == 1)
        .expect("second item");
    assert!(editors::construct_list_item_indent(adjacent, second).is_ok());

    // Blank line between items ends the list (tight-only): never nests.
    let blank_separated = "- a\n\n- b\n";
    let facts = profile_g0(blank_separated)
        .syntax_facts
        .iter()
        .filter(|f| {
            f.syntax_kind == SyntaxKind::ListItem
                && f.recognition_status == RecognitionStatus::Recognized
        })
        .cloned()
        .collect::<Vec<_>>();
    let second = facts
        .iter()
        .find(|f| f.occurrence == 1)
        .expect("second item");
    assert!(editors::construct_list_item_indent(blank_separated, second).is_err());
}

/// Destination break inserts exactly one space at the destination start.
#[test]
fn link_dest_break_inserts_space() {
    let doc = "see [the docs](https://example.test/x) now\n";
    let fact = g0_fact_of(doc, SyntaxKind::LinkInline);
    let brk = editors::construct_link_dest_break(doc, &fact).expect("break");
    assert_eq!(brk.edit.inserted_text, " ");
    let post = brk.edit.apply(doc).expect("apply");
    assert!(post.contains("( https://example.test/x)"));
    // The G0 reference parse no longer recognizes a link.
    let pre_links = parse_g0(doc)
        .nodes()
        .iter()
        .filter(|n| n.kind == SyntaxKind::LinkInline)
        .count();
    let post_links = parse_g0(&post)
        .nodes()
        .iter()
        .filter(|n| n.kind == SyntaxKind::LinkInline)
        .count();
    assert_eq!(pre_links, 1);
    assert_eq!(post_links, 0);
    let _ = (G0_GRAMMAR_ID, G1_GRAMMAR_ID);
}
