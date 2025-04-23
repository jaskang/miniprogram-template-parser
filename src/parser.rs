//! This parser is designed for internal use,
//! not generating general-purpose AST.
//!
//! Also, the parser consumes string then produces AST directly without tokenizing.
//! For a formal parser, it should be:
//! `source -> tokens (produced by lexer/tokenizer) -> AST (produced by parser)`.
//! So, if you're learning or looking for a parser,
//! this is not a good example and you should look for other projects.

use crate::{
  ast::*,
  error::{SyntaxError, SyntaxErrorKind},
  helpers,
};
use std::{cmp::Ordering, iter::Peekable, ops::ControlFlow, str::CharIndices};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Supported languages.
pub enum Language {
  Html,
  Vue,
}

pub struct Parser<'s> {
  source: &'s str,
  language: Language,
  chars: Peekable<CharIndices<'s>>,
  state: ParserState,
}

#[derive(Default)]
struct ParserState {
  has_front_matter: bool,
}

impl<'s> Parser<'s> {
  pub fn new(source: &'s str, language: Language) -> Self {
    Self {
      source,
      language,
      chars: source.char_indices().peekable(),
      state: Default::default(),
    }
  }

  fn try_parse<F, R>(&mut self, f: F) -> PResult<R>
  where
    F: FnOnce(&mut Self) -> PResult<R>,
  {
    let chars = self.chars.clone();
    let result = f(self);
    if result.is_err() {
      self.chars = chars;
    }
    result
  }

  fn emit_error(&mut self, kind: SyntaxErrorKind) -> SyntaxError {
    let pos = self
      .chars
      .peek()
      .map(|(pos, _)| *pos)
      .unwrap_or(self.source.len());
    self.emit_error_with_pos(kind, pos)
  }

  fn emit_error_with_pos(&self, kind: SyntaxErrorKind, pos: usize) -> SyntaxError {
    let search = memchr::memchr_iter(b'\n', self.source.as_bytes()).try_fold(
      (1, 0),
      |(line, prev_offset), offset| match pos.cmp(&offset) {
        Ordering::Less => ControlFlow::Break((line, prev_offset)),
        Ordering::Equal => ControlFlow::Break((line, prev_offset)),
        Ordering::Greater => ControlFlow::Continue((line + 1, offset)),
      },
    );
    let (line, column) = match search {
      ControlFlow::Break((line, offset)) => (line, pos - offset + 1),
      ControlFlow::Continue((line, _)) => (line, 0),
    };
    SyntaxError {
      kind,
      pos,
      line,
      column,
    }
  }

  fn skip_ws(&mut self) {
    while self
      .chars
      .next_if(|(_, c)| c.is_ascii_whitespace())
      .is_some()
    {}
  }

  fn with_taken<T, F>(&mut self, parser: F) -> PResult<(T, &'s str)>
  where
    F: FnOnce(&mut Self) -> PResult<T>,
  {
    let start = self
      .chars
      .peek()
      .map(|(i, _)| *i)
      .unwrap_or(self.source.len());
    let parsed = parser(self)?;
    let end = self
      .chars
      .peek()
      .map(|(i, _)| *i)
      .unwrap_or(self.source.len());
    Ok((parsed, unsafe { self.source.get_unchecked(start..end) }))
  }

  fn parse_attr(&mut self) -> PResult<Attribute<'s>> {
    match self.language {
      Language::Html | Language::Angular => self.parse_native_attr().map(Attribute::Native),
      Language::Vue => self
        .try_parse(Parser::parse_vue_directive)
        .map(Attribute::VueDirective)
        .or_else(|_| self.parse_native_attr().map(Attribute::Native)),
      Language::Svelte => self
        .try_parse(Parser::parse_svelte_attr)
        .map(Attribute::Svelte)
        .or_else(|_| self.parse_native_attr().map(Attribute::Native)),
      Language::Astro => self
        .try_parse(Parser::parse_astro_attr)
        .map(Attribute::Astro)
        .or_else(|_| self.parse_native_attr().map(Attribute::Native)),
      Language::Jinja => {
        self.skip_ws();
        let result = if matches!(self.chars.peek(), Some((_, '{'))) {
          let mut chars = self.chars.clone();
          chars.next();
          match chars.next() {
            Some((_, '{')) => self.parse_native_attr().map(Attribute::Native),
            Some((_, '#')) => self.parse_jinja_comment().map(Attribute::JinjaComment),
            _ => self.parse_jinja_tag_or_block(None, &mut Parser::parse_attr),
          }
        } else {
          self.parse_native_attr().map(Attribute::Native)
        };
        if result.is_ok() {
          self.skip_ws();
        }
        result
      }
      Language::Vento => self
        .try_parse(|parser| parser.parse_vento_tag_or_block(None))
        .map(Attribute::VentoTagOrBlock)
        .or_else(|_| self.parse_native_attr().map(Attribute::Native)),
    }
  }

