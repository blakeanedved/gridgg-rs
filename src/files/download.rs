//! The [`Download`] response body and its helpers.

use std::hash::{BuildHasher, RandomState};
use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use tokio::io::AsyncWriteExt;

use crate::error::{Error, Result};

/// A successful File Download response whose body has not been read yet.
///
/// Consume it with [`bytes`](Self::bytes), [`bytes_stream`](Self::bytes_stream)
/// or [`save_to`](Self::save_to). A failure while the body is being read is
/// returned as [`Error::Transport`] and is not retried.
#[derive(Debug)]
pub struct Download {
    response: reqwest::Response,
    file_name: String,
    content_length: Option<u64>,
    content_type: Option<String>,
}

impl Download {
    pub(crate) fn new(response: reqwest::Response, fallback_name: &str) -> Self {
        let headers = response.headers();
        let file_name = headers
            .get(CONTENT_DISPOSITION)
            .and_then(|value| disposition_file_name(&String::from_utf8_lossy(value.as_bytes())))
            .and_then(|name| sanitize_file_name(&name))
            .unwrap_or_else(|| fallback_name.replace(['/', '\\', ':'], "_"));
        let content_type = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let content_length = response.content_length();
        Self {
            response,
            file_name,
            content_length,
            content_type,
        }
    }

    /// The file name from the `Content-Disposition` header, or one derived
    /// from the request (for example `123-events-grid.zip`) if the header is
    /// missing or has no usable name.
    ///
    /// Only the final path component of the server's name is kept, and a
    /// name containing `:` (a Windows drive prefix or NTFS stream) is
    /// replaced by the derived one, so it is safe to join onto a directory.
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The body length in bytes, if the server sent it.
    #[must_use]
    pub fn content_length(&self) -> Option<u64> {
        self.content_length
    }

    /// The `Content-Type` header, if present and valid text.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Reads the whole body into memory.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if reading the body fails.
    pub async fn bytes(self) -> Result<Bytes> {
        self.response.bytes().await.map_err(Error::transport)
    }

    /// Streams the body in chunks as they arrive. Pin the stream (for example
    /// with [`std::pin::pin!`]) to poll it.
    ///
    /// Each item is a chunk, or [`Error::Transport`] if reading failed.
    pub fn bytes_stream(self) -> impl Stream<Item = Result<Bytes>> + Send + 'static {
        self.response
            .bytes_stream()
            .map(|chunk| chunk.map_err(Error::transport))
    }

    /// Streams the body to `path` and returns the number of bytes written.
    ///
    /// The body is written to a hidden temporary file in the same directory,
    /// which is renamed to `path` (replacing any existing file) only once the
    /// whole body has arrived. If anything fails, or the future is dropped,
    /// the temporary file is removed and `path` is left untouched.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if reading the body fails,
    /// [`Error::Io`] if writing or renaming fails, and [`Error::Config`] if
    /// `path` has no file name.
    pub async fn save_to(mut self, path: impl AsRef<Path>) -> Result<u64> {
        let path = path.as_ref();
        let name = path.file_name().ok_or_else(|| {
            Error::Config(format!("save_to path has no file name: {}", path.display()))
        })?;
        let dir = match path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir,
            _ => Path::new("."),
        };
        let random = RandomState::new().hash_one(0u8);
        let temp_path = dir.join(format!(".{}.{random:016x}.part", name.to_string_lossy()));
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .await
            .map_err(Error::Io)?;
        // Removes the temporary file on every exit except a successful rename.
        let mut guard = TempPath(Some(temp_path.clone()));
        // `write_body` owns the file, so it is closed before the guard runs.
        let written = write_body(&mut self.response, file).await?;
        tokio::fs::rename(&temp_path, path)
            .await
            .map_err(Error::Io)?;
        guard.0 = None;
        Ok(written)
    }

    /// Reads the body and unpacks it as a zipped JSONL archive; see
    /// [`parse_events_archive`]. Decompression runs on Tokio's blocking pool.
    /// The whole body and every decoded value are held in memory, with no
    /// size limit.
    ///
    /// # Errors
    ///
    /// As for [`bytes`](Self::bytes) and [`parse_events_archive`].
    #[cfg(feature = "zip")]
    pub async fn events_archive<T>(self) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let bytes = self.bytes().await?;
        match tokio::task::spawn_blocking(move || parse_events_archive(&bytes)).await {
            Ok(result) => result,
            Err(err) if err.is_panic() => std::panic::resume_unwind(err.into_panic()),
            Err(err) => Err(Error::Io(std::io::Error::other(err))),
        }
    }

    /// Reads the body of a [`Files::grid_events`](super::Files::grid_events)
    /// download and unpacks it into Series Events
    /// [`Transaction`](crate::events::Transaction)s. Requires the `zip` and
    /// `events` features.
    ///
    /// # Errors
    ///
    /// As for [`events_archive`](Self::events_archive).
    #[cfg(all(feature = "zip", feature = "events"))]
    pub async fn grid_events_transactions(self) -> Result<Vec<crate::events::Transaction>> {
        self.events_archive().await
    }
}

