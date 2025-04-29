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

  /// 获取当前位置信息
  pub fn current_position(&self) -> Position {
    Position {
      offset: self.offset as u32,
      line: self.line as u32,
      column: self.column as u32,
    }
  }

  /// 查看下一个字符，但不消费它
  pub fn peek(&mut self) -> Option<(usize, char)> {
    self.chars.peek().copied()
  }

  /// 消费下一个字符并返回
  pub fn next(&mut self) -> Option<(usize, char)> {
    let result = self.chars.next();

    if let Some((offset, ch)) = result {
      self.offset = offset;

      // 更新行列信息
      if ch == '\n' {
        self.line += 1;
        self.column = 1;
      } else {
        self.column += 1;
      }
    }

    result
  }

  /// 消费字符直到满足条件
  pub fn consume_until<F>(&mut self, predicate: F) -> String
  where
    F: Fn(char) -> bool,
  {
    let mut result = String::new();

    while let Some((_, c)) = self.peek() {
      if predicate(c) {
        break;
      }

      if let Some((_, c)) = self.next() {
        result.push(c);
      }
    }

    result
  }

  /// 消费字符直到遇到指定字符序列
  pub fn consume_until_sequence(&mut self, sequence: &str) -> String {
    if sequence.is_empty() {
      return String::new();
    }

    let mut result = String::new();
    let first_char = sequence.chars().next().unwrap();
    let sequence_len = sequence.len();

    'outer: while let Some((_, c)) = self.peek() {
      if c == first_char {
        // 可能找到序列的开始，需要向前查看
        let start_offset = self.offset;
        let start_line = self.line;
        let start_column = self.column;

        // 先消费第一个字符
        self.next();

        // 检查是否匹配整个序列
        let mut matched = true;
        let mut temp = String::new();
        temp.push(c);

        for expected_char in sequence.chars().skip(1) {
          if let Some((_, actual_char)) = self.next() {
            temp.push(actual_char);
            if actual_char != expected_char {
              matched = false;
              break;
            }
          } else {
            matched = false;
            break;
          }
        }

        if matched {
          // 找到完整序列，返回结果
          return result;
        } else {
          // 没找到完整序列，回退并将部分匹配添加到结果
          result.push_str(&temp);

          // 重置状态（简化处理，直接从当前位置继续）
          continue 'outer;
        }
      }

      // 没遇到序列的第一个字符，继续消费
      if let Some((_, c)) = self.next() {
        result.push(c);
      }
    }

    result
  }

  /// 跳过空白字符
  pub fn skip_whitespace(&mut self) {
    while let Some((_, c)) = self.peek() {
      if !c.is_whitespace() {
        break;
      }
      self.next();
    }
  }

  /// 判断是否可以匹配指定的字符，如果能则消费它
  pub fn eat(&mut self, c: char) -> bool {
    if let Some((_, next)) = self.peek() {
      if next == c {
        self.next();
        return true;
      }
    }
    false
  }

  /// 判断是否可以匹配指定的字符串，如果能则消费它
  pub fn eat_string(&mut self, s: &str) -> bool {
    if s.is_empty() {
      return true;
    }

    // 保存当前状态以便回退
    let current_offset = self.offset;
    let current_line = self.line;
    let current_column = self.column;
    let chars_clone = self.chars.clone();

    for expected in s.chars() {
      if let Some((_, c)) = self.next() {
        if c != expected {
          // 不匹配，恢复状态
          self.offset = current_offset;
          self.line = current_line;
          self.column = current_column;
          self.chars = chars_clone;
          return false;
        }
      } else {
        // 到达输入末尾，恢复状态
        self.offset = current_offset;
        self.line = current_line;
        self.column = current_column;
        self.chars = chars_clone;
        return false;
      }
    }

    true
  }

  /// 记录语法错误
  pub fn record_error(&mut self, kind: SyntaxErrorKind) -> SyntaxError {
    let position = self.current_position();
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

  /// 检查是否到达输入末尾
  pub fn is_eof(&mut self) -> bool {
    self.peek().is_none()
  }
}
