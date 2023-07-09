use oxc_ast::{AstKind, ast::{Statement, Expression}};
use oxc_diagnostics::{
    miette::{self, Diagnostic},
    thiserror::Error,
};
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;
use regex::Regex;

use crate::{context::LintContext, rule::Rule, AstNode};

#[derive(Debug, Error, Diagnostic)]
#[error("eslint(no-fallthrough): Disallow fallthrough of `case` statements")]
#[diagnostic(severity(warning), help("Expected a 'break' statement."))]
struct NoFallthroughDiagnostic(#[label] pub Span);

#[derive(Debug, Default, Clone)]
pub struct NoFallthrough {
    /// Set the `commentPattern` option to a regular expression string to change
    /// the test for intentional fallthrough comment. If the fallthrough comment
    /// matches a directive, that takes precedence over commentPattern.
    comment_pattern: Option<Regex>,

    /// Set the `allowEmptyCase` option to `true` to allow empty cases regardless of
    /// the layout. By default, this rule does not require a fallthrough comment
    /// after an empty case only if the empty case and the next case are on the
    /// same line or on consecutive lines.
    allow_empty_case: bool,
}

declare_oxc_lint!(
    /// ### What it does
    ///
    ///
    /// ### Why is this bad?
    ///
    ///
    /// ### Example
    /// ```javascript
    /// ```
    NoFallthrough,
    correctness
);

impl NoFallthrough {

    fn comment_matches(&self) -> bool {
        false // todo
    }
}

impl Rule for NoFallthrough {
    fn from_configuration(value: serde_json::Value) -> Self {
        let (comment_pattern, allow_empty_case) =
            value.get(0).map_or((Default::default(), Default::default()), |config| {
                (
                    config
                        .get("commentPattern")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|pattern| Some(Regex::new(pattern).unwrap())),
                    config
                        .get("allowEmptyCase")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or_default(),
                )
            });
        Self { comment_pattern, allow_empty_case }
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let AstKind::SwitchCase(case) = node.kind() else { return };

        if case.consequent.is_empty() {
            // if self.
        } else {

        }
    }
}

/// Returns `true` if the statement `return`s, `break`s, `continue`s, or `throw`s
/// Kinda the inverse of reachable, but not really
fn does_statement_branch<'a>(stmt: &Statement<'a>) -> bool {
    match stmt => {

    }
}

