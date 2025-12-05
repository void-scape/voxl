layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec2 atlas_index;
layout (location = 4) in vec3 translation;

uniform mat4 proj, view, light_space;
uniform vec2 atlas_size, texture_size;

out VS_OUT {
    vec3 position;
	vec3 view_position;
    vec4 light_space_position;
    vec3 normal;
    vec2 uv;
} vs_out;

void main() {
	mat4 model;
	model[0] = vec4(1.0, 0.0, 0.0, 0.0);
	model[1] = vec4(0.0, 1.0, 0.0, 0.0);
	model[2] = vec4(0.0, 0.0, 1.0, 0.0);
	model[3] = vec4(translation, 1.0);

	vs_out.position = vec3(model * vec4(position, 1.0));
	vs_out.view_position = vec3(view * vec4(vs_out.position, 1.0));
	vs_out.light_space_position = light_space * vec4(vs_out.position, 1.0);
	vs_out.normal = normal; //mat3(transpose(inverse(model))) * normal;
	vs_out.uv = (atlas_index + uv) * (texture_size / atlas_size);
	gl_Position = proj * view * vec4(vs_out.position, 1.0);
}
