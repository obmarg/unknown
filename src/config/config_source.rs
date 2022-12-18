use std::{fmt, sync::Arc};

use camino::Utf8PathBuf;
use miette::MietteSpanContents;

#[derive(Clone)]
pub struct ConfigSource {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for ConfigSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfigSource")
            .field("filename", &self.inner.filename)
            .finish_non_exhaustive()
    }
}

struct Inner {
    filename: String,
    code: String,
}

impl ConfigSource {
    pub fn new(filename: impl AsRef<str>, code: impl Into<String>) -> Self {
        ConfigSource {
            inner: Arc::new(Inner {
                filename: filename.as_ref().to_string(),
                code: code.into(),
            }),
        }
    }

    // pub fn to_miette(&self) -> miette::NamedSource {
    //     miette::NamedSource::new(&self.inner.filename, self.inner.code.clone())
    // }
}

impl miette::SourceCode for ConfigSource {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        let contents =
            self.inner
                .code
                .read_span(span, context_lines_before, context_lines_after)?;

        Ok(Box::new(MietteSpanContents::new_named(
            self.inner.filename.clone(),
            contents.data(),
            *contents.span(),
            contents.line(),
            contents.column(),
            contents.line_count(),
        )))
    }
}
