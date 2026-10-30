#define IS_NAN(v) ((v) != (v))

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

uniform sampler2D tex;

in vec2 frag_tex_coord;
in vec4 frag_blend;
flat in vec4 frag_blend_flat;
in float fog_z;

out vec4 out_colour;

void main() {
    vec4 tex_col;
    if (frag_tex_coord.x < -0.5) {
        tex_col = vec4(1.0);
    } else {
        tex_col = texture(tex, frag_tex_coord);
    }
    vec4 colour = tex_col * frag_blend * frag_blend_flat;
    // apply fog
    if (fog_enabled) {
        float f = clamp((fog_end - fog_z) / (fog_end - fog_begin), 0.0, 1.0);
        colour.rgb = mix(fog_colour.rgb, colour.rgb, f);
    }
    // Alpha test (PowerVR Rogue HSR safe: strictly discard only when alpha_test is active and colour.a <= 0.0)
    if (alpha_test && colour.a <= 0.0) {
        discard;
    }
    out_colour = colour;
}
