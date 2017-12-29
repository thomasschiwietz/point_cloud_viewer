#version 330 core

// inputs
in vec2 tex;
uniform sampler2D aTex;
uniform vec4 step_scale;    // x,y: step size in x/y direction; z: source texture scale; w: unused

// outputs
layout(location = 0) out vec4 FragColor;

void main()
{
    // unknown on MacOS
    //vec4 closest_depths = textureGather(aTex, tex.xy);

    // separate uniform parameter
    vec2 src_size = step_scale.xy;
    vec2 tex_step = step_scale.zw;
    
    // integer texture coordinates into the source texture
    vec2 src_tex = tex * src_size;

    // 4 nearest texels in source texture
    //vec2 src_tex_c = (src_tex + vec2(0.0, 0.0)) * tex_step;
    //vec2 src_tex_00 = (src_tex + vec2(-1.0, -1.0)) * tex_step;
    //vec2 src_tex_01 = (src_tex + vec2(-1.0, 1.0)) * tex_step;
    //vec2 src_tex_11 = (src_tex + vec2(1.0, 1.0)) * tex_step;
    //vec2 src_tex_10 = (src_tex + vec2(1.0, -1.0)) * tex_step;

    // 4 nearest texels in source texture
    vec2 src_tex_c = (src_tex + vec2( 0.0,  0.0)) * tex_step;
    vec2 src_tex_l = (src_tex + vec2(-1.0,  0.0)) * tex_step;
    vec2 src_tex_r = (src_tex + vec2( 1.0,  0.0)) * tex_step;
    vec2 src_tex_t = (src_tex + vec2( 0.0,  1.0)) * tex_step;
    vec2 src_tex_b = (src_tex + vec2( 0.0, -1.0)) * tex_step;

    // clamp texture coordinates because texture coordinates might go out sub region of the texture
    //vec2 min_tex = vec2(0);
    //vec2 max_tex = vec2(src_scale);
    //src_tex_00 = clamp(src_tex_00, min_tex, max_tex);
    //src_tex_01 = clamp(src_tex_01, min_tex, max_tex);
    //src_tex_11 = clamp(src_tex_11, min_tex, max_tex);
    //src_tex_10 = clamp(src_tex_10, min_tex, max_tex);

    // gather 4
    float d_c = texture(aTex, src_tex_c).x;
    float d_l = texture(aTex, src_tex_l).x;
    float d_r = texture(aTex, src_tex_r).x;
    float d_t = texture(aTex, src_tex_t).x;
    float d_b = texture(aTex, src_tex_b).x;

    // maximum of all values
    float d = max(d_c, max(max(d_l, d_r), max(d_t, d_b)));

    FragColor = vec4(d);
}
