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
  state::ParseState,
};

pub type PResult<T> = Result<T, SyntaxError>;

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
      state: ParseState::new(source),
    }
  }

  fn emit_error(&mut self, kind: SyntaxErrorKind) -> SyntaxError {
    self.state.add_error(kind)
  }

  fn parse_attr(&mut self) -> PResult<Attribute> {
    let start = self.state.position();
    let name = self.parse_attr_name()?;
    let value = if self.state.consume_str("=").is_some() {
      Some(self.parse_attr_value()?)
    } else {
      None
    };
    let end = self.state.position();
    // TODO: add support for dynamic attributes
    Ok(Attribute::Static {
      name: name.to_string(),
      value: match value {
        Some((s, range)) => Some(vec![AttributeValue::Text {
          content: s.to_string(),
          loc: range,
        }]),
        None => None,
      },
      loc: Range { start, end },
    })
  }

  fn parse_attr_name(&mut self) -> PResult<&'s str> {
    let start = self.state.position().offset as usize;
    while let Some(c) = self.state.peek() {
      if is_attr_name_char(c) {
        self.state.next();
      } else {
        break;
      }
    }
    let end = self.state.position().offset as usize;
    let ret = self.state.code_slice([start, end]);
    if ret.is_empty() {
      return Err(self.emit_error(SyntaxErrorKind::ExpectAttrName));
    }
    Ok(ret)
  }

  fn parse_attr_value(&mut self) -> PResult<(&'s str, Range)> {
    let quote = self.state.peek();
    // 如果 quote 是单引号或双引号，则开始解析
    // 否则报错
    match quote {
      Some('\"') | Some('\'') => {
        self.state.next();
        let quote = quote.unwrap().to_string();
        let start = self.state.position();
        let value = self.state.skip_until_before(&quote).unwrap_or("");
        let end = self.state.position();
        Ok((value, Range { start, end }))
      }
      _ => Err(self.emit_error(SyntaxErrorKind::ExpectAttrValue)),
    }
  }

  fn parse_comment(&mut self) -> PResult<Node> {
    let start = self.state.position();
    self.state.consume_str("<!--");
    self.state.skip_whitespace();

    match self.state.skip_until_after("-->") {
      Some(s) => Ok(Node::Comment {
        content: s.to_string(),
        loc: Range {
          start,
          end: self.state.position(),
        },
      }),
      None => {
        return Err(self.emit_error(SyntaxErrorKind::ExpectComment));
      }
    }
  }

  fn parse_element(&mut self) -> PResult<Node> {
    let start = self.state.position();
    self.state.consume_str("<");
    let tag_name = self.parse_tag_name()?;
    let mut attrs = vec![];
    let mut first_attr_same_line = true;
    loop {
      match self.state.peek() {
        // 自闭和合标签
        Some('/') => {
          self.state.next();
          if let Some(range) = self.state.consume_str("/>") {
            return Ok(Node::Element {
              name: tag_name.to_string(),
              attrs,
              first_attr_same_line,
              children: vec![],
              self_closing: true,
              loc: Range {
                start,
                end: range.end,
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
    let tag_end = format!("</{}>", tag_name);
    if tag_name.eq_ignore_ascii_case("script") || tag_name.eq_ignore_ascii_case("wxs") {
      let script_start = self.state.position();
      if let Some(raw) = self.state.skip_until_before(tag_end.as_str()) {
        let script_end = self.state.position();
        self.state.consume_str(tag_end.as_str());
        return Ok(Node::Element {
          name: tag_name.to_string(),
          attrs,
          first_attr_same_line,
          children: vec![Node::Text {
            content: raw.to_string(),
            loc: Range {
              start: script_start,
              end: script_end,
            },
          }],
          self_closing: false,
          loc: Range {
            start,
            end: self.state.position(),
          },
        });
      }
      return Err(self.emit_error(SyntaxErrorKind::ExpectCloseTag));
    }

    loop {
      match self.state.peek() {
        Some(_) => {
          if self.state.consume_str("</").is_some() {
            let close_tag_name = self.parse_tag_name()?;
            if !close_tag_name.eq_ignore_ascii_case(tag_name) {
              return Err(self.emit_error(SyntaxErrorKind::ExpectCloseTag));
            }
            self.state.skip_whitespace();
            match self.state.peek() {
              Some('>') => {
                self.state.next();
                break;
              }
              _ => {
                return Err(self.emit_error(SyntaxErrorKind::ExpectCloseTag));
              }
            }
          } else {
            children.extend(self.parse_node()?);
          }
        }
        None => return Err(self.emit_error(SyntaxErrorKind::ExpectCloseTag)),
      }
    }

    Ok(Node::Element {
      name: tag_name.to_string(),
      attrs,
      first_attr_same_line,
      children,
      self_closing: false,
      loc: Range {
        start,
        end: self.state.position(),
      },
    })
  }

  fn parse_expression(&mut self) -> PResult<Node> {
    let start = self.state.position();
    let raw = self.state.skip_until_after("}}");
    let end = self.state.position();
    match raw {
      Some(s) => Ok(Node::Expression {
        content: s.to_string(),
        loc: Range { start, end },
      }),
      None => Err(self.emit_error(SyntaxErrorKind::ExpectExpression)),
    }
  }

  fn parse_node(&mut self) -> PResult<Vec<Node>> {
    let ret = &mut vec![];
    loop {
      if self.state.ended() {
        // tag end, returns
        break;
      }
      if self.state.peek_str("<!--") {
        let node = self.parse_comment()?;
        ret.push(node);
        continue;
      }
      if self.state.peek_str("{{") {
        let node = self.parse_expression()?;
        ret.push(node);
        continue;
      }
      if let Some([peek, peek2]) = self.state.peek_n() {
        if peek == '<' && is_tag_name_char(peek2) {
          let node = self.parse_element()?;
          ret.push(node);
          continue;
        }
      }
      let text_start = self.state.position();
      if let Some(raw) = self.state.skip_until_with(vec!["<", "{{"]) {
        ret.push(Node::Text {
          content: raw.to_string(),
          loc: Range {
            start: text_start,
            end: self.state.position(),
          },
        });
      }
    }
    Ok(ret.to_vec())
  }

  pub fn parse_root(&mut self) -> PResult<Root> {
    let start = self.state.position();
    let children = self.parse_node()?;

    Ok(Root {
      children,
      loc: Range {
        start: start,
        end: self.state.position(),
      },
    })
  }

  fn parse_tag_name(&mut self) -> PResult<&'s str> {
    let start = self.state.position().offset as usize;
    while let Some(c) = self.state.peek() {
      if is_tag_name_char(c) {
        self.state.next();
      } else {
        break;
      }
    }
    let end = self.state.position().offset as usize;
    let ret = self.state.code_slice([start, end]);
    if ret.is_empty() {
      return Err(self.emit_error(SyntaxErrorKind::ExpectTagName));
    }
    Ok(ret)
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
