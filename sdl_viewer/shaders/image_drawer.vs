#version 330 core
layout (location = 0) in vec2 aPos;

// varying outputs
out vec2 tex;

uniform mat4 matrix;

void main()
{
	tex = aPos.xy * 0.5 + 0.5;
	gl_Position = matrix * vec4(aPos, 0.9910f, 1.0f);
}
