# 다중 쿼리 탭 & 리사이즈 가능 Object Explorer — 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 워크스페이스에 다중 쿼리 탭을 추가하고, Object Explorer 폭을 마우스 드래그로 조절 가능하게 만든다.

**Architecture:** 신규 `queryTabs` 스토어가 탭 배열과 활성 탭을 관리하고, `QueryTabsBar`/`ObjectExplorerPane` 컴포넌트가 UI 를 담당한다. `+page.svelte` 는 활성 탭 기준으로 모든 쿼리/파일/결과 wiring 을 재배치한다. Rust 백엔드 변경 없음.

**Tech Stack:** Svelte 5 (runes), TypeScript, Tailwind v4, CodeMirror 6, AG Grid, Tauri 2

**Spec:** `docs/superpowers/specs/2026-05-19-query-tabs-and-resizable-explorer-design.md`

**Verification approach:** 본 프로젝트엔 단위 테스트 인프라가 없음. 각 task 후 `npm run check` (svelte-kit sync + svelte-check) 로 타입 검증을 한다. 마지막 task 에서 수동 E2E 시나리오로 동작을 확인한다.

**파일 구조 (한눈에):**

```
src/lib/stores/queryTabs.svelte.ts          (신규)  탭 상태/조작
src/lib/components/QueryTabsBar.svelte      (신규)  탭바
src/lib/components/ObjectExplorerPane.svelte(신규)  익스플로러 + 리사이즈 핸들
src/lib/components/QueryEditor.svelte       (수정)  onSelectionChange 콜백 prop 추가
src/routes/workspace/+page.svelte           (수정)  활성 탭 기반 재배선 + 단축키
context.md                                   (수정)  진행 상태/E2E 시나리오
```

---

## Task 1: queryTabs 스토어 — 골격과 dirty 헬퍼

**Files:**
- Create: `src/lib/stores/queryTabs.svelte.ts`

- [ ] **Step 1: 스토어 파일 생성 (타입 + 기본 구조)**

`src/lib/stores/queryTabs.svelte.ts` 생성:

```ts
// 다중 쿼리 탭 store.
// - 각 탭은 자기만의 SQL/결과/실행 상태를 가진다 (탭별 독립 실행).
// - 탭 영속화 없음 — 재시작 시 빈 탭 1개로 시작.
// - 활성 탭 변경 시 백그라운드 쿼리는 영향받지 않음.

import type { QueryResult, SqlClassification } from "$lib/api/query";
import { queryApi } from "$lib/api/query";

export interface QueryTab {
  id: string;
  title: string;
  sql: string;
  selectedSql: string;
  filePath: string | null;
  savedContent: string | null;
  result: QueryResult | null;
  runError: string | null;
  busy: boolean;
  currentQueryId: string | null;
  pendingClassification: SqlClassification | null;
  exporting: boolean;
}

export function isDirty(t: QueryTab): boolean {
  if (t.filePath !== null) return t.sql !== t.savedContent;
  return t.sql.trim() !== "";
}

function newId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return Math.random().toString(36).slice(2) + Date.now().toString(36);
}

function emptyTab(title: string): QueryTab {
  return {
    id: newId(),
    title,
    sql: "",
    selectedSql: "",
    filePath: null,
    savedContent: null,
    result: null,
    runError: null,
    busy: false,
    currentQueryId: null,
    pendingClassification: null,
    exporting: false,
  };
}

class QueryTabsStore {
  tabs = $state<QueryTab[]>([]);
  activeId = $state<string | null>(null);
  #draftCounter = 1;

  get active(): QueryTab | null {
    if (!this.activeId) return null;
    return this.tabs.find((t) => t.id === this.activeId) ?? null;
  }

  init(): void {
    if (this.tabs.length === 0) {
      const t = emptyTab(`쿼리 ${this.#draftCounter++}`);
      this.tabs.push(t);
      this.activeId = t.id;
    }
  }

  addTab(opts?: { sql?: string; filePath?: string; title?: string }): string {
    const t = emptyTab(opts?.title ?? `쿼리 ${this.#draftCounter++}`);
    if (opts?.sql !== undefined) {
      t.sql = opts.sql;
      t.savedContent = opts.filePath ? opts.sql : null;
    }
    if (opts?.filePath) t.filePath = opts.filePath;
    this.tabs.push(t);
    this.activeId = t.id;
    return t.id;
  }

  switchTo(id: string): void {
    if (this.tabs.some((t) => t.id === id)) this.activeId = id;
  }

  closeTab(id: string): { needsConfirm: boolean } {
    const t = this.tabs.find((x) => x.id === id);
    if (!t) return { needsConfirm: false };
    return { needsConfirm: isDirty(t) };
  }

  forceCloseTab(id: string): void {
    const idx = this.tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;
    const t = this.tabs[idx];
    // 백그라운드 쿼리 정리 (await 안 함 — UI 흐름 막지 않음)
    if (t.currentQueryId) {
      queryApi.cancel(t.currentQueryId).catch((e) => {
        console.warn("탭 닫기 중 cancel 실패:", e);
      });
    }
    this.tabs.splice(idx, 1);
    if (this.activeId === id) {
      const next = this.tabs[idx] ?? this.tabs[idx - 1] ?? null;
      this.activeId = next?.id ?? null;
    }
    if (this.tabs.length === 0) {
      const fresh = emptyTab(`쿼리 ${this.#draftCounter++}`);
      this.tabs.push(fresh);
      this.activeId = fresh.id;
    }
  }

  updateTab(id: string, patch: Partial<QueryTab>): void {
    const idx = this.tabs.findIndex((t) => t.id === id);
    if (idx === -1) return;
    this.tabs[idx] = { ...this.tabs[idx], ...patch };
  }

  updateActive(patch: Partial<QueryTab>): void {
    if (this.activeId) this.updateTab(this.activeId, patch);
  }
}

