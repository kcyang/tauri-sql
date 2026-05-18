<script lang="ts">
  type Props = {
    title: string;
    message: string;
    detail?: string | null;
    confirmLabel?: string;
    danger?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  };
  let {
    title,
    message,
    detail = null,
    confirmLabel = "실행",
    danger = false,
    onConfirm,
    onCancel,
  }: Props = $props();
</script>

<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
  <div class="bg-white dark:bg-slate-800 rounded-lg shadow-lg p-6 max-w-md w-full mx-4">
    <h3 class="text-lg font-semibold text-slate-900 dark:text-slate-100">{title}</h3>
    <p class="text-slate-700 dark:text-slate-300 mt-2 whitespace-pre-wrap">{message}</p>
    {#if detail}
      <p class="text-sm text-slate-500 dark:text-slate-400 mt-2 font-mono">{detail}</p>
    {/if}
    <div class="flex justify-end gap-2 mt-4">
      <button
        class="px-4 py-2 rounded-md text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-700"
        onclick={onCancel}
      >
        취소
      </button>
      <button
        class={`px-4 py-2 rounded-md text-white ${danger ? "bg-rose-600 hover:bg-rose-500" : "bg-slate-900 hover:bg-slate-700 dark:bg-slate-200 dark:text-slate-900 dark:hover:bg-white"}`}
        onclick={onConfirm}
      >
        {confirmLabel}
      </button>
    </div>
  </div>
</div>
