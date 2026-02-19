各种输入框的高度是40px，而图标按钮是36x36，有很多地方不匹配

  - [High] FFI response path leaks memory on every call
    In Slice::from_string, bytes are copied into a new allocation, but then the original
    String is intentionally leaked via std::mem::forget(s).
    Reference: crates/end_web/src/lib.rs:568, crates/end_web/src/lib.rs:581
    Why this matters: every end_web_bootstrap / end_web_solve_from_aic_toml call leaks one
    full JSON buffer (often large), so long-running sessions will grow memory unboundedly.
  - [High] Worker wasm URL fallback is wrong for production asset paths
    Non-dev branch returns ./wasm/, and both importScripts + locateFile use it. For built
    worker bundles under /assets/..., this resolves to /assets/wasm/..., but wasm is copied
    to /wasm/... from web/public/wasm.
    References: web/src/lib/solve.worker.ts:24, web/src/lib/solve.worker.ts:144, web/src/
    lib/solve.worker.ts:167, scripts/build_web_wasm.sh:6
    Why this matters: solver worker likely fails to load wasm in production build.
  - [Medium] e2e wasm shim still mocks removed C-string ABI
    The smoke shim still handles end_web_free_c_string and UTF8ToString, while runtime now
    depends on malloc/free, HEAPU8/HEAPU32, and end_web_free_slice.
    References: web/e2e/smoke.spec.ts:79, web/e2e/smoke.spec.ts:95, web/src/lib/wasm-
    core.ts:23, web/src/lib/wasm-core.ts:106
    Why this matters: smoke test no longer validates real wasm integration behavior; once
    browser setup issue is fixed, this test is likely to fail or give misleading coverage.

物流图的折叠和化简