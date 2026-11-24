use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct FrameSlot {
    width: usize,
    height: usize,
    rgb: Vec<u8>,
    seq: u64,
}

/// Ultra-low-RAM background media renderer for local photos and videos.
/// - Photos are decoded once via `image` (clamped to 1920x1080) and uploaded to GPU.
/// - Videos stream frame-by-frame via `ffmpeg` into a SINGLE reused RGBA buffer (~3.5 MB RAM)
///   and update ONE GPU texture in-place (zero Chromium/WebView2 140 MB decoder pools).
pub struct BackgroundMedia {
    texture: Option<TextureHandle>,
    active_path: Option<PathBuf>,
    latest_frame: Arc<Mutex<Option<FrameSlot>>>,
    last_uploaded_seq: u64,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    pub last_error: Option<String>,
}

impl BackgroundMedia {
    pub fn new() -> Self {
        Self {
            texture: None,
            active_path: None,
            latest_frame: Arc::new(Mutex::new(None)),
            last_uploaded_seq: 0,
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
            last_error: None,
        }
    }

    pub fn clear(&mut self) {
        self.stop_video();
        self.texture = None;
        self.active_path = None;
        self.last_error = None;
    }

    fn stop_video(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
        self.stop = Arc::new(AtomicBool::new(false));
        if let Ok(mut g) = self.latest_frame.lock() {
            *g = None;
        }
        self.last_uploaded_seq = 0;
    }

    pub fn load_image(&mut self, ctx: &Context, path: &Path) -> Result<(), String> {
        if self.active_path.as_deref() == Some(path) && self.texture.is_some() {
            return Ok(());
        }
        self.stop_video();
        self.last_error = None;

        let img = image::open(path)
            .map_err(|e| format!("Failed decoding image {}: {e}", path.display()))?;
        let resized = if img.width() > 1920 || img.height() > 1080 {
            img.thumbnail(1920, 1080)
        } else {
            img
        };
        let rgba = resized.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color_img = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        self.texture = Some(ctx.load_texture("kissa-bg-media", color_img, TextureOptions::LINEAR));
        self.active_path = Some(path.to_path_buf());
        Ok(())
    }

    pub fn load_video(&mut self, path: &Path, fps_cap: u32) -> Result<(), String> {
        if self.active_path.as_deref() == Some(path) && self.worker.is_some() {
            return Ok(());
        }
        self.stop_video();
        self.texture = None;
        self.last_error = None;

        // Probe dimensions using ffprobe (or default 854x480 if unavailable)
        let (target_w, target_h) = probe_video_dims(path).unwrap_or((854, 480));
        let fps = fps_cap.clamp(12, 24);

        let mut cmd = Command::new("ffmpeg");
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-threads",
            "1",
            "-filter_threads",
            "1",
            "-probesize",
            "32768",
            "-analyzeduration",
            "100000",
            "-stream_loop",
            "-1",
            "-re",
            "-i",
        ])
        .arg(path)
        .args([
            "-an",
            "-sn",
            "-vf",
            &format!("scale={target_w}:{target_h}:flags=fast_bilinear,fps={fps}"),
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

        let mut child: Child = cmd
            .spawn()
            .map_err(|e| format!("Local video requires ffmpeg in PATH (not found: {e})"))?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed capturing ffmpeg video stream".to_string())?;

        let frame_bytes = target_w * target_h * 3;
        let stop = Arc::clone(&self.stop);
        let slot = Arc::clone(&self.latest_frame);

        let worker = thread::Builder::new()
            .name("kissa-video-stream".into())
            .spawn(move || {
                let mut read_buf = vec![0u8; frame_bytes];
                let mut seq: u64 = 0;

                while !stop.load(Ordering::Relaxed) {
                    if stdout.read_exact(&mut read_buf).is_err() {
                        break;
                    }
                    seq += 1;
                    if let Ok(mut guard) = slot.lock() {
                        if let Some(existing) = guard.as_mut() {
                            existing.rgb.copy_from_slice(&read_buf);
                            existing.seq = seq;
                        } else {
                            *guard = Some(FrameSlot {
                                width: target_w,
                                height: target_h,
                                rgb: read_buf.clone(),
                                seq,
                            });
                        }
                    }
                }
                let _ = child.kill();
                let _ = child.wait();
            })
            .map_err(|e| format!("Failed spawning video worker: {e}"))?;

        self.worker = Some(worker);
        self.active_path = Some(path.to_path_buf());
        Ok(())
    }

    /// Uploads any new streamed video frame in-place to the single GPU texture.
    pub fn poll_video_frame(&mut self, ctx: &Context) {
        let Ok(guard) = self.latest_frame.lock() else {
            return;
        };
        let Some(frame) = guard.as_ref() else {
            return;
        };
        if frame.seq == self.last_uploaded_seq {
            return;
        }
        let color_img = ColorImage::from_rgb([frame.width, frame.height], &frame.rgb);
        self.last_uploaded_seq = frame.seq;
        drop(guard);

        if let Some(tex) = &mut self.texture {
            tex.set(color_img, TextureOptions::LINEAR);
        } else {
            self.texture =
                Some(ctx.load_texture("kissa-bg-media", color_img, TextureOptions::LINEAR));
        }
    }

    pub fn texture(&self) -> Option<&TextureHandle> {
        self.texture.as_ref()
    }
}

impl Drop for BackgroundMedia {
    fn drop(&mut self) {
        self.stop_video();
    }
}

fn probe_video_dims(path: &Path) -> Option<(usize, usize)> {
    let mut cmd = Command::new("ffprobe");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
        ])
        .arg(path)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (w_s, h_s) = text.trim().split_once('x')?;
    let w: f32 = w_s.parse().ok()?;
    let h: f32 = h_s.parse().ok()?;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    // Cap video decode resolution at 960px wide (~2 MB RGBA frame) for ultra-low RAM
    let scale = (960.0 / w).min(540.0 / h).min(1.0);
    let tw = ((w * scale) as usize / 2) * 2;
    let th = ((h * scale) as usize / 2) * 2;
    Some((tw.max(160), th.max(90)))
}
