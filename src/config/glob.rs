use knuffel::{
    ast::Literal, decode::Kind, errors::DecodeError, span::Spanned, traits::ErrorSpan, DecodeScalar,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Glob(globset::Glob);

impl Glob {
    pub fn into_inner(self) -> globset::Glob {
        self.0
    }
}

impl<S> DecodeScalar<S> for Glob
where
    S: ErrorSpan,
{
    fn type_check(
        _type_name: &Option<knuffel::span::Spanned<knuffel::ast::TypeName, S>>,
        _ctx: &mut knuffel::decode::Context<S>,
    ) {
        // Not bothering with types for now...
    }

    fn raw_decode(
        value: &Spanned<Literal, S>,
        _ctx: &mut knuffel::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        let Literal::String(s) = &**value else {
            let found =  match **value {
                Literal::Null => Kind::Null,
                Literal::Bool(_) => Kind::Bool,
                Literal::Int(_) => Kind::Int,
                Literal::Decimal(_) => Kind::Decimal,
                Literal::String(_) => panic!("this should be impossible")
            };
            return Err(DecodeError::ScalarKind {
                span: value.span().to_owned(),
                expected: Kind::String.into(),
                found
            });
        };

        let glob = globset::Glob::new(s.as_ref()).map_err(|error| DecodeError::Conversion {
            span: value.span().to_owned(),
            source: Box::new(error),
        })?;

        Ok(Glob(glob))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(knuffel::Decode, Debug)]
    pub struct TestStruct {
        #[knuffel(children(name = "path"), unwrap(argument))]
        pub paths: Vec<Glob>,
    }

    #[test]
    fn test_decoding_globs() {
        let result = knuffel::parse::<TestStruct>(
            "whatevs.txt",
            r#"
        path "hello/**"
        path "**"
        path "*.txt"
        path "a_file.txt"
        "#,
        )
        .unwrap();

        insta::assert_debug_snapshot!(result);
    }
}
