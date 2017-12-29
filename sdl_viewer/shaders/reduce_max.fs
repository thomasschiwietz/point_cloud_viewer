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

    float d = texture(aTex, tex.xy + 100 * step.xy).x;
    FragColor = vec4(d);
}
