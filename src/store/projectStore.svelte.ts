import type { Project } from '../types/project';

export class ProjectStore {
	project = $state<Project | null>(null);
	filePath = $state<string | null>(null);
	dirty = $state<boolean>(false);
	selectedSceneId = $state<string | null>(null);
	selectedObjectId = $state<string | null>(null);
	selectedEntryName = $state<string | null>(null);
	checkedEntryNames = $state<Set<string>>(new Set());

	loadProject(project: Project, path: string | null) {
		this.project = project;
		this.filePath = path;
		this.dirty = false;
	}

	updateProject(updater: (p: Project) => void) {
		if (this.project) {
			updater(this.project);
			this.dirty = true;
		}
	}

	setFilePath(path: string) { this.filePath = path; }
	clearDirty() { this.dirty = false; }
	selectScene(id: string | null) { this.selectedSceneId = id; }
	selectObject(id: string | null) { this.selectedObjectId = id; }
	selectEntry(name: string | null) { this.selectedEntryName = name; }
	setCheckedEntries(names: Set<string>) { this.checkedEntryNames = names; }
}

export const projectStore = new ProjectStore();
