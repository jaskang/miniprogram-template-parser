//! 主解析模块，包含解析器的核心实现

use std::f32::consts::E;

use crate::ast::{Attribute, Location, Node, Position, Root, Value};
use crate::error::ParseError;
use crate::state::ParseState;

/// 主解析函数，解析WXML字符串并生成AST
pub fn parse(source: &str) -> Root {
  let mut state = ParseState::new(source);
  let start_pos = state.position();

  // 解析文档
  let document = parse_document(&mut state);

  // 创建文档节点
  Root {
    children: document,
    location: Location {
      start: start_pos,
      end: state.position(),
    },
  }
}

/// 解析整个文档
fn parse_document(state: &mut ParseState) -> Vec<Node> {
  let mut nodes = Vec::new();

  // 解析直到文件结束
  while !state.is_eof() {
    state.skip_whitespace();

    if state.is_eof() {
      break;
    }

    nodes.push(parse_next_node(state));
  }

  nodes
}

/// 解析下一个节点
fn parse_next_node(state: &mut ParseState) -> Node {
  // 如果遇到注释开始标记
  if state.peek_str("<!--") {
    parse_comment(state)
  }
  // 如果遇到标签开始标记
  else if state.peek_str("<") {
    // 查看是否是结束标签
    if state.peek_str("</") {
      // 这里可能是无效的关闭标签，跳过它
      let pos = state.position();
      state.consume_while(|c| c != '>');
      if state.peek() == Some('>') {
        state.consume(); // 消费 '>'
      }
      // 返回空文本节点表示跳过的内容
      Node::Text {
        content: String::new(),
        location: Location {
          start: pos,
          end: state.position(),
        },
      }
    } else {
      parse_element(state)
    }
  }
  // 如果是表达式的开始
  else if state.peek_str("{{") {
    parse_expression(state)
  }
  // 否则，这是文本内容
  else {
    parse_text(state)
  }
}

/// 解析元素节点
fn parse_element(state: &mut ParseState) -> Node {
  let start_pos = state.position();
  let start_offset = state.offset;

  // 消费开始的 '<'
  state.consume();

  // 解析标签名
  let name = state.consume_while(|c| !c.is_whitespace() && c != '>' && c != '/');

  state.skip_whitespace();

  // 解析属性
  let attributes = parse_attributes(state);

  state.skip_whitespace();

  // 检查是否是自闭合标签
  let is_self_closing = state.peek() == Some('/');
  if is_self_closing {
    state.consume(); // 消费 '/'
  }

  // 消费结束的 '>'
  if state.peek() == Some('>') {
    state.consume();
  } else {
    // 标签未正确关闭，记录错误
    state.record_error(ParseError::GeneralError {
      message: format!("未正确关闭的标签: <{}", name),
      position: start_pos,
    });
  }

  // 捕获原始内容
  let content = state.pick_rang(start_offset, state.offset);

  // 特殊处理 wxs 标签
  if name.to_lowercase() == "wxs" && !is_self_closing {
    return parse_wxs_tag(state, attributes, start_pos, start_offset, content);
  }

  // 如果是自闭合标签，没有子节点
  if is_self_closing {
    return Node::Element {
      name,
      attributes,
      children: Vec::new(),
      is_self_closing: true,
      content,
      location: Location {
        start: start_pos,
        end: state.position(),
      },
    };
  }

  // 解析子节点
  let children = parse_element_children(state, &name);
  let end_offset = state.offset;
  let end_pos = state.position();

  // 更新内容以包含整个元素
  let full_content = state.pick_rang(start_offset, end_offset);

  Node::Element {
    name,
    attributes,
    children,
    is_self_closing: false,
    content: full_content,
    location: Location {
      start: start_pos,
      end: end_pos,
    },
  }
}

/// 解析元素的子节点
fn parse_element_children(state: &mut ParseState, parent_tag_name: &str) -> Vec<Node> {
  let mut children = Vec::new();

  // 直到遇到结束标签或文件结束
  loop {
    if state.is_eof() {
      // 文件结束但标签未关闭，记录错误
      state.record_error(ParseError::UnclosedElement {
        tag_name: parent_tag_name.to_string(),
        position: state.position(),
      });
      break;
    }

    // 检查是否是结束标签
    if state.peek_str("</") {
      let close_tag_start = state.offset;

      // 消费 '</'
      state.consume_n(2);

      // 获取结束标签名
      let close_tag_name = state.consume_while(|c| !c.is_whitespace() && c != '>');

      state.skip_whitespace();

      // 消费结束的 '>'
      if state.peek() == Some('>') {
        state.consume();
      }

      // 检查标签名是否匹配
      if close_tag_name == parent_tag_name {
        break;
      } else {
        // 标签名不匹配，记录错误
        state.record_error(ParseError::MismatchedTag {
          expected: parent_tag_name.to_string(),
          found: close_tag_name.clone(),
          position: Position {
            offset: state.offset,
            line: state.line,
            column: state.column - close_tag_name.len() as u32,
          },
        });

        // 回溯到结束标签开始位置，将其作为文本处理
        state.offset = close_tag_start;
        children.push(parse_text(state));
      }
    } else {
      children.push(parse_next_node(state));
    }
  }

  children
}

