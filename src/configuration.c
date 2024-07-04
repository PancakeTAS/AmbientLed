#include "configuration.h"
#include "arduino.h"
#include "raspberrypi.h"
#include "log.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <cjson/cJSON.h>

#define GET_VAL(json, field, out, member, type) \
    do { \
        cJSON* item = cJSON_GetObjectItem(json, field); \
        if (!item) { \
            log_trace("CONFIGURATION", "cJSON_GetObjectItem() failed: No " field); \
            free(out); \
            return NULL; \
        } \
        out->member = item->type; \
    } while(0)

#define GET_VALS(json, field, out, member) \
    do { \
        cJSON* item = cJSON_GetObjectItem(json, field); \
        if (!item) { \
            log_trace("CONFIGURATION", "cJSON_GetObjectItem() failed: No " field); \
            free(out); \
            return NULL; \
        } \
        out->member = strdup(item->valuestring); \
    } while(0)

#define CONFIG_FILE "config.json" //!< Config file name

/**
 * Create a Raspberry Pi led strip controller
 *
 * \param strip
 *   Configuration for the led strip
 * \param ups
 *   Updates per second
 * \param lerp
 *   Lerp factor
 *
 * \return
 *   File descriptor of the led strip controller or -1 on error
 */
static int create_rpi(configuration_strip* strip, int ups, float lerp) {
    union {
        float f;
        char bytes[4];
    } ftb;
    char rpi_header[] = {
        // 4-byte max brightness
        (char) (strip->max_brightness >> 24), (char) (strip->max_brightness >> 16), (char) (strip->max_brightness >> 8), (char) strip->max_brightness,
        // 4-byte LED count
        (char) (strip->leds >> 24), (char) (strip->leds >> 16), (char) (strip->leds >> 8), (char) strip->leds,
        // float multipliers (padding)
        0, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
        0, 0, 0, 0,
        // 4-byte updates per second
        (char) (ups >> 24), (char) (ups >> 16), (char) (ups >> 8), (char) ups,
    };
    // (set multipliers in big endian)
    ftb.f = strip->r_mult;
    rpi_header[8] = ftb.bytes[3];
    rpi_header[9] = ftb.bytes[2];
    rpi_header[10] = ftb.bytes[1];
    rpi_header[11] = ftb.bytes[0];
    ftb.f = strip->g_mult;
    rpi_header[12] = ftb.bytes[3];
    rpi_header[13] = ftb.bytes[2];
    rpi_header[14] = ftb.bytes[1];
    rpi_header[15] = ftb.bytes[0];
    ftb.f = strip->b_mult;
    rpi_header[16] = ftb.bytes[3];
    rpi_header[17] = ftb.bytes[2];
    rpi_header[18] = ftb.bytes[1];
    rpi_header[19] = ftb.bytes[0];
    ftb.f = lerp;
    rpi_header[20] = ftb.bytes[3];
    rpi_header[21] = ftb.bytes[2];
    rpi_header[22] = ftb.bytes[1];
    rpi_header[23] = ftb.bytes[0];
    return raspberrypi_open(strip->addr, strip->port, rpi_header, 28);
}

/**
 * Create an Arduino led strip controller
 *
 * \param strip
 *   Configuration for the led strip
 * \param ups
 *   Updates per second
 * \param lerp
 *   Lerp factor
 *
 * \return
 *   File descriptor of the led strip controller or -1 on error
 */
static int create_arduino(configuration_strip* strip, int ups, float lerp) {
    char arduino_header[] = {
        // 4-byte max brightness
        (char) (strip->max_brightness >> 24), (char) (strip->max_brightness >> 16), (char) (strip->max_brightness >> 8), (char) strip->max_brightness,
        // 4-byte LED count
        (char) (strip->leds >> 24), (char) (strip->leds >> 16), (char) (strip->leds >> 8), (char) strip->leds,
        // multipliers (fixed point)
        (char) (strip->r_mult * 255.0 - 128.0), (char) (strip->g_mult * 255.0 - 128.0), (char) (strip->b_mult * 255.0 - 128.0), (char) (lerp * 255.0 - 128.0),
        // 4-byte updates per second
        (char) (ups >> 24), (char) (ups >> 16), (char) (ups >> 8), (char) ups,
    };
    return arduino_open(strip->addr, 981600, arduino_header, 16);
}

/**
 * Read the config file and return a cJSON object
 *
 * \return
 *   cJSON object of the config file or NULL on error
 */