export const queryTabs = new QueryTabsStore();
```

- [ ] **Step 2: 타입 검증**

Run: `npm run check`
Expected: 0 errors. (테마/세션 스토어와 일관된 스타일이라 통과해야 함)

- [ ] **Step 3: 커밋**

```bash
git add src/lib/stores/queryTabs.svelte.ts
git commit -m "feat: queryTabs 스토어 추가 (다중 쿼리 탭 상태 관리)"
```

---

## Task 2: QueryTabsBar 컴포넌트

**Files:**
- Create: `src/lib/components/QueryTabsBar.svelte`

- [ ] **Step 1: 컴포넌트 작성**

`src/lib/components/QueryTabsBar.svelte` 생성:

```svelte
<script lang="ts">
  import { queryTabs, isDirty } from "$lib/stores/queryTabs.svelte";

  type Props = {
    /** 닫기 요청 — 부모가 dirty 확인 후 forceCloseTab 호출 */
    onRequestClose: (id: string) => void;
    onAddTab: () => void;
  };
  let { onRequestClose, onAddTab }: Props = $props();

  function onTabClick(id: string, e: MouseEvent) {
    if (e.button === 1) {
      // 가운데 클릭 = 닫기
      e.preventDefault();
      onRequestClose(id);
      return;
    }
    queryTabs.switchTo(id);
  }
</script>

<div
  class="flex items-stretch border-b border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-900 overflow-x-auto"
  role="tablist"
>
  {#each queryTabs.tabs as t (t.id)}
    {@const active = queryTabs.activeId === t.id}
    {@const dirty = isDirty(t)}
    <div
      class="group flex items-center gap-1.5 px-3 py-1.5 text-sm border-r border-slate-200 dark:border-slate-800 cursor-pointer select-none whitespace-nowrap
      {active
        ? 'bg-white dark:bg-slate-800 text-slate-900 dark:text-slate-100 border-b-2 border-b-emerald-500 -mb-px'
        : 'text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/60'}"
      role="tab"
      tabindex="0"
      aria-selected={active}
      onmousedown={(e) => onTabClick(t.id, e)}
      onkeydown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          queryTabs.switchTo(t.id);
        }
      }}
      title={t.filePath ?? t.title}
    >
      <span class="w-2 text-rose-500" aria-hidden="true">
        {dirty ? "●" : ""}
      </span>
      <span class="max-w-[14rem] truncate">{t.title}</span>
      <button
        type="button"
        class="ml-1 w-4 h-4 inline-flex items-center justify-center rounded text-slate-400 hover:text-slate-700 dark:hover:text-slate-100 hover:bg-slate-200 dark:hover:bg-slate-700 opacity-0 group-hover:opacity-100 focus:opacity-100
        {active ? 'opacity-60' : ''}"
        aria-label="탭 닫기"
        onclick={(e) => {
          e.stopPropagation();
          onRequestClose(t.id);
        }}
        onmousedown={(e) => e.stopPropagation()}
      >
        ×
      </button>
    </div>
  {/each}

  <button
    type="button"
    class="px-3 py-1.5 text-sm text-slate-500 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/60 border-r border-slate-200 dark:border-slate-800"
    title="새 탭 (⌘T)"
    onclick={onAddTab}
  >
    +
  </button>
  <div class="flex-1"></div>
