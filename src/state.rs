use crate::{
  ast::{Position, Range},
  error::{SyntaxError, SyntaxErrorKind},
};

/// Some meta information of the parsing.
pub struct ParseState<'s> {
  path: String,
  whole_str: &'s str,
  offset: usize,
  line: usize,
  column: usize,
  errors: Vec<SyntaxError>,
}

impl<'s> ParseState<'s> {
  /// Prepare a string for parsing.
  ///
  /// `path` and `position_offset` are used to adjust warning output.
  pub fn new(path: &str, content: &'s str) -> Self {
    let s = content;
    let s = if s.len() >= u32::MAX as usize {
      // log::error!("Source code too long. Truncated to `u32::MAX - 1` .");
      &s[..(u32::MAX as usize - 1)]
    } else {
      s
    };
    Self {
      path: path.to_string(),
      whole_str: s,
      offset: 0,
      line: 0,
      column: 0,
      errors: vec![],
    }
  }

  fn add_error_with_position(&mut self, kind: SyntaxErrorKind, position: Position) -> SyntaxError {
    let err = SyntaxError {
      kind,
      offset: position.offset,
      line: position.line,
      column: position.column,
    };
    self.errors.push(err);
    err.clone()
  }
  /// Add a new error at the current position.
  pub fn add_error(&mut self, kind: SyntaxErrorKind) -> SyntaxError {
    let pos = self.position();
    self.add_error_with_position(kind, pos)
  }

  /// List errors.
  pub fn errors(&self) -> impl Iterator<Item = &SyntaxError> {
    self.errors.iter()
  }

  /// Extract and then clear all errors.
  pub fn take_errors(&mut self) -> Vec<SyntaxError> {
    std::mem::replace(&mut self.errors, vec![])
  }

