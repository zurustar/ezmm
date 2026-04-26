# Svelte 5 状態管理 (Runes)

各 store の state・メソッド定義。store 間の連携ルール。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [05_ipc.md](05_ipc.md)（IPC ペイロード型）、[01_project_schema.md](01_project_schema.md)（Project 型）

---

## Store 一覧

| Store | ファイル | 責務 |
|-------|--------|------|
| `ProjectStore` | `src/store/projectStore.svelte.ts` | プロジェクトデータ・選択状態・dirtyフラグ |
| `PreviewStore` | `src/store/previewStore.svelte.ts` | プレビュー再生状態・現在時刻・AudioContext |
| `BatchStore` | `src/store/batchStore.svelte.ts` | バッチ実行状態・進捗・エラー |
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
  selectedEntryName = $state<string | null>(null);    // プレビュー対象エントリ
  checkedEntryNames = $state<Set<string>>(new Set()); // バッチ実行対象エントリ

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
```

---

## PreviewStore

```typescript
export class PreviewStore {
  isPlaying = $state<boolean>(false);
  currentTime = $state<number>(0);          // 秒、プロジェクト全体の累積
  totalDuration = $state<number>(0);        // 秒。選択中エントリのシーン合計時間。
  audioContextReady = $state<boolean>(false);   // autoplay ポリシー対応フラグ

  play() { this.isPlaying = true; }
  pause() { this.isPlaying = false; }
  seek(time: number) { this.currentTime = time; }
  resumeAudioContext() { this.audioContextReady = true; }
  setTotalDuration(duration: number) { this.totalDuration = duration; }
}

export const previewStore = new PreviewStore();
```

---

## BatchStore

```typescript
export class BatchStore {
  status = $state<'idle' | 'running' | 'cancelling' | 'done'>('idle');
  currentEntryIndex = $state<number>(0);
  totalEntries = $state<number>(0);
  currentEntryName = $state<string | null>(null);
  currentEntryProgress = $state<number>(0);
  errors = $state<BatchEntryErrorPayload[]>([]);
  startedAt = $state<number | null>(null);

  async startBatch(entryNames: string[], overwritePolicy: 'overwrite' | 'skip') {
    // 呼び出し前に IPC で衝突をチェックし実行を開始する
  }
  
  async cancelBatch() {
    this.status = 'cancelling';
    // IPC にキャンセルをリクエスト
  }

  onProgress(payload: BatchProgressPayload) { /* 受信時の処理 */ }
  onEntryDone(payload: BatchEntryDonePayload) { /* 受信時の処理 */ }
  onEntryError(payload: BatchEntryErrorPayload) { /* エラー追加処理 */ }
  onDone(payload: BatchDonePayload) { this.status = 'done'; }
  onCancelled() { this.status = 'idle'; }
}

export const batchStore = new BatchStore();
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

- **ProjectStore → PreviewStore**: `loadProject` / `updateProject` 実行時に `previewStore.setTotalDuration()` を呼び出す（Svelte 5 ではメソッド内で直接参照しやすい）。
- **イベントリスナー登録**: `+layout.svelte` の `onMount` 時に `@tauri-apps/api/event` の `listen` でバッチイベント5種を一括登録し、アプリ終了（コンポーネントアンマウント）まで保持する。