</div>
```

- [ ] **Step 2: 타입 검증**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 3: 커밋**

```bash
git add src/lib/components/QueryTabsBar.svelte
git commit -m "feat: QueryTabsBar 컴포넌트 추가 (탭 칩 + 닫기/추가 버튼)"
```

---

## Task 3: ObjectExplorerPane (드래그 리사이즈)

**Files:**
- Create: `src/lib/components/ObjectExplorerPane.svelte`

- [ ] **Step 1: 컴포넌트 작성**

`src/lib/components/ObjectExplorerPane.svelte` 생성:

```svelte
<script lang="ts">
  import ObjectExplorer from "./ObjectExplorer.svelte";
  import type { DbObject, ObjectKind } from "$lib/api/explorer";

  type Props = {
    sessionId: string;
    onObjectActivate?: (database: string, kind: ObjectKind, obj: DbObject) => void;
  };
  let { sessionId, onObjectActivate }: Props = $props();

  const STORAGE_KEY = "tauri-sql:explorer-width";
  const MIN_W = 180;
  const MAX_W = 600;
  const DEFAULT_W = 256;

  function readStored(): number {
    if (typeof localStorage === "undefined") return DEFAULT_W;
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_W;
    const n = Number(raw);
    if (!Number.isFinite(n)) return DEFAULT_W;
    return Math.max(MIN_W, Math.min(MAX_W, n));
  }

  let width = $state(readStored());
  let dragging = $state(false);

  function onHandleMouseDown(e: MouseEvent) {
    dragging = true;
    const startX = e.clientX;
    const startW = width;
    function onMove(ev: MouseEvent) {
      const dx = ev.clientX - startX;
      width = Math.max(MIN_W, Math.min(MAX_W, startW + dx));
    }
    function onUp() {
      dragging = false;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      try {
        localStorage.setItem(STORAGE_KEY, String(width));
      } catch (e) {
        console.warn("explorer width 저장 실패:", e);
      }
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    e.preventDefault();
  }
</script>

<aside
  class="flex-shrink-0 overflow-hidden relative flex"
  style="width: {width}px"
>
  <div class="flex-1 min-w-0 overflow-hidden">
    <ObjectExplorer {sessionId} {onObjectActivate} />
  </div>
  <!-- 리사이즈 핸들 -->
  <button
    type="button"
    aria-label="Object Explorer 폭 조절"
    class="w-1.5 cursor-col-resize bg-slate-200 dark:bg-slate-800 hover:bg-slate-400 dark:hover:bg-slate-600 transition-colors
    {dragging ? 'bg-slate-400 dark:bg-slate-500' : ''}"
    onmousedown={onHandleMouseDown}
  ></button>
</aside>
```

- [ ] **Step 2: 타입 검증**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 3: 커밋**

```bash
git add src/lib/components/ObjectExplorerPane.svelte
git commit -m "feat: ObjectExplorerPane 추가 (드래그 + localStorage 폭 저장)"
```

---

## Task 4: QueryEditor — onSelectionChange 콜백 추가

`active` 는 `$derived` 값이라 `active.selectedSql` 에 직접 `bind:` 할 수 없다. QueryEditor 에 selection 변경 콜백을 추가해 부모가 직접 store 에 push 하게 한다. `selectedText` bindable 은 호환을 위해 그대로 둔다 (선택적 prop).

**Files:**
- Modify: `src/lib/components/QueryEditor.svelte`

- [ ] **Step 1: Props 타입에 onSelectionChange 추가**

`src/lib/components/QueryEditor.svelte` 의 Props 타입과 props 분해를 수정:

```ts
type Props = {
    value: string;
    dark: boolean;
    /** 현재 선택된 텍스트 — 선택 영역이 비어 있으면 빈 문자열. 부모가 bind 해서 사용 */
    selectedText?: string;
    /** selection 변경을 받는 콜백 (bind 대신 쓸 때) */
    onSelectionChange?: (text: string) => void;
    onChange: (value: string) => void;
    onRun?: () => void;
  };
  let {
    value,
    dark,
    selectedText = $bindable(""),
    onSelectionChange,
    onChange,
    onRun,
  }: Props = $props();
```

- [ ] **Step 2: updateListener 에서 콜백 호출**

`updateListener` 의 selection 처리 블록을 다음으로 교체:

```ts
if (update.selectionSet || update.docChanged) {
  const main = update.state.selection.main;
  const text = main.empty
    ? ""
    : update.state.doc.sliceString(main.from, main.to);
  if (selectedText !== text) selectedText = text;
  onSelectionChange?.(text);
}
```

- [ ] **Step 3: 타입 검증**

Run: `npm run check`
Expected: 0 errors. (기존 bind 호출자 영향 없음)

- [ ] **Step 4: 커밋**

```bash
git add src/lib/components/QueryEditor.svelte
git commit -m "feat: QueryEditor 에 onSelectionChange 콜백 추가"
```

---

## Task 5: +page.svelte 재배선 — 활성 탭 기반

이 task 는 한 파일에서 큰 폭 변경이므로 한 step 에서 통째로 교체 후 검증한다. 실수 위험을 줄이기 위해 **현재 파일을 먼저 읽고 비교** 한다.

**Files:**
- Modify: `src/routes/workspace/+page.svelte` (전체 교체)

- [ ] **Step 1: 현재 파일 확인**

```bash
wc -l src/routes/workspace/+page.svelte
```
Expected: 약 426줄.

- [ ] **Step 2: 전체 교체**

`src/routes/workspace/+page.svelte` 의 전체 내용을 아래로 교체:

```svelte
<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { v4 as uuidv4 } from "uuid";
  import { sessionApi } from "$lib/api/session";
  import {
    queryApi,
    type QueryResult,
    type SqlClassification,
  } from "$lib/api/query";
  import type { DbObject, ObjectKind } from "$lib/api/explorer";
  import { filesApi } from "$lib/api/files";
  import { sessionStore } from "$lib/stores/session.svelte";
  import { themeStore } from "$lib/stores/theme.svelte";
  import { queryTabs, isDirty } from "$lib/stores/queryTabs.svelte";
  import { errorMessage } from "$lib/types";
  import QueryEditor from "$lib/components/QueryEditor.svelte";
  import ResultGrid from "$lib/components/ResultGrid.svelte";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import ObjectExplorerPane from "$lib/components/ObjectExplorerPane.svelte";
  import QueryTabsBar from "$lib/components/QueryTabsBar.svelte";
  import ThemeToggle from "$lib/components/ThemeToggle.svelte";
  import QueryRunningOverlay from "$lib/components/QueryRunningOverlay.svelte";

  // 전역 (탭 공유) 설정
  let maxRows = $state(1000);
  let timeoutSec = $state(30);
  let sortable = $state(false);

  // 에디터/그리드 높이 분할 (UI 전역)
  let editorHeightPx = $state(220);
  let dragging = $state(false);

  // 탭 닫기 확인 다이얼로그
  let pendingCloseId = $state<string | null>(null);

  // 활성 탭 — 항상 존재해야 함 (onMount 에서 init)
  const active = $derived(queryTabs.active);
  const hasSelection = $derived(
    !!active && active.selectedSql.trim().length > 0,
  );

  function onKeyDown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (!mod) return;
    const key = e.key.toLowerCase();
    if (e.key === ".") {
      if (active?.currentQueryId) {
        e.preventDefault();
        onCancelPressed();
      }
    } else if (key === "o") {
      e.preventDefault();
      openFile();
    } else if (key === "s") {
      e.preventDefault();
      saveFile(e.shiftKey);
    } else if (key === "t") {
      e.preventDefault();
      queryTabs.addTab();
    } else if (key === "w") {
      e.preventDefault();
      if (active) requestCloseTab(active.id);
    }
  }

  function onSplitMouseDown(e: MouseEvent) {
    dragging = true;
    const startY = e.clientY;
    const startHeight = editorHeightPx;
    function onMove(ev: MouseEvent) {
      const dy = ev.clientY - startY;
      editorHeightPx = Math.max(80, Math.min(900, startHeight + dy));
    }
    function onUp() {
      dragging = false;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    }
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    e.preventDefault();
  }

  onMount(() => {
    if (!sessionStore.current) {
      goto("/");
      return;
    }
    queryTabs.init();
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  async function disconnect() {
    const s = sessionStore.current;
    sessionStore.clear();
    if (s) {
      try {
        await sessionApi.close(s.sessionId);
      } catch (e) {
        console.warn("close_session 실패:", e);
      }
    }
    goto("/");
  }

  function getSqlToRun(t: { sql: string; selectedSql: string }): string {
    const sel = t.selectedSql.trim();
    return sel ? t.selectedSql : t.sql;
  }

  async function onRunPressed() {
    if (!sessionStore.current) {
      if (active) queryTabs.updateTab(active.id, { runError: "세션이 없습니다." });
      return;
    }
    if (!active) return;
    const tabId = active.id;
    const target = getSqlToRun(active);
    if (!target.trim()) {
      queryTabs.updateTab(tabId, { runError: "SQL 이 비어 있습니다." });
      return;
    }
    queryTabs.updateTab(tabId, { runError: null });
    try {
      const cls = await queryApi.classify(target);
      if (cls.kind === "empty") {
        queryTabs.updateTab(tabId, { runError: "SQL 이 비어 있습니다." });
        return;
      }
      if (cls.kind === "write" || cls.kind === "ddl" || cls.kind === "unknown") {
        queryTabs.updateTab(tabId, { pendingClassification: cls });
        return;
      }
      await actuallyRun(tabId);
    } catch (e) {
      queryTabs.updateTab(tabId, { runError: errorMessage(e) });
    }
  }

  async function actuallyRun(tabId: string) {
    queryTabs.updateTab(tabId, { pendingClassification: null });
    const s = sessionStore.current;
    if (!s) return;
    const t = queryTabs.tabs.find((x) => x.id === tabId);
    if (!t) return;
    const target = getSqlToRun(t);
    const queryId = uuidv4();
    queryTabs.updateTab(tabId, {
      currentQueryId: queryId,
      busy: true,
      runError: null,
      result: null,
    });
    try {
      const result = await queryApi.execute({
        session_id: s.sessionId,
        sql: target,
        query_id: queryId,
        max_rows: maxRows,
        timeout_ms: timeoutSec * 1000,
      });
      queryTabs.updateTab(tabId, { result, busy: false, currentQueryId: null });
    } catch (e) {
      queryTabs.updateTab(tabId, {
        runError: errorMessage(e),
        busy: false,
        currentQueryId: null,
      });
    }
  }

  async function onCancelPressed() {
    const qid = active?.currentQueryId;
    if (!qid) return;
    try {
      await queryApi.cancel(qid);
    } catch (e) {
      console.warn("cancel_query 실패:", e);
    }
  }

  function onObjectActivate(database: string, kind: ObjectKind, obj: DbObject) {
    const escapedDb = database.replace(/]/g, "]]");
    const escapedSchema = obj.schema.replace(/]/g, "]]");
    const escapedName = obj.name.replace(/]/g, "]]");
    const next =
      kind === "procedure"
        ? `EXEC [${escapedDb}].[${escapedSchema}].[${escapedName}];\n`
        : `SELECT TOP 100 *\nFROM [${escapedDb}].[${escapedSchema}].[${escapedName}];\n`;
    if (!active) return;
    queryTabs.updateTab(active.id, { sql: next });
  }

  async function openFile() {
    try {
      const file = await filesApi.openSqlFile();
      if (!file) return;
      // 활성 탭이 빈 드래프트면 재사용, 아니면 새 탭으로
      const a = queryTabs.active;
      const reuse = a && a.filePath === null && a.sql === "";
      if (reuse && a) {
        queryTabs.updateTab(a.id, {
          sql: file.content,
          filePath: file.path,
          savedContent: file.content,
          title: basename(file.path),
          runError: null,
          result: null,
        });
      } else {
        queryTabs.addTab({
          sql: file.content,
          filePath: file.path,
          title: basename(file.path),
        });
      }
    } catch (e) {
      if (active)
        queryTabs.updateTab(active.id, {
          runError: `파일 열기 실패: ${errorMessage(e)}`,
        });
    }
  }

  function basename(p: string): string {
    return p.split(/[\\/]/).pop() ?? p;
  }

  async function saveFile(saveAs: boolean) {
    if (!active) return;
    const tabId = active.id;
    try {
      if (!saveAs && active.filePath) {
        const { invoke } = await import("$lib/api/invoke");
        await invoke<void>("write_text_file", {
          path: active.filePath,
          content: active.sql,
        });
        queryTabs.updateTab(tabId, { savedContent: active.sql });
        return;
      }
      const newPath = await filesApi.saveSqlFile(
        active.sql,
        active.filePath ?? undefined,
      );
      if (newPath) {
        queryTabs.updateTab(tabId, {
          filePath: newPath,
          savedContent: active.sql,
          title: basename(newPath),
        });
      }
    } catch (e) {
      queryTabs.updateTab(tabId, {
        runError: `파일 저장 실패: ${errorMessage(e)}`,
      });
    }
  }

  async function exportToExcel() {
    if (!active || !active.result) return;
    const tabId = active.id;
    queryTabs.updateTab(tabId, { exporting: true });
    try {
      await filesApi.exportResultToXlsx(active.result);
      queryTabs.updateTab(tabId, { runError: null });
    } catch (e) {
      queryTabs.updateTab(tabId, {
        runError: `Excel 저장 실패: ${errorMessage(e)}`,
      });
    } finally {
      queryTabs.updateTab(tabId, { exporting: false });
    }
  }

  function requestCloseTab(id: string) {
    const { needsConfirm } = queryTabs.closeTab(id);
    if (needsConfirm) {
      pendingCloseId = id;
    } else {
      queryTabs.forceCloseTab(id);
    }
  }

  function confirmCloseTab() {
    if (pendingCloseId) {
      queryTabs.forceCloseTab(pendingCloseId);
      pendingCloseId = null;
    }
  }

  function confirmDialogTitle(c: SqlClassification): string {
    switch (c.kind) {
      case "ddl":
        return "DDL 쿼리 실행 확인";
      case "write":
        return "쓰기 쿼리 실행 확인";
      case "unknown":
        return "쿼리 실행 확인";
      default:
        return "실행 확인";
    }
  }
  function confirmDialogMessage(c: SqlClassification): string {
    switch (c.kind) {
      case "ddl":
        return "이 쿼리는 스키마를 변경합니다(CREATE/ALTER/DROP 등). 실행할까요?";
      case "write":
        return "이 쿼리는 데이터를 변경합니다(INSERT/UPDATE/DELETE 등). 실행할까요?";
      case "unknown":
        return "이 쿼리의 영향 범위를 자동 분류할 수 없습니다(예: EXEC). 실행할까요?";
      default:
        return "실행할까요?";
    }
  }

  // QueryEditor onChange — 활성 탭의 sql 만 갱신
  function onEditorChange(v: string) {
    if (active) queryTabs.updateTab(active.id, { sql: v });
  }

  // QueryEditor selection 변경 — 활성 탭의 selectedSql 갱신
  function onEditorSelectionChange(text: string) {
    if (active && active.selectedSql !== text) {
      queryTabs.updateTab(active.id, { selectedSql: text });
    }
  }
