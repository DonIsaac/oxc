use std::borrow::Cow;

use oxc_ast::{
    AstKind,
    ast::{
        AssignmentTarget, AssignmentTargetProperty, BindingPatternKind, Expression, Function,
        FunctionType, PropertyKind,
    },
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_semantic::NodeId;
use oxc_span::{Atom, GetSpan, Span};
use oxc_syntax::identifier::is_identifier_name;

use crate::{AstNode, ast_util::get_function_name_with_kind, context::LintContext, rule::Rule};

fn named_diagnostic(function_name: &str, span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("Unexpected named {function_name}."))
        .with_label(span)
        .with_help("Remove the name on this function expression.")
}

fn unnamed_diagnostic(inferred_name_or_description: &str, span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("Unexpected unnamed {inferred_name_or_description}."))
        .with_label(span)
        .with_help("Consider giving this function expression a name.")
}

#[derive(Debug, Default, Clone)]
pub struct FuncNames {
    default_config: FuncNamesConfig,
    generators_config: FuncNamesConfig,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum FuncNamesConfig {
    #[default]
    Always,
    AsNeeded,
    Never,
}

impl FuncNamesConfig {
    fn is_invalid_function(self, func: &Function, parent_node: &AstNode<'_>) -> bool {
        let func_name = func.name();

        match self {
            Self::Never => func_name.is_some() && func.r#type != FunctionType::FunctionDeclaration,
            Self::AsNeeded => func_name.is_none() && !has_inferred_name(func, parent_node),
            Self::Always => func_name.is_none() && !is_object_or_class_method(parent_node),
        }
    }
}

impl TryFrom<&serde_json::Value> for FuncNamesConfig {
    type Error = OxcDiagnostic;

    fn try_from(raw: &serde_json::Value) -> Result<Self, Self::Error> {
        raw.as_str().map_or_else(
            || Err(OxcDiagnostic::warn("Expecting string for eslint/func-names configuration")),
            |v| match v {
                "always" => Ok(FuncNamesConfig::Always),
                "as-needed" => Ok(FuncNamesConfig::AsNeeded),
                "never" => Ok(FuncNamesConfig::Never),
                _ => Err(OxcDiagnostic::warn(
                    "Expecting always, as-needed or never for eslint/func-names configuration",
                )),
            },
        )
    }
}

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Require or disallow named function expressions.
    ///
    /// ### Why is this bad?
    ///
    /// Leaving the name off a function will cause `<anonymous>` to appear in
    /// stack traces of errors thrown in it or any function called within it.
    /// This makes it more difficult to find where an error is thrown.  If you
    /// provide the optional name for a function expression then you will get
    /// the name of the function expression in the stack trace.
    ///
    /// ## Configuration
    /// This rule has a string option:
    /// - `"always"` requires a function expression to have a name under all
    ///   circumstances.
    /// - `"as-needed"` requires a function expression to have a name only when
    ///    one will not be automatically inferred by the runtime.
    /// - `"never"` requires a function expression to not have a name under any
    ///    circumstances.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    ///
    /// ```javascript
    /// /*oxlint func-names: "error" */
    ///
    /// // default is "always" and there is an anonymous function
    /// Foo.prototype.bar = function() {};
    ///
    /// /*oxlint func-names: ["error", "always"] */
    ///
    /// // there is an anonymous function
    /// Foo.prototype.bar = function() {};
    ///
    /// /*oxlint func-names: ["error", "as-needed"] */
    ///
    /// // there is an anonymous function
    /// // where the name isn’t assigned automatically per the ECMAScript specs
    /// Foo.prototype.bar = function() {};
    ///
    /// /*oxlint func-names: ["error", "never"] */
    ///
    /// // there is a named function
    /// Foo.prototype.bar = function bar() {};
    /// ```
    ///
    /// Examples of **correct* code for this rule:
    ///
    /// ```javascript
    /// /*oxlint func-names: "error" */
    ///
    /// Foo.prototype.bar = function bar() {};
    ///
    /// /*oxlint func-names: ["error", "always"] */
    ///
    /// Foo.prototype.bar = function bar() {};
    ///
    /// /*oxlint func-names: ["error", "as-needed"] */
    ///
    /// var foo = function(){};
    ///
    /// /*oxlint func-names: ["error", "never"] */
    ///
    /// Foo.prototype.bar = function() {};
    /// ```
    FuncNames,
    eslint,
    style,
    conditional_fix_suggestion
);

