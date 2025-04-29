use std::{iter::Peekable, str::CharIndices};

use crate::{
  ast::Position,
  error::{SyntaxError, SyntaxErrorKind},
};

/// 解析过程中的状态信息
#[derive(Clone)]
pub struct ParseState<'s> {
  /// 源码引用
  source: &'s str,
  /// 源码字符迭代器
  chars: Peekable<CharIndices<'s>>,
  /// 当前字符偏移量
  offset: usize,
  /// 当前行号
  line: usize,
  /// 当前列号
  column: usize,
  /// 解析过程中收集的语法错误
  errors: Vec<SyntaxError>,
}

impl<'s> ParseState<'s> {
  /// 创建新的解析状态
  pub fn new(source: &'s str) -> Self {
    Self {
      source,
      chars: source.char_indices().peekable(),
      offset: 0,
      line: 1,
      column: 1,
      errors: Vec::new(),
    }
  }
}
