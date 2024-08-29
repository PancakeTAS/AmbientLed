#version 330 core

// vertex shader just like left_to_right.vert.glsl, but for right to left
// or bottom to top led strips.

in vec3 Pos;
in vec2 ImageTexCoord;
out vec2 TexCoord;

void main() {
	gl_Position = vec4(Pos, 1.0);
	TexCoord = vec2(1.0 - ImageTexCoord.x, ImageTexCoord.y);
}
