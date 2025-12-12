//! Text encoding and line ending types.

use std::fmt;

/// Line ending style detected in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    /// Unix-style line endings (LF, `\n`)
    #[default]
    Lf,
    /// Windows-style line endings (CRLF, `\r\n`)
    Crlf,
    /// Classic Mac line endings (CR, `\r`)
    Cr,
    /// Mixed line endings detected
    Mixed,
}

impl LineEnding {
    /// Detects the predominant line ending style in the given content.
    pub fn detect(content: &str) -> Self {
        let bytes = content.as_bytes();
        let mut i = 0;
        let mut crlf = 0;
        let mut lf = 0;
        let mut cr = 0;
        
        while i < bytes.len() {
            match bytes[i] {
                b'\r' => {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                        crlf += 1;
                        i += 2;
                        continue;
                    } else {
                        cr += 1;
                    }
                }
                b'\n' => lf += 1,
                _ => {}
            }
            i += 1;
        }

        match (crlf, lf, cr) {
            (c, 0, 0) if c > 0 => Self::Crlf,
            (0, l, 0) if l > 0 => Self::Lf,
            (0, 0, c) if c > 0 => Self::Cr,
            (0, 0, 0) => Self::Lf, // default when no newlines
            _ => Self::Mixed,
        }
    }
}

impl fmt::Display for LineEnding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lf => write!(f, "LF"),
            Self::Crlf => write!(f, "CRLF"),
            Self::Cr => write!(f, "CR"),
            Self::Mixed => write!(f, "Mixed"),
        }
    }
}

/// Text encoding of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    /// UTF-8 encoding (the default)
    #[default]
    Utf8,
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Utf8 => write!(f, "UTF-8"),
        }
    }
}
