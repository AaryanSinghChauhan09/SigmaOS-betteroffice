//! SigmaOS betteroffice — office document model.
//!
//! Unified document model for text docs, spreadsheets, and slides.
//! Documents serialize to a SigmaOS office JSON format (.sdoc).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Errors produced by this component.
#[derive(Debug, thiserror::Error)]
pub enum OfficeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid document: {0}")]
    Invalid(String),
    #[error("not found: {0}")]
    NotFound(String),
}

/// Document kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    Text,
    Sheet,
    Slides,
}

/// A text document body: list of paragraphs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextDoc {
    pub paragraphs: Vec<String>,
}

/// A spreadsheet: rows of cells (strings; numbers stay strings, parsed on demand).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SheetDoc {
    /// header row (optional)
    #[serde(default)]
    pub header: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
}

impl SheetDoc {
    /// Sum a numeric column.
    pub fn column_sum(&self, col: usize) -> f64 {
        self.rows
            .iter()
            .filter_map(|r| r.get(col))
            .filter_map(|c| c.parse::<f64>().ok())
            .sum()
    }

    /// Row count.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Column count (max row width).
    pub fn col_count(&self) -> usize {
        self.rows.iter().map(|r| r.len()).max().unwrap_or(0)
    }
}

/// A slide deck: slides with title + bullet points.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Slide {
    pub title: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlidesDoc {
    #[serde(default)]
    pub slides: Vec<Slide>,
}

/// Unified document.
#[derive(Debug, Clone, Serialize)]
pub struct Document {
    pub name: String,
    pub kind: DocKind,
    pub created_unix: u64,
    #[serde(default)]
    pub modified_unix: u64,
    #[serde(flatten)]
    pub body: DocBody,
}

/// Body variants — serialized inline per kind (untagged flattens the
/// inner struct). Deserialization is kind-directed via `Document`'s
/// custom Deserialize impl.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum DocBody {
    Text(TextDoc),
    Sheet(SheetDoc),
    Slides(SlidesDoc),
}

#[derive(Deserialize)]
struct DocumentMeta {
    name: String,
    kind: DocKind,
    created_unix: u64,
    #[serde(default)]
    modified_unix: u64,
}

impl<'de> Deserialize<'de> for Document {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        let meta: DocumentMeta =
            serde_json::from_value(v.clone()).map_err(serde::de::Error::custom)?;
        let body = match meta.kind {
            DocKind::Text => {
                DocBody::Text(serde_json::from_value(v.clone()).map_err(serde::de::Error::custom)?)
            }
            DocKind::Sheet => {
                DocBody::Sheet(serde_json::from_value(v.clone()).map_err(serde::de::Error::custom)?)
            }
            DocKind::Slides => {
                DocBody::Slides(serde_json::from_value(v).map_err(serde::de::Error::custom)?)
            }
        };
        Ok(Document {
            name: meta.name,
            kind: meta.kind,
            created_unix: meta.created_unix,
            modified_unix: meta.modified_unix,
            body,
        })
    }
}

