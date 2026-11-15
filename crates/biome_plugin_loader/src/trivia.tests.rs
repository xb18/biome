use super::tests::{load_test_plugin_from_source, render_diagnostics, services, snap_diagnostics};
use super::*;
use biome_diagnostics::PrintDescription;
use biome_js_parser::{JsParserOptions, parse};

fn evaluate(plugin: &AnalyzerJsPlugin, source: &str) -> PluginEvalResult {
    let parse = parse(source, JsFileSource::jsx(), JsParserOptions::default());
    plugin.evaluate(
        parse.syntax().into(),
        "/file.jsx".into(),
        &services(JsFileSource::jsx()),
    )
}

fn messages(result: &PluginEvalResult) -> Vec<String> {
    result
        .entries
        .iter()
        .map(|entry| PrintDescription(&entry.diagnostic).to_string())
        .collect()
}

#[test]
fn trivia_reports_comments_and_tokens() {
    let plugin_source = r#"import { ast, defineRule, registerDiagnostic } from "@biomejs/runtime/plugin";
        export const noTodo = defineRule({
            query: ast("JS_MODULE"),
            run(root) {
                for (const comment of root.comments()) {
                    if (comment.text.includes("TODO")) {
                        registerDiagnostic(comment, "warning", `Resolve this ${comment.kind}.`);
                    }
                }
            },
        });
        export const noVarKeyword = defineRule({
            query: ast("JS_VARIABLE_DECLARATION"),
            run(declaration) {
                const keyword = declaration.token("kindToken");
                if (keyword?.text === "var") {
                    registerDiagnostic(keyword, "information", "Use let or const instead.");
                }
            },
        });"#;
    let source = r#"// TODO: remove this file
var legacy = 1; /* TODO inline */
/*
 * TODO: multiple lines
 */
let modern = 2; // done
"#;
    let plugin = load_test_plugin_from_source("/plugin.js", plugin_source, None);
    let parse = parse(
        source,
        JsFileSource::js_module(),
        JsParserOptions::default(),
    );
    let rendered: String = parse
        .syntax()
        .descendants()
        .filter(|node| plugin.query().contains(&node.kind().to_raw()))
        .map(|node| {
            render_diagnostics(
                "/file.js",
                source,
                plugin.evaluate(
                    node.into(),
                    "/file.js".into(),
                    &services(JsFileSource::js_module()),
                ),
            )
        })
        .collect();

    snap_diagnostics(
        "trivia_reports_comments_and_tokens",
        "/plugin.js",
        plugin_source,
        &[("/file.js", source)],
        rendered,
    );
}

/// Leading trivia runs up to the token, and trailing trivia runs from the
/// token up to the next line break, so each comment belongs to exactly one
/// token.
#[test]
fn trivia_pieces_belong_to_their_tokens() {
    let plugin = load_test_plugin_from_source(
        "/plugin.js",
        r#"import { ast, defineRule, registerDiagnostic } from "@biomejs/runtime/plugin";
        const describe = (pieces) => pieces
            .map((piece) => `${piece.kind}:${JSON.stringify(piece.text)}@${piece.token.kind}`)
            .join(" ");
        export const trivia = defineRule({
            query: ast("JS_MODULE"),
            run(root) {
                registerDiagnostic(root, "information", `module leading: ${describe(root.leadingTrivia())}`);
                registerDiagnostic(root, "information", `module trailing: ${describe(root.trailingTrivia())}`);
                registerDiagnostic(root, "information", `module comments: ${describe(root.comments())}`);
                for (const statement of root.items) {
                    registerDiagnostic(statement, "information", `${statement.kind} leading: ${describe(statement.leadingTrivia())}`);
                    registerDiagnostic(statement, "information", `${statement.kind} trailing: ${describe(statement.trailingTrivia())}`);
                    registerDiagnostic(statement, "information", `${statement.kind} comments: ${describe(statement.comments())}`);
                }
                const declaration = root.items[0].declaration;
                const keyword = declaration.token("kindToken");
                registerDiagnostic(root, "information", `let leading: ${describe(keyword.leadingTrivia())}`);
                registerDiagnostic(root, "information", `let trailing: ${describe(keyword.trailingTrivia())}`);
            },
        });"#,
        None,
    );

    let result = evaluate(
        &plugin,
        "// leading\nlet a = 1; /* trailing */ // line\n/* multi\n   line */\ncall(); // end\n// eof\n",
    );
    assert_eq!(
        messages(&result),
        [
            r#"module leading: singleLineComment:"// leading"@LET_KW newline:"\n"@LET_KW"#,
            "module trailing: ",
            r#"module comments: singleLineComment:"// leading"@LET_KW singleLineComment:"/* trailing */"@SEMICOLON singleLineComment:"// line"@SEMICOLON multiLineComment:"/* multi\n   line */"@IDENT singleLineComment:"// end"@SEMICOLON singleLineComment:"// eof"@EOF"#,
            r#"JS_VARIABLE_STATEMENT leading: singleLineComment:"// leading"@LET_KW newline:"\n"@LET_KW"#,
            r#"JS_VARIABLE_STATEMENT trailing: whitespace:" "@SEMICOLON singleLineComment:"/* trailing */"@SEMICOLON whitespace:" "@SEMICOLON singleLineComment:"// line"@SEMICOLON"#,
            r#"JS_VARIABLE_STATEMENT comments: singleLineComment:"// leading"@LET_KW singleLineComment:"/* trailing */"@SEMICOLON singleLineComment:"// line"@SEMICOLON"#,
            r#"JS_EXPRESSION_STATEMENT leading: newline:"\n"@IDENT multiLineComment:"/* multi\n   line */"@IDENT newline:"\n"@IDENT"#,
            r#"JS_EXPRESSION_STATEMENT trailing: whitespace:" "@SEMICOLON singleLineComment:"// end"@SEMICOLON"#,
            r#"JS_EXPRESSION_STATEMENT comments: multiLineComment:"/* multi\n   line */"@IDENT singleLineComment:"// end"@SEMICOLON"#,
            r#"let leading: singleLineComment:"// leading"@LET_KW newline:"\n"@LET_KW"#,
            r#"let trailing: whitespace:" "@LET_KW"#,
        ]
    );
}

