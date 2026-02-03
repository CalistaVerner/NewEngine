#![forbid(unsafe_op_in_unsafe_fn)]

use serde_json::Value as JsonValue;
use std::io::Cursor;

const EXPECTED_SCHEMA_V1: &str = "kalitech.text.meta.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    Json,
    Xml,
    Html,
    Txt,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TextMeta {
    pub schema: String,
    pub container: String,
    pub encoding: String,
    pub byte_len: u64,
}

#[derive(Debug, Clone)]
pub struct TextDocument {
    pub format: TextFormat,
    pub meta: TextMeta,
    pub text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TextReadError {
    #[error("wire: too short")]
    TooShort,
    #[error("wire: meta length out of bounds")]
    MetaOutOfBounds,
    #[error("wire: meta length too large ({0} bytes)")]
    MetaTooLarge(usize),
    #[error("meta json: {0}")]
    MetaJson(String),
    #[error("meta schema mismatch: found '{found}', expected '{expected}'")]
    SchemaMismatch { found: String, expected: String },
    #[error("utf8: {0}")]
    Utf8(String),
    #[error("json parse: {0}")]
    JsonParse(String),
    #[error("xml parse: {0}")]
    XmlParse(String),
}

pub struct TextReader;

impl TextReader {
    pub const MAX_META_BYTES: usize = 64 * 1024;

    pub fn read_wire(bytes: &[u8]) -> Result<TextDocument, TextReadError> {
        if bytes.len() < 4 {
            return Err(TextReadError::TooShort);
        }

        let meta_len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if meta_len > Self::MAX_META_BYTES {
            return Err(TextReadError::MetaTooLarge(meta_len));
        }

        let meta_start: usize = 4;
        let meta_end: usize = meta_start + meta_len;
        if meta_end > bytes.len() {
            return Err(TextReadError::MetaOutOfBounds);
        }

        let meta_bytes = &bytes[meta_start..meta_end];
        let payload = &bytes[meta_end..];

        let meta_str =
            std::str::from_utf8(meta_bytes).map_err(|e| TextReadError::Utf8(e.to_string()))?;

        let meta_json: serde_json::Value =
            serde_json::from_str(meta_str).map_err(|e| TextReadError::MetaJson(e.to_string()))?;

        let schema = meta_json
            .get("schema")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        if schema != EXPECTED_SCHEMA_V1 {
            return Err(TextReadError::SchemaMismatch {
                found: schema,
                expected: EXPECTED_SCHEMA_V1.to_owned(),
            });
        }

        let container_raw = meta_json
            .get("container")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        let container = normalize_container(&container_raw);

        let encoding = meta_json
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("utf-8")
            .to_owned();

        if encoding.to_ascii_lowercase() != "utf-8" {
            return Err(TextReadError::MetaJson(format!(
                "unsupported encoding '{encoding}', only 'utf-8' is supported"
            )));
        }

        let byte_len = meta_json
            .get("byte_len")
            .and_then(|v| v.as_u64())
            .unwrap_or(payload.len() as u64);

        let text =
            std::str::from_utf8(payload).map_err(|e| TextReadError::Utf8(e.to_string()))?;
        let text = strip_utf8_bom(text).to_owned();

        let format = match container.as_str() {
            "json" => TextFormat::Json,
            "xml" | "ui" => TextFormat::Xml,
            "html" => TextFormat::Html,
            "txt" => TextFormat::Txt,
            _ => TextFormat::Unknown,
        };

        Ok(TextDocument {
            format,
            meta: TextMeta {
                schema: EXPECTED_SCHEMA_V1.to_owned(),
                container,
                encoding,
                byte_len,
            },
            text,
        })
    }

    pub fn parse_json(doc: &TextDocument) -> Result<JsonValue, TextReadError> {
        if doc.format != TextFormat::Json {
            return Err(TextReadError::JsonParse("document is not json".to_owned()));
        }
        serde_json::from_str(&doc.text).map_err(|e| TextReadError::JsonParse(e.to_string()))
    }

    pub fn validate_xml(doc: &TextDocument) -> Result<(), TextReadError> {
        if doc.format != TextFormat::Xml {
            return Err(TextReadError::XmlParse("document is not xml".to_owned()));
        }

        let mut r = quick_xml::Reader::from_reader(Cursor::new(doc.text.as_bytes()));
        let mut buf = Vec::new();

        loop {
            match r.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => return Err(TextReadError::XmlParse(e.to_string())),
            }
            buf.clear();
        }

        Ok(())
    }
}

#[inline]
fn normalize_container(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "htm" | "html" => "html".to_owned(),
        "json" => "json".to_owned(),
        "xml" => "xml".to_owned(),
        "ui" => "ui".to_owned(),
        "txt" | "text" | "md" => "txt".to_owned(),
        other => other.to_owned(),
    }
}

#[inline]
fn strip_utf8_bom(s: &str) -> &str {
    const BOM: char = '\u{feff}';
    s.strip_prefix(BOM).unwrap_or(s)
}