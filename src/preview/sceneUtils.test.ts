import { describe, it, expect } from 'vitest';
import { calculateTotalDuration, getCurrentScene, isObjectVisible, ptToPx } from './sceneUtils';
import type { Scene } from '../types/project';

const makeScene = (duration: number | undefined, id = 's'): Scene => ({
	id,
	duration,
	objects: []
});

// Cycle 6-1: calculateTotalDuration
describe('calculateTotalDuration', () => {
	it('sums durations of three scenes', () => {
		const scenes = [makeScene(3), makeScene(5), makeScene(2)];
		expect(calculateTotalDuration(scenes)).toBe(10.0);
	});

	it('treats scene with undefined duration as 0', () => {
		const scenes = [makeScene(3), makeScene(undefined), makeScene(2)];
		expect(calculateTotalDuration(scenes)).toBe(5.0);
	});

	it('treats scene with duration=0 as 0 contribution', () => {
		const scenes = [makeScene(3), makeScene(0), makeScene(2)];
		expect(calculateTotalDuration(scenes)).toBe(5.0);
	});

	it('returns 0 for empty scene list', () => {
		expect(calculateTotalDuration([])).toBe(0);
	});
});

// Cycle 6-2: getCurrentScene
describe('getCurrentScene', () => {
	const scenes = [makeScene(3, 's1'), makeScene(5, 's2')]; // spans [0,3) and [3,8)

	it('returns scene2 and relative=2.5 for currentTime=5.5', () => {
		const result = getCurrentScene(5.5, scenes);
		expect(result.sceneIndex).toBe(1);
		expect(result.relativeTime).toBeCloseTo(2.5);
	});

	it('boundary: currentTime=3.0 is start of scene2 (relative=0)', () => {
		const result = getCurrentScene(3.0, scenes);
		expect(result.sceneIndex).toBe(1);
		expect(result.relativeTime).toBeCloseTo(0.0);
	});

	it('currentTime within scene1 returns sceneIndex=0', () => {
		const result = getCurrentScene(1.5, scenes);
		expect(result.sceneIndex).toBe(0);
		expect(result.relativeTime).toBeCloseTo(1.5);
	});

	it('currentTime at start returns first scene', () => {
		const result = getCurrentScene(0, scenes);
		expect(result.sceneIndex).toBe(0);
		expect(result.relativeTime).toBe(0);
	});

	it('currentTime past end clamps to last scene', () => {
		const result = getCurrentScene(99, scenes);
		expect(result.sceneIndex).toBe(1);
	});
});

// Cycle 6-3: isObjectVisible
describe('isObjectVisible', () => {
	it('start=2.0, duration=3.0: visible at t=2.0 and t=4.9, not at t=1.9 or t=5.0', () => {
		expect(isObjectVisible({ start: 2.0, duration: 3.0 }, 1.9, 10)).toBe(false);
		expect(isObjectVisible({ start: 2.0, duration: 3.0 }, 2.0, 10)).toBe(true);
		expect(isObjectVisible({ start: 2.0, duration: 3.0 }, 4.9, 10)).toBe(true);
		expect(isObjectVisible({ start: 2.0, duration: 3.0 }, 5.0, 10)).toBe(false);
	});

	it('duration=0 (シーン終端): visible from start to sceneLen', () => {
		const sceneLen = 8;
		expect(isObjectVisible({ start: 2.0, duration: 0 }, 1.9, sceneLen)).toBe(false);
		expect(isObjectVisible({ start: 2.0, duration: 0 }, 2.0, sceneLen)).toBe(true);
		expect(isObjectVisible({ start: 2.0, duration: 0 }, 7.9, sceneLen)).toBe(true);
		expect(isObjectVisible({ start: 2.0, duration: 0 }, 8.0, sceneLen)).toBe(false);
	});
});

// Cycle 6-4: ptToPx
describe('ptToPx', () => {
	it('48pt → 64px', () => {
		expect(ptToPx(48)).toBe(64);
	});

	it('24pt → 32px', () => {
		expect(ptToPx(24)).toBe(32);
	});

	it('1pt → 1px (round)', () => {
		expect(ptToPx(1)).toBe(1);
	});
});
