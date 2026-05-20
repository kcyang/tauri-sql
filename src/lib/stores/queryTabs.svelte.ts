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
