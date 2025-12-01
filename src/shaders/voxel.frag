uniform sampler2D texture_atlas;

uniform float ambient_brightness;
uniform vec3 light_source;

in vec3 frag_position, frag_normal;
in vec2 uv;
out vec4 c;

void main() {
	float specular_strength = 0.75;
	vec3 light_color = vec3(1.0);

	// diffuse
	vec3 norm = normalize(frag_normal);
	vec3 light_dir = normalize(light_source - frag_position);  
	float diff = max(dot(norm, light_dir), 0.0);
	vec3 diffuse = diff * light_color;

	// specular
	// vec3 view_dir = normalize(camera_position - frag_position);
	// vec3 reflect_dir = reflect(-light_dir, norm); 
	// float spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
	// vec3 specular = specular_strength * spec * light_color;  

	vec3 white = vec3(1, 1, 1);
	// vec3 result = (ambient_brightness + diffuse + specular) * white;
	vec3 result = (ambient_brightness + diffuse) * white;
    c = vec4(result, 1.0);
// 	c = vec4(vec3(texture(texture_atlas, uv)) * color, 1.0);
} 
