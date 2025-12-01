uniform sampler2D texture_atlas;

uniform float ambient_brightness;
uniform vec3 light_source;
uniform float fog_near, fog_far;

in vec3 frag_view_position, frag_position, frag_normal;
in vec2 uv;
out vec4 c;

void main() {
	float specular_strength = 0.75;
	vec3 light_color = vec3(1.0);

	vec3 norm = normalize(frag_normal);
	vec3 light_dir = normalize(light_source - frag_position);  
	float diff = max(dot(norm, light_dir), 0.0);
	vec3 diffuse = diff * light_color;

	vec3 white = vec3(1, 1, 1);
	vec4 sample = texture(texture_atlas, uv);
	vec3 result = (ambient_brightness + diffuse) * vec3(sample);
    c = vec4(result, sample.w);
	c *= 1.0 - smoothstep(fog_near, fog_far, length(frag_view_position));
} 
