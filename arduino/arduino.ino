// ========= PREPROCESSOR CONFIGURATION =========

#define SERIAL_BAUD 500000
#define MAX_BRIGHTNESS 220

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
    int i = Serial.readBytes((char*) leds, BUFFER_LENGTH * 3);
    if (i != BUFFER_LENGTH * 3) {
        memset(leds, 0, BUFFER_LENGTH * 3);
    }

    FastLED.show();
}
