import {
	type AnyJsAstNode,
	type JsAstNode,
	type JsAstToken,
	type JsTriviaKind,
	type JsTriviaPiece,
	type JsVariableStatement,
	factory,
	registerDiagnostic,
} from "@biomejs/runtime/plugin";

declare const base: JsAstNode;
declare const anyNode: AnyJsAstNode;
declare const statement: JsVariableStatement;
declare const token: JsAstToken;

base.leadingTrivia() satisfies readonly JsTriviaPiece[];
base.trailingTrivia() satisfies readonly JsTriviaPiece[];
base.comments() satisfies readonly JsTriviaPiece[];
anyNode.comments() satisfies readonly JsTriviaPiece[];
statement.leadingTrivia() satisfies readonly JsTriviaPiece[];
token.leadingTrivia() satisfies readonly JsTriviaPiece[];
token.trailingTrivia() satisfies readonly JsTriviaPiece[];
factory.token("LET_KW").leadingTrivia() satisfies readonly JsTriviaPiece[];

const [piece] = base.comments();
if (piece) {
	piece.kind satisfies JsTriviaKind;
	piece.text satisfies string;
	piece.token satisfies JsAstToken;
	piece.token.leadingTrivia() satisfies readonly JsTriviaPiece[];

	switch (piece.kind) {
		case "newline":
		case "whitespace":
		case "singleLineComment":
		case "multiLineComment":
		case "skipped":
			break;
		default:
			piece.kind satisfies never;
	}

	// @ts-expect-error Trivia kinds are not syntax kinds.
	piece.kind === "COMMENT";
	// @ts-expect-error Pieces are read-only.
	piece.text = "";
	// @ts-expect-error Pieces are read-only.
	piece.token = token;
}

// @ts-expect-error Trivia arrays are not mutable.
base.comments() satisfies JsTriviaPiece[];
// @ts-expect-error Trivia arrays cannot be appended.
token.leadingTrivia().push(piece);
// @ts-expect-error Trivia pieces do not have their own trivia.
piece?.leadingTrivia();

registerDiagnostic(base, "information", "node");
registerDiagnostic(anyNode, "information", "node");
registerDiagnostic(token, "information", "token");
registerDiagnostic(base.comments()[0]!, "information", "comment");
// @ts-expect-error Diagnostics need a node, token, or trivia piece.
registerDiagnostic("text", "information", "string");
// @ts-expect-error Diagnostics need a node, token, or trivia piece.
registerDiagnostic(base.comments(), "information", "array");
