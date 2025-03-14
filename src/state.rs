//! 解析状态模块，包含用于追踪解析过程的状态和工具函数

use crate::ast::Position;
use crate::error::ParseError;

/// 解析状态，跟踪当前解析位置和上下文
pub struct ParseState<'a> {
  pub source: &'a str,
  pub chars: Vec<char>,
  pub offset: usize,
  pub line: u32,
  pub column: u32,
  pub errors: Vec<ParseError>,
}

impl<'a> ParseState<'a> {
  /// 创建新的解析状态
  pub fn new(source: &'a str) -> Self {
    Self {
      source,
      chars: source.chars().collect(),
      offset: 0,
      line: 1,
      column: 1,
      errors: Vec::new(),
    }
  }

  /// 记录错误
  pub fn record_error(&mut self, error: ParseError) {
    self.errors.push(error);
  }

  /// 获取已收集的错误
  pub fn get_errors(&self) -> &[ParseError] {
    &self.errors
  }

  /// 获取当前位置
  pub fn get_position(&self) -> Position {
    Position {
      line: self.line,
      column: self.column,
    }
  }

  /// 设置当前位置（用于回溯）
  pub fn set_position(&mut self, position: usize, line: u32, column: u32) {
    self.offset = position;
    self.line = line;
    self.column = column;
  }

  /// 保存当前状态（用于回溯）
  pub fn save_state(&self) -> (usize, u32, u32) {
    (self.offset, self.line, self.column)
  }

  /// 恢复状态（用于回溯）
  pub fn restore_state(&mut self, state: (usize, u32, u32)) {
    self.offset = state.0;
    self.line = state.1;
    self.column = state.2;
  }

  /// 检查是否已经到达源码结尾
  pub fn is_eof(&self) -> bool {
    self.offset >= self.chars.len()
  }

  /// 查看当前字符但不消费
  pub fn peek(&self) -> Option<char> {
    if self.is_eof() {
      None
    } else {
      Some(self.chars[self.offset])
    }
  }

  /// 查看接下来的n个字符但不消费
  pub fn peek_n(&self, n: usize) -> String {
    if self.is_eof() {
      return String::new();
    }

    let end = (self.offset + n).min(self.chars.len());
    self.chars[self.offset..end].iter().collect()
  }

  /// 查看接下来的字符是否匹配给定的字符串
  pub fn peek_str(&self, s: &str) -> bool {
    self.peek_n(s.len()) == s
  }

  /// 消费当前字符并前进
  pub fn consume(&mut self) -> Option<char> {
    if self.is_eof() {
      return None;
    }

    let c = self.chars[self.offset];
    self.offset += 1;

    // 更新行列信息
    if c == '\n' {
      self.line += 1;
      self.column = 1;
    } else {
      self.column += 1;
    }

    Some(c)
  }

  /// 消费指定数量的字符
  pub fn consume_n(&mut self, n: usize) -> String {
    let mut result = String::new();
    let count = n.min(self.chars.len() - self.offset);

    for _ in 0..count {
      if let Some(c) = self.consume() {
        result.push(c);
      }
    }

    result
  }

  /// 消费字符直到满足条件
  pub fn consume_while<F>(&mut self, predicate: F) -> String
  where
    F: Fn(char) -> bool,
  {
    let start_pos = self.offset;
    let mut end_pos = start_pos;

    while end_pos < self.chars.len() && predicate(self.chars[end_pos]) {
      end_pos += 1;
    }

    if start_pos == end_pos {
      return String::new();
    }

    // 构建结果字符串
    let result: String = self.chars[start_pos..end_pos].iter().collect();

    // 更新位置和行列信息
    for c in &self.chars[start_pos..end_pos] {
      self.offset += 1;
      if *c == '\n' {
        self.line += 1;
        self.column = 1;
      } else {
        self.column += 1;
      }
    }

    result
  }

  /// 跳过空白字符
  pub fn skip_whitespace(&mut self) {
    self.consume_while(|c| c.is_whitespace());
  }

  /// 消费直到指定的字符串
  pub fn consume_until(&mut self, target: &str) -> String {
    let mut result = String::new();
    let target_chars: Vec<char> = target.chars().collect();
    let target_len = target_chars.len();

    if target_len == 0 {
      return result;
    }

    while !self.is_eof() {
      if self.offset + target_len <= self.chars.len() {
        let window = &self.chars[self.offset..self.offset + target_len];
        if window == target_chars.as_slice() {
          break;
        }
      }

      if let Some(c) = self.consume() {
        result.push(c);
      }
    }

    result
  }
}
