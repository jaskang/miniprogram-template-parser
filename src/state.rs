//! 解析状态模块，包含用于追踪解析过程的状态和工具函数

use crate::ast::Position;
use crate::error::ParseError;

// 解析状态，跟踪当前解析位置和上下文
pub struct ParseState {
  pub chars: Vec<char>,
  // 字符索引
  pub offset: usize,
  // 行号
  pub line: usize,
  // 列号
  pub column: usize,
  // 错误列表
  pub errors: Vec<ParseError>,
}

impl ParseState {
  // 创建新的解析状态
  pub fn new(source: &str) -> Self {
    Self {
      chars: source.chars().collect(),
      offset: 0,
      line: 1,
      column: 1,
      errors: Vec::new(),
    }
  }

  // 记录错误
  pub fn record_error(&mut self, error: ParseError) {
    self.errors.push(error);
  }

  // 获取当前位置
  pub fn position(&self) -> Position {
    Position {
      line: self.line,
      column: self.column,
    }
  }

  // 检查是否已经到达源码结尾
  pub fn is_eof(&self) -> bool {
    self.offset >= self.chars.len()
  }

  // 查看当前字符但不消费
  pub fn peek(&self) -> Option<char> {
    if self.is_eof() {
      None
    } else {
      Some(self.chars[self.offset])
    }
  }

  // 查看接下来的n个字符但不消费
  pub fn peek_n(&self, n: usize) -> String {
    if self.is_eof() {
      return String::new();
    }

    let end = (self.offset + n).min(self.chars.len());
    self.chars[self.offset..end].iter().collect()
  }

  // 查看接下来的字符是否匹配给定的字符串
  pub fn peek_str(&self, s: &str) -> bool {
    self.peek_n(s.len()) == s
  }

  // 消费当前字符并前进
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

  // 消费指定数量的字符
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

  // 消费字符直到满足条件
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

  // 跳过空白字符
  pub fn skip_whitespace(&mut self) {
    self.consume_while(|c| c.is_whitespace());
  }

  // 消费直到指定的字符串
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

  // 获取指定位置的字符串
  pub fn get_content(&self, start: usize, end: usize) -> String {
    if start >= self.chars.len() || end >= self.chars.len() {
      String::new()
    } else {
      self.chars[start..end].iter().collect()
    }
  }
}
