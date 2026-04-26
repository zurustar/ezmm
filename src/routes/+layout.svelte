<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import '../app.css';
	import { settingsStore } from '../store/settingsStore.svelte';
	import { batchStore } from '../store/batchStore.svelte';
	import { projectStore } from '../store/projectStore.svelte';
	import type {
		AppSettings,
		BatchProgressPayload,
		BatchEntryDonePayload,
		BatchEntryErrorPayload,
		BatchDonePayload
	} from '../types/ipc';

	let { children } = $props();

	// Keep window title in sync with project state
	$effect(() => {
		const filePath = projectStore.filePath;
		const dirty = projectStore.dirty;
		const fileName = filePath ? filePath.split(/[\\/]/).pop() ?? filePath : '無題';
		const title = `ezmm — ${fileName}${dirty ? '*' : ''}`;
		document.title = title;
	});

	onMount(async () => {
		// Load settings on startup
		try {
			const s = await invoke<AppSettings>('load_settings');
			settingsStore.setSettings(s);
		} catch (e) {
			console.error('設定の読み込みに失敗しました:', e);
		}

		// Register Tauri batch event listeners
		const unlisteners = await Promise.all([
			listen<BatchProgressPayload>('batch:progress', ({ payload }) =>
				batchStore.onProgress(payload)
			),
			listen<BatchEntryDonePayload>('batch:entry_done', ({ payload }) =>
				batchStore.onEntryDone(payload)
			),
			listen<BatchEntryErrorPayload>('batch:entry_error', ({ payload }) =>
				batchStore.onEntryError(payload)
			),
			listen<BatchDonePayload>('batch:done', ({ payload }) => batchStore.onDone(payload)),
			listen<null>('batch:cancelled', () => batchStore.onCancelled())
		]);

		return () => unlisteners.forEach((fn) => fn());
	});
</script>

{@render children()}
