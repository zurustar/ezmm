import { describe, it, expect, beforeEach } from 'vitest';
import { SettingsStore } from './settingsStore.svelte';
import type { AppSettings } from '../types/ipc';

const makeSettings = (overrides: Partial<AppSettings> = {}): AppSettings => ({
	version: 1,
	default_crf: 23,
	default_preset: 'medium',
	preview_resolution_scale: 0.5,
	last_open_folder: null,
	recent_files: [],
	window: { width: 1280, height: 800, x: null, y: null },
	...overrides
});

describe('SettingsStore', () => {
	let store: SettingsStore;

	beforeEach(() => {
		store = new SettingsStore();
	});

	// Cycle 5-6
	it('starts with null settings', () => {
		expect(store.settings).toBe(null);
	});

	it('setSettings updates state.settings', () => {
		const s = makeSettings({ default_crf: 28 });
		store.setSettings(s);
		expect(store.settings).toBe(s);
		expect(store.settings?.default_crf).toBe(28);
	});

	it('setSettings replaces previous settings', () => {
		store.setSettings(makeSettings({ default_crf: 20 }));
		store.setSettings(makeSettings({ default_crf: 30 }));
		expect(store.settings?.default_crf).toBe(30);
	});
});
