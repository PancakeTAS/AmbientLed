#version 330 core

out vec4 FragColor;

in vec2 TexCoord;

uniform sampler2D texture1;

void main() {
    vec3 color = texture(texture1, vec2(1.0 - TexCoord.x, TexCoord.y)).rgb;

	// correct brightness
	float brightness = color.r * 0.299 + color.g * 0.587 + color.b * 0.114;
	brightness = log2(brightness) / 5.0 + 1.0;

    // normalize
    float max_component = max(max(color.r, color.g), color.b);
    if (max_component > 0.0) {
        color /= max_component;
    }

	// apply brightness
	color *= brightness;

    FragColor = vec4(color, 1.0);
}