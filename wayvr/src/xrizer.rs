//! Xrizer interop: read/write `xrizer_scale` / `xrizer_brightness` files.
//!
//! Mirrors `xrizer/src/settings.rs` candidate lookup + clamp logic,
//! but with the UI ranges requested for the WayVR watch:
//! scale 0.01..=1.4, brightness 0.00..=1.00.
//! Files contain plain float text like `1.000`. xrizer polls them (~500ms),
//! so writing here takes effect live without restart.

use std::path::PathBuf;

pub const SCALE_MIN: f32 = 0.01;
pub const SCALE_MAX: f32 = 1.4;
pub const SCALE_DEFAULT: f32 = 1.0;

pub const BRIGHTNESS_MIN: f32 = 0.0;
pub const BRIGHTNESS_MAX: f32 = 1.0;
pub const BRIGHTNESS_DEFAULT: f32 = 1.0;

fn scale_file_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("XRIZER_SCALE_FILE") {
        v.push(p.into());
    }
    if let Ok(d) = std::env::var("XDG_RUNTIME_DIR") {
        v.push(PathBuf::from(d).join("xrizer_scale"));
    }
    if let Ok(h) = std::env::var("HOME") {
        v.push(PathBuf::from(h.clone()).join(".config/xrizer_scale"));
        v.push(PathBuf::from(h).join(".local/share/xrizer_scale"));
    }
    v.push(PathBuf::from("/tmp/xrizer_scale"));
    v
}

fn brightness_file_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("XRIZER_BRIGHTNESS_FILE") {
        v.push(p.into());
    }
    if let Ok(d) = std::env::var("XDG_RUNTIME_DIR") {
        v.push(PathBuf::from(d).join("xrizer_brightness"));
    }
    if let Ok(h) = std::env::var("HOME") {
        v.push(PathBuf::from(h.clone()).join(".config/xrizer_brightness"));
        v.push(PathBuf::from(h).join(".local/share/xrizer_brightness"));
    }
    v.push(PathBuf::from("/tmp/xrizer_brightness"));
    v
}

fn read_first_float(paths: &[PathBuf], default: f32) -> Option<f32> {
    for path in paths {
        if let Ok(s) = std::fs::read_to_string(path)
            && let Ok(val) = s.trim().parse::<f32>()
        {
            return Some(val);
        }
    }
    let _ = default;
    None
}

fn write_all(paths: &[PathBuf], text: &str) {
    for path in paths {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(path, text) {
            log::warn!("xrizer: failed to write {}: {e:?}", path.display());
        }
    }
}

/// Current scale, clamped to 0.01..=1.4. Missing/unparseable file -> 1.0.
pub fn read_scale() -> f32 {
    match read_first_float(&scale_file_candidates(), SCALE_DEFAULT) {
        Some(raw) => {
            // Compat with xrizer: values > 4.0 are percent (e.g. 100 -> 1.0).
            let v = if raw > 4.0 { raw / 100.0 } else { raw };
            v.clamp(SCALE_MIN, SCALE_MAX)
        }
        None => SCALE_DEFAULT,
    }
}

/// Persist scale to all candidate locations (terminal sink: plain file write).
pub fn write_scale(value: f32) {
    let clamped = value.clamp(SCALE_MIN, SCALE_MAX);
    write_all(&scale_file_candidates(), &format!("{clamped:.3}"));
}

/// Current brightness, clamped to 0.0..=1.0. Missing/unparseable file -> 1.0.
pub fn read_brightness() -> f32 {
    match read_first_float(&brightness_file_candidates(), BRIGHTNESS_DEFAULT) {
        Some(raw) => {
            let v = if raw > 1.0 { raw / 100.0 } else { raw };
            v.clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX)
        }
        None => BRIGHTNESS_DEFAULT,
    }
}

/// Persist brightness to all candidate locations.
pub fn write_brightness(value: f32) {
    let clamped = value.clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX);
    write_all(
        &brightness_file_candidates(),
        &format!("{clamped:.3}"),
    );
}
