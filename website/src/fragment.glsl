#version 300 es
precision highp float;
precision highp usampler2D;

in vec2 v_texcoord;
uniform usampler2D u_data;
uniform float u_max;
uniform float u_averageSurfaceVisibility;

out vec4 fragColor;

void main() {
    vec3 final_color;
    float tile_width = 256.0;
    float normalisation_factor = 0.4;

    uvec2 rg = texelFetch(u_data, ivec2(v_texcoord * tile_width), 0).rg;
    uint bits = (rg.g << 16) | rg.r;
    float value = uintBitsToFloat(bits);

    // Handle "nodata"
    if (value == 0.0) {
        value = u_averageSurfaceVisibility;
    }

    float normalized = value / u_max;
    float normalized_v = pow(normalized, normalisation_factor);

    vec3 color_min = vec3(0.0, 0.0, 0.0);
    vec3 color_mid = vec3(0.5, 0.5, 0.5);
    vec3 color_max = vec3(1.0, 1.0, 1.0);

    if (normalized_v < 0.5) {
        float half_normalized = normalized_v / 0.5;
        final_color = mix(color_min, color_mid, half_normalized);
    } else {
        float half_normalized = (normalized_v - 0.5) / 0.5;
        final_color = mix(color_mid, color_max, half_normalized);
    }

    fragColor = vec4(final_color, 1.0);
}
