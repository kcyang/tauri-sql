import { invoke } from "./invoke";
import type { Profile, ProfileInput } from "$lib/types";

export const profilesApi = {
  list: () => invoke<Profile[]>("list_profiles"),

  save: (input: ProfileInput, password?: string) =>
    invoke<Profile>("save_profile", { input, password: password ?? null }),

  delete: (id: string) => invoke<void>("delete_profile", { id }),

  test: (input: ProfileInput, password?: string) =>
    invoke<void>("test_connection", { input, password: password ?? null }),
};
