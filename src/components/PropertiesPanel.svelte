<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { projectStore } from '../store/projectStore.svelte';
  import type { Project, SceneObject, VideoObject, ImageObject, TextObject, AudioObject } from '../types/project';
  import type { ProbeResult } from '../types/ipc';

  type Tab = 'object' | 'output';
  let activeTab = $state<Tab>('object');

  let project = $derived(projectStore.project);
  let selectedSceneId = $derived(projectStore.selectedSceneId);
  let selectedObjectId = $derived(projectStore.selectedObjectId);

  let selectedScene = $derived(project?.scenes.find((s) => s.id === selectedSceneId) ?? null);
  let selectedObject = $derived(
    selectedScene?.objects.find((o) => o.id === selectedObjectId) ?? null
  );

  function updateObj<T extends SceneObject>(updater: (o: T) => void) {
    if (!selectedObjectId || !selectedSceneId) return;
    projectStore.updateProject((p) => {
      const scene = p.scenes.find((s) => s.id === selectedSceneId);
      if (!scene) return;
      const obj = scene.objects.find((o) => o.id === selectedObjectId);
      if (obj) updater(obj as T);
    });
  }

  function updateOutput(updater: (o: Project['output']) => void) {
    if (!project) return;
    projectStore.updateProject((p) => updater(p.output));
  }

  function updateOutputFolder(value: string) {
    if (!project) return;
    projectStore.updateProject((p) => { p.output_folder = value; });
  }

  async function browseOutputFolder() {
    const path = await invoke<string | null>('show_folder_dialog');
    if (!path) return;
    projectStore.updateProject((p) => { p.output_folder = path; });
  }

  async function browseFile(objectId: string) {
    const scene = project?.scenes.find((s) => s.id === selectedSceneId);
    const obj = scene?.objects.find((o) => o.id === objectId);
    const kind = obj?.type === 'video' ? 'video' : obj?.type === 'image' ? 'image' : obj?.type === 'audio' ? 'audio' : 'any';
    const path = await invoke<string | null>('show_open_file_dialog', { kind });
    if (!path) return;

    projectStore.updateProject((p) => {
      const sc = p.scenes.find((s) => s.id === selectedSceneId);
      if (!sc) return;
      const o = sc.objects.find((o) => o.id === objectId);
      if (o && o.type !== 'text') (o as { file?: string }).file = path;
    });

    if ((obj?.type === 'video' || obj?.type === 'audio' || obj?.type === 'image') && selectedSceneId) {
      const sceneId = selectedSceneId;
      try {
        const probe = await invoke<ProbeResult>('probe_file', { path });

        // ソース寸法をオブジェクトに反映し、アスペクト比を記憶（video / image のみ）
        // 出力キャンバスに収まるようアスペクト比を維持しつつフィットさせ、中央配置する
        if (probe.width && probe.height && obj?.type !== 'audio') {
          const probeRatio = probe.width / probe.height;
          objectSourceRatios.set(objectId, probeRatio);
          _objLockedRatio = probeRatio; // effect の非同期実行を待たず即反映
          const srcW = probe.width;
          const srcH = probe.height;
          const outW = project?.output.width ?? srcW;
          const outH = project?.output.height ?? srcH;
          const fitScale = Math.min(outW / srcW, outH / srcH);
          const fitW = Math.round(srcW * fitScale);
          const fitH = Math.round(srcH * fitScale);
          const fitX = Math.round((outW - fitW) / 2);
          const fitY = Math.round((outH - fitH) / 2);
          projectStore.updateProject((p) => {
            const sc = p.scenes.find((s) => s.id === sceneId);
            if (!sc) return;
            const o = sc.objects.find((o) => o.id === objectId);
            if (o && 'width' in o && 'height' in o) {
              (o as { x: number; y: number; width: number; height: number }).x = fitX;
              (o as { x: number; y: number; width: number; height: number }).y = fitY;
              (o as { x: number; y: number; width: number; height: number }).width = fitW;
              (o as { x: number; y: number; width: number; height: number }).height = fitH;
            }
          });
        }

        // シーン長の更新（video / audio のみ）
        if (obj?.type !== 'image') {
          const mediaDuration = probe.duration ?? 0;
          if (mediaDuration > 0) {
            projectStore.updateProject((p) => {
              const sc = p.scenes.find((s) => s.id === sceneId);
              if (!sc) return;
              const o = sc.objects.find((o) => o.id === objectId);
              const objStart = o ? (o as { start: number }).start : 0;
              const needed = objStart + mediaDuration;
              if (!sc.duration || sc.duration < needed) sc.duration = Math.ceil(needed);
            });
          }
        }
      } catch {
        // probe 失敗時はシーン長を変えない
      }
    }
  }

  const PRESETS = ['ultrafast','superfast','veryfast','faster','fast','medium','slow','slower','veryslow'];
  const ALLOWED_FONTS = ['NotoSansCJK-Regular', 'NotoSansCJK-Bold'];

  // objectId → probe 由来の正確なアスペクト比 (width/height)。browseFile 時にセット
  const objectSourceRatios = new Map<string, number>();
  // 現在選択中オブジェクトのロック済みアスペクト比。$effect でオブジェクト選択時に確定する
  let _objLockedRatio = 1;
  // 出力設定用（onfocus 時にキャプチャ）
  let _outLockedRatio = 1;

  // selectedObjectId が変わるたび（＝別オブジェクトを選んだとき）に比率を確定する。
  // selectedObject の寸法変更では再計算しないことで誤差の積み重ねを防ぐ。
  $effect(() => {
    const id = selectedObjectId;
    if (!id) { _objLockedRatio = 1; return; }

    // probe データが最優先（整数丸め誤差なし）
    const probeRatio = objectSourceRatios.get(id);
    if (probeRatio !== undefined) { _objLockedRatio = probeRatio; return; }

    // probe なし：選択時点の寸法から取得
    const obj = selectedObject;
    if (obj && obj.type !== 'audio') {
      const w = (obj as VideoObject | ImageObject | TextObject).width;
      const h = (obj as VideoObject | ImageObject | TextObject).height;
      if (w > 0 && h > 0) _objLockedRatio = w / h;
    }
  });
