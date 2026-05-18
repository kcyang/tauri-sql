<script lang="ts">
  import { onDestroy, onMount } from "svelte";

  type Props = {
    /** 실행 시작 이후 경과 표시용. 부모가 직접 줘도 되지만, 안전하게 자체 타이머 사용 */
    show: boolean;
    onCancel?: () => void;
  };
  let { show, onCancel }: Props = $props();

  let elapsed = $state(0);
  let timer: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    elapsed = 0;
    timer = setInterval(() => {
      elapsed += 1;
    }, 1000);
  });

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });

  function fmt(s: number): string {
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    const rest = s % 60;
    return `${m}m ${rest}s`;
  }
</script>

{#if show}
  <div
    class="absolute inset-0 flex flex-col items-center justify-center gap-3
           bg-white/70 dark:bg-slate-950/70 backdrop-blur-[1px] z-10 select-none"
    aria-live="polite"
    aria-busy="true"
  >
    <!-- 회전 spinner. currentColor 사용 → 다크모드 자동 대응 -->
    <div class="text-slate-700 dark:text-slate-200">
      <svg class="animate-spin" width="28" height="28" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-opacity="0.18" stroke-width="3" />
        <path
          d="M21 12a9 9 0 0 0-9-9"
          stroke="currentColor"
          stroke-width="3"
          stroke-linecap="round"
        />
      </svg>
    </div>

    <div class="flex items-baseline gap-2">
      <span class="text-sm font-medium text-slate-700 dark:text-slate-200">
        쿼리 실행 중<span class="anim-dots"></span>
      </span>
      <span class="text-xs tabular-nums text-slate-500 dark:text-slate-400">
        {fmt(elapsed)}
      </span>
    </div>

    {#if onCancel}
      <button
        class="mt-1 px-3 py-1 text-xs rounded-md border border-slate-300 dark:border-slate-600
               text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800"
        onclick={onCancel}
      >
        취소 (⌘.)
      </button>
    {/if}
  </div>
{/if}

<style>
  /* 텍스트 옆 깜빡이는 점 3개 */
  .anim-dots::after {
    display: inline-block;
    width: 1.2em;
    text-align: left;
    content: "";
    animation: dots 1.4s steps(4, end) infinite;
  }
  @keyframes dots {
    0%   { content: ""; }
    25%  { content: "."; }
    50%  { content: ".."; }
    75%  { content: "..."; }
    100% { content: ""; }
  }
</style>
