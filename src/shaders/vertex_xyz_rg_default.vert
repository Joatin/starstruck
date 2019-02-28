#version 450

layout (push_constant) uniform PushConsts {
  mat4 mvp;
} push;

layout (location = 0) in vec3 position;
layout (location = 1) in vec2 uv;

layout (location = 0) out gl_PerVertex {
  vec4 gl_Position;
};
layout (location = 2) out vec2 frag_uv;

void main()
{
  gl_Position = push.mvp * vec4(position, 1.0);
  frag_uv = uv;
}