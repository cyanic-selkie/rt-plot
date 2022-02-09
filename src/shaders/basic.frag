#version 330 core

out vec4 Color;

in vec4 vertexColor; 

void main()
{
    Color = vertexColor;
    //Color = vec4(vertexColor[0], vertexColor[1], 0, 0.5);
}
