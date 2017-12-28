#version 330 core

// inputs
in vec2 tex;

layout(location = 0) out vec4 FragColor;

void main()
{
	FragColor = vec4(tex.xy,0.,1.);
}