/// Determines whether the current FunctionExpression node is a get, set, or
/// shorthand method in an object literal or a class.
fn is_object_or_class_method(parent_node: &AstNode) -> bool {
    match parent_node.kind() {
        AstKind::MethodDefinition(_) => true,
        AstKind::ObjectProperty(property) => {
            property.method || matches!(property.kind, PropertyKind::Get | PropertyKind::Set)
        }
        _ => false,
    }
}

/// Determines whether the current FunctionExpression node has a name that would be
/// inferred from context in a conforming ES6 environment.
fn has_inferred_name<'a>(function: &Function<'a>, parent_node: &AstNode<'a>) -> bool {
    if is_object_or_class_method(parent_node) {
        return true;
    }

    match parent_node.kind() {
        AstKind::VariableDeclarator(declarator) => {
            matches!(declarator.id.kind, BindingPatternKind::BindingIdentifier(_))
                && declarator.init.as_ref().is_some_and(|init| is_same_function(init, function))
        }
        AstKind::ObjectProperty(property) => is_same_function(&property.value, function),
        AstKind::PropertyDefinition(definition) => {
            definition.value.as_ref().is_some_and(|value| is_same_function(value, function))
        }
        AstKind::AssignmentExpression(expression) => {
            matches!(expression.left, AssignmentTarget::AssignmentTargetIdentifier(_))
                && is_same_function(&expression.right, function)
        }
        AstKind::AssignmentTargetWithDefault(target) => {
            matches!(target.binding, AssignmentTarget::AssignmentTargetIdentifier(_))
                && is_same_function(&target.init, function)
        }
        AstKind::AssignmentPattern(pattern) => {
            matches!(pattern.left.kind, BindingPatternKind::BindingIdentifier(_))
                && is_same_function(&pattern.right, function)
        }
        AstKind::AssignmentTargetPropertyIdentifier(ident) => {
            ident.init.as_ref().is_some_and(|expr| is_same_function(expr, function))
        }
        AstKind::ObjectAssignmentTarget(target) => {
            for property in &target.properties {
                let AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(identifier) =
                    property
                else {
                    continue;
                };
                let Expression::FunctionExpression(function_expression) =
                    &identifier.init.as_ref().unwrap()
                else {
                    continue;
                };
                if get_function_identifier(function_expression) == get_function_identifier(function)
                {
                    return true;
                }
            }

            false
        }
        _ => false,
    }
}

fn is_same_function<'a>(fn1: &Expression<'a>, fn2: &Function<'a>) -> bool {
    matches!(fn1, Expression::FunctionExpression(function_expression)
        if get_function_identifier(function_expression) == get_function_identifier(fn2)
    )
}

/**
 * Gets the identifier for the function
 */
fn get_function_identifier<'a>(func: &'a Function<'a>) -> Option<&'a Span> {
    func.id.as_ref().map(|id| &id.span)
}

impl Rule for FuncNames {
    fn from_configuration(value: serde_json::Value) -> Self {
        let Some(default_value) = value.get(0) else {
            return Self::default();
        };

        let default_config = FuncNamesConfig::try_from(default_value).unwrap();

        let generators_value =
            value.get(1).and_then(|v| v.get("generators")).unwrap_or(default_value);

        let generators_config = FuncNamesConfig::try_from(generators_value).unwrap();

        Self { default_config, generators_config }
    }

