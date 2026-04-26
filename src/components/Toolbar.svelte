<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { projectStore } from '../store/projectStore.svelte';
	import { batchStore } from '../store/batchStore.svelte';
	import { settingsStore } from '../store/settingsStore.svelte';
	import type { Project } from '../types/project';
	import type { ValidationResult } from '../types/ipc';

	interface Props {
		onRequestNew: () => void;
		onRequestOpen: () => void;
	}
	let { onRequestNew, onRequestOpen }: Props = $props();

	let project = $derived(projectStore.project);
	let isBusy = $derived(batchStore.status === 'running' || batchStore.status === 'cancelling');

	async function handleNew() {
		onRequestNew();
	}

	async function handleOpen() {
		onRequestOpen();
	}

	async function handleSave() {
		if (!project) return;
		if (!projectStore.filePath) {
			await handleSaveAs();
			return;
		}
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
		try {
			await invoke('save_settings', { settings: updated });
		} catch { /* best effort */ }
	}

	async function handleBrowseOutputFolder() {
		const path = await invoke<string | null>('show_folder_dialog');
		if (!path || !project) return;
		projectStore.updateProject((p) => { p.output_folder = path; });
	}

	async function handleBatchRun() {
		if (!project) return;

		// Validate first
		const result = await invoke<ValidationResult>('validate_project', { project });
		const errors = result.errors.filter(
			(e) => !(e.code === 'scenes_empty' || e.code === 'entries_empty')
		);
		if (errors.length > 0) {
			alert(`バリデーションエラー:\n${errors.map((e) => e.message).join('\n')}`);
			return;
		}

		const checkedNames = [...projectStore.checkedEntryNames];
		const entryNames = checkedNames.length > 0 ? checkedNames : project.entries.map((e) => e.name);

		// Check conflicts
		const conflicts = await invoke<string[]>('check_output_conflicts', { project, entryNames });
		let overwritePolicy: 'overwrite' | 'skip' = 'skip';
		if (conflicts.length > 0) {
			const msg = `以下のファイルが既に存在します:\n${conflicts.join('\n')}\n\n上書きしますか？`;
			if (confirm(msg)) {
				overwritePolicy = 'overwrite';
			}
		}

		try {
			await batchStore.startBatch(project, entryNames, overwritePolicy);
		} catch (e) {
			alert(`バッチ実行に失敗しました: ${e}`);
		}
	}

	async function handleCancel() {
		await batchStore.cancelBatch();
	}

	let outputFolder = $derived(project?.output_folder ?? '');
	let outputFolderInvalid = $derived(outputFolder.trim() === '');
</script>

<header class="toolbar">
	<div class="toolbar-left">
		<button onclick={handleNew} disabled={isBusy}>新規</button>
		<button onclick={handleOpen} disabled={isBusy}>開く</button>
		<button onclick={handleSave} disabled={!project || isBusy}>保存</button>
		<button onclick={handleSaveAs} disabled={!project || isBusy}>名前を付けて保存</button>

		<span class="sep"></span>

		<label for="output-folder">出力先:</label>
		<input
			id="output-folder"
			type="text"
			class="output-folder-input"
			class:invalid={outputFolderInvalid}
			value={outputFolder}
			oninput={(e) => {
				const v = (e.target as HTMLInputElement).value;
				if (project) projectStore.updateProject((p) => { p.output_folder = v; });
			}}
			disabled={isBusy}
			placeholder="（未設定）"
		/>
		<button onclick={handleBrowseOutputFolder} disabled={!project || isBusy} aria-label="参照">📁</button>
	</div>

	<div class="toolbar-right">
		{#if batchStore.status === 'idle' || batchStore.status === 'done'}
			<button
				class="btn-primary"
				onclick={handleBatchRun}
				disabled={!project || outputFolderInvalid || isBusy}
			>バッチ実行</button>
		{:else}
			<button onclick={handleCancel} disabled={batchStore.status === 'cancelling'}>
				{batchStore.status === 'cancelling' ? 'キャンセル中...' : 'キャンセル'}
			</button>
		{/if}
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
		flex: 1;
		min-width: 0;
	}

	.toolbar-right {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}

	.sep {
		width: 1px;
		height: 20px;
		background: var(--color-border);
		margin: 0 4px;
	}

	.output-folder-input {
		width: 240px;
		min-width: 100px;
	}

	.output-folder-input.invalid {
		border-color: var(--color-error);
	}
</style>
