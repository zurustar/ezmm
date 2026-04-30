import { invoke } from '@tauri-apps/api/core';
import type { Project } from '../types/project';
import type { ExportDonePayload, ExportErrorPayload } from '../types/ipc';

export type ExportStatus = 'idle' | 'running' | 'cancelling' | 'done' | 'error';

export class ExportStore {
  status = $state<ExportStatus>('idle');
  progress = $state<number | null>(null);
  outputPath = $state<string | null>(null);
  elapsedMs = $state<number | null>(null);
  error = $state<ExportErrorPayload | null>(null);

  get isRunning() {
    return this.status === 'running' || this.status === 'cancelling';
  }

  async startExport(project: Project): Promise<void> {
    this.status = 'running';
    this.progress = null;
    this.outputPath = null;
    this.elapsedMs = null;
    this.error = null;
    await invoke('start_export', { project });
  }

  async cancelExport(): Promise<void> {
    this.status = 'cancelling';
    await invoke('cancel_export');
  }

  onProgress(progress: number | null) {
    this.progress = progress;
  }

  onDone(payload: ExportDonePayload) {
    this.status = 'done';
    this.progress = 1;
    this.outputPath = payload.output_path;
    this.elapsedMs = payload.elapsed_ms;
  }

  onError(payload: ExportErrorPayload) {
    this.status = 'error';
    this.error = payload;
  }

  onCancelled() {
    this.status = 'idle';
    this.progress = null;
  }

  reset() {
    this.status = 'idle';
    this.progress = null;
    this.outputPath = null;
    this.elapsedMs = null;
    this.error = null;
  }

  // テスト用
  _setRunning() {
    this.status = 'running';
    this.progress = null;
  }
}

export const exportStore = new ExportStore();
