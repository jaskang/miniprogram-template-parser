use napi_derive::napi;
use std::{borrow::Cow, error::Error, fmt};

#[derive(Debug, Clone, Copy)]
#[napi(object)]
/// Syntax error when parsing tags, not `<script>` or `<style>` tag.
pub struct SyntaxError {
  pub kind: SyntaxErrorKind,
  pub offset: u32,
  pub line: u32,
  pub column: u32,
}

#[derive(Debug, Clone, Copy)]
#[napi]
pub enum SyntaxErrorKind {
  ExpectAttrName,
  ExpectAttrValue,
  ExpectCloseTag,
  ExpectComment,
  ExpectDoctype,
  ExpectElement,
  ExpectFrontMatter,
  ExpectIdentifier,
  ExpectMustacheInterpolation,
  ExpectSelfCloseTag,
  ExpectTagName,
  ExpectTextNode,
  ExpectExpression,
}

impl fmt::Display for SyntaxErrorKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let reason: Cow<_> = match self {
      SyntaxErrorKind::ExpectAttrName => "expected attribute name".into(),
      SyntaxErrorKind::ExpectAttrValue => "expected attribute value".into(),
      SyntaxErrorKind::ExpectCloseTag => "expected close tag".into(),
      SyntaxErrorKind::ExpectComment => "expected comment".into(),
      SyntaxErrorKind::ExpectDoctype => "expected HTML doctype".into(),
      SyntaxErrorKind::ExpectElement => "expected element".into(),
      SyntaxErrorKind::ExpectFrontMatter => "expected front matter".into(),
      SyntaxErrorKind::ExpectIdentifier => "expected identifier".into(),
      SyntaxErrorKind::ExpectMustacheInterpolation => "expected mustache-like interpolation".into(),
      SyntaxErrorKind::ExpectSelfCloseTag => "expected self close tag".into(),
      SyntaxErrorKind::ExpectTagName => "expected tag name".into(),
      SyntaxErrorKind::ExpectTextNode => "expected text node".into(),
      SyntaxErrorKind::ExpectExpression => "expected expression".into(),
    };

    write!(f, "{reason}")
  }
}

impl fmt::Display for SyntaxError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "syntax error '{}' at line {}, column {}",
      self.kind, self.line, self.column
    )
  }
}

impl Error for SyntaxError {}
