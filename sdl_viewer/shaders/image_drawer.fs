#version 330 core

// inputs
in vec2 tex;

uniform sampler2D aTex;

layout(location = 0) out vec4 FragColor;

void main()
{
    //FragColor = vec4(1.0, 1.0, 0.0, 0.0) * texture(aTex, tex.xy);
    FragColor = texture(aTex, tex.xy);
}
