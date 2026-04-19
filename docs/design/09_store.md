# Zustand 状態管理

各 store の state・actions 定義。store 間の連携ルール。

> **参照元**: [設計書インデックス](../design.md)  
> **依存**: [05_ipc.md](05_ipc.md)（IPC ペイロード型）、[01_project_schema.md](01_project_schema.md)（Project 型）

---

## Store 一覧

| Store | ファイル | 責務 |
|-------|--------|------|
| `ProjectStore` | `src/store/projectStore.ts` | プロジェクトデータ・選択状態・dirtyフラグ |
| `PreviewStore` | `src/store/previewStore.ts` | プレビュー再生状態・現在時刻・AudioContext |
| `BatchStore` | `src/store/batchStore.ts` | バッチ実行状態・進捗・エラー |
| `SettingsStore` | `src/store/settingsStore.ts` | アプリ設定（load_settings IPC から初期化） |

---

## ProjectStore

```typescript
interface ProjectStoreState {
  project: Project | null;
  filePath: string | null;      // 未保存の新規なら null
  dirty: boolean;               // 未保存変更の有無
  selectedSceneId: string | null;
  selectedObjectId: string | null;
  selectedEntryName: string | null;    // プレビュー対象エントリ
  checkedEntryNames: Set<string>;      // バッチ実行対象エントリ（セッション限定・永続化なし。アプリ再起動時は全エントリ選択済みで初期化）。IPC 送信時は `Array.from(project.entries.map(e => e.name).filter(n => checkedEntryNames.has(n)))` でエントリ一覧の表示順に変換する
  // actions
  loadProject: (project: Project, path: string | null) => void;
  updateProject: (updater: (p: Project) => void) => void;  // 呼ぶと dirty = true
  setFilePath: (path: string) => void;
  clearDirty: () => void;
  selectScene: (id: string | null) => void;
  selectObject: (id: string | null) => void;
  selectEntry: (name: string | null) => void;
  setCheckedEntries: (names: Set<string>) => void;
}
```

---

## PreviewStore

```typescript
interface PreviewStoreState {
  isPlaying: boolean;
  currentTime: number;          // 秒、プロジェクト全体の累積
  totalDuration: number;        // 秒。選択中エントリのシーン合計時間。可変映像が未指定でシーン長不明の場合は 0 で初期化し、loadProject / updateProject 時に再計算する。実装方式: ProjectStore の `loadProject` / `updateProject` アクション内で PreviewStore の `setTotalDuration()` を直接呼び出す（zustand の subscribe は使わない。副作用を1箇所に集約するため）
  audioContextReady: boolean;   // autoplay ポリシー対応フラグ
  // actions
  play: () => void;
  pause: () => void;
  seek: (time: number) => void;
  resumeAudioContext: () => void;
  setTotalDuration: (duration: number) => void;  // ProjectStore から呼ぶ
}
```

---

## BatchStore

```typescript
interface BatchStoreState {
  status: 'idle' | 'running' | 'cancelling' | 'done';
  currentEntryIndex: number;    // 0始まり
  totalEntries: number;
  currentEntryName: string | null;
  currentEntryProgress: number; // 0.0–1.0。entry_progress が null/undefined のときは 0.0 で初期化。entry_done 受信後は 1.0 にセット
  errors: BatchEntryErrorPayload[];
  startedAt: number | null;     // 残り時間予測用 epoch ms
  // actions
  startBatch: (entryNames: string[], overwritePolicy: 'overwrite' | 'skip') => Promise<void>;
  // entryNames: 空配列は全エントリを対象とする（IPC start_batch の entry_names: Vec<String> 空 Vec と整合）
  // 呼び出し前に check_output_conflicts IPC で衝突確認 → ダイアログ → overwritePolicy を決定してから呼ぶ
  cancelBatch: () => Promise<void>;  // cancel_batch IPC 呼び出し直前に status を 'cancelling' にセット
  onProgress: (payload: BatchProgressPayload) => void;
  onEntryDone: (payload: BatchEntryDonePayload) => void;
  onEntryError: (payload: BatchEntryErrorPayload) => void;
  // batch:entry_error 受信時: errors[] にエラーを追加。Rust 側は続けて batch:done を発行するため status はそこで 'done' に遷移する（status はこのハンドラでは変更しない）
  onDone: (payload: BatchDonePayload) => void;  // batch:done 受信時: status を 'done' にセット。完了ダイアログを表示し、ユーザーが OK を押したら 'idle' にリセット
  onCancelled: () => void;   // batch:cancelled 受信時: status を 'idle' にリセット。status 遷移: idle → running → cancelling → idle
}
```

---

## SettingsStore

```typescript
interface SettingsStoreState {
  settings: AppSettings;               // load_settings IPC で取得。アプリ起動時に初期化
  setSettings: (s: AppSettings) => void;  // save_settings IPC 呼び出し後に更新
}
// SettingsStore は App.tsx マウント時に load_settings IPC を呼んで初期化する
```

---

## Store 間の連携ルール

- **ProjectStore → PreviewStore**: `loadProject` / `updateProject` アクション内で `PreviewStore.setTotalDuration()` を直接呼び出す（zustand の `subscribe` は使わない）
- **BatchStore → ProjectStore**: BatchStore の `startBatch` は `ProjectStore.project` を参照して IPC に渡す
- **イベントリスナー登録**: `App.tsx` マウント時（`useEffect` 初回実行）に `@tauri-apps/api/event` の `listen` でバッチイベント5種を一括登録し、アプリ終了まで保持する。多重 mount 防止のため登録フラグ（ref または module-level singleton）を使い、2回目以降の `listen` 呼び出しをスキップする
