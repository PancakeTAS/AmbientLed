#version 330 core

// this shader showcases how the time uniform can
// be used to create a fancy rainbow cycle effect

in vec2 TexCoord;
out vec4 FragColor;

uniform float time;

void main() {
    vec3 color = vec3(0.5 + 0.5 * sin(time + TexCoord.x), 0.5 + 0.5 * cos(time + TexCoord.x), 0.5 + 0.5 * sin(time + TexCoord.x + 3.14));
    FragColor = vec4(color, 1.0);
}