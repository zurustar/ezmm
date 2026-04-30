// Export event payloads (Rust → frontend)
export interface ExportProgressPayload {
  progress: number | null; // 0.0–1.0
}

export interface ExportDonePayload {
  output_path: string;
  elapsed_ms: number;
}

export interface ExportErrorPayload {
  message: string;
  ffmpeg_stderr: string | null;
}

// Validation
export interface ValidationIssue {
  severity: 'Error' | 'Warning';
  code: string;
  message: string;
  scene_id: string | null;
  object_id: string | null;
}

export interface ValidationResult {
  errors: ValidationIssue[];
  warnings: ValidationIssue[];
}

// Infra
export interface ProbeResult {
  duration?: number;
  width?: number;
  height?: number;
  fps?: number;
  has_audio: boolean;
  sample_rate?: number;
}

export interface FontPaths {
  regular: string;
  bold: string;
}

// Settings
export interface WindowSettings {
  width: number;
  height: number;
  x: number | null;
  y: number | null;
}

export interface AppSettings {
  version: number;
  default_crf: number;
  default_preset: string;
  preview_resolution_scale: number;
  last_open_folder: string | null;
  recent_files: string[];
  window: WindowSettings;
}
