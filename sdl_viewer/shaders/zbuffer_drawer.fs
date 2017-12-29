#version 330 core

// inputs
in vec2 tex;

uniform sampler2D aTex;
uniform vec4 tex_scale;

layout(location = 0) out vec4 FragColor;

float linearize_depth(float depth)
{
    float zNear = 0.1;
    float zFar  = 75.0;
    return (2.0 * zNear) / (zFar + zNear - depth * (zFar - zNear));
}

void main()
{
    float d = texture(aTex, tex.xy * tex_scale.xy).x;
    FragColor = d < 1.0 ? vec4(1.0 - linearize_depth(d)) : vec4(0.3,0.3,1.0,1.0);   // infinite depth blue sky
}
