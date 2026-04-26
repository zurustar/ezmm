<script lang="ts">
	import { projectStore } from '../store/projectStore.svelte';
	import type { Entry } from '../types/project';

	let project = $derived(projectStore.project);
	let selectedEntryName = $derived(projectStore.selectedEntryName);
	let checkedEntryNames = $derived(projectStore.checkedEntryNames);

	function selectEntry(name: string) {
		projectStore.selectEntry(name);
	}

	function toggleCheck(name: string) {
		const next = new Set(checkedEntryNames);
		if (next.has(name)) next.delete(name);
		else next.add(name);
		projectStore.setCheckedEntries(next);
	}

	function addEntry() {
		const name = `entry_${Date.now()}`;
		projectStore.updateProject((p) => {
			p.entries.push({ name, variables: {} });
		});
	}

	function duplicateEntry(name: string) {
		projectStore.updateProject((p) => {
			const idx = p.entries.findIndex((e) => e.name === name);
			if (idx < 0) return;
			const src = p.entries[idx];
			const newName = `${src.name}_copy`;
			p.entries.splice(idx + 1, 0, { name: newName, variables: { ...src.variables } });
		});
	}

	function deleteEntry(name: string) {
		projectStore.updateProject((p) => {
			p.entries = p.entries.filter((e) => e.name !== name);
		});
		if (selectedEntryName === name) projectStore.selectEntry(null);
		const next = new Set(checkedEntryNames);
		next.delete(name);
		projectStore.setCheckedEntries(next);
	}

	// Rename entry name inline
	function renameEntry(oldName: string, newName: string) {
		if (!newName || newName === oldName) return;
		if (project?.entries.some((e) => e.name === newName)) {
			alert(`エントリ名 "${newName}" は既に使用されています`);
			return;
		}
		projectStore.updateProject((p) => {
			const entry = p.entries.find((e) => e.name === oldName);
			if (entry) entry.name = newName;
		});
		if (selectedEntryName === oldName) projectStore.selectEntry(newName);
		const next = new Set<string>();
		checkedEntryNames.forEach((n) => next.add(n === oldName ? newName : n));
		projectStore.setCheckedEntries(next);
	}

	// Drag-and-drop
	let dragIdx = $state<number | null>(null);

	function onDragStart(idx: number) { dragIdx = idx; }

	function onDrop(targetIdx: number) {
		if (dragIdx === null || dragIdx === targetIdx) return;
		projectStore.updateProject((p) => {
			const [moved] = p.entries.splice(dragIdx!, 1);
			p.entries.splice(targetIdx, 0, moved);
		});
		dragIdx = null;
	}
</script>

<div class="entry-list">
	<div class="entry-header">
		<span>エントリ一覧</span>
		<button onclick={addEntry} disabled={!project}>＋エントリ追加</button>
	</div>

	{#if project && project.entries.length > 0}
		<div class="entries">
			{#each project.entries as entry, idx}
				<div
					class="entry-card"
					class:selected={selectedEntryName === entry.name}
					draggable="true"
					ondragstart={() => onDragStart(idx)}
					ondragover={(e) => e.preventDefault()}
					ondrop={() => onDrop(idx)}
					role="row"
				>
					<span class="drag-handle" aria-hidden="true">⋮⋮</span>

					<input
						type="checkbox"
						checked={checkedEntryNames.has(entry.name)}
						onchange={() => toggleCheck(entry.name)}
						onclick={(e) => e.stopPropagation()}
						aria-label="バッチ対象"
					/>

					<span
						class="entry-name"
						role="button"
						tabindex="0"
						onclick={() => selectEntry(entry.name)}
						onkeydown={(e) => e.key === 'Enter' && selectEntry(entry.name)}
					>
						{entry.name}
					</span>

					<button
						class="icon-btn"
						onclick={() => duplicateEntry(entry.name)}
						title="複製"
					>⧉</button>
					<button
						class="icon-btn del"
						onclick={() => deleteEntry(entry.name)}
						title="削除"
					>✕</button>
				</div>
			{/each}
		</div>
	{:else if project}
		<div class="empty-hint">エントリを追加してください</div>
	{:else}
		<div class="empty-hint">プロジェクトを開いてください</div>
	{/if}
</div>

<style>
	.entry-list {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--bg-panel);
		border-top: 1px solid var(--color-border);
		overflow: hidden;
	}

	.entry-header {
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

	.entries {
		overflow-y: auto;
		flex: 1;
		padding: 2px 4px;
	}

	.entry-card {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px;
		border-radius: var(--radius);
		border: 1px solid transparent;
	}

	.entry-card:hover { background: var(--bg-main); }
	.entry-card.selected { border-color: var(--color-primary); background: color-mix(in srgb, var(--color-primary) 10%, transparent); }

	.drag-handle {
		cursor: grab;
		color: var(--color-handle);
		font-size: 12px;
		width: 16px;
		flex-shrink: 0;
	}

	.entry-name {
		flex: 1;
		cursor: pointer;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 13px;
	}

	.icon-btn {
		border: none;
		background: none;
		font-size: 12px;
		padding: 2px;
		opacity: 0.4;
		cursor: pointer;
	}
	.icon-btn:hover { opacity: 1; }
	.icon-btn.del { color: var(--color-error); }

	.empty-hint {
		color: var(--color-text-muted);
		font-size: 12px;
		padding: 12px;
	}
</style>
