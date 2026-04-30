<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { projectStore } from '../store/projectStore.svelte';
  import { exportStore } from '../store/exportStore.svelte';
  import { settingsStore } from '../store/settingsStore.svelte';
  import type { ValidationResult } from '../types/ipc';

  interface Props {
    onRequestNew: () => void;
    onRequestOpen: () => void;
  }
  let { onRequestNew, onRequestOpen }: Props = $props();

  let project = $derived(projectStore.project);
  let isBusy = $derived(exportStore.isRunning);

  async function handleSave() {
    if (!project) return;
    if (!projectStore.filePath) { await handleSaveAs(); return; }
    try {
      await invoke('save_project', { path: projectStore.filePath, project });
      projectStore.clearDirty();
    } catch (e) {
      alert(`保存に失敗しました: ${e}`);
    }
  }

  async function handleSaveAs() {
    if (!project) return;
    const defaultName = projectStore.filePath?.split(/[\\/]/).pop() ?? 'project.yaml';
    const path = await invoke<string | null>('show_save_yaml_dialog', { defaultName });
    if (!path) return;
    try {
      await invoke('save_project', { path, project });
      projectStore.setFilePath(path);
      projectStore.clearDirty();
      await addRecentFile(path);
    } catch (e) {
      alert(`保存に失敗しました: ${e}`);
    }
  }

  async function addRecentFile(path: string) {
    const s = settingsStore.settings;
    if (!s) return;
    const updated = { ...s, recent_files: [path, ...s.recent_files.filter((f) => f !== path)].slice(0, 10) };
    settingsStore.setSettings(updated);
    try { await invoke('save_settings', { settings: updated }); } catch { /* best effort */ }
  }

  async function handleExport() {
    if (!project) return;
    const result = await invoke<ValidationResult>('validate_project', { project });
    if (result.errors.length > 0) {
      alert(`バリデーションエラー:\n${result.errors.map((e) => e.message).join('\n')}`);
      return;
    }
    try {
      await exportStore.startExport(project);
    } catch (e) {
      alert(`書き出しに失敗しました: ${e}`);
    }
  }

  let outputFolderInvalid = $derived((project?.output_folder ?? '').trim() === '');
  let outputNameInvalid = $derived((project?.output.output_name ?? '').trim() === '');
  let exportDisabled = $derived(!project || outputFolderInvalid || outputNameInvalid || isBusy);
  let exportTitle = $derived(
    !project ? '' :
    outputFolderInvalid || outputNameInvalid
      ? 'プロパティパネルの「出力設定」タブで出力先とファイル名を設定してください'
      : ''
  );
</script>

<header class="toolbar">
  <div class="toolbar-left">
    <button onclick={onRequestNew} disabled={isBusy}>新規</button>
    <button onclick={onRequestOpen} disabled={isBusy}>開く</button>
    <button id="save-btn" onclick={handleSave} disabled={!project || isBusy}>保存</button>
    <button id="save-as-btn" onclick={handleSaveAs} disabled={!project || isBusy}>名前を付けて保存</button>
  </div>

  <div class="toolbar-right">
    <button
      class="btn-primary"
      onclick={handleExport}
      disabled={exportDisabled}
      title={exportTitle}
    >
      {isBusy ? '書き出し中...' : '書き出し'}
    </button>
  </div>
</header>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: var(--toolbar-height);
    padding: 0 8px;
    background: var(--bg-toolbar);
    border-bottom: 1px solid var(--color-border);
    gap: 6px;
    flex-shrink: 0;
  }
  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
</style>
