#version 330 core
layout (location = 0) in vec2 aPos;

// varying outputs
out vec2 tex;

void main()
{
	tex = aPos.xy * 0.5 + 0.5;
	gl_Position = vec4(aPos, 0.0f, 1.0f);
}
