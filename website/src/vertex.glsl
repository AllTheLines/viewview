#version 300 es
precision highp float;

in vec2 a_pos;
out vec2 v_texcoord;

uniform mat4 u_projectionMatrix;
uniform vec4 u_tileMatrix;
uniform float u_worldOffset;
uniform float u_scale;
uniform vec2 u_offset;

void main() {
  vec2 normalised_coord = a_pos / 4096.0;
  v_texcoord = normalised_coord / u_scale + u_offset;

  // Why??
  float magicScaler = 2.0;

  vec2 tileOrigin = u_tileMatrix.xy;
  vec2 tileSize = u_tileMatrix.zw * magicScaler;
  vec2 in_tile = a_pos;
  vec4 uv = vec4(tileOrigin + in_tile * tileSize, 0.0, 1.0);

  // Wrap the world infinitely along the x-axis.
  uv.x = uv.x + u_worldOffset;

  gl_Position = u_projectionMatrix * uv;
}
