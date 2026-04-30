import type { Scene } from '../types/project';

/** Sum of all scene durations; scenes without an explicit duration contribute 0. */
export function calculateTotalDuration(scenes: Scene[]): number {
	return scenes.reduce((sum, s) => sum + (s.duration ?? 0), 0);
}

/** Find which scene contains currentTime and the relative offset within that scene. */
export function getCurrentScene(
	currentTime: number,
	scenes: Scene[]
): { sceneIndex: number; relativeTime: number } {
	let cumulative = 0;
	for (let i = 0; i < scenes.length; i++) {
		const dur = scenes[i].duration ?? 0;
		const isLast = i === scenes.length - 1;
		if (isLast || currentTime < cumulative + dur) {
			return { sceneIndex: i, relativeTime: currentTime - cumulative };
		}
		cumulative += dur;
	}
	return { sceneIndex: 0, relativeTime: 0 };
}

/**
 * Whether an object is visible at relativeTime within its scene.
 * duration=0 means "show until end of scene" (from start to sceneLen).
 */
export function isObjectVisible(
	obj: { start: number; duration: number },
	relativeTime: number,
	sceneLen: number
): boolean {
	if (relativeTime < obj.start) return false;
	if (obj.duration === 0) return relativeTime < sceneLen;
	return relativeTime < obj.start + obj.duration;
}

/** Convert font size in points to pixels (96 DPI: 1pt = 96/72 px). */
export function ptToPx(pt: number): number {
	return Math.round(pt * 96 / 72);
}
