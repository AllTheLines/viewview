precision highp float;
varying vec2 v_texcoord;
uniform sampler2D u_data;
uniform float u_max;
void main() {
    float value = texture2D(u_data, v_texcoord).r;

    float normalized = value / u_max;
    float normalized_v = pow(normalized, 0.5);

    vec3 color_min = vec3(0.0, 0.0, 0.0);
    vec3 color_mid = vec3(0.5, 0.5, 0.5);
    vec3 color_max = vec3(1.0, 1.0, 1.0);

    vec3 final_color;

    if (normalized_v < 0.5) {
        float half_normalized = normalized_v / 0.5;
        final_color = mix(color_min, color_mid, half_normalized);
    } else {
        float half_normalized = (normalized_v - 0.5) / 0.5;
        final_color = mix(color_mid, color_max, half_normalized);
    }

    float alpha = 1.0;
    if (normalized <= 0.0) {
        alpha = 0.0;
    }
    gl_FragColor = vec4(final_color, alpha);
}
