/// Shared file classification helpers used by file tools and workspace readers.
#[derive(Debug, Clone, Default)]
pub struct FileUtils;

impl FileUtils {
    /// Portable storage components use the common filename subset on all hosts.
    pub fn isPortableFileName(name: &str) -> bool {
        if name.is_empty() || name.len() > 255 || name.ends_with([' ', '.'])
            || name.chars().any(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')) {
            return false;
        }
        let stem = name.split('.').next().unwrap_or_default().trim_end_matches(' ').to_ascii_uppercase();
        !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
            && !["COM", "LPT"].iter().any(|prefix| stem.strip_prefix(*prefix).is_some_and(|number|
                matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")))
    }

    /// Storage paths are relative, slash-separated, and include directory names.
    pub fn isPortableFilePath(path: &str) -> bool {
        path.split('/').all(Self::isPortableFileName)
    }

    /// Reports whether the supplied bytes look like UTF-8 or plain text.
    pub fn isTextLike(bytes: &[u8]) -> bool {
        Self::isTextLikeSample(bytes, bytes.len())
    }

    /// Reports whether a bounded byte sample looks like UTF-8 or plain text.
    pub fn isTextLikeSample(bytes: &[u8], sampleSize: usize) -> bool {
        let length = bytes.len().min(sampleSize);
        if length == 0 {
            return true;
        }

        let mut textChars = 0usize;
        let mut nonTextChars = 0usize;
        let mut index = 0usize;
        while index < length {
            let byte = bytes[index];
            if isAsciiTextByte(byte) {
                textChars += 1;
                index += 1;
                continue;
            }

            let sequenceLength = utf8SequenceLength(bytes, index, length);
            if sequenceLength > 0 {
                textChars += sequenceLength;
                index += sequenceLength;
            } else {
                nonTextChars += 1;
                index += 1;
            }
        }

        if nonTextChars == 0 {
            return true;
        }
        let totalChars = textChars + nonTextChars;
        totalChars > 0 && nonTextChars.saturating_mul(100) < totalChars.saturating_mul(10)
    }

    /// Reports whether an extension belongs to the known text-file set.
    pub fn isTextBasedExtension(extension: &str) -> bool {
        matches!(
            extension.trim().to_ascii_lowercase().as_str(),
            "txt"
                | "md"
                | "log"
                | "ini"
                | "env"
                | "csv"
                | "tsv"
                | "text"
                | "me"
                | "html"
                | "htm"
                | "css"
                | "js"
                | "json"
                | "xml"
                | "yaml"
                | "yml"
                | "svg"
                | "url"
                | "sass"
                | "scss"
                | "less"
                | "ejs"
                | "hbs"
                | "pug"
                | "rss"
                | "atom"
                | "vtt"
                | "webmanifest"
                | "jsp"
                | "asp"
                | "aspx"
                | "java"
                | "kt"
                | "kts"
                | "gradle"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "cs"
                | "m"
                | "py"
                | "rb"
                | "php"
                | "go"
                | "swift"
                | "ts"
                | "tsx"
                | "jsx"
                | "sh"
                | "bat"
                | "ps1"
                | "zsh"
                | "sql"
                | "groovy"
                | "lua"
                | "perl"
                | "pl"
                | "r"
                | "dart"
                | "rust"
                | "rs"
                | "scala"
                | "asm"
                | "pas"
                | "f"
                | "f90"
                | "for"
                | "lisp"
                | "hs"
                | "erl"
                | "vb"
                | "vbs"
                | "tcl"
                | "d"
                | "nim"
                | "sol"
                | "zig"
                | "vala"
                | "cob"
                | "cbl"
                | "properties"
                | "toml"
                | "dockerfile"
                | "gitignore"
                | "gitattributes"
                | "editorconfig"
                | "conf"
                | "cfg"
                | "jsonc"
                | "json5"
                | "reg"
                | "iml"
                | "inf"
                | "rtf"
                | "tex"
                | "srt"
                | "sub"
                | "asciidoc"
                | "adoc"
                | "rst"
                | "org"
                | "wiki"
                | "mediawiki"
                | "vcf"
                | "ics"
                | "gpx"
                | "kml"
                | "opml"
        )
    }

    /// Reports whether a file name belongs to the known text-file set.
    pub fn isTextBasedFileName(fileName: &str) -> bool {
        let name = fileName.trim();
        if name.is_empty() {
            return false;
        }
        let lowerName = name.to_ascii_lowercase();
        let extension = lowerName.rsplit_once('.').map(|(_, value)| value);
        match extension {
            Some(value) if !value.is_empty() && value != lowerName => {
                Self::isTextBasedExtension(value)
            }
            _ => matches!(
                lowerName.as_str(),
                "readme"
                    | "makefile"
                    | "dockerfile"
                    | "license"
                    | "changelog"
                    | "authors"
                    | "contributors"
                    | "copying"
                    | "install"
                    | "news"
                    | "todo"
                    | "version"
                    | "gemfile"
                    | "rakefile"
                    | "vagrantfile"
                    | "buildfile"
            ),
        }
    }
}

/// Reports whether one byte is printable ASCII or common text whitespace.
fn isAsciiTextByte(byte: u8) -> bool {
    (32..=126).contains(&byte) || matches!(byte, b'\t' | b'\n' | b'\r')
}

/// Returns the length of a valid UTF-8 sequence beginning at an index.
fn utf8SequenceLength(bytes: &[u8], index: usize, length: usize) -> usize {
    let first = bytes[index];
    let expected = match first {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return 0,
    };
    if index + expected > length {
        return 0;
    }
    if bytes[index + 1..index + expected]
        .iter()
        .all(|byte| (byte & 0xC0) == 0x80)
    {
        expected
    } else {
        0
    }
}
