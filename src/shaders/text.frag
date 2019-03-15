#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(set = 0, binding = 0) uniform texture2D tex;
layout(set = 0, binding = 1) uniform sampler samp;

layout (location = 1) in vec4 frag_color;
layout (location = 2) in vec2 frag_uv;

layout(location = 0) out vec4 target;

void main() {
    float alpha = texture(sampler2D(tex, samp), frag_uv).r;
    if (alpha <= 0.0) {
        discard;
    }
    target = frag_color * vec4(1.0, 1.0, 1.0, alpha);
}