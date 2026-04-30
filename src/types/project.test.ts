import { describe, it, expect } from 'vitest';
import { isProject } from './project';

describe('isProject Type Guard', () => {
  it('validates a valid minimal project object', () => {
    const valid = {
      version: 1,
      output_folder: '/path',
      output: { output_name: 'out', width: 1920, height: 1080, fps: 30, codec: 'h264', format: 'mp4', crf: 23, preset: 'medium' },
      scenes: []
    };
    expect(isProject(valid)).toBe(true);
  });

  it('rejects invalid objects', () => {
    expect(isProject(null)).toBe(false);
    expect(isProject(undefined)).toBe(false);
    expect(isProject({})).toBe(false);
    expect(isProject({ version: 2 })).toBe(false);
    expect(isProject({ version: 1, output_folder: 123, output: {}, scenes: [] })).toBe(false);
    expect(isProject({ version: 1, output_folder: '/path', output: null, scenes: [] })).toBe(false);
  });
});
