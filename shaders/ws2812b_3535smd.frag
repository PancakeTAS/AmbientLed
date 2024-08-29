#version 330 core

// this shader is for (my) WS2812B 3535SMD led strip.
// apparently one of my strips didn't use 5050SMD LEDs, but 3535SMD LEDs. while they're supposedly really bad when it comes to color accuracy,
// i actually found them to be the most precise of all my strips. unlike being too blue, they're actually too green (and a bit too red).
// this shader tries to correct them to sRGB, because that's what my monitor is calibrated to. it also
// corrects the brightness to a logarithmic scale to make darker colors more visible.
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
	color.r *= 0.9; // adjust red and green to closer match sRGB
	color.g *= 0.9;
	vec3 norm_color = norm(color);

	float perceptual_brightness =
		color.r * 0.299 +
		color.g * (0.587 / 0.9) + // perceptual brightness is unaffected by the color correction
		color.b * (0.114 / 0.9);

	float logarithmic_brightness = log2(perceptual_brightness) / 5.0 + 1.0;
	norm_color *= logarithmic_brightness * 0.9;

	FragColor = vec4(norm_color, 1.0);
}
