import { describe, it, expect, beforeEach } from 'vitest';
import { ExportStore } from './exportStore.svelte';

describe('ExportStore', () => {
  let store: ExportStore;

  beforeEach(() => {
    store = new ExportStore();
  });

  describe('status transitions', () => {
    it('starts in idle state', () => {
      expect(store.status).toBe('idle');
    });

    it('transitions to running on _setRunning', () => {
      store._setRunning();
      expect(store.status).toBe('running');
      expect(store.isRunning).toBe(true);
    });

    it('transitions running → cancelling on cancelExport (sync part)', () => {
      store._setRunning();
      store.status = 'cancelling';
      expect(store.status).toBe('cancelling');
      expect(store.isRunning).toBe(true);
    });

    it('transitions to idle on onCancelled', () => {
      store._setRunning();
      store.onCancelled();
      expect(store.status).toBe('idle');
      expect(store.progress).toBeNull();
    });

    it('transitions to done on onDone', () => {
      store._setRunning();
      store.onDone({ output_path: '/out/video.mp4', elapsed_ms: 3000 });
      expect(store.status).toBe('done');
      expect(store.progress).toBe(1);
      expect(store.outputPath).toBe('/out/video.mp4');
      expect(store.elapsedMs).toBe(3000);
    });

    it('transitions to error on onError', () => {
      store._setRunning();
      store.onError({ message: 'FFmpeg error', ffmpeg_stderr: null });
      expect(store.status).toBe('error');
      expect(store.error?.message).toBe('FFmpeg error');
    });
  });

  describe('progress updates', () => {
    it('onProgress updates progress value', () => {
      store._setRunning();
      store.onProgress(0.5);
      expect(store.progress).toBe(0.5);
    });

    it('onProgress handles null progress', () => {
      store._setRunning();
      store.onProgress(null);
      expect(store.progress).toBeNull();
    });
  });

  describe('reset', () => {
    it('reset clears all state', () => {
      store.onDone({ output_path: '/out/x.mp4', elapsed_ms: 1000 });
      store.reset();
      expect(store.status).toBe('idle');
      expect(store.outputPath).toBeNull();
      expect(store.error).toBeNull();
    });
  });
});
