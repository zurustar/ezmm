// filter.rs — filter_complex グラフ生成

use std::collections::HashMap;
use std::path::Path;
use crate::project::{
    AudioObject, ImageObject, LoopMode, OutputSettings, Project,
    Scene, SceneObject, TextObject, VideoObject,
};
use crate::renderer::escape::{escape_drawtext_value, escape_filter_value};
use crate::renderer::probe::ProbeResult;
use crate::renderer::codec::output_sample_rate;

// ──────────────────────────────────────────────────────────────────────────────
// 公開型
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FilterGraph {
    pub inputs: Vec<InputSpec>,
    pub filter_complex: String,
    pub video_map: String,
    pub audio_map: String,
}

#[derive(Debug, Clone)]
pub struct InputSpec {
    pub path: String,
    pub image_flags: Option<ImageFlags>,
}

#[derive(Debug, Clone)]
pub struct ImageFlags {
    pub duration: f64,
}

// ──────────────────────────────────────────────────────────────────────────────
// 入力ファイル重複排除
// ──────────────────────────────────────────────────────────────────────────────

struct InputIndex {
    map: HashMap<String, usize>,
    specs: Vec<InputSpec>,
}

impl InputIndex {
    fn new() -> Self {
        Self { map: HashMap::new(), specs: Vec::new() }
    }

    fn register_video_audio(&mut self, path: &str) -> usize {
        if let Some(&idx) = self.map.get(path) {
            idx
        } else {
            let idx = self.specs.len();
            self.specs.push(InputSpec { path: path.to_string(), image_flags: None });
            self.map.insert(path.to_string(), idx);
            idx
        }
    }

    fn register_image(&mut self, path: &str, duration: f64) -> usize {
        let idx = self.specs.len();
        self.specs.push(InputSpec { path: path.to_string(), image_flags: Some(ImageFlags { duration }) });
        idx
    }

