<script lang="ts">
	import { onMount } from 'svelte';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { projectStore } from '../store/projectStore.svelte';
	import { previewStore } from '../store/previewStore.svelte';
	import { settingsStore } from '../store/settingsStore.svelte';
	import { calculateTotalDuration, getCurrentScene, isObjectVisible, ptToPx } from './sceneUtils';
	import type { VideoObject, ImageObject, TextObject, AudioObject, Entry, Scene } from '../types/project';

	// Canvas element
	let canvas = $state<HTMLCanvasElement | undefined>(undefined);

	// Derived
	let project = $derived(projectStore.project);
	let selectedEntryName = $derived(projectStore.selectedEntryName);
	let scale = $derived(settingsStore.settings?.preview_resolution_scale ?? 0.5);
	let canvasWidth = $derived(project ? Math.round(project.output.width * scale) : 960);
	let canvasHeight = $derived(project ? Math.round(project.output.height * scale) : 540);

	// RAF
	let rafId: number | null = null;
	const FRAME_INTERVAL = 1000 / 30;
	let playStartWallMs = 0;
	let playStartPreviewTime = 0;
	let lastRenderMs = 0;

	// Media (keyed by object id)
	const videoEls = new Map<string, HTMLVideoElement>();
	const imageEls = new Map<string, HTMLImageElement>();

	// Audio
	let audioCtx: AudioContext | null = null;
	type AudioNodeEntry = { source: AudioBufferSourceNode; gain: GainNode };
	const audioNodes = new Map<string, AudioNodeEntry>();
	const audioBuffers = new Map<string, AudioBuffer>();

	// Scene tracking
	let activeSceneIdx = $state(-1);
	let activeEntryName = $state<string | null>(null);

	onMount(() => {
		return () => {
			stopLoop();
			stopAllAudio();
			videoEls.forEach((v) => v.pause());
		};
	});

	// Start/stop loop on isPlaying change
	$effect(() => {
		if (previewStore.isPlaying) {
			playStartWallMs = performance.now();
			playStartPreviewTime = previewStore.currentTime;
			lastRenderMs = 0;
			startLoop();
			// Resume playing videos in current scene
			videoEls.forEach((v) => v.play().catch(() => {}));
		} else {
			stopLoop();
			videoEls.forEach((v) => v.pause());
		}
	});

	// Entry switch
	$effect(() => {
		const entryName = selectedEntryName;
		if (entryName === activeEntryName) return;
		activeEntryName = entryName;
		previewStore.pause();
		previewStore.seek(0);
		activeSceneIdx = -1;
		stopAllAudio();
		videoEls.forEach((v) => {
			v.pause();
			v.src = '';
		});
		videoEls.clear();
		imageEls.clear();
		if (project) {
			previewStore.setTotalDuration(calculateTotalDuration(project.scenes));
		}
	});

	function startLoop() {
		if (rafId !== null) return;
		rafId = requestAnimationFrame(loop);
	}

	function stopLoop() {
		if (rafId !== null) {
			cancelAnimationFrame(rafId);
			rafId = null;
		}
	}

	function loop(now: DOMHighResTimeStamp) {
		rafId = requestAnimationFrame(loop);

		// 30fps throttle
		const elapsed = now - lastRenderMs;
		if (lastRenderMs !== 0 && elapsed < FRAME_INTERVAL) return;
		lastRenderMs = now;

		if (!project || !canvas) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		// Advance preview time using wall clock (avoids drift from frame-skipping)
		if (previewStore.isPlaying) {
			const wallElapsed = (now - playStartWallMs) / 1000;
			const newTime = playStartPreviewTime + wallElapsed;
			const total = previewStore.totalDuration;
			if (total > 0 && newTime >= total) {
				previewStore.seek(total);
				previewStore.pause();
			} else {
				previewStore.seek(newTime);
			}
		}

		renderFrame(ctx);
	}

	function renderFrame(ctx: CanvasRenderingContext2D) {
		if (!project) return;
		const { sceneIndex, relativeTime } = getCurrentScene(previewStore.currentTime, project.scenes);
		const scene = project.scenes[sceneIndex];
		if (!scene) return;

		// Scene transition
		if (sceneIndex !== activeSceneIdx) {
			const entry = project.entries.find((e) => e.name === selectedEntryName) ?? null;
			handleSceneTransition(scene, entry, relativeTime);
			activeSceneIdx = sceneIndex;
		}

		const sceneLen = scene.duration ?? 0;

		// Clear to black
		ctx.globalAlpha = 1;
		ctx.globalCompositeOperation = 'source-over';
		ctx.fillStyle = '#000000';
		ctx.fillRect(0, 0, canvasWidth, canvasHeight);

		// Draw objects in YAML order (= Z-order)
		for (const obj of scene.objects) {
			const vis = obj as unknown as { start: number; duration: number };
			if (!isObjectVisible(vis, relativeTime, sceneLen)) continue;
			ctx.globalCompositeOperation = 'source-over';
			if (obj.type === 'video') drawVideo(ctx, obj);
			else if (obj.type === 'image') drawImage(ctx, obj);
			else if (obj.type === 'text') drawText(ctx, obj);
		}
	}

	function drawVideo(ctx: CanvasRenderingContext2D, obj: VideoObject) {
		const el = videoEls.get(obj.id);
		if (!el || el.readyState < 2) {
			drawPlaceholder(ctx, obj.x, obj.y, obj.width, obj.height);
			return;
		}
		ctx.globalAlpha = (obj.opacity ?? 100) / 100;
		ctx.drawImage(el, obj.x * scale, obj.y * scale, obj.width * scale, obj.height * scale);
	}

	function drawImage(ctx: CanvasRenderingContext2D, obj: ImageObject) {
		const el = imageEls.get(obj.id);
		if (!el || !el.complete || el.naturalWidth === 0) {
			drawPlaceholder(ctx, obj.x, obj.y, obj.width, obj.height);
			return;
		}
		ctx.globalAlpha = (obj.opacity ?? 100) / 100;
		ctx.drawImage(el, obj.x * scale, obj.y * scale, obj.width * scale, obj.height * scale);
	}

	function drawText(ctx: CanvasRenderingContext2D, obj: TextObject) {
		const pxSize = Math.round(ptToPx(obj.font_size) * scale);
		ctx.globalAlpha = (obj.opacity ?? 100) / 100;
		const x = obj.x * scale;
		const y = obj.y * scale;
		const w = obj.width * scale;
		const h = obj.height * scale;

		if (obj.background_color) {
			ctx.fillStyle = obj.background_color;
			ctx.fillRect(x, y, w, h);
		}

		ctx.font = `${pxSize}px "${obj.font}"`;
		ctx.fillStyle = obj.color;
		ctx.textBaseline = 'top';

		const text = resolveText(obj);
		switch (obj.align ?? 'left') {
			case 'center':
				ctx.textAlign = 'center';
				ctx.fillText(text, x + w / 2, y);
				break;
			case 'right':
				ctx.textAlign = 'right';
				ctx.fillText(text, x + w, y);
				break;
			default:
				ctx.textAlign = 'left';
				ctx.fillText(text, x, y);
		}
	}

	function drawPlaceholder(
		ctx: CanvasRenderingContext2D,
		x: number,
		y: number,
		w: number,
		h: number
	) {
		ctx.globalAlpha = 1;
		ctx.fillStyle = '#333333';
		ctx.fillRect(x * scale, y * scale, w * scale, h * scale);
	}

	function handleSceneTransition(scene: Scene, entry: Entry | null, startRelativeTime: number) {
		stopAllAudio();

		for (const obj of scene.objects) {
			if (obj.type === 'video') {
				ensureVideoEl(obj, entry);
			} else if (obj.type === 'image') {
				ensureImageEl(obj, entry);
			} else if (obj.type === 'audio') {
				if (previewStore.isPlaying && audioCtx && audioCtx.state !== 'suspended') {
					scheduleAudio(obj, entry, startRelativeTime);
				}
			}
		}
	}

	function ensureVideoEl(obj: VideoObject, entry: Entry | null) {
		if (videoEls.has(obj.id)) return;
		const file = resolveFile(obj, entry);
		const el = document.createElement('video');
		el.muted = true; // audio handled via Web Audio API
		el.preload = 'auto';
		if (file) el.src = convertFileSrc(file);
		el.load();
		videoEls.set(obj.id, el);

		// Use requestVideoFrameCallback when available for smooth rendering
		if ('requestVideoFrameCallback' in el) {
			const scheduleRVFC = () => {
				(el as HTMLVideoElement & { requestVideoFrameCallback: (cb: () => void) => void }).requestVideoFrameCallback(scheduleRVFC);
			};
			scheduleRVFC();
		}

		if (previewStore.isPlaying) el.play().catch(() => {});
	}

	function ensureImageEl(obj: ImageObject, entry: Entry | null) {
		if (imageEls.has(obj.id)) return;
		const file = resolveFile(obj, entry);
		const el = new Image();
		if (file) el.src = convertFileSrc(file);
		imageEls.set(obj.id, el);
	}

	async function scheduleAudio(obj: AudioObject, entry: Entry | null, sceneRelativeTime: number) {
		if (!audioCtx) return;
		const file = resolveFile(obj, entry);
		if (!file) return;

		let buffer = audioBuffers.get(file);
		if (!buffer) {
			try {
				const res = await fetch(convertFileSrc(file));
				const arrBuf = await res.arrayBuffer();
				buffer = await audioCtx!.decodeAudioData(arrBuf);
				audioBuffers.set(file, buffer);
			} catch {
				return;
			}
		}

		const vol = (obj.volume ?? 100) / 100;
		const gain = audioCtx.createGain();
		const source = audioCtx.createBufferSource();
		source.buffer = buffer;
		source.loop = obj.loop === 'loop';

		const startDelay = Math.max(0, obj.start - sceneRelativeTime);
		const targetTime = audioCtx.currentTime + startDelay;

		if (obj.fade_in && obj.fade_in > 0) {
			gain.gain.setValueAtTime(0, targetTime);
			gain.gain.linearRampToValueAtTime(vol, targetTime + obj.fade_in);
		} else {
			gain.gain.value = vol;
		}

		if (obj.fade_out && obj.fade_out > 0 && obj.duration > 0) {
			const fadeOutStart = targetTime + obj.duration - obj.fade_out;
			gain.gain.setValueAtTime(vol, fadeOutStart);
			gain.gain.linearRampToValueAtTime(0, targetTime + obj.duration);
		}

		source.connect(gain);
		gain.connect(audioCtx.destination);
		source.start(targetTime);

		audioNodes.set(obj.id, { source, gain });
	}

	function stopAllAudio() {
		audioNodes.forEach(({ source }) => {
			try {
				source.stop();
			} catch {
				// already stopped
			}
		});
		audioNodes.clear();
	}

	// Resolve variable/static file path
	function resolveFile(
		obj: { id: string; variable?: boolean; file?: string },
		entry: Entry | null
	): string | null {
		if (obj.variable && entry) {
			const v = entry.variables[obj.id];
			if (v && 'file' in v) return v.file as string;
			return null;
		}
		return obj.file ?? null;
	}

	// Resolve variable/static text
	function resolveText(obj: TextObject): string {
		const entry = project?.entries.find((e) => e.name === selectedEntryName) ?? null;
		if (obj.variable && entry) {
			const v = entry.variables[obj.id];
			if (v && 'text' in v) return v.text as string;
			return '';
		}
		return obj.text ?? '';
	}

	// Initialize AudioContext on user gesture and resume if suspended
	function handleUserClick() {
		if (!audioCtx) {
			audioCtx = new AudioContext();
		}
		if (audioCtx.state === 'suspended') {
			audioCtx.resume().then(() => previewStore.resumeAudioContext());
		} else {
			previewStore.resumeAudioContext();
		}
	}

	// External seek: sync video positions
	let prevCurrentTime = 0;
	$effect(() => {
		const t = previewStore.currentTime;
		if (Math.abs(t - prevCurrentTime) < 0.05) {
			prevCurrentTime = t;
			return; // small increments from playback, no seek needed
		}
		prevCurrentTime = t;
		if (!project) return;
		const { sceneIndex, relativeTime } = getCurrentScene(t, project.scenes);
		const scene = project.scenes[sceneIndex];
		if (!scene) return;

		// Sync video elements to new position
		for (const obj of scene.objects) {
			if (obj.type !== 'video') continue;
			const el = videoEls.get(obj.id);
			if (!el) continue;
			const offset = relativeTime - obj.start;
			if (offset >= 0) {
				el.currentTime = offset;
			}
		}
	});
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="preview-canvas-wrap" onclick={handleUserClick} role="presentation">
	<canvas bind:this={canvas} width={canvasWidth} height={canvasHeight}></canvas>
	{#if !previewStore.audioContextReady}
		<div class="audio-hint">クリックして再生を有効化</div>
	{/if}
</div>

<style>
	.preview-canvas-wrap {
		position: relative;
		display: inline-block;
		background: #000;
		line-height: 0;
	}
	canvas {
		display: block;
	}
	.audio-hint {
		position: absolute;
		bottom: 8px;
		left: 50%;
		transform: translateX(-50%);
		background: rgba(0, 0, 0, 0.72);
		color: #fff;
		padding: 4px 12px;
		border-radius: 4px;
		font-size: 12px;
		pointer-events: none;
		white-space: nowrap;
	}
</style>
