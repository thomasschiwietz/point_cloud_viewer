#version 330 core

// inputs
in vec2 tex;
uniform sampler2D aTex;
uniform vec4 size_step;

// outputs
layout(location = 0) out vec4 FragColor;

void main()
{
    // unknown on MacOS
    // vec4 closest_depths = textureGather(aTex, tex.xy);

    // separate uniform parameter
    vec2 src_size = size_step.xy;
    vec2 src_step = size_step.zw;
    
    // integer texture coordinates into the source texture
    vec2 src_tex = tex * src_size - vec2(0.5, 0.5);

    // 4 nearest texels in source texture
    vec2 src_tex_00 = (src_tex + vec2( 0.0,  0.0)) * src_step;
    vec2 src_tex_01 = (src_tex + vec2( 0.0,  1.0)) * src_step;
    vec2 src_tex_11 = (src_tex + vec2( 1.0,  1.0)) * src_step;
    vec2 src_tex_10 = (src_tex + vec2( 1.0,  0.0)) * src_step;

    // clamp texture coordinates because texture coordinates might go out sub region of the texture
    // in power of two texture access is never outside of the sub region

    // gather 4
    float d_00 = texture(aTex, src_tex_00).x;
    float d_01 = texture(aTex, src_tex_01).x;
    float d_11 = texture(aTex, src_tex_11).x;
    float d_10 = texture(aTex, src_tex_10).x;

    // maximum of all values
    float d = max(max(d_00, d_11), max(d_01, d_10));

    FragColor = vec4(d);
}
