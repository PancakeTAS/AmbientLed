#version 330

// this example fragment shader compresses the realistic brightness of each pixel to a logarithmic scale

layout (location = 0) in vec2 TexCoord;
layout (location = 0) out vec4 FragColor;

uniform sampler2D texture1;

void main() {
    vec3 color = texture(texture1, vec2(1.0 - TexCoord.x, TexCoord.y)).rgb;

    // get logarithmic brightness multiplier
    float brightness = color.r * 0.299 + color.g * 0.587 + color.b * 0.114;
    brightness = log2(brightness) / 5.0 + 1.0;

    // normalize color
    float max_component = max(max(color.r, color.g), color.b);
    if (max_component > 0.0) {
        color /= max_component;
    }

    // apply brightness
    color *= brightness;

    FragColor = vec4(color, 1.0);
}
