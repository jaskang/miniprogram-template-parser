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
  source: &'s str,
  state: ParseState<'s>,
}

impl<'s> Parser<'s> {
  /// 创建一个新的解析器实例
  pub fn new(source: &'s str) -> Self {
    Self {
      source,
      state: ParseState::new(source),
    }
  }

  /// 解析WXML模板并生成抽象语法树
  pub fn parse(&mut self) -> Root {
    // 记录开始位置
    let start = self.state.position();

    // 解析所有子节点
    let children = self.parse_nodes();

    // 获取结束位置
    let end = self.state.position();

    // 生成根节点
    Root {
      children,
      start,
      end,
    }
  }

  /// 解析一系列节点，直到遇到结束标签或文件结束
  fn parse_nodes(&mut self) -> Vec<Node> {
    let mut nodes = Vec::new();

    while !self.state.is_eof() {
      match self.state.peek_n() {
        Some(['<', '/']) => {
          if self.is_closing_tag() {
            break;
          }
        }
        Some(['<', '!']) => {
          if self.is_comment() {
            break;
          }
        }
        Some(['<', _]) => {
          if self.parse_node() {
            break;
          }
        }
        Some(['{', '{']) => {
          if self.is_expression() {
            break;
          }
        }
        _ => {

        }
      }
      // 检查是否遇到结束标签
      if let Some([_, '<']) = self.state.peek_n() {
        let offset = self.state.position().offset;

        if self.is_closing_tag() {
          break;
        }

        // 尝试解析节点
        match self.parse_node() {
          Ok(node) => nodes.push(node),
          Err(e) => {
            // 解析失败，跳过当前字符，继续尝试解析后续内容
            if self.state.current_position().offset == offset as usize {
              self.state.next();
            }
          }
        }
      } else {
        // 尝试解析节点
        match self.parse_node() {
          Ok(node) => nodes.push(node),
          Err(e) => {
            // 解析失败，跳过当前字符，继续尝试解析后续内容
            self.state.next();
          }
        }
      }
    }

    nodes
  }

  /// 检查当前位置是否是一个结束标签 </xxx>
  fn is_closing_tag(&mut self) -> bool {
    // 保存当前状态
    let current_state = self.state.clone();

    // 检查是否是结束标签
    let is_closing = self.state.eat('<') && self.state.eat('/');

    // 恢复状态
    self.state = current_state;

    is_closing
  }

  /// 解析单个节点
  fn parse_node(&mut self) -> PResult<Node> {
    self.state.skip_whitespace();

    // 根据下一个字符决定如何解析
    match self.state.peek_n() {
      Some(['<', ch]) => {
        match ch {
          '!' => {
            if let Some(['<','!','-', '-']) = self.state.peek_n() {
              return self.parse_comment();
            } else {
              return Err(self.state.add_error(SyntaxErrorKind::ExpectComment));
            }
          }
          '/' => {
            // 结束标签，不应该在这里处理
            return Err(self.state.add_error(SyntaxErrorKind::ExpectCloseTag));
          }
          _ => {
            if is_tag_name_char(ch) {
              // 正常的开始标签
              return self.parse_element();
            } else {
              return Err(self.state.add_error(SyntaxErrorKind::ExpectElement));
            }
          }
        }
      }
      Some(['{', '{']) => {
        // 可能是表达式 {{ ... }}
        return self.parse_expression();
      }
      Some(_) => {
        // 普通文本节点
        return self.parse_text();
      }
      None => {
        // 到达文件尾部
        return Err(self.state.add_error(SyntaxErrorKind::ExpectTextNode));
      }
    }
  }

