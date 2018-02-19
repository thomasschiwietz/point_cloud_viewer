#version 330 core

// inputs
in vec2 tex;

uniform sampler2D aTex;

layout(location = 0) out vec4 FragColor;

void main()
{
    FragColor = texture(aTex, tex.xy);
}
