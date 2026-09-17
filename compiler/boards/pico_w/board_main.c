/* Pico W std.led backend — CYW43439 activity LED (not RP2040 GPIO 25). */
#include "pico/stdlib.h"
#include "pico/cyw43_arch.h"

static int led_ready = 0;
static int led_state = 0;

static void ensure_led(void) {
    if (led_ready) {
        return;
    }
    if (cyw43_arch_init()) {
        return;
    }
    led_ready = 1;
}

void buraaq_led_on(void) {
    ensure_led();
    if (!led_ready) {
        return;
    }
    led_state = 1;
    cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, 1);
}

void buraaq_led_off(void) {
    ensure_led();
    if (!led_ready) {
        return;
    }
    led_state = 0;
    cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, 0);
}

void buraaq_led_toggle(void) {
    if (led_state) {
        buraaq_led_off();
    } else {
        buraaq_led_on();
    }
}

void buraaq_led_wait_ms(int ms) {
    sleep_ms((uint32_t)(ms < 0 ? 0 : ms));
}

/* Stock superloop until Buraaq thumb objects link as main.
 * Same timing as `use std.led` blink examples. */
int main(void) {
    for (;;) {
        buraaq_led_on();
        buraaq_led_wait_ms(200);
        buraaq_led_off();
        buraaq_led_wait_ms(200);
    }
}
