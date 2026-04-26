<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

	interface Props { onClose: () => void; }
	let { onClose }: Props = $props();

	let ffmpegVersion = $state('取得中...');

	onMount(async () => {
		try {
			ffmpegVersion = await invoke<string>('get_ffmpeg_version');
		} catch {
			ffmpegVersion = '不明';
		}
	});
</script>

<div
	class="modal-overlay"
	role="dialog"
	aria-modal="true"
	onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
>
	<div class="modal-box about-dialog">
		<div class="modal-title">ezmm について</div>
		<p><strong>ezmm</strong> v0.1.0</p>
		<p>社内向け動画バッチ編集ツール</p>
		<p style="margin-top:8px">ライセンス: GPL-3.0</p>
		<p>FFmpeg: {ffmpegVersion}</p>
		<div class="modal-actions">
			<button class="btn-primary" onclick={onClose}>閉じる</button>
		</div>
	</div>
</div>

<style>
	.about-dialog { min-width: 280px; }
	p { margin-bottom: 4px; font-size: 13px; }
</style>
