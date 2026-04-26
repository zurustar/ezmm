// Batch event payloads (Rust → frontend)
export interface BatchProgressPayload {
	entry_index: number;
	total: number;
	entry_name: string;
	entry_progress?: number; // 0.0–1.0
}

export interface BatchEntryDonePayload {
	entry_name: string;
	output_path: string;
	elapsed_ms: number;
}

export interface BatchEntryErrorPayload {
	entry_name: string;
	message: string;
	ffmpeg_stderr?: string;
}

export interface BatchDonePayload {
	success_count: number;
	error_count: number;
	total_elapsed_ms: number;
}

// Validation
export interface ValidationIssue {
	severity: 'error' | 'warning';
	code: string;
	message: string;
	object_id?: string;
	entry_name?: string;
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
