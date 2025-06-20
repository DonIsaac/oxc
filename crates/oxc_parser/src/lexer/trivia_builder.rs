use memchr::memchr_iter;
use oxc_ast::ast::{Comment, CommentContent, CommentKind, CommentPosition};
use oxc_span::Span;

use super::{Kind, Token};

#[derive(Debug)]
pub struct TriviaBuilder {
    // This is a set of unique comments. Duplicated
    // comments could be generated in case of rewind; they are
    // filtered out at insertion time.
    pub(crate) comments: Vec<Comment>,

    pub(crate) irregular_whitespaces: Vec<Span>,

    // states
    /// index of processed comments
    processed: usize,

    /// Saw a newline before this position
    saw_newline: bool,

    /// Previous token kind, used to indicates comments are trailing from what kind
    previous_kind: Kind,

    pub(super) has_pure_comment: bool,

    pub(super) has_no_side_effects_comment: bool,
}

impl Default for TriviaBuilder {
    fn default() -> Self {
        Self {
            comments: vec![],
            irregular_whitespaces: vec![],
            processed: 0,
            saw_newline: true,
            previous_kind: Kind::Undetermined,
            has_pure_comment: false,
            has_no_side_effects_comment: false,
        }
    }
}

impl TriviaBuilder {
    pub fn previous_token_has_pure_comment(&self) -> bool {
        self.has_pure_comment
    }

    pub fn previous_token_has_no_side_effects_comment(&self) -> bool {
        self.has_no_side_effects_comment
    }

    pub fn add_irregular_whitespace(&mut self, start: u32, end: u32) {
        self.irregular_whitespaces.push(Span::new(start, end));
    }

    pub fn add_line_comment(&mut self, start: u32, end: u32, source_text: &str) {
        self.add_comment(Comment::new(start, end, CommentKind::Line), source_text);
    }

    pub fn add_block_comment(&mut self, start: u32, end: u32, source_text: &str) {
        self.add_comment(Comment::new(start, end, CommentKind::Block), source_text);
    }

    // For block comments only. This function is not called after line comments because the lexer skips
    // newline after line comments.
    pub fn handle_newline(&mut self) {
        // The last unprocessed comment is on a newline.
        let len = self.comments.len();
        if self.processed < len {
            self.comments[len - 1].set_followed_by_newline(true);
            if !self.saw_newline {
                self.processed = self.comments.len();
            }
        }
        self.saw_newline = true;
    }

    pub fn handle_token(&mut self, token: Token) {
        let len = self.comments.len();
        self.previous_kind = token.kind();
        if self.processed < len {
            // All unprocessed preceding comments are leading comments attached to this token start.
            for comment in &mut self.comments[self.processed..] {
                comment.position = CommentPosition::Leading;
                comment.attached_to = token.start();
            }
            self.processed = len;
        }
        self.saw_newline = false;
    }

    /// Determines if the current line comment should be treated as a trailing comment.
    ///
    /// A line comment should be treated as trailing when both of the following conditions are met:
    ///
    /// 1. It is not preceded by a newline.
    ///
    /// ```javascript
    /// let x = 5; // This should be treated as a trailing comment
    /// foo(); // This should also be treated as a trailing comment
    ///
    /// // This should not be treated as trailing (preceded by newline)
    /// let x = 5;
    /// ```
    ///
    /// 2. It does not immediately follow an `=` [`Kind::Eq`] or `(` [`Kind::LParen`]
    ///    token.
    ///
    /// ```javascript
    /// let y = // This should not be treated as trailing (follows `=`)
    ///     10;
    ///
    /// function foo( // This should not be treated as trailing (follows `(`)
    ///     param
    /// ) {}
    /// ```
    fn should_be_treated_as_trailing_comment(&self) -> bool {
        !self.saw_newline && !matches!(self.previous_kind, Kind::Eq | Kind::LParen)
    }

    fn add_comment(&mut self, mut comment: Comment, source_text: &str) {
        self.parse_annotation(&mut comment, source_text);
        // The comments array is an ordered vec, only add the comment if its not added before,
        // to avoid situations where the parser needs to rewind and tries to reinsert the comment.
        if let Some(last_comment) = self.comments.last() {
            if comment.span.start <= last_comment.span.start {
                return;
            }
        }

        // This newly added comment may be preceded by a newline.
        comment.set_preceded_by_newline(self.saw_newline);
        if comment.is_line() {
            // A line comment is always followed by a newline. This is never set in `handle_newline`.
            comment.set_followed_by_newline(true);
            if self.should_be_treated_as_trailing_comment() {
                self.processed = self.comments.len() + 1; // +1 to include this comment.
            }
            self.saw_newline = true;
        }

        self.comments.push(comment);
    }

