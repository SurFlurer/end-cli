可以先用这一组参数，实测是你这个仓库里收益最大且不改代码的方案。

```
source "/home/xks/apps/emsdk/emsdk_env.sh" && \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_LTO=true \
RUSTFLAGS='-Copt-level=z -Cstrip=symbols -Clink-arg=-sASSERTIONS=0 -Clink-arg=-sMALLOC=emmalloc
-Clink-arg=-sINCOMING_MODULE_JS_API=[]' \
cargo build --target wasm32-unknown-emscripten --release --config 'patch.crates-io.highs-
sys.path="../highs-sys"'
```

然后做一遍后处理（进一步压缩 wasm）：

```
/home/xks/apps/emsdk/upstream/bin/wasm-opt -Oz --all-features \
  target/wasm32-unknown-emscripten/release/end_cli.wasm \
  -o target/wasm32-unknown-emscripten/release/end_cli.min.wasm
```

我在你当前工程上的实测结果（target/wasm32-unknown-emscripten/release/end_cli.wasm）：

1. 基线：4,704,990 bytes
2. 上面这组构建参数后：4,244,291 bytes（-9.79%）
3. 再跑 wasm-opt -Oz：4,195,096 bytes（相对基线 -10.84%）

end-cli.js 也从 91,500 降到 88,772（小幅下降）。

补充两个关键点：

1. -Cpanic=abort 在这个 target 上会和预编译标准库冲突（会报 panic strategy 不兼容），不要直接用。
2. -Clto=... 放在 RUSTFLAGS 里会触发 embed-bitcode 冲突；用 CARGO_PROFILE_RELEASE_LTO=true（或
   Cargo.toml 的 [profile.release] lto=true）是可行的。

如果你不需要文件系统（纯内存/纯函数调用场景），再加 -Clink-arg=-sFILESYSTEM=0 能大幅缩小 end-
cli.js，但会影响依赖 cwd/文件 IO 的行为。