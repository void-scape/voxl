layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 auv;
layout (location = 3) in vec3 translation;

uniform mat4 proj, view;

out vec3 frag_position, frag_normal;
out vec2 uv;

void main() {
	mat4 model;
	model[0] = vec4(1.0, 0.0, 0.0, 0.0);
	model[1] = vec4(0.0, 1.0, 0.0, 0.0);
	model[2] = vec4(0.0, 0.0, 1.0, 0.0);
	model[3] = vec4(translation, 1.0);
	vec4 model_position = model * vec4(position, 1.0);
	gl_Position = proj * view * model_position;
	frag_position = vec3(model_position);
	frag_normal = mat3(transpose(inverse(model))) * normal;
	uv = auv;
}
