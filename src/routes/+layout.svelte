<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { themeStore } from "$lib/stores/theme.svelte";

  let { children } = $props();

  onMount(() => {
    themeStore.init();
  });
</script>

<div class="h-full w-full flex flex-col bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100">
  {@render children?.()}
</div>
