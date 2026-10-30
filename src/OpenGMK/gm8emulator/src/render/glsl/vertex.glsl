struct Light {
    vec4 pos; // padded vec3
    vec4 colour; // padded vec3
    bool enabled;
    bool is_point;
    float range;
};

layout(std140) uniform RenderState {
    // vertex shader
    mat4 model;
    mat4 viewproj;
    Light lights[8];
    vec4 ambient_colour; // padded vec3
    bool lighting_enabled;
    bool gouraud_shading;
    bool colour_blending;
    // frag shader
    bool repeat;
    bool lerp; // only used if repeat is on
    bool alpha_test;
    bool fog_enabled;
    float fog_begin;
    float fog_end;
    vec4 fog_colour; // padded vec3
};

uniform bool gm81_normalize; // this will never change so we don't need to include it in the state
uniform float interp_amount;

layout (location = 0) in vec2 in_pos;
layout (location = 1) in vec4 in_color;
layout (location = 2) in vec2 in_uv;

out vec2 frag_tex_coord;
out vec4 frag_blend;
flat out vec4 frag_blend_flat;
out float fog_z;

void main() {
    vec4 world_pos = model * vec4(in_pos, 0.0, 1.0);
    frag_blend_flat = vec4(1.0);

    if (in_uv.x < -0.5) {
        frag_tex_coord = in_uv;
        frag_blend = in_color;
    } else {
        frag_tex_coord = in_uv;
        frag_blend = colour_blending ? in_color : vec4(1.0);
    }

    if (lighting_enabled) {
        vec3 light_col = vec3(0.0);
        vec3 new_normal = -(model * vec4(0.0, 0.0, 1.0, 0.0)).xyz;
        if (gm81_normalize) {
            new_normal = normalize(new_normal);
        }
        for (int i = 0; i < 8; i++) {
            if (lights[i].enabled) {
                vec3 this_light_col = lights[i].colour.rgb;
                vec3 ray = lights[i].pos.xyz;
                if (lights[i].is_point) {
                    ray = world_pos.xyz - ray;
                    float dist = length(ray);
                    if (dist < lights[i].range) {
                        this_light_col /= 1.0 + (4.0 / lights[i].range) * dist;
                    } else {
                        this_light_col = vec3(0.0);
                    }
                }
                light_col += this_light_col * clamp(dot(normalize(ray), new_normal), 0.0, 1.0);
            }
        }
        if (gouraud_shading) {
            frag_blend.rgb *= light_col;
            frag_blend.rgb += ambient_colour.rgb;
        } else {
            frag_blend_flat.rgb *= light_col;
            frag_blend_flat.rgb += ambient_colour.rgb;
        }
    }
    gl_Position = viewproj * world_pos;
    fog_z = gl_Position.z;
}