    /// Parse Notation
    fn parse_annotation(&mut self, comment: &mut Comment, source_text: &str) {
        let mut s = comment.content_span().source_text(source_text);

        if s.starts_with('!') {
            comment.content = CommentContent::Legal;
            return;
        }

        if comment.is_block() && s.starts_with('*') {
            // Ignore webpack comment `/*****/`
            if !s.bytes().all(|c| c == b'*') {
                if contains_license_or_preserve_comment(s) {
                    comment.content = CommentContent::JsdocLegal;
                } else {
                    comment.content = CommentContent::Jsdoc;
                }
                return;
            }
        }

        s = s.trim_ascii_start();

        if let Some(ss) = s.strip_prefix('@') {
            if ss.starts_with("vite") {
                comment.content = CommentContent::Vite;
                return;
            }
            if ss.starts_with("license") || ss.starts_with("preserve") {
                comment.content = CommentContent::Legal;
                return;
            }
            s = ss;
        } else if let Some(ss) = s.strip_prefix('#') {
            s = ss;
        } else if s
            .strip_prefix("webpack")
            .and_then(|s| s.bytes().next())
            .is_some_and(|b| b.is_ascii_uppercase())
        {
            comment.content = CommentContent::Webpack;
            return;
        } else if ["v8 ignore", "c8 ignore", "node:coverage", "istanbul ignore"]
            .iter()
            .any(|ss| s.starts_with(ss))
        {
            comment.content = CommentContent::CoverageIgnore;
        } else {
            if contains_license_or_preserve_comment(s) {
                comment.content = CommentContent::Legal;
            }
            return;
        }

        let Some(s) = s.strip_prefix("__") else { return };
        if s.starts_with("PURE__") {
            comment.content = CommentContent::Pure;
            self.has_pure_comment = true;
        }
        if s.starts_with("NO_SIDE_EFFECTS__") {
            comment.content = CommentContent::NoSideEffects;
            self.has_no_side_effects_comment = true;
        }
    }
}

#[expect(clippy::inline_always)]
#[inline(always)]
fn contains_license_or_preserve_comment(s: &str) -> bool {
    let hay = s.as_bytes();

    if hay.len() < 9 {
        return false;
    }

    let search_len = hay.len() - 8;

    for i in memchr_iter(b'@', &hay[..search_len]) {
        debug_assert!(i < search_len);
        // SAFETY: we `i` has a max val of len of bytes - 8, so accessing `i + 1` is safe
        match unsafe { hay.get_unchecked(i + 1) } {
            // spellchecker:off
            b'l' => {
                // SAFETY: we `i` has a max val of len of bytes - 8, so accessing `i + 7` is safe
                if unsafe { hay.get_unchecked(i + 2..i + 1 + 7) } == b"icense" {
                    return true;
                }
            }
            b'p' => {
                // SAFETY: we `i` has a max val of len of bytes - 8, so accessing `i + 8` is safe
                if unsafe { hay.get_unchecked(i + 2..i + 1 + 8) } == b"reserve" {
                    return true;
                }
            }
            // spellchecker:on
            _ => {}
        }
    }

    false
}

#[cfg(test)]
mod test {
    use oxc_allocator::Allocator;
    use oxc_ast::{Comment, CommentContent, CommentKind, CommentPosition, ast::CommentNewlines};
    use oxc_span::{SourceType, Span};

    use crate::Parser;

    fn get_comments(source_text: &str) -> Vec<Comment> {
        let allocator = Allocator::default();
        let source_type = SourceType::default();
        let ret = Parser::new(&allocator, source_text, source_type).parse();
        ret.program.comments.iter().copied().collect::<Vec<_>>()
    }

    #[test]
    fn comment_attachments() {
        let source_text = "
        /* Leading 1 */
        // Leading 2
        /* Leading 3 */ token /* Trailing 1 */ // Trailing 2
        // Leading of EOF token
        ";
        let comments = get_comments(source_text);
        let expected = [
            Comment {
                span: Span::new(9, 24),
                kind: CommentKind::Block,
                position: CommentPosition::Leading,
                attached_to: 70,
                newlines: CommentNewlines::Leading | CommentNewlines::Trailing,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(33, 45),
                kind: CommentKind::Line,
                position: CommentPosition::Leading,
                attached_to: 70,
                newlines: CommentNewlines::Leading | CommentNewlines::Trailing,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(54, 69),
                kind: CommentKind::Block,
                position: CommentPosition::Leading,
                attached_to: 70,
                newlines: CommentNewlines::Leading,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(76, 92),
                kind: CommentKind::Block,
                position: CommentPosition::Trailing,
                attached_to: 0,
                newlines: CommentNewlines::None,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(93, 106),
                kind: CommentKind::Line,
                position: CommentPosition::Trailing,
                attached_to: 0,
                newlines: CommentNewlines::Trailing,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(115, 138),
                kind: CommentKind::Line,
                position: CommentPosition::Leading,
                attached_to: 147,
                newlines: CommentNewlines::Leading | CommentNewlines::Trailing,
                content: CommentContent::None,
            },
        ];

        assert_eq!(comments.len(), expected.len());
        for (comment, expected) in comments.iter().copied().zip(expected) {
            assert_eq!(comment, expected, "{}", comment.content_span().source_text(source_text));
        }
    }

