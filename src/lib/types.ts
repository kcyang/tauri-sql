// Rust 백엔드와 공유하는 타입.

export type AuthMethod = "sql_server";

export interface Profile {
  id: string; // UUID
  name: string;
  host: string;
  port: number;
  database: string;
  username: string;
  auth_method: AuthMethod;
  trust_server_certificate: boolean;
  application_name: string | null;
}

export interface ProfileInput {
  id?: string;
  name: string;
  host: string;
  port: number;
  database: string;
  username: string;
  auth_method: AuthMethod;
  trust_server_certificate: boolean;
  application_name: string | null;
}

export type AppErrorKind =
  | "config_invalid"
  | "connect"
  | "auth"
  | "timeout"
  | "cancelled"
  | "sql"
  | "keyring"
  | "io"
  | "session_not_found"
  | "internal";

export interface AppError {
  kind: AppErrorKind;
  message: string;
}

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    "message" in e &&
    typeof (e as AppError).message === "string"
  );
}

export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}
