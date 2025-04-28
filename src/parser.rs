//! This parser is designed for internal use,
//! not generating general-purpose AST.
//!
//! Also, the parser consumes string then produces AST directly without tokenizing.
//! For a formal parser, it should be:
//! `source -> tokens (produced by lexer/tokenizer) -> AST (produced by parser)`.
//! So, if you're learning or looking for a parser,
//! this is not a good example and you should look for other projects.

use std::vec;

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
    let values = if self.state.consume_str("=").is_some() {
      Some(self.parse_attr_value()?)
    } else {
      None
    };
    let end = self.state.position();
    // TODO: add support for dynamic attributes
    Ok(Attribute {
      name: name.to_string(),
      value: match values {
        Some(attr_values) => Some(attr_values),
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
  /// 解析属性值
  /// 属性值可以是单引号或双引号包裹的字符串
  /// 返回值是一个 AttributeValue 的 Vec
  /// AttributeValue 是一个枚举类型，表示属性值的类型，有 Text 和 Expression
  /// 例如：`<div id="myId {{ clx }}">` 中的 "myId {{ clx }}" 会被解析为两个 AttributeValue
  /// 分别是 Text 和 Expression
  /// 其中 "myId" 是 Text 类型，"{{ clx }}" 是 Expression 类型
  fn parse_attr_value(&mut self) -> PResult<Vec<AttributeValue>> {
    // 如果 quote 是单引号或双引号，则开始解析
    let ret = &mut vec![];
    let quote = self.state.peek();
    if quote != Some('"') && quote != Some('\'') {
      return Err(self.emit_error(SyntaxErrorKind::ExpectAttrValue));
    }
    self.state.next();
    loop {
      if self.state.peek() == Some(quote.unwrap()) {
        self.state.next();
        break;
      }
      if self.state.peek_str("{{") {
        if let Node::Expression { content, loc } = self.parse_expression()? {
          ret.push(AttributeValue::Expression { content, loc });
        }
      } else {
        let quote_str = quote.unwrap().to_string();
        if let Node::Text { content, loc } = self.parse_text(vec!["{{", &quote_str])? {
          ret.push(AttributeValue::Text { content, loc });
        } else {
          return Err(self.emit_error(SyntaxErrorKind::ExpectAttrValue));
        }
      }
    }
    Ok(ret.to_vec())
  }

  fn parse_comment(&mut self) -> PResult<Node> {
    let start = self.state.position();
    self.state.consume_str("<!--");
    self.state.skip_whitespace();

    match self.state.skip_until_after(vec!["-->"]) {
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
      if let Some(raw) = self.state.skip_until_before(vec![tag_end.as_str()]) {
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
  fn parse_text(&mut self, end: Vec<&str>) -> PResult<Node> {
    let start = self.state.position();
    let raw = self.state.skip_until_before(end);
    let end = self.state.position();
    match raw {
      Some(s) => Ok(Node::Text {
        content: s.to_string(),
        loc: Range { start, end },
      }),
      None => Err(self.emit_error(SyntaxErrorKind::ExpectTextNode)),
    }
  }
  fn parse_expression(&mut self) -> PResult<Node> {
    let start = self.state.position();
    let raw = self.state.skip_until_after(vec!["}}"]);
    let end = self.state.position();
    match raw {
      Some(s) => Ok(Node::Expression {
        content: format!("{}}}}}", s),
        loc: Range { start, end },
      }),
      None => Err(self.emit_error(SyntaxErrorKind::ExpectExpression)),
    }
  }

  fn parse_node(&mut self) -> PResult<Vec<Node>> {
    let ret = &mut vec![];
    loop {
      if self.state.ended() || self.state.peek_str("</") {
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
      if let Some(raw) = self.state.skip_until_before(vec!["<", "{{"]) {
        ret.push(Node::Text {
          content: raw.to_string(),
          loc: Range {
            start: text_start,
            end: self.state.position(),
          },
        });
        continue;
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
