<script lang="ts">
  import { onDestroy, onMount, untrack } from "svelte";
  import {
    createGrid,
    type ColDef,
    type GridApi,
    type GridOptions,
  } from "ag-grid-community";
  import "ag-grid-community/styles/ag-grid.css";
  import "ag-grid-community/styles/ag-theme-quartz.css";
  import { formatCell, type QueryResult, type RowValue } from "$lib/api/query";

  type Props = {
    result: QueryResult | null;
    /** 정렬 허용 여부 (기본 false — 원본 쿼리 순서 유지) */
    sortable?: boolean;
    /** 다크 테마 사용 */
    dark?: boolean;
  };
  let { result, sortable = false, dark = false }: Props = $props();

  let host: HTMLDivElement | undefined = $state();
  let api: GridApi | null = null;

  const MAX_AUTO_WIDTH = 600;
  const MIN_AUTO_WIDTH = 60;

  onMount(() => {
    if (!host) return;
    const options: GridOptions = {
      columnDefs: [],
      rowData: [],
      defaultColDef: {
        sortable: false,
        filter: false,
        resizable: true,
        minWidth: MIN_AUTO_WIDTH,
        maxWidth: MAX_AUTO_WIDTH,
      },
      rowBuffer: 30,
      animateRows: false,
      // 행 선택: ⌘/⇧ 클릭으로 멀티
      rowSelection: "multiple",
      // 셀 안의 텍스트 마우스 드래그도 함께 허용 — 클릭 시점에 ag-grid 가 우선순위 결정
      enableCellTextSelection: true,
      ensureDomOrder: true,
    };
    api = createGrid(host, options);
    untrack(() => applyResult(result, sortable));

    // 그리드 영역에서 발생한 ⌘C / ⌃C → 자체 클립보드 복사
    // window 에서 잡지만, 포커스가 그리드 안에 있을 때만 동작 (에디터/입력 포커스에는 영향 없음)
    window.addEventListener("keydown", onWindowKey);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onWindowKey);
    api?.destroy();
    api = null;
  });

  $effect(() => {
    applyResult(result, sortable);
  });

  function applyResult(r: QueryResult | null, sortableNow: boolean) {
    if (!api) return;
    if (!r) {
      api.setGridOption("columnDefs", []);
      api.setGridOption("rowData", []);
      return;
    }
    const colDefs: ColDef[] = r.columns.map((c, i) => ({
      field: `c${i}`,
      headerName: c.name,
      headerTooltip: `${c.name} (${c.sql_type})`,
      tooltipValueGetter: (p) => String(p.value ?? ""),
      sortable: sortableNow,
      cellClass:
        c.sql_type === "int" ||
        c.sql_type === "float" ||
        c.sql_type === "decimal" ||
        c.sql_type === "money"
          ? "text-right font-mono"
          : "font-mono",
    }));
    const rowData = r.rows.map((row: RowValue[]) => {
      const obj: Record<string, string> = {};
      row.forEach((v, i) => {
        obj[`c${i}`] = formatCell(v);
      });
      return obj;
    });
    api.setGridOption("columnDefs", colDefs);
    api.setGridOption("rowData", rowData);
    requestAnimationFrame(() => {
      if (!api) return;
      const allColIds = (api.getColumns() ?? []).map((c) => c.getColId());
      api.autoSizeColumns(allColIds, false);
    });
  }

  function onWindowKey(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key.toLowerCase() !== "c") return;
    // 포커스가 그리드 호스트 내부에 있을 때만 자체 처리
    if (!host || !document.activeElement || !host.contains(document.activeElement)) return;
    // 사용자가 셀 안의 텍스트를 드래그 선택한 경우(브라우저 선택 영역이 비어있지 않음)는
    // 기본 브라우저 복사 동작에 양보한다.
    const sel = window.getSelection();
    if (sel && !sel.isCollapsed && sel.toString().length > 0) return;
    const copied = copySelection();
    if (copied) e.preventDefault();
  }

  /** 행 선택 있으면 그 행들을 TSV, 없으면 포커스된 셀 1칸을 클립보드에 쓴다. */
  function copySelection(): boolean {
    if (!api) return false;

    const selectedRows = api.getSelectedRows() as Record<string, string>[];
    if (selectedRows.length > 0) {
      const cols = api.getAllDisplayedColumns();
      const headers = cols.map(
        (c) => (c.getColDef().headerName as string | undefined) ?? c.getColId(),
      );
      const lines: string[] = [headers.join("\t")];
      for (const row of selectedRows) {
        lines.push(
          cols.map((c) => escapeTsv(row[c.getColId()] ?? "")).join("\t"),
        );
      }
      void navigator.clipboard.writeText(lines.join("\n"));
      return true;
    }

    const focused = api.getFocusedCell();
    if (focused) {
      const rowNode = api.getDisplayedRowAtIndex(focused.rowIndex);
      if (rowNode) {
        const val = api.getCellValue({ rowNode, colKey: focused.column });
        void navigator.clipboard.writeText(String(val ?? ""));
        return true;
      }
    }
    return false;
  }

  /** TSV 셀에서 탭/줄바꿈은 공백으로 치환 — 행 정렬 깨짐 방지. */
  function escapeTsv(v: string): string {
    return v.replace(/\t/g, " ").replace(/\r?\n/g, " ");
  }
</script>

<div
  bind:this={host}
  class="w-full h-full"
  class:ag-theme-quartz={!dark}
  class:ag-theme-quartz-dark={dark}
></div>

<style>
  :global(.ag-theme-quartz),
  :global(.ag-theme-quartz-dark) {
    --ag-grid-size: 6px;
    --ag-list-item-height: 28px;
    --ag-header-height: 32px;
    --ag-row-height: 28px;
    --ag-font-size: 12px;
  }
</style>
