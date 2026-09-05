use std::path::Path;
use std::process::Command;

/// Media player interface for video placeholders
pub struct MediaPlayer;

impl MediaPlayer {
    /// Play a video file using FFmpeg (ffplay) with seamless borderless / fullscreen flags
    /// Fully compatible across Linux, macOS, and Windows
    pub fn play_video(source: &str) -> std::io::Result<()> {
        let video_path = Path::new(source);

        // 1. Check if ffplay is directly callable (cross-platform check)
        if let Ok(status) = Command::new("ffplay").arg("-version").output()
            && status.status.success()
        {
            let mut cmd = Command::new("ffplay");
            cmd.arg("-autoexit")
                .arg("-alwaysontop")
                .arg("-noborder")
                .arg("-fs")
                .arg("-window_title")
                .arg("Cargo Slide Video Player")
                .arg(video_path);

            let child = cmd.spawn()?;
            let _ = child.wait_with_output();
            return Ok(());
        }

        // 2. Fallback to mpv if available
        if let Ok(status) = Command::new("mpv").arg("--version").output()
            && status.status.success()
        {
            let mut cmd = Command::new("mpv");
            cmd.arg("--fs").arg(video_path);
            let child = cmd.spawn()?;
            let _ = child.wait_with_output();
            return Ok(());
        }

        // 3. Fallback to system default video opener (ShellExecute on Windows, open on macOS, xdg-open on Linux)
        open::that(source).map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(())
    }
}
