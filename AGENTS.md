## Lint 和测试

修改rust代码后跑，没改rust代码不用跑：`cargo make done`
修改前端代码后在`web`目录下跑：`npm run check` 检查类型和 `npm run test:console` 检查console error。

## 代码风格

- 充分开发类型系统潜力，parse, don't validate。调用处、业务代码禁止 `allow`/`panic`/`expect`/`unsafe`，类型内部可 `allow`/`panic`/`expect`/`unsafe`。
- 业务代码的错误处理尽量移动到 IO、model 层等。禁止防御编程，使用类型系统代替，关键路径消除所有死代码或 no-op 代码。
- 模块里的类型的关系类似C++友元。如果类型依赖私有成员、方法维护不变量，必须最小化能访问到这些成员、方法的类型集合。公开所有成员、方法的平凡的类型允许和别的类型放一起。