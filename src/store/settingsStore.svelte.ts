import type { AppSettings } from '../types/ipc';

export class SettingsStore {
	settings = $state<AppSettings | null>(null);

	setSettings(s: AppSettings) {
		this.settings = s;
	}
}

export const settingsStore = new SettingsStore();
// Initialized in +layout.svelte onMount via load_settings IPC
