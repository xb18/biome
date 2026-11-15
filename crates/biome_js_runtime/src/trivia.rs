//! Trivia exposed to plugins: `token.leadingTrivia()`, `token.trailingTrivia()`,
//! `node.leadingTrivia()`, `node.trailingTrivia()`, and `node.comments()`.
//!
//! Biome doesn't store whitespace, line breaks, comments, and skipped text as
//! syntax elements. Each token owns the trivia before it (leading) and the
//! trivia after it up to the next line break (trailing). Plugins receive this
//! trivia as `JsTriviaPiece` objects, one per piece, that remember their
//! owning token.

use crate::ast::JsAstNode;
use crate::token::JsAstToken;
use biome_js_syntax::JsLanguage;
use biome_rowan::{Direction, SyntaxTriviaPiece, TriviaPieceKind};
use boa_engine::class::{Class, ClassBuilder};
use boa_engine::object::JsObject;
use boa_engine::object::builtins::JsArray;
use boa_engine::property::Attribute;
use boa_engine::{
    Context, Finalize, JsData, JsNativeError, JsResult, JsString, JsValue, NativeFunction, Trace,
    js_string,
};

#[derive(Clone, Debug, JsData)]
pub(crate) struct JsTriviaPiece {
    pub(crate) piece: SyntaxTriviaPiece<JsLanguage>,
}

impl Finalize for JsTriviaPiece {}

// SAFETY: Rowan trivia pieces contain no Boa-managed values.
unsafe impl Trace for JsTriviaPiece {
    boa_engine::gc::empty_trace!();
}

impl JsTriviaPiece {
    pub(crate) fn register(context: &mut Context) -> JsResult<()> {
        context.register_global_class::<Self>()?;
        context
            .global_object()
            .delete_property_or_throw(js_string!(Self::NAME), context)?;
        Ok(())
    }

    /// Wraps a trivia piece. Call `register` first to attach the class prototype.
    fn from_piece(piece: SyntaxTriviaPiece<JsLanguage>, context: &mut Context) -> JsValue {
        let prototype = context
            .get_global_class::<Self>()
            .map(|class| class.prototype());
        JsObject::from_proto_and_data(prototype, Self { piece }).into()
    }

    pub(crate) fn from_value(value: &JsValue) -> Option<Self> {
        let object = value.as_object()?;
        let piece = object.downcast_ref::<Self>()?;
        Some(piece.clone())
    }

    fn kind_name(kind: TriviaPieceKind) -> JsString {
        match kind {
            TriviaPieceKind::Newline => js_string!("newline"),
            TriviaPieceKind::Whitespace => js_string!("whitespace"),
            TriviaPieceKind::SingleLineComment => js_string!("singleLineComment"),
            TriviaPieceKind::MultiLineComment => js_string!("multiLineComment"),
            TriviaPieceKind::Skipped => js_string!("skipped"),
        }
    }

    fn get_kind(this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        let piece = Self::from_value(this).ok_or_else(|| {
            JsNativeError::typ().with_message("Cannot read piece.kind because this value is not a trivia piece. Use a piece returned by leadingTrivia(), trailingTrivia(), or node.comments().")
        })?;
        Ok(Self::kind_name(piece.piece.kind()).into())
    }

    fn get_text(this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        let piece = Self::from_value(this).ok_or_else(|| {
            JsNativeError::typ().with_message("Cannot read piece.text because this value is not a trivia piece. Use a piece returned by leadingTrivia(), trailingTrivia(), or node.comments().")
        })?;
        Ok(JsString::from(piece.piece.text()).into())
    }

    fn get_token(this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let piece = Self::from_value(this).ok_or_else(|| {
            JsNativeError::typ().with_message("Cannot read piece.token because this value is not a trivia piece. Use a piece returned by leadingTrivia(), trailingTrivia(), or node.comments().")
        })?;
        Ok(JsAstToken::from_token(piece.piece.token(), context))
    }

    fn wrap_pieces<I>(pieces: I, context: &mut Context) -> JsValue
    where
        I: IntoIterator<Item = SyntaxTriviaPiece<JsLanguage>>,
    {
        let pieces = pieces
            .into_iter()
            .map(|piece| Self::from_piece(piece, context))
            .collect::<Vec<_>>();
        JsArray::from_iter(pieces, context).into()
    }
}

