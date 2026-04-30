<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { projectStore } from '../store/projectStore.svelte';
  import { previewStore } from '../store/previewStore.svelte';
  import { exportStore } from '../store/exportStore.svelte';
  import { settingsStore } from '../store/settingsStore.svelte';
  import { calculateTotalDuration } from '../preview/sceneUtils';
  import Toolbar from '../components/Toolbar.svelte';
  import PreviewPanel from '../components/PreviewPanel.svelte';
  import PropertiesPanel from '../components/PropertiesPanel.svelte';
  import Timeline from '../components/Timeline.svelte';
  import ExportProgressDialog from '../components/ExportProgressDialog.svelte';
  import RecentFilesModal from '../components/RecentFilesModal.svelte';
  import AboutDialog from '../components/AboutDialog.svelte';
  import UnsavedChangesDialog from '../components/UnsavedChangesDialog.svelte';
  import type { Project } from '../types/project';

  let showRecentFiles = $state(false);
  let showAbout = $state(false);
  let showUnsavedDialog = $state(false);
  let pendingAction = $state<(() => void) | null>(null);

  let showExportDialog = $derived(
    exportStore.status === 'running' ||
    exportStore.status === 'cancelling' ||
    exportStore.status === 'done' ||
    exportStore.status === 'error'
  );

  onMount(async () => {
    await new Promise((r) => setTimeout(r, 50));
    newProject(); // always start with an empty project
    if ((settingsStore.settings?.recent_files?.length ?? 0) > 0) {
      showRecentFiles = true;
    }
  });

  function withUnsavedCheck(action: () => void) {
    if (projectStore.dirty) {
      pendingAction = action;
      showUnsavedDialog = true;
    } else {
      action();
    }
  }

  function newProject() {
    const empty: Project = {
      version: 1,
      output_folder: '',
      output: {
        output_name: '',
        width: 1920, height: 1080, fps: 30,
        codec: 'h264', format: 'mp4', crf: 23, preset: 'medium'
      },
      scenes: []
    };
    projectStore.loadProject(empty, null);
    previewStore.seek(0);
    previewStore.setTotalDuration(0);
  }

  async function openProject(path?: string) {
    let filePath = path;
    if (!filePath) {
      filePath = await invoke<string | null>('show_open_yaml_dialog') ?? undefined;
    }
    if (!filePath) return;
    try {
      const proj = await invoke<Project>('open_project', { path: filePath });
      projectStore.loadProject(proj, filePath);
      previewStore.seek(0);
      previewStore.setTotalDuration(calculateTotalDuration(proj.scenes));
      await addRecentFile(filePath);
    } catch (e) {
      alert(`ファイルを開けませんでした: ${e}`);
    }
  }

  async function addRecentFile(path: string) {
    const s = settingsStore.settings;
    if (!s) return;
    const updated = { ...s, recent_files: [path, ...s.recent_files.filter((f) => f !== path)].slice(0, 10) };
    settingsStore.setSettings(updated);
    try { await invoke('save_settings', { settings: updated }); } catch { /* best effort */ }
  }

  function handleRequestNew() { withUnsavedCheck(newProject); }
  function handleRequestOpen() { withUnsavedCheck(() => openProject()); }

  async function handleSaveAndContinue() {
    showUnsavedDialog = false;
    if (!projectStore.project) return;
    if (!projectStore.filePath) {
      const path = await invoke<string | null>('show_save_yaml_dialog', { defaultName: 'project.yaml' });
      if (!path) return;
      try {
        await invoke('save_project', { path, project: projectStore.project });
        projectStore.setFilePath(path);
        projectStore.clearDirty();
      } catch (e) { alert(`保存に失敗しました: ${e}`); return; }
    } else {
      try {
        await invoke('save_project', { path: projectStore.filePath, project: projectStore.project });
        projectStore.clearDirty();
      } catch (e) { alert(`保存に失敗しました: ${e}`); return; }
    }
    pendingAction?.();
    pendingAction = null;
  }

  function handleDiscardAndContinue() {
    showUnsavedDialog = false;
    projectStore.clearDirty();
    pendingAction?.();
    pendingAction = null;
  }

  function handleCancelUnsaved() {
    showUnsavedDialog = false;
    pendingAction = null;
  }

  const isTextInput = (el: EventTarget | null) => {
    if (!el) return false;
    const tag = (el as HTMLElement).tagName;
    return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
  };

  function handleKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;

    if (mod && e.key === 'n') { e.preventDefault(); handleRequestNew(); return; }
    if (mod && e.key === 'o') { e.preventDefault(); handleRequestOpen(); return; }
    if (mod && !e.shiftKey && e.key === 's') { e.preventDefault(); document.getElementById('save-btn')?.click(); return; }
    if (mod && e.shiftKey && e.key === 's') { e.preventDefault(); document.getElementById('save-as-btn')?.click(); return; }

    if (isTextInput(document.activeElement)) return;

    if (e.key === ' ') {
      e.preventDefault();
      previewStore.isPlaying ? previewStore.pause() : previewStore.play();
      return;
    }
    if (e.key === 'Escape') {
      if (previewStore.isPlaying) {
        previewStore.pause();
        previewStore.seek(0);
      } else {
        projectStore.selectObject(null);
        projectStore.selectScene(null);
      }
      return;
    }
    if (e.key === 'Delete' || e.key === 'Backspace') {
      if (projectStore.selectedObjectId && projectStore.selectedSceneId) {
        const sid = projectStore.selectedSceneId;
        const oid = projectStore.selectedObjectId;
        projectStore.updateProject((p) => {
          const scene = p.scenes.find((s) => s.id === sid);
          if (scene) scene.objects = scene.objects.filter((o) => o.id !== oid);
        });
        projectStore.selectObject(null);
      }
      return;
    }
    if (mod && e.key === 'd') {
      e.preventDefault();
      if (projectStore.selectedObjectId && projectStore.selectedSceneId) {
        const sid = projectStore.selectedSceneId;
        const oid = projectStore.selectedObjectId;
        projectStore.updateProject((p) => {
          const scene = p.scenes.find((s) => s.id === sid);
          if (!scene) return;
          const idx = scene.objects.findIndex((o) => o.id === oid);
          if (idx < 0) return;
          const copy = { ...scene.objects[idx], id: `${oid}_copy` };
          scene.objects.splice(idx + 1, 0, copy);
        });
      }
      return;
    }
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      const dir = e.key === 'ArrowLeft' ? -1 : 1;
      const fps = projectStore.project?.output.fps ?? 30;
      const delta = previewStore.isPlaying ? 5 : 1 / fps;
      const newTime = Math.max(0, Math.min(previewStore.totalDuration, previewStore.currentTime + dir * delta));
      previewStore.seek(newTime);
      return;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app-layout">
  <Toolbar onRequestNew={handleRequestNew} onRequestOpen={handleRequestOpen} />

  <div class="editor-area">
    <div class="preview-wrap">
      <PreviewPanel />
    </div>
    <div class="props-wrap">
      <PropertiesPanel />
    </div>
  </div>

  <div class="timeline-wrap">
    <Timeline />
  </div>
</div>

{#if showExportDialog}
  <ExportProgressDialog />
{/if}

{#if showRecentFiles}
  <RecentFilesModal
    onOpen={(path) => { showRecentFiles = false; openProject(path); }}
    onClose={() => { showRecentFiles = false; newProject(); }}
  />
{/if}

{#if showAbout}
  <AboutDialog onClose={() => (showAbout = false)} />
{/if}

{#if showUnsavedDialog}
  <UnsavedChangesDialog
    onSave={handleSaveAndContinue}
    onDiscard={handleDiscardAndContinue}
    onCancel={handleCancelUnsaved}
  />
{/if}

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }
  .editor-area {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .preview-wrap {
    flex: 1;
    min-width: 0;
    border-right: 1px solid var(--color-border);
    overflow: hidden;
  }
  .props-wrap {
    width: 280px;
    flex-shrink: 0;
    overflow: hidden;
  }
  .timeline-wrap {
    height: 200px;
    flex-shrink: 0;
    border-top: 1px solid var(--color-border);
    overflow: hidden;
  }
</style>
