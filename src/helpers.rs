/// 判断是否为模板中的空白字符
pub(crate) fn is_template_whitespace(c: char) -> bool {
  match c {
    ' ' | '\t' | '\n' | '\r' => true,
    _ => false,
  }
}

/// 判断是否为标签名允许的字符
pub(crate) fn is_tag_name_char(c: char) -> bool {
  c.is_ascii_alphanumeric()
    || c == '-'
    || c == '_'
    || c == '.'
    || c == ':'
    || !c.is_ascii()
    || c == '\\'
}

/// 判断是否为属性名允许的字符
pub(crate) fn is_attr_name_char(c: char) -> bool {
  !matches!(c, '"' | '\'' | '>' | '/' | '=') && !c.is_ascii_whitespace()
}
