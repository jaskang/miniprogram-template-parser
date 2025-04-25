import { describe, it, expect } from "vitest";
import { Attribute, parse } from "../";

describe("parse", () => {
  it("toBeDefined", () => {
    const result = parse(`<text>Hello {{name}}</text>`);
    // console.log(result);
    expect(result).toBeDefined();
  });
  // it("start tag", () => {
  //   const result = parse(`<text>Hello</text>`);
  //   // console.log(result);
  //   expect(result.children[0].startTag).toEqual({
  //     location: {
  //       start: { column: 2, line: 1, offset: 1 },
  //       end: { column: 6, line: 1, offset: 5 },
  //     },
  //     value: "text",
  //   });
  //   const result1 = parse(`<text  class="cls" />`);
  //   // console.log(result);
  //   expect(result1.children[0].startTag).toEqual({
  //     location: {
  //       start: { column: 2, line: 1, offset: 1 },
  //       end: { column: 6, line: 1, offset: 5 },
  //     },
  //     value: "text",
  //   });
  // });
  // it("end tag", () => {
  //   const result = parse(`<text>Hello</text>`);
  //   // console.log(result);
  //   expect(result.children[0].endTag).toEqual({
  //     location: {
  //       start: { column: 14, line: 1, offset: 13 },
  //       end: { column: 18, line: 1, offset: 17 },
  //     },
  //     value: "text",
  //   });
  //   const result1 = parse(`<text  class="cls" />`);
  //   // console.log(result);
  //   expect(result1.children[0].endTag).toEqual(null);
  // });
  // it("attributes", () => {
  //   const wxml = `<view class="cls1" bindtap="{{handleTap}}"></view>`;
  //   const result = parse(wxml);
  //   const attributes = result.children[0].attributes as Attribute[];
  //   expect(attributes[0]).toEqual({
  //     name: "class",
  //     value: [
  //       {
  //         type: "Text",
  //         content: "cls1",
  //         location: {
  //           start: { column: 14, line: 1, offset: 13 },
  //           end: { column: 18, line: 1, offset: 17 },
  //         },
  //       },
  //     ],
  //     location: {
  //       start: { column: 7, line: 1, offset: 6 },
  //       end: { column: 19, line: 1, offset: 18 },
  //     },
  //   });
  //   expect(attributes[1]).toEqual({
  //     name: "bindtap",
  //     value: [
  //       {
  //         type: "Expression",
  //         content: "{{handleTap}}",
  //         location: {
  //           start: { column: 29, line: 1, offset: 28 },
  //           end: { column: 42, line: 1, offset: 41 },
  //         },
  //       },
  //     ],
  //     location: {
  //       start: { column: 20, line: 1, offset: 19 },
  //       end: { column: 43, line: 1, offset: 42 },
  //     },
  //   });
  // });
  // // <view class="cls1 {{test}} cls2"></view>
  // it("mixin attributes", () => {
  //   const wxml = `<view class="cls1 {{test}} cls2"></view>`;
  //   const result = parse(wxml);
  //   const attributes = result.children[0].attributes as Attribute[];
  //   expect(attributes[0].value).toEqual([
  //     {
  //       type: "Text",
  //       content: "cls1 ",
  //       location: {
  //         start: { column: 14, line: 1, offset: 13 },
  //         end: { column: 19, line: 1, offset: 18 },
  //       },
  //     },
  //     {
  //       type: "Expression",
  //       content: "{{test}}",
  //       location: {
  //         start: { column: 19, line: 1, offset: 18 },
  //         end: { column: 27, line: 1, offset: 26 },
  //       },
  //     },
  //     {
  //       type: "Text",
  //       content: " cls2",
  //       location: {
  //         start: { column: 27, line: 1, offset: 26 },
  //         end: { column: 32, line: 1, offset: 31 },
  //       },
  //     },
  //   ]);
  // });
  // it("wxs", () => {
  //   const result = parse(`<wxs>console.log(a)</wxs>`);
  //   console.log(result);
  //   expect(result).toBeDefined();
  // });
});
