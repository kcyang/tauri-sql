import { invoke } from "./invoke";
import type { Profile } from "$lib/types";

export const sessionApi = {
  open: (profile_id: string) =>
    invoke<string>("open_session", { profileId: profile_id }),

  close: (session_id: string) =>
    invoke<void>("close_session", { sessionId: session_id }),

  profile: (session_id: string) =>
    invoke<Profile>("session_profile", { sessionId: session_id }),
};
