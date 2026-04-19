// filter.rs — filter_complex グラフ生成
//
// 設計参照: docs/design/03_renderer.md

use std::collections::HashMap;
use crate::project::{
    AudioObject, Entry, Format, ImageObject, LoopMode, OutputSettings, Project,
    Scene, SceneObject, TextObject, VariableValue, VideoObject,
};
use crate::renderer::escape::{escape_drawtext_value, escape_filter_value};
use crate::renderer::probe::ProbeResult;
use crate::renderer::codec::output_sample_rate;

// ──────────────────────────────────────────────────────────────────────────────
// 公開型
// ──────────────────────────────────────────────────────────────────────────────

/// filter_complex グラフ構築の出力
#[derive(Debug, Clone)]
pub struct FilterGraph {
    /// `-i <path>` の順序通りの入力ファイルリスト
    pub inputs: Vec<InputSpec>,
    /// `-filter_complex` に渡す文字列
    pub filter_complex: String,
    /// `-map` に渡すビデオストリームラベル（例: `"[vout]"` or `"[vfinal]"`）
    pub video_map: String,
    /// `-map` に渡すオーディオストリームラベル
    pub audio_map: String,
}

/// 一つの `-i` 入力を表す
#[derive(Debug, Clone)]
pub struct InputSpec {
    /// ファイルパス
    pub path: String,
    /// 画像オブジェクト用 `-loop 1 -t <duration>` フラグ群（Some の場合は先行 args として追加）
    pub image_flags: Option<ImageFlags>,
}

/// 静止画入力に必要な前置オプション
#[derive(Debug, Clone)]
pub struct ImageFlags {
    /// シーン内の表示時間（秒）
    pub duration: f64,
}

// ──────────────────────────────────────────────────────────────────────────────
// エントリ展開ヘルパー
// ──────────────────────────────────────────────────────────────────────────────

/// エントリの可変値をプロジェクトに展開してシーンリストを取得する
fn resolve_entry<'a>(project: &'a Project, entry: &'a Entry) -> ResolvedEntry<'a> {
    ResolvedEntry { project, entry }
}

struct ResolvedEntry<'a> {
    project: &'a Project,
    entry: &'a Entry,
}

impl<'a> ResolvedEntry<'a> {
    /// オブジェクトの実効ファイルパスを返す（variable=true なら entry から取得）
    fn resolve_file(&self, obj_id: &str, static_file: Option<&str>) -> Option<String> {
        if let Some(vv) = self.entry.variables.get(obj_id) {
            match vv {
                VariableValue::Media { file, .. } => return Some(file.clone()),
                VariableValue::Text { .. } => {}
            }
        }
        static_file.map(|s| s.to_string())
    }

    /// オブジェクトの実効テキストを返す
    fn resolve_text(&self, obj_id: &str, static_text: Option<&str>) -> Option<String> {
        if let Some(vv) = self.entry.variables.get(obj_id) {
            if let VariableValue::Text { text } = vv {
                return Some(text.clone());
            }
        }
        static_text.map(|s| s.to_string())
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// 入力ファイル重複排除
// ──────────────────────────────────────────────────────────────────────────────

/// ファイルパス → 入力インデックスのマップ（重複排除用）
struct InputIndex {
    /// path → (input_index, 参照回数)
    map: HashMap<String, (usize, usize)>,
    /// 順番通りの InputSpec
    specs: Vec<InputSpec>,
    /// path → split フィルタの指令层のテキスト（split が実行済みなら一意に決まっている）
    /// (n_refs, Vec<video_label_for_ref_k>, Vec<audio_label_for_ref_k>)
    split_labels: HashMap<String, SplitEntry>,
}

struct SplitEntry {
    input_idx: usize,
    /// k 番目の参照にあて込む映像ラベルリスト
    video_labels: Vec<String>,
    /// k 番目の参照にあて込む音声ラベルリスト
    audio_labels: Vec<String>,
}

impl InputIndex {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            specs: Vec::new(),
            split_labels: HashMap::new(),
        }
    }

