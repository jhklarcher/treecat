use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Kind of file for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Text,
    Binary,
}

// Common binary-ish extensions.
fn is_binary_ext(ext: &str) -> bool {
    matches!(
        ext,
        "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "webp"
            | "ico"
            | "zip"
            | "tar"
            | "gz"
            | "bz2"
            | "xz"
            | "7z"
            | "mp3"
            | "flac"
            | "ogg"
            | "mp4"
            | "mkv"
            | "avi"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "class"
            | "o"
            | "a"
            | "db"
            | "sqlite"
            | "db3"
    )
}

pub fn classify_path(path: &str) -> FileKind {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if is_binary_ext(ext.as_str()) {
        return FileKind::Binary;
    }
    classify_by_content(path).unwrap_or(FileKind::Binary)
}

fn classify_by_content(path: &str) -> Result<FileKind, std::io::Error> {
    let mut f = File::open(path)?;
    let mut buf = [0u8; 4096];
    let n = f.read(&mut buf)?;
    if n == 0 {
        return Ok(FileKind::Text);
    }
    let slice = &buf[..n];
    if slice.contains(&0) {
        return Ok(FileKind::Binary);
    }
    let printable = slice.iter().filter(|&&b| is_printable(b)).count();
    let ratio = printable as f32 / n as f32;
    if ratio < 0.75 {
        Ok(FileKind::Binary)
    } else {
        Ok(FileKind::Text)
    }
}

fn is_printable(b: u8) -> bool {
    matches!(b, b'\n' | b'\r' | b'\t' | 0x20..=0x7E)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_text_and_binary() {
        let dir = tempdir().unwrap();
        let text_path = dir.path().join("a.txt");
        fs::write(&text_path, b"hello world\n").unwrap();

        let bin_path = dir.path().join("b.bin");
        fs::write(&bin_path, [0u8, 1, 2, 3]).unwrap();

        assert_eq!(classify_path(text_path.to_str().unwrap()), FileKind::Text);
        assert_eq!(classify_path(bin_path.to_str().unwrap()), FileKind::Binary);
    }
}
