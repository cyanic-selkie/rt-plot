#version 330 core

layout (location = 0) in vec2 Position;
layout (location = 1) in vec4 Color;

out vec4 vertexColor;

uniform mat3 coordinate_transform;
uniform vec2 translation;

void main() {
    gl_Position = vec4(coordinate_transform * vec3(Position + translation, 1.0), 1.0);
    vertexColor = Color;
}
