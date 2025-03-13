//! 主解析模块，包含解析器的核心实现

use serde_json;

use crate::ast::{Attribute, AttributeValue, ExpressionPart, Location, Node, Position};
use crate::error::ParseError;
use crate::state::ParseState;

/// 解析结果类型
#[derive(Debug)]
pub struct ParserResult {
  /// 解析生成的AST
  pub ast: Node,
  /// 解析过程中发现的错误
  pub errors: Vec<ParseError>,
}

/// 主解析函数，解析WXML字符串并生成AST
pub fn parse(source: &str) -> ParserResult {
  let mut state = ParseState::new(source);
  let start_pos = state.get_position();

  // 解析文档
  let document = parse_document(&mut state);

  // 创建文档节点
  let ast = Node::Document {
    children: document,
    location: Location {
      start: start_pos,
      end: state.get_position(),
    },
  };

  ParserResult {
    ast,
    errors: state.errors,
  }
}

/// 将AST转换为JSON字符串
pub fn ast_to_json(ast: &Node) -> String {
  serde_json::to_string_pretty(ast).unwrap_or_else(|_| "{}".to_string())
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
      let pos = state.get_position();
      state.consume_while(|c| c != '>');
      if state.peek() == Some('>') {
        state.consume(); // 消费 '>'
      }
      // 返回空文本节点表示跳过的内容
      Node::Text {
        content: String::new(),
        location: Location {
          start: pos,
          end: state.get_position(),
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
  let start_pos = state.get_position();

  // 消费开始的 '<'
  state.consume();

  // 解析标签名
  let tag_name = state.consume_while(|c| !c.is_whitespace() && c != '>' && c != '/');

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
      message: format!("未正确关闭的标签: <{}", tag_name),
      position: start_pos,
    });
  }

  // 特殊处理 wxs 标签
  if tag_name.to_lowercase() == "wxs" && !is_self_closing {
    return parse_wxs_tag(state, attributes, start_pos);
  }

  // 如果是自闭合标签，没有子节点
  if is_self_closing {
    return Node::Element {
      tag_name,
      attributes,
      children: Vec::new(),
      is_self_closing: true,
      location: Location {
        start: start_pos,
        end: state.get_position(),
      },
    };
  }

  // 解析子节点
  let children = parse_element_children(state, &tag_name);

  Node::Element {
    tag_name,
    attributes,
    children,
    is_self_closing: false,
    location: Location {
      start: start_pos,
      end: state.get_position(),
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
        position: state.get_position(),
      });
      break;
    }

    // 检查是否是结束标签
    if state.peek_str("</") {
      let close_tag_start = state.position;

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
            line: state.line,
            column: state.column - close_tag_name.len() as u32,
          },
        });

        // 回溯到结束标签开始位置，将其作为文本处理
        state.position = close_tag_start;
        children.push(parse_text(state));
      }
    } else {
      children.push(parse_next_node(state));
    }
  }

  children
}

/// 解析wxs标签
fn parse_wxs_tag(state: &mut ParseState, attributes: Vec<Attribute>, start_pos: Position) -> Node {
  // 收集wxs标签内容直到</wxs>
  let content = state.consume_until("</wxs");

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
        position: state.get_position(),
      });
    }
  }

  Node::WxsScript {
    attributes,
    content,
    location: Location {
      start: start_pos,
      end: state.get_position(),
    },
  }
}

/// 解析文本节点
fn parse_text(state: &mut ParseState) -> Node {
  let start_pos = state.get_position();
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
      end: state.get_position(),
    },
  }
}

/// 解析双括号表达式
fn parse_expression(state: &mut ParseState) -> Node {
  let start_pos = state.get_position();

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
      end: state.get_position(),
    },
  }
}