#[test]
fn trivia_includes_text_skipped_by_the_parser() {
    let plugin = load_test_plugin_from_source(
        "/plugin.js",
        r#"import { ast, defineRule, registerDiagnostic } from "@biomejs/runtime/plugin";
        export const skipped = defineRule({
            query: ast("JSX_EXPRESSION_ATTRIBUTE_VALUE"),
            run(value) {
                const curly = value.token("rCurlyToken");
                for (const piece of curly.leadingTrivia()) {
                    registerDiagnostic(piece, "information", `${piece.kind}:${piece.text}`);
                }
                if (value.comments().length !== 0) throw new Error("Skipped text is not a comment");
            },
        });"#,
        None,
    );
    let parse = parse(
        "<div className={asdf asdf} />;",
        JsFileSource::jsx(),
        JsParserOptions::default(),
    );
    assert!(parse.has_errors());
    let value = parse
        .syntax()
        .descendants()
        .find(|node| plugin.query().contains(&node.kind().to_raw()))
        .unwrap();
    let result = plugin.evaluate(
        value.into(),
        "/file.jsx".into(),
        &services(JsFileSource::jsx()),
    );
    let [entry] = result.entries.as_slice() else {
        panic!("expected a single diagnostic, got {result:?}");
    };
    assert_eq!(
        PrintDescription(&entry.diagnostic).to_string(),
        "skipped:asdf"
    );
    assert_eq!(
        entry.diagnostic.span(),
        Some(TextRange::new(21.into(), 25.into()))
    );
}

#[test]
fn diagnostics_highlight_nodes_tokens_and_trivia() {
    let plugin = load_test_plugin_from_source(
        "/plugin.js",
        r#"import { ast, createMutation, defineRule, factory, registerDiagnostic } from "@biomejs/runtime/plugin";
        export const targets = defineRule({
            query: ast("JS_VARIABLE_STATEMENT"),
            run(statement) {
                const keyword = statement.declaration.token("kindToken");
                const [comment] = statement.comments();
                registerDiagnostic(statement, "information", "node");
                registerDiagnostic(keyword, "information", "token");
                registerDiagnostic(comment, "information", "comment");
                registerDiagnostic(keyword.trailingTrivia()[0], "information", "whitespace");

                const mutation = createMutation(statement);
                mutation.replaceToken(keyword, factory.token("CONST_KW"));
                registerDiagnostic(comment, "information", "comment with fix", {
                    mutation,
                    message: "Use const.",
                    kind: "safe",
                });

                const updated = statement.withSemicolonToken(undefined);
                const updatedKeyword = updated.declaration.token("kindToken");
                const fix = () => ({ mutation: createMutation(statement), message: "Never applied.", kind: "safe" });
                for (const [detached, withFix] of [
                    [factory.token("LET_KW"), false],
                    [factory.token("LET_KW"), true],
                    [updatedKeyword, false],
                    [updatedKeyword.trailingTrivia()[0], false],
                    [updated, true],
                ]) {
                    try {
                        registerDiagnostic(detached, "information", "detached", withFix ? fix() : undefined);
                    } catch (error) {
                        if (!(error instanceof TypeError) ||
                            !error.message.includes("does not belong to the source being analyzed")) {
                            throw error;
                        }
                        continue;
                    }
                    throw new Error("Expected a detached target to be rejected");
                }
            },
        });"#,
        None,
    );
    let source = "  let a = 1; // note\n";
    let parse = parse(
        source,
        JsFileSource::js_module(),
        JsParserOptions::default(),
    );
    let statement = parse
        .syntax()
        .descendants()
        .find(|node| plugin.query().contains(&node.kind().to_raw()))
        .unwrap();
    let result = plugin.evaluate(
        statement.into(),
        "/file.js".into(),
        &services(JsFileSource::js_module()),
    );
    let expected = [
        ("node", 2, 12),
        ("token", 2, 5),
        ("comment", 13, 20),
        ("whitespace", 5, 6),
        ("comment with fix", 13, 20),
    ];
    assert_eq!(result.entries.len(), expected.len(), "{result:?}");
    for (entry, (message, start, end)) in result.entries.iter().zip(expected) {
        assert_eq!(PrintDescription(&entry.diagnostic).to_string(), message);
        assert_eq!(
            entry.diagnostic.span(),
            Some(TextRange::new(start.into(), end.into())),
            "{message}"
        );
        assert_eq!(entry.action.is_some(), message == "comment with fix");
    }
    let action = result.entries[4].action.as_ref().unwrap();
    assert_eq!(
        action.text_edit.new_string(source),
        "  const a = 1; // note\n"
    );
}

