import { invoke } from '@tauri-apps/api/core';
import type {
	BatchProgressPayload,
	BatchEntryDonePayload,
	BatchEntryErrorPayload,
	BatchDonePayload
} from '../types/ipc';
import type { Project } from '../types/project';

export class BatchStore {
	status = $state<'idle' | 'running' | 'cancelling' | 'done'>('idle');
	currentEntryIndex = $state<number>(0);
	totalEntries = $state<number>(0);
	currentEntryName = $state<string | null>(null);
	currentEntryProgress = $state<number>(0);
	errors = $state<BatchEntryErrorPayload[]>([]);
	startedAt = $state<number | null>(null);

	// Called by UI before IPC; exposed for testing
	_setRunning(total: number) {
		this.status = 'running';
		this.totalEntries = total;
		this.currentEntryIndex = 0;
		this.currentEntryName = null;
		this.currentEntryProgress = 0;
		this.errors = [];
		this.startedAt = Date.now();
	}

	async startBatch(
		project: Project,
		entryNames: string[],
		overwritePolicy: 'overwrite' | 'skip'
	) {
		this._setRunning(entryNames.length || 0);
		await invoke('start_batch', {
			project,
			entryNames,
			overwritePolicy
		});
	}

	async cancelBatch() {
		this.status = 'cancelling';
		await invoke('cancel_batch');
	}

	onProgress(payload: BatchProgressPayload) {
		this.currentEntryIndex = payload.entry_index;
		this.currentEntryName = payload.entry_name;
		this.currentEntryProgress = payload.entry_progress ?? 0;
	}

	onEntryDone(_payload: BatchEntryDonePayload) {
		// Progress is driven by onProgress; nothing to update here
	}

	onEntryError(payload: BatchEntryErrorPayload) {
		this.errors = [...this.errors, payload];
	}

	onDone(_payload: BatchDonePayload) {
		this.status = 'done';
	}

	onCancelled() {
		this.status = 'idle';
		this.errors = [];
	}
}

export const batchStore = new BatchStore();
