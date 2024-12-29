#version 450

layout(location = 0) in vec3 frag_normal;
layout(location = 1) in vec2 frag_tex_coords;

layout(location = 0) out vec4 out_color;

layout(set = 1, binding = 0) uniform sampler2D texture_sampler;

void main() {
    vec3 light_dir = normalize(vec3(0.3, -0.8, -0.4));
    float diff = max(dot(normalize(frag_normal), -light_dir), 0.0);

    vec3 texture_color = texture(texture_sampler, frag_tex_coords).rgb;
    vec3 final_color = diff * texture_color;

    out_color = vec4(final_color, 1.0);
}
