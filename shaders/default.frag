#version 330 core

// unlike the WS2812B/WS2812E/WS2812B 3535SMD shaders, this one doesn't correct any colors and only adjusts the brightness.
// depending on your LED strip, it might be a bit too dark when applying correction shaders, therefore this one here
// is meant to be used as a "raw" kind of shader.

in vec2 TexCoord;
out vec4 FragColor;

uniform sampler2D texture1;

vec3 norm(vec3 color) {
	float max_component = max(max(color.r, color.g), color.b);
	if (max_component > 0.0) {
		color /= max_component;
	}
	return color;
}

void main() {
	vec3 color = texture(texture1, TexCoord).rgb;
	vec3 norm_color = norm(color);

	float perceptual_brightness =
		color.r * 0.299 +
		color.g * 0.587 +
		color.b * 0.114;

	float logarithmic_brightness = log2(perceptual_brightness) / 5.0 + 1.0;
	norm_color *= logarithmic_brightness;

	FragColor = vec4(norm_color, 1.0);
}
