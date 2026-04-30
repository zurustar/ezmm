import type { Project } from '../types/project';

export class ProjectStore {
  project = $state<Project | null>(null);
  filePath = $state<string | null>(null);
  dirty = $state<boolean>(false);
  selectedSceneId = $state<string | null>(null);
  selectedObjectId = $state<string | null>(null);
  /** Increments on every loadProject / updateProject call — use to trigger renders */
  updateCount = $state(0);

  loadProject(project: Project, path: string | null) {
    this.project = project;
    this.filePath = path;
    this.dirty = false;
    this.updateCount++;
  }

  updateProject(updater: (p: Project) => void) {
    if (this.project) {
      updater(this.project);
      this.dirty = true;
      this.updateCount++;
    }
  }

  setFilePath(path: string) { this.filePath = path; }
  clearDirty() { this.dirty = false; }
  selectScene(id: string | null) { this.selectedSceneId = id; }
  selectObject(id: string | null) { this.selectedObjectId = id; }
}

export const projectStore = new ProjectStore();
