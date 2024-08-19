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
    
float u_bending_factor = 0.4;
float u_darkness = 0.5;
float u_num_lines = 240.0;
float u_horizontal_offset = 0.0;
uniform sampler2D u_image;
in vec2 uv;

out vec4 outColor;


void main() {
    vec2 nuv = uv - vec2(0.5, 0.5);
    vec2 offset = vec2(nuv.x * abs(nuv.x) * nuv.y * nuv.y * u_bending_factor, nuv.x * nuv.x * nuv.y * abs(nuv.y) * u_bending_factor);
    vec2 uvo = uv + offset;
        
    float tint = mix(1.0, u_darkness, smoothstep(0.0, 0.5, fract(uvo.y * u_num_lines)) - smoothstep(0.5, 1.0, fract(uvo.y * u_num_lines)));
	outColor = vec4(texture(u_image, uvo + vec2(u_horizontal_offset, 0.0)).bgr * tint, 1);

    if(uvo.x < u_horizontal_offset || uvo.y < 0.0 || uvo.x >= (1.0 - u_horizontal_offset) || uvo.y >= 1.0) {
		outColor = vec4(0.0, 0.0, 0.0, 1.0);
	}
}    
#endif