import esbuild from "esbuild"
import http from 'node:http'

const ctx = await esbuild.context({
    entryPoints: ["src/index.ts", "src/nes-audio-processor.ts"],
    bundle: true,
    outdir: "www",
    sourcemap: true,
    logLevel: "info",
    loader: {
        ".wasm": "file",
        ".glsl": "text"
    }
});

let { host, port } = await ctx.serve({
    servedir: "www",
});

// Then start a proxy server on port 3000
const server = http.createServer((req, res) => {
    const options = {
        hostname: host,
        port: port,
        path: req.url,
        method: req.method,
        headers: req.headers,
    }

    // Forward each incoming request to esbuild
    const proxyReq = http.request(options, proxyRes => {
        // If esbuild returns "not found", send a custom 404 page
        if (proxyRes.statusCode === 404) {
            res.writeHead(404, { 'Content-Type': 'text/html' })
            res.end('<h1>A custom 404 page</h1>')
            return;
        }

        // Otherwise, forward the response from esbuild to the client
        res.writeHead(proxyRes.statusCode, {
            ...proxyRes.headers,
            "Cross-Origin-Embedder-Policy": "require-corp",
            "Cross-Origin-Opener-Policy": "same-origin",
        });
        proxyRes.pipe(res, { end: true });
    })

    // Forward the body of the request to esbuild
    req.pipe(proxyReq, { end: true })
}).listen(3000);


await ctx.watch();