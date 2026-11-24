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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub name: String,
    pub kind: DocKind,
    pub created_unix: u64,
    #[serde(default)]
    pub modified_unix: u64,
    #[serde(flatten)]
    pub body: DocBody,
}

/// Body variants — serialized inline per kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DocBody {
    Text(TextDoc),
    Sheet(SheetDoc),
    Slides(SlidesDoc),
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
        .map(|p| p.documents_dir().to_path_buf())
}