    /// 通常の動画・音声ファイルを登録し、入力インデックスを返す
    fn register_video_audio(&mut self, path: &str) -> usize {
        if let Some((idx, count)) = self.map.get_mut(path) {
            *count += 1;
            *idx
        } else {
            let idx = self.specs.len();
            self.specs.push(InputSpec {
                path: path.to_string(),
                image_flags: None,
            });
            self.map.insert(path.to_string(), (idx, 1));
            idx
        }
    }

    /// 静止画ファイルを登録し、入力インデックスを返す（-loop / -t オプション付き）
    fn register_image(&mut self, path: &str, duration: f64) -> usize {
        // 静止画は同一パスでも異なる duration の場合があるため、重複排除しない
        let idx = self.specs.len();
        self.specs.push(InputSpec {
            path: path.to_string(),
            image_flags: Some(ImageFlags { duration }),
        });
        idx
    }

    /// パスの参照回数を返す（split フィルタ生成判定用）
    fn ref_count(&self, path: &str) -> usize {
        self.map.get(path).map(|(_, c)| *c).unwrap_or(0)
    }

    /// split フィルタ指令のフラグメントを生成する。
    /// 参照回数が 2 以上のパスについて `split=N` を指定するフラグメントリストを返す。
    fn build_split_fragments(&mut self) -> Vec<String> {
        let mut frags = Vec::new();
        for (path, (input_idx, ref_count)) in &self.map {
            if *ref_count < 2 {
                continue;
            }
            let n = *ref_count;
            let idx = *input_idx;

            // 映像ストリームの split
            let v_labels: Vec<String> = (0..n)
                .map(|k| format!("[split{idx}v{k}]"))
                .collect();
            let v_out = v_labels.join("");
            frags.push(format!("[{idx}:v]split={n}{v_out}"));

            // 音声ストリームの asplit
            let a_labels: Vec<String> = (0..n)
                .map(|k| format!("[split{idx}a{k}]"))
                .collect();
            let a_out = a_labels.join("");
            frags.push(format!("[{idx}:a]asplit={n}{a_out}"));

            self.split_labels.insert(
                path.clone(),
                SplitEntry {
                    input_idx: idx,
                    video_labels: v_labels,
                    audio_labels: a_labels,
                },
            );
        }
        frags
    }

    /// 入力データの映像ラベルを返す。
    /// split が必要な場合は split ラベルを、そうでない場合は元の `[idx:v]` を返す。
    fn video_label_for(&self, path: &str, use_count: usize) -> String {
        if let Some(se) = self.split_labels.get(path) {
            se.video_labels[use_count].clone()
        } else {
            let idx = self.map[path].0;
            format!("[{idx}:v]")
        }
    }

