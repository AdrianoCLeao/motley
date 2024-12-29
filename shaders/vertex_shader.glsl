#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 tex_coords;

layout(location = 0) out vec3 frag_normal;
layout(location = 1) out vec2 frag_tex_coords;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 model;
    mat4 view_proj;
};

void main() {
    gl_Position = view_proj * model * vec4(position, 1.0);
    frag_normal = mat3(transpose(inverse(model))) * normal;
    frag_tex_coords = tex_coords;
}
