#![cfg_attr(not(test), no_std)]

// See "specification" on Wikipedia at https://en.wikipedia.org/wiki/M3U
// Can use these as example m3us : https://github.com/junguler/m3u-radio-music-playlists

use core::str::Utf8Error;

use heapless::String;

pub struct M3U<const MAX_URL_LEN: usize> {
    // contents: &'a [u8],
    url_buffer: [u8; MAX_URL_LEN],
    state: State,
    pos: usize,
}

#[derive(Clone, Debug)]
enum State {
    Initial,
    H,
    T1,
    T2,
    P,
    Colon,
    Slash1,
    Slash2,
    Rest,
}

impl<const MAX_URL_LEN: usize> M3U<MAX_URL_LEN> {
    pub fn new() -> M3U<MAX_URL_LEN> {
        M3U {
            url_buffer: [0; MAX_URL_LEN],
            state: State::Initial,
            pos: 0,
        }
    }

    // Extracts the first URL it finds in an extended M3U file and uses that as the next location
    // This functions expects individual characters from a stream of data to be given.If no url currently
    // found in the parsing process then it returns None. This means it should be goven more characters
    // until the URL is found and Some is returned.
    // It is designed to be sparing with memory.
    // TODO  can we make this a "stream"  function
    // TODO can we , should we, extend this so that it returns all the urls?
    pub fn parse_m3u(&mut self, char: u8) -> Result<Option<String<MAX_URL_LEN>>, M3UError> {
        // Assuming the first url found is the location and that it points to an audio stream and
        // not another m3u file.

        // Look for http://

        // let state = self.state.clone();
        match (self.state.clone(), char) {
            (State::Initial, b'h') => {
                self.url_buffer[self.pos] = char;
                self.pos += 1;
                self.state = State::H;
                Ok(None)
            }
            (State::Initial, _) => Ok(None),

            (State::H, b't') => {
                self.url_buffer[self.pos] = char;
                self.pos += 1;
                self.state = State::T1;
                Ok(None)
            }
            (State::T1, b't') => {
                self.url_buffer[self.pos] = char;
                self.pos += 1;
                self.state = State::T2;
                Ok(None)
            }
            (State::T2, b'p') => {
                self.url_buffer[self.pos] = char;
                self.pos += 1;
                self.state = State::P;
                Ok(None)
            }
            (State::P, b':') => {
                self.url_buffer[self.pos] = char;
                self.pos += 1;
                self.state = State::Colon;
                Ok(None)
            }
            (State::Colon, b'/') => {
                self.url_buffer[self.pos] = char;
                self.pos += 1;
                self.state = State::Slash1;
                Ok(None)
            }
            (State::Slash1, b'/') => {
                self.url_buffer[self.pos] = char;
                self.pos += 1;
                self.state = State::Slash2;
                Ok(None)
            }
            (State::Slash2, b'\n') => Err(M3UError::MalformedUrl),
            (State::Slash2, _) => {
                if char.is_ascii_whitespace() {
                    Err(M3UError::MalformedUrl)
                } else {
                    self.url_buffer[self.pos] = char;
                    self.pos += 1;
                    self.state = State::Rest;
                    Ok(None)
                }
            }
            (State::Rest, b'\n') | (State::Rest, b'\r') => {
                // End state
                let url_str = core::str::from_utf8(&self.url_buffer[0..self.pos])?;
                let mut url = String::<MAX_URL_LEN>::new();

                url.push_str(url_str).map_err(|_| M3UError::UrlTooLong)?;
                Ok(Some(url))
            }
            (State::Rest, _) => {
                // Process rest of the url
                if char.is_ascii_whitespace() {
                    Err(M3UError::MalformedUrl)
                } else {
                    self.url_buffer[self.pos] = char;
                    self.pos += 1;
                    self.state = State::Rest;
                    Ok(None)
                }
            }
            (_, _) => Err(M3UError::InvalidInternalState),
        }
    }