    /// 入力データの音声ラベルを返す。
    fn audio_label_for(&self, path: &str, use_count: usize) -> String {
        if let Some(se) = self.split_labels.get(path) {
            se.audio_labels[use_count].clone()
        } else {
            let idx = self.map[path].0;
            format!("[{idx}:a]")
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// メイン API
// ──────────────────────────────────────────────────────────────────────────────

/// プロジェクト・エントリ・各ファイルの ProbeResult から FilterGraph を構築する。
///
/// `probes` は `InputSpec.path` ごとの ffprobe 結果。バッチ処理側でファイルをスキャンして渡す。
pub fn build_filter_graph(
    project: &Project,
    entry: &Entry,
    probes: &HashMap<String, ProbeResult>,
) -> Result<FilterGraph, String> {
    let settings = &project.output;
    let scenes = &project.scenes;
    let resolved = resolve_entry(project, entry);

    let mut input_idx = InputIndex::new();
    // シーン → (video_label, audio_label) の事前収集
    // まず全シーンの入力を確定してから filter_complex を組む
    let scene_filters: Vec<SceneFilter> = scenes
        .iter()
        .enumerate()
        .map(|(si, scene)| {
            build_scene_filter(si, scene, settings, &resolved, probes, &mut input_idx)
        })
        .collect::<Result<_, _>>()?;

    // 全入力を確定した後に split フィルタフラグメントを生成
    let split_frags = input_idx.build_split_fragments();

    // filter_complex 文字列を組み立て
    let sample_rate = output_sample_rate(&settings.format);
    let filter_complex = assemble_filter_complex(&scene_filters, scenes, sample_rate, &split_frags);

    let (video_map, audio_map) = if scenes.len() == 1 {
        ("[vout]".to_string(), "[aout]".to_string())
    } else {
        ("[vfinal]".to_string(), "[afinal]".to_string())
    };

    Ok(FilterGraph {
        inputs: input_idx.specs,
        filter_complex,
        video_map,
        audio_map,
    })
}

// ──────────────────────────────────────────────────────────────────────────────
// シーン単位フィルタ
// ──────────────────────────────────────────────────────────────────────────────

struct SceneFilter {
    /// このシーンを構成する filter_complex フラグメント（セミコロンで接続済み）
    fragments: Vec<String>,
    /// シーンの映像出力ラベル（単一シーンでは "[vout]"、複数シーンでは "[s{n}v]"）
    video_out: String,
    /// シーンの音声出力ラベル
    audio_out: String,
}

fn build_scene_filter(
    si: usize,
    scene: &Scene,
    settings: &OutputSettings,
    resolved: &ResolvedEntry<'_>,
    probes: &HashMap<String, ProbeResult>,
    input_idx: &mut InputIndex,
) -> Result<SceneFilter, String> {
    let scene_len = scene.duration.unwrap_or(0.0);
    let w = settings.width;
    let h = settings.height;

    let mut frags: Vec<String> = Vec::new();

    // --- 映像系オブジェクト処理 ---
    // まず映像オブジェクトを収集して、ベースとなる映像トラックを作る
    let mut current_v_label = format!("[s{si}_bg]");

    // 背景（黒）生成
    frags.push(format!(
        "color=black:s={w}x{h}:d={scene_len:.6},format=yuva420p{current_v_label}"
    ));

    let mut audio_labels: Vec<String> = Vec::new();

    for (oi, obj) in scene.objects.iter().enumerate() {
        match obj {
            SceneObject::Video(v) => {
                let next_label = format!("[s{si}_v{oi}]");
                let frags_v = build_video_filter(
                    si, oi, v, settings, scene_len, resolved, probes, input_idx, &current_v_label, &next_label,
                    &mut audio_labels,
                )?;
                frags.extend(frags_v);
                current_v_label = next_label;
            }
            SceneObject::Image(img) => {
                let next_label = format!("[s{si}_v{oi}]");
                let frags_i = build_image_filter(
                    si, oi, img, scene_len, resolved, input_idx, &current_v_label, &next_label,
                )?;
                frags.extend(frags_i);
                current_v_label = next_label;
            }
            SceneObject::Text(txt) => {
                let next_label = format!("[s{si}_v{oi}]");
                let frags_t = build_text_filter(
                    si, oi, txt, scene_len, resolved, &current_v_label, &next_label,
                )?;
                frags.extend(frags_t);
                current_v_label = next_label;
            }
            SceneObject::Audio(aud) => {
                let label = format!("[s{si}_a{oi}]");
                let frags_a = build_audio_filter(
                    si, oi, aud, scene_len, settings, resolved, probes, input_idx, &label,
                )?;
                frags.extend(frags_a);
                audio_labels.push(label);
            }
        }
    }

    // 最終映像ラベルを [vout] または [s{n}v] にリラベル
    let video_out = if resolved.project.scenes.len() == 1 {
        "[vout]".to_string()
    } else {
        format!("[s{si}v]")
    };
    frags.push(format!("{current_v_label}null{video_out}"));

    // 音声ミックス
    let audio_out = if resolved.project.scenes.len() == 1 {
        "[aout]".to_string()
    } else {
        format!("[s{si}a]")
    };
    let audio_frag = build_amix(&audio_labels, scene_len, &audio_out);
    frags.push(audio_frag);

    Ok(SceneFilter {
        fragments: frags,
        video_out,
        audio_out,
    })
}

// ──────────────────────────────────────────────────────────────────────────────
// オブジェクト別フィルタ生成
// ──────────────────────────────────────────────────────────────────────────────

fn build_video_filter(
    si: usize,
    oi: usize,
    v: &VideoObject,
    settings: &OutputSettings,
    scene_len: f64,
    resolved: &ResolvedEntry<'_>,
    probes: &HashMap<String, ProbeResult>,
    input_idx: &mut InputIndex,
    base_label: &str,
    out_label: &str,
    audio_labels: &mut Vec<String>,
) -> Result<Vec<String>, String> {
    let file = resolved
        .resolve_file(&v.id, v.file.as_deref())
        .ok_or_else(|| format!("video object '{}': file が解決できません", v.id))?;

    let idx = input_idx.register_video_audio(&file);
    let probe = probes.get(&file);

    let obj_w = v.width;
    let obj_h = v.height;

    let video_input_label = format!("[{idx}:v]");
    let proc_label = format!("[s{si}_vraw{oi}]");

    // opacity 適用
    let opacity_frag = if v.opacity < 100 {
        let aa = v.opacity as f64 / 100.0;
        format!(
            "{video_input_label}scale={obj_w}:{obj_h}:flags=lanczos,format=yuva420p,colorchannelmixer=aa={aa:.6}{proc_label}"
        )
    } else {
        format!(
            "{video_input_label}scale={obj_w}:{obj_h}:flags=lanczos,format=yuva420p{proc_label}"
        )
    };

    let effective_duration = if scene_len > 0.0 { scene_len } else { 0.0 };
    let overlay_enable = if v.start > 0.0 || effective_duration > 0.0 {
        format!("enable='between(t,{:.6},{:.6})'", v.start, v.start + effective_duration)
    } else {
        String::new()
    };

    let overlay_frag = if overlay_enable.is_empty() {
        format!("{base_label}{proc_label}overlay={x}:{y}{out_label}", x = v.x, y = v.y)
    } else {
        format!(
            "{base_label}{proc_label}overlay={x}:{y}:{enable}{out_label}",
            x = v.x,
            y = v.y,
            enable = overlay_enable,
        )
    };

    let mut frags = vec![opacity_frag, overlay_frag];

    // 音声トラック
    if probe.map(|p| p.has_audio).unwrap_or(false) {
        let audio_label = format!("[s{si}_va{oi}]");
        let vol = v.volume as f64 / 100.0;
        frags.push(format!(
            "[{idx}:a]volume={vol:.6}{audio_label}"
        ));
        audio_labels.push(audio_label);
    }

    Ok(frags)
}

fn build_image_filter(
    si: usize,
    oi: usize,
    img: &ImageObject,
    scene_len: f64,
    resolved: &ResolvedEntry<'_>,
    input_idx: &mut InputIndex,
    base_label: &str,
    out_label: &str,
) -> Result<Vec<String>, String> {
    let file = resolved
        .resolve_file(&img.id, img.file.as_deref())
        .ok_or_else(|| format!("image object '{}': file が解決できません", img.id))?;

    let eff_dur = if img.duration == 0.0 { scene_len } else { img.duration };
    let idx = input_idx.register_image(&file, eff_dur);
    let proc_label = format!("[s{si}_iraw{oi}]");

    let opacity_frag = if img.opacity < 100 {
        let aa = img.opacity as f64 / 100.0;
        format!(
            "[{idx}:v]scale={w}:{h}:flags=lanczos,format=yuva420p,colorchannelmixer=aa={aa:.6}{proc_label}",
            w = img.width,
            h = img.height,
        )
    } else {
        format!(
            "[{idx}:v]scale={w}:{h}:flags=lanczos,format=yuva420p{proc_label}",
            w = img.width,
            h = img.height,
        )
    };

    let enable = format!("enable='between(t,{:.6},{:.6})'", img.start, img.start + eff_dur);
    let overlay_frag = format!(
        "{base_label}{proc_label}overlay={x}:{y}:{enable}{out_label}",
        x = img.x,
        y = img.y,
    );

    Ok(vec![opacity_frag, overlay_frag])
}

fn build_text_filter(
    si: usize,
    oi: usize,
    txt: &TextObject,
    scene_len: f64,
    resolved: &ResolvedEntry<'_>,
    base_label: &str,
    out_label: &str,
) -> Result<Vec<String>, String> {
    let text_val = resolved
        .resolve_text(&txt.id, txt.text.as_deref())
        .ok_or_else(|| format!("text object '{}': text が解決できません", txt.id))?;

    let eff_dur = if txt.duration == 0.0 { scene_len } else { txt.duration };
    // pt → px 変換: round(pt × 96/72)
    let font_px = pt_to_px(txt.font_size);

    let escaped_text = escape_drawtext_value(&text_val);
    let escaped_font = escape_filter_value(&txt.font);
    let color_hex = txt.color.trim_start_matches('#');

    let mut drawtext_args = format!(
        "fontfile={escaped_font}:text='{escaped_text}':x={x}:y={y}:fontsize={font_px}:fontcolor=0x{color_hex}",
        x = txt.x,
        y = txt.y,
    );

    if let Some(bg) = &txt.background_color {
        let bg_hex = bg.trim_start_matches('#');
        drawtext_args.push_str(&format!(":box=1:boxcolor=0x{bg_hex}"));
    }

    drawtext_args.push_str(&format!(
        ":enable='between(t,{:.6},{:.6})'",
        txt.start,
        txt.start + eff_dur,
    ));

    let frag = format!("{base_label}drawtext={drawtext_args}{out_label}");
    Ok(vec![frag])
}

fn build_audio_filter(
    si: usize,
    oi: usize,
    aud: &AudioObject,
    scene_len: f64,
    settings: &OutputSettings,
    resolved: &ResolvedEntry<'_>,
    probes: &HashMap<String, ProbeResult>,
    input_idx: &mut InputIndex,
    out_label: &str,
) -> Result<Vec<String>, String> {
    let file = resolved
        .resolve_file(&aud.id, aud.file.as_deref())
        .ok_or_else(|| format!("audio object '{}': file が解決できません", aud.id))?;

    let idx = input_idx.register_video_audio(&file);
    let probe = probes.get(&file);
    let sample_rate = output_sample_rate(&settings.format);
    let file_duration = probe.map(|p| p.duration).unwrap_or(scene_len);
    let file_sample_rate = probe
        .and_then(|p| p.sample_rate)
        .unwrap_or(sample_rate);

    let eff_dur = if aud.duration == 0.0 { scene_len } else { aud.duration };
    let vol = aud.volume as f64 / 100.0;
    let delay_ms = (aud.start * 1000.0).round() as i64;

    let mut chain = format!(
        "[{idx}:a]aformat=sample_rates={sample_rate}:channel_layouts=stereo,volume={vol:.6}"
    );

    // fade_in / fade_out
    if let Some(fi) = aud.fade_in {
        chain.push_str(&format!(",afade=in:st={:.6}:d={:.6}", aud.start, fi));
    }
    if let Some(fo) = aud.fade_out {
        let fade_start = scene_len - fo;
        chain.push_str(&format!(",afade=out:st={:.6}:d={:.6}", fade_start, fo));
    }

    // loop or silence
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
// amix
// ──────────────────────────────────────────────────────────────────────────────

fn build_amix(audio_labels: &[String], scene_len: f64, out_label: &str) -> String {
    match audio_labels.len() {
        0 => {
            // N=0: anullsrc
            format!("anullsrc=r=44100:cl=stereo,atrim=duration={scene_len:.6}{out_label}")
        }
        1 => {
            // N=1: amix 不要、ラベルを直接リラベル
            format!("{}anull{out_label}", audio_labels[0])
        }
        n => {
            let inputs_str = audio_labels.join("");
            format!("{inputs_str}amix=inputs={n}:duration=longest{out_label}")
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// concat
// ──────────────────────────────────────────────────────────────────────────────

fn assemble_filter_complex(
    scene_filters: &[SceneFilter],
    scenes: &[Scene],
    sample_rate: u32,
    split_frags: &[String],
) -> String {
    let mut parts: Vec<String> = Vec::new();

    // split フィルタは最前に追加
    parts.extend_from_slice(split_frags);

    for sf in scene_filters {
        parts.extend(sf.fragments.iter().cloned());
    }

    if scene_filters.len() > 1 {
        // aresample でサンプルレートを統一してから concat
        let mut concat_inputs = String::new();
        for (i, sf) in scene_filters.iter().enumerate() {
            let ar_out = format!("[s{i}ar]");
            parts.push(format!("{}aresample={sample_rate}{ar_out}", sf.audio_out));
            concat_inputs.push_str(&sf.video_out);
            concat_inputs.push_str(&ar_out);
        }
        let n = scene_filters.len();
        parts.push(format!(
            "{concat_inputs}concat=n={n}:v=1:a=1[vfinal][afinal]"
        ));
    }

    parts.join(";")
}

// ──────────────────────────────────────────────────────────────────────────────
// ヘルパー
// ──────────────────────────────────────────────────────────────────────────────

/// フォントサイズを pt から px に変換する（96dpi 基準: px = round(pt × 96/72)）
pub fn pt_to_px(pt: u32) -> u32 {
    ((pt as f64 * 96.0 / 72.0).round()) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Codec, Format, OutputSettings, TextAlign};

    fn base_settings() -> OutputSettings {
        OutputSettings {
            width: 1920,
            height: 1080,
            fps: 30,
            codec: Codec::H264,
            format: Format::Mp4,
            crf: 23,
            preset: "medium".into(),
        }
    }

    // ── pt_to_px ──────────────────────────────────────────────────────────────

    #[test]
    fn pt_to_px_48pt_is_64px() {
        assert_eq!(pt_to_px(48), 64);
    }

    #[test]
    fn pt_to_px_24pt_is_32px() {
        assert_eq!(pt_to_px(24), 32);
    }

    #[test]
    fn pt_to_px_1pt_is_1px() {
        assert_eq!(pt_to_px(1), 1);
    }

    // ── amix ──────────────────────────────────────────────────────────────────

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
        assert!(frag.contains("[s0_a0]"), "frag={frag}");
        assert!(!frag.contains("amix"), "amix は N=1 で使わない: frag={frag}");
    }

    #[test]
    fn amix_two_audio_uses_amix_inputs_2() {
        let labels = vec!["[s0_a0]".to_string(), "[s0_a1]".to_string()];
        let frag = build_amix(&labels, 5.0, "[aout]");
        assert!(frag.contains("amix=inputs=2"), "frag={frag}");
    }

    // ── InputIndex ・ split フィルタ ────────────────────────────────────────────

    #[test]
    fn same_file_registered_once_in_inputs() {
        let mut idx = InputIndex::new();
        let i1 = idx.register_video_audio("/videos/foo.mp4");
        let i2 = idx.register_video_audio("/videos/foo.mp4");
        // 同じインデックスが返す
        assert_eq!(i1, i2);
        // 入力リストには 1 件だけ
        assert_eq!(idx.specs.len(), 1);
        // 参照回数は 2
        assert_eq!(idx.ref_count("/videos/foo.mp4"), 2);
    }

    #[test]
    fn split_fragments_generated_for_duplicate_file() {
        let mut idx = InputIndex::new();
        idx.register_video_audio("/videos/foo.mp4");
        idx.register_video_audio("/videos/foo.mp4");
        let frags = idx.build_split_fragments();
        // split と asplit の 2 フラグメントが生成される
        assert_eq!(frags.len(), 2, "frags={frags:?}");
        assert!(frags[0].contains("split=2"), "video split: {}", frags[0]);
        assert!(frags[1].contains("asplit=2"), "audio asplit: {}", frags[1]);
    }

    #[test]
    fn no_split_fragments_for_single_reference() {
        let mut idx = InputIndex::new();
        idx.register_video_audio("/videos/bar.mp4");
        let frags = idx.build_split_fragments();
        assert!(frags.is_empty(), "1回参照で split は不要: {frags:?}");
    }
}
