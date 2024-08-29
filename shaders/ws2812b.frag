#version 330 core

// this shader is for (my) WS2812B led strip.
// the strip I own is a tiny bit too green for me. since my own
// monitor is calibrated to sRGB, so I'm reducing the green and blue channels, to get from "green whites" to "yellow whites".
// this shader also adjusts the brightness to a logarithmic scale to make darker colors more visible.
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
	color.g *= 0.6; // adjust green and blue to closer match sRGB
	color.b *= 0.55;
	vec3 norm_color = norm(color);

	float perceptual_brightness =
		color.r * 0.299 +
		color.g * (0.587 / 0.6) + // perceptual brightness is unaffected by the color correction
		color.b * (0.114 / 0.55);

	float logarithmic_brightness = log2(perceptual_brightness) / 5.0 + 1.0;
	norm_color *= logarithmic_brightness * 0.9;

	FragColor = vec4(norm_color, 1.0);
}