static cJSON* read_config() {
    // open config file
    FILE *file = fopen(CONFIG_FILE, "r");
    if (!file) {
        log_trace("CONFIGURATION", "fopen() failed: %s", strerror(errno));

        return NULL;
    }

    // get file size
    fseek(file, 0, SEEK_END);
    size_t size = ftell(file);
    fseek(file, 0, SEEK_SET);

    // allocate memory for file
    char *config = malloc(size + 1);
    if (!config) {
        log_trace("CONFIGURATION", "malloc() failed: %s", strerror(errno));

        fclose(file);
        return NULL;
    }

    // read file
    if (fread(config, 1, size, file) != size) {
        log_trace("CONFIGURATION", "fread() failed: %s", strerror(errno));

        fclose(file);
        free(config);
        return NULL;
    }
    config[size] = 0;

    // parse json
    cJSON *json = cJSON_Parse(config);
    if (!json) {
        log_trace("CONFIGURATION", "cJSON_Parse() failed: %s", strerror(errno));

        fclose(file);
        free(config);
        return NULL;
    }

    // close file and free memory
    fclose(file);
    free(config);

    log_debug("CONFIGURATION", "Config file read successfully");
    return json;
}

/**
 * Parse led segment from a cJSON object
 *
 * \param json
 *   cJSON object to parse
 * \param fps
 *   Frames per second
 *
 * \return
 *   Led segment or NULL on error
 */
configuration_segment* parse_segment(cJSON* json, int fps) {
    // allocate memory for segment
    configuration_segment* segment = malloc(sizeof(configuration_segment));
    if (!segment) {
        log_trace("CONFIGURATION", "malloc() failed: %s", strerror(errno));

        return NULL;
    }

    // get values
    GET_VAL(json, "offset", segment, offset, valueint);
    GET_VAL(json, "length", segment, length, valueint);
    GET_VALS(json, "display", segment, display);
    GET_VAL(json, "x", segment, x, valueint);
    GET_VAL(json, "y", segment, y, valueint);
    GET_VAL(json, "width", segment, width, valueint);
    GET_VAL(json, "height", segment, height, valueint);
    GET_VAL(json, "orientation", segment, orientation, valueint);
    GET_VAL(json, "flip", segment, flip, valueint);

    // create capture session
    segment->capture.display = segment->display;
    segment->capture.framerate = fps;
    segment->capture.area.x = segment->x;
    segment->capture.area.y = segment->y;
    segment->capture.area.width = segment->width;
    segment->capture.area.height = segment->height;
    if (segment->orientation == CONFIGURATION_ORIENTATION_HORIZONTAL) {
        segment->capture.size.width = segment->length;
        segment->capture.size.height = 1;
    } else {
        segment->capture.size.width = 1;
        segment->capture.size.height = segment->length;
    }

    if (capture_create_session(&segment->capture)) {
        log_trace("CONFIGURATION", "capture_create_session() failed");

        free(segment->display);
        free(segment);
        return NULL;
    }

    log_debug("CONFIGURATION", "Segment parsed successfully (display: %s)", segment->display);
    return segment;
}

/**
 * Parse led strip from a cJSON object
 *
 * \param json
 *   cJSON object to parse
 * \param ups
 *   Updates per second
 * \param lerp
 *   Lerp factor
 * \param fps
 *   Frames per second
 *
 * \return
 *   Led strip or NULL on error
 */
configuration_strip* parse_strip(cJSON* json, int ups, float lerp, int fps) {
    // allocate memory for strip
    configuration_strip* strip = malloc(sizeof(configuration_strip));
    if (!strip) {
        log_trace("CONFIGURATION", "malloc() failed: %s", strerror(errno));

        return NULL;
    }

    // get type
    cJSON* type = cJSON_GetObjectItem(json, "type");
    if (!type) {
        log_trace("CONFIGURATION", "cJSON_GetObjectItem() failed: No type");

        free(strip);
        return NULL;
    }

    // get values
    GET_VALS(json, "addr", strip, addr);
    GET_VAL(json, "leds", strip, leds, valueint);
    GET_VAL(json, "max_brightness", strip, max_brightness, valueint);
    GET_VAL(json, "r_mult", strip, r_mult, valuedouble);
    GET_VAL(json, "g_mult", strip, g_mult, valuedouble);
    GET_VAL(json, "b_mult", strip, b_mult, valuedouble);

    // initialize strip
    if (strcmp(type->valuestring, "rpi") == 0) {
        strip->type = CONFIGURATION_TYPE_RPI;
        GET_VAL(json, "port", strip, port, valueint);
    } else if (strcmp(type->valuestring, "arduino") == 0) {
        strip->type = CONFIGURATION_TYPE_ARDUINO;
    } else {
        log_trace("CONFIGURATION", "cJSON_GetObjectItem() failed: Invalid type");

        free(strip->addr);
        free(strip);
        return NULL;
    }

    // create buffer
    strip->buffer = malloc(strip->leds * 3);

    // get segments
    cJSON* segments = cJSON_GetObjectItem(json, "segments");
    if (!segments) {
        log_trace("CONFIGURATION", "cJSON_GetObjectItem() failed: No segments");

        free(strip->buffer);
        free(strip->addr);
        free(strip);
        return NULL;
    }

    int num_segments = cJSON_GetArraySize(segments);
    log_debug("CONFIGURATION", "num_segments: %d", num_segments);
    strip->segments = malloc(sizeof(configuration_segment*) * num_segments);
    if (!strip->segments) {
        log_trace("CONFIGURATION", "malloc() failed: %s", strerror(errno));

        free(strip->buffer);
        free(strip->addr);
        free(strip);
        return NULL;
    }

    cJSON* segment;
    strip->num_segments = 0;
    cJSON_ArrayForEach(segment, segments) {
        configuration_segment* seg = parse_segment(segment, fps);
        if (!seg) {
            log_trace("CONFIGURATION", "parse_segment() failed");

            free(strip->buffer);
            free(strip->addr);
            free(strip->segments);
            free(strip);
            return NULL;
        }

        strip->segments[strip->num_segments] = seg;
        strip->num_segments++;
    }

    log_debug("CONFIGURATION", "Strip parsed successfully (addr: %s)", strip->addr);
    return strip;
}

