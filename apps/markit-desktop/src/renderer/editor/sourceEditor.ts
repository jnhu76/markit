// The Source Mode editor: CodeMirror 6 as presentation/input machinery.
// The editor's buffer is NOT a document authority: user changes go to the
// domain as typed edits (UTF-8 byte offsets), and after every
// authoritative operation the renderer reconciles to the Document
// revision. Undo/redo are domain commands — CodeMirror's own history is
// deliberately NOT installed, and the standard keys route to the domain.
//
// ALL authoritative operations (typed edits, undo, redo) run through ONE
// serial queue against the session. Each queued task captures the epoch
// it was enqueued under; a refusal or a thrown bridge error resyncs the
// editor to the authoritative snapshot and invalidates every queued task
// that still carries pre-resync coordinates. There is no path where the
// buffer diverges from the authority without a resync.

import { EditorView } from "@codemirror/view";
import { EditorState, Compartment } from "@codemirror/state";
import { markdown } from "@codemirror/lang-markdown";
import { defaultKeymap, indentWithTab } from "@codemirror/commands";
import { keymap, lineNumbers } from "@codemirror/view";
import {
  defaultHighlightStyle,
  syntaxHighlighting,
  bracketMatching,
  indentOnInput,
} from "@codemirror/language";
import type { DocumentSession } from "../adapter/types";
import { utf16ToByte } from "../adapter/fake";

export interface SourceEditor {
  readonly host: HTMLElement;
  /** Domain undo through the serial queue; a no-op at history start. */
  undo(): Promise<void>;
  /** Domain redo through the serial queue. */
  redo(): Promise<void>;
  /** Force-resync to the authoritative snapshot (after the queue drains). */
  reconcile(): Promise<void>;
  /** The UI caret head, in domain bytes. */
  caretByteOffset(): number;
  /** Destroy the CodeMirror instance (document switch). */
  destroy(): void;
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
  let epoch = 0;
  // Bound late: the keymap closures below read this at keypress time.
  let api: SourceEditor;

  const language = new Compartment();

  const resync = async (): Promise<void> => {
    epoch += 1;
    const authoritative = await session.view();
    baseRevision = authoritative.revision;
    applyDoc(authoritative.source);
    onAuthoritativeChange();
  };

  /** Run `task` through the serial queue under the current epoch. */
  const enqueue = (task: () => Promise<void>): Promise<void> => {
    const myEpoch = epoch;
    queue = queue
      .then(() => {
        if (myEpoch !== epoch) return; // pre-resync coordinates: skip
        return task();
      })
      .catch(() => resync());
    return queue;
  };

  const state = EditorState.create({
    doc: initial.source,
    extensions: [
      lineNumbers(),
      language.of(markdown()),
      syntaxHighlighting(defaultHighlightStyle),
      bracketMatching(),
      indentOnInput(),
      keymap.of([
        { key: "Mod-z", run: () => (void api.undo(), true) },
        { key: "Mod-Shift-z", run: () => (void api.redo(), true) },
        { key: "Mod-y", run: () => (void api.redo(), true) },
        ...defaultKeymap,
        indentWithTab,
      ]),
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
          enqueue(async () => {
            const result = await session.edit(
              baseRevision,
              { start: edit.from, end: edit.to },
              edit.inserted.toString(),
            );
            if (result.kind === "rejected") {
              // Never corrupt source on refusal: resync to the authority;
              // queued successors carry stale coordinates and are skipped.
              await resync();
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

  api = {
    host,

    undo() {
      return enqueue(async () => {
        const revision = await session.undo();
        if (revision !== null) await resync();
      });
    },

    redo() {
      return enqueue(async () => {
        const revision = await session.redo();
        if (revision !== null) await resync();
      });
    },

    async reconcile() {
      await queue;
      const authoritative = await session.view();
      baseRevision = authoritative.revision;
      applyDoc(authoritative.source);
      onAuthoritativeChange();
    },

    caretByteOffset() {
      return utf16ToByte(view.state.doc.toString(), view.state.selection.main.head);
    },

    destroy() {
      view.destroy();
    },
  };

  return api;
}
