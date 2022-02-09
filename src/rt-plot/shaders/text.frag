#version 330 core

out vec4 Color;

in vec4 vertexColor; 
in vec2 texCoord; 

uniform sampler2D textTexture;

void main() {
    Color = vec4(vertexColor.rgb, texture(textTexture, texCoord).r);
}