/// 解析注释
fn parse_comment(state: &mut ParseState) -> Node {
  let start_pos = state.get_position();

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
      end: state.get_position(),
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
  let start_pos = state.get_position();

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

    parse_attribute_value(state)
  } else {
    None
  };

  Some(Attribute {
    name,
    value,
    location: Location {
      start: start_pos,
      end: state.get_position(),
    },
  })
}

/// 解析属性值，可能是静态字符串或表达式或混合内容
fn parse_attribute_value(state: &mut ParseState) -> Option<AttributeValue> {
  state.skip_whitespace();

  let start_pos = state.get_position();

  // 检查是否是引号包裹的属性值
  let quote = if state.peek() == Some('"') || state.peek() == Some('\'') {
    state.consume() // 消费开始的引号
  } else {
    // 记录错误：属性值必须用引号包裹
    state.record_error(ParseError::GeneralError {
      message: "属性值必须用引号包裹".to_string(),
      position: start_pos,
    });
    return None;
  };

  // 解析引号内的内容
  let (has_expression, parts) = parse_attribute_content(state, quote, start_pos);

  // 根据收集到的内容创建属性值
  if !has_expression && parts.len() == 1 {
    // 如果没有表达式且只有一个部分，则整个内容是静态的
    if let ExpressionPart::Static { content, location } = &parts[0] {
      return Some(AttributeValue::Static {
        content: content.clone(),
        location: location.clone(),
      });
    }
  }

  // 如果有表达式或多个部分，统一返回Expression类型
  Some(AttributeValue::Expression {
    parts,
    location: Location {
      start: start_pos,
      end: state.get_position(),
    },
  })
}

/// 解析属性内容（引号内）
fn parse_attribute_content(
  state: &mut ParseState,
  quote: Option<char>,
  start_pos: Position,
) -> (bool, Vec<ExpressionPart>) {
  // 表达式部分的列表
  let mut parts = Vec::new();
  // 当前静态文本内容
  let mut static_content = String::new();
  // 是否包含表达式
  let mut has_expression = false;

  // 在属性值中查找表达式和静态文本
  while !state.is_eof() {
    // 如果遇到结束引号
    if state.peek() == quote {
      state.consume(); // 消费结束的引号

      // 如果有剩余的静态内容，添加到parts中
      if !static_content.is_empty() {
        parts.push(ExpressionPart::Static {
          content: static_content.clone(),
          location: Location {
            start: start_pos, // 简化位置处理
            end: state.get_position(),
          },
        });
      }

      break;
    }

    // 如果遇到表达式开始
    if state.peek_str("{{") {
      has_expression = true;

      // 如果当前有静态内容，先添加到parts中
      if !static_content.is_empty() {
        parts.push(ExpressionPart::Static {
          content: static_content,
          location: Location {
            start: start_pos, // 简化位置处理
            end: state.get_position(),
          },
        });
        static_content = String::new();
      }

      // 解析表达式部分
      let expr_part = parse_expression_part(state);
      parts.push(expr_part);
    } else {
      // 处理静态内容
      if let Some(c) = state.consume() {
        static_content.push(c);
      }
    }
  }

  (has_expression, parts)
}

/// 解析表达式部分（用于属性值中的表达式）
fn parse_expression_part(state: &mut ParseState) -> ExpressionPart {
  let expr_start_pos = state.get_position();

  // 消费 '{{'
  state.consume_n(2);

  let mut expression_content = String::new();
  let mut brace_count = 1; // 追踪嵌套的花括号

  // 解析表达式内容
  while !state.is_eof() {
    if state.peek_str("{{") {
      expression_content.push_str("{{");
      state.consume_n(2);
      brace_count += 1;
    } else if state.peek_str("}}") {
      brace_count -= 1;
      if brace_count == 0 {
        state.consume_n(2); // 消费 '}}'
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
      position: expr_start_pos,
    });
  }

  ExpressionPart::Expression {
    content: expression_content.trim().to_string(),
    location: Location {
      start: expr_start_pos,
      end: state.get_position(),
    },
  }
}