    fn run_once(&self, ctx: &LintContext<'_>) {
        let mut invalid_funcs: Vec<(&Function, &AstNode, &AstNode)> = vec![];

        for node in ctx.nodes() {
            match node.kind() {
                // check function if it invalid, do not report it because maybe later the function is calling itself
                AstKind::Function(func) => {
                    let parent_node = ctx.nodes().parent_node(node.id());
                    let config =
                        if func.generator { &self.generators_config } else { &self.default_config };

                    if config.is_invalid_function(func, parent_node) {
                        invalid_funcs.push((func, node, parent_node));
                    }
                }

                // check if the calling function is inside of its own body
                // when yes remove it from invalid_funcs because recursion are always named
                AstKind::CallExpression(expression) => {
                    if let Expression::Identifier(identifier) = &expression.callee {
                        // check at first if the callee calls an invalid function
                        if !invalid_funcs
                            .iter()
                            .filter_map(|(func, _, _)| func.name())
                            .any(|func_name| func_name == identifier.name)
                        {
                            continue;
                        }

                        // a function which is calling itself inside is always valid
                        let ast_span =
                            ctx.nodes().ancestors(node.id()).find_map(|p| match p.kind() {
                                AstKind::Function(func) => {
                                    let func_name = func.name()?;

                                    if func_name == identifier.name {
                                        return Some(func.span);
                                    }

                                    None
                                }
                                _ => None,
                            });

                        // we found a recursive function, remove it from the invalid list
                        if let Some(span) = ast_span {
                            invalid_funcs.retain(|(func, _, _)| func.span != span);
                        }
                    }
                }
                _ => {}
            }
        }

        for (func, node, parent_node) in invalid_funcs {
            let func_name_complete = get_function_name_with_kind(node, parent_node);

            let report_span = Span::new(func.span.start, func.params.span.start);
            let replace_span = Span::new(
                func.span.start,
                func.type_parameters
                    .as_ref()
                    .map_or_else(|| func.params.span.start, |tp| tp.span.start),
            );
            if let Some(id) = func.id.as_ref() {
                ctx.diagnostic_with_suggestion(
                    named_diagnostic(&func_name_complete, report_span),
                    |fixer| fixer.delete(id),
                );
            } else {
                ctx.diagnostic_with_fix(
                    unnamed_diagnostic(&func_name_complete, report_span),
                    |fixer| {
                        guess_function_name(ctx, node.id()).map_or_else(
                            || fixer.noop(),
                            |name| {
                                // if this name shadows a variable in the outer scope **and** that name is referenced
                                // inside the function body, it is unsafe to add a name to this function
                                if ctx.scoping().find_binding(func.scope_id(), &name).is_some_and(
                                    |shadowed_var| {
                                        ctx.semantic().symbol_references(shadowed_var).any(
                                            |reference| {
                                                func.span.contains_inclusive(
                                                    ctx.nodes()
                                                        .get_node(reference.node_id())
                                                        .kind()
                                                        .span(),
                                                )
                                            },
                                        )
                                    },
                                ) {
                                    return fixer.noop();
                                }

                                fixer.insert_text_after(&replace_span, format!(" {name}"))
                            },
                        )
                    },
                );
            }
        }
    }
}

fn guess_function_name<'a>(ctx: &LintContext<'a>, node_id: NodeId) -> Option<Cow<'a, str>> {
    for parent_kind in ctx.nodes().ancestor_kinds(node_id) {
        match parent_kind {
            AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSSatisfiesExpression(_) => {}
            AstKind::AssignmentExpression(assign) => {
                return assign.left.get_identifier_name().map(Cow::Borrowed);
            }
            AstKind::VariableDeclarator(decl) => {
                return decl.id.get_identifier_name().as_ref().map(Atom::as_str).map(Cow::Borrowed);
            }
            AstKind::ObjectProperty(prop) => {
                return prop.key.static_name().filter(|name| is_valid_identifier_name(name));
            }
            AstKind::PropertyDefinition(prop) => {
                return prop.key.static_name().filter(|name| is_valid_identifier_name(name));
            }

            _ => return None,
        }
    }
    None
}

const INVALID_NAMES: [&str; 9] =
    ["arguments", "async", "await", "constructor", "default", "eval", "null", "undefined", "yield"];