    // Not all urls are terminated with a new line. If the end of the stream has been reached
    // and no URL found, use this function to return the URL if it exists
    pub fn terminate(&mut self) -> Result<String<MAX_URL_LEN>, M3UError> {
        match self.state.clone() {
            State::Rest => {
                let url_str = core::str::from_utf8(&self.url_buffer[0..self.pos])?;
                let mut url = String::<MAX_URL_LEN>::new();

                url.push_str(url_str).map_err(|_| M3UError::UrlTooLong)?;
                Ok(url)
            }
            _ => Err(M3UError::MalformedUrl),
        }
    }
}

impl<const MAX_URL_LEN: usize> Default for M3U<MAX_URL_LEN> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum M3UError {
    Utf8ConversionError(Utf8Error),
    UrlTooLong,
    MalformedUrl,
    InvalidInternalState,
}

impl From<Utf8Error> for M3UError {
    fn from(e: Utf8Error) -> Self {
        Self::Utf8ConversionError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_M3U: &str = "http://listen.181fm.com/181-classical_128k.mp3\n";

    const EXTENDED_M3U: &str = "#EXTM3U\n\n#EXTINF:0, station name\nhttp://radio.com/stream/mp3/stream.mp3\nn#EXTINF:0, station name\nhttp://hitradio.com//mp3/stream.mp3\n";

    #[test]
    fn test_parse_simple_m3u() {
        let mut url = String::<1024>::new();

        let mut m3u = M3U::new();
        for c in SIMPLE_M3U.chars() {
            let b: u8 = c.try_into().unwrap();
            let partial_url = m3u.parse_m3u(b).unwrap();
            match partial_url {
                Some(full_url) => {
                    url = full_url;
                    break;
                }
                None => continue,
            }
        }

        let mut expected_url = String::<1024>::new();
        expected_url
            .push_str("http://listen.181fm.com/181-classical_128k.mp3")
            .unwrap();
        assert_eq!(expected_url, url);
    }

    #[test]
    fn test_parse_extended_m3u() {
        let mut url = String::<1024>::new();

        let mut m3u = M3U::new();
        for c in EXTENDED_M3U.chars() {
            let b: u8 = c.try_into().unwrap();
            let partial_url = m3u.parse_m3u(b).unwrap();
            match partial_url {
                Some(full_url) => {
                    url = full_url;
                    break;
                }
                None => continue,
            }
        }

        let mut expected_url = String::<1024>::new();
        expected_url
            .push_str("http://radio.com/stream/mp3/stream.mp3")
            .unwrap();
        assert_eq!(expected_url, url);
    }

    #[test]
    fn test_parse_simple_m3u_unterminated() {
        let simple_m3u_unterminated = "http://listen.181fm.com/181-classical_128k.mp3";

        let mut m3u = M3U::<1024>::new();
        for c in simple_m3u_unterminated.chars() {
            let b: u8 = c.try_into().unwrap();
            let partial_url = m3u.parse_m3u(b).unwrap();
            match partial_url {
                Some(_) => {
                    assert!(false, "Should not happen as the url is unterminated")
                }
                None => continue,
            }
        }

        let url = m3u.terminate().unwrap();

        let mut expected_url = String::<1024>::new();
        expected_url
            .push_str("http://listen.181fm.com/181-classical_128k.mp3")
            .unwrap();
        assert_eq!(expected_url, url);
    }

    #[test]
    fn test_parse_extended_m3u_invalid_url() {
        let incorrect_m3u_string = "#EXTM3U\n\n#EXTINF:0, station name\nhttp://radio.com/ stream/mp3/stream.mp3\nn#EXTINF:0, station name\nhttp://hitradio.com//mp3/stream.mp3\n";

        let mut m3u = M3U::<1024>::new();
        for c in incorrect_m3u_string.chars() {
            let b: u8 = c.try_into().unwrap();
            let r = m3u.parse_m3u(b);
            match r {
                Ok(None) => continue,
                Ok(Some(_)) => assert!(false, "Fully parsed malformed URL!"),
                Err(err) => {
                    assert_eq!(err, M3UError::MalformedUrl);
                    break;
                }
            };
        }
    }
}
