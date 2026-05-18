<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { sql } from "@codemirror/lang-sql";
  import {
    syntaxHighlighting,
    defaultHighlightStyle,
    HighlightStyle,
    bracketMatching,
  } from "@codemirror/language";
  import { tags as t } from "@lezer/highlight";

  type Props = {
    value: string;
    dark: boolean;
    /** 현재 선택된 텍스트 — 선택 영역이 비어 있으면 빈 문자열. 부모가 bind 해서 사용 */
    selectedText?: string;
    onChange: (value: string) => void;
    onRun?: () => void;
  };
  let {
    value,
    dark,
    selectedText = $bindable(""),
    onChange,
    onRun,
  }: Props = $props();

  let host: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  // 외부 prop value 가 갱신 됐을 때 view 와 비교용
  let lastExternal = $state(untrack(() => value));

  // 다크 모드용 하이라이트 스타일 — 가독성 좋은 색 8종
  const darkHighlightStyle = HighlightStyle.define([
    { tag: [t.keyword, t.operatorKeyword], color: "#c792ea" }, // 자주색 (SELECT/FROM/WHERE 등)
    { tag: [t.string, t.special(t.string)], color: "#c3e88d" }, // 연두
    { tag: t.number, color: "#f78c6c" }, // 주황
    { tag: [t.bool, t.null], color: "#ff5370" }, // 빨강
    { tag: t.comment, color: "#546e7a", fontStyle: "italic" },
    { tag: t.variableName, color: "#82aaff" }, // 파랑
    { tag: t.typeName, color: "#ffcb6b" }, // 노랑 (자료형)
    { tag: t.punctuation, color: "#89ddff" }, // 청록 (쉼표/세미콜론)
  ]);

  function buildExtensions(isDark: boolean) {
    const runKeymap = onRun
      ? keymap.of([
          {
            key: "Mod-Enter",
            run: () => {
              onRun?.();
              return true;
            },
          },
        ])
      : keymap.of([]);

    return [
      lineNumbers(),
      highlightActiveLine(),
      bracketMatching(),
      history(),
      sql(), // SQL 모드 — 키워드/문자열/숫자 등을 lezer tag 로 분류
      syntaxHighlighting(isDark ? darkHighlightStyle : defaultHighlightStyle),
      runKeymap,
      keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          const next = update.state.doc.toString();
          lastExternal = next;
          onChange(next);
        }
        if (update.selectionSet || update.docChanged) {
          const main = update.state.selection.main;
          if (main.empty) {
            if (selectedText !== "") selectedText = "";
          } else {
            const text = update.state.doc.sliceString(main.from, main.to);
            if (selectedText !== text) selectedText = text;
          }
        }
      }),
      EditorView.theme(
        isDark
          ? {
              "&": {
                height: "100%",
                fontSize: "13px",
                backgroundColor: "#0f172a",
                color: "#e2e8f0",
              },
              ".cm-scroller": {
                fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
              },
              ".cm-gutters": {
                backgroundColor: "#0f172a",
                color: "#64748b",
                borderRight: "1px solid #1e293b",
              },
              ".cm-activeLine": { backgroundColor: "#1e293b" },
              ".cm-activeLineGutter": { backgroundColor: "#1e293b" },
              ".cm-cursor": { borderLeftColor: "#e2e8f0" },
              ".cm-selectionBackground, ::selection": { backgroundColor: "#334155 !important" },
            }
          : {
              "&": { height: "100%", fontSize: "13px" },
              ".cm-scroller": {
                fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
              },
            },
        { dark: isDark },
      ),
    ];
  }

  onMount(() => {
    if (!host) return;
    const state = EditorState.create({
      doc: value,
      extensions: buildExtensions(dark),
    });
    view = new EditorView({ state, parent: host });
  });

  onDestroy(() => {
    view?.destroy();
    view = null;
  });

  $effect(() => {
    if (!view) return;
    if (value === lastExternal) return;
    const current = view.state.doc.toString();
    if (current === value) {
      lastExternal = value;
      return;
    }
    view.dispatch({ changes: { from: 0, to: current.length, insert: value } });
    lastExternal = value;
  });

  // 다크 모드 변경 시: extensions 재구성을 위해 state 교체
  $effect(() => {
    if (!view) return;
    const _ = dark; // 추적
    const currentDoc = view.state.doc.toString();
    view.setState(
      EditorState.create({
        doc: currentDoc,
        extensions: buildExtensions(dark),
      }),
    );
  });
</script>

<div bind:this={host} class="w-full h-full bg-white dark:bg-slate-900 border-y border-slate-200 dark:border-slate-800"></div>
