#ifdef VS

layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;

out vec2 uv;
    
void main() {
    uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}    
#endif

#ifdef FS

precision highp float;
    
float u_bending_factor = 0.5;
float u_darkness = 0.5;
float u_num_lines = 240.0;
uniform sampler2D u_image;
in vec2 uv;

out vec4 outColor;


void main() {
    vec2 nuv = uv - vec2(0.5, 0.5);
    float x = nuv.x * nuv.x;
    float y = nuv.y * abs(nuv.y);
    vec2 offset = vec2(0, x * y * u_bending_factor);
    vec2 uvo = uv + offset;
        
    float tint = mix(1.0, u_darkness, smoothstep(0.0, 0.5, fract(uvo.y * u_num_lines)) - smoothstep(0.5, 1.0, fract(uvo.y * u_num_lines)));
	outColor = vec4(texture(u_image, uvo).rgb * tint, 1);

    if(uvo.x < 0.0 || uvo.y < 0.0 || uvo.x >= 1.0 || uvo.y >= 1.0) {
		outColor = vec4(0.0, 0.0, 0.0, 1.0);
	}
}    
#endif