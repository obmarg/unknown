use std::{fmt, ops::Deref};

use knuffel::traits::ErrorSpan;

#[derive(Clone)]
pub struct Spanned<T> {
    inner: T,
    pub span: miette::SourceSpan,
}

impl<T> Spanned<T> {
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T, U> AsRef<U> for Spanned<T>
where
    T: AsRef<U>,
{
    fn as_ref(&self) -> &U {
        self.inner.as_ref()
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> fmt::Debug for Spanned<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> fmt::Display for Spanned<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<S, T> knuffel::DecodeScalar<S> for Spanned<T>
where
    T: knuffel::DecodeScalar<S>,
    S: ErrorSpan,
{
    fn type_check(
        _type_name: &Option<knuffel::span::Spanned<knuffel::ast::TypeName, S>>,
        _ctx: &mut knuffel::decode::Context<S>,
    ) {
        // Not doing this.
    }

    fn raw_decode(
        value: &knuffel::span::Spanned<knuffel::ast::Literal, S>,
        ctx: &mut knuffel::decode::Context<S>,
    ) -> Result<Self, knuffel::errors::DecodeError<S>> {
        let inner = T::raw_decode(value, ctx)?;
        let span = value.span().clone().into();

        Ok(Spanned { inner, span })
    }
}

pub trait WithSpan: Sized {
    fn with_span(self, span: miette::SourceSpan) -> Spanned<Self>;
}

impl<T> WithSpan for T {
    fn with_span(self, span: miette::SourceSpan) -> Spanned<Self> {
        Spanned { inner: self, span }
    }
}

pub trait SourceSpanExt {
    fn subspan(&self, start: usize, len: usize) -> miette::SourceSpan;
}

impl SourceSpanExt for miette::SourceSpan {
    fn subspan(&self, start: usize, len: usize) -> miette::SourceSpan {
        let offset = self.offset();
        let current_len = self.len();

        if len > current_len {
            panic!("tried to make an invalid subspan");
        }

        let start = offset + start;

        miette::SourceSpan::from((start, start + len))
    }
}
