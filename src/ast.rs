//! 抽象语法树(AST)相关的数据结构

use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

/// 定义位置信息，用于标记AST节点在源码中的位置

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[napi(object)]
pub struct Position {
  /// 行号，从1开始
  pub line: u32,
  /// 列号，从1开始
  pub column: u32,
}

impl fmt::Display for Position {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}:{}", self.line, self.column)
  }
}

/// 定义AST节点的位置范围
#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct Location {
  /// 开始位置
  pub start: Position,
  /// 结束位置
  pub end: Position,
}

impl From<Range<Position>> for Location {
  fn from(range: Range<Position>) -> Self {
    Self {
      start: range.start,
      end: range.end,
    }
  }
}

/// 属性节点，表示元素上的属性
#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct Attribute {
  // 属性名
  pub name: String,
  // 属性值，可能为空（如布尔属性）
  // 静态值（纯字符串），如 class="container" value 为 [Static]
  // 动态值（包含表达式），如 class="{{index}}" value 为 [Expression]
  // 多个值，如 class="container {{index}} {{name}}" value 为 [Static, Expression, Expression, Static]
  pub value: Vec<AttributeValue>,
  // 位置信息
  pub location: Location,
}

/// 属性值类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[napi]
pub enum AttributeValue {
  Static {
    content: String,
    start: u32,
    end: u32,
    location: Location,
  },
  Expression {
    content: String,
    start: u32,
    end: u32,
    location: Location,
  },
}

/// AST节点类型，代表WXML文档中的各种元素
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[napi]
pub enum Node {
  Document {
    children: Vec<Node>,
    start: u32,
    end: u32,
    location: Location,
  },
  /// 元素节点，如 <view>, <button> 等
  Element {
    name: String,
    attributes: Vec<Attribute>,
    children: Vec<Node>,
    is_self_closing: bool,
    content: String,
    start: u32,
    end: u32,
    location: Location,
  },
  Text {
    content: String,
    start: u32,
    end: u32,
    location: Location,
  },
  /// 表达式节点（双括号表达式），如 {{message}}
  Expression {
    // 表达式内容，不包含外层的双括号
    content: String,
    start: u32,
    end: u32,
    location: Location,
  },
  /// 注释节点，如 <!-- 注释 -->
  Comment {
    // 注释内容，不包含 <!-- 和 -->
    content: String,
    start: u32,
    end: u32,
    location: Location,
  },
}
