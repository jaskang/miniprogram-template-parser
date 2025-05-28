use std::vec;

use crate::{
  ast::*,
  error::{SyntaxError, SyntaxErrorKind},
  helpers::*,
  state::ParseState,
};

pub type PResult<T> = Result<T, SyntaxError>;

/// Parser结构体表示模板解析器的状态
///
/// 字段说明：
/// * `source` - 待解析的源代码字符串
/// * `state` - 解析状态，包含字符迭代器和位置信息
pub struct Parser<'s> {
  state: ParseState<'s>,
}

impl<'s> Parser<'s> {
  /// 创建一个新的解析器实例
  pub fn new(source: &'s str) -> Self {
    Self {
      state: ParseState::new(source),
    }
  }

  pub fn parse_root(&mut self) -> PResult<Root> {
    let start = self.state.position();
    let children = self.parse_children(None)?;
    let end = self.state.position();
    Ok(Root {
      children,
      start,
      end,
    })
  }

  /// 解析一系列节点，直到遇到结束标签或文件结束
  fn parse_children(&mut self, parent_name: Option<&str>) -> PResult<Vec<Node>> {
    let mut children = vec![];
    while !self.state.is_end() {
      if self.state.starts_with("</") {
        if let Some(name) = parent_name {
          if self.state.starts_with(name) {
            self.state.next_n(name.len());
            break;
          } else {
            return Err(self.state.emit_error(SyntaxErrorKind::ExpectCloseTag));
          }
        }
        break;
      }
      children.push(self.parse_node()?);
    }
    Ok(children)
  }

  /// 解析单个节点
  fn parse_node(&mut self) -> PResult<Node> {
    self.state.skip_whitespace();

    // 根据下一个字符决定如何解析
    match self.state.peek_n() {
      // 注释 <!-- ... -->
      Some(['<', '!']) => {
        if let Some(['<', '!', '-', '-']) = self.state.peek_n() {
          return self.parse_comment();
        } else {
          return Err(self.state.emit_error(SyntaxErrorKind::ExpectComment));
        }
      }
      // 开始标签 <tagName
      Some(['<', ch]) => {
        if is_tag_name_char(ch) {
          // 正常的开始标签
          return self.parse_element();
        } else if ch == '/' {
          // 错误的结束标签
          return Err(self.state.emit_error(SyntaxErrorKind::ExpectElement));
        } else {
          // 错误的标签名
          return Err(self.state.emit_error(SyntaxErrorKind::ExpectElement));
        }
      }
      // 表达式 {{ ... }}
      Some(['{', '{']) => {
        return self.parse_expression_node();
      }
      // 普通文本节点
      Some(_) => {
        return self.parse_text();
      }
      None => {
        // 到达文件尾部
        return Err(self.state.emit_error(SyntaxErrorKind::ExpectTextNode));
      }
    }
  }

  /// 解析元素节点
  fn parse_element(&mut self) -> PResult<Node> {
    let start = self.state.position();
    // 消费 "<"
    self.state.next();

    // 解析标签名
    let name = self.parse_tag_name()?;

    // 解析属性
    let (attrs, first_attr_same_line) = self.parse_attributes()?;

    self.state.skip_whitespace();
    // 检查是否是自闭合标签
    let self_closing = self.state.next_if(|c, _| c == '/');

    let mut children = Vec::new();

    if !self_closing {
      // 消费结束 >
      if !self.state.next_if(|c, _| c == '>') {
        return Err(self.state.emit_error(SyntaxErrorKind::ExpectElement));
      }

      // 解析子节点
      children = self.parse_children(Some(name))?;

      self.state.skip_whitespace();
      // 解析结束标签
      self.parse_closing_tag(&name)?;
    }

    // 获取结束位置
    let end = self.state.position();

    Ok(Node::Element {
      name: name.to_string(),
      attrs,
      children,
      self_closing,
      first_attr_same_line,
      start,
      end,
    })
  }

