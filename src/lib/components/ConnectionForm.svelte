<script lang="ts">
  import { untrack } from "svelte";
  import { profilesApi } from "$lib/api/profiles";
  import { errorMessage, type Profile, type ProfileInput } from "$lib/types";

  type Props = {
    profile?: Profile | null;
    onSaved: (profile: Profile) => void;
    onCancel: () => void;
  };
  let { profile = null, onSaved, onCancel }: Props = $props();

  // 폼은 mount 시 1회만 초기값을 캡처한다 (부모가 편집 시마다 remount 함).
  let name = $state(untrack(() => profile?.name ?? ""));
  let host = $state(untrack(() => profile?.host ?? "localhost"));
  let port = $state<number>(untrack(() => profile?.port ?? 1433));
  let database = $state(untrack(() => profile?.database ?? "master"));
  let username = $state(untrack(() => profile?.username ?? ""));
  let password = $state("");
  let trustCert = $state(untrack(() => profile?.trust_server_certificate ?? true));
  let appName = $state(untrack(() => profile?.application_name ?? "just-sql"));

  let busy = $state(false);
  let testMessage = $state<{ ok: boolean; text: string } | null>(null);
  let saveError = $state<string | null>(null);

  const isEditing = $derived(profile !== null);
  const passwordRequired = $derived(!isEditing);

  function buildInput(): ProfileInput {
    return {
      id: profile?.id,
      name: name.trim(),
      host: host.trim(),
      port: Number(port),
      database: database.trim(),
      username: username.trim(),
      auth_method: "sql_server",
      trust_server_certificate: trustCert,
      application_name: appName.trim() || null,
    };
  }

  function validate(): string | null {
    if (!name.trim()) return "프로필 이름을 입력하세요.";
    if (!host.trim()) return "호스트를 입력하세요.";
    if (!Number.isFinite(port) || port < 1 || port > 65535) return "포트는 1~65535 범위여야 합니다.";
    if (!database.trim()) return "데이터베이스를 입력하세요.";
    if (!username.trim()) return "사용자 이름을 입력하세요.";
    if (passwordRequired && !password) return "비밀번호를 입력하세요.";
    return null;
  }

  async function handleTest() {
    saveError = null;
    testMessage = null;
    const err = validate();
    if (err) {
      testMessage = { ok: false, text: err };
      return;
    }
    busy = true;
    try {
      await profilesApi.test(buildInput(), password || undefined);
      testMessage = { ok: true, text: "연결 성공" };
    } catch (e) {
      testMessage = { ok: false, text: `연결 실패: ${errorMessage(e)}` };
    } finally {
      busy = false;
    }
  }

  async function handleSave() {
    saveError = null;
    testMessage = null;
    const err = validate();
    if (err) {
      saveError = err;
      return;
    }
    busy = true;
    try {
      const saved = await profilesApi.save(buildInput(), password ? password : undefined);
      onSaved(saved);
    } catch (e) {
      saveError = errorMessage(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="max-w-md mx-auto bg-white dark:bg-slate-800 rounded-xl shadow p-6 space-y-4">
  <h2 class="text-xl font-semibold text-slate-900 dark:text-slate-100">
    {isEditing ? "연결 편집" : "새 연결"}
  </h2>

  <div class="space-y-3">
    <label class="block">
      <span class="text-sm text-slate-700 dark:text-slate-300">프로필 이름</span>
      <input
        class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 px-3 py-2"
        type="text"
        bind:value={name}
        placeholder="예: 로컬 개발"
      />
    </label>

    <div class="grid grid-cols-3 gap-2">
      <label class="col-span-2 block">
        <span class="text-sm text-slate-700 dark:text-slate-300">호스트</span>
        <input
          class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 px-3 py-2"
          type="text"
          bind:value={host}
        />
      </label>
      <label class="block">
        <span class="text-sm text-slate-700 dark:text-slate-300">포트</span>
        <input
          class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 px-3 py-2"
          type="number"
          bind:value={port}
          min="1"
          max="65535"
        />
      </label>
    </div>

    <label class="block">
      <span class="text-sm text-slate-700 dark:text-slate-300">데이터베이스</span>
      <input
        class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 px-3 py-2"
        type="text"
        bind:value={database}
      />
    </label>

    <label class="block">
      <span class="text-sm text-slate-700 dark:text-slate-300">사용자 이름</span>
      <input
        class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 px-3 py-2"
        type="text"
        bind:value={username}
        autocomplete="off"
      />
    </label>

    <label class="block">
      <span class="text-sm text-slate-700 dark:text-slate-300">
        비밀번호
        {#if isEditing}
          <span class="text-slate-400 dark:text-slate-500">(비워두면 기존 값 유지)</span>
        {/if}
      </span>
      <input
        class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 px-3 py-2"
        type="password"
        bind:value={password}
        autocomplete="new-password"
      />
    </label>

    <label class="block">
      <span class="text-sm text-slate-700 dark:text-slate-300">애플리케이션 이름</span>
      <input
        class="mt-1 w-full rounded-md border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 px-3 py-2"
        type="text"
        bind:value={appName}
      />
    </label>

    <label class="flex items-center gap-2">
      <input type="checkbox" bind:checked={trustCert} />
      <span class="text-sm text-slate-700 dark:text-slate-300">서버 인증서 신뢰 (자체 서명 환경)</span>
    </label>
  </div>

  {#if testMessage}
    <div
      class="text-sm rounded-md px-3 py-2"
      class:bg-emerald-50={testMessage.ok}
      class:dark:bg-emerald-900={testMessage.ok}
      class:text-emerald-700={testMessage.ok}
      class:dark:text-emerald-200={testMessage.ok}
      class:bg-rose-50={!testMessage.ok}
      class:dark:bg-rose-900={!testMessage.ok}
      class:text-rose-700={!testMessage.ok}
      class:dark:text-rose-200={!testMessage.ok}
    >
      {testMessage.text}
    </div>
  {/if}

  {#if saveError}
    <div class="text-sm rounded-md px-3 py-2 bg-rose-50 text-rose-700 dark:bg-rose-900 dark:text-rose-200">
      저장 실패: {saveError}
    </div>
  {/if}

  <div class="flex items-center justify-between pt-2">
    <button
      class="px-4 py-2 rounded-md border border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700 disabled:opacity-50"
      onclick={handleTest}
      disabled={busy}
    >
      연결 테스트
    </button>
    <div class="flex gap-2">
      <button
        class="px-4 py-2 rounded-md text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-700"
        onclick={onCancel}
        disabled={busy}
      >
        취소
      </button>
      <button
        class="px-4 py-2 rounded-md bg-slate-900 text-white hover:bg-slate-700 dark:bg-slate-200 dark:text-slate-900 dark:hover:bg-white disabled:opacity-50"
        onclick={handleSave}
        disabled={busy}
      >
        저장
      </button>
    </div>
  </div>
</div>
