import { describe, it, expect } from "vitest";
import { parse } from "../";

describe("parse", () => {
  it("toBeDefined", () => {
    const result = parse(`<text>Hello {{name}}</text>`);
    // console.log(result);
    expect(result).toBeDefined();
  });
  it("复杂的", () => {
    const wxml = `<view class="container" bindtap="{{handleTap}}"></view>`;
    const result = parse(wxml);
    console.log(result.children[0]);
    expect(result).toBeDefined();
  });
});
