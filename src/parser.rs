use std::vec;

use crate::{
  ast::*,
  error::{SyntaxError, SyntaxErrorKind},
  state::ParseState,
};

pub type PResult<T> = Result<T, SyntaxError>;

/// Parser结构体表示模板解析器的状态
///
/// 字段说明：
/// * `source` - 待解析的源代码字符串
/// * `state` - 解析状态，包含字符迭代器和位置信息
pub struct Parser<'s> {
  source: &'s str,
  state: ParseState<'s>,
}

