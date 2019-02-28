#version 450

layout (push_constant) uniform PushConsts {
  mat4 mvp;
} push;

layout (location = 0) in vec3 position;

layout (location = 0) out gl_PerVertex {
  vec4 gl_Position;
};

void main()
{
  gl_Position = push.mvp * vec4(position, 1.0);
}