fn is_valid_identifier_name(name: &str) -> bool {
    !INVALID_NAMES.contains(&name) && is_identifier_name(name)
}

#[test]
fn test() {
    use serde_json::json;

    use crate::tester::Tester;

    let always = Some(json!(["always"]));
    let as_needed = Some(json!(["as-needed"]));
    let never = Some(json!(["never"]));

    let pass = vec![
        ("Foo.prototype.bar = function bar(){};", None),
        ("Foo.prototype.bar = () => {}", None), // { "ecmaVersion": 6 },
        ("function foo(){}", None),
        ("function test(d, e, f) {}", None),
        ("new function bar(){}", None),
        ("exports = { get foo() { return 1; }, set bar(val) { return val; } };", None),
        ("({ foo() { return 1; } });", None), // { "ecmaVersion": 6 },
        ("class A { constructor(){} foo(){} get bar(){} set baz(value){} static qux(){}}", None), // { "ecmaVersion": 6 },
        ("function foo() {}", always.clone()),
        ("var a = function foo() {};", always.clone()),
        (
            "class A { constructor(){} foo(){} get bar(){} set baz(value){} static qux(){}}",
            as_needed.clone(),
        ), // { "ecmaVersion": 6 },
        ("({ foo() {} });", as_needed.clone()), // { "ecmaVersion": 6 },
        ("var foo = function(){};", as_needed.clone()),
        ("({foo: function(){}});", as_needed.clone()),
        ("(foo = function(){});", as_needed.clone()),
        ("({foo = function(){}} = {});", as_needed.clone()), // { "ecmaVersion": 6 },
        ("({key: foo = function(){}} = {});", as_needed.clone()), // { "ecmaVersion": 6 },
        ("[foo = function(){}] = [];", as_needed.clone()),   // { "ecmaVersion": 6 },
        ("function fn(foo = function(){}) {}", as_needed.clone()), // { "ecmaVersion": 6 },
        ("function foo() {}", never.clone()),
        ("var a = function() {};", never.clone()),
        ("var a = function foo() { foo(); };", never.clone()),
        ("var foo = {bar: function() {}};", never.clone()),
        ("$('#foo').click(function() {});", never.clone()),
        ("Foo.prototype.bar = function() {};", never.clone()),
        (
            "class A { constructor(){} foo(){} get bar(){} set baz(value){} static qux(){}}",
            never.clone(),
        ), // { "ecmaVersion": 6 },
        ("({ foo() {} });", never.clone()), // { "ecmaVersion": 6 },
        ("export default function foo() {}", always.clone()), // { "sourceType": "module", "ecmaVersion": 6 },
        ("export default function foo() {}", as_needed.clone()), // { "sourceType": "module", "ecmaVersion": 6 },
        ("export default function foo() {}", never.clone()), // { "sourceType": "module", "ecmaVersion": 6 },
        ("export default function() {}", never.clone()), // { "sourceType": "module", "ecmaVersion": 6 },
        ("var foo = bar(function *baz() {});", always.clone()), // { "ecmaVersion": 6 },
        ("var foo = bar(function *baz() {});", Some(json!(["always", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        (
            "var foo = bar(function *baz() {});",
            Some(json!(["always", { "generators": "as-needed" }])),
        ), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["always", { "generators": "as-needed" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *baz() {});", as_needed.clone()), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", as_needed.clone()),          // { "ecmaVersion": 6 },
        (
            "var foo = bar(function *baz() {});",
            Some(json!(["as-needed", { "generators": "always" }])),
        ), // { "ecmaVersion": 6 },
        (
            "var foo = bar(function *baz() {});",
            Some(json!(["as-needed", { "generators": "as-needed" }])),
        ), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["as-needed", { "generators": "as-needed" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *baz() {});", Some(json!(["never", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        (
            "var foo = bar(function *baz() {});",
            Some(json!(["never", { "generators": "as-needed" }])),
        ), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["never", { "generators": "as-needed" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", never.clone()), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", never.clone()),       // { "ecmaVersion": 6 },
        ("(function*() {}())", never.clone()),              // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", Some(json!(["never", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["never", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["never", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", Some(json!(["always", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["always", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["always", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", Some(json!(["as-needed", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["as-needed", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["as-needed", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("class C { foo = function() {}; }", as_needed.clone()), // { "ecmaVersion": 2022 },
        ("class C { [foo] = function() {}; }", as_needed.clone()), // { "ecmaVersion": 2022 },
        ("class C { #foo = function() {}; }", as_needed.clone()), // { "ecmaVersion": 2022 }
    ];

    let fail = vec![
        ("Foo.prototype.bar = function() {};", None),
        ("(function(){}())", None),
        ("f(function(){})", None),
        ("var a = new Date(function() {});", None),
        ("var test = function(d, e, f) {};", None),
        ("new function() {}", None),
        ("Foo.prototype.bar = function() {};", as_needed.clone()),
        ("(function(){}())", as_needed.clone()),
        ("f(function(){})", as_needed.clone()),
        ("var a = new Date(function() {});", as_needed.clone()),
        ("new function() {}", as_needed.clone()),
        ("var {foo} = function(){};", as_needed.clone()), // { "ecmaVersion": 6 },
        ("({ a: obj.prop = function(){} } = foo);", as_needed.clone()), // { "ecmaVersion": 6 },
        ("[obj.prop = function(){}] = foo;", as_needed.clone()), // { "ecmaVersion": 6 },
        ("var { a: [b] = function(){} } = foo;", as_needed.clone()), // { "ecmaVersion": 6 },
        ("function foo({ a } = function(){}) {};", as_needed.clone()), // { "ecmaVersion": 6 },
        ("var x = function foo() {};", never.clone()),
        ("Foo.prototype.bar = function foo() {};", never.clone()),
        ("({foo: function foo() {}})", never.clone()),
        ("export default function() {}", always.clone()), // { "sourceType": "module", "ecmaVersion": 6 },
        ("export default function() {}", as_needed.clone()), // { "sourceType": "module", "ecmaVersion": 6 },
        ("export default (function(){});", as_needed.clone()), // { "sourceType": "module", "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", always.clone()),   // { "ecmaVersion": 6 },
        ("var foo = function*() {};", always.clone()),         // { "ecmaVersion": 6 },
        ("(function*() {}())", always.clone()),                // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", Some(json!(["always", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["always", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["always", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", Some(json!(["always", { "generators": "as-needed" }]))), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["always", { "generators": "as-needed" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", as_needed.clone()), // { "ecmaVersion": 6 },
        ("(function*() {}())", as_needed.clone()),              // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", Some(json!(["as-needed", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["as-needed", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["as-needed", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        (
            "var foo = bar(function *() {});",
            Some(json!(["as-needed", { "generators": "as-needed" }])),
        ), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["as-needed", { "generators": "as-needed" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", Some(json!(["never", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        ("var foo = function*() {};", Some(json!(["never", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["never", { "generators": "always" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *() {});", Some(json!(["never", { "generators": "as-needed" }]))), // { "ecmaVersion": 6 },
        ("(function*() {}())", Some(json!(["never", { "generators": "as-needed" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *baz() {});", never.clone()), // { "ecmaVersion": 6 },
        ("var foo = bar(function *baz() {});", Some(json!(["never", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        ("var foo = bar(function *baz() {});", Some(json!(["always", { "generators": "never" }]))), // { "ecmaVersion": 6 },
        (
            "var foo = bar(function *baz() {});",
            Some(json!(["as-needed", { "generators": "never" }])),
        ), // { "ecmaVersion": 6 },
        ("class C { foo = function() {} }", always.clone()), // { "ecmaVersion": 2022 },
        ("class C { public foo = function() {} }", always.clone()), // { "ecmaVersion": 2022 },
        ("class C { [foo] = function() {} }", always.clone()), // { "ecmaVersion": 2022 },
        ("class C { #foo = function() {} }", always.clone()), // { "ecmaVersion": 2022 },
        ("class C { foo = bar(function() {}) }", as_needed), // { "ecmaVersion": 2022 },
        ("class C { foo = function bar() {} }", never.clone()), // { "ecmaVersion": 2022 }
    ];

    let fix = vec![
        // lb
        ("const foo = function() {}", "const foo = function foo() {}", always.clone()),
        (
            "Foo.prototype.bar = function() {}",
            "Foo.prototype.bar = function bar() {}",
            always.clone(),
        ),
        ("let foo; foo = function() {}", "let foo; foo = function foo() {}", always.clone()),
        (
            "class C { public foo = function() {} }",
            "class C { public foo = function foo() {} }",
            always.clone(),
        ),
        (
            "class C { public ['foo'] = function() {} }",
            "class C { public ['foo'] = function foo() {} }",
            always.clone(),
        ),
        (
            "class C { public [`foo`] = function() {} }",
            "class C { public [`foo`] = function foo() {} }",
            always.clone(),
        ),
        (
            "class C { public ['invalid identifier name'] = function() {} }",
            "class C { public ['invalid identifier name'] = function() {} }",
            always.clone(),
        ),
        (
            "class C { public [foo] = function() {} }",
            "class C { public [foo] = function() {} }",
            always.clone(),
        ),
        (
            "class C { public [undefined] = function() {} }",
            "class C { public [undefined] = function() {} }",
            always.clone(),
        ),
        (
            "class C { public [null] = function() {} }",
            "class C { public [null] = function() {} }",
            always.clone(),
        ),
        (
            "class C { public ['undefined'] = function() {} }",
            "class C { public ['undefined'] = function() {} }",
            always.clone(),
        ),
        (
            "class C { public ['null'] = function() {} }",
            "class C { public ['null'] = function() {} }",
            always.clone(),
        ),
        (
            "const x = { foo: function() {} }",
            "const x = { foo: function foo() {} }",
            always.clone(),
        ),
        (
            "const x = { ['foo']: function() {} }",
            "const x = { ['foo']: function foo() {} }",
            always.clone(),
        ),
        // suggest removal when configured to "never"
        ("const foo = function foo() {}", "const foo = function () {}", never.clone()),
        (
            "Foo.prototype.bar = function bar() {}",
            "Foo.prototype.bar = function () {}",
            never.clone(),
        ),
        ("class C { foo = function foo() {} }", "class C { foo = function () {} }", never),
        (
            "const restoreGracefully = function <T>(entries: T[]) { }",
            "const restoreGracefully = function  restoreGracefully<T>(entries: T[]) { }",
            None,
        ),
        ("const foo = async function() {}", "const foo = async function foo() {}", always.clone()),
        (
            "const foo = async function            () {}",
            "const foo = async function             foo() {}",
            always.clone(),
        ),
        (
            "const foo =      async          function      <T>      ()           {}",
            "const foo =      async          function       foo<T>      ()           {}",
            always.clone(),
        ),
        (
            "const foo =      async          function      <T           >      ()           {}",
            "const foo =      async          function       foo<T           >      ()           {}",
            always.clone(),
        ),
        ("const foo = function* () {}", "const foo = function*  foo() {}", always.clone()),
        (
            "const foo = async function* (){}",
            "const foo = async function*  foo(){}",
            always.clone(),
        ),
        (
            "const foo = async function* <T extends foo>(){}",
            "const foo = async function*  foo<T extends foo>(){}",
            always.clone(),
        ),
        // we can't fix this case because adding a name would cause the
        (
            "const setState = Component.prototype.setState;
             Component.prototype.setState = function (update, callback) {
	             return setState.call(this, update, callback);
            };",
            "const setState = Component.prototype.setState;
             Component.prototype.setState = function (update, callback) {
	             return setState.call(this, update, callback);
            };",
            always,
        ),
    ];

    Tester::new(FuncNames::NAME, FuncNames::PLUGIN, pass, fail).expect_fix(fix).test_and_snapshot();
}
