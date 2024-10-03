// ========= PREPROCESSOR CONFIGURATION =========

#define SERIAL_BAUD 500000
#define MAX_BRIGHTNESS 240 // keep this at the maximum value your PSU can handle and do the rest in the shader

#define STRIP1_LENGTH 88
#define STRIP1_DATA 2

#define STRIP2_LENGTH 91
#define STRIP2_DATA 4

// (do not change anything below this line)
#define SERIAL_TIMEOUT 5000

#ifdef STRIP1_LENGTH
    #ifdef STRIP2_LENGTH
        #ifdef STRIP3_LENGTH
            #ifdef STRIP4_LENGTH
                #define BUFFER_LENGTH (STRIP1_LENGTH + STRIP2_LENGTH + STRIP3_LENGTH + STRIP4_LENGTH)
                #define STRIP_COUNT 4
            #else
                #define BUFFER_LENGTH (STRIP1_LENGTH + STRIP2_LENGTH + STRIP3_LENGTH)
                #define STRIP_COUNT 3
            #endif
        #else
            #define BUFFER_LENGTH (STRIP1_LENGTH + STRIP2_LENGTH)
            #define STRIP_COUNT 2
        #endif
    #else
        #define BUFFER_LENGTH STRIP1_LENGTH
        #define STRIP_COUNT 1
    #endif
#else
    #error "At least one and at most four strips must be defined"
#endif

#define FASTLED_ALLOW_INTERRUPTS 0
#include <FastLED.h>

// ========== PREPROCESSOR CONFIGURATION END ==========

struct CRGB leds[BUFFER_LENGTH]; //!< buffer for all leds

int timeout_steps = 0; //!< timeout animation counter

void setup() {
    // init serial
    Serial.begin(SERIAL_BAUD);
    Serial.setTimeout(SERIAL_TIMEOUT);

    // init leds
    #if STRIP_COUNT >= 1
    FastLED.addLeds<WS2812B, STRIP1_DATA, GRB>(leds, STRIP1_LENGTH);
    #endif

    #if STRIP_COUNT >= 2
    FastLED.addLeds<WS2812B, STRIP2_DATA, GRB>(leds + STRIP1_LENGTH, STRIP2_LENGTH);
    #endif

    #if STRIP_COUNT >= 3
    FastLED.addLeds<WS2812B, STRIP3_DATA, GRB>(leds + STRIP1_LENGTH + STRIP2_LENGTH, STRIP3_LENGTH);
    #endif

    #if STRIP_COUNT >= 4
    FastLED.addLeds<WS2812B, STRIP4_DATA, GRB>(leds + STRIP1_LENGTH + STRIP2_LENGTH + STRIP3_LENGTH, STRIP4_LENGTH);
    #endif

    // configure fastled
    FastLED.setDither(0);
    FastLED.setBrightness(MAX_BRIGHTNESS);
    FastLED.setMaxRefreshRate(0);
}

void loop() {
    // try to read the leds
    int i = Serial.readBytes((char*) leds, BUFFER_LENGTH * 3);
    // check if data was fully read
    if (i != BUFFER_LENGTH * 3) {
        // if not, start timeout animation
        if (!timeout_steps) {
            Serial.setTimeout(30);
            FastLED.setCorrection(CRGB(255, 0, 0));
        }

        // timeout animation
        timeout_steps++;
        memset(leds, (sin(timeout_steps * 0.025) + 1) / 2 * 220 + 20, BUFFER_LENGTH * 3);
    } else if (timeout_steps) {
        // otherwise reset timeout counter
        Serial.setTimeout(SERIAL_TIMEOUT);
        FastLED.setCorrection(CRGB(255, 255, 255));
        timeout_steps = 0;
    }

    FastLED.show();
}