</script>

<div class="props-panel">
  <div class="tabs">
    <button class:active={activeTab === 'object'} onclick={() => (activeTab = 'object')}>
      プロパティ
</button>
    <button class:active={activeTab === 'output'} onclick={() => (activeTab = 'output')}>
      出力設定
    </button>
  </div>

  <div class="panel-body">
    {#if activeTab === 'object'}
      {#if !selectedObject}
        <p class="hint">動画・画像・テキスト・音声を選択してください</p>

      {:else}
        {@const OBJ_TYPE_LABEL: Record<string, string> = { video: '動画', image: '画像', text: 'テキスト', audio: '音声' }}
        <div class="section-title">{OBJ_TYPE_LABEL[selectedObject.type] ?? selectedObject.type}</div>

        <!-- 共通フィールド -->
        <div class="field-row">
          <label>開始</label>
          <input type="number" step="0.01" value={selectedObject.start}
            oninput={(e) => updateObj((o) => { o.start = parseFloat((e.target as HTMLInputElement).value) || 0; })} />
          <span class="unit">s</span>
        </div>

        {#if selectedObject.type !== 'audio'}
          {@const o = selectedObject as VideoObject | ImageObject | TextObject}
          <div class="field-row">
            <label>X</label>
            <input type="number" value={o.x} oninput={(e) => updateObj<typeof o>((obj) => { obj.x = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">px</span>
          </div>
          <div class="field-row">
            <label>Y</label>
            <input type="number" value={o.y} oninput={(e) => updateObj<typeof o>((obj) => { obj.y = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">px</span>
          </div>
          <div class="field-row">
            <label>幅</label>
            <input type="number" value={o.width}
              oninput={(e) => {
                const w = +(e.target as HTMLInputElement).value;
                if (w <= 0 || !isFinite(w)) return;
                const ratio = _objLockedRatio;
                updateObj<typeof o>((obj) => { obj.width = w; obj.height = Math.round(w / ratio); });
              }} />
            <span class="unit">px</span>
          </div>
          <div class="field-row">
            <label>高さ</label>
            <input type="number" value={o.height}
              oninput={(e) => {
                const h = +(e.target as HTMLInputElement).value;
                if (h <= 0 || !isFinite(h)) return;
                const ratio = _objLockedRatio;
                updateObj<typeof o>((obj) => { obj.height = h; obj.width = Math.round(h * ratio); });
              }} />
            <span class="unit">px</span>
          </div>
          <div class="field-row">
            <label>不透明度</label>
            <input type="number" min="0" max="100" value={o.opacity} oninput={(e) => updateObj<typeof o>((obj) => { obj.opacity = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">%</span>
          </div>
        {/if}

        {#if selectedObject.type === 'video'}
          {@const o = selectedObject as VideoObject}
          <div class="field-row">
            <label>ファイル</label>
            <input type="text" value={o.file ?? ''} oninput={(e) => updateObj<VideoObject>((obj) => { obj.file = (e.target as HTMLInputElement).value; })} />
            <button onclick={() => browseFile(o.id)}>...</button>
          </div>
          <div class="field-row">
            <label>音量</label>
            <input type="number" min="0" max="100" value={o.volume} oninput={(e) => updateObj<VideoObject>((obj) => { obj.volume = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">%</span>
          </div>
          <div class="field-row">
            <label>トリム開始</label>
            <input type="number" step="0.01" min="0" placeholder="0"
              value={o.trim_start ?? ''}
              oninput={(e) => updateObj<VideoObject>((obj) => { const v = parseFloat((e.target as HTMLInputElement).value); obj.trim_start = isNaN(v) || v <= 0 ? undefined : v; })} />
            <span class="unit">s</span>
          </div>
          <div class="field-row">
            <label>トリム終了</label>
            <input type="number" step="0.01" min="0" placeholder="末尾"
              value={o.trim_end ?? ''}
              oninput={(e) => updateObj<VideoObject>((obj) => { const v = parseFloat((e.target as HTMLInputElement).value); obj.trim_end = isNaN(v) || v <= 0 ? undefined : v; })} />
            <span class="unit">s</span>
          </div>

        {:else if selectedObject.type === 'image'}
          {@const o = selectedObject as ImageObject}
          <div class="field-row">
            <label>ファイル</label>
            <input type="text" value={o.file ?? ''} oninput={(e) => updateObj<ImageObject>((obj) => { obj.file = (e.target as HTMLInputElement).value; })} />
            <button onclick={() => browseFile(o.id)}>...</button>
          </div>
          <div class="field-row">
            <label>表示時間</label>
            <input type="number" step="0.01" value={o.duration} oninput={(e) => updateObj<ImageObject>((obj) => { obj.duration = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">s</span>
          </div>

        {:else if selectedObject.type === 'text'}
          {@const o = selectedObject as TextObject}
          <div class="field-row">
            <label>テキスト</label>
            <input type="text" value={o.text ?? ''} oninput={(e) => updateObj<TextObject>((obj) => { obj.text = (e.target as HTMLInputElement).value; })} />
          </div>
          <div class="field-row">
            <label>フォントサイズ</label>
            <input type="number" min="1" value={o.font_size} oninput={(e) => updateObj<TextObject>((obj) => { obj.font_size = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">pt</span>
          </div>
          <div class="field-row">
            <label>文字色</label>
            <input type="color" value={o.color} oninput={(e) => updateObj<TextObject>((obj) => { obj.color = (e.target as HTMLInputElement).value; })} />
          </div>
          <div class="field-row">
            <label>フォント</label>
            <select value={o.font} onchange={(e) => updateObj<TextObject>((obj) => { obj.font = (e.target as HTMLSelectElement).value; })}>
              {#each ALLOWED_FONTS as f}
                <option value={f}>{f}</option>
              {/each}
            </select>
          </div>
          <div class="field-row">
            <label>揃え</label>
            <select value={o.align ?? 'left'} onchange={(e) => updateObj<TextObject>((obj) => { obj.align = (e.target as HTMLSelectElement).value as 'left'|'center'|'right'; })}>
              <option value="left">左</option>
              <option value="center">中央</option>
              <option value="right">右</option>
            </select>
          </div>
          <div class="field-row">
            <label>背景色</label>
            <input type="checkbox" checked={o.background_color !== undefined}
              onchange={(e) => updateObj<TextObject>((obj) => {
                obj.background_color = (e.target as HTMLInputElement).checked ? (obj.background_color ?? '#000000') : undefined;
              })} />
            {#if o.background_color !== undefined}
              <input type="color" value={o.background_color}
                oninput={(e) => updateObj<TextObject>((obj) => { obj.background_color = (e.target as HTMLInputElement).value; })} />
            {/if}
          </div>
          <div class="field-row">
            <label>表示時間</label>
            <input type="number" step="0.01" value={o.duration} oninput={(e) => updateObj<TextObject>((obj) => { obj.duration = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">s</span>
          </div>

        {:else if selectedObject.type === 'audio'}
          {@const o = selectedObject as AudioObject}
          <div class="field-row">
            <label>ファイル</label>
            <input type="text" value={o.file ?? ''} oninput={(e) => updateObj<AudioObject>((obj) => { obj.file = (e.target as HTMLInputElement).value; })} />
            <button onclick={() => browseFile(o.id)}>...</button>
          </div>
          <div class="field-row">
            <label>再生時間</label>
            <input type="number" step="0.01" value={o.duration} oninput={(e) => updateObj<AudioObject>((obj) => { obj.duration = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">s</span>
          </div>
          <div class="field-row">
            <label>音量</label>
            <input type="number" min="0" max="100" value={o.volume} oninput={(e) => updateObj<AudioObject>((obj) => { obj.volume = +(e.target as HTMLInputElement).value; })} />
            <span class="unit">%</span>
          </div>
          <div class="field-row">
            <label>フェードイン</label>
            <input type="number" step="0.1" min="0" value={o.fade_in ?? 0}
              oninput={(e) => updateObj<AudioObject>((obj) => { const v = +(e.target as HTMLInputElement).value; obj.fade_in = v > 0 ? v : undefined; })} />
            <span class="unit">s</span>
          </div>
          <div class="field-row">
            <label>フェードアウト</label>
            <input type="number" step="0.1" min="0" value={o.fade_out ?? 0}
              oninput={(e) => updateObj<AudioObject>((obj) => { const v = +(e.target as HTMLInputElement).value; obj.fade_out = v > 0 ? v : undefined; })} />
            <span class="unit">s</span>
          </div>
          <div class="field-row">
            <label>ループ</label>
            <select value={o.loop} onchange={(e) => updateObj<AudioObject>((obj) => { obj.loop = (e.target as HTMLSelectElement).value as 'loop'|'silence'; })}>
              <option value="loop">繰り返し</option>
              <option value="silence">無音</option>
            </select>
          </div>
        {/if}

        <!-- Z順ボタン -->
        <div class="field-row z-order">
          <label>Z順</label>
          <button onclick={() => {
            if (!selectedSceneId || !selectedObjectId) return;
            projectStore.updateProject((p) => {
              const scene = p.scenes.find(s => s.id === selectedSceneId);
              if (!scene) return;
              const idx = scene.objects.findIndex(o => o.id === selectedObjectId);
              if (idx > 0) [scene.objects[idx-1], scene.objects[idx]] = [scene.objects[idx], scene.objects[idx-1]];
            });
          }}>背面へ</button>
          <button onclick={() => {
            if (!selectedSceneId || !selectedObjectId) return;
            projectStore.updateProject((p) => {
              const scene = p.scenes.find(s => s.id === selectedSceneId);
              if (!scene) return;
              const idx = scene.objects.findIndex(o => o.id === selectedObjectId);
              if (idx < scene.objects.length - 1) [scene.objects[idx], scene.objects[idx+1]] = [scene.objects[idx+1], scene.objects[idx]];
            });
          }}>前面へ</button>
        </div>
      {/if}

    {:else}
      <!-- 出力設定タブ -->
      {#if project}
        <div class="section-title">出力設定</div>
        <div class="field-row">
          <label>出力先</label>
          <input type="text" value={project.output_folder}
            oninput={(e) => updateOutputFolder((e.target as HTMLInputElement).value)}
            placeholder="（未設定）"
            class:invalid={(project.output_folder ?? '').trim() === ''} />
          <button onclick={browseOutputFolder}>📁</button>
        </div>
        <div class="field-row">
          <label>ファイル名</label>
          <div class="name-with-ext">
            <input type="text" value={project.output.output_name}
              oninput={(e) => updateOutput((o) => { o.output_name = (e.target as HTMLInputElement).value; })}
              placeholder="output"
              class:invalid={(project.output.output_name ?? '').trim() === ''} />
            <span class="ext">.{project.output.format}</span>
          </div>
        </div>
        <div class="field-row">
          <label>幅</label>
          <input type="number" value={project.output.width}
            onfocus={() => {
              if (project.output.width > 0 && project.output.height > 0)
                _outLockedRatio = project.output.width / project.output.height;
            }}
            oninput={(e) => {
              const w = +(e.target as HTMLInputElement).value;
              if (w <= 0 || !isFinite(w)) return;
              const ratio = _outLockedRatio;
              updateOutput((o) => { o.width = w; o.height = Math.round(w / ratio); });
            }} />
          <span class="unit">px</span>
        </div>
        <div class="field-row">
          <label>高さ</label>
          <input type="number" value={project.output.height}
            onfocus={() => {
              if (project.output.width > 0 && project.output.height > 0)
                _outLockedRatio = project.output.width / project.output.height;
            }}
            oninput={(e) => {
              const h = +(e.target as HTMLInputElement).value;
              if (h <= 0 || !isFinite(h)) return;
              const ratio = _outLockedRatio;
              updateOutput((o) => { o.height = h; o.width = Math.round(h * ratio); });
            }} />
          <span class="unit">px</span>
        </div>
        <div class="field-row">
          <label>フレームレート</label>
          <input type="number" value={project.output.fps}
            oninput={(e) => updateOutput((o) => { o.fps = +(e.target as HTMLInputElement).value; })} />
          <span class="unit">fps</span>
        </div>
        <div class="field-row">
          <label>コーデック</label>
          <select value={project.output.codec}
            onchange={(e) => updateOutput((o) => { o.codec = (e.target as HTMLSelectElement).value as typeof o.codec; })}>
            <option value="h264">H.264</option>
            <option value="h265">H.265</option>
            <option value="vp9">VP9</option>
          </select>
        </div>
        <div class="field-row">
          <label>形式</label>
          <select value={project.output.format}
            onchange={(e) => updateOutput((o) => { o.format = (e.target as HTMLSelectElement).value as typeof o.format; })}>
            <option value="mp4">mp4</option>
            <option value="mov">mov</option>
            <option value="webm">webm</option>
          </select>
        </div>
        <div class="field-row">
          <label>品質 (CRF)</label>
          <input type="number" min="0" max={project.output.codec === 'vp9' ? 63 : 51}
            value={project.output.crf}
            oninput={(e) => updateOutput((o) => { o.crf = +(e.target as HTMLInputElement).value; })} />
          <input type="range" min="0" max={project.output.codec === 'vp9' ? 63 : 51}
            value={project.output.crf}
            oninput={(e) => updateOutput((o) => { o.crf = +(e.target as HTMLInputElement).value; })} />
        </div>
        <div class="field-row">
          <label>プリセット</label>
          <select value={project.output.preset} disabled={project.output.codec === 'vp9'}
            onchange={(e) => updateOutput((o) => { o.preset = (e.target as HTMLSelectElement).value; })}>
            {#each PRESETS as p}
              <option value={p}>{p}</option>
            {/each}
          </select>
        </div>
      {:else}
        <p class="hint">プロジェクトを開いてください</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .props-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-panel);
    overflow: hidden;
  }
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }
  .tabs button {
    flex: 1;
    border: none;
    border-radius: 0;
    padding: 6px;
    background: none;
    border-bottom: 2px solid transparent;
  }
  .tabs button.active {
    border-bottom-color: var(--color-primary);
    color: var(--color-primary);
  }
  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }
  .section-title {
    font-weight: 600;
    font-size: 11px;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 6px;
  }
  .field-row {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-bottom: 4px;
  }
  .field-row label {
    width: 80px;
    font-size: 12px;
    color: var(--color-text-muted);
    flex-shrink: 0;
  }
  .unit {
    font-size: 11px;
    color: var(--color-text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .field-row input[type="text"],
  .field-row input[type="number"] {
    flex: 1;
    min-width: 0;
  }
  .field-row input[type="range"] { flex: 1; }
  .z-order { margin-top: 8px; }
  input.invalid { border-color: var(--color-error); }
  .name-with-ext {
    display: flex;
    align-items: center;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }
  .name-with-ext input { flex: 1; min-width: 0; }
  .ext { font-size: 12px; color: var(--color-text-muted); white-space: nowrap; }
  .hint {
    color: var(--color-text-muted);
    font-size: 12px;
    margin-top: 12px;
  }
</style>
