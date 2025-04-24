use std::{borrow::Cow, error::Error, fmt};

#[derive(Clone, Debug)]
/// Syntax error when parsing tags, not `<script>` or `<style>` tag.
pub struct SyntaxError {
  pub kind: SyntaxErrorKind,
  pub offset: u32,
  pub line: u32,
  pub column: u32,
}

#[derive(Clone, Debug)]
pub enum SyntaxErrorKind {
  ExpectAttrName,
  ExpectAttrValue,
  ExpectChar(char),
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
      SyntaxErrorKind::ExpectChar(c) => format!("expected char '{c}'").into(),
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

#[derive(Debug)]
/// The error type for markup_fmt.
pub enum FormatError<E> {
  /// Syntax error when parsing tags.
  Syntax(SyntaxError),
  /// Error from external formatter, for example,
  /// there're errors when formatting the `<script>` or `<style>` tag.
  External(Vec<E>),
}

impl<E> fmt::Display for FormatError<E>
where
  E: fmt::Display,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FormatError::Syntax(e) => e.fmt(f),
      FormatError::External(errors) => {
        writeln!(f, "failed to format code with external formatter:")?;
        for error in errors {
          writeln!(f, "{error}")?;
        }
        Ok(())
      }
    }
  }
}

impl<E> Error for FormatError<E> where E: Error {}
