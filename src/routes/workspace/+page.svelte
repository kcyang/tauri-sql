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
  import { errorMessage } from "$lib/types";
  import QueryEditor from "$lib/components/QueryEditor.svelte";
  import ResultGrid from "$lib/components/ResultGrid.svelte";
  import ConfirmDialog from "$lib/components/ConfirmDialog.svelte";
  import ObjectExplorer from "$lib/components/ObjectExplorer.svelte";
  import ThemeToggle from "$lib/components/ThemeToggle.svelte";
  import QueryRunningOverlay from "$lib/components/QueryRunningOverlay.svelte";

  let sql = $state(
    "SELECT TOP 100 name, object_id, type_desc, create_date\nFROM sys.objects\nORDER BY create_date DESC;",
  );
  // 에디터의 현재 선택 영역 텍스트 (QueryEditor 와 양방향 bind)
  let selectedSql = $state("");
  let currentFilePath = $state<string | null>(null);
  let busy = $state(false);
  let result = $state<QueryResult | null>(null);
  let runError = $state<string | null>(null);
  let maxRows = $state(1000);
  let timeoutSec = $state(30);
  let currentQueryId = $state<string | null>(null);
  let pendingClassification = $state<SqlClassification | null>(null);
  let sortable = $state(false);

  // 에디터/그리드 높이 분할
  let editorHeightPx = $state(220);
  let dragging = $state(false);

  // 전역 단축키 (⌘. 취소, ⌘O 열기, ⌘S 저장)
  function onKeyDown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    if (!mod) return;
    if (e.key === ".") {
      if (currentQueryId) {
        e.preventDefault();
        onCancelPressed();
      }
    } else if (e.key.toLowerCase() === "o") {
      e.preventDefault();
      openFile();
    } else if (e.key.toLowerCase() === "s") {
      e.preventDefault();
      saveFile(e.shiftKey); // Shift 누르면 다른 이름으로 저장
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
    }
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

  /** 실행 시점에 보낼 SQL — 선택 영역이 있으면 그 부분만, 없으면 전체 */
  function getSqlToRun(): string {
    const sel = selectedSql.trim();
    return sel ? selectedSql : sql;
  }

  const hasSelection = $derived(selectedSql.trim().length > 0);

  async function onRunPressed() {
    if (!sessionStore.current) {
      runError = "세션이 없습니다.";
      return;
    }
    const target = getSqlToRun();
    if (!target.trim()) {
      runError = "SQL 이 비어 있습니다.";
      return;
    }
    runError = null;
    try {
      const cls = await queryApi.classify(target);
      if (cls.kind === "empty") {
        runError = "SQL 이 비어 있습니다.";
        return;
      }
      if (cls.kind === "write" || cls.kind === "ddl" || cls.kind === "unknown") {
        pendingClassification = cls;
        return;
      }
      await actuallyRun();
    } catch (e) {
      runError = errorMessage(e);
    }
  }

  async function actuallyRun() {
    pendingClassification = null;
    const s = sessionStore.current;
    if (!s) return;
    const target = getSqlToRun();
    const queryId = uuidv4();
    currentQueryId = queryId;
    busy = true;
    runError = null;
    result = null;
    try {
      result = await queryApi.execute({
        session_id: s.sessionId,
        sql: target,
        query_id: queryId,
        max_rows: maxRows,
        timeout_ms: timeoutSec * 1000,
      });
    } catch (e) {
      runError = errorMessage(e);
    } finally {
      busy = false;
      currentQueryId = null;
    }
  }

  async function onCancelPressed() {
    if (!currentQueryId) return;
    try {
      await queryApi.cancel(currentQueryId);
    } catch (e) {
      console.warn("cancel_query 실패:", e);
    }
  }

  function onObjectActivate(database: string, kind: ObjectKind, obj: DbObject) {
    const escapedDb = database.replace(/]/g, "]]");
    const escapedSchema = obj.schema.replace(/]/g, "]]");
    const escapedName = obj.name.replace(/]/g, "]]");
    if (kind === "procedure") {
      sql = `EXEC [${escapedDb}].[${escapedSchema}].[${escapedName}];\n`;
    } else {
      sql = `SELECT TOP 100 *\nFROM [${escapedDb}].[${escapedSchema}].[${escapedName}];\n`;
    }
  }

  async function openFile() {
    try {
      const file = await filesApi.openSqlFile();
      if (file) {
        sql = file.content;
        currentFilePath = file.path;
        runError = null;
      }
    } catch (e) {
      runError = `파일 열기 실패: ${errorMessage(e)}`;
    }
  }

  async function saveFile(saveAs: boolean) {
    try {
      // saveAs 가 false 이고 이미 경로가 있으면 그 경로로 바로 저장
      if (!saveAs && currentFilePath) {
        const { invoke } = await import("$lib/api/invoke");
        await invoke<void>("write_text_file", {
          path: currentFilePath,
          content: sql,
        });
        return;
      }
      const newPath = await filesApi.saveSqlFile(sql, currentFilePath ?? undefined);
      if (newPath) currentFilePath = newPath;
    } catch (e) {
      runError = `파일 저장 실패: ${errorMessage(e)}`;
    }
  }

  let exporting = $state(false);

  async function exportToExcel() {
    if (!result) return;
    exporting = true;
    try {
      const path = await filesApi.exportResultToXlsx(result);
      if (path) {
        // 성공 토스트 대용 — 상태 영역에 잠깐 표시
        runError = null;
      }
    } catch (e) {
      runError = `Excel 저장 실패: ${errorMessage(e)}`;
    } finally {
      exporting = false;
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
</script>

<div class="flex-1 flex flex-col bg-slate-50 dark:bg-slate-950 h-full">
  <header class="px-4 py-2 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 flex items-center justify-between">
    <div class="text-sm min-w-0 flex items-center gap-2">
      <span class="font-medium text-slate-900 dark:text-slate-100">
        {sessionStore.current?.profile.name ?? "—"}
      </span>
      <span class="text-slate-500 dark:text-slate-400 truncate">
        {sessionStore.current?.profile.username}@{sessionStore.current?.profile.host}:{sessionStore.current?.profile.port}
        / {sessionStore.current?.profile.database}
      </span>
      {#if currentFilePath}
        <span class="text-xs text-slate-400 dark:text-slate-500 truncate" title={currentFilePath}>
          📄 {currentFilePath.split("/").pop()}
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
    <aside class="w-64 flex-shrink-0 overflow-hidden">
      {#if sessionStore.current}
        <ObjectExplorer
          sessionId={sessionStore.current.sessionId}
          {onObjectActivate}
        />
      {/if}
    </aside>

    <section class="flex-1 flex flex-col overflow-hidden">
      <div class="px-3 py-2 bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 flex items-center gap-2 flex-wrap">
        {#if busy}
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
          disabled={!result || exporting}
        >
          📊 {exporting ? "내보내는 중…" : "Excel 저장"}
        </button>

        <span class="w-px h-6 bg-slate-200 dark:bg-slate-700 mx-1"></span>

        <label class="text-sm text-slate-600 dark:text-slate-300 flex items-center gap-2">
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
        <label class="text-sm text-slate-600 dark:text-slate-300 flex items-center gap-2">
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
        <label class="text-sm text-slate-600 dark:text-slate-300 flex items-center gap-1.5">
          <input type="checkbox" bind:checked={sortable} />
          정렬 허용
        </label>

        <div class="flex-1"></div>

        {#if result}
          <span class="text-sm text-slate-500 dark:text-slate-400">
            {result.row_count}행 · {result.elapsed_ms}ms
            {#if result.truncated}<span class="text-amber-600 dark:text-amber-400 ml-1">· 잘림</span>{/if}
          </span>
        {/if}
        {#if runError}
          <span class="text-sm text-rose-700 dark:text-rose-300 truncate max-w-[40%]" title={runError}>
            에러: {runError}
          </span>
        {/if}
      </div>

      <div class="flex flex-col flex-1 min-h-0">
        <div style="height: {editorHeightPx}px" class="min-h-[5rem] overflow-hidden">
          <QueryEditor
            value={sql}
            dark={themeStore.isDark}
            bind:selectedText={selectedSql}
            onChange={(v) => (sql = v)}
            onRun={onRunPressed}
          />
        </div>

        <!-- 드래그 핸들 -->
        <button
          type="button"
          aria-label="에디터/결과 높이 조절"
          class="h-1.5 cursor-row-resize bg-slate-200 dark:bg-slate-800 hover:bg-slate-400 dark:hover:bg-slate-600 transition-colors w-full
          {dragging ? 'bg-slate-400 dark:bg-slate-500' : ''}"
          onmousedown={onSplitMouseDown}
        ></button>

        <div class="flex-1 min-h-0 relative">
          <ResultGrid {result} {sortable} dark={themeStore.isDark} />
          <QueryRunningOverlay show={busy} onCancel={onCancelPressed} />
        </div>
      </div>
    </section>
  </main>
</div>

{#if pendingClassification}
  <ConfirmDialog
    title={confirmDialogTitle(pendingClassification)}
    message={confirmDialogMessage(pendingClassification)}
    detail={pendingClassification.keywords.length > 0
      ? `감지된 키워드: ${pendingClassification.keywords.join(", ")}`
      : null}
    confirmLabel="실행"
    danger={pendingClassification.kind === "ddl"}
    onConfirm={actuallyRun}
    onCancel={() => (pendingClassification = null)}
  />
{/if}
