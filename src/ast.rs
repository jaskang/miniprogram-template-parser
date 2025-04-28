//! 抽象语法树(AST)相关的数据结构

use napi_derive::napi;
use std::fmt;

use crate::error::SyntaxError;

/// 定义位置信息，用于标记AST节点在源码中的位置

#[derive(Debug, Clone, Copy)]
#[napi(object)]
pub struct Position {
  /// chars 索引, 从 0 开始
  pub offset: u32,
  /// 行号，从1开始
  pub line: u32,
  /// 列号，从1开始
  pub column: u32,
}

#[derive(Debug, Clone)]
#[napi(object)]
pub struct Range {
  pub start: Position,
  pub end: Position,
}

impl fmt::Display for Position {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} {}:{}", self.offset, self.line, self.column)
  }
}

#[derive(Debug, Clone)]
#[napi]
pub enum Value {
  /// 静态值
  Text { content: String, position: Position },
  /// 动态值
  Expression { content: String, position: Position },
}

#[derive(Debug, Clone)]
#[napi(object)]
pub struct Root {
  pub children: Vec<Node>,
  pub loc: Range,
}

#[derive(Debug, Clone)]
#[napi(object)]
pub struct Attribute {
  pub name: String,
  pub value: Option<Vec<AttributeValue>>,
  pub loc: Range,
}

#[derive(Debug, Clone)]
#[napi]
pub enum AttributeValue {
  Text { content: String, loc: Range },
  Expression { content: String, loc: Range },
}

/// AST节点类型，代表WXML文档中的各种元素
#[derive(Debug, Clone)]
#[napi]
pub enum Node {
  /// 元素节点，如 <view>, <button> 等
  Element {
    name: String,
    attrs: Vec<Attribute>,
    children: Vec<Node>,
    self_closing: bool,
    first_attr_same_line: bool,
    loc: Range,
  },
  Text {
    content: String,
    loc: Range,
  },
  Comment {
    content: String,
    loc: Range,
  },
  Expression {
    content: String,
    loc: Range,
  },
}
