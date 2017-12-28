#version 330 core
layout (location = 0) in vec3 aPos;

// varying outputs
out vec2 tex;

void main()
{
	tex = aPos.xy * 0.5 + 0.5;
	gl_Position = vec4(aPos, 1.0f);
}
