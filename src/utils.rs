//! 工具函数模块，提供辅助函数来处理AST结构

use crate::ast::{Attribute, AttributeValue, ExpressionPart, Location, Node};

//------------------------------------------------------------------------------
// AST 操作辅助函数
//------------------------------------------------------------------------------

impl Node {
  /// 获取节点的位置信息
  pub fn location(&self) -> &Location {
    match self {
      Node::Document { location, .. } => location,
      Node::Element { location, .. } => location,
      Node::WxsScript { location, .. } => location,
      Node::Text { location, .. } => location,
      Node::Expression { location, .. } => location,
      Node::Comment { location, .. } => location,
    }
  }

  /// 获取节点的子节点（如果有）
  pub fn children(&self) -> Option<&Vec<Node>> {
    match self {
      Node::Document { children, .. } => Some(children),
      Node::Element { children, .. } => Some(children),
      _ => None,
    }
  }

  /// 获取节点的子节点的可变引用（如果有）
  pub fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
    match self {
      Node::Document { children, .. } => Some(children),
      Node::Element { children, .. } => Some(children),
      _ => None,
    }
  }

  /// 获取元素节点的属性（如果是元素节点）
  pub fn attributes(&self) -> Option<&Vec<Attribute>> {
    match self {
      Node::Element { attributes, .. } => Some(attributes),
      Node::WxsScript { attributes, .. } => Some(attributes),
      _ => None,
    }
  }

  /// 获取节点的内容（如果有）
  pub fn content(&self) -> Option<&str> {
    match self {
      Node::Text { content, .. } => Some(content),
      Node::Expression { content, .. } => Some(content),
      Node::Comment { content, .. } => Some(content),
      Node::WxsScript { content, .. } => Some(content),
      _ => None,
    }
  }

  /// 检查节点是否为特定类型
  pub fn is_element(&self) -> bool {
    matches!(self, Node::Element { .. })
  }

  pub fn is_text(&self) -> bool {
    matches!(self, Node::Text { .. })
  }

  pub fn is_expression(&self) -> bool {
    matches!(self, Node::Expression { .. })
  }

  pub fn is_comment(&self) -> bool {
    matches!(self, Node::Comment { .. })
  }

  pub fn is_wxs_script(&self) -> bool {
    matches!(self, Node::WxsScript { .. })
  }

  /// 如果是元素节点，获取标签名
  pub fn tag_name(&self) -> Option<&str> {
    match self {
      Node::Element { tag_name, .. } => Some(tag_name),
      _ => None,
    }
  }

  /// 递归查找所有符合条件的节点
  pub fn find_all<F>(&self, predicate: F) -> Vec<&Node>
  where
    F: Fn(&Node) -> bool + Copy,
  {
    let mut result = Vec::new();

    if predicate(self) {
      result.push(self);
    }

    if let Some(children) = self.children() {
      for child in children {
        result.extend(child.find_all(predicate));
      }
    }

    result
  }

  /// 查找第一个符合条件的节点
  pub fn find<F>(&self, predicate: F) -> Option<&Node>
  where
    F: Fn(&Node) -> bool + Copy,
  {
    if predicate(self) {
      return Some(self);
    }

    if let Some(children) = self.children() {
      for child in children {
        if let Some(found) = child.find(predicate) {
          return Some(found);
        }
      }
    }

    None
  }

  /// 根据标签名查找所有元素
  pub fn find_elements_by_tag(&self, tag_name: &str) -> Vec<&Node> {
    self.find_all(|node| {
      if let Node::Element { tag_name: name, .. } = node {
        name.to_lowercase() == tag_name.to_lowercase()
      } else {
        false
      }
    })
  }
}

//------------------------------------------------------------------------------
// 属性辅助函数
//------------------------------------------------------------------------------

impl Attribute {
  /// 创建一个新的静态属性
  pub fn new_static(name: String, content: String, location: Location) -> Self {
    Self {
      name,
      value: Some(AttributeValue::Static {
        content,
        location: location.clone(),
      }),
      location,
    }
  }

  /// 创建一个新的表达式属性
  pub fn new_expression(name: String, parts: Vec<ExpressionPart>, location: Location) -> Self {
    Self {
      name,
      value: Some(AttributeValue::Expression {
        parts,
        location: location.clone(),
      }),
      location,
    }
  }

  /// 创建一个没有值的属性（布尔属性）
  pub fn new_boolean(name: String, location: Location) -> Self {
    Self {
      name,
      value: None,
      location,
    }
  }

  /// 获取属性的静态值（如果是静态属性）
  pub fn static_value(&self) -> Option<&str> {
    match &self.value {
      Some(AttributeValue::Static { content, .. }) => Some(content),
      _ => None,
    }
  }

  /// 检查属性是否包含表达式
  pub fn has_expression(&self) -> bool {
    matches!(&self.value, Some(AttributeValue::Expression { .. }))
  }

  /// 获取属性的表达式部分（如果是表达式属性）
  pub fn expression_parts(&self) -> Option<&Vec<ExpressionPart>> {
    match &self.value {
      Some(AttributeValue::Expression { parts, .. }) => Some(parts),
      _ => None,
    }
  }
}
