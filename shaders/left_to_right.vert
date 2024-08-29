#version 330 core

// simple vertex shader for led strips strting at the left side of the screen
// and moving towards the right side. can also be used for top to bottom.

in vec3 Pos;
in vec2 ImageTexCoord;
out vec2 TexCoord;

void main() {
	gl_Position = vec4(Pos, 1.0);
	TexCoord = ImageTexCoord;
}
