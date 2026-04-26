<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { batchStore } from '../store/batchStore.svelte';

	interface Props {
		onClose: () => void;
	}
	let { onClose }: Props = $props();

	let confirmCancel = $state(false);

	function handleCancel() {
		if (batchStore.status === 'cancelling') return;
		confirmCancel = true;
	}

	async function confirmCancelBatch() {
		confirmCancel = false;
		await batchStore.cancelBatch();
	}

	async function openOutputFolder() {
		// Open the output folder in Finder/Explorer via shell
		// (Would use tauri-plugin-shell in production)
		onClose();
	}

	let entryPct = $derived(
		batchStore.totalEntries > 0
			? Math.round((batchStore.currentEntryIndex / batchStore.totalEntries) * 100)
			: 0
	);
	let innerPct = $derived(Math.round((batchStore.currentEntryProgress ?? 0) * 100));
	let isDone = $derived(batchStore.status === 'done');
	let isCancelling = $derived(batchStore.status === 'cancelling');
</script>

<div class="modal-overlay" role="dialog" aria-modal="true">
	<div class="modal-box progress-dialog">
		<div class="modal-title">
			{#if isDone}バッチ完了
			{:else if isCancelling}キャンセル中...
			{:else}バッチ実行中{/if}
		</div>

		{#if !isDone}
			<div class="progress-info">
				{batchStore.currentEntryName ?? '準備中...'}
				（{batchStore.currentEntryIndex + 1} / {batchStore.totalEntries}）
			</div>
			<div class="progress-bar-wrap">
				<div class="progress-bar" style="width:{entryPct}%"></div>
			</div>
			{#if innerPct > 0}
				<div class="progress-bar-wrap inner">
					<div class="progress-bar" style="width:{innerPct}%"></div>
				</div>
				<div class="pct-text">{innerPct}%</div>
			{/if}
		{:else}
			<p>処理が完了しました。</p>
			{#if batchStore.errors.length > 0}
				<p class="error-count">エラー: {batchStore.errors.length} 件</p>
				<ul class="error-list">
					{#each batchStore.errors as err}
						<li>{err.entry_name}: {err.message}</li>
					{/each}
				</ul>
			{/if}
		{/if}

		{#if confirmCancel}
			<p class="confirm-msg">バッチ実行をキャンセルしますか？</p>
			<div class="modal-actions">
				<button onclick={() => (confirmCancel = false)}>続ける</button>
				<button class="btn-primary" onclick={confirmCancelBatch}>キャンセルする</button>
			</div>
		{:else}
			<div class="modal-actions">
				{#if isDone}
					<button class="btn-primary" onclick={onClose}>閉じる</button>
				{:else}
					<button onclick={handleCancel} disabled={isCancelling}>キャンセル</button>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.progress-dialog {
		min-width: 380px;
	}

	.progress-info {
		font-size: 13px;
		margin-bottom: 8px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.progress-bar-wrap {
		height: 12px;
		background: var(--color-border);
		border-radius: 6px;
		overflow: hidden;
		margin-bottom: 4px;
	}

	.progress-bar-wrap.inner {
		height: 6px;
	}

	.progress-bar {
		height: 100%;
		background: var(--color-primary);
		transition: width 0.3s;
	}

	.pct-text {
		font-size: 11px;
		color: var(--color-text-muted);
		text-align: right;
		margin-bottom: 4px;
	}

	.error-count { color: var(--color-error); margin-top: 8px; }

	.error-list {
		max-height: 120px;
		overflow-y: auto;
		font-size: 12px;
		color: var(--color-error);
		padding-left: 16px;
		margin-top: 4px;
	}

	.confirm-msg {
		margin-top: 8px;
		font-size: 13px;
	}
</style>
