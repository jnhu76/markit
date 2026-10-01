// The Source Mode editor: CodeMirror 6 as presentation/input machinery.
// The editor's buffer is NOT a document authority: user changes go to the
// domain as typed edits (UTF-8 byte offsets), and after every
// authoritative operation the renderer reconciles to the Document
// revision. Undo/redo are domain commands — CodeMirror's own history is
// deliberately NOT installed, so there is exactly one undo authority.
//
// Typed changes are applied through a serial queue against the session:
// each queued edit carries its transaction's coordinates (relative to the
// document state its predecessor produced), and a refusal triggers a full
// resync to the authoritative snapshot instead of divergent source.

import { EditorView } from "@codemirror/view";
import { EditorState, Compartment } from "@codemirror/state";
import { markdown } from "@codemirror/lang-markdown";
import { defaultKeymap, indentWithTab } from "@codemirror/commands";
import { keymap, lineNumbers } from "@codemirror/view";
import { defaultHighlightStyle, syntaxHighlighting, bracketMatching, indentOnInput } from "@codemirror/language";
import type { DocumentSession } from "../adapter/types";
import { utf16ToByte } from "../adapter/fake";

export interface SourceEditor {
  readonly host: HTMLElement;
  /** Full reconcile to the authoritative source (undo/redo/resync). */
  reconcile(source: string): Promise<void>;
  /** The UI caret head, in domain bytes. */
  caretByteOffset(): number;
}

export async function createSourceEditor(
  host: HTMLElement,
  session: DocumentSession,
  onAuthoritativeChange: () => void,
): Promise<SourceEditor> {
  host.className = "source-editor";

  const initial = await session.view();
  let baseRevision = initial.revision;
  let applying = false;
  let queue: Promise<void> = Promise.resolve();

  const language = new Compartment();

  const state = EditorState.create({
    doc: initial.source,
    extensions: [
      lineNumbers(),
      language.of(markdown()),
      syntaxHighlighting(defaultHighlightStyle),
      bracketMatching(),
      indentOnInput(),
      keymap.of([...defaultKeymap, indentWithTab]),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) return;
        if (applying) return; // our own reconcile echo
        const startDoc = update.startState.doc.toString();
        update.changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
          const edit = {
            from: utf16ToByte(startDoc, fromA),
            to: utf16ToByte(startDoc, toA),
            inserted,
          };
          queue = queue.then(async () => {
            const result = await session.edit(
              baseRevision,
              { start: edit.from, end: edit.to },
              edit.inserted.toString(),
            );
            if (result.kind === "rejected") {
              // Never corrupt source on refusal: resync to the authority.
              const authoritative = await session.view();
              baseRevision = authoritative.revision;
              applyDoc(authoritative.source);
              onAuthoritativeChange();
              return;
            }
            baseRevision = result.revision;
            onAuthoritativeChange();
          });
        });
      }),
    ],
  });

  const view = new EditorView({ state, parent: host });

  function applyDoc(source: string): void {
    if (view.state.doc.toString() === source) return;
    applying = true;
    try {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: source },
      });
    } finally {
      applying = false;
    }
  }

  return {
    host,
    async reconcile(source: string) {
      await queue;
      const authoritative = await session.view();
      baseRevision = authoritative.revision;
      applyDoc(source);
      onAuthoritativeChange();
    },
    caretByteOffset() {
      return utf16ToByte(view.state.doc.toString(), view.state.selection.main.head);
    },
  };
}
