<script lang="ts">
  import { tick } from "svelte";
  import { queryTabs, isDirty } from "$lib/stores/queryTabs.svelte";

  type Props = {
    /** 닫기 요청 — 부모가 dirty 확인 후 forceCloseTab 호출 */
    onRequestClose: (id: string) => void;
    onAddTab: () => void;
  };
  let { onRequestClose, onAddTab }: Props = $props();

  // 인라인 이름 편집 상태 (한 번에 한 탭만 편집 가능)
  let editingId = $state<string | null>(null);
  let editValue = $state("");
  let editInput = $state<HTMLInputElement | undefined>();

  function onTabClick(id: string, e: MouseEvent) {
    if (editingId === id) return; // 편집 중 클릭은 무시
    if (e.button === 1) {
      // 가운데 클릭 = 닫기
      e.preventDefault();
      onRequestClose(id);
      return;
    }
    queryTabs.switchTo(id);
  }

  async function startEdit(id: string, currentTitle: string) {
    queryTabs.switchTo(id);
    editingId = id;
    editValue = currentTitle;
    await tick();
    editInput?.focus();
    editInput?.select();
  }

  function commitEdit() {
    if (!editingId) return;
    const next = editValue.trim();
    if (next.length > 0) {
      queryTabs.updateTab(editingId, { title: next });
    }
    editingId = null;
    editValue = "";
  }

  function cancelEdit() {
    editingId = null;
    editValue = "";
  }

  function onEditKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commitEdit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancelEdit();
    }
  }
</script>

<div
  class="flex items-stretch border-b border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-900 overflow-x-auto"
  role="tablist"
>
  {#each queryTabs.tabs as t (t.id)}
    {@const active = queryTabs.activeId === t.id}
    {@const dirty = isDirty(t)}
    {@const editing = editingId === t.id}
    <div
      class="group flex items-center gap-1.5 px-3 py-1.5 text-sm border-r border-slate-200 dark:border-slate-800 cursor-pointer select-none whitespace-nowrap
      {active
        ? 'bg-white dark:bg-slate-800 text-slate-900 dark:text-slate-100 border-b-2 border-b-emerald-500 -mb-px'
        : 'text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/60'}"
      role="tab"
      tabindex="0"
      aria-selected={active}
      onmousedown={(e) => onTabClick(t.id, e)}
      ondblclick={() => startEdit(t.id, t.title)}
      onkeydown={(e) => {
        if (editing) return;
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          queryTabs.switchTo(t.id);
        } else if (e.key === "F2") {
          e.preventDefault();
          startEdit(t.id, t.title);
        }
      }}
      title={editing ? "이름 입력 후 Enter (Esc 취소)" : t.filePath ?? t.title}
    >
      <span class="w-2 text-rose-500" aria-hidden="true">
        {dirty ? "●" : ""}
      </span>
      {#if editing}
        <input
          bind:this={editInput}
          type="text"
          class="max-w-[14rem] bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 border border-emerald-500 rounded px-1 py-0 text-sm outline-none"
          bind:value={editValue}
          onkeydown={onEditKeyDown}
          onblur={commitEdit}
          onmousedown={(e) => e.stopPropagation()}
          ondblclick={(e) => e.stopPropagation()}
        />
      {:else}
        <span class="max-w-[14rem] truncate">{t.title}</span>
      {/if}
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
    aria-label="새 탭"
    title="새 탭 (⌘T)"
    onclick={onAddTab}
  >
    +
  </button>
  <div class="flex-1"></div>
</div>
