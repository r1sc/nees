import esbuild from "esbuild"

const ctx = await esbuild.context({
    entryPoints: ["src/index.ts", "src/nes-audio-processor.ts"],
    bundle: true,
    outdir: "www",
    sourcemap: true,
    logLevel: "info",
    loader: {
        ".wasm": "file"
    }
});

await ctx.serve({
    servedir: "www"
});
await ctx.watch();