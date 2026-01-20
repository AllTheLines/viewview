#version 300 es
precision highp float;
precision highp usampler2D;
precision highp int;

in vec2 v_texcoord;
uniform usampler2D u_data;
uniform float u_max;
uniform float u_averageSurfaceVisibility;
uniform float u_intensity;
uniform float u_contrast;

out vec4 fragColor;

void main() {
  vec3 final_color;
  float normalisation_factor = u_intensity;
  float tile_width = 256.0;

  uvec4 pixel = texelFetch(u_data, ivec2(v_texcoord * tile_width), 0);
  uint bits =
    (pixel.a << 24) |
      (pixel.b << 16) |
      (pixel.g << 8) |
      pixel.r;
  float value = uintBitsToFloat(bits);

  // Handle "nodata"
  if (value == 0.0) {
    value = u_averageSurfaceVisibility;
  }

  float normalized = value / u_max;
  float normalized_v = pow(normalized, normalisation_factor);

  vec3 color_0 = vec3(0.03137254901960784, 0.09019607843137255, 0.23137254901960785);
  vec3 color_1 = vec3(0.30980392156862746, 0.2784313725490196, 0.36470588235294116);
  vec3 color_2 = vec3(0.9921568627450981, 0.3803921568627451, 0.0392156862745098);
  vec3 color_3 = vec3(1.0, 1.0, 1.0);

  // Everything above this is considered "good" visibility.
  float upper = u_contrast;
  float middle = upper / 2.0;

  if (normalized_v < middle) {
    float t = normalized_v / middle;
    final_color = mix(color_0, color_1, t);
  } else if (normalized_v < upper) {
    float t = (normalized_v - middle) / (upper - middle);
    final_color = mix(color_1, color_2, t);
  } else {
    float t = (normalized_v - upper) / (1.0 - upper);
    final_color = mix(color_2, color_3, t);
  }

  fragColor = vec4(final_color, 1.0);
}