#[test]
fn trivia_contract() {
    let plugin = load_test_plugin_from_source(
        "/plugin.js",
        r#"import { ast, defineRule, factory, registerDiagnostic } from "@biomejs/runtime/plugin";
        const assert = (condition, message) => {
            if (!condition) throw new Error(message);
        };
        const throwsTypeError = (callback, expected) => {
            try {
                callback();
            } catch (error) {
                assert(error instanceof TypeError, `${expected}: ${error}`);
                assert(error.message.includes(expected), `${expected}: ${error.message}`);
                return;
            }
            throw new Error(`Expected a TypeError: ${expected}`);
        };
        export const contract = defineRule({
            query: ast("JS_MODULE"),
            run(root) {
                const statement = root.items[0];
                const keyword = statement.declaration.token("kindToken");

                // Every call returns a fresh, ordinary array.
                for (const [owner, name] of [[root, "comments"], [statement, "leadingTrivia"],
                    [statement, "trailingTrivia"], [keyword, "leadingTrivia"], [keyword, "trailingTrivia"]]) {
                    const first = owner[name]();
                    const second = owner[name]();
                    assert(Array.isArray(first) && first !== second && first.length > 0, `${name} fresh array`);
                    const length = first.length;
                    first.length = 0;
                    assert(owner[name]().length === length, `${name} independent arrays`);
                }

                // Nodes without tokens and tokens without trivia return empty arrays.
                const directives = root.children()[0];
                assert(directives.kind === "JS_DIRECTIVE_LIST", "empty list");
                for (const empty of [directives.leadingTrivia(), directives.trailingTrivia(), directives.comments(),
                    factory.token("LET_KW").leadingTrivia(), factory.token("LET_KW").trailingTrivia()]) {
                    assert(Array.isArray(empty) && empty.length === 0, "empty trivia");
                }

                // `kind` and `text` are enumerable accessors on a shared prototype; `token` is not enumerable.
                const [comment, newline] = keyword.leadingTrivia();
                const prototype = Object.getPrototypeOf(comment);
                assert(prototype === Object.getPrototypeOf(newline), "shared prototype");
                assert(Object.keys(comment).length === 0, "no own properties");
                const keys = [];
                for (const key in comment) keys.push(key);
                assert(keys.join() === "kind,text", `enumerable keys: ${keys}`);
                assert(comment.token.kind === "LET_KW" && comment.token.text === "let", "owning token");
                assert(comment.token.parent.kind === "JS_VARIABLE_DECLARATION", "owning token parent");
                assert(comment.text === "/** doc */" && comment.kind === "singleLineComment", "comment");
                assert(newline.text === "\n" && newline.kind === "newline", "newline");
                for (const name of ["kind", "text", "token"]) {
                    const descriptor = Object.getOwnPropertyDescriptor(prototype, name);
                    assert(typeof descriptor.get === "function" && descriptor.set === undefined, `${name} getter`);
                    throwsTypeError(() => descriptor.get.call({}), `Cannot read piece.${name} because this value is not a trivia piece.`);
                }
                // Modules are strict, so assigning to a getter-only property throws.
                throwsTypeError(() => { comment.text = ""; }, "");
                throwsTypeError(() => new prototype.constructor(), "Trivia pieces do not have a public constructor.");

                throwsTypeError(() => root.comments.call(keyword), "comments() was called on a value that is not a node.");
                throwsTypeError(() => root.leadingTrivia.call(keyword), "leadingTrivia() was called on a value that is not a node.");
                throwsTypeError(() => root.trailingTrivia.call(comment), "trailingTrivia() was called on a value that is not a node.");
                throwsTypeError(() => keyword.leadingTrivia.call(root), "leadingTrivia() was called on a value that is not a token.");
                throwsTypeError(() => keyword.trailingTrivia.call(comment), "trailingTrivia() was called on a value that is not a token.");
                throwsTypeError(() => registerDiagnostic(comment.text, "information", "text"),
                    "The first argument to registerDiagnostic() is not a Biome node, token, or trivia piece.");
                throwsTypeError(() => registerDiagnostic(Object.create(comment), "information", "derived"),
                    "The first argument to registerDiagnostic() is not a Biome node, token, or trivia piece.");

                registerDiagnostic(root, "information", "Trivia contract holds");
            },
        });"#,
        None,
    );

    let result = evaluate(&plugin, "/** doc */\nlet a; // after\n");
    assert_eq!(messages(&result), ["Trivia contract holds"]);
}
