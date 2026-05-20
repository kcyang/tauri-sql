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
