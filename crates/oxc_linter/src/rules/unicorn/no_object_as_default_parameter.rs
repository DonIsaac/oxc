use oxc_ast::{
    AstKind,
    ast::{BindingPatternKind, Expression},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{AstNode, context::LintContext, rule::Rule};

fn identifier(span: Span, param: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("Do not use an object literal as default for parameter `{param}`."))
        .with_label(span)
}

fn non_identifier(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("Do not use an object literal as default").with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct NoObjectAsDefaultParameter;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Disallow the use of an object literal as a default value for a parameter.
    ///
    /// ### Why is this bad?
    ///
    /// Default parameters should not be passed to a function through an object literal. The `foo = {a: false}` parameter works fine if only used with one option. As soon as additional options are added, you risk replacing the whole `foo = {a: false, b: true}` object when passing only one option: `{a: true}`. For this reason, object destructuring should be used instead.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```javascript
    /// function foo(foo = {a: false}) {}
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```javascript
    /// function foo({a = false} = {}) {}
    /// ```
    NoObjectAsDefaultParameter,
    unicorn,
    pedantic
);

impl Rule for NoObjectAsDefaultParameter {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let AstKind::AssignmentPattern(assignment_pat) = node.kind() else {
            return;
        };

        let Expression::ObjectExpression(object_expr) = &assignment_pat.right.without_parentheses()
        else {
            return;
        };

        if object_expr.properties.is_empty() {
            return;
        }

        if !matches!(ctx.nodes().parent_kind(node.id()), AstKind::FormalParameter(_)) {
            return;
        }

        if let BindingPatternKind::BindingIdentifier(binding_id) = &assignment_pat.left.kind {
            ctx.diagnostic(identifier(object_expr.span, &binding_id.name));
            return;
        }

        ctx.diagnostic(non_identifier(object_expr.span));
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"const abc = {};",
        r"const abc = {foo: 123};",
        r"function abc(foo) {}",
        r"function abc(foo = null) {}",
        r"function abc(foo = undefined) {}",
        r"function abc(foo = 123) {}",
        r"function abc(foo = true) {}",
        r#"function abc(foo = "bar") {}"#,
        r#"function abc(foo = 123, bar = "foo") {}"#,
        r"function abc(foo = {}) {}",
        r"function abc({foo = 123} = {}) {}",
        r"(function abc() {})(foo = {a: 123})",
        r"const abc = foo => {};",
        r"const abc = (foo = null) => {};",
        r"const abc = (foo = undefined) => {};",
        r"const abc = (foo = 123) => {};",
        r"const abc = (foo = true) => {};",
        r#"const abc = (foo = "bar") => {};"#,
        r#"const abc = (foo = ("bar")) => {};"#,
        r#"const abc = (foo = 123, bar = "foo") => {};"#,
        r"const abc = (foo = {}) => {};",
        r#"const abc = ({a = true, b = "foo"}) => {};"#,
        r"const abc = function(foo = 123) {}",
        r"const {abc = {foo: 123}} = bar;",
        r#"const {abc = {null: "baz"}} = bar;"#,
        r"const {abc = {foo: undefined}} = undefined;",
        r"const abc = ([{foo = false, bar = 123}]) => {};",
        r"const abc = ({foo = {a: 123}}) => {};",
        r"const abc = ({foo = ({a: 123})}) => {};",
        r"const abc = ([foo = {a: 123}]) => {};",
        r"const abc = ({foo: bar = {a: 123}}) => {};",
        r"const abc = () => (foo = {a: 123});",
    ];

    let fail = vec![
        r"function abc(foo = {a: 123}) {}",
        r"async function * abc(foo = {a: 123}) {}",
        r"function abc(foo = {a: false}) {}",
        r"function abc(foo = ({a: false})) {}",
        r#"function abc(foo = {a: "bar"}) {}"#,
        r#"function abc(foo = {a: "bar", b: {c: true}}) {}"#,
        r"const abc = (foo = {a: false}) => {};",
        r"const abc = (foo = {a: 123, b: false}) => {};",
        r#"const abc = (foo = {a: false, b: 1, c: "test", d: null}) => {};"#,
        r"const abc = function(foo = {a: 123}) {}",
        r"function abc(foo = {a: 123}) {}",
        r"const abc = (foo = {a: false}) => {};",
        r"function abc({a} = {a: 123}) {}",
        r"function abc([a] = {a: 123}) {}",
    ];

    Tester::new(NoObjectAsDefaultParameter::NAME, NoObjectAsDefaultParameter::PLUGIN, pass, fail)
        .test_and_snapshot();
}