/// 解析wxs标签
fn parse_wxs_tag(
  state: &mut ParseState,
  attributes: Vec<Attribute>,
  start_pos: Position,
  start_offset: u32,
  tag_content: String,
) -> Node {
  // 收集wxs标签内容直到</wxs>
  let script_content = state.consume_until("</wxs");

  // 消费结束标签 </wxs>
  if state.peek_str("</wxs") {
    state.consume_n(5);
    state.skip_whitespace();
    if state.peek() == Some('>') {
      state.consume();
    } else {
      // wxs标签未正确关闭，记录错误
      state.record_error(ParseError::GeneralError {
        message: "wxs标签未正确关闭".to_string(),
        position: state.position(),
      });
    }
  }

  let full_content = state.pick_rang(start_offset, state.offset);

  // 创建一个文本节点来保存脚本内容
  let script_node = Node::Expression {
    content: script_content,
    // start: (start_offset + tag_content.len() as u32),
    // end: state.offset,
    // end: (state.offset - 6), // 减去 "</wxs>" 的长度
    location: Location {
      start: start_pos,
      end: state.position(),
    },
  };

  // 返回包含脚本内容的 wxs 元素节点
  Node::Element {
    name: "wxs".to_string(),
    attributes,
    children: vec![script_node],
    is_self_closing: false,
    content: full_content,
    location: Location {
      start: start_pos,
      end: state.position(),
    },
  }
}

/// 解析文本节点
fn parse_text(state: &mut ParseState) -> Node {
  let start_pos = state.position();
  let start_offset = state.offset;
  let mut text_content = String::new();

  // 收集文本内容直到遇到 <, {{ 或文件结束
  while !state.is_eof() {
    // 如果遇到元素、注释或表达式的开始
    if state.peek() == Some('<') || state.peek_str("{{") {
      break;
    }

    if let Some(c) = state.consume() {
      text_content.push(c);
    }
  }

  Node::Text {
    content: text_content,
    location: Location {
      start: start_pos,
      end: state.position(),
    },
  }
}

/// 解析双括号表达式
fn parse_expression(state: &mut ParseState) -> Node {
  let start_pos = state.position();
  let start_offset = state.offset;

  // 消费开始的 '{{'
  state.consume_n(2);

  let mut expression_content = String::new();
  let mut brace_count = 1; // 追踪嵌套的花括号

  // 收集表达式内容直到遇到匹配的 }} 或文件结束
  while !state.is_eof() {
    if state.peek_str("{{") {
      expression_content.push_str("{{");
      state.consume_n(2);
      brace_count += 1;
    } else if state.peek_str("}}") {
      brace_count -= 1;
      if brace_count == 0 {
        state.consume_n(2); // 消费结束的 '}}'
        break;
      } else {
        expression_content.push_str("}}");
        state.consume_n(2);
      }
    } else if let Some(c) = state.consume() {
      expression_content.push(c);
    }
  }

  // 如果表达式未关闭，记录错误
  if brace_count > 0 {
    state.record_error(ParseError::UnclosedExpression {
      position: start_pos,
    });
  }

  Node::Expression {
    content: expression_content.trim().to_string(),
    location: Location {
      start: start_pos,
      end: state.position(),
    },
  }
}

/// 解析注释
fn parse_comment(state: &mut ParseState) -> Node {
  let start_pos = state.position();
  let start_offset = state.offset;

  // 消费开始的 '<!--'
  state.consume_n(4);

  // 收集注释内容直到遇到 --> 或文件结束
  let comment_content = state.consume_until("-->");

  // 消费结束的 '-->'
  if state.peek_str("-->") {
    state.consume_n(3);
  } else {
    // 注释未关闭，记录错误
    state.record_error(ParseError::GeneralError {
      message: "注释未关闭".to_string(),
      position: start_pos,
    });
  }

  Node::Comment {
    content: comment_content,
    location: Location {
      start: start_pos,
      end: state.position(),
    },
  }
}

