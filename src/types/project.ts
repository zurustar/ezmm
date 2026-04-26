export interface Project {
  version: number;
  output_folder: string;
  output: OutputSettings;
  scenes: Scene[];
  entries: Entry[];
}

export interface OutputSettings {
  width: number;
  height: number;
  fps: number;
  codec: 'h264' | 'h265' | 'vp9';
  format: 'mp4' | 'mov' | 'webm';
  crf: number;
  preset: string;
}

export interface Scene {
  id: string;
  duration?: number;
  objects: SceneObject[];
}

export type SceneObject = VideoObject | ImageObject | TextObject | AudioObject;

export interface BaseObject {
  id: string;
  variable?: boolean;
  start: number;
}

export interface VideoObject extends BaseObject {
  type: 'video';
  file?: string;
  x: number; y: number; width: number; height: number;
  opacity: number;
  volume: number;
}

export interface ImageObject extends BaseObject {
  type: 'image';
  file?: string;
  x: number; y: number; width: number; height: number;
  duration: number;
  opacity: number;
}

export interface TextObject extends BaseObject {
  type: 'text';
  text?: string;
  x: number; y: number; width: number; height: number;
  duration: number;
  opacity: number;
  font: string;
  font_size: number;
  color: string;
  background_color?: string;
  align?: 'left' | 'center' | 'right';
}

export interface AudioObject extends BaseObject {
  type: 'audio';
  file?: string;
  duration: number;
  volume: number;
  fade_in?: number;
  fade_out?: number;
  loop: 'loop' | 'silence';
}

export interface Entry {
  name: string;
  variables: Record<string, VariableValue>;
}

export type VideoVariable   = { file: string; trim_start?: number; trim_end?: number };
export type ImageVariable   = { file: string };
export type AudioVariable   = { file: string };
export type TextVariable    = { text: string };
export type VariableValue = VideoVariable | ImageVariable | AudioVariable | TextVariable;

export function isProject(obj: unknown): obj is Project {
  if (typeof obj !== 'object' || obj === null) return false;
  const p = obj as Record<string, unknown>;
  if (typeof p.version !== 'number' || p.version !== 1) return false;
  if (typeof p.output_folder !== 'string') return false;
  if (typeof p.output !== 'object' || p.output === null) return false;
  if (!Array.isArray(p.scenes)) return false;
  if (!Array.isArray(p.entries)) return false;
  return true;
}
