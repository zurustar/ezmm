import { describe, it, expect, beforeEach } from 'vitest';
import { ProjectStore } from './projectStore.svelte';
import type { Project } from '../types/project';

const makeProject = (overrides: Partial<Project> = {}): Project => ({
	version: 1,
	output_folder: '/out',
	output: { width: 1920, height: 1080, fps: 30, codec: 'h264', format: 'mp4', crf: 23, preset: 'medium' },
	scenes: [],
	entries: [],
	...overrides
});

describe('ProjectStore', () => {
	let store: ProjectStore;

	beforeEach(() => {
		store = new ProjectStore();
	});

	// Cycle 5-2: loadProject
	describe('loadProject', () => {
		it('sets project and filePath, clears dirty', () => {
			const proj = makeProject();
			store.loadProject(proj, '/path/to/file.yaml');
			expect(store.project).toBe(proj);
			expect(store.filePath).toBe('/path/to/file.yaml');
			expect(store.dirty).toBe(false);
		});

		it('accepts null filePath for new unsaved project', () => {
			const proj = makeProject();
			store.loadProject(proj, null);
			expect(store.filePath).toBe(null);
			expect(store.dirty).toBe(false);
		});

		it('resets dirty flag when loading over a dirty state', () => {
			const proj = makeProject();
			store.loadProject(proj, '/first.yaml');
			store.updateProject((p) => { p.output_folder = '/changed'; });
			expect(store.dirty).toBe(true);

			store.loadProject(makeProject(), '/second.yaml');
			expect(store.dirty).toBe(false);
		});
	});

	// Cycle 5-3: updateProject
	describe('updateProject', () => {
		it('applies updater to project and sets dirty=true', () => {
			const proj = makeProject({ output_folder: '/original' });
			store.loadProject(proj, '/file.yaml');

			store.updateProject((p) => { p.output_folder = '/updated'; });

			expect(store.project?.output_folder).toBe('/updated');
			expect(store.dirty).toBe(true);
		});

		it('does nothing when project is null', () => {
			expect(store.project).toBe(null);
			store.updateProject((p) => { p.output_folder = '/noop'; });
			expect(store.project).toBe(null);
			expect(store.dirty).toBe(false);
		});
	});
});