/// 解析属性列表
fn parse_attributes(state: &mut ParseState) -> Vec<Attribute> {
  let mut attributes = Vec::new();

  while !state.is_eof() {
    state.skip_whitespace();

    // 如果遇到标签结束或自闭合标签
    if state.peek() == Some('>') || state.peek() == Some('/') {
      break;
    }

    // 解析单个属性
    if let Some(attribute) = parse_attribute(state) {
      attributes.push(attribute);
    } else {
      // 属性解析失败，跳过至下一个空白字符
      state.consume_while(|c| !c.is_whitespace() && c != '>' && c != '/');
    }
  }

  attributes
}

/// 解析单个属性
fn parse_attribute(state: &mut ParseState) -> Option<Attribute> {
  let start_pos = state.position();

  // 解析属性名
  let name = state.consume_while(|c| !c.is_whitespace() && c != '=' && c != '>' && c != '/');

  if name.is_empty() {
    return None;
  }

  state.skip_whitespace();

  // 检查是否有属性值
  let value = if state.peek() == Some('=') {
    state.consume(); // 消费 '='
    state.skip_whitespace();

    Some(parse_attribute_value(state))
  } else {
    None
  };

  Some(Attribute {
    name,
    value,
    location: Location {
      start: start_pos,
      end: state.position(),
    },
  })
}

/// 解析属性值，可能是静态字符串或表达式或混合内容
fn parse_attribute_value(state: &mut ParseState) -> Vec<Value> {
  state.skip_whitespace();

  let mut values = Vec::new();
  let start_pos = state.position();
  let start_offset = state.offset;

  // 检查是否是引号包裹的属性值
  let quote = if state.peek() == Some('"') || state.peek() == Some('\'') {
    state.consume() // 消费开始的引号
  } else {
    // 记录错误：属性值必须用引号包裹
    state.record_error(ParseError::GeneralError {
      message: "属性值必须用引号包裹".to_string(),
      position: start_pos,
    });
    return values;
  };

  // 解析引号内的所有内容
  parse_attribute_content(state, quote, start_pos, &mut values);

  values
}

/// 解析属性内容（引号内）
fn parse_attribute_content(
  state: &mut ParseState,
  quote: Option<char>,
  start_pos: Position,
  values: &mut Vec<Value>,
) {
  // 当前静态内容
  let mut static_content = String::new();
  let mut current_start = state.offset;

  // 在属性值中查找表达式和静态文本
  while !state.is_eof() {
    // 如果遇到结束引号
    if state.peek() == quote {
      // 处理剩余的静态内容
      if !static_content.is_empty() {
        values.push(Value::Text {
          content: static_content,
          location: Location {
            start: start_pos, // 简化处理，使用属性开始位置
            end: state.position(),
          },
        });
      }

      state.consume(); // 消费结束的引号
      break;
    }

    // 如果遇到表达式开始
    if state.peek_str("{{") {
      // 先处理前面积累的静态内容
      if !static_content.is_empty() {
        values.push(Value::Text {
          content: static_content,
          location: Location {
            start: start_pos, // 简化处理，使用属性开始位置
            end: state.position(),
          },
        });
        static_content = String::new();
      }

      // 记录表达式开始位置
      let expr_start_pos = state.position();
      let expr_start_offset = state.offset;

      // 消费开始的 '{{'
      state.consume_n(2);

      // 解析表达式内容
      let mut expr_content = String::new();
      let mut brace_count = 1;

      while !state.is_eof() {
        if state.peek_str("{{") {
          expr_content.push_str("{{");
          state.consume_n(2);
          brace_count += 1;
        } else if state.peek_str("}}") {
          brace_count -= 1;
          if brace_count == 0 {
            state.consume_n(2); // 消费结束的 '}}'
            break;
          } else {
            expr_content.push_str("}}");
            state.consume_n(2);
          }
        } else if let Some(c) = state.consume() {
          expr_content.push(c);
        }
      }

      // 添加表达式部分到 values
      let full_expr = format!("{{{{{}}}}}", expr_content.trim());
      values.push(Value::Expression {
        content: full_expr,
        location: Location {
          start: expr_start_pos,
          end: state.position(),
        },
      });

      // 重置静态内容起始点
      current_start = state.offset;
    } else {
      // 处理普通字符
      if let Some(c) = state.consume() {
        static_content.push(c);
      }
    }
  }
}
