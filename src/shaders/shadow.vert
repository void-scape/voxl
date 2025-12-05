layout (location = 0) in vec3 position;
layout (location = 1) in vec3 translation;

uniform mat4 light_space;

void main() {
	mat4 model;
	model[0] = vec4(1.0, 0.0, 0.0, 0.0);
	model[1] = vec4(0.0, 1.0, 0.0, 0.0);
	model[2] = vec4(0.0, 0.0, 1.0, 0.0);
	model[3] = vec4(translation, 1.0);
	gl_Position = light_space * model * vec4(position, 1.0);
}
