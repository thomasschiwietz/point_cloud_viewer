#version 330 core

// inputs
in vec2 tex;

uniform sampler2D aTex;
uniform vec4 step;

layout(location = 0) out vec4 FragColor;

void main()
{
    // unknown on MacOS
    //vec4 closest_depths = textureGather(aTex, tex.xy);

    // gather 4
    float d_00 = texture(aTex, tex.xy * step.z + vec2(0.0, 0.0) * step.xy).x;
    float d_01 = texture(aTex, tex.xy * step.z + vec2(0.0, 1.0) * step.xy).x;
    float d_11 = texture(aTex, tex.xy * step.z + vec2(1.0, 1.0) * step.xy).x;
    float d_10 = texture(aTex, tex.xy * step.z + vec2(1.0, 0.0) * step.xy).x;

    // max
    float d = max(max(d_00, d_01), max(d_11, d_10));

    FragColor = vec4(d);
}
