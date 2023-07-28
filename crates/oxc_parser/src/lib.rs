//! Oxc Parser for JavaScript and TypeScript
//!
//! # Performance
//!
//! The following optimization techniques are used:
//! * AST is allocated in a memory arena ([bumpalo](https://docs.rs/bumpalo)) for fast AST drop
//! * Short strings are inlined by [CompactString](https://github.com/ParkMyCar/compact_str)
//! * No other heap allocations are done except the above two
//! * [oxc_span::Span] offsets uses `u32` instead of `usize`
//! * Scope binding, symbol resolution and complicated syntax errors are not done in the parser,
//! they are deligated to the [semantic analyzer](https://docs.rs/oxc_semantic)
//!
//! # Conformance
//! The parser parses all of Test262 and most of Babel and TypeScript parser conformance tests.
//!
//! See [oxc coverage](https://github.com/Boshen/oxc/tree/main/tasks/coverage) for details
//! ```
//! Test262 Summary:
//! AST Parsed     : 44000/44000 (100.00%)
//!
//! Babel Summary:
//! AST Parsed     : 2065/2071 (99.71%)
//!
//! TypeScript Summary:
//! AST Parsed     : 2337/2337 (100.00%)
//! ```
//!
//! # Usage
//!
//! The parser has a minimal API with three inputs and one return struct ([ParserReturn]).
//!
//! ```rust
//! let parser_return = Parser::new(&allocator, &source_text, source_type).parse();
//! ```
//!
//! # Example
//! <https://github.com/Boshen/oxc/blob/main/crates/oxc_parser/examples/parser.rs>
//!
//! ```rust
#![doc = include_str!("../examples/parser.rs")]
//! ```
//!
//! # Visitor
//!
//! See [oxc_ast::Visit] and [oxc_ast::VisitMut]
//!
//! # Visiting without a visitor
//!
//! For ad-hoc tasks, the semantic analyzer can be used to get a parent pointing tree with untyped nodes,
//! the nodes can be iterated through a sequential loop.
//!
//! ```rust
//! for node in semantic.nodes().iter() {
//!     match node.kind() {
//!         // check node
//!     }
//! }
//! ```
//!
//! See [full linter example](https://github.com/Boshen/oxc/blob/ab2ef4f89ba3ca50c68abb2ca43e36b7793f3673/crates/oxc_linter/examples/linter.rs#L38-L39)

#![allow(clippy::wildcard_imports)] // allow for use `oxc_ast::ast::*`

mod context;
mod cursor;
mod list;
mod state;

mod js;
mod jsx;
mod ts;

mod diagnostics;
mod lexer;

use std::rc::Rc;

use context::{Context, StatementContext};
use oxc_allocator::Allocator;
use oxc_ast::{ast::Program, AstBuilder, Trivias};
use oxc_diagnostics::{Error, Result};
use oxc_span::{ModuleKind, SourceType, Span};

use crate::{
    lexer::{Kind, Lexer, Token},
    state::ParserState,
};

/// Return value of parser consisting of AST, errors and comments
///
/// The parser always return a valid AST.
/// When `panicked = true`, then program will always be empty.
/// When `errors.len() > 0`, then program may or may not be empty due to error recovery.
pub struct ParserReturn<'a> {
    pub program: Program<'a>,
    pub errors: Vec<Error>,
    pub trivias: Rc<Trivias>,
    pub panicked: bool,
}

/// Recursive Descent Parser for ECMAScript and TypeScript
///
/// See [`Parser::parse`] for entry function.
pub struct Parser<'a> {
    lexer: Lexer<'a>,

    /// SourceType: JavaScript or TypeScript, Script or Module, jsx support?
    source_type: SourceType,

    /// Source Code
    source_text: &'a str,

    /// All syntax errors from parser and lexer
    /// Note: favor adding to `Diagnostics` instead of raising Err
    errors: Vec<Error>,

    /// The current parsing token
    token: Token<'a>,

    /// The end range of the previous token
    prev_token_end: u32,

    /// Parser state
    state: ParserState<'a>,

    /// Parsing context
    ctx: Context,

    /// Ast builder for creating AST spans
    ast: AstBuilder<'a>,
}