/// Related to [`does_statement_jump`]
fn does_expr_jump<'a>(expr: &Expression<'a>) -> bool {

}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        ("switch(foo) { case 0: a(); /* falls through */ case 1: b(); }", None),
        ("switch(foo) { case 0: a()\n /* falls through */ case 1: b(); }", None),
        ("switch(foo) { case 0: a(); /* fall through */ case 1: b(); }", None),
        ("switch(foo) { case 0: a(); /* fallthrough */ case 1: b(); }", None),
        ("switch(foo) { case 0: a(); /* FALLS THROUGH */ case 1: b(); }", None),
        ("switch(foo) { case 0: { a(); /* falls through */ } case 1: b(); }", None),
        ("switch(foo) { case 0: { a()\n /* falls through */ } case 1: b(); }", None),
        ("switch(foo) { case 0: { a(); /* fall through */ } case 1: b(); }", None),
        ("switch(foo) { case 0: { a(); /* fallthrough */ } case 1: b(); }", None),
        ("switch(foo) { case 0: { a(); /* FALLS THROUGH */ } case 1: b(); }", None),
        ("switch(foo) { case 0: { a(); } /* falls through */ case 1: b(); }", None),
        ("switch(foo) { case 0: { a(); /* falls through */ } /* comment */ case 1: b(); }", None),
        ("switch(foo) { case 0: { /* falls through */ } case 1: b(); }", None),
        ("function foo() { switch(foo) { case 0: a(); return; case 1: b(); }; }", None),
        ("switch(foo) { case 0: a(); throw 'foo'; case 1: b(); }", None),
        ("while (a) { switch(foo) { case 0: a(); continue; case 1: b(); } }", None),
        ("switch(foo) { case 0: a(); break; case 1: b(); }", None),
        ("switch(foo) { case 0: case 1: a(); break; case 2: b(); }", None),
        ("switch(foo) { case 0: case 1: break; case 2: b(); }", None),
        ("switch(foo) { case 0: case 1: break; default: b(); }", None),
        ("switch(foo) { case 0: case 1: a(); }", None),
        ("switch(foo) { case 0: case 1: a(); break; }", None),
        ("switch(foo) { case 0: case 1: break; }", None),
        ("switch(foo) { case 0:\n case 1: break; }", None),
        ("switch(foo) { case 0: // comment\n case 1: break; }", None),
        ("function foo() { switch(foo) { case 0: case 1: return; } }", None),
        ("function foo() { switch(foo) { case 0: {return;}\n case 1: {return;} } }", None),
        ("switch(foo) { case 0: case 1: {break;} }", None),
        ("switch(foo) { }", None),
        (
            "switch(foo) { case 0: switch(bar) { case 2: break; } /* falls through */ case 1: break; }",
            None,
        ),
        ("function foo() { switch(foo) { case 1: return a; a++; }}", None),
        ("switch (foo) { case 0: a(); /* falls through */ default:  b(); /* comment */ }", None),
        ("switch (foo) { case 0: a(); /* falls through */ default: /* comment */ b(); }", None),
        ("switch (foo) { case 0: if (a) { break; } else { throw 0; } default: b(); }", None),
        ("switch (foo) { case 0: try { break; } finally {} default: b(); }", None),
        ("switch (foo) { case 0: try {} finally { break; } default: b(); }", None),
        ("switch (foo) { case 0: try { throw 0; } catch (err) { break; } default: b(); }", None),
        ("switch (foo) { case 0: do { throw 0; } while(a); default: b(); }", None),
        (
            "switch (foo) { case 0: a(); \n// eslint-disable-next-line no-fallthrough\n case 1: }",
            None,
        ),
        (
            "switch(foo) { case 0: a(); /* no break */ case 1: b(); }",
            Some(serde_json::json!([{
                "commentPattern": "no break"
            }])),
        ),
        (
            "switch(foo) { case 0: a(); /* no break: need to execute b() */ case 1: b(); }",
            Some(serde_json::json!([{
                "commentPattern": r"no break:\s?\w+"
            }])),
        ),
        (
            "switch(foo) { case 0: a();\n// need to execute b(), so\n// falling through\n case 1: b(); }",
            Some(serde_json::json!([{
                "commentPattern": "falling through"
            }])),
        ),
        (
            "switch(foo) { case 0: a(); /* break omitted */ default:  b(); /* comment */ }",
            Some(serde_json::json!([{
                "commentPattern": "break omitted"
            }])),
        ),
        (
            "switch(foo) { case 0: a(); /* caution: break is omitted intentionally */ case 1: b(); /* break omitted */ default: c(); }",
            Some(serde_json::json!([{
                "commentPattern": r"break[\s\w]+omitted"
            }])),
        ),
        (
            "switch(foo) { case 0: \n\n\n case 1: b(); }",
            Some(serde_json::json!([{ "allowEmptyCase": true }])),
        ),
        (
            "switch(foo) { case 0: \n /* with comments */  \n case 1: b(); }",
            Some(serde_json::json!([{ "allowEmptyCase": true }])),
        ),
        (
            "switch (a) {\n case 1: ; break; \n case 3: }",
            Some(serde_json::json!([{ "allowEmptyCase": true }])),
        ),
        (
            "switch (a) {\n case 1: ; break; \n case 3: }",
            Some(serde_json::json!([{ "allowEmptyCase": false }])),
        ),
    ];

    let fail = vec![
        ("switch(foo) { case 0: a();\ncase 1: b() }", None),
        ("switch(foo) { case 0: a();\ndefault: b() }", None),
        ("switch(foo) { case 0: a(); default: b() }", None),
        ("switch(foo) { case 0: if (a) { break; } default: b() }", None),
        ("switch(foo) { case 0: try { throw 0; } catch (err) {} default: b() }", None),
        ("switch(foo) { case 0: while (a) { break; } default: b() }", None),
        ("switch(foo) { case 0: do { break; } while (a); default: b() }", None),
        ("switch(foo) { case 0:\n\n default: b() }", None),
        ("switch(foo) { case 0: {} default: b() }", None),
        ("switch(foo) { case 0: a(); { /* falls through */ } default: b() }", None),
        ("switch(foo) { case 0: { /* falls through */ } a(); default: b() }", None),
        ("switch(foo) { case 0: if (a) { /* falls through */ } default: b() }", None),
        ("switch(foo) { case 0: { { /* falls through */ } } default: b() }", None),
        ("switch(foo) { case 0: { /* comment */ } default: b() }", None),
        ("switch(foo) { case 0:\n // comment\n default: b() }", None),
        ("switch(foo) { case 0: a(); /* falling through */ default: b() }", None),
        (
            "switch(foo) { case 0: a();\n/* no break */\ncase 1: b(); }",
            Some(serde_json::json!([{
                "commentPattern": "break omitted"
            }])),
        ),
        (
            "switch(foo) { case 0: a();\n/* no break */\n/* todo: fix readability */\ndefault: b() }",
            Some(serde_json::json!([{
                "commentPattern": "no break"
            }])),
        ),
        (
            "switch(foo) { case 0: { a();\n/* no break */\n/* todo: fix readability */ }\ndefault: b() }",
            Some(serde_json::json!([{
                "commentPattern": "no break"
            }])),
        ),
        ("switch(foo) { case 0: \n /* with comments */  \ncase 1: b(); }", None),
        (
            "switch(foo) { case 0:\n\ncase 1: b(); }",
            Some(serde_json::json!([{
                "allowEmptyCase": false
            }])),
        ),
        ("switch(foo) { case 0:\n\ncase 1: b(); }", Some(serde_json::json!([{}]))),
        (
            "switch (a) { case 1: \n ; case 2:  }",
            Some(serde_json::json!([{ "allowEmptyCase": false }])),
        ),
        (
            "switch (a) { case 1: ; case 2: ; case 3: }",
            Some(serde_json::json!([{ "allowEmptyCase": true }])),
        ),
        (
            "switch (foo) { case 0: a(); \n// eslint-enable no-fallthrough\n case 1: }",
            Some(serde_json::json!([{}])),
        ),
    ];

    Tester::new(NoFallthrough::NAME, pass, fail).test_and_snapshot();
}
