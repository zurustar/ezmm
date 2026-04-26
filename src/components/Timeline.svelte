<script lang="ts">
	import { projectStore } from '../store/projectStore.svelte';
	import { calculateTotalDuration } from '../preview/sceneUtils';
	import type { Scene, SceneObject } from '../types/project';

	let project = $derived(projectStore.project);
	let selectedSceneId = $derived(projectStore.selectedSceneId);
	let selectedObjectId = $derived(projectStore.selectedObjectId);
	let isBusy = $state(false);

	let totalDur = $derived(project ? calculateTotalDuration(project.scenes) : 0);

	function sceneWidth(scene: Scene): string {
		if (totalDur === 0 || !scene.duration) return '80px';
		return `${Math.max(60, (scene.duration / totalDur) * 100)}%`;
	}

	function addScene() {
		projectStore.updateProject((p) => {
			p.scenes.push({
				id: `scene_${Date.now()}`,
				duration: 5,
				objects: []
			});
		});
	}

	function deleteScene(sceneId: string) {
		projectStore.updateProject((p) => {
			p.scenes = p.scenes.filter((s) => s.id !== sceneId);
		});
		if (selectedSceneId === sceneId) {
			projectStore.selectScene(null);
			projectStore.selectObject(null);
		}
	}

	function duplicateScene(sceneId: string) {
		projectStore.updateProject((p) => {
			const idx = p.scenes.findIndex((s) => s.id === sceneId);
			if (idx < 0) return;
			const src = p.scenes[idx];
			const copy: Scene = {
				id: `scene_${Date.now()}`,
				duration: src.duration,
				objects: src.objects.map((o) => ({ ...o, id: `${o.id}_copy` }))
			};
			p.scenes.splice(idx + 1, 0, copy);
		});
	}

	function addObject(sceneId: string, type: SceneObject['type']) {
		const id = `obj_${Date.now()}`;
		projectStore.updateProject((p) => {
			const scene = p.scenes.find((s) => s.id === sceneId);
			if (!scene) return;
			let obj: SceneObject;
			const base = { id, start: 0, variable: false };
			if (type === 'video') {
				obj = { ...base, type: 'video', x: 0, y: 0, width: 1920, height: 1080, opacity: 100, volume: 100 } as SceneObject;
			} else if (type === 'image') {
				obj = { ...base, type: 'image', x: 0, y: 0, width: 400, height: 300, duration: 0, opacity: 100 } as SceneObject;
			} else if (type === 'text') {
				obj = { ...base, type: 'text', text: 'テキスト', x: 0, y: 0, width: 400, height: 60, duration: 0, opacity: 100, font: 'NotoSansCJK-Regular', font_size: 24, color: '#ffffff', align: 'left' } as SceneObject;
			} else {
				obj = { ...base, type: 'audio', duration: 0, volume: 100, loop: 'loop' } as SceneObject;
			}
			scene.objects.push(obj);
		});
	}

	function deleteObject(sceneId: string, objectId: string) {
		projectStore.updateProject((p) => {
			const scene = p.scenes.find((s) => s.id === sceneId);
			if (scene) scene.objects = scene.objects.filter((o) => o.id !== objectId);
		});
		if (selectedObjectId === objectId) projectStore.selectObject(null);
	}

	const OBJ_TYPE_ICON: Record<string, string> = { video: '🎬', image: '🖼', text: '📝', audio: '♪' };
</script>