/// Writes the rest of the body to `file`, then flushes and syncs it.
async fn write_body(response: &mut reqwest::Response, mut file: tokio::fs::File) -> Result<u64> {
    let mut written = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(Error::transport)? {
        file.write_all(&chunk).await.map_err(Error::Io)?;
        written += chunk.len() as u64;
    }
    file.flush().await.map_err(Error::Io)?;
    file.sync_all().await.map_err(Error::Io)?;
    Ok(written)
}

/// A temporary file path, removed on drop unless cleared.
struct TempPath(Option<PathBuf>);

impl Drop for TempPath {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Unpacks a zip archive of JSONL files (such as the one from
/// [`Files::grid_events`](super::Files::grid_events)) into one `T` per line.
///
/// Every file entry is parsed in archive order and the values are
/// concatenated; directory entries are skipped and blank lines are ignored.
/// GRID documents the archive only as "zipped Series Events JSONL", so the
/// entry layout is an assumption (unverified live). Use
/// [`serde_json::Value`] for `T` to inspect an archive.
///
/// The whole archive and all decoded values are held in memory, with no
/// size limit. This is CPU-bound; in async code prefer
/// [`Download::events_archive`].
///
/// # Errors
///
/// Returns [`Error::Archive`] if the bytes are not a readable zip archive
/// and [`Error::Decode`] if an entry cannot be decompressed or a line is not
/// a valid `T`.
#[cfg(feature = "zip")]
pub fn parse_events_archive<T: serde::de::DeserializeOwned>(archive: &[u8]) -> Result<Vec<T>> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(Error::Archive)?;
    let mut values = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(Error::Archive)?;
        if entry.is_dir() {
            continue;
        }
        let reader = std::io::BufReader::new(entry);
        for value in serde_json::Deserializer::from_reader(reader).into_iter::<T>() {
            values.push(value.map_err(Error::Decode)?);
        }
    }
    Ok(values)
}

/// Extracts the file name from a `Content-Disposition` value, preferring the
/// RFC 6266 `filename*` parameter over `filename`.
fn disposition_file_name(value: &str) -> Option<String> {
    let (_, mut rest) = value.split_once(';')?;
    let mut plain = None;
    let mut extended = None;
    loop {
        rest = rest.trim_start_matches(|c: char| c == ';' || c.is_whitespace());
        if rest.is_empty() {
            break;
        }
        let Some(eq) = rest
            .find(['=', ';'])
            .filter(|&i| rest.as_bytes()[i] == b'=')
        else {
            // A parameter without a value: skip to the next one.
            rest = rest.find(';').map_or("", |i| &rest[i..]);
            continue;
        };
        let name = rest[..eq].trim().to_ascii_lowercase();
        let after = rest[eq + 1..].trim_start();
        let (param, remaining) = if let Some(quoted) = after.strip_prefix('"') {
            unquote(quoted)
        } else {
            let end = after.find(';').unwrap_or(after.len());
            (after[..end].trim().to_owned(), &after[end..])
        };
        match name.as_str() {
            "filename" => plain = Some(param),
            "filename*" => extended = decode_extended(&param),
            _ => {}
        }
        rest = remaining;
    }
    extended.or(plain)
}

