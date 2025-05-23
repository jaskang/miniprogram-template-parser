//! WXML 模板解析库
//!
//! 此库实现了微信小程序 WXML 模板语法的解析器，可以将 WXML 模板转换为抽象语法树
//! 支持标准 WXML 的常见功能和 {{ }} 表达式语法

pub mod ast;
pub mod error;
pub mod helpers;
pub mod parser;
pub mod state;

use napi_derive::napi;
use parser::Parser;

/// 将 WXML 模板字符串解析为抽象语法树
#[napi]
pub fn parse(source: String) -> ast::Root {
  let mut parser = Parser::new(&source);
  parser.parse_root().unwrap()
}

/// 暴露 AST 类型以方便使用
pub use ast::{Attribute, AttributeValue, Node, Position, Root, Value};

/// 暴露错误类型以方便使用
pub use error::{SyntaxError, SyntaxErrorKind};

#[cfg(test)]
mod tests {
  use crate::{ast::Node, parse, AttributeValue, Position};

  #[test]
  fn basic() {
    let ast = parse("<div></div>".to_string());
    assert_eq!(ast.children.len(), 1);
    assert_eq!(ast.start.offset, 0);
    assert_eq!(ast.end.offset, 11);
  }

  #[test]
  fn attrs() {
    let ast = parse("<view class=\"cls1\" bindtap=\"{{handleTap}}\"></view>".to_string());
    if let Node::Element { attrs, .. } = &ast.children[0] {
      println!("attrs: {:?}", attrs);
      assert_eq!(attrs.len(), 2);
      let attr0 = &attrs[0];
      let attr1 = &attrs[1];
      assert_eq!(attr0.name, "class");
      assert_eq!(attr1.name, "bindtap");
      if let AttributeValue::Text { content, .. } = attr0.value.as_ref().unwrap().get(0).unwrap() {
        assert_eq!(content, "cls1");
      } else {
        panic!("Expected a Text variant");
      }
    } else {
      panic!("Expected an Element node");
    }
  }

  #[test]
  fn mixedattrs() {
    let ast = parse(
      "<view class=\"cls1 {{tst}} cls2\" bindtap=\"tap1 tap2 {{handleTap}}\"></view>".to_string(),
    );

    if let Node::Element { attrs, .. } = &ast.children[0] {
      assert_eq!(attrs.len(), 2);
      let attr0 = &attrs[0];
      let attr1 = &attrs[1];

      if let Some(values) = &attr0.value {
        assert_eq!(values.len(), 3);
      } else {
        panic!("Expected attribute value");
      }

      if let Some(values) = &attr1.value {
        assert_eq!(values.len(), 2);
      } else {
        panic!("Expected attribute value");
      }
    } else {
      panic!("Expected an Element node");
    }
  }

  #[test]
  fn expressions() {
    let ast = parse("<text>Hello {{ world }}</text>".to_string());
    if let Node::Element { children, .. } = &ast.children[0] {
      println!("children: {:?}", children);
      assert_eq!(children.len(), 2);
      if let Node::Text { content, .. } = &children[0] {
        assert_eq!(content, "Hello ");
      } else {
        panic!("Expected a Text node");
      }
      if let Node::Expression { content, .. } = &children[1] {
        assert_eq!(content, "world");
      } else {
        panic!("Expected an Expression node");
      }
    } else {
      panic!("Expected an Element node");
    }
  }
}
