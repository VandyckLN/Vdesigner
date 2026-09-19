use crate::export::{resolve_output_path, write_bytes};
use crate::session::{Session, SourceImage};
use base64::Engine;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use vdesigner_core::{
    decode, is_svg, rasterize_svg, run_job, run_preview, EncodeSpec, Job, OutputFormat,
};

const PREVIEW_MAX_SIDE: u32 = 2048;

/// The size estimate re-encodes the job in the real output format, which AVIF
/// makes expensive. A much smaller sample keeps that affordable on every
/// control change; the result is extrapolated back up.
const ESTIMATE_MAX_SIDE: u32 = 512;

/// Event name the export progress is published under.
pub const EXPORT_PROGRESS_EVENT: &str = "export-progress";

#[derive(Serialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub file_stem: String,
    pub preview_png_base64: String,
}

#[derive(Serialize)]
pub struct PreviewResult {
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize)]
pub struct ExportResult {
    pub path: String,
    pub bytes_written: u64,
}

#[derive(Serialize)]
pub struct EstimateResult {
    pub bytes: u64,
}

/// One stage of an export, as the engine reports it.
#[derive(Serialize, Clone)]
pub struct ExportProgress {
    pub current: u32,
    pub total: u32,
    pub label: String,
}

/// Every command returns a plain string on failure, because the UI only ever
/// displays the message. The engine keeps the typed errors.
type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub fn open_image(path: String, session: State<Session>) -> CommandResult<ImageInfo> {
    let bytes = std::fs::read(&path).map_err(|e| format!("não foi possível ler o arquivo: {e}"))?;

    let image = if is_svg(&bytes) {
        rasterize_svg(&bytes, None, None).map_err(|e| e.to_string())?
    } else {
        decode(&bytes).map_err(|e| e.to_string())?
    };

    let file_stem = PathBuf::from(&path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "imagem".to_string());

    let info = ImageInfo {
        width: image.width(),
        height: image.height(),
        file_stem: file_stem.clone(),
        preview_png_base64: preview_png(&bytes)?,
    };

    *session
        .source
        .lock()
        .map_err(|_| "estado corrompido".to_string())? = Some(SourceImage {
        bytes,
        width: image.width(),
        height: image.height(),
        file_stem,
    });

    Ok(info)
}

#[tauri::command]
pub fn preview(job: Job, session: State<Session>) -> CommandResult<PreviewResult> {
    let guard = session
        .source
        .lock()
        .map_err(|_| "estado corrompido".to_string())?;
    let source = guard.as_ref().ok_or("nenhuma imagem aberta")?;

    // The preview is always served as PNG, so the canvas shows exactly the
    // pixels the pipeline produced, without a second lossy pass.
    let png_job = Job {
        steps: job.steps.clone(),
        output: EncodeSpec {
            format: OutputFormat::Png,
            quality: 100,
            lossless: true,
        },
    };
    let output =
        run_preview(&source.bytes, &png_job, PREVIEW_MAX_SIDE).map_err(|e| e.to_string())?;

    Ok(PreviewResult {
        png_base64: base64::engine::general_purpose::STANDARD.encode(&output.bytes),
        width: output.width,
        height: output.height,
    })
}

/// Estimates the exported file size without paying for a full-resolution
/// encode. The job runs in its real output format over a small copy, and the
/// byte count is scaled back up by the reduction — so the number is an
/// approximation, and the UI presents it as one.
#[tauri::command]
pub fn estimate(job: Job, session: State<Session>) -> CommandResult<EstimateResult> {
    let guard = session
        .source
        .lock()
        .map_err(|_| "estado corrompido".to_string())?;
    let source = guard.as_ref().ok_or("nenhuma imagem aberta")?;

    let sample = run_preview(&source.bytes, &job, ESTIMATE_MAX_SIDE).map_err(|e| e.to_string())?;

    Ok(EstimateResult {
        bytes: extrapolate(
            sample.bytes.len() as u64,
            source.width,
            source.height,
            ESTIMATE_MAX_SIDE,
        ),
    })
}

/// Scales a sample's byte count back to full resolution. The sample came from
/// a copy whose longest side was capped at `max_side`, so it holds the square
/// of that reduction fewer pixels. Compressed size is not exactly linear in
/// pixel count, which is the reason the caller labels the result an estimate.
pub fn extrapolate(sample_bytes: u64, width: u32, height: u32, max_side: u32) -> u64 {
    let longest = width.max(height);
    if max_side == 0 || longest <= max_side {
        return sample_bytes;
    }

    let ratio = longest as f64 / max_side as f64;
    (sample_bytes as f64 * ratio * ratio).round() as u64
}

#[tauri::command]
pub fn export(
    app: AppHandle,
    job: Job,
    output_dir: String,
    file_stem: String,
    overwrite: bool,
    session: State<Session>,
) -> CommandResult<ExportResult> {
    let guard = session
        .source
        .lock()
        .map_err(|_| "estado corrompido".to_string())?;
    let source = guard.as_ref().ok_or("nenhuma imagem aberta")?;

    let extension = extension_for(job.output.format);
    let path = resolve_output_path(
        std::path::Path::new(&output_dir),
        &file_stem,
        extension,
        overwrite,
    )
    .map_err(|e| e.to_string())?;

    // A failed emit must not fail the export: the file still gets written, and
    // the UI simply stops hearing about the stages.
    let output = run_job(&source.bytes, &job, &mut |p| {
        let _ = app.emit(
            EXPORT_PROGRESS_EVENT,
            ExportProgress {
                current: p.current,
                total: p.total,
                label: p.label.to_string(),
            },
        );
    })
    .map_err(|e| e.to_string())?;
    let bytes_written = write_bytes(&path, &output.bytes).map_err(|e| e.to_string())?;

    Ok(ExportResult {
        path: path.display().to_string(),
        bytes_written,
    })
}

fn extension_for(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::WebP => "webp",
        OutputFormat::Avif => "avif",
        OutputFormat::Png => "png",
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Tiff => "tiff",
        OutputFormat::Ico => "ico",
    }
}

fn preview_png(bytes: &[u8]) -> CommandResult<String> {
    let job = Job {
        steps: vec![],
        output: EncodeSpec {
            format: OutputFormat::Png,
            quality: 100,
            lossless: true,
        },
    };
    let output = run_preview(bytes, &job, PREVIEW_MAX_SIDE).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&output.bytes))
}
