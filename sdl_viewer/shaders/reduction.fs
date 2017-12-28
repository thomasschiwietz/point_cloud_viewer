#version 330 core

// inputs
in vec2 tex;

uniform sampler2D aTex;

layout(location = 0) out vec4 FragColor;

void main()
{
    // textureGather?
    float d = texture(aTex, tex.xy * 8.0).x;
    FragColor = vec4(d);
    //FragColor = vec4(1.0, 0.0, 1.0, 1.0);
}