    fn build_split_fragments(&self) -> Vec<String> {
        // FFmpeg 5.0+ resolves multiple references to the same input stream
        // automatically, so no explicit split filters are needed.
        Vec::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// メイン API
// ──────────────────────────────────────────────────────────────────────────────

pub fn build_filter_graph(
    project: &Project,
    probes: &HashMap<String, ProbeResult>,
    font_dir: &Path,
) -> Result<FilterGraph, String> {
    let settings = &project.output;
    let scenes = &project.scenes;

    let mut input_idx = InputIndex::new();
    let scene_filters: Vec<SceneFilter> = scenes
        .iter()
        .enumerate()
        .map(|(si, scene)| build_scene_filter(si, scene, settings, probes, &mut input_idx, scenes.len(), font_dir))
        .collect::<Result<_, _>>()?;

    let split_frags = input_idx.build_split_fragments();
    let sample_rate = output_sample_rate(&settings.format);
    let filter_complex = assemble_filter_complex(&scene_filters, sample_rate, &split_frags);

    let (video_map, audio_map) = if scenes.len() == 1 {
        ("[vout]".to_string(), "[aout]".to_string())
    } else {
        ("[vfinal]".to_string(), "[afinal]".to_string())
    };

    Ok(FilterGraph { inputs: input_idx.specs, filter_complex, video_map, audio_map })
}

// ──────────────────────────────────────────────────────────────────────────────
// シーン単位フィルタ
// ──────────────────────────────────────────────────────────────────────────────

struct SceneFilter {
    fragments: Vec<String>,
    video_out: String,
    audio_out: String,
}

/// シーンの実効長を計算する。
/// `scene.duration` が設定済みならそれを優先。未設定なら `probes` を使って
/// オブジェクトの終了時刻の最大値を求める（duration=0 のオブジェクトは除外）。
fn video_trim_duration(v: &VideoObject, file_dur: f64) -> f64 {
    let s = v.trim_start.unwrap_or(0.0);
    let e = v.trim_end.unwrap_or(file_dur);
    (e - s).max(0.0)
}

pub fn compute_scene_len(scene: &Scene, probes: &HashMap<String, ProbeResult>) -> f64 {
    if let Some(d) = scene.duration {
        if d > 0.0 { return d; }
    }
    scene.objects.iter().filter_map(|obj| {
        match obj {
            SceneObject::Video(v) => {
                let file_dur = v.file.as_deref()
                    .and_then(|f| probes.get(f))
                    .map(|p| p.duration)
                    .unwrap_or(0.0);
                let dur = video_trim_duration(v, file_dur);
                Some(v.start + dur)
            }
            SceneObject::Image(img) if img.duration > 0.0 => Some(img.start + img.duration),
            SceneObject::Text(txt) if txt.duration > 0.0 => Some(txt.start + txt.duration),
            SceneObject::Audio(aud) if aud.duration > 0.0 => Some(aud.start + aud.duration),
            _ => None,
        }
    }).fold(0.0_f64, f64::max)
}

fn build_scene_filter(
    si: usize,
    scene: &Scene,
    settings: &OutputSettings,
    probes: &HashMap<String, ProbeResult>,
    input_idx: &mut InputIndex,
    total_scenes: usize,
    font_dir: &Path,
) -> Result<SceneFilter, String> {
    let scene_len = compute_scene_len(scene, probes);
    let w = settings.width;
    let h = settings.height;

    let mut frags: Vec<String> = Vec::new();
    let mut current_v_label = format!("[s{si}_bg]");

    frags.push(format!(
        "color=black:s={w}x{h}:d={scene_len:.6},format=yuva420p{current_v_label}"
    ));

    let mut audio_labels: Vec<String> = Vec::new();

    for (oi, obj) in scene.objects.iter().enumerate() {
        match obj {
            SceneObject::Video(v) => {
                let next_label = format!("[s{si}_v{oi}]");
                let frags_v = build_video_filter(
                    si, oi, v, scene_len, probes, input_idx, &current_v_label, &next_label,
                    &mut audio_labels,
                )?;
                frags.extend(frags_v);
                current_v_label = next_label;
            }
            SceneObject::Image(img) => {
                let next_label = format!("[s{si}_v{oi}]");
                let frags_i = build_image_filter(
                    si, oi, img, scene_len, input_idx, &current_v_label, &next_label,
                )?;
                frags.extend(frags_i);
                current_v_label = next_label;
            }
            SceneObject::Text(txt) => {
                let next_label = format!("[s{si}_v{oi}]");
                let frags_t = build_text_filter(si, oi, txt, scene_len, &current_v_label, &next_label, font_dir)?;
                frags.extend(frags_t);
                current_v_label = next_label;
            }
            SceneObject::Audio(aud) => {
                let label = format!("[s{si}_a{oi}]");
                let frags_a = build_audio_filter(
                    si, oi, aud, scene_len, settings, probes, input_idx, &label,
                )?;
                frags.extend(frags_a);
                audio_labels.push(label);
            }
        }
    }

    let video_out = if total_scenes == 1 { "[vout]".to_string() } else { format!("[s{si}v]") };
    frags.push(format!("{current_v_label}null{video_out}"));

    let audio_out = if total_scenes == 1 { "[aout]".to_string() } else { format!("[s{si}a]") };
    frags.push(build_amix(&audio_labels, scene_len, &audio_out));

    Ok(SceneFilter { fragments: frags, video_out, audio_out })
}

// ──────────────────────────────────────────────────────────────────────────────
// オブジェクト別フィルタ生成
// ──────────────────────────────────────────────────────────────────────────────

fn build_video_filter(
    si: usize,
    oi: usize,
    v: &VideoObject,
    scene_len: f64,
    probes: &HashMap<String, ProbeResult>,
    input_idx: &mut InputIndex,
    base_label: &str,
    out_label: &str,
    audio_labels: &mut Vec<String>,
) -> Result<Vec<String>, String> {
    let file = v.file.as_deref()
        .ok_or_else(|| format!("video object '{}': file が指定されていません", v.id))?;

    let idx = input_idx.register_video_audio(file);
    let probe = probes.get(file);
    let obj_w = v.width;
    let obj_h = v.height;

    let file_dur = probe.map(|p| p.duration).unwrap_or(0.0);
    let trim_start = v.trim_start.unwrap_or(0.0);
    let trim_end = v.trim_end.unwrap_or(file_dur);
    let clip_dur = video_trim_duration(v, file_dur);
    let needs_trim = trim_start > 0.0 || v.trim_end.is_some();

    // Determine label feeding into the scale filter
    let pre_scale_label: String;
    let mut frags: Vec<String> = Vec::new();

    if needs_trim {
        let trim_label = format!("[s{si}_vtrim{oi}]");
        frags.push(format!(
            "[{idx}:v]trim=start={trim_start:.6}:end={trim_end:.6},setpts=PTS-STARTPTS{trim_label}"
        ));
        pre_scale_label = trim_label;
    } else {
        pre_scale_label = format!("[{idx}:v]");
    }

    let proc_label = format!("[s{si}_vraw{oi}]");
    if v.opacity < 100 {
        let aa = v.opacity as f64 / 100.0;
        frags.push(format!(
            "{pre_scale_label}scale={obj_w}:{obj_h}:flags=lanczos,format=yuva420p,colorchannelmixer=aa={aa:.6}{proc_label}"
        ));
    } else {
        frags.push(format!(
            "{pre_scale_label}scale={obj_w}:{obj_h}:flags=lanczos,format=yuva420p{proc_label}"
        ));
    }

    let overlay_dur = if clip_dur > 0.0 { clip_dur } else { scene_len };
    let overlay_end = v.start + overlay_dur;
    let overlay_enable = format!("enable='between(t,{:.6},{:.6})'", v.start, overlay_end);
    frags.push(format!(
        "{base_label}{proc_label}overlay={x}:{y}:{enable}{out_label}",
        x = v.x, y = v.y, enable = overlay_enable
    ));

    if probe.map(|p| p.has_audio).unwrap_or(false) {
        let audio_label = format!("[s{si}_va{oi}]");
        let vol = v.volume as f64 / 100.0;
        if needs_trim {
            frags.push(format!(
                "[{idx}:a]atrim=start={trim_start:.6}:end={trim_end:.6},asetpts=PTS-STARTPTS,volume={vol:.6}{audio_label}"
            ));
        } else {
            frags.push(format!("[{idx}:a]volume={vol:.6}{audio_label}"));
        }
        audio_labels.push(audio_label);
    }

    Ok(frags)
}

fn build_image_filter(
    si: usize,
    oi: usize,
    img: &ImageObject,
    scene_len: f64,
    input_idx: &mut InputIndex,
    base_label: &str,
    out_label: &str,
) -> Result<Vec<String>, String> {
    let file = img.file.as_deref()
        .ok_or_else(|| format!("image object '{}': file が指定されていません", img.id))?;

    let eff_dur = if img.duration == 0.0 { scene_len } else { img.duration };
    let idx = input_idx.register_image(file, eff_dur);
    let proc_label = format!("[s{si}_iraw{oi}]");

    let opacity_frag = if img.opacity < 100 {
        let aa = img.opacity as f64 / 100.0;
        format!("[{idx}:v]scale={w}:{h}:flags=lanczos,format=yuva420p,colorchannelmixer=aa={aa:.6}{proc_label}",
            w = img.width, h = img.height)
    } else {
        format!("[{idx}:v]scale={w}:{h}:flags=lanczos,format=yuva420p{proc_label}",
            w = img.width, h = img.height)
    };

    let enable = format!("enable='between(t,{:.6},{:.6})'", img.start, img.start + eff_dur);
    let overlay_frag = format!("{base_label}{proc_label}overlay={x}:{y}:{enable}{out_label}",
        x = img.x, y = img.y);

    Ok(vec![opacity_frag, overlay_frag])
}

fn build_text_filter(
    _si: usize,
    _oi: usize,
    txt: &TextObject,
    scene_len: f64,
    base_label: &str,
    out_label: &str,
    font_dir: &Path,
) -> Result<Vec<String>, String> {
    let text_val = txt.text.as_deref()
        .ok_or_else(|| format!("text object '{}': text が指定されていません", txt.id))?;

    let eff_dur = if txt.duration == 0.0 { scene_len } else { txt.duration };
    let font_px = pt_to_px(txt.font_size);
    let escaped_text = escape_drawtext_value(text_val);

    // font_dir/FontName.ttc のフルパスを使う（drawtext の fontfile= はフルパス必須）
    let font_path = font_dir.join(format!("{}.otf", txt.font));
    let escaped_font = escape_filter_value(&font_path.to_string_lossy());
    let color_hex = txt.color.trim_start_matches('#');

    let mut drawtext_args = format!(
        "fontfile={escaped_font}:text='{escaped_text}':x={x}:y={y}:fontsize={font_px}:fontcolor=0x{color_hex}",
        x = txt.x, y = txt.y,
    );

    if let Some(bg) = &txt.background_color {
        let bg_hex = bg.trim_start_matches('#');
        drawtext_args.push_str(&format!(":box=1:boxcolor=0x{bg_hex}"));
    }

    drawtext_args.push_str(&format!(":enable='between(t,{:.6},{:.6})'", txt.start, txt.start + eff_dur));

    Ok(vec![format!("{base_label}drawtext={drawtext_args}{out_label}")])
}

fn build_audio_filter(
    _si: usize,
    _oi: usize,
    aud: &AudioObject,
    scene_len: f64,
    settings: &OutputSettings,
    probes: &HashMap<String, ProbeResult>,
    input_idx: &mut InputIndex,
    out_label: &str,
) -> Result<Vec<String>, String> {
    let file = aud.file.as_deref()
        .ok_or_else(|| format!("audio object '{}': file が指定されていません", aud.id))?;

    let idx = input_idx.register_video_audio(file);
    let probe = probes.get(file);
    let sample_rate = output_sample_rate(&settings.format);
    let file_duration = probe.map(|p| p.duration).unwrap_or(scene_len);
    let file_sample_rate = probe.and_then(|p| p.sample_rate).unwrap_or(sample_rate);

    let eff_dur = if aud.duration == 0.0 { scene_len } else { aud.duration };
    let vol = aud.volume as f64 / 100.0;
    let delay_ms = (aud.start * 1000.0).round() as i64;

    let mut chain = format!(
        "[{idx}:a]aformat=sample_rates={sample_rate}:channel_layouts=stereo,volume={vol:.6}"
    );

    if let Some(fi) = aud.fade_in {
        chain.push_str(&format!(",afade=in:st={:.6}:d={:.6}", aud.start, fi));
    }
    if let Some(fo) = aud.fade_out {
        let fade_start = scene_len - fo;
        chain.push_str(&format!(",afade=out:st={:.6}:d={:.6}", fade_start, fo));
    }

    match &aud.r#loop {
        LoopMode::Loop => {
            let size = (file_duration * file_sample_rate as f64).round() as i64;
            chain.push_str(&format!(",aloop=loop=-1:size={size}"));
        }
        LoopMode::Silence => {
            chain.push_str(&format!(",atrim=end={file_duration:.6},apad=whole_dur={eff_dur:.6}"));
        }
    }

    if delay_ms > 0 {
        chain.push_str(&format!(",adelay={delay_ms}|{delay_ms}"));
    }
    chain.push_str(&format!(",atrim=duration={eff_dur:.6}"));
    chain.push_str(out_label);

    Ok(vec![chain])
}

// ──────────────────────────────────────────────────────────────────────────────
// amix / concat
// ──────────────────────────────────────────────────────────────────────────────

fn build_amix(audio_labels: &[String], scene_len: f64, out_label: &str) -> String {
    match audio_labels.len() {
        0 => format!("anullsrc=r=44100:cl=stereo,atrim=duration={scene_len:.6}{out_label}"),
        1 => format!("{}anull{out_label}", audio_labels[0]),
        n => {
            let inputs_str = audio_labels.join("");
            format!("{inputs_str}amix=inputs={n}:duration=longest{out_label}")
        }
    }
}

fn assemble_filter_complex(
    scene_filters: &[SceneFilter],
    sample_rate: u32,
    split_frags: &[String],
) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.extend_from_slice(split_frags);