void init_strip(configuration_strip* strip, int ups, float lerp) {
    // initialize strip
    if (strip->type == CONFIGURATION_TYPE_RPI) {
        strip->fd = create_rpi(strip, ups, lerp);
    } else {
        strip->fd = create_arduino(strip, ups, lerp);
    }

    if (strip->fd < 0) {
        log_trace("CONFIGURATION", "create_%s() failed", strip->type == CONFIGURATION_TYPE_RPI ? "rpi" : "arduino");

        free(strip->addr);
        free(strip);
    }
}

/**
 * Parse configuration data from a cJSON object
 *
 * \param json
 *   cJSON object to parse
 *
 * \return
 *   Configuration data or NULL on error
 */
configuration_data* parse_configuration(cJSON* json) {
    // allocate memory for data
    configuration_data* data = malloc(sizeof(configuration_data));
    if (!data) {
        log_trace("CONFIGURATION", "malloc() failed: %s", strerror(errno));

        return NULL;
    }

    // get values
    GET_VAL(json, "ups", data, ups, valueint);
    GET_VAL(json, "fps", data, fps, valueint);
    GET_VAL(json, "lerp", data, lerp, valuedouble);

    // get strips
    cJSON* strips = cJSON_GetObjectItem(json, "strips");
    if (!strips) {
        log_trace("CONFIGURATION", "cJSON_GetObjectItem() failed: No strips");

        free(data);
        return NULL;
    }

    int num_strips = cJSON_GetArraySize(strips);
    data->strips = malloc(sizeof(configuration_strip*) * num_strips);
    if (!data->strips) {
        log_trace("CONFIGURATION", "malloc() failed: %s", strerror(errno));

        free(data);
        return NULL;
    }

    cJSON* strip;
    data->num_strips = 0;
    cJSON_ArrayForEach(strip, strips) {
        configuration_strip* s = parse_strip(strip, data->ups, data->lerp, data->fps);
        if (!s) {
            log_trace("CONFIGURATION", "parse_strip() failed");

            free(data->strips);
            free(data);
            return NULL;
        }

        data->strips[data->num_strips] = s;
        data->num_strips++;
    }

    log_debug("CONFIGURATION", "Configuration parsed successfully");
    return data;
}

configuration_data* configuration_parse() {
    // read config file
    cJSON* json = read_config();
    if (!json) {
        log_trace("CONFIGURATION", "read_config() failed");
        return NULL;
    }

    // parse config
    configuration_data* data = parse_configuration(json);
    if (!data) {
        log_trace("CONFIGURATION", "parse_configuration() failed");

        cJSON_Delete(json);
        return NULL;
    }

    // connect to controllers
    for (int i = 0; i < data->num_strips; i++) {
        configuration_strip* strip = data->strips[i];
        init_strip(strip, data->ups, data->lerp);
    }

    cJSON_Delete(json);
    return data;
}

void configuration_free(configuration_data* data) {
    for (int i = 0; i < data->num_strips; i++) {
        for (int j = 0; j < data->strips[i]->num_segments; j++) {
            capture_destroy_session(&data->strips[i]->segments[j]->capture);

            free(data->strips[i]->segments[j]->display);
            free(data->strips[i]->segments[j]);
        }

        if (data->strips[i]->type == CONFIGURATION_TYPE_RPI) raspberrypi_close(data->strips[i]->fd);
        else if (data->strips[i]->type == CONFIGURATION_TYPE_ARDUINO) arduino_close(data->strips[i]->fd);

        free(data->strips[i]->addr);
        free(data->strips[i]->segments);
        free(data->strips[i]->buffer);
        free(data->strips[i]);
    }

    free(data->strips);
    free(data);
}