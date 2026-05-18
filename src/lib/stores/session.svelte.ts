// 활성 세션 1개 보관. Svelte 5 runes 기반.

import type { Profile } from "$lib/types";

interface SessionState {
  sessionId: string;
  profile: Profile;
}

let current = $state<SessionState | null>(null);

export const sessionStore = {
  get current() {
    return current;
  },
  set(state: SessionState) {
    current = state;
  },
  clear() {
    current = null;
  },
};
