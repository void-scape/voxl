uniform sampler2D texture_atlas;
uniform sampler2D shadow_map;

uniform vec3 light_source;
uniform vec3 light_color;
uniform float ambient_brightness;
uniform float fog_near, fog_far;

in VS_OUT {
    vec3 position;
	vec3 view_position;
    vec4 light_space_position;
    vec3 normal;
    vec2 uv;
} fs_in;

out vec4 c;

// https://learnopengl.com/Advanced-Lighting/Shadows/Shadow-Mapping
float shadow_calculation(vec4 frag_pos_light_space, vec3 normal, vec3 light_dir) {
	vec3 proj_coords = frag_pos_light_space.xyz / frag_pos_light_space.w;
	proj_coords = proj_coords * 0.5 + 0.5;
	float depth = proj_coords.z;
	float closest_depth = texture(shadow_map, proj_coords.xy).r;
	float bias = max(0.05 * (1.0 - dot(normal, light_dir)), 0.005);
	return depth - bias > closest_depth ? 1.0 : 0.0;
}

void main() {
	vec3 norm = normalize(fs_in.normal);
	vec3 light_dir = normalize(light_source - fs_in.position);  
	float diff = max(dot(norm, light_dir), 0.0);
	vec3 diffuse = diff * light_color;

	vec4 sample = texture(texture_atlas, fs_in.uv);
	float shadow = shadow_calculation(fs_in.light_space_position, norm, light_dir);
	vec3 lighting = (ambient_brightness + (1.0 - shadow) * diffuse) * vec3(sample);
    c = vec4(lighting, sample.w);
	c *= 1.0 - smoothstep(fog_near, fog_far, length(fs_in.view_position));
} 
