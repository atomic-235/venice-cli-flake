use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn plain(text: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

pub fn open_file(path: &Path) {
    if try_spawn("imv", &[path.as_os_str()]) || try_spawn("xdg-open", &[path.as_os_str()]) {
        return;
    }
    eprintln!("Install imv or xdg-open to auto-open images");
}

pub fn play_audio(path: &Path) {
    if !try_spawn("mpv", &[std::ffi::OsStr::new("--force-window"), path.as_os_str()]) {
        eprintln!("Install mpv to auto-play audio");
    }
}

fn try_spawn(program: &str, args: &[&std::ffi::OsStr]) -> bool {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn().is_ok()
}

pub fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/png",
    }
}
