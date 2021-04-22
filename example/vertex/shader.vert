#define TAU 6.28318530718

uniform float time;

mat2 rot(float a)
{
  float s=sin(a), c=cos(a);
  return mat2(c,s,-s,c);
}

void main() {
  float a = 4. * TAU * gl_VertexID / 1000.0;

  float r = 3.25;
  vec2 off = vec2(0.5 * sin(r*a) + 1., cos(r*a));
  vec3 p = vec3(sin(a), 1, cos(a)) * off.xyx;

  p.xz *= rot(time);
  p.yz *= rot(0.8);

  gl_Position = vec4(p/(p.z+3.), 1);
  //v_color = vec4(1./(p.z+2.));
}