/// Reads a quoted string (after its opening quote), returning the unescaped
/// value and the input after the parameter.
fn unquote(input: &str) -> (String, &str) {
    let mut value = String::new();
    let mut chars = input.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => {
                if let Some((_, escaped)) = chars.next() {
                    value.push(escaped);
                }
            }
            '"' => {
                let after = &input[i + 1..];
                let next = after.find(';').unwrap_or(after.len());
                return (value, &after[next..]);
            }
            c => value.push(c),
        }
    }
    (value, "")
}

/// Decodes an RFC 8187 extended value (`charset'language'percent-encoded`).
fn decode_extended(value: &str) -> Option<String> {
    let mut parts = value.splitn(3, '\'');
    let charset = parts.next()?.to_ascii_lowercase();
    let _language = parts.next()?;
    let encoded = parts.next()?.as_bytes();
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut i = 0;
    while i < encoded.len() {
        if encoded[i] == b'%' {
            let hex = std::str::from_utf8(encoded.get(i + 1..i + 3)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            bytes.push(encoded[i]);
            i += 1;
        }
    }
    match charset.as_str() {
        "utf-8" => String::from_utf8(bytes).ok(),
        "iso-8859-1" => Some(bytes.into_iter().map(char::from).collect()),
        _ => None,
    }
}

/// Keeps only the final path component of a server-supplied name; rejects
/// names that are empty, `.`, `..`, contain `:` (a Windows drive prefix or
/// NTFS alternate data stream) or control characters, or are not a single
/// plain path component. The rules are the same on every platform.
fn sanitize_file_name(name: &str) -> Option<String> {
    let name = name.rsplit(['/', '\\']).next()?.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(':')
        || name.chars().any(char::is_control)
    {
        return None;
    }
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) => Some(name.to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(value: &str) -> Option<String> {
        disposition_file_name(value).and_then(|name| sanitize_file_name(&name))
    }

    #[test]
    fn parses_content_disposition() {
        assert_eq!(name("attachment; filename=a.zip").as_deref(), Some("a.zip"));
        assert_eq!(
            name(r#"attachment; filename="a \"b\"; c.zip"; size=3"#).as_deref(),
            Some(r#"a "b"; c.zip"#)
        );
        assert_eq!(
            name("attachment; filename=\"plain.zip\"; filename*=UTF-8''%C3%A9t%C3%A9.zip")
                .as_deref(),
            Some("été.zip")
        );
        assert_eq!(
            name("attachment; FILENAME*=iso-8859-1'en'%E9.json").as_deref(),
            Some("é.json")
        );
        assert_eq!(
            name("attachment; inline; filename=x.jsonl").as_deref(),
            Some("x.jsonl")
        );
        // A broken extended value falls back to `filename`.
        assert_eq!(
            name("attachment; filename*=UTF-8''%ZZ; filename=ok.zip").as_deref(),
            Some("ok.zip")
        );
    }

    #[test]
    fn rejects_unusable_names() {
        assert_eq!(name("attachment"), None);
        assert_eq!(name("attachment; size=3"), None);
        assert_eq!(name("attachment; filename=\"\""), None);
        assert_eq!(name("attachment; filename=\"..\""), None);
        assert_eq!(name("attachment; filename=\"a\nb\""), None);
    }

    #[test]
    fn strips_directories_from_names() {
        assert_eq!(
            name("attachment; filename=\"../../etc/passwd\"").as_deref(),
            Some("passwd")
        );
        assert_eq!(
            name(r#"attachment; filename="C:\\x\\y.zip""#).as_deref(),
            Some("y.zip")
        );
        assert_eq!(name("attachment; filename=\"dir/\""), None);
        assert_eq!(name("attachment; filename=\"a/..\""), None);
        assert_eq!(name(r#"attachment; filename="..\\.""#), None);
    }

    #[test]
    fn rejects_drive_prefixes_and_streams() {
        assert_eq!(name("attachment; filename=\"C:x.zip\""), None);
        assert_eq!(name("attachment; filename=C:x.zip"), None);
        assert_eq!(name("attachment; filename=\"a.zip:s\""), None);
        assert_eq!(name("attachment; filename*=UTF-8''a.zip%3Astream"), None);
        assert_eq!(sanitize_file_name("C:"), None);
        assert_eq!(sanitize_file_name("ok.zip").as_deref(), Some("ok.zip"));
    }
}
