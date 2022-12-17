//! Some diagnostics machinery built on miette

use std::{error::Error, fmt::Display};

use miette::{Diagnostic, SourceCode};

pub struct DynDiagnostic {
    inner: Box<dyn Diagnostic + Send + Sync + 'static>,
    source_code: Option<Box<dyn miette::SourceCode + 'static>>,
}

impl std::fmt::Debug for DynDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynDiagnostic")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl Display for DynDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Error for DynDiagnostic {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}

impl Diagnostic for DynDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.inner.code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.inner.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.inner.help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.inner.url()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.source_code
            .as_ref()
            .map(|s| s.as_ref())
            .or_else(|| self.inner.source_code())
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.inner.labels()
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        self.inner.related()
    }
}

impl DynDiagnostic {
    pub fn new(diagnostic: impl Diagnostic + Send + Sync + 'static) -> Self {
        DynDiagnostic {
            inner: diagnostic.into(),
            source_code: None,
        }
    }

    pub fn with_source_code(mut self, source_code: impl SourceCode + 'static) -> Self {
        // If inner already has source_code we trust that is correct and leave it alone
        if let None = self.inner.source_code() {
            self.source_code = Some(Box::new(source_code));
        }
        self
    }
}