<div class="timeline">
	<div class="timeline-header">
		<span>シーンタイムライン</span>
		<button onclick={addScene} disabled={!project}>＋シーン追加</button>
	</div>

	{#if project && project.scenes.length > 0}
		<div class="scenes-row">
			{#each project.scenes as scene}
				<div
					class="scene-block"
					class:selected={selectedSceneId === scene.id}
					style="width: {sceneWidth(scene)}"
					role="button"
					tabindex="0"
					onclick={() => projectStore.selectScene(scene.id)}
					onkeydown={(e) => e.key === 'Enter' && projectStore.selectScene(scene.id)}
				>
					<div class="scene-header">
						<span class="scene-id">{scene.id}</span>
						<span class="scene-dur">{scene.duration ?? '?'}s</span>
						<button class="icon-btn" onclick={(e) => { e.stopPropagation(); duplicateScene(scene.id); }} title="複製">⧉</button>
						<button class="icon-btn" onclick={(e) => { e.stopPropagation(); deleteScene(scene.id); }} title="削除">✕</button>
					</div>

					<div class="objects-list">
						{#each scene.objects as obj}
							<div
								class="obj-row"
								class:selected={selectedObjectId === obj.id}
								role="button"
								tabindex="0"
								onclick={(e) => {
									e.stopPropagation();
									projectStore.selectScene(scene.id);
									projectStore.selectObject(obj.id);
								}}
								onkeydown={(e) => e.key === 'Enter' && projectStore.selectObject(obj.id)}
							>
								<span>{OBJ_TYPE_ICON[obj.type] ?? '?'}</span>
								<span class="obj-id">{obj.id}{obj.variable ? ' ★' : ''}</span>
								<button class="icon-btn del" onclick={(ev) => { ev.stopPropagation(); deleteObject(scene.id, obj.id); }} title="削除">✕</button>
							</div>
						{/each}

						<div class="add-obj-row">
							{#each (['video', 'image', 'text', 'audio'] as SceneObject['type'][]) as t}
								<button class="add-obj-btn" onclick={(e) => { e.stopPropagation(); addObject(scene.id, t); }} title={t}>
									{OBJ_TYPE_ICON[t]}
								</button>
							{/each}
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else if project}
		<div class="empty-hint">シーンを追加してください</div>
	{:else}
		<div class="empty-hint">プロジェクトを開いてください</div>
	{/if}
</div>

<style>
	.timeline {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--bg-panel);
		border-top: 1px solid var(--color-border);
		overflow: hidden;
	}

	.timeline-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 4px 8px;
		background: var(--bg-toolbar);
		border-bottom: 1px solid var(--color-border);
		font-size: 12px;
		font-weight: 600;
		flex-shrink: 0;
	}

	.scenes-row {
		display: flex;
		gap: 2px;
		padding: 4px;
		overflow-x: auto;
		overflow-y: hidden;
		flex: 1;
		align-items: flex-start;
	}

	.scene-block {
		border: 1px solid var(--color-border);
		border-radius: var(--radius);
		background: var(--bg-main);
		flex-shrink: 0;
		cursor: pointer;
		min-width: 60px;
		overflow: hidden;
	}

	.scene-block.selected {
		border-color: var(--color-primary);
	}

	.scene-header {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 2px 4px;
		background: var(--bg-toolbar);
		border-bottom: 1px solid var(--color-border);
		font-size: 11px;
	}

	.scene-id { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.scene-dur { color: var(--color-text-muted); font-size: 10px; }

	.objects-list { padding: 2px 4px; }

	.obj-row {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 1px 2px;
		border-radius: 2px;
		font-size: 11px;
		cursor: pointer;
	}

	.obj-row:hover { background: var(--bg-panel); }
	.obj-row.selected { background: color-mix(in srgb, var(--color-primary) 15%, transparent); }
	.obj-id { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

	.add-obj-row {
		display: flex;
		gap: 2px;
		margin-top: 2px;
		padding-top: 2px;
		border-top: 1px dashed var(--color-border);
	}

	.add-obj-btn {
		border: none;
		background: none;
		font-size: 13px;
		padding: 1px;
		opacity: 0.5;
	}
	.add-obj-btn:hover { opacity: 1; }

	.icon-btn {
		border: none;
		background: none;
		font-size: 10px;
		padding: 1px 2px;
		opacity: 0.5;
	}
	.icon-btn:hover { opacity: 1; }
	.icon-btn.del { color: var(--color-error); }

	.empty-hint {
		color: var(--color-text-muted);
		font-size: 12px;
		padding: 12px;
	}
</style>
