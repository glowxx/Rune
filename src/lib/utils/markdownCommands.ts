/**
 * Komendy formatowania Markdown działające na CodeMirror 6 (`EditorView`).
 * Wspólne dla paska narzędzi i skrótów klawiszowych edytora — trzymane osobno,
 * żeby komponent edytora pozostał czytelny.
 *
 * Wszystkie komendy obsługują wiele zaznaczeń/kursorów przez `changeByRange`,
 * które samo mapuje pozycje po wstawieniach (bez ręcznego liczenia offsetów).
 */
import type { EditorView } from "@codemirror/view";
import { EditorSelection } from "@codemirror/state";

/**
 * Owija każde zaznaczenie sekwencjami `before`/`after`. Gdy zaznaczenie jest
 * puste, wstawia samą składnię i ustawia kursor w środku (gotowe do pisania).
 */
export function wrapSelection(view: EditorView, before: string, after: string): void {
  view.dispatch(
    view.state.changeByRange((range) => {
      const selected = view.state.sliceDoc(range.from, range.to);
      const insert = before + selected + after;
      const anchor = range.from + before.length;
      const head = range.empty ? anchor : anchor + selected.length;
      return {
        changes: { from: range.from, to: range.to, insert },
        range: EditorSelection.range(anchor, head),
      };
    }),
  );
  view.focus();
}

/** Numery linii objętych zaznaczeniem (deduplikowane, rosnąco). */
function selectedLineNumbers(view: EditorView): number[] {
  const { state } = view;
  const nums = new Set<number>();
  for (const range of state.selection.ranges) {
    const start = state.doc.lineAt(range.from).number;
    const end = state.doc.lineAt(range.to).number;
    for (let n = start; n <= end; n++) nums.add(n);
  }
  return [...nums].sort((a, b) => a - b);
}

/**
 * Przełącza prefiks na początku każdej objętej linii (listy, cytat). Jeśli
 * wszystkie linie mają już prefiks — usuwa go; w przeciwnym razie dodaje.
 */
export function togglePrefix(view: EditorView, prefix: string): void {
  const { state } = view;
  const lines = selectedLineNumbers(view).map((n) => state.doc.line(n));
  const allHave = lines.every((l) => l.text.startsWith(prefix));

  const changes = lines.map((line) =>
    allHave
      ? { from: line.from, to: line.from + prefix.length, insert: "" }
      : { from: line.from, insert: prefix },
  );

  view.dispatch({ changes });
  view.focus();
}

/**
 * Ustawia poziom nagłówka (`#`…`######`) na objętych liniach. Powtórne wywołanie
 * z tym samym poziomem zdejmuje nagłówek. Istniejący nagłówek innego poziomu
 * jest podmieniany.
 */
export function setHeading(view: EditorView, level: number): void {
  const prefix = "#".repeat(level) + " ";
  const { state } = view;
  const changes: { from: number; to: number; insert: string }[] = [];

  for (const n of selectedLineNumbers(view)) {
    const line = state.doc.line(n);
    const m = line.text.match(/^(#{1,6} )/);
    const to = line.from + (m ? m[1].length : 0);
    const insert = m && m[1] === prefix ? "" : prefix; // toggle off jeśli ten sam poziom
    changes.push({ from: line.from, to, insert });
  }

  view.dispatch({ changes });
  view.focus();
}

/** Wstawia link `[tekst](url)`; kursor ląduje na `url` (lub na `tekst` gdy puste). */
export function insertLink(view: EditorView): void {
  view.dispatch(
    view.state.changeByRange((range) => {
      const text = view.state.sliceDoc(range.from, range.to);
      if (text) {
        const insert = `[${text}](url)`;
        const urlFrom = range.from + 1 + text.length + 2; // po „](”
        return {
          changes: { from: range.from, to: range.to, insert },
          range: EditorSelection.range(urlFrom, urlFrom + 3), // zaznacz „url”
        };
      }
      const insert = `[text](url)`;
      const textFrom = range.from + 1;
      return {
        changes: { from: range.from, to: range.to, insert },
        range: EditorSelection.range(textFrom, textFrom + 4), // zaznacz „text”
      };
    }),
  );
  view.focus();
}

/**
 * Kod: inline `` `…` `` dla pojedynczej linii, blok ``` ``` ``` ``` dla
 * zaznaczenia wielolinijkowego.
 */
export function toggleCode(view: EditorView): void {
  view.dispatch(
    view.state.changeByRange((range) => {
      const text = view.state.sliceDoc(range.from, range.to);
      if (text.includes("\n")) {
        const insert = "```\n" + text + "\n```";
        const anchor = range.from + 4; // po „```\n”
        return {
          changes: { from: range.from, to: range.to, insert },
          range: EditorSelection.range(anchor, anchor + text.length),
        };
      }
      const insert = "`" + text + "`";
      const anchor = range.from + 1;
      const head = range.empty ? anchor : anchor + text.length;
      return {
        changes: { from: range.from, to: range.to, insert },
        range: EditorSelection.range(anchor, head),
      };
    }),
  );
  view.focus();
}
