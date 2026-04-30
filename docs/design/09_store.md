# Svelte 5 状態管理 (Runes)

各 store の state・メソッド定義。store 間の連携ルール。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [05_ipc.md](05_ipc.md)（IPC ペイロード型）、[01_project_schema.md](01_project_schema.md)（Project 型）

---

## Store 一覧

| Store | ファイル | 責務 |
|-------|--------|------|
| `ProjectStore` | `src/store/projectStore.svelte.ts` | プロジェクトデータ・選択状態・dirtyフラグ |
| `PreviewStore` | `src/store/previewStore.svelte.ts` | プレビュー再生状態・現在時刻 |
| `ExportStore` | `src/store/exportStore.svelte.ts` | 書き出し実行状態・進捗・エラー |
| `SettingsStore` | `src/store/settingsStore.svelte.ts` | アプリ設定（load_settings IPC から初期化） |

---

## ProjectStore

```typescript
export class ProjectStore {
  project = $state<Project | null>(null);
  filePath = $state<string | null>(null);      // 未保存の新規なら null
  dirty = $state<boolean>(false);               // 未保存変更の有無
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
```

---

## PreviewStore

```typescript
export class PreviewStore {
  isPlaying = $state<boolean>(false);
  currentTime = $state<number>(0);          // 秒、プロジェクト全体の累積
  totalDuration = $state<number>(0);        // 秒。probe 済みシーン合計時間（未 probe シーンは 0 加算）

  play() { this.isPlaying = true; }
  pause() { this.isPlaying = false; }
  seek(time: number) { this.currentTime = time; }
  setTotalDuration(duration: number) { this.totalDuration = duration; }
}

export const previewStore = new PreviewStore();
```

---

## ExportStore

```typescript
export type ExportStatus = 'idle' | 'running' | 'cancelling' | 'done' | 'error';

export class ExportStore {
  status = $state<ExportStatus>('idle');
  progress = $state<number | null>(null);    // 0.0–1.0、未確定は null
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

  onProgress(progress: number | null) { this.progress = progress; }
  onDone(payload: ExportDonePayload) {
    this.status = 'done';
    this.progress = 1;
    this.outputPath = payload.output_path;
    this.elapsedMs = payload.elapsed_ms;
  }
  onError(payload: ExportErrorPayload) { this.status = 'error'; this.error = payload; }
  onCancelled() { this.status = 'idle'; this.progress = null; }
  reset() { /* status/progress/outputPath/elapsedMs/error を初期値に戻す */ }
}

export const exportStore = new ExportStore();
```

---

## SettingsStore

```typescript
export class SettingsStore {
  settings = $state<AppSettings | null>(null);
  
  setSettings(s: AppSettings) {
    this.settings = s;
  }
}

export const settingsStore = new SettingsStore();
// SettingsStore は +layout.svelte の onMount 時に load_settings IPC を呼んで初期化する
```

---

## Store 間の連携ルール

- **ProjectStore.updateCount** → PreviewCanvas の `$effect` がトラックし、静止フレームを再描画する
- **イベントリスナー登録**: `+layout.svelte` の `onMount` 時に `@tauri-apps/api/event` の `listen` でエクスポートイベント4種（`export:progress`, `export:done`, `export:error`, `export:cancelled`）を一括登録し、アプリ終了（コンポーネントアンマウント）まで保持する
