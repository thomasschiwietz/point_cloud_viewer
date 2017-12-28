#version 330 core

// inputs
in vec4 v_color;

// outputs
layout(location = 0) out vec4 FragColor;

precision mediump float;

void main() {
    FragColor = v_color;
}
