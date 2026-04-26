import { describe, it, expect, beforeEach } from 'vitest';
import { BatchStore } from './batchStore.svelte';
import type { BatchProgressPayload, BatchEntryErrorPayload } from '../types/ipc';

describe('BatchStore', () => {
	let store: BatchStore;

	beforeEach(() => {
		store = new BatchStore();
	});

	// Cycle 5-4: status state transitions
	describe('status transitions', () => {
		it('starts in idle state', () => {
			expect(store.status).toBe('idle');
		});

		it('transitions idle → running on startBatch', () => {
			store._setRunning(3);
			expect(store.status).toBe('running');
			expect(store.totalEntries).toBe(3);
		});

		it('transitions running → cancelling on cancelBatch', () => {
			store._setRunning(2);
			store.cancelBatch();
			expect(store.status).toBe('cancelling');
		});

		it('transitions cancelling → idle on onCancelled', () => {
			store._setRunning(1);
			store.cancelBatch();
			store.onCancelled();
			expect(store.status).toBe('idle');
		});

		it('transitions running → done on onDone', () => {
			store._setRunning(2);
			store.onDone({ success_count: 2, error_count: 0, total_elapsed_ms: 5000 });
			expect(store.status).toBe('done');
		});
	});

	// Cycle 5-5: progress updates
	describe('progress updates', () => {
		it('onProgress updates currentEntryIndex, currentEntryName, currentEntryProgress', () => {
			const payload: BatchProgressPayload = {
				entry_index: 1,
				total: 5,
				entry_name: 'tanaka',
				entry_progress: 0.42
			};
			store.onProgress(payload);
			expect(store.currentEntryIndex).toBe(1);
			expect(store.currentEntryName).toBe('tanaka');
			expect(store.currentEntryProgress).toBe(0.42);
		});

		it('onProgress handles missing entry_progress (undefined)', () => {
			const payload: BatchProgressPayload = {
				entry_index: 0,
				total: 1,
				entry_name: 'entry0'
			};
			store.onProgress(payload);
			expect(store.currentEntryProgress).toBe(0);
		});

		it('onEntryError appends to errors array', () => {
			const err: BatchEntryErrorPayload = { entry_name: 'suzuki', message: 'FFmpeg error' };
			store.onEntryError(err);
			expect(store.errors).toHaveLength(1);
			expect(store.errors[0].entry_name).toBe('suzuki');
		});

		it('errors accumulate across multiple onEntryError calls', () => {
			store.onEntryError({ entry_name: 'a', message: 'err1' });
			store.onEntryError({ entry_name: 'b', message: 'err2' });
			expect(store.errors).toHaveLength(2);
		});

		it('onCancelled resets errors', () => {
			store.onEntryError({ entry_name: 'a', message: 'err' });
			store._setRunning(1);
			store.cancelBatch();
			store.onCancelled();
			expect(store.errors).toHaveLength(0);
		});
	});
});
