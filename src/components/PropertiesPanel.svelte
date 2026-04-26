<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { projectStore } from '../store/projectStore.svelte';
	import { settingsStore } from '../store/settingsStore.svelte';
	import type {
		SceneObject, VideoObject, ImageObject, TextObject, AudioObject,
		VariableValue, VideoVariable
	} from '../types/project';

	type Tab = 'object' | 'output';
	let activeTab = $state<Tab>('object');

	let project = $derived(projectStore.project);
	let selectedSceneId = $derived(projectStore.selectedSceneId);
	let selectedObjectId = $derived(projectStore.selectedObjectId);
	let selectedEntryName = $derived(projectStore.selectedEntryName);
	let isBusy = $state(false);

	let selectedScene = $derived(project?.scenes.find((s) => s.id === selectedSceneId) ?? null);
	let selectedObject = $derived(
		selectedScene?.objects.find((o) => o.id === selectedObjectId) ?? null
	);
	let selectedEntry = $derived(
		project?.entries.find((e) => e.name === selectedEntryName) ?? null
	);

	// Variable objects in the selected scene (for the entry variable editing)
	let variableObjects = $derived(
		selectedScene?.objects.filter((o) => o.variable) ?? []
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

	function updateOutput(updater: (o: typeof project.output) => void) {
		if (!project) return;
		projectStore.updateProject((p) => updater(p.output));
	}

	function fileKindForObject(id: string): string {
		const scene = project?.scenes.find((s) => s.id === selectedSceneId);
		const obj = scene?.objects.find((o) => o.id === id);
		if (!obj) return 'any';
		if (obj.type === 'video') return 'video';
		if (obj.type === 'image') return 'image';
		if (obj.type === 'audio') return 'audio';
		return 'any';
	}

	async function browseFile(objectId: string, forEntry = false) {
		const kind = fileKindForObject(objectId);
		const path = await invoke<string | null>('show_open_file_dialog', { kind });
		if (!path) return;
		if (forEntry && selectedEntryName) {
			projectStore.updateProject((p) => {
				const entry = p.entries.find((e) => e.name === selectedEntryName);
				if (!entry) return;
				const v = entry.variables[objectId] as VideoVariable | undefined;
				if (v && 'file' in v) {
					(entry.variables[objectId] as { file: string }).file = path;
				} else {
					entry.variables[objectId] = { file: path };
				}
			});
		} else {
			projectStore.updateProject((p) => {
				const scene = p.scenes.find((s) => s.id === selectedSceneId);
				if (!scene) return;
				const obj = scene.objects.find((o) => o.id === objectId);
				if (obj && 'file' in obj) (obj as { file?: string }).file = path;
			});
		}
	}

	const PRESETS = ['ultrafast','superfast','veryfast','faster','fast','medium','slow','slower','veryslow'];
</script>

<div class="props-panel">
	<div class="tabs">
		<button class:active={activeTab === 'object'} onclick={() => (activeTab = 'object')}>
			オブジェクト
		</button>
		<button class:active={activeTab === 'output'} onclick={() => (activeTab = 'output')}>
			出力設定
		</button>
	</div>

	<div class="panel-body">
		{#if activeTab === 'object'}
			{#if !selectedObject && !selectedEntry}
				<p class="hint">オブジェクトまたはエントリを選択してください</p>

			{:else if selectedObject}
				<div class="section-title">オブジェクト ({selectedObject.type})</div>

				<!-- Common fields -->
				<div class="field-row">
					<label>start</label>
					<input type="number" step="0.01" value={selectedObject.start}
						oninput={(e) => updateObj((o) => { o.start = parseFloat((e.target as HTMLInputElement).value) || 0; })} />
				</div>

				{#if selectedObject.type !== 'audio'}
					{@const o = selectedObject as VideoObject | ImageObject | TextObject}
					<div class="field-row">
						<label>x</label>
						<input type="number" value={o.x} oninput={(e) => updateObj<typeof o>((obj) => { obj.x = +e.target.value; })} />
					</div>
					<div class="field-row">
						<label>y</label>
						<input type="number" value={o.y} oninput={(e) => updateObj<typeof o>((obj) => { obj.y = +e.target.value; })} />
					</div>
					<div class="field-row">
						<label>width</label>
						<input type="number" value={o.width} oninput={(e) => updateObj<typeof o>((obj) => { obj.width = +e.target.value; })} />
					</div>
					<div class="field-row">
						<label>height</label>
						<input type="number" value={o.height} oninput={(e) => updateObj<typeof o>((obj) => { obj.height = +e.target.value; })} />
					</div>
					<div class="field-row">
						<label>opacity</label>
						<input type="number" min="0" max="100" value={o.opacity} oninput={(e) => updateObj<typeof o>((obj) => { obj.opacity = +e.target.value; })} />
					</div>
				{/if}

				{#if selectedObject.type === 'video'}
					{@const o = selectedObject as VideoObject}
					{#if !o.variable}
						<div class="field-row">
							<label>file</label>
							<input type="text" value={o.file ?? ''} oninput={(e) => updateObj<VideoObject>((obj) => { obj.file = e.target.value; })} />
							<button onclick={() => browseFile(o.id)}>...</button>
						</div>
					{/if}
					<div class="field-row">
						<label>volume</label>
						<input type="number" min="0" max="100" value={o.volume} oninput={(e) => updateObj<VideoObject>((obj) => { obj.volume = +e.target.value; })} />
					</div>

				{:else if selectedObject.type === 'image'}
					{@const o = selectedObject as ImageObject}
					{#if !o.variable}
						<div class="field-row">
							<label>file</label>
							<input type="text" value={o.file ?? ''} oninput={(e) => updateObj<ImageObject>((obj) => { obj.file = e.target.value; })} />
							<button onclick={() => browseFile(o.id)}>...</button>
						</div>
					{/if}
					<div class="field-row">
						<label>duration</label>
						<input type="number" step="0.01" value={o.duration} oninput={(e) => updateObj<ImageObject>((obj) => { obj.duration = +e.target.value; })} />
					</div>

				{:else if selectedObject.type === 'text'}
					{@const o = selectedObject as TextObject}
					{#if !o.variable}
						<div class="field-row">
							<label>text</label>
							<input type="text" value={o.text ?? ''} oninput={(e) => updateObj<TextObject>((obj) => { obj.text = e.target.value; })} />
						</div>
					{/if}
					<div class="field-row">
						<label>font_size</label>
						<input type="number" min="1" value={o.font_size} oninput={(e) => updateObj<TextObject>((obj) => { obj.font_size = +e.target.value; })} />
					</div>
					<div class="field-row">
						<label>color</label>
						<input type="color" value={o.color} oninput={(e) => updateObj<TextObject>((obj) => { obj.color = e.target.value; })} />
					</div>
					<div class="field-row">
						<label>align</label>
						<select value={o.align ?? 'left'} onchange={(e) => updateObj<TextObject>((obj) => { obj.align = e.target.value as 'left'|'center'|'right'; })}>
							<option value="left">left</option>
							<option value="center">center</option>
							<option value="right">right</option>
						</select>
					</div>
					<div class="field-row">
						<label>duration</label>
						<input type="number" step="0.01" value={o.duration} oninput={(e) => updateObj<TextObject>((obj) => { obj.duration = +e.target.value; })} />
					</div>

				{:else if selectedObject.type === 'audio'}
					{@const o = selectedObject as AudioObject}
					{#if !o.variable}
						<div class="field-row">
							<label>file</label>
							<input type="text" value={o.file ?? ''} oninput={(e) => updateObj<AudioObject>((obj) => { obj.file = e.target.value; })} />
							<button onclick={() => browseFile(o.id)}>...</button>
						</div>
					{/if}
					<div class="field-row">
						<label>duration</label>
						<input type="number" step="0.01" value={o.duration} oninput={(e) => updateObj<AudioObject>((obj) => { obj.duration = +e.target.value; })} />
					</div>
					<div class="field-row">
						<label>volume</label>
						<input type="number" min="0" max="100" value={o.volume} oninput={(e) => updateObj<AudioObject>((obj) => { obj.volume = +e.target.value; })} />
					</div>
					<div class="field-row">
						<label>loop</label>
						<select value={o.loop} onchange={(e) => updateObj<AudioObject>((obj) => { obj.loop = e.target.value as 'loop'|'silence'; })}>
							<option value="loop">loop</option>
							<option value="silence">silence</option>
						</select>
					</div>
				{/if}

				<!-- Z-order buttons -->
				<div class="field-row z-order">
					<label>Z順</label>
					<button onclick={() => {
						if (!selectedSceneId || !selectedObjectId) return;
						projectStore.updateProject((p) => {
							const scene = p.scenes.find(s => s.id === selectedSceneId);
							if (!scene) return;
							const idx = scene.objects.findIndex(o => o.id === selectedObjectId);
							if (idx > 0) {
								[scene.objects[idx-1], scene.objects[idx]] = [scene.objects[idx], scene.objects[idx-1]];
							}
						});
					}}>背面へ</button>
					<button onclick={() => {
						if (!selectedSceneId || !selectedObjectId) return;
						projectStore.updateProject((p) => {
							const scene = p.scenes.find(s => s.id === selectedSceneId);
							if (!scene) return;
							const idx = scene.objects.findIndex(o => o.id === selectedObjectId);
							if (idx < scene.objects.length - 1) {
								[scene.objects[idx], scene.objects[idx+1]] = [scene.objects[idx+1], scene.objects[idx]];
							}
						});
					}}>前面へ</button>
				</div>
			{/if}

			<!-- Variable values for selected entry -->
			{#if selectedEntry && variableObjects.length > 0}
				<div class="section-title" style="margin-top:12px">可変値（{selectedEntry.name}）</div>
				{#each variableObjects as obj}
					<div class="field-row">
						<label>{obj.id}</label>
						{#if obj.type === 'text'}
							<input type="text"
								value={(selectedEntry.variables[obj.id] as {text:string})?.text ?? ''}
								oninput={(e) => {
									const v = e.target.value;
									projectStore.updateProject((p) => {
										const entry = p.entries.find(en => en.name === selectedEntryName);
										if (entry) entry.variables[obj.id] = { text: v };
									});
								}} />
						{:else}
							<input type="text"
								value={(selectedEntry.variables[obj.id] as {file:string})?.file ?? ''}
								oninput={(e) => {
									const v = e.target.value;
									projectStore.updateProject((p) => {
										const entry = p.entries.find(en => en.name === selectedEntryName);
										if (entry) entry.variables[obj.id] = { file: v };
									});
								}} />
							<button onclick={() => browseFile(obj.id, true)}>...</button>
						{/if}
					</div>
					{#if obj.type === 'video'}
						{@const vv = selectedEntry.variables[obj.id] as VideoVariable | undefined}
						<div class="field-row indent">
							<label>trim_start</label>
							<input type="number" step="0.01" value={vv?.trim_start ?? 0}
								oninput={(e) => {
									const v = parseFloat(e.target.value) || 0;
									projectStore.updateProject((p) => {
										const entry = p.entries.find(en => en.name === selectedEntryName);
										if (!entry) return;
										const cur = entry.variables[obj.id] as VideoVariable ?? { file: '' };
										entry.variables[obj.id] = { ...cur, trim_start: v };
									});
								}} />
						</div>
						<div class="field-row indent">
							<label>trim_end</label>
							<input type="number" step="0.01" value={vv?.trim_end ?? 0}
								oninput={(e) => {
									const v = parseFloat(e.target.value) || 0;
									projectStore.updateProject((p) => {
										const entry = p.entries.find(en => en.name === selectedEntryName);
										if (!entry) return;
										const cur = entry.variables[obj.id] as VideoVariable ?? { file: '' };
										entry.variables[obj.id] = { ...cur, trim_end: v };
									});
								}} />
						</div>
					{/if}
				{/each}
			{/if}

		{:else}
			<!-- Output settings tab -->
			{#if project}
				<div class="section-title">出力設定</div>
				<div class="field-row">
					<label>width</label>
					<input type="number" value={project.output.width}
						oninput={(e) => updateOutput((o) => { o.width = +e.target.value; })} />
				</div>
				<div class="field-row">
					<label>height</label>
					<input type="number" value={project.output.height}
						oninput={(e) => updateOutput((o) => { o.height = +e.target.value; })} />
				</div>
				<div class="field-row">
					<label>fps</label>
					<input type="number" value={project.output.fps}
						oninput={(e) => updateOutput((o) => { o.fps = +e.target.value; })} />
				</div>
				<div class="field-row">
					<label>codec</label>
					<select value={project.output.codec}
						onchange={(e) => updateOutput((o) => { o.codec = e.target.value as typeof o.codec; })}>
						<option value="h264">h264</option>
						<option value="h265">h265</option>
						<option value="vp9">vp9</option>
					</select>
				</div>
				<div class="field-row">
					<label>format</label>
					<select value={project.output.format}
						onchange={(e) => updateOutput((o) => { o.format = e.target.value as typeof o.format; })}>
						<option value="mp4">mp4</option>
						<option value="mov">mov</option>
						<option value="webm">webm</option>
					</select>
				</div>
				<div class="field-row">
					<label>crf</label>
					<input type="number" min="0" max={project.output.codec === 'vp9' ? 63 : 51}
						value={project.output.crf}
						oninput={(e) => updateOutput((o) => { o.crf = +e.target.value; })} />
					<input type="range" min="0" max={project.output.codec === 'vp9' ? 63 : 51}
						value={project.output.crf}
						oninput={(e) => updateOutput((o) => { o.crf = +e.target.value; })} />
				</div>
				<div class="field-row">
					<label>preset</label>
					<select value={project.output.preset} disabled={project.output.codec === 'vp9'}
						onchange={(e) => updateOutput((o) => { o.preset = e.target.value; })}>
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
		width: 72px;
		font-size: 12px;
		color: var(--color-text-muted);
		flex-shrink: 0;
	}

	.field-row input[type="text"],
	.field-row input[type="number"] {
		flex: 1;
		min-width: 0;
	}

	.field-row input[type="range"] {
		flex: 1;
	}

	.field-row.indent label {
		padding-left: 12px;
	}

	.z-order {
		margin-top: 8px;
	}

	.hint {
		color: var(--color-text-muted);
		font-size: 12px;
		margin-top: 12px;
	}
</style>
