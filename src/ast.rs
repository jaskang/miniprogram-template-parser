//! 抽象语法树(AST)相关的数据结构

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

/// 定义位置信息，用于标记AST节点在源码中的位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

/// AST节点类型，代表WXML文档中的各种元素
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
  /// 文档根节点，包含所有顶层节点
  Document {
    /// 子节点列表
    children: Vec<Node>,
    /// 位置信息
    location: Location,
  },
  /// 元素节点，如 <view>, <button> 等
  Element {
    /// 标签名
    tag_name: String,
    /// 属性列表
    attributes: Vec<Attribute>,
    /// 子节点列表
    children: Vec<Node>,
    /// 是否是自闭合标签，如 <input />
    is_self_closing: bool,
    /// 位置信息
    location: Location,
  },
  /// 特殊的wxs脚本节点，用于定义模块
  WxsScript {
    /// 属性列表
    attributes: Vec<Attribute>,
    /// 脚本内容
    content: String,
    /// 位置信息
    location: Location,
  },
  /// 文本节点，包含纯文本内容
  Text {
    /// 文本内容
    content: String,
    /// 位置信息
    location: Location,
  },
  /// 表达式节点（双括号表达式），如 {{message}}
  Expression {
    /// 表达式内容，不包含外层的双括号
    content: String,
    /// 位置信息
    location: Location,
  },
  /// 注释节点，如 <!-- 注释 -->
  Comment {
    /// 注释内容，不包含 <!-- 和 -->
    content: String,
    /// 位置信息
    location: Location,
  },
}

/// 属性节点，表示元素上的属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
  /// 属性名
  pub name: String,
  /// 属性值，可能为空（如布尔属性）
  pub value: Option<AttributeValue>,
  /// 位置信息
  pub location: Location,
}

/// 属性值类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AttributeValue {
  /// 静态值（纯字符串），如 class="container"
  Static {
    /// 属性值内容
    content: String,
    /// 位置信息
    location: Location,
  },
  /// 动态值（包含表达式），如 class="item-{{index}}"
  Expression {
    /// 表达式部分列表
    parts: Vec<ExpressionPart>,
    /// 位置信息
    location: Location,
  },
}

/// 表达式组成部分
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExpressionPart {
  /// 静态文本部分
  Static {
    /// 静态文本内容
    content: String,
    /// 位置信息
    location: Location,
  },
  /// 表达式部分
  Expression {
    /// 表达式内容，不包含外层的双括号
    content: String,
    /// 位置信息
    location: Location,
  },
}
