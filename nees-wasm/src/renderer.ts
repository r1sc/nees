
function create_shader(gl: WebGL2RenderingContext, program_src: string): WebGLProgram {
    function load_shader(src: string, type: "vertex" | "fragment") {
        const shader = gl.createShader(type === "vertex" ? gl.VERTEX_SHADER : gl.FRAGMENT_SHADER);
        if (!shader) {
            throw new Error("Couldn't create shader");
        }
        gl.shaderSource(shader, src);
        gl.compileShader(shader);

        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            const compile_info = gl.getShaderInfoLog(shader);
            throw new Error(`Shader compile error: ${compile_info}`);
        }
        return shader;
    }

    const gl_program = gl.createProgram();
    if (!gl_program) {
        throw new Error("Couldn't create shader program");
    }

    const vs = load_shader("#version 300 es\n#define VS\n" + program_src, "vertex");
    gl.attachShader(gl_program, vs);

    const fs = load_shader("#version 300 es\n#define FS\n" + program_src, "fragment");
    gl.attachShader(gl_program, fs);

    gl.linkProgram(gl_program);
    if (!gl.getProgramParameter(gl_program, gl.LINK_STATUS)) {
        const compile_info = gl.getProgramInfoLog(gl_program);
        throw new Error(`Program link error: ${compile_info}`);
    }

    return gl_program;
}

export function make_renderer() {
    const canvas = document.createElement("canvas");
    document.body.append(canvas);
    const gl = canvas.getContext("webgl2");

    if (!gl)
        throw new Error("No webgl2");

    const aspect = 256 / 240;
    const resize = window.onresize = () => {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;

        let width = canvas.width;
        let height = width / aspect;

        if (height > canvas.height) {
            height = canvas.height;
            width = height * aspect;
        }
        gl.viewport((canvas.width - width) / 2, (canvas.height - height) / 2, width, height);
    };
    resize();

    const glsl = (x: any) => x;

    const shader_src = glsl`
    #ifdef VS

    layout(location = 0) in vec2 a_position;
    layout(location = 1) in vec2 a_texcoord_0;
    
    out vec2 v_uv;
    
    void main() {
        gl_Position = vec4(a_position, 0.0, 1.0);
        v_uv = a_texcoord_0;
    }
    
    #endif

    #ifdef FS
    precision highp float;
    
    float u_bending_factor = 0.5;
    float u_darkness = 0.5;
    float u_num_lines = 240.0;
    uniform sampler2D u_image;
    in vec2 v_uv;

    out vec4 outColor;


    void main() {
        vec2 nuv = v_uv - vec2(0.5, 0.5);
        float x = nuv.x * nuv.x;
        float y = nuv.y * abs(nuv.y);
        vec2 offset = vec2(0, x * y * u_bending_factor);
        vec2 uvo = v_uv + offset;
        
        float tint = mix(1.0, u_darkness, smoothstep(0.0, 0.5, fract(uvo.y * u_num_lines)) - smoothstep(0.5, 1.0, fract(uvo.y * u_num_lines)));
        
        outColor = vec4(texture(u_image, uvo).bgr * tint, 1);
    }    
    #endif
    `;

    const program = create_shader(gl, shader_src);
    gl.useProgram(program);

    const u_image = gl.getUniformLocation(program, "u_image");
    gl.uniform1i(u_image, 0);

    const vertices_texcoords = [
        -1, -1, 0.0, 1.0,
        -1, 1, 0.0, 0.0,
        1, 1, 1.0, 0.0,
        1, -1, 1.0, 1.0
    ];
    const vertex_buffer = gl.createBuffer();
    if (!vertex_buffer) throw new Error("Failed to create vertex buffer");
    gl.bindBuffer(gl.ARRAY_BUFFER, vertex_buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(vertices_texcoords), gl.STATIC_DRAW);

    const indices = [
        0, 1, 2,
        0, 2, 3
    ];
    const index_buffer = gl.createBuffer();
    if (!index_buffer) throw new Error("Failed to create vertex buffer");
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, index_buffer);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint8Array(indices), gl.STATIC_DRAW);

    const vao = gl.createVertexArray();
    if (!vao) throw new Error("Failed to create VAO");

    gl.bindVertexArray(vao);
    gl.enableVertexAttribArray(0);
    gl.bindBuffer(gl.ARRAY_BUFFER, vertex_buffer);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 16, 0);

    gl.enableVertexAttribArray(1);
    gl.bindBuffer(gl.ARRAY_BUFFER, vertex_buffer);
    gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 16, 8);

    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, index_buffer);

    const texture = gl.createTexture();
    if (!texture)
        throw new Error("Failed to create texture");
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 256, 240, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);

    return {
        draw: (pixeldata: Uint8Array) => {
            gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 256, 240, gl.RGBA, gl.UNSIGNED_BYTE, pixeldata);
            gl.drawElements(gl.TRIANGLES, 6, gl.UNSIGNED_BYTE, 0);
        },
        gl
    }
};