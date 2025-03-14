import { parse } from './index.js';

const wxmlContent = `<view class="container" bindtap="{{handleTap}}">
  <!-- 注释测试 -->
  <text class="asdf {{show}} {{isActive ? 'active' : ''}} lest">Hello {{name}}</text>
  <text wx:if="{{show}}">{{m1.foo()}}</text> 
  <text>Hello {{name}}</text>
  <wxs module="m2" />
  <wxs module="m1">
    var msg = "Hello World";
    module.exports.message = msg;
  </wxs>
  <button wx:if="{{ show }}" disabled></button>
  <view class="asdf {{ show }} {{ isActive ? 'active' : '' }} world" data-value="test-{{index}}-item">测试内容</view>
  <view class="hello {{ isActive ? 'active' : '' }} world" data-value="test-{{index}}-item">测试内容</view>
  <button disabled="{{!canClick}}" class="btn {{type}} {{loading ? 'loading' : ''}}">点击按钮</button>
  <text wx:if="{{show}}">{{m1.foo()}}</text> 
</view>`;
const start = Date.now();
for (let i = 0; i < 1000; i++) {
  parse(wxmlContent)
  // console.log(parse(wxmlContent));
}
const end = Date.now();
console.log(`Time taken: ${end - start}ms`);
