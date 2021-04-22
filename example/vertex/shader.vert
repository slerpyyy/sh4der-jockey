#version 140

out vec4 v_color;

uniform float time;

const float pi = acos(-1.0);

mat2 rot(float a)
{
  float s=sin(a), c=cos(a);
  return mat2(c,s,-s,c);
}

void main() {
  float a = 8. * pi * gl_VertexID / 1000.0;

  float r = 3.25;
  vec2 off = vec2(0.5 * sin(r*a) + 1., cos(r*a));
  vec3 p = vec3(sin(a), 1, cos(a)) * off.xyx;

  p.xz *= rot(time);
  p.yz *= rot(0.8);

  gl_Position = vec4(p/(p.z+3.), 1);
  gl_PointSize = 1.0 - abs(p.z);
  v_color = vec4(1./(p.z+2.));
}