  fn cur_str(&self) -> &'s str {
    &self.whole_str[self.offset..]
  }

  /// Whether the input is ended.
  pub fn ended(&self) -> bool {
    self.cur_str().len() == 0
  }

  /// Try parse with `f` , reverting the state if it returns `None` .
  pub(crate) fn try_parse<T>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
    let prev = self.offset;
    let prev_line = self.line;
    let prev_column = self.column;
    let ret = f(self);
    if ret.is_none() {
      self.offset = prev;
      self.line = prev_line;
      self.column = prev_column;
    }
    ret
  }

  fn skip_bytes(&mut self, count: usize) {
    let skipped = &self.cur_str()[..count];
    self.offset += count;
    let line_wrap_count = skipped
      .as_bytes()
      .into_iter()
      .filter(|x| **x == b'\n')
      .count();
    self.line += line_wrap_count;
    if line_wrap_count > 0 {
      let last_line_start = skipped.rfind('\n').unwrap() + 1;
      self.column = skipped[last_line_start..].encode_utf16().count();
    } else {
      self.column += skipped.encode_utf16().count();
    }
  }

  pub(crate) fn skip_until_before(&mut self, until: &str) -> Option<&'s str> {
    let s = self.cur_str();
    if let Some(index) = s.find(until) {
      let ret = &s[..index];
      self.skip_bytes(index);
      Some(ret)
    } else {
      self.skip_bytes(s.len());
      None
    }
  }

  pub(crate) fn skip_until_after(&mut self, until: &str) -> Option<&'s str> {
    let ret = self.skip_until_before(until);
    if ret.is_some() {
      self.skip_bytes(until.len());
    }
    ret
  }

  pub(crate) fn peek_chars(&mut self) -> impl 's + Iterator<Item = char> {
    self.cur_str().chars()
  }

  pub(crate) fn peek_n<const N: usize>(&mut self) -> Option<[char; N]> {
    let mut ret: [char; N] = ['\x00'; N];
    let mut iter = self.peek_chars();
    for i in 0..N {
      ret[i] = iter.next()?;
    }
    Some(ret)
  }

  pub(crate) fn peek(&mut self) -> Option<char> {
    let mut iter = self.peek_chars();
    iter.next()
  }

  pub(crate) fn peek_str(&mut self, s: &str) -> bool {
    self.cur_str().starts_with(s)
  }

  fn consume_str_except_followed<const N: usize>(
    &mut self,
    s: &str,
    excepts: [&str; N],
  ) -> Option<Range> {
    if !self.peek_str(s) {
      return None;
    }
    let s_followed = &self.cur_str()[s.len()..];
    for except in excepts {
      if s_followed.starts_with(except) {
        return None;
      }
    }
    let start = self.position();
    self.skip_bytes(s.len());
    let end = self.position();
    Some(Range { start, end })
  }

  fn consume_str_except_followed_char(
    &mut self,
    s: &str,
    reject_followed: impl FnOnce(char) -> bool,
  ) -> Option<Range> {
    if !self.peek_str(s) {
      return None;
    }
    let s_followed = &self.cur_str()[s.len()..];
    match s_followed.chars().next() {
      None => {}
      Some(ch) => {
        if reject_followed(ch) {
          return None;
        }
      }
    }
    let start = self.position();
    self.skip_bytes(s.len());
    let end = self.position();
    Some(Range { start, end })
  }

  /// Consume the specified string if it matches the peek of the input.
  pub(crate) fn consume_str(&mut self, s: &str) -> Option<Range> {
    self.consume_str_except_followed(s, [])
  }

  fn next_char_as_str(&mut self) -> &'s str {
    let s = self.cur_str();
    if s.len() > 0 {
      let mut i = 0;
      loop {
        i += 1;
        if s.is_char_boundary(i) {
          break;
        }
      }
      let ret = &s[..i];
      self.skip_bytes(i);
      ret
    } else {
      ""
    }
  }

  pub(crate) fn next(&mut self) -> Option<char> {
    let mut i = self.cur_str().char_indices();
    let (_, ret) = i.next()?;
    self.offset += match i.next() {
      Some((p, _)) => p,
      None => self.cur_str().len(),
    };
    if ret == '\n' {
      self.line += 1;
      self.column = 0;
    } else {
      self.column += ret.encode_utf16(&mut [0; 2]).len();
    }
    Some(ret)
  }

  pub(crate) fn skip_whitespace(&mut self) -> Option<Range> {
    let mut start_pos = None;
    let s = self.cur_str();
    let mut i = s.char_indices();
    self.offset += loop {
      let Some((index, c)) = i.next() else {
        break s.len();
      };
      if !is_template_whitespace(c) {
        break index;
      }
      if start_pos.is_none() {
        start_pos = Some(self.position());
      }
      if c == '\n' {
        self.line += 1;
        self.column = 0;
      } else {
        self.column += c.encode_utf16(&mut [0; 2]).len();
      }
    };
    start_pos.map(|x| Range {
      start: x,
      end: self.position(),
    })
  }

  pub(crate) fn skip_whitespace_with_js_comments(&mut self) -> Option<Range> {
    let mut start_pos = None;
    loop {
      if let Some(range) = self.skip_whitespace() {
        if start_pos.is_none() {
          start_pos = Some(range.start);
        }
        continue;
      }
      if self.cur_str().starts_with("/*") {
        if start_pos.is_none() {
          start_pos = Some(self.position());
        }
        self.skip_bytes(2);
        self.skip_until_after("*/");
        continue;
      }
      break;
    }
    start_pos.map(|x| Range {
      start: x,
      end: self.position(),
    })
  }

  /// Get the input slice by UTF-8 byte index range.
  ///
  /// Panics if the start or the end is not at a character boundary.
  ///
  pub(crate) fn code_slice(&self, range: [usize; 2]) -> &'s str {
    &self.whole_str[range[0]..range[1]]
  }

  /// Get the current UTF-8 byte index in the input.
  pub(crate) fn cur_offset(&self) -> u32 {
    self.offset as u32
  }

  /// Get the current position.
  pub(crate) fn position(&self) -> Position {
    Position {
      offset: self.cur_offset(),
      line: self.line as u32,
      column: self.column as u32,
    }
  }
}

const fn is_template_whitespace(c: char) -> bool {
  match c {
    ' ' => true,
    '\x09'..='\x0D' => true,
    _ => false,
  }
}
