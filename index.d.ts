/* auto-generated by NAPI-RS */
/* eslint-disable */
/** 属性节点，表示元素上的属性 */
export interface Attribute {
  name: string
  value: Array<AttributeValue>
  location: Location
}

/** 属性值类型 */
export type AttributeValue =
  | { type: 'Static', content: string, start: number, end: number, location: Location }
  | { type: 'Expression', content: string, start: number, end: number, location: Location }

/** 定义AST节点的位置范围 */
export interface Location {
  /** 开始位置 */
  start: Position
  /** 结束位置 */
  end: Position
}

/** AST节点类型，代表WXML文档中的各种元素 */
export type Node =
  | { type: 'Document', children: Array<Node>, start: number, end: number, location: Location }
  | { type: 'Element', name: string, attributes: Array<Attribute>, children: Array<Node>, isSelfClosing: boolean, content: string, start: number, end: number, location: Location }
  | { type: 'Text', content: string, start: number, end: number, location: Location }
  | { type: 'Expression', content: string, start: number, end: number, location: Location }
  | { type: 'Comment', content: string, start: number, end: number, location: Location }

/**
 * 解析 WXML 字符串并返回 AST
 *
 * 返回 AST 对象
 *
 * 由于 napi-rs 3.0.0-alpha 版本的限制，我们返回一个包装对象
 * 通过 toJson() 方法可以获取 JSON 格式的 AST
 */
export declare function parse(input: string): Node

/** 定义位置信息，用于标记AST节点在源码中的位置 */
export interface Position {
  /** 行号，从1开始 */
  line: number
  /** 列号，从1开始 */
  column: number
}
