<script lang="ts">
  import { themeStore, type ThemeChoice } from "$lib/stores/theme.svelte";

  const choices: { value: ThemeChoice; label: string; icon: string }[] = [
    { value: "system", label: "시스템", icon: "🖥" },
    { value: "light", label: "라이트", icon: "☀" },
    { value: "dark", label: "다크", icon: "🌙" },
  ];
</script>

<div class="inline-flex rounded-md border border-slate-300 dark:border-slate-700 overflow-hidden text-sm">
  {#each choices as c}
    <button
      class="px-2 py-1 transition-colors {themeStore.choice === c.value
        ? 'bg-slate-900 text-white dark:bg-slate-200 dark:text-slate-900'
        : 'bg-white text-slate-700 hover:bg-slate-100 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700'}"
      title={c.label}
      onclick={() => themeStore.set(c.value)}
    >
      {c.icon}
    </button>
  {/each}
</div>
