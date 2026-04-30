<script lang="ts">
  import { exportStore } from '../store/exportStore.svelte';

  let confirmCancel = $state(false);

  function handleCancel() {
    if (exportStore.status === 'running') {
      confirmCancel = true;
    }
  }

  async function confirmCancelExport() {
    confirmCancel = false;
    await exportStore.cancelExport();
  }
</script>

<div class="modal-overlay" role="dialog" aria-modal="true">
  <div class="modal-box export-dialog">
    {#if exportStore.status === 'done'}
      <h2>書き出し完了</h2>
      <p class="done-path">{exportStore.outputPath}</p>
      {#if exportStore.elapsedMs !== null}
        <p class="elapsed">{(exportStore.elapsedMs / 1000).toFixed(1)}秒</p>
      {/if}
      <div class="actions">
        <button class="btn-primary" onclick={() => exportStore.reset()}>閉じる</button>
      </div>

    {:else if exportStore.status === 'error'}
      <h2>書き出しエラー</h2>
      <p class="error-message">{exportStore.error?.message}</p>
      {#if exportStore.error?.ffmpeg_stderr}
        <details>
          <summary>FFmpeg 出力</summary>
          <pre class="stderr">{exportStore.error.ffmpeg_stderr}</pre>
        </details>
      {/if}
      <div class="actions">
        <button class="btn-primary" onclick={() => exportStore.reset()}>閉じる</button>
      </div>

    {:else}
      <h2>書き出し中...</h2>
      <div class="progress-bar-wrap">
        <div
          class="progress-bar"
          style="width: {exportStore.progress !== null ? Math.round(exportStore.progress * 100) : 0}%"
        ></div>
      </div>
      <p class="progress-label">
        {exportStore.progress !== null ? `${Math.round(exportStore.progress * 100)}%` : '処理中...'}
      </p>

      {#if confirmCancel}
        <p>書き出しをキャンセルしますか？</p>
        <div class="actions">
          <button onclick={confirmCancelExport}>キャンセルする</button>
          <button onclick={() => (confirmCancel = false)}>続ける</button>
        </div>
      {:else}
        <div class="actions">
          <button
            onclick={handleCancel}
            disabled={exportStore.status === 'cancelling'}
          >
            {exportStore.status === 'cancelling' ? 'キャンセル中...' : 'キャンセル'}
          </button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .export-dialog { min-width: 360px; max-width: 560px; }
  h2 { margin: 0 0 1rem; font-size: 1rem; }
  .progress-bar-wrap { height: 12px; background: var(--bg-main); border-radius: 6px; overflow: hidden; margin-bottom: .5rem; }
  .progress-bar { height: 100%; background: var(--color-primary); transition: width .3s; }
  .progress-label { font-size: .85rem; text-align: center; margin: 0 0 1rem; }
  .done-path { font-size: .85rem; word-break: break-all; margin: 0 0 .5rem; }
  .elapsed { font-size: .8rem; color: var(--color-text-muted, #888); margin: 0 0 1rem; }
  .error-message { color: var(--color-error); font-size: .9rem; margin: 0 0 .75rem; }
  .stderr { font-size: .75rem; max-height: 120px; overflow-y: auto; background: var(--bg-main); padding: .5rem; border-radius: 4px; }
  .actions { display: flex; gap: .5rem; justify-content: flex-end; margin-top: .75rem; }
</style>
