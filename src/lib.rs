#![deny(clippy::all)]

mod ast;
mod error;
mod parser;
mod state;
mod utils;

use napi_derive::napi;

#[napi]
pub fn parse(input: String) -> String {
  let result = parser::parse(&input);
  parser::ast_to_json(&result.ast)
}
