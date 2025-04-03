#![deny(clippy::all)]

mod ast;
mod error;
mod parser;
mod state;

use ast::{Document, Node};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// 解析 WXML 字符串并返回 AST
///
/// 返回 AST 对象
///
/// 由于 napi-rs 3.0.0-alpha 版本的限制，我们返回一个包装对象
/// 通过 toJson() 方法可以获取 JSON 格式的 AST
#[napi]
pub fn parse(input: String) -> napi::Result<Document> {
  let result = parser::parse(&input);
  Ok(result)
}
