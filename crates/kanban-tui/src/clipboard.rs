use std::io;

pub fn copy_to_clipboard(text: &str) -> io::Result<()> {
    let mut clipboard = arboard::Clipboard::new().map_err(io::Error::other)?;

    #[cfg(target_os = "linux")]
    {
        use arboard::SetExtLinux;
        use std::time::{Duration, Instant};

        // wait_until gives clipboard managers time to take ownership
        // This prevents clipboard clearing when our app exits
        clipboard
            .set()
            .wait_until(Instant::now() + Duration::from_millis(250))
            .text(text.to_owned())
            .map_err(io::Error::other)
    }

    #[cfg(not(target_os = "linux"))]
    {
        clipboard.set_text(text).map_err(io::Error::other)
    }
}
