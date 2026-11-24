# Testing

> This page describes the upstream PhotoCraft verification gates. In this fork, the scope of
> verification for any change is defined by the fork's [`agent.md`](https://github.com/huairuiLove/phcraft-comfy/blob/main/agent.md) §7: run only the minimal
> necessary verification for the change at hand; do not run full-workspace, corpus, `wasm`, or
> `panic_hunt` checks for ordinary platform-adaptation work, and do not add tests by default
> (§7.3). The gates below apply to upstream core development, or when `agent.md` explicitly
> requires them.

Tests are the gate for source changes. Choose the narrowest relevant package tests during development, then run the repository gates appropriate to the touched layers.

```sh
cargo test -p <package>
cargo clippy -p <package> --all-targets -- -D warnings
cargo xtask layers
```

For changes below or at L6, also run `cargo xtask wasm`. Command changes require the ignored adversarial test:

```sh
cargo test -p photocraft-engine --test panic_hunt -- --ignored
```

Format changes should include valid round trips, exact boundary values, truncation, inconsistent lengths, malformed structures, and allocation/decompression limit cases. Pixel behavior should be exercised at applicable 8-bit, 16-bit, and 32-bit depths.

UI changes need rendered evidence from the offscreen snapshot example or a control-channel screenshot. A successful build or HTTP response is not visual verification.

Documentation changes are verified with:

```sh
mdbook build book
```

Internal Markdown links and every `SUMMARY.md` destination should also be checked. The documentation workflow performs a clean mdBook build in CI.
