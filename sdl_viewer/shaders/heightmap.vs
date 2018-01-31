#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;

uniform mat4 transform;
uniform mat4 modelViewTransform;

out vec3 normal;

void main()
{
	gl_Position = transform * vec4(aPos, 1.0f);
	normal = mat3(transpose(inverse(modelViewTransform))) * aNormal;		// normal in view space
}