impl<'a> Parser<'a> {
    /// Create a new parser
    pub fn new(allocator: &'a Allocator, source_text: &'a str, source_type: SourceType) -> Self {
        Self {
            lexer: Lexer::new(allocator, source_text, source_type),
            source_type,
            source_text,
            errors: vec![],
            token: Token::default(),
            prev_token_end: 0,
            state: ParserState::new(allocator),
            ctx: Self::default_context(source_type),
            ast: AstBuilder::new(allocator),
        }
    }

    /// Allow return outside of function
    ///
    /// By default, a return statement at the top level raises an error.
    /// Set this to true to accept such code.
    #[must_use]
    pub fn allow_return_outside_function(mut self, allow: bool) -> Self {
        self.ctx = self.ctx.and_return(allow);
        self
    }

    /// Main entry point
    ///
    /// Returns an empty `Program` on unrecoverable error,
    /// Recoverable errors are stored inside `errors`.
    pub fn parse(mut self) -> ParserReturn<'a> {
        let (program, panicked) = match self.parse_program() {
            Ok(program) => (program, false),
            Err(error) => {
                self.error(self.flow_error().unwrap_or(error));
                let program = self.ast.program(
                    Span::default(),
                    self.source_type,
                    self.ast.new_vec(),
                    None,
                    self.ast.new_vec(),
                );
                (program, true)
            }
        };
        let errors = self.lexer.errors.into_iter().chain(self.errors).collect();
        let trivias = self.lexer.trivia_builder.build();
        ParserReturn { program, errors, trivias, panicked }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn parse_program(&mut self) -> Result<Program<'a>> {
        // initialize cur_token and prev_token by moving onto the first token
        self.bump_any();

        let hashbang = self.parse_hashbang();
        let (directives, statements) =
            self.parse_directives_and_statements(/* is_top_level */ true)?;

        let span = Span::new(0, self.source_text.len() as u32);
        Ok(self.ast.program(span, self.source_type, directives, hashbang, statements))
    }

    fn default_context(source_type: SourceType) -> Context {
        let ctx = Context::default().and_ambient(source_type.is_typescript_definition());
        match source_type.module_kind() {
            ModuleKind::Script => ctx,
            // for [top-level-await](https://tc39.es/proposal-top-level-await/)
            ModuleKind::Module => ctx.and_await(true),
        }
    }

    /// Check for Flow declaration if the file cannot be parsed.
    /// The declaration must be [on the first line before any code](https://flow.org/en/docs/usage/#toc-prepare-your-code-for-flow)
    fn flow_error(&self) -> Option<Error> {
        if self.source_type.is_javascript()
            && (self.source_text.starts_with("// @flow")
                || self.source_text.starts_with("/* @flow */"))
        {
            return Some(diagnostics::Flow(Span::new(0, 8)).into());
        }
        None
    }

    /// Return error info at current token
    /// # Panics
    ///   * The lexer did not push a diagnostic when `Kind::Undetermined` is returned
    fn unexpected(&mut self) -> Error {
        // The lexer should have reported a more meaningful diagnostic
        // when it is a undetermined kind.
        if self.cur_kind() == Kind::Undetermined {
            if let Some(error) = self.lexer.errors.pop() {
                return error;
            }
        }
        diagnostics::UnexpectedToken(self.cur_token().span()).into()
    }

    /// Push a Syntax Error
    fn error<T: Into<Error>>(&mut self, error: T) {
        self.errors.push(error.into());
    }

    fn ts_enabled(&self) -> bool {
        self.source_type.is_typescript()
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn smoke_test() {
        let allocator = Allocator::default();
        let source_type = SourceType::default();
        let source = "";
        let ret = Parser::new(&allocator, source, source_type).parse();
        assert!(ret.program.is_empty());
        assert!(ret.errors.is_empty());
    }

    #[test]
    fn flow_error() {
        let allocator = Allocator::default();
        let source_type = SourceType::default();
        let source = "// @flow\nasdf adsf";
        let ret = Parser::new(&allocator, source, source_type).parse();
        assert!(ret.program.is_empty());
        assert_eq!(ret.errors.first().unwrap().to_string(), "Flow is not supported");

        let source = "/* @flow */\n asdf asdf";
        let ret = Parser::new(&allocator, source, source_type).parse();
        assert!(ret.program.is_empty());
        assert_eq!(ret.errors.first().unwrap().to_string(), "Flow is not supported");
    }
}
