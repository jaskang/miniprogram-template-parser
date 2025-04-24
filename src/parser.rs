//! This parser is designed for internal use,
//! not generating general-purpose AST.
//!
//! Also, the parser consumes string then produces AST directly without tokenizing.
//! For a formal parser, it should be:
//! `source -> tokens (produced by lexer/tokenizer) -> AST (produced by parser)`.
//! So, if you're learning or looking for a parser,
//! this is not a good example and you should look for other projects.

use crate::{
  ast::*,
  error::{SyntaxError, SyntaxErrorKind},
  helpers,
  state::ParseState,
};
use std::{cmp::Ordering, f32::consts::E, iter::Peekable, ops::ControlFlow, str::CharIndices};

/// Parser结构体表示模板解析器的状态
///
/// 字段说明：
/// * `source` - 待解析的源代码字符串
/// * `chars` - 字符迭代器，用于遍历源代码
///    使用char_indices()可以同时获取字符和它在源码中的位置索引
///    使用peekable()包装使迭代器支持预览下一个字符而不消费它
pub struct Parser<'s> {
  source: &'s str,
  state: ParseState<'s>,
}

impl<'s> Parser<'s> {
  pub fn new(source: &'s str) -> Self {
    Self {
      source,
      state: ParseState::new("", source),
    }
  }

  fn emit_error(&mut self, kind: SyntaxErrorKind) -> SyntaxError {
    self.state.add_error(kind)
  }

  // fn with_taken<T, F>(&mut self, parser: F) -> PResult<(T, &'s str)>
  // where
  //   F: FnOnce(&mut Self) -> PResult<T>,
  // {
  //   let start = self
  //     .chars
  //     .peek()
  //     .map(|(i, _)| *i)
  //     .unwrap_or(self.source.len());
  //   let parsed = parser(self)?;
  //   let end = self
  //     .chars
  //     .peek()
  //     .map(|(i, _)| *i)
  //     .unwrap_or(self.source.len());
  //   Ok((parsed, unsafe { self.source.get_unchecked(start..end) }))
  // }

  fn parse_attr(&mut self) -> PResult<Attribute> {
    let name = self.parse_attr_name()?;
    self.skip_ws();
    let value = if self.chars.next_if(|(_, c)| *c == '=').is_some() {
      self.skip_ws();
      Some(self.parse_attr_value()?)
    } else {
      None
    };
    // TODO: add support for dynamic attributes
    Ok(Attribute::Static(StaticAttribute {
      name: name.to_string(),
      value: match value {
        Some((value, _)) => Some(value.to_string()),
        None => None,
      },
      loc: Range {
        start: Position {
          offset: 0,
          line: 0,
          column: 0,
        },
        end: Position {
          offset: 0,
          line: 0,
          column: 0,
        },
        source: String::new(),
      },
    }))
  }

