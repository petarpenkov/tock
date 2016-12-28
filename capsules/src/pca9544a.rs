//! Driver for the PCA9544A I2C Selector.
//!
//! This chip allows for multiple I2C devices with the same addresses to
//! sit on the same I2C bus.
//!
//! http://www.ti.com/product/PCA9544A

use core::cell::Cell;

use kernel::{AppId, Callback, Driver};
use kernel::ReturnCode;
use kernel::common::take_cell::TakeCell;
use kernel::hil::i2c;

pub static mut BUFFER: [u8; 5] = [0; 5];

#[derive(Clone,Copy,PartialEq)]
enum State {
    Idle,

    /// Read the control register and return the specified data field.
    ReadControl(ControlField),

    Done,
}

#[derive(Clone,Copy,PartialEq)]
enum ControlField {
    InterruptMask,
    SelectedChannels,
}


pub struct PCA9544A<'a> {
    i2c: &'a i2c::I2CDevice,
    state: Cell<State>,
    buffer: TakeCell<'static, [u8]>,
    callback: Cell<Option<Callback>>,
}

impl<'a> PCA9544A<'a> {
    pub fn new(i2c: &'a i2c::I2CDevice, buffer: &'static mut [u8]) -> PCA9544A<'a> {
        PCA9544A {
            i2c: i2c,
            state: Cell::new(State::Idle),
            buffer: TakeCell::new(buffer),
            callback: Cell::new(None),
        }
    }

    /// Choose which channel(s) are active. Channels are encoded with a bitwise
    /// mask (0x01 means enable channel 0, 0x0F means enable all channels).
    /// Send 0 to disable all channels.
    fn select_channels(&self, channel_bitmask: u8) -> ReturnCode {
        self.buffer.take().map_or(ReturnCode::ENOMEM, |buffer| {
            self.i2c.enable();

            // Always clear the settings so we get to a known state
            buffer[0] = 0;

            // Iterate the bit array to send the correct channel enables
            let mut index = 1;
            for i in 0..4 {
                if channel_bitmask & (0x01 << i) != 0 {
                    // B2 B1 B0 are set starting at 0x04
                    buffer[index] = i + 4;
                    index += 1;
                }
            }

            self.i2c.write(buffer, index as u8);
            self.state.set(State::Done);

            ReturnCode::SUCCESS
        })
    }

    fn read_interrupts(&self) -> ReturnCode {
        self.read_control(ControlField::InterruptMask)
    }

    fn read_selected_channels(&self) -> ReturnCode {
        self.read_control(ControlField::SelectedChannels)
    }

    fn read_control(&self, field: ControlField) -> ReturnCode {
        self.buffer.take().map_or(ReturnCode::ENOMEM, |buffer| {
            self.i2c.enable();

            // Just issuing a read to the selector reads its control register.
            self.i2c.read(buffer, 1);
            self.state.set(State::ReadControl(field));

            ReturnCode::SUCCESS
        })
    }
}

impl<'a> i2c::I2CClient for PCA9544A<'a> {
    fn command_complete(&self, buffer: &'static mut [u8], _error: i2c::Error) {
        match self.state.get() {
            State::ReadControl(field) => {
                let ret = match field {
                    ControlField::InterruptMask => (buffer[0] >> 4) & 0x0F,
                    ControlField::SelectedChannels => buffer[0] & 0x07,
                };

                self.callback
                    .get()
                    .map(|mut cb| cb.schedule((field as usize) + 1, ret as usize, 0));

                self.buffer.replace(buffer);
                self.i2c.disable();
                self.state.set(State::Idle);
            }
            State::Done => {
                self.callback.get().map(|mut cb| cb.schedule(0, 0, 0));

                self.buffer.replace(buffer);
                self.i2c.disable();
                self.state.set(State::Idle);
            }
            _ => {}
        }
    }
}

impl<'a> Driver for PCA9544A<'a> {
    fn subscribe(&self, subscribe_num: usize, callback: Callback) -> ReturnCode {
        match subscribe_num {
            0 => {
                self.callback.set(Some(callback));
                ReturnCode::SUCCESS
            }

            // default
            _ => ReturnCode::ENOSUPPORT,
        }
    }

    fn command(&self, command_num: usize, data: usize, _: AppId) -> ReturnCode {
        match command_num {
            // Check if present.
            0 => ReturnCode::SUCCESS,

            // Select channels.
            1 => self.select_channels(data as u8),

            // Disable all channels.
            2 => self.select_channels(0),

            // Read the current interrupt fired mask.
            3 => self.read_interrupts(),

            // Read the current selected channels.
            4 => self.read_selected_channels(),

            // default
            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
