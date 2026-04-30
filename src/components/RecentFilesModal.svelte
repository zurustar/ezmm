<script lang="ts">
	import { settingsStore } from '../store/settingsStore.svelte';

	interface Props {
		onOpen: (path: string) => void;
		onClose: () => void;
	}
	let { onOpen, onClose }: Props = $props();

	let recentFiles = $derived(settingsStore.settings?.recent_files ?? []);
</script>

{#if recentFiles.length > 0}
	<div
		class="modal-overlay"
		role="dialog"
		aria-modal="true"
		onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
	>
		<div class="modal-box recent-modal">
			<button class="close-btn" onclick={onClose} aria-label="閉じる">✕</button>
			<div class="modal-title">最近開いたプロジェクト</div>
			<ul class="file-list">
				{#each recentFiles as path}
					<li>
						<button class="file-item" onclick={() => onOpen(path)}>
							<span class="file-name">{path.split(/[\\/]/).pop()}</span>
							<span class="file-path">{path}</span>
						</button>
					</li>
				{/each}
			</ul>
			<div class="modal-actions">
				<button onclick={onClose}>新規プロジェクトで開始</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.recent-modal {
		min-width: 400px;
		max-width: 600px;
		position: relative;
	}

	.close-btn {
		position: absolute;
		top: 8px;
		right: 8px;
		border: none;
		background: none;
		font-size: 14px;
		cursor: pointer;
		opacity: 0.5;
	}
	.close-btn:hover { opacity: 1; }

	.file-list {
		list-style: none;
		max-height: 300px;
		overflow-y: auto;
	}

	.file-item {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		width: 100%;
		text-align: left;
		border: none;
		background: none;
		padding: 6px 4px;
		border-radius: var(--radius);
		cursor: pointer;
	}
	.file-item:hover { background: var(--bg-panel); }

	.file-name { font-weight: 600; font-size: 13px; }
	.file-path { font-size: 11px; color: var(--color-text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 100%; }
</style>
