use end_model::{AicBuildError, CatalogBuildError};
use std::collections::BTreeMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

/// Result alias for IO and schema-loading operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors raised while loading/validating TOML inputs.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to read file at path `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse TOML file at path `{path}`")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error(
        "{}",
        render_schema_error(
            path,
            field,
            *index,
            span.as_ref(),
            src.as_deref(),
            message
        )
    )]
    Schema {
        path: PathBuf,
        field: &'static str,
        index: Option<usize>,
        span: Option<std::ops::Range<usize>>,
        /// Source text used to render line/column excerpts from span offsets.
        src: Option<Arc<str>>,
        message: Box<str>,
    },

    #[error(
        "{}",
        render_span_error(
            format!("Duplicate {kind} key `{key}` in {}", path.display()),
            span.as_ref(),
            src.as_deref()
        )
    )]
    DuplicateKey {
        path: PathBuf,
        kind: &'static str,
        key: Box<str>,
        span: Option<std::ops::Range<usize>>,
        /// Source text used to render line/column excerpts from span offsets.
        src: Option<Arc<str>>,
    },

    #[error(
        "{}",
        render_span_error(
            format!("Unknown item `{key}` in {}", path.display()),
            span.as_ref(),
            src.as_deref()
        )
    )]
    UnknownItem {
        path: PathBuf,
        key: Box<str>,
        span: Option<std::ops::Range<usize>>,
        /// Source text used to render line/column excerpts from span offsets.
        src: Option<Arc<str>>,
    },

    #[error(
        "{}",
        render_span_error(
            format!("Unknown facility `{key}` in {}", path.display()),
            span.as_ref(),
            src.as_deref()
        )
    )]
    UnknownFacility {
        path: PathBuf,
        key: Box<str>,
        span: Option<std::ops::Range<usize>>,
        /// Source text used to render line/column excerpts from span offsets.
        src: Option<Arc<str>>,
    },
}

fn schema_index_suffix(index: Option<usize>) -> String {
    index
        .map(|value| format!(", index={value}"))
        .unwrap_or_default()
}

