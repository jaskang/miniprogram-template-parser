import { describe, expect, it } from 'vitest'
import { parse } from '..'

describe('template parser', () => {
  it('basic ', () => {
    const tpl = `<view>{{name}}</view>`
    const result = JSON.parse(parse(tpl))
    expect(result).toEqual({
      type: 'Document',
      children: [
        {
          type: 'Element',
          name: 'view',
          attributes: [],
          children: [
            {
              type: 'Expression',
              content: 'name',
              start: 6,
              end: 14,
              location: {
                start: { line: 1, column: 7 },
                end: { line: 1, column: 15 },
              },
            },
          ],
          is_self_closing: false,
          content: '',
          start: 0,
          end: 21,
          location: {
            start: { line: 1, column: 1 },
            end: { line: 1, column: 22 },
          },
        },
      ],
      start: 0,
      end: 21,
      location: {
        start: { line: 1, column: 1 },
        end: { line: 1, column: 22 },
      },
    })
  })
  it('attributes', () => {
    const tpl = `<view class="container" bindtap="{{handleTap}}" />`
    const result = JSON.parse(parse(tpl))
    expect(result.children[0].attributes).toEqual([
      {
        end: 23,
        location: {
          end: { column: 24, line: 1 },
          start: { column: 7, line: 1 },
        },
        name: 'class',
        start: 6,
        value: [
          {
            content: 'container',
            end: 22,
            location: {
              end: { column: 23, line: 1 },
              start: { column: 13, line: 1 },
            },
            start: 13,
            type: 'Text',
          },
        ],
      },
      {
        end: 47,
        location: {
          end: { column: 48, line: 1 },
          start: { column: 25, line: 1 },
        },
        name: 'bindtap',
        start: 24,
        value: [
          {
            content: '{{handleTap}}',
            end: 46,
            location: {
              end: { column: 47, line: 1 },
              start: { column: 34, line: 1 },
            },
            start: 33,
            type: 'Expression',
          },
        ],
      },
    ])
  })
})
