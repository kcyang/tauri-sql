import { invoke } from "./invoke";

/** Rust `RowValue` 의 직렬화 형태 — `{t, v}` 또는 null variant 의 `{t:"null"}` */
export type RowValue =
  | { t: "null" }
  | { t: "bool"; v: boolean }
  | { t: "int"; v: number }
  | { t: "float"; v: number }
  | { t: "decimal"; v: string }
  | { t: "text"; v: string }
  | { t: "date_time"; v: string }
  | { t: "uuid"; v: string }
  | { t: "binary"; v: string }
  | { t: "unknown"; v: string };

export interface ColumnMeta {
  name: string;
  sql_type: string;
}

export interface QueryResult {
  columns: ColumnMeta[];
  rows: RowValue[][];
  row_count: number;
  truncated: boolean;
  elapsed_ms: number;
}

export interface ExecuteQueryArgs {
  session_id: string;
  sql: string;
  query_id: string;
  max_rows?: number;
  timeout_ms?: number;
}

export type SqlKind = "read_only" | "write" | "ddl" | "empty" | "unknown";

export interface SqlClassification {
  kind: SqlKind;
  keywords: string[];
}

export const queryApi = {
  execute: (args: ExecuteQueryArgs) =>
    invoke<QueryResult>("execute_query", { args }),

  cancel: (query_id: string) =>
    invoke<boolean>("cancel_query", { queryId: query_id }),

  classify: (sql: string) =>
    invoke<SqlClassification>("classify_sql", { sql }),
};

/** 그리드/리스트 표시용 평탄화 문자열 변환. */
export function formatCell(v: RowValue): string {
  switch (v.t) {
    case "null":
      return "";
    case "bool":
      return v.v ? "true" : "false";
    case "int":
    case "float":
      return String(v.v);
    default:
      return v.v;
  }
}
