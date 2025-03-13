//! 错误处理模块，定义解析过程中可能出现的错误类型

use crate::ast::Position;
use std::fmt;

/// 解析错误的枚举类型
#[derive(Debug, Clone)]
pub enum ParseError {
  /// 当源码提前结束时触发
  UnexpectedEOF {
    expected: String,
    position: Position,
  },
  /// 当遇到不匹配的标签时触发
  MismatchedTag {
    expected: String,
    found: String,
    position: Position,
  },
  /// 当属性格式不正确时触发
  InvalidAttribute { name: String, position: Position },
  /// 当一个元素未被正确闭合时触发
  UnclosedElement {
    tag_name: String,
    position: Position,
  },
  /// 当表达式未被正确闭合时触发
  UnclosedExpression { position: Position },
  /// 其他类型的错误
  GeneralError { message: String, position: Position },
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ParseError::UnexpectedEOF { expected, position } => {
        write!(
          f,
          "意外的文件结束，期望 {} 在位置 {}:{}",
          expected, position.line, position.column
        )
      }
      ParseError::MismatchedTag {
        expected,
        found,
        position,
      } => {
        write!(
          f,
          "标签不匹配，期望 </{}> 但找到 </{}> 在位置 {}:{}",
          expected, found, position.line, position.column
        )
      }
      ParseError::InvalidAttribute { name, position } => {
        write!(
          f,
          "无效的属性 '{}' 在位置 {}:{}",
          name, position.line, position.column
        )
      }
      ParseError::UnclosedElement { tag_name, position } => {
        write!(
          f,
          "未闭合的元素 '{}' 在位置 {}:{}",
          tag_name, position.line, position.column
        )
      }
      ParseError::UnclosedExpression { position } => {
        write!(
          f,
          "未闭合的表达式 '{{{{' 在位置 {}:{}",
          position.line, position.column
        )
      }
      ParseError::GeneralError { message, position } => {
        write!(
          f,
          "{} 在位置 {}:{}",
          message, position.line, position.column
        )
      }
    }
  }
}

impl std::error::Error for ParseError {}
