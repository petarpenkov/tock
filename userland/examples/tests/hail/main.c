#include <stdio.h>
#include <stdbool.h>

#include <nordic_common.h>
#include <nrf_error.h>
#include <ble_advdata.h>

#include <simple_ble.h>
#include <simple_adv.h>

#include <timer.h>
#include <isl29035.h>
#include <si7021.h>
#include <ninedof.h>
#include <button.h>
#include <led.h>
#include <adc.h>
#include <gpio.h>
#include <nrf51_serialization.h>

// Intervals for BLE advertising and connections
simple_ble_config_t ble_config = {
    .platform_id       = 0x13,              // used as 4th octect in device BLE address
    .device_id         = DEVICE_ID_DEFAULT,
    .adv_name          = (char*)"Hail",
    .adv_interval      = MSEC_TO_UNITS(1000, UNIT_0_625_MS),
    .min_conn_interval = MSEC_TO_UNITS(1000, UNIT_1_25_MS),
    .max_conn_interval = MSEC_TO_UNITS(1250, UNIT_1_25_MS),
};

// Empty handler for setting BLE addresses
void ble_address_set (void) {
  // nop
}

// Callback for button presses.
//   btn_num: The index of the button associated with the callback
//   val: 0 if pressed, 1 if depressed
static void button_callback(__attribute__ ((unused)) int btn_num,
                            int val,
                            __attribute__ ((unused)) int arg2,
                            __attribute__ ((unused)) void *ud) {
  if (val == 0) {
    led_on(1); // green
  } else {
    led_off(1);
  }
}

static void sample_sensors (void) {

  // Sensors: temperature/humidity, acceleration, light
  int temp;
  unsigned humi;
  si7021_get_temperature_humidity_sync(&temp, &humi);
  uint32_t accel_mag = ninedof_read_accel_mag();
  int light = isl29035_read_light_intensity();

  // Analog inputs: A0-A5
  int a0 = (adc_read_single_sample(0) * 3300) / 4095;
  int a1 = (adc_read_single_sample(1) * 3300) / 4095;
  int a2 = (adc_read_single_sample(3) * 3300) / 4095;
  int a3 = (adc_read_single_sample(4) * 3300) / 4095;
  int a4 = (adc_read_single_sample(5) * 3300) / 4095;
  int a5 = (adc_read_single_sample(6) * 3300) / 4095;

  // Digital inputs: D0, D1, D6, D7
  int d0 = gpio_read(0);
  int d1 = gpio_read(1);
  int d6 = gpio_read(2);
  int d7 = gpio_read(3);

  // print results
  printf("[Hail Sensor Reading]\n");
  printf("  Temperature:  %d 1/100 degrees C\n", temp);
  printf("  Humidity:     %u 0.01%%\n", humi);
  printf("  Light:        %d\n", light);
  printf("  Acceleration: %lu\n", accel_mag);
  printf("  A0:           %d mV\n", a0);
  printf("  A1:           %d mV\n", a1);
  printf("  A2:           %d mV\n", a2);
  printf("  A3:           %d mV\n", a3);
  printf("  A4:           %d mV\n", a4);
  printf("  A5:           %d mV\n", a5);
  printf("  D0:           %d\n", d0);
  printf("  D1:           %d\n", d1);
  printf("  D6:           %d\n", d6);
  printf("  D7:           %d\n", d7);
  printf("\n");

  // toggle the blue LED
  led_toggle(2);
}

int main(void) {
  printf("[Hail] Test App!\n");
  printf("[Hail] Samples all sensors.\n");
  printf("[Hail] Transmits name over BLE.\n");
  printf("[Hail] Button controls LED.\n");

  // Setup BLE
  simple_ble_init(&ble_config);
  simple_adv_only_name();

  // Enable button callbacks
  button_subscribe(button_callback, NULL);
  button_enable_interrupt(0);

  // Setup the ADC
  adc_initialize();

  // Setup D0, D1, D6, D7
  gpio_enable_input(0, PullDown); // D0
  gpio_enable_input(1, PullDown); // D1
  gpio_enable_input(2, PullDown); // D6
  gpio_enable_input(3, PullDown); // D7

  // sample sensors every second
  while (1) {
    sample_sensors();
    delay_ms(1000);
  }
}

