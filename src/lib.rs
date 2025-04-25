#![deny(clippy::all)]

mod ast;
mod error;
mod parser;
mod state;

use ast::{Node, Root};
use error::SyntaxError;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use parser::Parser;

/// 解析 WXML 字符串并返回 AST
///
/// 返回 AST 对象
///
/// 由于 napi-rs 3.0.0-alpha 版本的限制，我们返回一个包装对象
/// 通过 toJson() 方法可以获取 JSON 格式的 AST
#[napi]
pub fn parse(input: String) -> napi::Result<Root> {
  match Parser::new(input.as_str()).parse_root() {
    Ok(ast) => Ok(ast),
    Err(err) => Err(napi::Error::new(
      napi::Status::GenericFailure,
      format!("Parse error: {}", err),
    )),
  }
}

#[cfg(test)]
mod tests {
  use crate::{ast::Node, parse};

  #[test]
  fn basic() {
    let ast = parse("<div></div>".to_string()).unwrap();
    assert_eq!(ast.children.len(), 1);
    assert_eq!(ast.loc.start.offset, 0);
    assert_eq!(ast.loc.end.offset, 11); // Assuming the length of "<div></div>" is 15
  }
  #[test]
  fn expressions() {
    let ast = parse("<text>Hello {{ world }}</text>".to_string()).unwrap();
    if let Node::Element { children, .. } = &ast.children[0] {
      assert_eq!(children.len(), 2);
      if let Node::Text { content, .. } = &children[0] {
        assert_eq!(content, "Hello ");
      } else {
        panic!("Expected a text node");
      }
      if let Node::Expression { content, .. } = &children[1] {
        assert_eq!(content, "{{ world }}");
      } else {
        panic!("Expected a expression node");
      }
    } else {
      panic!("Expected an element node");
    }
  }
}