    for sf in scene_filters {
        parts.extend(sf.fragments.iter().cloned());
    }

    if scene_filters.len() > 1 {
        let mut concat_inputs = String::new();
        for (i, sf) in scene_filters.iter().enumerate() {
            let ar_out = format!("[s{i}ar]");
            parts.push(format!("{}aresample={sample_rate}{ar_out}", sf.audio_out));
            concat_inputs.push_str(&sf.video_out);
            concat_inputs.push_str(&ar_out);
        }
        let n = scene_filters.len();
        parts.push(format!("{concat_inputs}concat=n={n}:v=1:a=1[vfinal][afinal]"));
    }

    parts.join(";")
}

// ──────────────────────────────────────────────────────────────────────────────
// ヘルパー
// ──────────────────────────────────────────────────────────────────────────────

pub fn pt_to_px(pt: u32) -> u32 {
    ((pt as f64 * 96.0 / 72.0).round()) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Codec, Format, OutputSettings};

    fn base_settings() -> OutputSettings {
        OutputSettings {
            output_name: "output".into(),
            width: 1920,
            height: 1080,
            fps: 30,
            codec: Codec::H264,
            format: Format::Mp4,
            crf: 23,
            preset: "medium".into(),
        }
    }

    #[test]
    fn pt_to_px_48pt_is_64px() { assert_eq!(pt_to_px(48), 64); }

    #[test]
    fn pt_to_px_24pt_is_32px() { assert_eq!(pt_to_px(24), 32); }

    #[test]
    fn pt_to_px_1pt_is_1px() { assert_eq!(pt_to_px(1), 1); }

    #[test]
    fn amix_no_audio_generates_anullsrc() {
        let frag = build_amix(&[], 5.0, "[aout]");
        assert!(frag.contains("anullsrc"), "frag={frag}");
        assert!(frag.ends_with("[aout]"), "frag={frag}");
    }

    #[test]
    fn amix_single_audio_uses_anull() {
        let frag = build_amix(&["[s0_a0]".to_string()], 5.0, "[aout]");
        assert!(frag.contains("anull"), "frag={frag}");
        assert!(!frag.contains("amix"), "frag={frag}");
    }

    #[test]
    fn amix_two_audio_uses_amix_inputs_2() {
        let labels = vec!["[s0_a0]".to_string(), "[s0_a1]".to_string()];
        let frag = build_amix(&labels, 5.0, "[aout]");
        assert!(frag.contains("amix=inputs=2"), "frag={frag}");
    }

    #[test]
    fn same_file_registered_once_in_inputs() {
        let mut idx = InputIndex::new();
        let i1 = idx.register_video_audio("/videos/foo.mp4");
        let i2 = idx.register_video_audio("/videos/foo.mp4");
        assert_eq!(i1, i2);
        assert_eq!(idx.specs.len(), 1);
    }

    #[test]
    fn no_explicit_split_for_duplicate_file() {
        // FFmpeg handles duplicate stream refs natively; we don't emit split filters
        let mut idx = InputIndex::new();
        idx.register_video_audio("/videos/foo.mp4");
        idx.register_video_audio("/videos/foo.mp4");
        let frags = idx.build_split_fragments();
        assert!(frags.is_empty(), "frags={frags:?}");
    }

    #[test]
    fn no_split_fragments_for_single_reference() {
        let mut idx = InputIndex::new();
        idx.register_video_audio("/videos/bar.mp4");
        let frags = idx.build_split_fragments();
        assert!(frags.is_empty(), "{frags:?}");
    }

    // P1-A: compute_scene_len tests
    #[test]
    fn compute_scene_len_uses_scene_duration_when_set() {
        let scene = crate::project::Scene {
            id: "s1".into(),
            duration: Some(10.0),
            objects: vec![],
            ..Default::default()
        };
        assert_eq!(compute_scene_len(&scene, &HashMap::new()), 10.0);
    }

    #[test]
    fn compute_scene_len_uses_probe_when_duration_unset() {
        use crate::project::VideoObject;
        use crate::renderer::probe::ProbeResult;
        let scene = crate::project::Scene {
            id: "s1".into(),
            duration: None,
            objects: vec![crate::project::SceneObject::Video(VideoObject {
                id: "v1".into(),
                file: Some("/video.mp4".to_string()),
                x: 0, y: 0, width: 1920, height: 1080,
                start: 2.0, opacity: 100, volume: 100,
                ..Default::default()
            })],
            ..Default::default()
        };
        let mut probes = HashMap::new();
        probes.insert("/video.mp4".to_string(), ProbeResult {
            duration: 30.0, width: Some(1920), height: Some(1080),
            fps: Some(30.0), has_audio: false, sample_rate: None,
        });
        // scene_len = start(2.0) + probe.duration(30.0) = 32.0
        assert_eq!(compute_scene_len(&scene, &probes), 32.0);
    }

    #[test]
    fn build_filter_graph_empty_scene() {
        let project = crate::project::Project {
            version: 1,
            output_folder: "/tmp".into(),
            output: base_settings(),
            scenes: vec![crate::project::Scene {
                id: "s1".into(),
                duration: Some(5.0),
                objects: vec![],
                ..Default::default()
            }],
        };
        let graph = build_filter_graph(&project, &HashMap::new(), Path::new("/tmp/fonts")).unwrap();
        assert_eq!(graph.video_map, "[vout]");
        assert_eq!(graph.audio_map, "[aout]");
    }
}
