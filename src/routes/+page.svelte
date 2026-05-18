<script lang="ts">
  import { goto } from "$app/navigation";
  import ConnectionList from "$lib/components/ConnectionList.svelte";
  import { sessionApi } from "$lib/api/session";
  import { sessionStore } from "$lib/stores/session.svelte";
  import { errorMessage, type Profile } from "$lib/types";

  let connecting = $state(false);
  let connectError = $state<string | null>(null);

  async function handleConnect(profile: Profile) {
    connectError = null;
    connecting = true;
    try {
      const sessionId = await sessionApi.open(profile.id);
      sessionStore.set({ sessionId, profile });
      await goto("/workspace");
    } catch (e) {
      connectError = errorMessage(e);
    } finally {
      connecting = false;
    }
  }
</script>

{#if connectError}
  <div class="bg-rose-50 text-rose-700 text-sm px-6 py-2 border-b border-rose-200">
    연결 실패: {connectError}
  </div>
{/if}
{#if connecting}
  <div class="bg-amber-50 text-amber-700 text-sm px-6 py-2 border-b border-amber-200">
    연결 중…
  </div>
{/if}

<ConnectionList onConnect={handleConnect} />
