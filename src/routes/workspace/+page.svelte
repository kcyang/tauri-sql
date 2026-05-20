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
  import { queryTabs } from "$lib/stores/queryTabs.svelte";
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
      const reuse =
        a && a.filePath === null && a.sql === "" && !a.result && !a.busy;
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
    const snapshot = active.sql;
    try {
      if (!saveAs && active.filePath) {
        const { invoke } = await import("$lib/api/invoke");
        await invoke<void>("write_text_file", {
          path: active.filePath,
          content: snapshot,
        });
        queryTabs.updateTab(tabId, { savedContent: snapshot });
        return;
      }
      const newPath = await filesApi.saveSqlFile(
        snapshot,
        active.filePath ?? undefined,
      );
      if (newPath) {
        queryTabs.updateTab(tabId, {
          filePath: newPath,
          savedContent: snapshot,
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