  /// 解析元素节点
  fn parse_element(&mut self) -> PResult<Node> {
    let start = self.state.position();

    // 消费开始标签 <
    self.state.next();

    // 解析标签名
    let name = self.parse_tag_name()?;

    // 解析属性
    let (attrs, first_attr_same_line) = self.parse_attributes()?;

    // 检查是否是自闭合标签
    let self_closing = self.check_self_closing();

    let mut children = Vec::new();

    if !self_closing {
      // 消费结束 >
      if !self.state.eat('>') {
        return Err(self.state.record_error(SyntaxErrorKind::ExpectElement));
      }

      // 解析子节点
      children = self.parse_nodes();

      // 解析结束标签
      self.parse_closing_tag(&name)?;
    }

    // 获取结束位置
    let end = self.state.current_position();

    Ok(Node::Element {
      name,
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
    let name = self.state.consume_while(|c| is_tag_name_char(c));

    if name.is_empty() {
      return Err(self.state.add_error(SyntaxErrorKind::ExpectTagName));
    }

    Ok(name)
  }

  /// 解析属性列表
  fn parse_attributes(&mut self) -> PResult<(Vec<Attribute>, bool)> {
    let mut attrs = Vec::new();
    let start_line = self.state.position().line;
    let mut first_attr_same_line = false;

    // 跳过空格
    self.state.skip_whitespace();

    // 检查第一个属性是否在同一行
    if let Some(attr) = self.parse_attribute().ok() {
      first_attr_same_line = attr.start.line == start_line;
      attrs.push(attr);
    }

    // 解析剩余属性
    loop {
      self.state.skip_whitespace();

      // 检查是否到达标签结束
      match self.state.peek() {
        Some( '>') | Some('/') => break,
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

    Ok((attrs, first_attr_same_line))
  }

  /// 解析单个属性
  fn parse_attribute(&mut self) -> PResult<Attribute> {
    let start = self.state.position();

    // 解析属性名
    let name = self.state.consume_while(|c| is_attr_name_char(c));

    if name.is_empty() {
      return Err(self.state.add_error(SyntaxErrorKind::ExpectAttrName));
    }

    // 跳过空格
    self.state.skip_whitespace();

    // 检查是否有属性值
    let value = if self.state.next_if('=') {
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
    let start = self.state.position();
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
      let mut value_start = self.state.position();
      self.state.next();

      while let Some(c) = self.state.peek() {
        if c == quote {
          self.state.next();
          break;
        } else if self.state.peek_n() == Some(['{', '{']) {
          self.state.consume_while(|c| c != '}');
        } else{

        }
      }
    } else {
      // 没有引号，解析到下一个空格或标签结束
      let content = self
        .state
        .consume_until(|c| c.is_whitespace() || c == '>' || c == '/');
      if content.is_empty() {
        return Err(self.state.record_error(SyntaxErrorKind::ExpectAttrValue));
      }

      // 检查是否包含表达式
      // TODO: 处理不带引号的属性值中的表达式，这种情况比较复杂，暂不实现
      let start = self.state.current_position();
      let end = self.state.current_position();
      values.push(AttributeValue::Text {
        content,
        start,
        end,
      });
    }

    if values.is_empty() {
      return Err(self.state.record_error(SyntaxErrorKind::ExpectAttrValue));
    }

    Ok(values)
  }

  /// 检查是否为自闭合标签
  fn check_self_closing(&mut self) -> bool {
    self.state.skip_whitespace();

    if self.state.eat('/') {
      // 消费 >
      if self.state.eat('>') {
        return true;
      }
      // 缺少 >，报错
      self.state.record_error(SyntaxErrorKind::ExpectSelfCloseTag);
    }

    false
  }

  /// 解析结束标签 </tagName>
  fn parse_closing_tag(&mut self, expected_name: &str) -> PResult<()> {
    // 查找和解析结束标签
    loop {
      if self.state.is_eof() {
        return Err(self.state.record_error(SyntaxErrorKind::ExpectCloseTag));
      }

      if self.state.eat('<') && self.state.eat('/') {
        // 找到结束标签的开始
        break;
      }

      self.state.next();
    }

    // 解析标签名
    let name = self.parse_tag_name()?;

    // 检查标签名是否匹配
    if name != expected_name {
      return Err(self.state.record_error(SyntaxErrorKind::ExpectCloseTag));
    }

    // 跳过空格
    self.state.skip_whitespace();

    // 检查结束标签是否正确关闭
    if !self.state.eat('>') {
      return Err(self.state.record_error(SyntaxErrorKind::ExpectCloseTag));
    }

    Ok(())
  }

  /// 解析文本节点
  fn parse_text(&mut self) -> PResult<Node> {
    let start = self.state.current_position();
    let mut content = String::new();

    while let Some((_, c)) = self.state.peek() {
      // 如果遇到 < 或 {{，停止解析文本
      if c == '<'
        || (c == '{' && {
          let mut state_clone = self.state.clone();
          state_clone.next();
          state_clone.peek().map_or(false, |(_, c)| c == '{')
        })
      {
        break;
      }

      // 否则，添加到文本内容
      if let Some((_, c)) = self.state.next() {
        content.push(c);
      }
    }

    // 如果文本内容为空，返回错误
    if content.is_empty() {
      return Err(self.state.record_error(SyntaxErrorKind::ExpectTextNode));
    }

    let end = self.state.current_position();

    Ok(Node::Text {
      content,
      start,
      end,
    })
  }

  /// 解析注释节点 <!-- ... -->
  fn parse_comment(&mut self) -> PResult<Node> {
    let start = self.state.current_position();

    // 解析注释内容，直到找到 -->
    let mut content = String::new();

    while let Some((_, c)) = self.state.peek() {
      // 检查是否是 -->
      if c == '-' {
        let mut state_clone = self.state.clone();
        state_clone.next(); // 消费 '-'
        if let Some((_, '-')) = state_clone.peek() {
          state_clone.next(); // 消费 '-'
          if let Some((_, '>')) = state_clone.peek() {
            // 找到注释结束
            self.state.next(); // 消费 '-'
            self.state.next(); // 消费 '-'
            self.state.next(); // 消费 '>'
            break;
          }
        }
      }

      // 不是注释结束，继续添加字符到内容
      if let Some((_, c)) = self.state.next() {
        content.push(c);
      } else {
        // 到达文件尾部，注释没有正确闭合
        return Err(self.state.record_error(SyntaxErrorKind::ExpectComment));
      }
    }

    let end = self.state.current_position();

    Ok(Node::Comment {
      content,
      start,
      end,
    })
  }

  /// 解析表达式节点 {{ ... }}
  fn parse_expression(&mut self) -> PResult<Node> {
    let start = self.state.current_position();

    // 消费 {{
    if !self.state.eat('{') || !self.state.eat('{') {
      return Err(self.state.record_error(SyntaxErrorKind::ExpectExpression));
    }

    // 跳过表达式开始处的空白
    self.state.skip_whitespace();

    // 解析表达式内容
    let mut content = String::new();
    let expression_start = self.state.current_position();

    while let Some((_, c)) = self.state.peek() {
      // 检查是否是 }}
      if c == '}' {
        let mut state_clone = self.state.clone();
        state_clone.next(); // 消费 '}'
        if let Some((_, '}')) = state_clone.peek() {
          // 找到表达式结束
          self.state.next(); // 消费 '}'
          self.state.next(); // 消费 '}'
          break;
        }
      }

      // 不是表达式结束，继续添加字符到内容
      if let Some((_, c)) = self.state.next() {
        content.push(c);
      } else {
        // 到达文件尾部，表达式没有正确闭合
        return Err(self.state.record_error(SyntaxErrorKind::ExpectExpression));
      }
    }

    // 去除表达式结尾处的空白
    let content = content.trim().to_string();

    let end = self.state.current_position();

    Ok(Node::Expression {
      content,
      start,
      end,
    })
  }
}
