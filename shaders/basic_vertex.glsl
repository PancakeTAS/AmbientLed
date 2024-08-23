#version 330

// this example vertex shader simply passes the vertex positions and texture coordinates to the fragment shader
// it does not apply any transformations to the vertex coordinates

layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;

out vec2 TexCoord;

void main() {
    gl_Position = vec4(aPos, 1.0);
    TexCoord = vec2(aTexCoord.x, aTexCoord.y);
}
