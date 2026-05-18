<script lang="ts">
  import { profilesApi } from "$lib/api/profiles";
  import { errorMessage, type Profile } from "$lib/types";
  import ConnectionForm from "./ConnectionForm.svelte";
  import ThemeToggle from "./ThemeToggle.svelte";

  type Props = {
    onConnect: (profile: Profile) => void;
  };
  let { onConnect }: Props = $props();

  let profiles = $state<Profile[]>([]);
  let loading = $state(true);
  let listError = $state<string | null>(null);

  let mode = $state<"list" | "form">("list");
  let editing = $state<Profile | null>(null);
  let deleteTarget = $state<Profile | null>(null);

  $effect(() => {
    refresh();
  });

  async function refresh() {
    loading = true;
    listError = null;
    try {
      profiles = await profilesApi.list();
    } catch (e) {
      listError = errorMessage(e);
    } finally {
      loading = false;
    }
  }

  function openNew() {
    editing = null;
    mode = "form";
  }
  function openEdit(p: Profile) {
    editing = p;
    mode = "form";
  }
  async function confirmDelete() {
    if (!deleteTarget) return;
    const target = deleteTarget;
    deleteTarget = null;
    try {
      await profilesApi.delete(target.id);
      await refresh();
    } catch (e) {
      listError = errorMessage(e);
    }
  }
</script>

<div class="flex-1 flex flex-col">
  <header class="px-6 py-4 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900">
    <div class="max-w-3xl mx-auto flex items-center justify-between">
      <h1 class="text-lg font-semibold text-slate-900 dark:text-slate-100">
        Tauri SQL — 연결 관리
      </h1>
      <div class="flex items-center gap-2">
        <ThemeToggle />
        {#if mode === "list"}
          <button
            class="px-4 py-2 rounded-md bg-slate-900 text-white hover:bg-slate-700 dark:bg-slate-200 dark:text-slate-900 dark:hover:bg-white text-sm"
            onclick={openNew}
          >
            + 새 연결
          </button>
        {/if}
      </div>
    </div>
  </header>

  <main class="flex-1 overflow-auto py-8 px-6">
    {#if mode === "form"}
      <ConnectionForm
        profile={editing}
        onSaved={async () => {
          mode = "list";
          editing = null;
          await refresh();
        }}
        onCancel={() => {
          mode = "list";
          editing = null;
        }}
      />
    {:else}
      <div class="max-w-3xl mx-auto">
        {#if loading}
          <p class="text-slate-500 dark:text-slate-400">로딩 중…</p>
        {:else if listError}
          <p class="text-rose-700 bg-rose-50 dark:text-rose-200 dark:bg-rose-900 rounded-md px-3 py-2">
            목록 로드 실패: {listError}
          </p>
        {:else if profiles.length === 0}
          <div class="text-center py-12 text-slate-500 dark:text-slate-400">
            <p>저장된 연결이 없습니다.</p>
            <p class="mt-2 text-sm">우측 상단의 “+ 새 연결” 버튼으로 추가하세요.</p>
          </div>
        {:else}
          <ul class="space-y-2">
            {#each profiles as p (p.id)}
              <li class="bg-white dark:bg-slate-800 rounded-lg shadow-sm border border-slate-200 dark:border-slate-700 px-4 py-3 flex items-center gap-3">
                <div class="flex-1 min-w-0">
                  <div class="font-medium truncate text-slate-900 dark:text-slate-100">{p.name}</div>
                  <div class="text-sm text-slate-500 dark:text-slate-400 truncate">
                    {p.username}@{p.host}:{p.port} / {p.database}
                  </div>
                </div>
                <button
                  class="px-3 py-1.5 rounded-md text-sm bg-slate-900 text-white hover:bg-slate-700 dark:bg-slate-200 dark:text-slate-900 dark:hover:bg-white"
                  onclick={() => onConnect(p)}
                >
                  연결
                </button>
                <button
                  class="px-3 py-1.5 rounded-md text-sm border border-slate-300 dark:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-200"
                  onclick={() => openEdit(p)}
                >
                  편집
                </button>
                <button
                  class="px-3 py-1.5 rounded-md text-sm text-rose-700 dark:text-rose-300 hover:bg-rose-50 dark:hover:bg-rose-900/40"
                  onclick={() => (deleteTarget = p)}
                >
                  삭제
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </main>
</div>

{#if deleteTarget}
  <div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 max-w-sm w-full mx-4">
      <h3 class="text-lg font-semibold text-slate-900 dark:text-slate-100">프로필 삭제</h3>
      <p class="text-slate-600 dark:text-slate-300 mt-2">
        “{deleteTarget.name}” 을 삭제할까요? 저장된 비밀번호도 함께 제거됩니다.
      </p>
      <div class="flex justify-end gap-2 mt-4">
        <button
          class="px-4 py-2 rounded-md text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-700"
          onclick={() => (deleteTarget = null)}
        >
          취소
        </button>
        <button
          class="px-4 py-2 rounded-md bg-rose-600 text-white hover:bg-rose-500"
          onclick={confirmDelete}
        >
          삭제
        </button>
      </div>
    </div>
  </div>
{/if}
