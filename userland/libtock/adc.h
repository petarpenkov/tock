#pragma once

#include <stdint.h>

#include "tock.h"

#ifdef __cplusplus
extern "C" {
#endif

#define DRIVER_NUM_ADC 7

int adc_set_callback(subscribe_cb callback, void* callback_args);
int adc_initialize(void);
int adc_single_sample(uint8_t channel);

// Due to the 32-bit limit of the data parameter to the
// `command()' system call, only the lower 24 bits of
// FREQUENCY are used, leaving 8 bits for CHANNEL.
int adc_cont_sample(uint8_t channel, uint32_t frequency);

// Synchronous function to read a single ADC sample.
int adc_read_single_sample(uint8_t channel);

// Asynchronous function to read samples at the given FREQUENCY,
// with units of Hz.
// Due to the 32-bit limit of the data parameter to the
// `command()' system call, only the lower 24 bits of
// FREQUENCY are used, leaving 8 bits for CHANNEL.
int adc_read_cont_sample(uint8_t channel, uint32_t frequency, void (*cb)(int));
int adc_cancel_sampling(void);

// Returns the continuous sampling frequency nearest
// to FREQUENCY that the ADC can actually achieve.
// This calculation is done internally by ADC
// continuous read function, but is offered here as
// a convenience to the user.
uint32_t adc_nearest_interval(uint32_t frequency);

#ifdef __cplusplus
}
#endif
