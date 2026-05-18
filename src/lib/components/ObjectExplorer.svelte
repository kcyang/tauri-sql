<script lang="ts">
  import { onMount } from "svelte";
  import { explorerApi, type DbObject, type ObjectKind } from "$lib/api/explorer";
  import { errorMessage } from "$lib/types";

  type Props = {
    sessionId: string;
    onObjectActivate?: (database: string, kind: ObjectKind, obj: DbObject) => void;
  };
  let { sessionId, onObjectActivate }: Props = $props();

  let databases = $state<string[]>([]);
  let dbLoading = $state(true);
  let dbError = $state<string | null>(null);
  let expandedDbs = $state<Set<string>>(new Set());

  let groupCache = $state<Record<string, DbObject[]>>({});
  let groupLoading = $state<Record<string, boolean>>({});
  let groupError = $state<Record<string, string>>({});
  let expandedGroups = $state<Set<string>>(new Set());

  onMount(() => {
    loadDatabases();
  });

  async function loadDatabases() {
    dbLoading = true;
    dbError = null;
    try {
      databases = await explorerApi.listDatabases(sessionId);
    } catch (e) {
      dbError = errorMessage(e);
    } finally {
      dbLoading = false;
    }
  }

  function toggleDb(db: string) {
    if (expandedDbs.has(db)) expandedDbs.delete(db);
    else expandedDbs.add(db);
    expandedDbs = new Set(expandedDbs);
  }

  function groupKey(db: string, kind: ObjectKind): string {
    return `${db}::${kind}`;
  }

  async function toggleGroup(db: string, kind: ObjectKind) {
    const key = groupKey(db, kind);
    if (expandedGroups.has(key)) {
      expandedGroups.delete(key);
      expandedGroups = new Set(expandedGroups);
      return;
    }
    expandedGroups.add(key);
    expandedGroups = new Set(expandedGroups);
    if (!groupCache[key]) {
      groupLoading[key] = true;
      groupError[key] = "";
      try {
        groupCache[key] = await explorerApi.listObjects(sessionId, db, kind);
      } catch (e) {
        groupError[key] = errorMessage(e);
      } finally {
        groupLoading[key] = false;
      }
    }
  }

  const GROUPS: { kind: ObjectKind; label: string }[] = [
    { kind: "table", label: "테이블" },
    { kind: "view", label: "뷰" },
    { kind: "procedure", label: "프로시저" },
  ];
</script>

<div class="h-full flex flex-col bg-white dark:bg-slate-900 border-r border-slate-200 dark:border-slate-800 text-sm">
  <div class="px-3 py-2 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
    <span class="font-medium text-slate-900 dark:text-slate-100">오브젝트</span>
    <button
      class="text-xs text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100"
      onclick={loadDatabases}
      title="새로고침"
    >
      ↻
    </button>
  </div>

  <div class="flex-1 overflow-auto py-1">
    {#if dbLoading}
      <p class="text-slate-500 dark:text-slate-400 px-3 py-2">로딩 중…</p>
    {:else if dbError}
      <p class="text-rose-700 dark:text-rose-300 px-3 py-2">에러: {dbError}</p>
    {:else if databases.length === 0}
      <p class="text-slate-500 dark:text-slate-400 px-3 py-2">표시할 데이터베이스가 없습니다.</p>
    {:else}
      <ul>
        {#each databases as db (db)}
          {@const dbOpen = expandedDbs.has(db)}
          <li>
            <button
              class="w-full text-left px-3 py-1 hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-800 dark:text-slate-200 flex items-center gap-1"
              onclick={() => toggleDb(db)}
            >
              <span class="text-slate-400 dark:text-slate-500 w-3">{dbOpen ? "▾" : "▸"}</span>
              <span class="font-mono">{db}</span>
            </button>
            {#if dbOpen}
              <ul class="pl-4">
                {#each GROUPS as g (g.kind)}
                  {@const key = groupKey(db, g.kind)}
                  {@const open = expandedGroups.has(key)}
                  <li>
                    <button
                      class="w-full text-left px-3 py-1 hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-300 flex items-center gap-1"
                      onclick={() => toggleGroup(db, g.kind)}
                    >
                      <span class="text-slate-400 dark:text-slate-500 w-3">{open ? "▾" : "▸"}</span>
                      <span>{g.label}</span>
                      {#if groupCache[key]}
                        <span class="text-xs text-slate-400 dark:text-slate-500 ml-1">
                          ({groupCache[key].length})
                        </span>
                      {/if}
                    </button>
                    {#if open}
                      {#if groupLoading[key]}
                        <p class="pl-8 py-1 text-slate-500 dark:text-slate-400 text-xs">로딩…</p>
                      {:else if groupError[key]}
                        <p class="pl-8 py-1 text-rose-700 dark:text-rose-300 text-xs">{groupError[key]}</p>
                      {:else if groupCache[key]}
                        <ul class="pl-5">
                          {#each groupCache[key] as obj}
                            <li>
                              <button
                                class="w-full text-left px-3 py-0.5 hover:bg-slate-100 dark:hover:bg-slate-800 text-xs font-mono truncate text-slate-700 dark:text-slate-300"
                                ondblclick={() => onObjectActivate?.(db, g.kind, obj)}
                                title="더블클릭 → 쿼리 삽입"
                              >
                                {obj.schema}.{obj.name}
                              </button>
                            </li>
                          {/each}
                        </ul>
                      {/if}
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
