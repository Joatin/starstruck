#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec4 color;
layout (location = 2) in vec2 uv;

layout (location = 0) out gl_PerVertex {
  vec4 gl_Position;
};
layout (location = 1) out vec4 frag_color;
layout (location = 2) out vec2 frag_uv;


void main() {
    gl_Position = vec4(position, 1.0);
    frag_color = color;
    frag_uv = uv;
}