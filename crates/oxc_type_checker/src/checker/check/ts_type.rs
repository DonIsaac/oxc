use super::{Check, CheckContext, CheckMode, Checker};
#[allow(clippy::wildcard_imports)]
use oxc_ast::ast::*;

impl<'a> Check<'a> for TSLiteral<'a> {
    fn check(&self, checker: &mut Checker<'a>, ctx: &CheckContext) -> oxc_syntax::types::TypeId {
        match self {
            Self::BigIntLiteral(lit) => lit.check(checker, ctx),
            Self::BooleanLiteral(lit) => lit.check(checker, ctx),
            Self::NullLiteral(lit) => lit.check(checker, ctx),
            Self::NumericLiteral(lit) => lit.check(checker, ctx),
            Self::RegExpLiteral(lit) => lit.check(checker, ctx),
            Self::StringLiteral(lit) => lit.check(checker, ctx),
            Self::TemplateLiteral(lit) => lit.check(checker, ctx),
            Self::UnaryExpression(lit) => lit.check(checker, ctx),
        }
    }
}