</script>

<div class="flex-1 flex flex-col bg-slate-50 dark:bg-slate-950 h-full">
  <header
    class="px-4 py-2 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 flex items-center justify-between"
  >
    <div class="text-sm min-w-0 flex items-center gap-2">
      <span class="font-medium text-slate-900 dark:text-slate-100">
        {sessionStore.current?.profile.name ?? "—"}
      </span>
      <span class="text-slate-500 dark:text-slate-400 truncate">
        {sessionStore.current?.profile.username}@{sessionStore.current?.profile
          .host}:{sessionStore.current?.profile.port} / {sessionStore.current
          ?.profile.database}
      </span>
      {#if active?.filePath}
        <span
          class="text-xs text-slate-400 dark:text-slate-500 truncate"
          title={active.filePath}
        >
          📄 {basename(active.filePath)}
        </span>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <ThemeToggle />
      <button
        class="px-3 py-1.5 text-sm rounded-md border border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-700"
        onclick={disconnect}
      >
        연결 해제
      </button>
    </div>
  </header>

  <main class="flex-1 flex overflow-hidden">
    {#if sessionStore.current}
      <ObjectExplorerPane
        sessionId={sessionStore.current.sessionId}
        {onObjectActivate}
      />
    {/if}

    <section class="flex-1 flex flex-col overflow-hidden min-w-0">
      <QueryTabsBar
        onRequestClose={requestCloseTab}
        onAddTab={() => queryTabs.addTab()}
      />

      <div
        class="px-3 py-2 bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 flex items-center gap-2 flex-wrap"
      >
        {#if active?.busy}
          <button
            class="px-4 py-1.5 text-sm rounded-md bg-rose-600 text-white hover:bg-rose-500"
            onclick={onCancelPressed}
          >
            ■ 취소 <span class="text-xs opacity-75">(⌘.)</span>
          </button>
        {:else}
          <button
            class="px-4 py-1.5 text-sm rounded-md bg-emerald-600 text-white hover:bg-emerald-500"
            onclick={onRunPressed}
            title={hasSelection ? "선택한 영역만 실행" : "전체 SQL 실행"}
          >
            ▶ {hasSelection ? "선택 실행" : "실행"}
            <span class="text-xs opacity-75">(⌘↵)</span>
          </button>
        {/if}

        <span class="w-px h-6 bg-slate-200 dark:bg-slate-700 mx-1"></span>

        <button
          class="px-2.5 py-1.5 text-sm rounded-md border border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700"
          title="파일 열기 (⌘O)"
          onclick={openFile}
        >
          📂 열기
        </button>
        <button
          class="px-2.5 py-1.5 text-sm rounded-md border border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700"
          title="저장 (⌘S, ⇧⌘S = 다른 이름으로)"
          onclick={() => saveFile(false)}
        >
          💾 저장
        </button>
        <button
          class="px-2.5 py-1.5 text-sm rounded-md border border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700 disabled:opacity-50"
          title="결과를 Excel(.xlsx) 로 저장"
          onclick={exportToExcel}
          disabled={!active?.result || active?.exporting}
        >
          📊 {active?.exporting ? "내보내는 중…" : "Excel 저장"}
        </button>

        <span class="w-px h-6 bg-slate-200 dark:bg-slate-700 mx-1"></span>

        <label
          class="text-sm text-slate-600 dark:text-slate-300 flex items-center gap-2"
        >
          최대 행
          <select
            bind:value={maxRows}
            class="border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-slate-800 dark:text-slate-100 rounded px-2 py-1 text-sm"
          >
            <option value={100}>100</option>
            <option value={1000}>1,000</option>
            <option value={10000}>10,000</option>
            <option value={100000}>100,000</option>
          </select>
        </label>
        <label
          class="text-sm text-slate-600 dark:text-slate-300 flex items-center gap-2"
        >
          타임아웃
          <select
            bind:value={timeoutSec}
            class="border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-slate-800 dark:text-slate-100 rounded px-2 py-1 text-sm"
          >
            <option value={10}>10초</option>
            <option value={30}>30초</option>
            <option value={60}>60초</option>
            <option value={300}>5분</option>
            <option value={3600}>1시간</option>
          </select>
        </label>
        <label
          class="text-sm text-slate-600 dark:text-slate-300 flex items-center gap-1.5"
        >
          <input type="checkbox" bind:checked={sortable} />
          정렬 허용
        </label>

        <div class="flex-1"></div>

        {#if active?.result}
          <span class="text-sm text-slate-500 dark:text-slate-400">
            {active.result.row_count}행 · {active.result.elapsed_ms}ms
            {#if active.result.truncated}
              <span class="text-amber-600 dark:text-amber-400 ml-1"
                >· 잘림</span
              >
            {/if}
          </span>
        {/if}
        {#if active?.runError}
          <span
            class="text-sm text-rose-700 dark:text-rose-300 truncate max-w-[40%]"
            title={active.runError}
          >
            에러: {active.runError}
          </span>
        {/if}
      </div>

      <div class="flex flex-col flex-1 min-h-0">
        <div
          style="height: {editorHeightPx}px"
          class="min-h-[5rem] overflow-hidden"
        >
          {#if active}
            {#key active.id}
              <QueryEditor
                value={active.sql}
                dark={themeStore.isDark}
                onChange={onEditorChange}
                onSelectionChange={onEditorSelectionChange}
                onRun={onRunPressed}
              />
            {/key}
          {/if}
        </div>

        <button
          type="button"
          aria-label="에디터/결과 높이 조절"
          class="h-1.5 cursor-row-resize bg-slate-200 dark:bg-slate-800 hover:bg-slate-400 dark:hover:bg-slate-600 transition-colors w-full
          {dragging ? 'bg-slate-400 dark:bg-slate-500' : ''}"
          onmousedown={onSplitMouseDown}
        ></button>

        <div class="flex-1 min-h-0 relative">
          <ResultGrid
            result={active?.result ?? null}
            {sortable}
            dark={themeStore.isDark}
          />
          <QueryRunningOverlay
            show={!!active?.busy}
            onCancel={onCancelPressed}
          />
        </div>
      </div>
    </section>
  </main>
</div>

{#if active?.pendingClassification}
  <ConfirmDialog
    title={confirmDialogTitle(active.pendingClassification)}
    message={confirmDialogMessage(active.pendingClassification)}
    detail={active.pendingClassification.keywords.length > 0
      ? `감지된 키워드: ${active.pendingClassification.keywords.join(", ")}`
      : null}
    confirmLabel="실행"
    danger={active.pendingClassification.kind === "ddl"}
    onConfirm={() => active && actuallyRun(active.id)}
    onCancel={() =>
      active && queryTabs.updateTab(active.id, { pendingClassification: null })}
  />
{/if}

{#if pendingCloseId}
  <ConfirmDialog
    title="탭 닫기"
    message="이 탭의 변경 사항이 저장되지 않았습니다. 닫을까요?"
    detail={null}
    confirmLabel="닫기"
    danger={true}
    onConfirm={confirmCloseTab}
    onCancel={() => (pendingCloseId = null)}
  />
{/if}
```

주요 변경 포인트:
- selection 은 Task 4 에서 추가한 `onSelectionChange` 콜백으로 스토어에 push (bindable 미사용).
- `{#key active.id}` 로 탭 전환 시 CodeMirror state 를 새로 만들도록 강제 → 탭마다 독립 selection 보장.
- 단축키 ⌘T/⌘W 추가, ⌘. 은 active 기준으로 동작.
- 헤더의 파일명 표시는 활성 탭의 `filePath` 로부터 계산.

- [ ] **Step 3: 타입 검증**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: 커밋**

```bash
git add src/routes/workspace/+page.svelte
git commit -m "feat: 워크스페이스 페이지를 활성 탭 기반으로 재배선 (⌘T/⌘W 추가)"
```

---

## Task 6: context.md 갱신

**Files:**
- Modify: `context.md`

- [ ] **Step 1: 진행 상태 섹션과 V2 항목 갱신**

`context.md` 의 진행 상태 섹션에 추가:

```markdown
- [x] 후속5: 다중 쿼리 탭(독립 실행), Object Explorer 폭 드래그(localStorage 저장)
```

V2 목록에서 "다중 쿼리 탭" 항목 제거.

E2E 섹션에 다음 시나리오 추가:

```markdown
10. ⌘T 로 탭 2개 만들고 한쪽 `WAITFOR DELAY '00:00:05'` 실행 → 즉시 다른 탭으로 전환 → 빠른 쿼리 실행 → 두 결과가 각각 표시
11. dirty 탭 ⌘W → 확인 다이얼로그 → 취소/닫기 동작
12. 마지막 탭 ⌘W → 빈 새 탭 자동 생성
13. Object Explorer 핸들 좌우 드래그 → 180/600 한계 클램프, 재시작 후 폭 유지
```

- [ ] **Step 2: 커밋**

```bash
git add context.md
git commit -m "docs: context.md — 다중 탭/익스플로러 리사이즈 진행 반영"
```

---

## Task 7: 수동 E2E 검증

이 task 는 **사용자 환경(macOS + Docker SQL Server)** 에서만 가능. 구현 에이전트는 빌드 가능 여부만 확인하고, 시나리오 항목들을 사용자에게 직접 확인 요청한다.

**Files:** (변경 없음)

- [ ] **Step 1: 프로덕션 빌드 검증**

Run: `npm run build`
Expected: SvelteKit 빌드 성공, 에러 없음.

- [ ] **Step 2: 사용자 시나리오 확인 요청**

다음 시나리오를 사용자에게 확인 요청:

1. `npm run tauri dev` 로 앱 실행
2. 로그인 후 워크스페이스 진입 — 탭 1개 "쿼리 1" 표시되는지
3. ⌘T → 새 탭 "쿼리 2" 생성, 활성화되는지
4. 탭 1 에 `WAITFOR DELAY '00:00:05'; SELECT 1` 작성 → 실행 → 즉시 탭 2 로 전환 → 탭 2 에서 `SELECT 2` 실행 → 양쪽 결과가 각자 그 탭에서 표시되는지
5. 탭 1 에서 ⌘O 로 SQL 파일 열기 → 내용을 수정 → ⌘W → 확인 다이얼로그 → "닫기" 선택 시 닫히는지
6. Object Explorer 오른쪽 경계를 마우스로 좌우 드래그 → 너비가 부드럽게 변하는지, 180/600 한계가 적용되는지
7. 앱 재시작 → 익스플로러 너비가 이전 값으로 복원되는지
8. 마지막 탭 ⌘W → 즉시 새 빈 탭이 자동 생성되는지

- [ ] **Step 3: 사용자 confirm 후 종료**

빌드/타입체크는 통과했고 사용자 시나리오가 OK 이면 task 완료. 문제 있으면 해당 task 로 돌아가 수정.

---

## 회귀 위험 체크리스트

구현 후 다음이 깨지지 않았는지 빠르게 확인:

- [ ] SQL 파일 열기/저장 동작 (⌘O / ⌘S / ⇧⌘S)
- [ ] write/ddl/unknown 확인 다이얼로그 (sys.objects 가 아닌 `UPDATE` 같은 쿼리)
- [ ] 쿼리 취소 (⌘. / 오버레이 취소)
- [ ] Excel 내보내기
- [ ] 정렬 토글, 최대 행/타임아웃 변경이 즉시 다음 실행에 반영
- [ ] Object Explorer 항목 더블클릭 → 활성 탭에 SELECT 들어가는지
- [ ] 다크모드/라이트모드 전환 시 탭바/핸들 색
- [ ] 앱 종료 시 RunEvent::ExitRequested → 백그라운드 실행 중인 모든 탭의 쿼리 정리

---

## 비범위 (Out of Scope) — spec 과 동일

- 탭 드래그 재정렬
- 탭 영속화
- 탭별 결과 히스토리
- 윈도우 분할

향후 spec 으로 분리.