    #[test]
    fn comment_attachments2() {
        let source_text = "#!/usr/bin/env node
/* Leading 1 */
token /* Trailing 1 */
        ";
        let comments = get_comments(source_text);
        let expected = vec![
            Comment {
                span: Span::new(20, 35),
                kind: CommentKind::Block,
                position: CommentPosition::Leading,
                attached_to: 36,
                newlines: CommentNewlines::Leading | CommentNewlines::Trailing,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(42, 58),
                kind: CommentKind::Block,
                position: CommentPosition::Trailing,
                attached_to: 0,
                newlines: CommentNewlines::Trailing,
                content: CommentContent::None,
            },
        ];
        assert_eq!(comments, expected);
    }

    #[test]
    fn comment_attachments3() {
        let source_text = "
/*
 * A
 **/
/*
 * B
 **/
 token
        ";
        let comments = get_comments(source_text);
        let expected = vec![
            Comment {
                span: Span::new(1, 13),
                kind: CommentKind::Block,
                position: CommentPosition::Leading,
                attached_to: 28,
                newlines: CommentNewlines::Leading | CommentNewlines::Trailing,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(14, 26),
                kind: CommentKind::Block,
                position: CommentPosition::Leading,
                attached_to: 28,
                newlines: CommentNewlines::Leading | CommentNewlines::Trailing,
                content: CommentContent::None,
            },
        ];
        assert_eq!(comments, expected);
    }

    #[test]
    fn leading_comments_after_eq() {
        let source_text = "
            const v1 = // Leading comment 1
            foo();
            function foo(param =// Leading comment 2
            new Foo()
            ) {}
        ";
        let comments = get_comments(source_text);
        let expected = vec![
            Comment {
                span: Span::new(24, 44),
                kind: CommentKind::Line,
                position: CommentPosition::Leading,
                attached_to: 57,
                newlines: CommentNewlines::Trailing,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(96, 116),
                kind: CommentKind::Line,
                position: CommentPosition::Leading,
                attached_to: 129,
                newlines: CommentNewlines::Trailing,
                content: CommentContent::None,
            },
        ];
        assert_eq!(comments, expected);
    }

    #[test]
    fn leading_comments_after_left_parenthesis() {
        let source_text = "
            call(// Leading comment 1
                arguments)
            (// Leading comment 2
                arguments)
        ";
        let comments = get_comments(source_text);
        let expected = vec![
            Comment {
                span: Span::new(18, 38),
                kind: CommentKind::Line,
                position: CommentPosition::Leading,
                attached_to: 55,
                newlines: CommentNewlines::Trailing,
                content: CommentContent::None,
            },
            Comment {
                span: Span::new(79, 99),
                kind: CommentKind::Line,
                position: CommentPosition::Leading,
                attached_to: 116,
                newlines: CommentNewlines::Trailing,
                content: CommentContent::None,
            },
        ];
        assert_eq!(comments, expected);
    }

    #[test]
    fn comment_parsing() {
        let data = [
            ("/*! legal */", CommentContent::Legal),
            ("/* @preserve */", CommentContent::Legal),
            ("/* @license */", CommentContent::Legal),
            ("/* foo @preserve */", CommentContent::Legal),
            ("/* foo @license */", CommentContent::Legal),
            ("/** foo @preserve */", CommentContent::JsdocLegal),
            ("/** foo @license */", CommentContent::JsdocLegal),
            ("/** jsdoc */", CommentContent::Jsdoc),
            ("/**/", CommentContent::None),
            ("/***/", CommentContent::None),
            ("/*@*/", CommentContent::None),
            ("/*@xreserve*/", CommentContent::None),
            ("/*@preserve*/", CommentContent::Legal),
            ("/*@voidzeroignoreme*/", CommentContent::None),
            ("/****/", CommentContent::None),
            ("/* @vite-ignore */", CommentContent::Vite),
            ("/* @vite-xxx */", CommentContent::Vite),
            ("/* webpackChunkName: 'my-chunk-name' */", CommentContent::Webpack),
            ("/* webpack */", CommentContent::None),
            ("/* @__PURE__ */", CommentContent::Pure),
            ("/* @__NO_SIDE_EFFECTS__ */", CommentContent::NoSideEffects),
            ("/* #__PURE__ */", CommentContent::Pure),
            ("/* #__NO_SIDE_EFFECTS__ */", CommentContent::NoSideEffects),
        ];

        for (source_text, expected) in data {
            let comments = get_comments(source_text);
            assert_eq!(comments.len(), 1, "{source_text}");
            assert_eq!(comments[0].content, expected, "{source_text}");
        }
    }
}
