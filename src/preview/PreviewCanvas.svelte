<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { projectStore } from '../store/projectStore.svelte';
	import { previewStore } from '../store/previewStore.svelte';
	import { settingsStore } from '../store/settingsStore.svelte';
	import { isObjectVisible, ptToPx } from './sceneUtils';
	import type { VideoObject, ImageObject, TextObject, AudioObject, Scene } from '../types/project';

	// Canvas element
	let canvas = $state<HTMLCanvasElement | undefined>(undefined);
	let wrapEl = $state<HTMLDivElement | undefined>(undefined);
	let wrapW = $state(0);
	let wrapH = $state(0);

	// Derived
	let project = $derived(projectStore.project);
	let scale = $derived(settingsStore.settings?.preview_resolution_scale ?? 0.5);
	let canvasWidth = $derived(project ? Math.round(project.output.width * scale) : 960);
	let canvasHeight = $derived(project ? Math.round(project.output.height * scale) : 540);

	// Scale canvas display size down to fit available container (never upscale)
	let displayScale = $derived(
		wrapW > 0 && wrapH > 0
			? Math.min(1, wrapW / canvasWidth, wrapH / canvasHeight)
			: 1
	);
	let displayW = $derived(Math.round(canvasWidth * displayScale));
	let displayH = $derived(Math.round(canvasHeight * displayScale));

	// RAF
	let rafId: number | null = null;
	const FRAME_INTERVAL = 1000 / 30;
	let playStartWallMs = 0;
	let playStartPreviewTime = 0;
	let lastRenderMs = 0;

	// Media (keyed by object id)
	const videoEls = new Map<string, HTMLVideoElement>();
	const imageEls = new Map<string, HTMLImageElement>();

	// ffprobe 相当の尺キャッシュ（file path → duration in seconds）
	// video element の onloadedmetadata で埋める
	const probedDurations = new Map<string, number>();

	// Audio
	let audioCtx: AudioContext | null = null;
	type AudioNodeEntry = { source: AudioBufferSourceNode; gain: GainNode };
	const audioNodes = new Map<string, AudioNodeEntry>();
	const audioBuffers = new Map<string, AudioBuffer>();
	// Incremented on every stopAllAudio() call; async scheduleAudio checks this to
	// detect cancellation and discard nodes that completed after a stop was requested.
	let audioGeneration = 0;

	// Scene tracking (plain variable — must not be $state; read+write inside $effect would cycle)
	let activeSceneIdx = -1;

	// Drag-to-move state
	type DragState = {
		objectId: string;
		startMouseX: number;
		startMouseY: number;
		startObjX: number;
		startObjY: number;
		capturedScale: number;
		capturedDisplayScale: number;
	};
	let dragState = $state<DragState | null>(null);
	let hoverObjectId = $state<string | null>(null);
	let cursorStyle = $derived(dragState ? 'grabbing' : (hoverObjectId ? 'grab' : 'default'));

	// Track structural changes (object ids + file paths) to detect when media must reload
	let projectSignature = $derived(
		project?.scenes.flatMap(s => s.objects.map(o => {
			const file = 'file' in o ? (o as { file?: string }).file ?? '' : '';
			return `${o.id}:${file}`;
		})).join('|') ?? ''
	);

	onMount(() => {
		// Create AudioContext immediately; resume if browser suspends it automatically.
		// In Tauri (desktop), no user gesture is required.
		audioCtx = new AudioContext();
		if (audioCtx.state === 'suspended') {
			audioCtx.resume().catch(() => {});
		}

		return () => {
			stopLoop();
			stopAllAudio();
			videoEls.forEach((v) => v.pause());
			audioCtx?.close();
			audioCtx = null;
		};
	});

	// Media reset when object structure or files change
	$effect(() => {
		const _sig = projectSignature;
		videoEls.forEach(v => { v.pause(); v.src = ''; });
		videoEls.clear();
		imageEls.clear();
		activeSceneIdx = -1;
	});

	// Static render + totalDuration on any project change or seek
	$effect(() => {
		const _count = projectStore.updateCount;
		const _t = previewStore.currentTime;

		if (project) previewStore.setTotalDuration(computeTotalDuration());

		if (!previewStore.isPlaying) {
			if (canvas) {
				const ctx = canvas.getContext('2d');
				if (ctx) renderFrame(ctx);
			} else {
				// canvas not yet bound (first mount race) – retry on next frame
				requestAnimationFrame(() => {
					if (!canvas) return;
					const ctx = canvas.getContext('2d');
					if (ctx) renderFrame(ctx);
				});
			}
		}
	});

	// Start/stop loop on isPlaying change.
	// IMPORTANT: previewStore.currentTime must be read with untrack() here.
	// Reading it directly would make this effect re-run every frame during playback
	// (since seek() is called every frame), causing stopAllAudio/activeSceneIdx reset
	// and videoEls.play() to fire 30× per second.
	$effect(() => {
		if (previewStore.isPlaying) {
			// Resume AudioContext in case browser suspended it (e.g. no activity)
			if (audioCtx?.state === 'suspended') audioCtx.resume().catch(() => {});
			// Reset scene tracking so handleSceneTransition fires on next frame
			// and schedules fresh audio nodes from the current position.
			stopAllAudio();
			activeSceneIdx = -1;
			playStartWallMs = performance.now();
			const startT = untrack(() => previewStore.currentTime);
			playStartPreviewTime = startT;
			lastRenderMs = 0;
			startLoop();
			// Sync video elements to current preview time before resuming playback
			if (project) {
				const { sceneIndex, relativeTime } = getCurrentSceneEffective(startT);
				const scene = project.scenes[sceneIndex];
				if (scene) {
					for (const obj of scene.objects) {
						if (obj.type !== 'video') continue;
						const el = videoEls.get(obj.id);
						if (!el) continue;
						const offset = relativeTime - obj.start;
						if (offset >= 0) el.currentTime = (obj.trim_start ?? 0) + offset;
					}
				}
			}
			videoEls.forEach((v) => v.play().catch(() => {}));
		} else {
			stopLoop();
			stopAllAudio();
			videoEls.forEach((v) => v.pause());
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
		// Always clear to black first
		ctx.globalAlpha = 1;
		ctx.globalCompositeOperation = 'source-over';
		ctx.fillStyle = '#000000';
		ctx.fillRect(0, 0, canvasWidth, canvasHeight);

		const { sceneIndex, relativeTime } = getCurrentSceneEffective(previewStore.currentTime);
		const scene = project.scenes[sceneIndex];
		if (!scene) return;

		// Scene transition (audio scheduling only)
		if (sceneIndex !== activeSceneIdx) {
			handleSceneTransition(scene, relativeTime);
			activeSceneIdx = sceneIndex;
		}

		const sceneLen = computeEffectiveSceneLen(scene);

		// Ensure media elements exist for all objects in current scene
		// (handles objects added after scene was already active)
		for (const obj of scene.objects) {
			if (obj.type === 'video') ensureVideoEl(obj);
			else if (obj.type === 'image') ensureImageEl(obj);
		}

		// Draw objects in YAML order (= Z-order)
		for (const obj of scene.objects) {
			// For video objects use trimmed duration so the last frame doesn't linger.
			// If trim_end is set, duration = trim_end - trim_start.
			// Otherwise use full probed duration minus trim_start.
			// Fallback 0 = "show until end of scene".
			const objDuration: number = obj.type === 'video'
				? (() => {
					const fileDur = probedDurations.get(obj.file ?? '') ?? 0;
					const ts = obj.trim_start ?? 0;
					const te = obj.trim_end ?? fileDur;
					return fileDur > 0 ? Math.max(0, te - ts) : 0;
				})()
				: 'duration' in obj ? ((obj as { duration: number }).duration ?? 0) : 0;
			if (!isObjectVisible({ start: obj.start, duration: objDuration }, relativeTime, sceneLen)) {
				// Pause video elements that have passed their end time to stop HTML5 audio.
				// Only pause when the video has definitively ended (objDuration known and exceeded),
				// not when it simply hasn't started yet.
				if (obj.type === 'video' && objDuration > 0 && relativeTime >= obj.start + objDuration) {
					const el = videoEls.get(obj.id);
					if (el && !el.paused) el.pause();
				}
				continue;
			}
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

	function handleSceneTransition(scene: Scene, startRelativeTime: number) {
		stopAllAudio();
		const sceneLen = computeEffectiveSceneLen(scene);
		for (const obj of scene.objects) {
			if (obj.type === 'audio') {
				if (previewStore.isPlaying && audioCtx && audioCtx.state !== 'suspended') {
					scheduleAudio(obj, startRelativeTime, sceneLen);
				}
			}
		}
	}

	function ensureVideoEl(obj: VideoObject) {
		const expectedSrc = obj.file ? convertFileSrc(obj.file) : '';
		const existing = videoEls.get(obj.id);
		if (existing) {
			if (existing.dataset.fileSrc !== expectedSrc) {
				existing.pause();
				existing.src = expectedSrc;
				existing.dataset.fileSrc = expectedSrc;
				existing.load();
				existing.onloadeddata = () => triggerStaticRender();
				existing.onerror = () => console.error('[Preview] video load error:', expectedSrc, existing.error);
			}
			return;
		}
		const el = document.createElement('video');
		el.volume = (obj.volume ?? 100) / 100;
		el.preload = 'auto';
		el.src = expectedSrc;
		el.dataset.fileSrc = expectedSrc;
		el.load();
		el.onloadedmetadata = () => {
			if (isFinite(el.duration) && el.duration > 0 && obj.file) {
				probedDurations.set(obj.file, el.duration);
			}
			// Position at trim_start so static preview shows the correct frame
			if ((obj.trim_start ?? 0) > 0) {
				el.currentTime = obj.trim_start!;
			}
		};
		el.onloadeddata = () => { triggerStaticRender(); };
		el.onerror = () => console.error('[Preview] video load error:', expectedSrc, el.error);
		videoEls.set(obj.id, el);

		if ('requestVideoFrameCallback' in el) {
			const scheduleRVFC = () => {
				(el as HTMLVideoElement & { requestVideoFrameCallback: (cb: () => void) => void }).requestVideoFrameCallback(scheduleRVFC);
			};
			scheduleRVFC();
		}

		if (previewStore.isPlaying) el.play().catch(() => {});
	}

	function ensureImageEl(obj: ImageObject) {
		const expectedSrc = obj.file ? convertFileSrc(obj.file) : '';
		const existing = imageEls.get(obj.id);
		if (existing) {
			if (existing.dataset.fileSrc !== expectedSrc) {
				existing.src = expectedSrc;
				existing.dataset.fileSrc = expectedSrc;
				existing.onload = () => triggerStaticRender();
			}
			return;
		}
		const el = new Image();
		el.src = expectedSrc;
		el.dataset.fileSrc = expectedSrc;
		el.onload = () => triggerStaticRender();
		imageEls.set(obj.id, el);
	}

	/**
	 * scene.duration が設定済みならそれを使う。未設定なら probedDurations を参照して
	 * オブジェクト終了時刻の最大値を計算する。
	 * それでも 0 なら Number.MAX_SAFE_INTEGER を返して全オブジェクトが表示されるようにする。
	 */
	function computeEffectiveSceneLen(scene: import('../types/project').Scene): number {
		if (scene.duration != null && scene.duration > 0) return scene.duration;
		let maxEnd = 0;
		for (const obj of scene.objects) {
			let end = 0;
			if (obj.type === 'video') {
				const fileDur = probedDurations.get(obj.file ?? '') ?? 0;
				if (fileDur > 0) {
					const ts = obj.trim_start ?? 0;
					const te = obj.trim_end ?? fileDur;
					end = obj.start + Math.max(0, te - ts);
				}
			} else if ('duration' in obj) {
				const d = (obj as { duration: number }).duration;
				if (d > 0) end = obj.start + d;
			}
			if (end > maxEnd) maxEnd = end;
		}
		return maxEnd > 0 ? maxEnd : Number.MAX_SAFE_INTEGER;
	}

	/**
	 * probe キャッシュを使ってシーン遷移を判定する。
	 * sceneUtils.ts の getCurrentScene (scene.duration のみ参照) の代替として PreviewCanvas 内で使う。
	 * 未 probe のシーン長は MAX_SAFE_INTEGER とみなし、probe 完了まで常にそのシーンに留まる。
	 */
	function getCurrentSceneEffective(time: number): { sceneIndex: number; relativeTime: number } {
		if (!project || project.scenes.length === 0) return { sceneIndex: 0, relativeTime: 0 };
		let cumulative = 0;
		for (let i = 0; i < project.scenes.length; i++) {
			const isLast = i === project.scenes.length - 1;
			const len = computeEffectiveSceneLen(project.scenes[i]);
			if (isLast || len === Number.MAX_SAFE_INTEGER || time < cumulative + len) {
				return { sceneIndex: i, relativeTime: time - cumulative };
			}
			cumulative += len;
		}
		return { sceneIndex: 0, relativeTime: 0 };
	}

	function computeTotalDuration(): number {
		if (!project) return 0;
		return project.scenes.reduce((sum, scene) => {
			const len = computeEffectiveSceneLen(scene);
			// MAX_SAFE_INTEGER = 未 probe：totalDuration には加算しない（seekbar を 0 のままにする）
			return sum + (len === Number.MAX_SAFE_INTEGER ? 0 : len);
		}, 0);
	}

	function triggerStaticRender() {
		if (project) previewStore.setTotalDuration(computeTotalDuration());
		if (!previewStore.isPlaying && canvas) {
			const ctx = canvas.getContext('2d');
			if (ctx) renderFrame(ctx);
		}
	}

	async function scheduleAudio(obj: AudioObject, sceneRelativeTime: number, sceneLen: number) {
		if (!audioCtx) return;
		const file = obj.file ?? null;
		if (!file) return;

		// Capture generation at call time; discard if cancelled while awaiting
		const gen = audioGeneration;

		let buffer = audioBuffers.get(file);
		if (!buffer) {
			try {
				const res = await fetch(convertFileSrc(file));
				const arrBuf = await res.arrayBuffer();
				if (audioGeneration !== gen || !audioCtx) return;
				buffer = await audioCtx.decodeAudioData(arrBuf);
				if (audioGeneration !== gen || !audioCtx) return;
				audioBuffers.set(file, buffer);
			} catch {
				return;
			}
		}

		// Final check before creating nodes
		if (audioGeneration !== gen || !audioCtx) return;

		const vol = (obj.volume ?? 100) / 100;
		const gain = audioCtx.createGain();
		const source = audioCtx.createBufferSource();
		source.buffer = buffer;
		source.loop = obj.loop === 'loop';

		const startDelay = Math.max(0, obj.start - sceneRelativeTime);
		const targetTime = audioCtx.currentTime + startDelay;
		// duration=0 means "play until end of scene"
		const effectiveDuration = obj.duration > 0 ? obj.duration : Math.max(0, sceneLen - obj.start);

		if (obj.fade_in && obj.fade_in > 0) {
			gain.gain.setValueAtTime(0, targetTime);
			gain.gain.linearRampToValueAtTime(vol, targetTime + obj.fade_in);
		} else {
			gain.gain.value = vol;
		}

		if (obj.fade_out && obj.fade_out > 0 && effectiveDuration > 0) {
			const fadeOutStart = targetTime + effectiveDuration - obj.fade_out;
			gain.gain.setValueAtTime(vol, fadeOutStart);
			gain.gain.linearRampToValueAtTime(0, targetTime + effectiveDuration);
		}

		source.connect(gain);
		gain.connect(audioCtx.destination);
		source.start(targetTime);
		// Schedule a hard stop so looping audio doesn't outlive the scene.
		// Skip when effectiveDuration is effectively infinite (scene has no defined length);
		// stopAllAudio() will handle cleanup on pause/scene-change.
		if (effectiveDuration > 0 && effectiveDuration < Number.MAX_SAFE_INTEGER / 2) {
			source.stop(targetTime + effectiveDuration);
		}

		audioNodes.set(obj.id, { source, gain });
	}

	function stopAllAudio() {
		audioGeneration++; // invalidate any in-flight scheduleAudio calls
		audioNodes.forEach(({ source }) => {
			try {
				source.stop();
			} catch {
				// already stopped
			}
		});
		audioNodes.clear();
	}

	function resolveText(obj: TextObject): string {
		return obj.text ?? '';
	}

	function handleUserClick() {
		// Resume AudioContext if it was suspended by the browser
		if (audioCtx?.state === 'suspended') audioCtx.resume().catch(() => {});
	}

	// Returns the topmost visible-bounds object id under the given canvas-element-relative coords.
	// Audio objects are skipped (no visual bounds).
	function hitTestObject(offsetX: number, offsetY: number): string | null {
		if (!project) return null;
		const { sceneIndex } = getCurrentSceneEffective(previewStore.currentTime);
		const scene = project.scenes[sceneIndex];
		if (!scene) return null;

		const projX = offsetX / (scale * displayScale);
		const projY = offsetY / (scale * displayScale);

		for (let i = scene.objects.length - 1; i >= 0; i--) {
			const obj = scene.objects[i];
			if (obj.type === 'audio') continue;
			const o = obj as { x: number; y: number; width: number; height: number };
			if (projX >= o.x && projX <= o.x + o.width && projY >= o.y && projY <= o.y + o.height) {
				return obj.id;
			}
		}
		return null;
	}

	function getCanvasRelativeOffset(e: PointerEvent): { x: number; y: number } {
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		return { x: e.clientX - rect.left, y: e.clientY - rect.top };
	}

	function handlePointerDown(e: PointerEvent) {
		if (audioCtx?.state === 'suspended') audioCtx.resume().catch(() => {});
		const { x: offsetX, y: offsetY } = getCanvasRelativeOffset(e);
		const hitId = hitTestObject(offsetX, offsetY);
		if (!hitId || !project) return;

		e.preventDefault();
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);

		const { sceneIndex } = getCurrentSceneEffective(previewStore.currentTime);
		const scene = project.scenes[sceneIndex];
		if (scene) {
			projectStore.selectScene(scene.id);
			projectStore.selectObject(hitId);
		}

		const obj = scene?.objects.find(o => o.id === hitId);
		if (!obj || obj.type === 'audio') return;
		const o = obj as { x: number; y: number };
		dragState = {
			objectId: hitId,
			startMouseX: offsetX,
			startMouseY: offsetY,
			startObjX: o.x,
			startObjY: o.y,
			capturedScale: scale,
			capturedDisplayScale: displayScale,
		};
	}

	function handlePointerMove(e: PointerEvent) {
		const { x: offsetX, y: offsetY } = getCanvasRelativeOffset(e);

		if (dragState) {
			const totalScale = dragState.capturedScale * dragState.capturedDisplayScale;
			const dx = (offsetX - dragState.startMouseX) / totalScale;
			const dy = (offsetY - dragState.startMouseY) / totalScale;
			const newX = Math.round(dragState.startObjX + dx);
			const newY = Math.round(dragState.startObjY + dy);
			const objId = dragState.objectId;
			projectStore.updateProject((p) => {
				for (const scene of p.scenes) {
					const obj = scene.objects.find(o => o.id === objId);
					if (obj && 'x' in obj) {
						(obj as { x: number; y: number }).x = newX;
						(obj as { x: number; y: number }).y = newY;
					}
				}
			});
			return;
		}

		hoverObjectId = hitTestObject(offsetX, offsetY);
	}

	function handlePointerUp(_e: PointerEvent) {
		dragState = null;
	}

	// External seek: sync video positions when user manually scrubs the seekbar
	let prevCurrentTime = 0;
	$effect(() => {
		const t = previewStore.currentTime;
		if (Math.abs(t - prevCurrentTime) < 0.05) {
			prevCurrentTime = t;
			return; // small increments from playback loop, skip
		}
		prevCurrentTime = t;
		if (!project) return;
		const { sceneIndex, relativeTime } = getCurrentSceneEffective(t);
		const scene = project.scenes[sceneIndex];
		if (!scene) return;

		// Sync video elements to new position (account for trim_start offset)
		for (const obj of scene.objects) {
			if (obj.type !== 'video') continue;
			const el = videoEls.get(obj.id);
			if (!el) continue;
			const offset = relativeTime - obj.start;
			if (offset >= 0) {
				el.currentTime = (obj.trim_start ?? 0) + offset;
			}
		}
		// Audio is handled by stopAllAudio() in the isPlaying effect when play restarts.
		// No action needed here: if paused the nodes are already stopped; if playing the
		// generation counter ensures stale async completions are discarded.
	});
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="preview-canvas-wrap" bind:this={wrapEl} bind:clientWidth={wrapW} bind:clientHeight={wrapH} onclick={handleUserClick} role="presentation">
	<canvas
		bind:this={canvas}
		width={canvasWidth}
		height={canvasHeight}
		style="width:{displayW}px;height:{displayH}px;cursor:{cursorStyle};"
		onpointerdown={handlePointerDown}
		onpointermove={handlePointerMove}
		onpointerup={handlePointerUp}
	></canvas>
</div>

<style>
	.preview-canvas-wrap {
		position: relative;
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #000;
		line-height: 0;
	}
	canvas {
		display: block;
	}
</style>
