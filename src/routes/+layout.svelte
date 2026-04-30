<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import '../app.css';
  import { settingsStore } from '../store/settingsStore.svelte';
  import { exportStore } from '../store/exportStore.svelte';
  import { projectStore } from '../store/projectStore.svelte';
  import type {
    AppSettings,
    ExportProgressPayload,
    ExportDonePayload,
    ExportErrorPayload
  } from '../types/ipc';

  let { children } = $props();

  $effect(() => {
    const filePath = projectStore.filePath;
    const dirty = projectStore.dirty;
    const fileName = filePath ? filePath.split(/[\\/]/).pop() ?? filePath : '無題';
    document.title = `ezmm — ${fileName}${dirty ? '*' : ''}`;
  });

  let cleanupFns: Array<() => void> = [];

  onMount(async () => {
    try {
      const s = await invoke<AppSettings>('load_settings');
      settingsStore.setSettings(s);
    } catch (e) {
      console.error('設定の読み込みに失敗しました:', e);
    }

    cleanupFns = await Promise.all([
      listen<ExportProgressPayload>('export:progress', ({ payload }) =>
        exportStore.onProgress(payload.progress)
      ),
      listen<ExportDonePayload>('export:done', ({ payload }) =>
        exportStore.onDone(payload)
      ),
      listen<ExportErrorPayload>('export:error', ({ payload }) =>
        exportStore.onError(payload)
      ),
      listen<null>('export:cancelled', () => exportStore.onCancelled())
    ]);
  });

  onDestroy(() => cleanupFns.forEach((fn) => fn()));
</script>

{@render children()}