fn prev_char_boundary(src: &str, mut idx: usize) -> usize {
    while idx > 0 && !src.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn next_char_boundary(src: &str, mut idx: usize) -> usize {
    while idx < src.len() && !src.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

fn render_schema_error(
    path: &std::path::Path,
    field: &str,
    index: Option<usize>,
    span: Option<&std::ops::Range<usize>>,
    src: Option<&str>,
    message: &str,
) -> String {
    let base = format!(
        "Schema error in {}, field `{field}`{}: {message}",
        path.display(),
        schema_index_suffix(index)
    );
    render_span_error(base, span, src)
}

fn render_span_error(
    base: String,
    span: Option<&std::ops::Range<usize>>,
    src: Option<&str>,
) -> String {
    let Some(span) = span else {
        return base;
    };
    if let Some(src) = src
        && let Some(loc) = SpanExcerpt::from_source(src, span)
    {
        return loc.render(base);
    }
    format!("{base}, span={}..{}", span.start, span.end)
}

#[derive(Debug)]
struct SpanExcerpt {
    line_number: usize,
    column_number: usize,
    line_text: Box<str>,
    caret_offset: usize,
    caret_len: usize,
}

impl SpanExcerpt {
    fn from_source(src: &str, span: &std::ops::Range<usize>) -> Option<Self> {
        if src.is_empty() {
            return None;
        }

        let src_len = src.len();
        let start = prev_char_boundary(src, span.start.min(src_len));
        let mut end = next_char_boundary(src, span.end.min(src_len));
        if end < start {
            end = start;
        }

        let line_start = src[..start].rfind('\n').map_or(0, |idx| idx + 1);
        let line_end = src[start..]
            .find('\n')
            .map_or(src_len, |offset| start + offset);

        let line_number = src[..start].bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column_number = src[line_start..start].chars().count() + 1;
        let line_text: Box<str> = src[line_start..line_end].trim_end_matches('\r').into();

        let line_highlight_end = if end <= start {
            if start < line_end {
                src[start..line_end]
                    .chars()
                    .next()
                    .map_or(start, |ch| start + ch.len_utf8())
            } else {
                start
            }
        } else {
            end.min(line_end)
        };
        let caret_len = if line_highlight_end <= start {
            1
        } else {
            src[start..line_highlight_end].chars().count().max(1)
        };

        Some(Self {
            line_number,
            column_number,
            line_text,
            caret_offset: column_number.saturating_sub(1),
            caret_len,
        })
    }

    fn render(&self, base: String) -> String {
        let gutter_width = self.line_number.to_string().len();
        let gutter_pad = " ".repeat(gutter_width);
        let caret_pad = " ".repeat(self.caret_offset);
        let caret = "^".repeat(self.caret_len);
        format!(
            "{base} at line {}, column {}\n{gutter_pad} |\n{} | {}\n{gutter_pad} | {}{}",
            self.line_number,
            self.column_number,
            self.line_number,
            self.line_text,
            caret_pad,
            caret
        )
    }
}

#[derive(Clone, Debug)]
pub struct RecipeSpanContext {
    pub recipe: Option<Range<usize>>,
    pub ingredients: Option<Range<usize>>,
    pub products: Option<Range<usize>>,
}

/// Re-map item-related builder errors to precise user-facing schema fields.
pub fn map_item_build_error(
    path: &Path,
    src: &Arc<str>,
    index: usize,
    span: Option<Range<usize>>,
    source: CatalogBuildError,
) -> Error {
    match source {
        CatalogBuildError::DuplicateItemKey(key) => Error::DuplicateKey {
            path: path.to_path_buf(),
            kind: "item",
            key: key.to_string().into_boxed_str(),
            span,
            src: Some(Arc::clone(src)),
        },
        _ => Error::Schema {
            path: path.to_path_buf(),
            field: "items",
            index: Some(index),
            span,
            src: Some(Arc::clone(src)),
            message: source.to_string().into_boxed_str(),
        },
    }
}

/// Re-map machine-related builder errors to precise user-facing schema fields.
pub fn map_machine_build_error(
    path: &Path,
    src: &Arc<str>,
    index: usize,
    span: Option<Range<usize>>,
    source: CatalogBuildError,
) -> Error {
    match source {
        CatalogBuildError::DuplicateFacilityKey(key) => Error::DuplicateKey {
            path: path.to_path_buf(),
            kind: "facility",
            key: key.to_string().into_boxed_str(),
            span,
            src: Some(Arc::clone(src)),
        },
        _ => Error::Schema {
            path: path.to_path_buf(),
            field: "machines",
            index: Some(index),
            span,
            src: Some(Arc::clone(src)),
            message: source.to_string().into_boxed_str(),
        },
    }
}

/// Re-map thermal-bank builder errors to the top-level `thermal_bank` section.
pub fn map_thermal_facility_build_error(
    path: &Path,
    src: &Arc<str>,
    span: Option<Range<usize>>,
    source: CatalogBuildError,
) -> Error {
    match source {
        CatalogBuildError::DuplicateFacilityKey(key) => Error::DuplicateKey {
            path: path.to_path_buf(),
            kind: "facility",
            key: key.to_string().into_boxed_str(),
            span,
            src: Some(Arc::clone(src)),
        },
        _ => Error::Schema {
            path: path.to_path_buf(),
            field: "thermal_bank",
            index: None,
            span,
            src: Some(Arc::clone(src)),
            message: source.to_string().into_boxed_str(),
        },
    }
}

/// Collapse recipe build errors into the most specific TOML field path possible.
pub fn map_recipe_build_error(
    path: &Path,
    src: &Arc<str>,
    index: usize,
    spans: RecipeSpanContext,
    source: CatalogBuildError,
) -> Error {
    let (field, span) = match source {
        CatalogBuildError::RecipeIngredientsMustNotBeEmpty
        | CatalogBuildError::DuplicateRecipeIngredientItem { .. } => (
            "recipes.ingredients",
            spans.ingredients.or_else(|| spans.recipe.clone()),
        ),

        CatalogBuildError::DuplicateRecipeProductItem { .. } => (
            "recipes.products",
            spans.products.or_else(|| spans.recipe.clone()),
        ),
        _ => ("recipes", spans.recipe),
    };
    Error::Schema {
        path: path.to_path_buf(),
        field,
        index: Some(index),
        span,
        src: Some(Arc::clone(src)),
        message: source.to_string().into_boxed_str(),
    }
}

/// Collapse power-recipe build errors into the most specific TOML field path possible.
pub fn map_power_recipe_build_error(
    path: &Path,
    src: &Arc<str>,
    index: usize,
    recipe_span: Option<Range<usize>>,
    ingredient_span: Option<Range<usize>>,
    source: CatalogBuildError,
) -> Error {
    let _ = ingredient_span;
    let (field, span) = ("power_recipes", recipe_span);
    Error::Schema {
        path: path.to_path_buf(),
        field,
        index: Some(index),
        span,
        src: Some(Arc::clone(src)),
        message: source.to_string().into_boxed_str(),
    }
}

/// Translate model-level AIC build errors into crate-level loading errors.
pub fn map_aic_build_error(
    path: PathBuf,
    src: Arc<str>,
    source: AicBuildError,
    outpost_spans: &BTreeMap<String, Range<usize>>,
) -> Error {
    match source {
        AicBuildError::DuplicateOutpostKey { key } => Error::DuplicateKey {
            path,
            kind: "outpost",
            key: key.to_string().into_boxed_str(),
            span: outpost_spans.get(key.as_str()).cloned(),
            src: Some(src),
        },
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::render_span_error;

    #[test]
    fn render_span_error_uses_line_and_column_when_source_is_available() {
        let src = "[[outposts]]\nkey = \"Dup\"\n";
        let span = src.find("Dup").expect("must find key");
        let rendered = render_span_error(
            "duplicate outpost key `Dup` in aic.toml".to_string(),
            Some(&(span..span + 3)),
            Some(src),
        );

        assert!(
            rendered.contains("line 2, column 8"),
            "rendered output: {rendered}"
        );
        assert!(
            rendered.contains("2 | key = \"Dup\""),
            "rendered output: {rendered}"
        );
        assert!(rendered.contains("^^^"), "rendered output: {rendered}");
    }

    #[test]
    fn render_span_error_falls_back_to_byte_span_without_source() {
        let rendered = render_span_error(
            "unknown item `X` in aic.toml".to_string(),
            Some(&(8..9)),
            None,
        );
        assert!(
            rendered.ends_with("span=8..9"),
            "rendered output: {rendered}"
        );
    }

    #[test]
    fn render_span_error_handles_non_char_boundary_span() {
        let src = "v = \"\u{4E2D}\"\n";
        let ch = '\u{4E2D}';
        let span_start = src.find('\u{4E2D}').expect("must find character");
        let rendered = render_span_error(
            "unknown item `X` in aic.toml".to_string(),
            Some(&(span_start + 1..span_start + 2)),
            Some(src),
        );

        assert!(
            rendered.contains("line 1, column 6"),
            "rendered output: {rendered}"
        );
        assert!(
            rendered.contains(format!("1 | v = \"{ch}\"").as_str()),
            "rendered output: {rendered}"
        );
        assert!(rendered.contains("^"), "rendered output: {rendered}");
    }
}
