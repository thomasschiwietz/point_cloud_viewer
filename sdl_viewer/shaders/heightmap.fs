#version 330 core

uniform vec4 color;

in vec3 normal;

layout(location = 0) out vec4 FragColor;

void main()
{
    vec3 lightDir = vec3(0,0,1);
    float diff = max(dot(normal, lightDir), 0.0);
    float ambient = 0.5;
    FragColor = color * (ambient + (1. - ambient) * diff);
}
