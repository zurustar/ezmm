<script lang="ts">
	interface Props {
		onSave: () => void;
		onDiscard: () => void;
		onCancel: () => void;
	}
	let { onSave, onDiscard, onCancel }: Props = $props();
</script>

<div class="modal-overlay" role="dialog" aria-modal="true">
	<div class="modal-box">
		<div class="modal-title">未保存の変更があります</div>
		<p>変更が保存されていません。保存しますか？</p>
		<div class="modal-actions">
			<button onclick={onCancel}>キャンセル</button>
			<button onclick={onDiscard}>保存しないで続ける</button>
			<button class="btn-primary" onclick={onSave}>保存して続ける</button>
		</div>
	</div>
</div>

<style>
	p { font-size: 13px; margin-bottom: 4px; }
</style>
