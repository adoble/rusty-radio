#![cfg_attr(not(test), no_std)]

// See "specification" on Wikipedia at https://en.wikipedia.org/wiki/M3U
// Can use these as example m3us : https://github.com/junguler/m3u-radio-music-playlists

use core::str::Utf8Error;

pub struct M3U<'a> {
    contents: &'a [u8],
}

impl<'a> M3U<'a> {
    pub fn new(contents: &'a [u8]) -> M3U<'a> {
        M3U { contents }
    }

    pub fn location(&self) -> Result<&str, M3UError> {
        let m3u_file_contents = str::from_utf8(self.contents).unwrap();

        let first_line = m3u_file_contents
            .lines()
            .next()
            .ok_or(M3UError::EmptyFile)?;

        if first_line.starts_with("#EXTM3U") {
            // Extended M3U format - find first non-comment line
            m3u_file_contents
                .lines()
                .find(|line| !line.starts_with('#') && !line.trim().is_empty())
                .ok_or(M3UError::NoValidUrl)
        } else {
            // Simple M3U format - first line is the URL
            Ok(first_line)
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum M3UError {
    Utf8ConversionError(Utf8Error),
    EmptyFile,
    NoValidUrl,
}

impl From<Utf8Error> for M3UError {
    fn from(e: Utf8Error) -> Self {
        Self::Utf8ConversionError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_M3U: &str = "http://listen.181fm.com/181-classical_128k.mp3";

    #[test]
    fn test_location_old() {
        let buffer: &[u8; 5] = b"abcde";
        let m3u = M3U::new(buffer);
        assert_eq!(m3u.location().unwrap(), "abcde");
    }

    #[test]
    fn test_location() {
        let buffer = SIMPLE_M3U.as_bytes();
        let m3u = M3U::new(buffer);
        assert_eq!(
            m3u.location().unwrap(),
            "http://listen.181fm.com/181-classical_128k.mp3"
        );
    }

    #[test]
    fn test_extended() {
        let extended_m3u = include_str!("../test_resources/extended_m3u.m3u");
        let buffer = extended_m3u.as_bytes();

        let m3u = M3U::new(buffer);
        assert_eq!(
            m3u.location().unwrap(),
            "http://listen.181fm.com/181-classical_128k.mp3"
        );
    }
}
