<script lang="ts">
	import PreviewCanvas from '../preview/PreviewCanvas.svelte';
	import { previewStore } from '../store/previewStore.svelte';
	import { projectStore } from '../store/projectStore.svelte';

	let project = $derived(projectStore.project);

	function formatTime(secs: number): string {
		const m = Math.floor(secs / 60);
		const s = Math.floor(secs % 60);
		return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
	}

	function handleSeek(e: Event) {
		const v = parseFloat((e.target as HTMLInputElement).value);
		previewStore.seek(v);
		if (previewStore.isPlaying) {
			// Re-anchor wall clock reference so playback continues from new position
			// The $effect in PreviewCanvas handles syncing videos
		}
	}

	function handleStop() {
		previewStore.pause();
		previewStore.seek(0);
	}

	let total = $derived(previewStore.totalDuration);
	let current = $derived(previewStore.currentTime);
</script>

<div class="preview-panel">
	<div class="canvas-area">
		{#if project}
			<PreviewCanvas />
		{:else}
			<div class="no-project">プロジェクトを開いてください</div>
		{/if}
	</div>

	<div class="controls">
		<button
			onclick={() => (previewStore.isPlaying ? previewStore.pause() : previewStore.play())}
			disabled={!project}
			aria-label={previewStore.isPlaying ? '一時停止' : '再生'}
		>{previewStore.isPlaying ? '⏸' : '▶'}</button>

		<button onclick={handleStop} disabled={!project} aria-label="停止">⏹</button>

		<input
			class="seekbar"
			type="range"
			min="0"
			max={total}
			step={project ? 1 / (project.output.fps || 30) : 0.033}
			value={current}
			oninput={handleSeek}
			disabled={!project || total === 0}
		/>

		<span class="time-display">
			{total > 0 ? `${formatTime(current)} / ${formatTime(total)}` : '-- : -- / -- : --'}
		</span>
	</div>

	<div class="preview-notice">
		プレビューは参考表示です。テキスト描画・フォントメトリクスは最終出力（FFmpeg）と異なる場合があります。
	</div>
</div>

<style>
	.preview-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--bg-panel);
		overflow: hidden;
	}

	.canvas-area {
		flex: 1;
		min-height: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #000;
		overflow: hidden;
	}

	.no-project {
		color: var(--color-text-muted);
		font-size: 13px;
	}

	.controls {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 8px;
		background: var(--bg-toolbar);
		border-top: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.seekbar {
		flex: 1;
		min-width: 60px;
	}

	.time-display {
		font-size: 12px;
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
		color: var(--color-text-muted);
	}

	.preview-notice {
		font-size: 10px;
		color: var(--color-text-muted);
		padding: 2px 8px;
		background: var(--bg-toolbar);
		border-top: 1px solid var(--color-border);
		flex-shrink: 0;
	}
</style>
