#version 330 core

// this shader is for (my) WS2812E led strip.
// due to the nature of the WS2812E LEDs, the colors are far too blue.
// my own monitor is calibrated to sRGB at 400 nits, so I tried correcting the colors to sRGB.
// ontop of that, this shader also adjusts the brightness from a linear scale to a logarithmic scale.
// (-10% brightness is applied)

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
	color.g *= 0.69; // adjust green and blue to closer match sRGB
	color.b *= 0.31;
	vec3 norm_color = norm(color);

	float perceptual_brightness =
		color.r * 0.299 +
		color.g * (0.587 / 0.69) + // perceptual brightness is unaffected by the color correction
		color.b * (0.114 / 0.31);

	float logarithmic_brightness = log2(perceptual_brightness) / 5.0 + 1.0;
	norm_color *= logarithmic_brightness * 0.9;

	FragColor = vec4(norm_color, 1.0);
}
