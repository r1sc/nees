del www\*.js
del www\*.wasm
del www\*.map
wasm-pack build --release --target web
npm run build-release