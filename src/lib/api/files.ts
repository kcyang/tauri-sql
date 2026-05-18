import { invoke } from "./invoke";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { QueryResult } from "./query";

const SQL_FILTERS = [
  { name: "SQL", extensions: ["sql"] },
  { name: "Text", extensions: ["txt"] },
  { name: "All", extensions: ["*"] },
];

const XLSX_FILTERS = [{ name: "Excel", extensions: ["xlsx"] }];

export const filesApi = {
  /** 열기 다이얼로그 → 텍스트 로드. 사용자가 취소하면 null 반환. */
  async openSqlFile(): Promise<{ path: string; content: string } | null> {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: SQL_FILTERS,
    });
    if (typeof selected !== "string") return null;
    const content = await invoke<string>("read_text_file", { path: selected });
    return { path: selected, content };
  },

  /** 저장 다이얼로그 → 텍스트 기록. 사용자가 취소하면 null 반환. */
  async saveSqlFile(content: string, defaultPath?: string): Promise<string | null> {
    const path = await save({
      filters: SQL_FILTERS,
      defaultPath,
    });
    if (!path) return null;
    await invoke<void>("write_text_file", { path, content });
    return path;
  },

  /** 현재 결과를 Excel(.xlsx) 로 저장. 사용자가 취소하면 null 반환. */
  async exportResultToXlsx(
    result: QueryResult,
    defaultName = "query-result.xlsx",
  ): Promise<string | null> {
    const path = await save({
      filters: XLSX_FILTERS,
      defaultPath: defaultName,
    });
    if (!path) return null;
    await invoke<void>("export_query_xlsx", { path, result });
    return path;
  },
};
