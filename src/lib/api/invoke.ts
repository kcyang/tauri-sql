import { invoke as tauriInvoke } from "@tauri-apps/api/core";

/**
 * Tauri invoke 의 얇은 typed 래퍼.
 * 호출 측에서 명시적 제네릭 + 인자 객체를 강제하여 오타를 줄인다.
 */
export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return tauriInvoke<T>(cmd, args);
}
