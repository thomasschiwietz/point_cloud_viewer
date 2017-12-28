#version 330 core

// inputs
in vec2 tex;

uniform sampler2D aTex;

layout(location = 0) out vec4 FragColor;

void main()
{
	float d = texture(aTex, tex.xy).x;
	d = clamp(d - 0.99, 0.0, 1.0) * 100.0;
	FragColor = vec4(d);
}
