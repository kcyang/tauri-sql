import { invoke } from "./invoke";

export type ObjectKind = "table" | "view" | "procedure";

export interface DbObject {
  schema: string;
  name: string;
}

export const explorerApi = {
  listDatabases: (session_id: string) =>
    invoke<string[]>("list_databases", { sessionId: session_id }),

  listObjects: (session_id: string, database: string, kind: ObjectKind) =>
    invoke<DbObject[]>("list_objects", {
      sessionId: session_id,
      database,
      kind,
    }),
};
