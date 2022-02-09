#version 330 core

out vec4 Color;

in vec4 vertexColor; 

void main() {
    Color = vertexColor;
}