  fn parse_attr_name(&mut self) -> PResult<&'s str> {
    if matches!(self.language, Language::Jinja | Language::Vento) {
      let Some((start, mut end)) = (match self.chars.peek() {
        Some((i, '{')) => {
          let start = *i;
          let mut chars = self.chars.clone();
          chars.next();
          if let Some((_, '{')) = chars.next() {
            let end = start + self.parse_mustache_interpolation()?.0.len() + "{{}}".len();
            Some((start, end))
          } else {
            None
          }
        }
        Some((_, c)) if is_attr_name_char(*c) => self
          .chars
          .next()
          .map(|(start, c)| (start, start + c.len_utf8())),
        _ => None,
      }) else {
        return Err(self.emit_error(SyntaxErrorKind::ExpectAttrName));
      };

      while let Some((_, c)) = self.chars.peek() {
        if is_attr_name_char(*c) && *c != '{' {
          end += c.len_utf8();
          self.chars.next();
        } else if *c == '{' {
          let mut chars = self.chars.clone();
          chars.next();
          match chars.next() {
            Some((_, '%')) => {
              break;
            }
            Some((_, '{')) => {
              end += self.parse_mustache_interpolation()?.0.len() + "{{}}".len();
            }
            Some((_, c)) => {
              end += c.len_utf8();
              self.chars.next();
            }
            None => break,
          }
        } else {
          break;
        }
      }

      unsafe { Ok(self.source.get_unchecked(start..end)) }
    } else {
      let Some((start, start_char)) = self.chars.next_if(|(_, c)| is_attr_name_char(*c)) else {
        return Err(self.emit_error(SyntaxErrorKind::ExpectAttrName));
      };
      let mut end = start + start_char.len_utf8();

      while let Some((_, c)) = self.chars.next_if(|(_, c)| is_attr_name_char(*c)) {
        end += c.len_utf8();
      }

      unsafe { Ok(self.source.get_unchecked(start..end)) }
    }
  }

  fn parse_attr_value(&mut self) -> PResult<(&'s str, usize)> {
    let quote = self.chars.next_if(|(_, c)| *c == '"' || *c == '\'');

    if let Some((start, quote)) = quote {
      let is_jinja_or_vento = matches!(self.language, Language::Jinja | Language::Vento);
      let start = start + 1;
      let mut end = start;
      let mut chars_stack = vec![];
      loop {
        match self.chars.next() {
          Some((i, c)) if c == quote => {
            if chars_stack.is_empty() || !is_jinja_or_vento {
              end = i;
              break;
            } else if chars_stack.last().is_some_and(|last| *last == c) {
              chars_stack.pop();
            } else {
              chars_stack.push(c);
            }
          }
          Some((_, '{')) if is_jinja_or_vento => {
            chars_stack.push('{');
          }
          Some((_, '}'))
            if is_jinja_or_vento && chars_stack.last().is_some_and(|last| *last == '{') =>
          {
            chars_stack.pop();
          }
          Some(..) => continue,
          None => break,
        }
      }
      Ok((unsafe { self.source.get_unchecked(start..end) }, start))
    } else {
      fn is_unquoted_attr_value_char(c: char) -> bool {
        !c.is_ascii_whitespace() && !matches!(c, '"' | '\'' | '=' | '<' | '>' | '`')
      }

      let start = match self.chars.peek() {
        Some((i, c)) if is_unquoted_attr_value_char(*c) => *i,
        _ => return Err(self.emit_error(SyntaxErrorKind::ExpectAttrValue)),
      };

      let mut end = start;
      loop {
        match self.chars.peek() {
          Some((i, '{')) if matches!(self.language, Language::Jinja | Language::Vento) => {
            end = *i;
            let mut chars = self.chars.clone();
            chars.next();
            if chars.next_if(|(_, c)| *c == '{').is_some() {
              // We use inclusive range when returning string,
              // so we need to substract 1 here.
              end += self.parse_mustache_interpolation()?.0.len() + "{{}}".len() - 1;
            } else {
              self.chars.next();
            }
          }
          Some((i, c)) if is_unquoted_attr_value_char(*c) => {
            end = *i;
            self.chars.next();
          }
          _ => break,
        }
      }

      Ok((unsafe { self.source.get_unchecked(start..=end) }, start))
    }
  }

  fn parse_comment(&mut self) -> PResult<Comment<'s>> {
    let Some((start, _)) = self
      .chars
      .next_if(|(_, c)| *c == '<')
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '!'))
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '-'))
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '-'))
    else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectComment));
    };
    let start = start + 1;

    let mut end = start;
    loop {
      match self.chars.next() {
        Some((i, '-')) => {
          let mut chars = self.chars.clone();
          if chars
            .next_if(|(_, c)| *c == '-')
            .and_then(|_| chars.next_if(|(_, c)| *c == '>'))
            .is_some()
          {
            end = i;
            self.chars = chars;
            break;
          }
        }
        Some(..) => continue,
        None => break,
      }
    }

    Ok(Comment {
      raw: unsafe { self.source.get_unchecked(start..end) },
    })
  }

  fn parse_doctype(&mut self) -> PResult<Doctype<'s>> {
    let keyword_start = if let Some((start, _)) = self
      .chars
      .next_if(|(_, c)| *c == '<')
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '!'))
    {
      start + 1
    } else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectDoctype));
    };
    let keyword = if let Some((end, _)) = self
      .chars
      .next_if(|(_, c)| c.eq_ignore_ascii_case(&'d'))
      .and_then(|_| self.chars.next_if(|(_, c)| c.eq_ignore_ascii_case(&'o')))
      .and_then(|_| self.chars.next_if(|(_, c)| c.eq_ignore_ascii_case(&'c')))
      .and_then(|_| self.chars.next_if(|(_, c)| c.eq_ignore_ascii_case(&'t')))
      .and_then(|_| self.chars.next_if(|(_, c)| c.eq_ignore_ascii_case(&'y')))
      .and_then(|_| self.chars.next_if(|(_, c)| c.eq_ignore_ascii_case(&'p')))
      .and_then(|_| self.chars.next_if(|(_, c)| c.eq_ignore_ascii_case(&'e')))
    {
      unsafe { self.source.get_unchecked(keyword_start..end + 1) }
    } else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectDoctype));
    };
    self.skip_ws();

    let value_start = if let Some((start, _)) = self.chars.peek() {
      *start
    } else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectDoctype));
    };
    while self.chars.next_if(|(_, c)| *c != '>').is_some() {}

    if let Some((value_end, _)) = self.chars.next_if(|(_, c)| *c == '>') {
      Ok(Doctype {
        keyword,
        value: unsafe { self.source.get_unchecked(value_start..value_end) }.trim_end(),
      })
    } else {
      Err(self.emit_error(SyntaxErrorKind::ExpectDoctype))
    }
  }

  fn parse_element(&mut self) -> PResult<Element<'s>> {
    let Some(..) = self.chars.next_if(|(_, c)| *c == '<') else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectElement));
    };
    let tag_name = self.parse_tag_name()?;
    let void_element = helpers::is_void_element(tag_name, self.language);

    let mut attrs = vec![];
    let mut first_attr_same_line = true;
    loop {
      match self.chars.peek() {
        Some((_, '/')) => {
          self.chars.next();
          if self.chars.next_if(|(_, c)| *c == '>').is_some() {
            return Ok(Element {
              tag_name,
              attrs,
              first_attr_same_line,
              children: vec![],
              self_closing: true,
              void_element,
            });
          }
          return Err(self.emit_error(SyntaxErrorKind::ExpectSelfCloseTag));
        }
        Some((_, '>')) => {
          self.chars.next();
          if void_element {
            return Ok(Element {
              tag_name,
              attrs,
              first_attr_same_line,
              children: vec![],
              self_closing: false,
              void_element,
            });
          }
          break;
        }
        Some((_, '\n')) => {
          if attrs.is_empty() {
            first_attr_same_line = false;
          }
          self.chars.next();
        }
        Some((_, c)) if c.is_ascii_whitespace() => {
          self.chars.next();
        }
        _ => {
          attrs.push(self.parse_attr()?);
        }
      }
    }

    let mut children = vec![];
    if tag_name.eq_ignore_ascii_case("script")
      || tag_name.eq_ignore_ascii_case("style")
      || tag_name.eq_ignore_ascii_case("pre")
      || tag_name.eq_ignore_ascii_case("textarea")
    {
      let text_node = self.parse_raw_text_node(tag_name)?;
      let raw = text_node.raw;
      if !raw.is_empty() {
        children.push(Node {
          kind: NodeKind::Text(text_node),
          raw,
        });
      }
    }

    loop {
      match self.chars.peek() {
        Some((_, '<')) => {
          let mut chars = self.chars.clone();
          chars.next();
          if let Some((pos, _)) = chars.next_if(|(_, c)| *c == '/') {
            self.chars = chars;
            let close_tag_name = self.parse_tag_name()?;
            if !close_tag_name.eq_ignore_ascii_case(tag_name) {
              return Err(self.emit_error_with_pos(SyntaxErrorKind::ExpectCloseTag, pos));
            }
            self.skip_ws();
            if self.chars.next_if(|(_, c)| *c == '>').is_some() {
              break;
            }
            return Err(self.emit_error(SyntaxErrorKind::ExpectCloseTag));
          }
          children.push(self.parse_node()?);
        }
        Some(..) => {
          if tag_name.eq_ignore_ascii_case("script")
            || tag_name.eq_ignore_ascii_case("style")
            || tag_name.eq_ignore_ascii_case("pre")
            || tag_name.eq_ignore_ascii_case("textarea")
          {
            let text_node = self.parse_raw_text_node(tag_name)?;
            let raw = text_node.raw;
            if !raw.is_empty() {
              children.push(Node {
                kind: NodeKind::Text(text_node),
                raw,
              });
            }
          } else {
            children.push(self.parse_node()?);
          }
        }
        None => return Err(self.emit_error(SyntaxErrorKind::ExpectCloseTag)),
      }
    }

    Ok(Element {
      tag_name,
      attrs,
      first_attr_same_line,
      children,
      self_closing: false,
      void_element,
    })
  }

  fn parse_front_matter(&mut self) -> PResult<FrontMatter<'s>> {
    let Some((start, _)) = self
      .chars
      .next_if(|(_, c)| *c == '-')
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '-'))
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '-'))
    else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectFrontMatter));
    };
    let start = start + 1;

    let mut pair_stack = vec![];
    let mut end = start;
    loop {
      match self.chars.next() {
        Some((i, '-')) if pair_stack.is_empty() => {
          let mut chars = self.chars.clone();
          if chars
            .next_if(|(_, c)| *c == '-')
            .and_then(|_| chars.next_if(|(_, c)| *c == '-'))
            .is_some()
          {
            end = i;
            self.chars = chars;
            break;
          }
        }
        Some((_, c @ '\'' | c @ '"' | c @ '`')) => {
          let last = pair_stack.last();
          if last.is_some_and(|last| *last == c) {
            pair_stack.pop();
          } else if matches!(last, Some('$' | '{') | None) {
            pair_stack.push(c);
          }
        }
        Some((_, '$')) if matches!(pair_stack.last(), Some('`')) => {
          if self.chars.next_if(|(_, c)| *c == '{').is_some() {
            pair_stack.push('$');
          }
        }
        Some((_, '{')) if matches!(pair_stack.last(), Some('$' | '{') | None) => {
          pair_stack.push('{');
        }
        Some((_, '}')) if matches!(pair_stack.last(), Some('$' | '{')) => {
          pair_stack.pop();
        }
        Some((_, '/')) if !matches!(pair_stack.last(), Some('\'' | '"' | '`' | '/' | '*')) => {
          if let Some((_, c)) = self.chars.next_if(|(_, c)| *c == '/' || *c == '*') {
            pair_stack.push(c);
          }
        }
        Some((_, '\n')) => {
          if let Some('/') = pair_stack.last() {
            pair_stack.pop();
          }
        }
        Some((_, '*')) => {
          if self
            .chars
            .next_if(|(_, c)| *c == '/' && matches!(pair_stack.last(), Some('*')))
            .is_some()
          {
            pair_stack.pop();
          }
        }
        Some((_, '\\')) if matches!(pair_stack.last(), Some('\'' | '"' | '`')) => {
          self.chars.next();
        }
        Some(..) => continue,
        None => break,
      }
    }

    self.state.has_front_matter = true;
    Ok(FrontMatter {
      raw: unsafe { self.source.get_unchecked(start..end) },
      start,
    })
  }

  fn parse_identifier(&mut self) -> PResult<&'s str> {
    fn is_identifier_char(c: char) -> bool {
      c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii() || c == '\\'
    }

    let Some((start, _)) = self.chars.next_if(|(_, c)| is_identifier_char(*c)) else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectIdentifier));
    };
    let mut end = start;

    while let Some((i, _)) = self.chars.next_if(|(_, c)| is_identifier_char(*c)) {
      end = i;
    }

    unsafe { Ok(self.source.get_unchecked(start..=end)) }
  }

  /// This will consume the open and close char.
  fn parse_inside(&mut self, open: char, close: char, inclusive: bool) -> PResult<&'s str> {
    let Some(start) =
      self.chars.next_if(|(_, c)| *c == open).map(
        |(i, c)| {
          if inclusive {
            i
          } else {
            i + c.len_utf8()
          }
        },
      )
    else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectChar(open)));
    };
    let mut end = start;
    let mut stack = 0u8;
    for (i, c) in self.chars.by_ref() {
      if c == open {
        stack += 1;
      } else if c == close {
        if stack == 0 {
          end = if inclusive { i + close.len_utf8() } else { i };
          break;
        }
        stack -= 1;
      }
    }
    Ok(unsafe { self.source.get_unchecked(start..end) })
  }

  fn parse_jinja_block_children<T, F>(&mut self, children_parser: &mut F) -> PResult<Vec<T>>
  where
    T: HasJinjaFlowControl<'s>,
    F: FnMut(&mut Self) -> PResult<T>,
  {
    let mut children = vec![];
    loop {
      match self.chars.peek() {
        Some((_, '{')) => {
          let mut chars = self.chars.clone();
          chars.next();
          if chars.next_if(|(_, c)| *c == '%').is_some() {
            break;
          }
          children.push(children_parser(self)?);
        }
        Some(..) => {
          children.push(children_parser(self)?);
        }
        None => return Err(self.emit_error(SyntaxErrorKind::ExpectJinjaBlockEnd)),
      }
    }
    Ok(children)
  }

  fn parse_mustache_interpolation(&mut self) -> PResult<(&'s str, usize)> {
    let Some((start, _)) = self
      .chars
      .next_if(|(_, c)| *c == '{')
      .and_then(|_| self.chars.next_if(|(_, c)| *c == '{'))
    else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectMustacheInterpolation));
    };
    let start = start + 1;

    let mut braces_stack = 0usize;
    let mut end = start;
    loop {
      match self.chars.next() {
        Some((_, '{')) => braces_stack += 1,
        Some((i, '}')) => {
          if braces_stack == 0 {
            if self.chars.next_if(|(_, c)| *c == '}').is_some() {
              end = i;
              break;
            }
          } else {
            braces_stack -= 1;
          }
        }
        Some(..) => continue,
        None => break,
      }
    }

    Ok((unsafe { self.source.get_unchecked(start..end) }, start))
  }

  fn parse_native_attr(&mut self) -> PResult<NativeAttribute<'s>> {
    let name = self.parse_attr_name()?;
    self.skip_ws();
    let mut quote = None;
    let value = if self.chars.next_if(|(_, c)| *c == '=').is_some() {
      self.skip_ws();
      quote = self
        .chars
        .peek()
        .and_then(|(_, c)| (*c == '\'' || *c == '"').then_some(*c));
      Some(self.parse_attr_value()?)
    } else {
      None
    };
    Ok(NativeAttribute { name, value, quote })
  }

  fn parse_node(&mut self) -> PResult<Node<'s>> {
    let (kind, raw) = self.with_taken(Parser::parse_node_kind)?;
    Ok(Node { kind, raw })
  }

  fn parse_node_kind(&mut self) -> PResult<NodeKind<'s>> {
    match self.chars.peek() {
      Some((_, '<')) => {
        let mut chars = self.chars.clone();
        chars.next();
        match chars.next() {
          Some((_, c))
            if is_html_tag_name_char(c) || is_special_tag_name_char(c, self.language) =>
          {
            self.parse_element().map(NodeKind::Element)
          }
          Some((_, '!')) => {
            if matches!(
              self.language,
              Language::Html | Language::Astro | Language::Jinja | Language::Vento
            ) {
              self
                .try_parse(Parser::parse_comment)
                .map(NodeKind::Comment)
                .or_else(|_| self.try_parse(Parser::parse_doctype).map(NodeKind::Doctype))
                .or_else(|_| self.parse_text_node().map(NodeKind::Text))
            } else {
              self.parse_comment().map(NodeKind::Comment)
            }
          }
          _ => self.parse_text_node().map(NodeKind::Text),
        }
      }
      Some((_, '{')) => {
        let mut chars = self.chars.clone();
        chars.next();
        match chars.next() {
          Some((_, '{'))
            if matches!(
              self.language,
              Language::Vue | Language::Jinja | Language::Angular
            ) =>
          {
            self
              .parse_mustache_interpolation()
              .map(|(expr, start)| match self.language {
                Language::Vue => NodeKind::VueInterpolation(VueInterpolation { expr, start }),
                Language::Jinja => NodeKind::JinjaInterpolation(JinjaInterpolation { expr }),
                Language::Angular => {
                  NodeKind::AngularInterpolation(AngularInterpolation { expr, start })
                }
                _ => unreachable!(),
              })
          }
          Some((_, '{')) if matches!(self.language, Language::Vento) => {
            self.parse_vento_tag_or_block(None)
          }
          Some((_, '#')) if matches!(self.language, Language::Svelte) => match chars.next() {
            Some((_, 'i')) => self.parse_svelte_if_block().map(NodeKind::SvelteIfBlock),
            Some((_, 'e')) => self
              .parse_svelte_each_block()
              .map(NodeKind::SvelteEachBlock),
            Some((_, 'a')) => self
              .parse_svelte_await_block()
              .map(NodeKind::SvelteAwaitBlock),
            Some((_, 'k')) => self.parse_svelte_key_block().map(NodeKind::SvelteKeyBlock),
            Some((_, 's')) => self
              .parse_svelte_snippet_block()
              .map(NodeKind::SvelteSnippetBlock),
            _ => self.parse_text_node().map(NodeKind::Text),
          },
          Some((_, '#')) if matches!(self.language, Language::Jinja) => {
            self.parse_jinja_comment().map(NodeKind::JinjaComment)
          }
          Some((_, '@')) => self.parse_svelte_at_tag().map(NodeKind::SvelteAtTag),
          Some((_, '%')) if matches!(self.language, Language::Jinja) => {
            self.parse_jinja_tag_or_block(None, &mut Parser::parse_node)
          }
          _ => match self.language {
            Language::Svelte => self
              .parse_svelte_interpolation()
              .map(NodeKind::SvelteInterpolation),
            Language::Astro => self.parse_astro_expr().map(NodeKind::AstroExpr),
            _ => self.parse_text_node().map(NodeKind::Text),
          },
        }
      }
      Some((_, '-'))
        if matches!(
          self.language,
          Language::Astro | Language::Jinja | Language::Vento
        ) && !self.state.has_front_matter =>
      {
        let mut chars = self.chars.clone();
        chars.next();
        if let Some(((_, '-'), (_, '-'))) = chars.next().zip(chars.next()) {
          self.parse_front_matter().map(NodeKind::FrontMatter)
        } else {
          self.parse_text_node().map(NodeKind::Text)
        }
      }
      Some((_, '@')) if matches!(self.language, Language::Angular) => {
        let mut chars = self.chars.clone();
        chars.next();
        match chars.next() {
          Some((_, 'i')) => self.parse_angular_if().map(NodeKind::AngularIf),
          Some((_, 'f')) => self.parse_angular_for().map(NodeKind::AngularFor),
          Some((_, 's')) => self.parse_angular_switch().map(NodeKind::AngularSwitch),
          Some((_, 'l')) => self.parse_angular_let().map(NodeKind::AngularLet),
          _ => self.parse_text_node().map(NodeKind::Text),
        }
      }
      Some(..) => self.parse_text_node().map(NodeKind::Text),
      None => Err(self.emit_error(SyntaxErrorKind::ExpectElement)),
    }
  }

  fn parse_raw_text_node(&mut self, tag_name: &str) -> PResult<TextNode<'s>> {
    let start = self
      .chars
      .peek()
      .map(|(i, _)| *i)
      .unwrap_or(self.source.len());

    let allow_nested = tag_name.eq_ignore_ascii_case("pre");
    let mut nested = 0u16;
    let mut line_breaks = 0;
    let end;
    loop {
      match self.chars.peek() {
        Some((i, '<')) => {
          let i = *i;
          let mut chars = self.chars.clone();
          chars.next();
          if chars.next_if(|(_, c)| *c == '/').is_some()
            && chars
              .by_ref()
              .zip(tag_name.chars())
              .all(|((_, a), b)| a.eq_ignore_ascii_case(&b))
          {
            if nested == 0 {
              end = i;
              break;
            } else {
              nested -= 1;
              self.chars = chars;
              continue;
            }
          } else if allow_nested
            && chars
              .by_ref()
              .zip(tag_name.chars())
              .all(|((_, a), b)| a.eq_ignore_ascii_case(&b))
          {
            nested += 1;
            self.chars = chars;
            continue;
          }
          self.chars.next();
        }
        Some((_, c)) => {
          if *c == '\n' {
            line_breaks += 1;
          }
          self.chars.next();
        }
        None => {
          end = self.source.len();
          break;
        }
      }
    }

    Ok(TextNode {
      raw: unsafe { self.source.get_unchecked(start..end) },
      line_breaks,
      start,
    })
  }

  pub fn parse_root(&mut self) -> PResult<Root<'s>> {
    let mut children = vec![];
    while self.chars.peek().is_some() {
      children.push(self.parse_node()?);
    }

    Ok(Root { children })
  }

  fn parse_tag_name(&mut self) -> PResult<&'s str> {
    let (start, mut end) = match self.chars.peek() {
      Some((i, c)) if is_html_tag_name_char(*c) => {
        let c = *c;
        let start = *i;
        self.chars.next();
        (start, start + c.len_utf8())
      }
      Some((i, '{')) if matches!(self.language, Language::Jinja) => (*i, *i + 1),
      Some((_, '>')) if matches!(self.language, Language::Astro) => {
        // Astro allows fragment
        return Ok("");
      }
      _ => return Err(self.emit_error(SyntaxErrorKind::ExpectTagName)),
    };

    while let Some((i, c)) = self.chars.peek() {
      if is_html_tag_name_char(*c) {
        end = *i + c.len_utf8();
        self.chars.next();
      } else if *c == '{' && matches!(self.language, Language::Jinja) {
        let current_i = *i;
        let mut chars = self.chars.clone();
        chars.next();
        if chars.next_if(|(_, c)| *c == '{').is_some() {
          end = current_i + self.parse_mustache_interpolation()?.0.len() + "{{}}".len();
        } else {
          break;
        }
      } else {
        break;
      }
    }

    unsafe { Ok(self.source.get_unchecked(start..end)) }
  }

  fn parse_text_node(&mut self) -> PResult<TextNode<'s>> {
    let Some((start, first_char)) = self.chars.next_if(|(_, c)| {
      if matches!(
        self.language,
        Language::Vue | Language::Svelte | Language::Jinja | Language::Vento | Language::Angular
      ) {
        *c != '{'
      } else {
        true
      }
    }) else {
      return Err(self.emit_error(SyntaxErrorKind::ExpectTextNode));
    };

    if matches!(
      self.language,
      Language::Vue | Language::Jinja | Language::Vento | Language::Angular
    ) && first_char == '{'
      && matches!(self.chars.peek(), Some((_, '{')))
    {
      return Err(self.emit_error(SyntaxErrorKind::ExpectTextNode));
    }

    let mut line_breaks = if first_char == '\n' { 1 } else { 0 };
    let end;
    loop {
      match self.chars.peek() {
        Some((i, '{')) => match self.language {
          Language::Html => {
            self.chars.next();
          }
          Language::Vue | Language::Vento | Language::Angular => {
            let i = *i;
            let mut chars = self.chars.clone();
            chars.next();
            if chars.next_if(|(_, c)| *c == '{').is_some() {
              end = i;
              break;
            }
            self.chars.next();
          }
          Language::Svelte | Language::Astro => {
            end = *i;
            break;
          }
          Language::Jinja => {
            let i = *i;
            let mut chars = self.chars.clone();
            chars.next();
            if chars
              .next_if(|(_, c)| *c == '%' || *c == '{' || *c == '#')
              .is_some()
            {
              end = i;
              break;
            }
            self.chars.next();
          }
        },
        Some((i, '<')) => {
          let i = *i;
          let mut chars = self.chars.clone();
          chars.next();
          match chars.next() {
            Some((_, c))
              if is_html_tag_name_char(c)
                || is_special_tag_name_char(c, self.language)
                || c == '/'
                || c == '!' =>
            {
              end = i;
              break;
            }
            _ => {
              self.chars.next();
            }
          }
        }
        Some((i, '-'))
          if matches!(self.language, Language::Astro) && !self.state.has_front_matter =>
        {
          let i = *i;
          let mut chars = self.chars.clone();
          chars.next();
          if let Some(((_, '-'), (_, '-'))) = chars.next().zip(chars.next()) {
            end = i;
            break;
          }
          self.chars.next();
        }
        Some((i, '}' | '@')) if matches!(self.language, Language::Angular) => {
          end = *i;
          break;
        }
        Some((_, c)) => {
          if *c == '\n' {
            line_breaks += 1;
          }
          self.chars.next();
        }
        None => {
          end = self.source.len();
          break;
        }
      }
    }

    Ok(TextNode {
      raw: unsafe { self.source.get_unchecked(start..end) },
      line_breaks,
      start,
    })
  }

  fn parse_vue_directive(&mut self) -> PResult<VueDirective<'s>> {
    let name = match self.chars.peek() {
      Some((_, ':')) => {
        self.chars.next();
        ":"
      }
      Some((_, '@')) => {
        self.chars.next();
        "@"
      }
      Some((_, '#')) => {
        self.chars.next();
        "#"
      }
      Some((_, 'v')) => {
        let mut chars = self.chars.clone();
        chars.next();
        if chars.next_if(|(_, c)| *c == '-').is_some() {
          self.chars = chars;
          self.parse_identifier()?
        } else {
          return Err(self.emit_error(SyntaxErrorKind::ExpectVueDirective));
        }
      }
      _ => return Err(self.emit_error(SyntaxErrorKind::ExpectVueDirective)),
    };

    let arg_and_modifiers = if matches!(name, ":" | "@" | "#")
      || self
        .chars
        .peek()
        .map(|(_, c)| is_attr_name_char(*c))
        .unwrap_or_default()
    {
      Some(self.parse_attr_name()?)
    } else {
      None
    };

    self.skip_ws();
    let value = if self.chars.next_if(|(_, c)| *c == '=').is_some() {
      self.skip_ws();
      Some(self.parse_attr_value()?)
    } else {
      None
    };

    Ok(VueDirective {
      name,
      arg_and_modifiers,
      value,
    })
  }
}

/// Returns true if the provided character is a valid HTML tag name character.
fn is_html_tag_name_char(c: char) -> bool {
  c.is_ascii_alphanumeric()
    || c == '-'
    || c == '_'
    || c == '.'
    || c == ':'
    || !c.is_ascii()
    || c == '\\'
}

/// Checks whether a character is valid in an HTML tag name, for specific template languages.
///
/// For example:
/// - Astro allows '>' in tag names (for fragments)
/// - Jinja allows '{' for template expressions like <{{ tag_name }}>
fn is_special_tag_name_char(c: char, language: Language) -> bool {
  match language {
    Language::Astro => c == '>',
    Language::Jinja => c == '{',
    _ => false,
  }
}

fn is_attr_name_char(c: char) -> bool {
  !matches!(c, '"' | '\'' | '>' | '/' | '=') && !c.is_ascii_whitespace()
}

pub type PResult<T> = Result<T, SyntaxError>;