impl Document {
    /// Create a new empty document.
    pub fn new(name: &str, kind: DocKind) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let body = match kind {
            DocKind::Text => DocBody::Text(TextDoc::default()),
            DocKind::Sheet => DocBody::Sheet(SheetDoc::default()),
            DocKind::Slides => DocBody::Slides(SlidesDoc::default()),
        };
        Self {
            name: name.to_string(),
            kind,
            created_unix: now,
            modified_unix: now,
            body,
        }
    }

    /// Validate document invariants.
    pub fn validate(&self) -> Result<(), OfficeError> {
        if self.name.is_empty() {
            return Err(OfficeError::Invalid("empty document name".into()));
        }
        Ok(())
    }

    /// Touch modified timestamp.
    pub fn touch(&mut self) {
        self.modified_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Save to .sdoc JSON file (atomic write).
    pub fn save(&self, path: &Path) -> Result<(), OfficeError> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("sdoc.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load from .sdoc JSON file.
    pub fn load(path: &Path) -> Result<Self, OfficeError> {
        let text = std::fs::read_to_string(path)
            .map_err(|_| OfficeError::NotFound(path.display().to_string()))?;
        let doc: Document = serde_json::from_str(&text)?;
        doc.validate()?;
        Ok(doc)
    }

    /// Plain-text rendering for previews.
    pub fn render_text(&self) -> String {
        match &self.body {
            DocBody::Text(t) => t.paragraphs.join("\n\n"),
            DocBody::Sheet(s) => {
                let mut out = String::new();
                if !s.header.is_empty() {
                    out.push_str(&s.header.join(" | "));
                    out.push('\n');
                }
                for row in &s.rows {
                    out.push_str(&row.join(" | "));
                    out.push('\n');
                }
                out
            }
            DocBody::Slides(d) => {
                let mut out = String::new();
                for (i, s) in d.slides.iter().enumerate() {
                    out.push_str(&format!("Slide {}: {}\n", i + 1, s.title));
                    for b in &s.bullets {
                        out.push_str(&format!("  - {b}\n"));
                    }
                }
                out
            }
        }
    }
}

/// Default documents directory.
pub fn default_docs_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "sigmaos", "sigmaos")
        .map(|p| p.data_dir().join("documents"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bo-{}-{}", tag, std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn slides_roundtrip_preserves_content() {
        // Regression: untagged DocBody matched Sheet first (all fields
        // defaulted), silently dropping slides on load.
        let dir = tmpdir("slides");
        let path = dir.join("deck.sdoc");
        let mut doc = Document::new("deck", DocKind::Slides);
        if let DocBody::Slides(d) = &mut doc.body {
            d.slides.push(Slide {
                title: "Intro".into(),
                bullets: vec!["one".into(), "two".into()],
            });
        }
        doc.save(&path).unwrap();
        let back = Document::load(&path).unwrap();
        match back.body {
            DocBody::Slides(d) => {
                assert_eq!(d.slides.len(), 1);
                assert_eq!(d.slides[0].title, "Intro");
                assert_eq!(d.slides[0].bullets.len(), 2);
            }
            _ => panic!("slides doc loaded as wrong kind"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sheet_roundtrip_and_math() {
        let dir = tmpdir("sheet");
        let path = dir.join("s.sdoc");
        let mut doc = Document::new("s", DocKind::Sheet);
        if let DocBody::Sheet(s) = &mut doc.body {
            s.header = vec!["item".into(), "qty".into()];
            s.rows = vec![
                vec!["a".into(), "2".into()],
                vec!["b".into(), "3".into()],
                vec!["c".into(), "x".into()], // non-numeric ignored
            ];
        }
        doc.save(&path).unwrap();
        let back = Document::load(&path).unwrap();
        match back.body {
            DocBody::Sheet(s) => {
                assert_eq!(s.column_sum(1), 5.0);
                assert_eq!(s.row_count(), 3);
                assert_eq!(s.col_count(), 2);
            }
            _ => panic!("sheet doc loaded as wrong kind"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_roundtrip_and_render() {
        let dir = tmpdir("text");
        let path = dir.join("t.sdoc");
        let mut doc = Document::new("t", DocKind::Text);
        if let DocBody::Text(t) = &mut doc.body {
            t.paragraphs = vec!["hello".into(), "world".into()];
        }
        doc.save(&path).unwrap();
        let back = Document::load(&path).unwrap();
        assert!(back.render_text().contains("hello\n\nworld"));
        assert!(!path.with_extension("sdoc.tmp").exists()); // atomic rename
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_rejects_empty_name() {
        let doc = Document::new("", DocKind::Text);
        assert!(doc.validate().is_err());
        let dir = tmpdir("bad");
        let p = dir.join("bad.sdoc");
        assert!(doc.save(&p).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_errors() {
        assert!(Document::load(Path::new("/nonexistent/x.sdoc")).is_err());
    }

    #[test]
    fn slide_render_format() {
        let mut doc = Document::new("d", DocKind::Slides);
        if let DocBody::Slides(d) = &mut doc.body {
            d.slides.push(Slide {
                title: "T".into(),
                bullets: vec!["b1".into()],
            });
        }
        let r = doc.render_text();
        assert!(r.contains("Slide 1: T"));
        assert!(r.contains("  - b1"));
    }
}
