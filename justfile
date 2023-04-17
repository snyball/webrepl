build:
    mkdir -p .cargo
    echo "[unstable]" > .cargo/config.toml
    echo 'build-std = ["std", "panic_abort"]' >> .cargo/config.toml
    echo 'build-std-features = ["panic_immediate_abort"]' >> .cargo/config.toml
    trunk build --release
    rm .cargo/config.toml
    wasm-opt -Oz -o dist/webrepl-*.wasm dist/webrepl-*.wasm
    minify-js -m module dist/webrepl-*.js -o dist/webrepl-*.js
    mkdir -p releases
    tar cvfJ releases/webrepl-$(git describe)
