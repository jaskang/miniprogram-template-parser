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
  /// 当前 char 的索引
  index: usize,
  /// 当前 chars 的偏移量
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
      index: 0,
      offset: 0,
      line: 1,
      column: 1,
      errors: Vec::new(),
    }
  }

  /// 记录语法错误
  pub fn add_error(&mut self, kind: SyntaxErrorKind) -> SyntaxError {
    let position = self.position();
    let error = SyntaxError {
      kind,
      offset: position.offset,
      line: position.line,
      column: position.column,
    };
    self.errors.push(error.clone());
    error
  }

  /// 获取所有收集到的错误
  pub fn errors(&self) -> &[SyntaxError] {
    &self.errors
  }

  /// 获取当前位置信息
  pub fn position(&self) -> Position {
    Position {
      offset: self.offset as u32,
      line: self.line as u32,
      column: self.column as u32,
    }
  }

  pub fn current_str(&self) -> &'s str {
    &self.source[self.index..]
  }

  /// 检查是否到达输入末尾
  pub fn is_eof(&mut self) -> bool {
    self.peek().is_none()
  }

  /// 查看下一个字符，但不消费它
  pub fn peek(&mut self) -> Option<char> {
    self.chars.peek().map(|(_, c)| *c)
  }

  // 查看接下来N个字符，但不消费它们
  pub fn peek_n<const N: usize>(&mut self) -> Option<[char; N]> {
    let mut chars = self.chars.clone();
    let mut result = ['\x00'; N];
    for i in 0..N {
      result[i] = chars.next()?.1;
    }
    Some(result)
  }

  pub fn starts_with(&mut self, s: &str) -> bool {
    self.current_str().starts_with(s)
  }

  /// 消费下一个字符并返回
  pub fn next(&mut self) -> Option<(usize, char)> {
    match self.chars.next() {
      Some((offset, ch)) => {
        self.offset = offset + 1;
        self.index += ch.len_utf8();
        if ch == '\n' {
          self.line += 1;
          self.column = 1;
        } else {
          self.column += 1;
        }
        // println!(
        //   "next: {} {} {} {} {} str: {}",
        //   ch,
        //   self.index,
        //   self.offset,
        //   self.line,
        //   self.column,
        //   self.current_str()
        // );
        Some((offset, ch))
      }
      None => None,
    }
  }

  /// 判断是否可以匹配指定的字符，如果能则消费它
  pub fn next_if(&mut self, c: char) -> bool {
    if let Some(ch) = self.peek() {
      if ch == c {
        self.next();
        return true;
      }
    }
    false
  }

  /// 消费字符直到不满足条件
  pub fn consume_while<F>(&mut self, predicate: F) -> &'s str
  where
    F: Fn(char) -> bool,
  {
    let start = self.index;
    loop {
      if let Some(ch) = self.peek() {
        if predicate(ch) {
          self.next();
        } else {
          break;
        }
      } else {
        break;
      }
    }
    &self.source[start..self.index]
  }

  /// 消费字符直到遇到目标字符串
  pub fn consume_until(&mut self, targets: Vec<&str>) -> &'s str {
    let start = self.index;
    loop {
      let current_str = self.current_str();
      if targets.iter().any(|target| current_str.starts_with(target)) {
        break;
      }
      self.next();
    }
    &self.source[start..self.index]
  }

  /// 跳过空白字符
  pub fn skip_whitespace(&mut self) {
    while let Some(c) = self.peek() {
      if !c.is_whitespace() {
        break;
      }
      self.next();
    }
  }
}