  fn parse_attr_name(&mut self) -> PResult<&'s str> {
    let Some((start, start_char)) = self.chars.next_if(|(_, c)| is_attr_name_char(*c)) else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectAttrName));
    };
    let mut end = start + start_char.len_utf8();

    while let Some((_, c)) = self.chars.next_if(|(_, c)| is_attr_name_char(*c)) {
      end += c.len_utf8();
    }

    unsafe { Ok(self.source.get_unchecked(start..end)) }
  }

  fn parse_attr_value(&mut self) -> PResult<(&'s str, usize)> {
    let quote = self.chars.next_if(|(_, c)| *c == '"');

    if let Some((start, quote)) = quote {
      let start = start + 1;
      let mut end = start;
      let mut chars_stack = vec![];
      loop {
        match self.chars.next() {
          Some((i, c)) if c == quote => {
            if chars_stack.is_empty() {
              end = i;
              break;
            } else if chars_stack.last().is_some_and(|last| *last == c) {
              chars_stack.pop();
            } else {
              chars_stack.push(c);
            }
          }
          Some(..) => continue,
          None => break,
        }
      }
      Ok((unsafe { self.source.get_unchecked(start..end) }, start))
    } else {
      fn is_unquoted_attr_value_char(c: char) -> bool {
        !c.is_ascii_whitespace() && !matches!(c, '"' | '\'' | '=' | '<' | '>' | '`')
      }

      let start = match self.chars.peek() {
        Some((i, c)) if is_unquoted_attr_value_char(*c) => *i,
        _ => return Err(self.emit_error(SyntaxErrorKind::ExpectAttrValue)),
      };

      let mut end = start;
      loop {
        match self.chars.peek() {
          Some((i, c)) if is_unquoted_attr_value_char(*c) => {
            end = *i;
            self.chars.next();
          }
          _ => break,
        }
      }

      Ok((unsafe { self.source.get_unchecked(start..=end) }, start))
    }
  }

  fn parse_comment(&mut self) -> PResult<Comment> {
    let Some((start, _)) = self
      .chars
      .next_if(|(_, c)| *c == '<')
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '!'))
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '-'))
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '-'))
    else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectComment));
    };
    let start = start + 1;

    let mut end = start;
    loop {
      match self.chars.next() {
        Some((i, '-')) => {
          let mut chars = self.chars.clone();
          if chars
            .next_if(|(_, c)| *c == '-')
            .and_then(|_| chars.next_if(|(_, c)| *c == '>'))
            .is_some()
          {
            end = i;
            self.chars = chars;
            break;
          }
        }
        Some(..) => continue,
        None => break,
      }
    }

    Ok(Comment {
      raw: unsafe { self.source.get_unchecked(start..end) }.to_string(),
      loc: Range {
        start: Position {
          offset: start as u32,
          line: 0,
          column: 0,
        },
        end: Position {
          offset: end as u32,
          line: 0,
          column: 0,
        },
        source: String::new(),
      },
    })
  }

  fn parse_element(&mut self) -> PResult<Element> {
    let Some(..) = self.state.peek() else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectElement));
    };
    let start = self.state.position();

    let tag_name = self.parse_tag_name()?;

    let mut attrs = vec![];
    let mut first_attr_same_line = true;
    loop {
      match self.state.peek() {
        // 自闭和合标签
        Some('/') => {
          self.state.next();
          if self.state.next_if(|(_, c)| *c == '>').is_some() {
            return Ok(Element {
              name: tag_name.to_string(),
              attrs,
              first_attr_same_line,
              children: vec![],
              self_closing: true,
              loc: Range {
                start: Position {
                  offset: start as u32,
                  line: 0,
                  column: 0,
                },
                end: Position {
                  offset: 0,
                  line: 0,
                  column: 0,
                },
                source: String::new(),
              },
            });
          }
          return Err(self.emit_error(SyntaxErrorKind::ExpectSelfCloseTag));
        }
        Some('>') => {
          self.state.next();
          break;
        }
        Some('\n') => {
          if attrs.is_empty() {
            first_attr_same_line = false;
          }
          self.state.next();
        }
        Some(c) if c.is_ascii_whitespace() => {
          self.state.next();
        }
        _ => {
          attrs.push(self.parse_attr()?);
        }
      }
    }

    let mut children: Vec<Node> = vec![];
    if tag_name.eq_ignore_ascii_case("script") || tag_name.eq_ignore_ascii_case("wxs") {
      let text_node = self.parse_raw_text_node(tag_name)?;
      let raw = text_node.raw;
      if !raw.is_empty() {
        children.push(Node::Text(Text {
          raw,
          loc: Range {
            start: Position {
              offset: 0,
              line: 0,
              column: 0,
            },
            end: Position {
              offset: 0,
              line: 0,
              column: 0,
            },
            source: String::new(),
          },
        }));
      }
    }

    loop {
      match self.chars.peek() {
        Some((_, '<')) => {
          let mut chars = self.chars.clone();
          chars.next();
          if let Some((pos, _)) = chars.next_if(|(_, c)| *c == '/') {
            self.chars = chars;
            let close_tag_name = self.parse_tag_name()?;
            if !close_tag_name.eq_ignore_ascii_case(tag_name) {
              return Err(self.emit_error_with_pos(SyntaxErrorKind::ExpectCloseTag, pos));
            }
            self.skip_ws();
            if self.chars.next_if(|(_, c)| *c == '>').is_some() {
              break;
            }
            return Err(self.emit_error(SyntaxErrorKind::ExpectCloseTag));
          }
          children.push(self.parse_node()?);
        }
        Some(..) => {
          if tag_name.eq_ignore_ascii_case("script") || tag_name.eq_ignore_ascii_case("wxs") {
            let text_node = self.parse_raw_text_node(tag_name)?;
            let raw = text_node.raw.clone();
            if !raw.is_empty() {
              children.push(Node::Text(text_node));
            }
          } else {
            children.push(self.parse_node()?);
          }
        }
        None => return Err(self.emit_error(SyntaxErrorKind::ExpectCloseTag)),
      }
    }

    Ok(Element {
      name: tag_name.to_string(),
      attrs,
      first_attr_same_line,
      children,
      self_closing: false,
      loc: Range {
        start: Position {
          offset: start as u32,
          line: 0,
          column: 0,
        },
        end: Position {
          offset: 0,
          line: 0,
          column: 0,
        },
        source: String::new(),
      },
    })
  }

  /// This will consume the open and close char.
  fn parse_inside(&mut self, open: char, close: char, inclusive: bool) -> PResult<&'s str> {
    let Some(start) =
      self.chars.next_if(|(_, c)| *c == open).map(
        |(i, c)| {
          if inclusive {
            i
          } else {
            i + c.len_utf8()
          }
        },
      )
    else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectChar(open)));
    };
    let mut end = start;
    let mut stack = 0u8;
    for (i, c) in self.chars.by_ref() {
      if c == open {
        stack += 1;
      } else if c == close {
        if stack == 0 {
          end = if inclusive { i + close.len_utf8() } else { i };
          break;
        }
        stack -= 1;
      }
    }
    Ok(unsafe { self.source.get_unchecked(start..end) })
  }

  fn parse_mustache_interpolation(&mut self) -> PResult<(&'s str, usize)> {
    let Some((start, _)) = self
      .chars
      .next_if(|(_, c)| *c == '{')
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '{'))
    else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectMustacheInterpolation));
    };
    let start = start + 1;

    let mut braces_stack = 0usize;
    let mut end = start;
    loop {
      match self.chars.next() {
        Some((_, '{')) => braces_stack += 1,
        Some((i, '}')) => {
          if braces_stack == 0 {
            if self.chars.next_if(|(_, c)| *c == '}').is_some() {
              end = i;
              break;
            }
          } else {
            braces_stack -= 1;
          }
        }
        Some(..) => continue,
        None => break,
      }
    }

    Ok((unsafe { self.source.get_unchecked(start..end) }, start))
  }

  // fn parse_node(&mut self) -> PResult<Node> {
  //   let (kind, raw) = self.with_taken(Parser::parse_node_kind)?;
  //   Ok(Node { kind, raw })
  // }

  fn parse_node(&mut self) -> PResult<Node> {
    match self.state.peek_n() {
      Some(['<', c]) => {
        if is_tag_name_char(c) {
          self.parse_element().map(Node::Element)
        } else if c == '!' {
          self.parse_comment().map(Node::Comment)
        } else {
          self.parse_text_node().map(Node::Text)
        }
      }
      Some(['{', '{']) => {
        let start = self.state.position();
        let (expr, _) = self.parse_mustache_interpolation()?;
        let end = self.state.position();

        Ok(Node::Expression(Expression {
          raw: expr.to_string(),
          loc: Range { start, end },
        }))
      }
      Some(..) => self.parse_text_node().map(Node::Text),
      None => Err(self.emit_error(SyntaxErrorKind::ExpectElement)),
    }
  }

  fn parse_raw_text_node(&mut self, tag_name: &str) -> PResult<Text> {
    let start = self
      .chars
      .peek()
      .map(|(i, _)| *i)
      .unwrap_or(self.source.len());

    let allow_nested = tag_name.eq_ignore_ascii_case("pre");
    let mut nested = 0u16;
    let mut line_breaks = 0;
    let end;
    loop {
      match self.chars.peek() {
        Some((i, '<')) => {
          let i = *i;
          let mut chars = self.chars.clone();
          chars.next();
          if chars.next_if(|(_, c)| *c == '/').is_some()
            && chars
              .by_ref()
              .zip(tag_name.chars())
              .all(|((_, a), b)| a.eq_ignore_ascii_case(&b))
          {
            if nested == 0 {
              end = i;
              break;
            } else {
              nested -= 1;
              self.chars = chars;
              continue;
            }
          } else if allow_nested
            && chars
              .by_ref()
              .zip(tag_name.chars())
              .all(|((_, a), b)| a.eq_ignore_ascii_case(&b))
          {
            nested += 1;
            self.chars = chars;
            continue;
          }
          self.chars.next();
        }
        Some((_, c)) => {
          if *c == '\n' {
            line_breaks += 1;
          }
          self.chars.next();
        }
        None => {
          end = self.source.len();
          break;
        }
      }
    }

    Ok(Text {
      raw: unsafe { self.source.get_unchecked(start..end) }.to_string(),
      loc: Range {
        start: Position {
          offset: start as u32,
          line: 1,
          column: 1,
        },
        end: Position {
          offset: end as u32,
          line: 1,
          column: 1,
        },
        source: String::new(),
      },
    })
  }

  pub fn parse_root(&mut self) -> PResult<Root> {
    let mut children = vec![];
    let start = self.state.position();
    while self.state.peek().is_some() {
      children.push(self.parse_node()?);
    }

    Ok(Root {
      children,
      loc: Range {
        start: start,
        end: self.state.position(),
      },
    })
  }

  fn parse_tag_name(&mut self) -> PResult<String> {
    let mut ret = vec![];

    while let Some(c) = self.state.peek() {
      if is_tag_name_char(c) {
        ret.push(c);
        self.state.next();
      } else {
        break;
      }
    }
    if ret.is_empty() {
      return Err(self.emit_error(SyntaxErrorKind::ExpectTagName));
    }
    Ok(ret.iter().collect())
  }

  fn parse_text_node(&mut self) -> PResult<Text> {
    let Some((start, first_char)) = self.chars.next_if(|(_, c)| *c != '{') else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectTextNode));
    };

    if first_char == '{' && matches!(self.chars.peek(), Some((_, '{'))) {
      return Err(self.emit_error(SyntaxErrorKind::ExpectTextNode));
    }

    let mut line_breaks = if first_char == '\n' { 1 } else { 0 };
    let end;
    loop {
      match self.chars.peek() {
        Some((i, '{')) => {
          let i = *i;
          let mut chars = self.chars.clone();
          chars.next();
          if chars.next_if(|(_, c)| *c == '{').is_some() {
            end = i;
            break;
          }
          self.chars.next();
        }
        Some((i, '<')) => {
          let i = *i;
          let mut chars = self.chars.clone();
          chars.next();
          match chars.next() {
            Some((_, c)) if is_tag_name_char(c) || c == '/' || c == '!' => {
              end = i;
              break;
            }
            _ => {
              self.chars.next();
            }
          }
        }
        Some((_, c)) => {
          if *c == '\n' {
            line_breaks += 1;
          }
          self.chars.next();
        }
        None => {
          end = self.source.len();
          break;
        }
      }
    }

    Ok(Text {
      raw: unsafe { self.source.get_unchecked(start..end) }.to_string(),
      loc: Range {
        start: Position {
          offset: start as u32,
          line: 1,
          column: 1,
        },
        end: Position {
          offset: end as u32,
          line: 1,
          column: 1,
        },
        source: String::new(),
      },
    })
  }
}

/// Returns true if the provided character is a valid HTML tag name character.
fn is_tag_name_char(c: char) -> bool {
  c.is_ascii_alphanumeric()
    || c == '-'
    || c == '_'
    || c == '.'
    || c == ':'
    || !c.is_ascii()
    || c == '\\'
}

fn is_attr_name_char(c: char) -> bool {
  !matches!(c, '"' | '\'' | '>' | '/' | '=') && !c.is_ascii_whitespace()
}

pub type PResult<T> = Result<T, SyntaxError>;
