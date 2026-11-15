---
"@biomejs/biome": patch
---

Added trivia APIs to JavaScript plugins. `node.comments()` returns every comment in the trivia of a node's tokens, including the comments directly before and after the node, and `leadingTrivia()` and `trailingTrivia()` on nodes and tokens return the whitespace, line breaks, comments, and skipped text around them. `registerDiagnostic()` now also accepts tokens and trivia pieces, and highlights only that element.

```js
for (const comment of root.comments()) {
  if (comment.text.includes("TODO")) {
    registerDiagnostic(comment, "warning", "Resolve this TODO.");
  }
}
```
