package de.pancake.ambientled.host.arduino;

import de.pancake.ambientled.host.AmbientLed;
import de.pancake.ambientled.host.util.ColorUtil;
import de.pancake.ambientled.host.util.DesktopCapture;
import lombok.RequiredArgsConstructor;

/**
 * Arduino screen grabber class
 * @author Pancake
 */
@RequiredArgsConstructor
public class ArduinoGrabber implements Runnable {

    // Scaled screen size
    private static final int WIDTH = 3840;
    private static final int HEIGHT = 2160;
    // Amount of LEDs on each side
    private static final int LEDS_SIDE = 55;
    private static final int LEDS_TOP = 75;
    // Size of each LED in pixels
    private static final int HEIGHT_PER_LED = HEIGHT / LEDS_SIDE;
    private static final int WIDTH_PER_LED = WIDTH / LEDS_TOP;

    /** Ambient led instance */
    private final AmbientLed led;
    /** Captures */
    private final DesktopCapture.Capture
            LEFT = DesktopCapture.setupCapture(3840, 0, 300, HEIGHT),
            TOP = DesktopCapture.setupCapture(3840, 0, WIDTH, 180),
            RIGHT = DesktopCapture.setupCapture(3840 + WIDTH - 300, 0, 300, HEIGHT);

    /**
     * Grab screen and calculate average color for each led
     */
    @Override
    public void run() {
        if (this.led.isPaused())
            return;

        // capture screen
        var left = DesktopCapture.screenshot(LEFT);
        var top = DesktopCapture.screenshot(TOP);
        var right = DesktopCapture.screenshot(RIGHT);

        // calculate average color for each led
        for (int i = 0; i < LEDS_SIDE; i++) {
            var c = ColorUtil.average(
                    left,
                    0, HEIGHT_PER_LED * (LEDS_SIDE - i - 1),
                    300, HEIGHT_PER_LED - 1,
                    2, true, false, true
            );

            this.led.getArduinoUpdater().getColors()[i] = c;
        }

        for (int i = 0; i < LEDS_TOP; i++) {
            var c = ColorUtil.average(
                    top,
                    WIDTH_PER_LED * i, 0,
                    WIDTH_PER_LED - 1, 180,
                    2, true, false, true
            );

            this.led.getArduinoUpdater().getColors()[i + LEDS_SIDE] = c;
        }

        for (int i = 0; i < LEDS_SIDE - 5; i++) {
            var c = ColorUtil.average(
                    right,
                    0, HEIGHT_PER_LED * i,
                    300, HEIGHT_PER_LED - 1,
                    2, true, false, true
            );

            this.led.getArduinoUpdater().getColors()[i + LEDS_SIDE + LEDS_TOP] = c;
        }
    }

}