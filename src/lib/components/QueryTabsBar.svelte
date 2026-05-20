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
