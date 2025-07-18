use cow_utils::CowUtils;

use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_minifier::CompressOptions;
use oxc_minifier::Compressor;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::default_options;

#[track_caller]
fn run(source_text: &str, source_type: SourceType, options: Option<CompressOptions>) -> String {
    let allocator = Allocator::default();
    let mut ret = Parser::new(&allocator, source_text, source_type).parse();
    let program = &mut ret.program;
    if let Some(options) = options {
        Compressor::new(&allocator, options).dead_code_elimination(program);
    }
    Codegen::new().build(program).code
}

#[track_caller]
fn test(source_text: &str, expected: &str) {
    let t = "('production' == 'production')";
    let f = "('production' == 'development')";
    let source_text = source_text.cow_replace("true", t);
    let source_text = source_text.cow_replace("false", f);

    let source_type = SourceType::default();
    let result = run(&source_text, source_type, Some(default_options()));
    let expected = run(expected, source_type, None);
    assert_eq!(result, expected, "\nfor source\n{source_text}\nexpect\n{expected}\ngot\n{result}");
}

#[track_caller]
fn test_same(source_text: &str) {
    test(source_text, source_text);
}

#[test]
fn dce_if_statement() {
    test("if (true) { foo }", "foo");
    test("if (true) { foo } else { bar }", "foo");
    test("if (false) { foo } else { bar }", "bar");

    test("if (xxx) { foo } else if (false) { bar }", "if (xxx) foo");
    test("if (xxx) { foo } else if (false) { bar } else { baz }", "if (xxx) foo; else baz");
    test("if (xxx) { foo } else if (false) { bar } else if (false) { baz }", "if (xxx) foo");
    test(
        "if (xxx) { foo } else if (false) { bar } else if (false) { baz } else { quaz }",
        "if (xxx) foo; else quaz",
    );
    test(
        "if (xxx) { foo } else if (true) { bar } else if (false) { baz }",
        "if (xxx) foo; else bar",
    );
    test(
        "if (xxx) { foo } else if (false) { bar } else if (true) { baz }",
        "if (xxx) foo; else baz",
    );
    test(
        "if (xxx) { foo } else if (true) { bar } else if (true) { baz }",
        "if (xxx) foo; else bar",
    );
    test(
        "if (xxx) { foo } else if (false) { var a; var b; } else if (false) { var c; var d; }",
        "if (xxx) foo; else if (0) var a, b; else if (0) var c, d;",
    );

    test("if (!false) { foo }", "foo");
    test("if (!true) { foo } else { bar }", "bar");

    test("if (!false && xxx) { foo }", "if (xxx) foo");
    test("if (!true && yyy) { foo } else { bar }", "bar");
    test("if (xxx && false) { foo } else { bar }", "if (xxx && false); else bar");

    test("if (true || xxx) { foo }", "foo");
    test("if (false || xxx) { foo }", "if (xxx) foo");
    test("if (xxx || true) { foo } else { bar }", "if (xxx || true) foo");

    test("if ('production' == 'production') { foo } else { bar }", "foo");
    test("if ('development' == 'production') { foo } else { bar }", "bar");

    test("if ('production' === 'production') { foo } else { bar }", "foo");
    test("if ('development' === 'production') { foo } else { bar }", "bar");

    // Shadowed `undefined` as a variable should not be erased.
    // This is a rollup test.
    // <https://github.com/rollup/rollup/blob/master/test/function/samples/allow-undefined-as-parameter/main.js>
    test_same("function foo(undefined) { if (!undefined) throw Error('') }");

    test("function foo() { if (undefined) { bar } }", "function foo() { }");
    test("function foo() { { bar } }", "function foo() { bar }");

    test("if (true) { foo; } if (true) { foo; }", "foo; foo;");
    test("if (true) { foo; return } foo; if (true) { bar; return } bar;", "{ foo; return }");

    // nested expression
    test(
        "const a = { fn: function() { if (true) { foo; } } }",
        "const a = { fn: function() { foo; } }",
    );

    // parenthesized
    test("if (!!(false)) { REMOVE; } else { KEEP; }", "KEEP");

    // typeof
    test("if (typeof 1 !== 'number') { REMOVE; }", "");
    test("if (typeof false !== 'boolean') { REMOVE; }", "");
    test("if (typeof 1 === 'string') { REMOVE; }", "");
}

#[test]
fn dce_while_statement() {
    test_same("while (true);");
    test_same("while (false);");
}

#[test]
fn dce_conditional_expression() {
    test("false ? foo : bar;", "bar");
    test("true ? foo : bar;", "foo");

    test("!true ? foo : bar;", "bar");
    test("!false ? foo : bar;", "foo");

    test("!!false ? foo : bar;", "bar");
    test("!!true ? foo : bar;", "foo");

    test("const foo = true ? A : B", "const foo = A");
    test("const foo = false ? A : B", "const foo = B");
}

#[test]
fn dce_logical_expression() {
    test("false && bar()", "");
    test("true && bar()", "bar()");

    test("const foo = false && bar()", "const foo = false");
    test("const foo = true && bar()", "const foo = bar()");
}

#[test]
fn dce_var_hoisting() {
    test(
        "function f() {
          return () => {
            var x;
          }
          REMOVE;
          function KEEP() {}
          REMOVE;
        }",
        "function f() {
          return () => {
            var x;
          }
          function KEEP() {}
        }",
    );
    test(
        "function f() {
          return function g() {
            var x;
          }
          REMOVE;
          function KEEP() {}
          REMOVE;
        }",
        "function f() {
          return function g() {
            var x;
          }
          function KEEP() {}
        }",
    );
}

#[test]
fn pure_comment_for_pure_global_constructors() {
    test("var x = new WeakSet", "var x = /* @__PURE__ */ new WeakSet();\n");
    test("var x = new WeakSet(null)", "var x = /* @__PURE__ */ new WeakSet(null);\n");
    test("var x = new WeakSet(undefined)", "var x = /* @__PURE__ */ new WeakSet(void 0);\n");
    test("var x = new WeakSet([])", "var x = /* @__PURE__ */ new WeakSet([]);\n");
}

#[test]
fn fold_number_nan() {
    test("foo(Number.NaN)", "foo(NaN)");
    test_same("let Number; foo(Number.NaN)");
}

// https://github.com/terser/terser/blob/v5.9.0/test/compress/dead-code.js
#[test]
fn dce_from_terser() {
    test(
        "function f() {
            a();
            b();
            x = 10;
            return;
            if (x) {
                y();
            }
        }",
        "function f() {
            a();
            b();
            x = 10;
            return;
        }",
    );

    test(
        r#"function f() {
            g();
            x = 10;
            throw new Error("foo");
            if (true) {
                y();
                var x;
                function g(){};
                (function(){
                    var q;
                    function y(){};
                })();
            }
        }
        f();
        "#,
        r#"function f() {
            g();
            x = 10;
            throw new Error("foo");
            var x;
        }
        f();
        "#,
    );

    test(
        "if (0) {
            let foo = 6;
            const bar = 12;
            class Baz {};
            var qux;
        }
        console.log(foo, bar, Baz);
        ",
        "
        if (0) var qux;
        console.log(foo, bar, Baz);
        ",
    );
}