  /// 解析标签名
  fn parse_tag_name(&mut self) -> PResult<&'s str> {
    let name = self.state.next_while(|c, _| is_tag_name_char(c));
    if name.is_empty() {
      return Err(self.state.emit_error(SyntaxErrorKind::ExpectTagName));
    }
    Ok(name)
  }

  /// 解析属性列表
  fn parse_attributes(&mut self) -> PResult<(Vec<Attribute>, bool)> {
    let mut attrs = Vec::new();
    let start = self.state.position();
    let mut first_attr_same_line = true;

    // 解析剩余属性
    loop {
      self.state.skip_whitespace();

      // 检查是否到达标签结束
      match self.state.peek() {
        Some('>') | Some('/') => break,
        None => break,
        _ => {
          // 尝试解析下一个属性
          match self.parse_attribute() {
            Ok(attr) => attrs.push(attr),
            Err(_) => {
              // 属性解析错误，跳过这个字符
              self.state.next();
            }
          }
        }
      }
    }
    // 检查第一个属性是否在同一行
    first_attr_same_line = if let Some(attr) = attrs.first() {
      attr.start.line == start.line
    } else {
      true
    };

    Ok((attrs, first_attr_same_line))
  }

  /// 解析单个属性
  fn parse_attribute(&mut self) -> PResult<Attribute> {
    let start = self.state.position();

    // 解析属性名
    let name = self.state.next_while(|c, _| is_attr_name_char(c));

    if name.is_empty() {
      return Err(self.state.emit_error(SyntaxErrorKind::ExpectAttrName));
    }
    // 检查是否有属性值
    let value = if self.state.next_if(|c, _| c == '=') {
      Some(self.parse_attribute_value()?)
    } else {
      None
    };
    let end = self.state.position();
    Ok(Attribute {
      name: name.to_string(),
      value,
      start,
      end,
    })
  }

  /// 解析属性值
  fn parse_attribute_value(&mut self) -> PResult<Vec<AttributeValue>> {
    let quote = match self.state.peek() {
      Some('"') | Some('\'') => {
        let (_, q) = self.state.next().unwrap();
        Some(q)
      }
      _ => None,
    };

    let mut values = Vec::new();
    // 如果有引号，解析引号内的内容
    if let Some(quote) = quote {
      loop {
        if self.state.next_if(|c, _| c == quote) {
          break;
        }
        match self.state.peek_n() {
          Some(['{', '{']) => {
            let exp = self.parse_expression();
            match exp {
              Ok(exp) => {
                values.push(AttributeValue::Expression {
                  content: exp.content,
                  start: exp.start,
                  end: exp.end,
                });
              }
              Err(e) => {
                return Err(e);
              }
            }
          }
          _ => {
            let start = self.state.position();
            let text = self
              .state
              .next_until(|c, s| c == quote || s.starts_with("{{"));
            let end = self.state.position();
            values.push(AttributeValue::Text {
              content: text.to_string(),
              start,
              end,
            });
          }
        }
      }
    } else {
      return Err(self.state.emit_error(SyntaxErrorKind::ExpectAttrValue));
    }

    if values.is_empty() {
      let pos = self.state.position();
      values.push(AttributeValue::Text {
        content: "".to_string(),
        start: pos,
        end: pos,
      });
    }

    Ok(values)
  }

  /// 解析结束标签 </tagName>
  fn parse_closing_tag(&mut self, expected_name: &str) -> PResult<()> {
    if self.state.starts_with("</") {
      self.state.next_n(2);
    } else {
      return Err(self.state.emit_error(SyntaxErrorKind::ExpectCloseTag));
    }
    // 解析标签名
    let name = self.parse_tag_name()?;

    // 检查标签名是否匹配
    if name != expected_name {
      return Err(self.state.emit_error(SyntaxErrorKind::ExpectCloseTag));
    }

    // 跳过空格
    self.state.skip_whitespace();

    // 检查结束标签是否正确关闭
    if !self.state.next_if(|c, _| c == '>') {
      return Err(self.state.emit_error(SyntaxErrorKind::ExpectCloseTag));
    }

    Ok(())
  }

  /// 解析文本节点
  fn parse_text(&mut self) -> PResult<Node> {
    let start = self.state.position();
    let str = self
      .state
      .next_until(|c, s| c == '<' || s.starts_with("{{"));
    let content = str.to_string();
    // 如果文本内容为空，返回错误
    if content.is_empty() {
      return Err(self.state.emit_error(SyntaxErrorKind::ExpectTextNode));
    }
    let end = self.state.position();
    Ok(Node::Text {
      content,
      start,
      end,
    })
  }

  /// 解析注释节点 <!-- ... -->
  fn parse_comment(&mut self) -> PResult<Node> {
    let start = self.state.position();
    // 消费 "<!--"
    self.state.next_n(4);
    let str = self.state.next_until(|_, s| s.starts_with("-->"));
    let content = str.to_string();
    // 如果文本内容为空，返回错误
    if content.is_empty() {
      return Err(self.state.emit_error(SyntaxErrorKind::ExpectTextNode));
    }
    // 消费 "-->"
    self.state.next_n(3);
    let end = self.state.position();
    Ok(Node::Comment {
      content,
      start,
      end,
    })
  }

  /// 解析表达式节点 {{ ... }}
  fn parse_expression(&mut self) -> PResult<Expression> {
    let start = self.state.position();
    // 消费 "{{"
    self.state.next_n(2);
    // 跳过表达式开始处的空白
    self.state.skip_whitespace();
    let str = self.state.next_until(|_, s| s.starts_with("}}"));
    let content = str.trim().to_string();
    // 消费 "}}"
    self.state.next_n(2);
    let end = self.state.position();

    Ok(Expression {
      content,
      start,
      end,
    })
  }

  fn parse_expression_node(&mut self) -> PResult<Node> {
    let expr = self.parse_expression()?;
    Ok(Node::Expression {
      content: expr.content,
      start: expr.start,
      end: expr.end,
    })
  }
}