impl Class for JsTriviaPiece {
    const NAME: &'static str = "__JsTriviaPiece";

    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        let kind =
            NativeFunction::from_fn_ptr(Self::get_kind).to_js_function(class.context().realm());
        let text =
            NativeFunction::from_fn_ptr(Self::get_text).to_js_function(class.context().realm());
        let token =
            NativeFunction::from_fn_ptr(Self::get_token).to_js_function(class.context().realm());
        class
            .accessor(js_string!("kind"), Some(kind), None, Attribute::ENUMERABLE)
            .accessor(js_string!("text"), Some(text), None, Attribute::ENUMERABLE)
            .accessor(js_string!("token"), Some(token), None, Attribute::empty());
        Ok(())
    }

    fn data_constructor(
        _new_target: &JsValue,
        _args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<Self> {
        Err(JsNativeError::typ()
            .with_message("Trivia pieces do not have a public constructor. Obtain them from leadingTrivia(), trailingTrivia(), or node.comments().")
            .into())
    }
}

impl JsAstToken {
    /// Implements `token.leadingTrivia()`.
    pub(crate) fn leading_trivia(
        this: &JsValue,
        _args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let token = Self::from_value(this).ok_or_else(|| {
            JsNativeError::typ().with_message("leadingTrivia() was called on a value that is not a token. Call token.leadingTrivia() on a token returned by node.token(field), node.childrenWithTokens(), or factory.token().")
        })?;
        Ok(JsTriviaPiece::wrap_pieces(
            token.token.leading_trivia().pieces(),
            context,
        ))
    }

    /// Implements `token.trailingTrivia()`.
    pub(crate) fn trailing_trivia(
        this: &JsValue,
        _args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let token = Self::from_value(this).ok_or_else(|| {
            JsNativeError::typ().with_message("trailingTrivia() was called on a value that is not a token. Call token.trailingTrivia() on a token returned by node.token(field), node.childrenWithTokens(), or factory.token().")
        })?;
        Ok(JsTriviaPiece::wrap_pieces(
            token.token.trailing_trivia().pieces(),
            context,
        ))
    }
}

impl JsAstNode {
    /// Implements `node.leadingTrivia()`: the leading trivia of the first token
    /// of the node, or an empty array for a node without tokens.
    pub(crate) fn leading_trivia(
        this: &JsValue,
        _args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let Some(Self { node }) = Self::from_value(this) else {
            return Err(JsNativeError::typ()
                .with_message("leadingTrivia() was called on a value that is not a node. Call node.leadingTrivia() on the node passed to run() or a node obtained by traversing it.")
                .into());
        };
        let pieces = node
            .first_token()
            .into_iter()
            .flat_map(|token| token.leading_trivia().pieces());
        Ok(JsTriviaPiece::wrap_pieces(pieces, context))
    }

    /// Implements `node.trailingTrivia()`: the trailing trivia of the last
    /// token of the node, or an empty array for a node without tokens.
    pub(crate) fn trailing_trivia(
        this: &JsValue,
        _args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let Some(Self { node }) = Self::from_value(this) else {
            return Err(JsNativeError::typ()
                .with_message("trailingTrivia() was called on a value that is not a node. Call node.trailingTrivia() on the node passed to run() or a node obtained by traversing it.")
                .into());
        };
        let pieces = node
            .last_token()
            .into_iter()
            .flat_map(|token| token.trailing_trivia().pieces());
        Ok(JsTriviaPiece::wrap_pieces(pieces, context))
    }

    /// Implements `node.comments()`: the comments in the leading and trailing
    /// trivia of every token of the node, in source order.
    pub(crate) fn comments(
        this: &JsValue,
        _args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let Some(Self { node }) = Self::from_value(this) else {
            return Err(JsNativeError::typ()
                .with_message("comments() was called on a value that is not a node. Call node.comments() on the node passed to run() or a node obtained by traversing it.")
                .into());
        };
        let comments = node
            .descendants_tokens(Direction::Next)
            .flat_map(|token| {
                token
                    .leading_trivia()
                    .pieces()
                    .chain(token.trailing_trivia().pieces())
            })
            .filter(|piece| piece.is_comments());
        Ok(JsTriviaPiece::wrap_pieces(comments, context))
    }
}
