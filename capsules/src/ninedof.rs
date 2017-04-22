//! This allows a 9DOF sensor to be used by multiple apps.

use core::cell::Cell;
use kernel::{AppId, Callback, Container, Driver};
use kernel::hil;
use kernel::returncode::ReturnCode;

#[derive(Clone,Copy,PartialEq)]
pub enum NineDofCommand {
    Exists,
    ReadAccelerometer,
    ReadMagnetometer,
    ReadGyroscope,
}

pub struct App {
    callback: Option<Callback>,
    pending_command: bool,
    command: NineDofCommand,
    arg1: usize,
}

impl Default for App {
    fn default() -> App {
        App {
            callback: None,
            pending_command: false,
            command: NineDofCommand::Exists,
            arg1: 0,
        }
    }
}

pub struct NineDof<'a> {
    driver: &'a hil::ninedof::NineDof,
    apps: Container<App>,
    current_app: Cell<Option<AppId>>,
}

impl<'a> NineDof<'a> {
    pub fn new(driver: &'a hil::ninedof::NineDof, container: Container<App>) -> NineDof<'a> {
        NineDof {
            driver: driver,
            apps: container,
            current_app: Cell::new(None),
        }
    }

    // Check so see if we are doing something. If not,
    // go ahead and do this command. If so, this is queued
    // and will be run when the pending command completes.
    fn enqueue_command(&self, command: NineDofCommand, arg1: usize, appid: AppId) -> ReturnCode {
        self.apps
            .enter(appid, |app, _| if self.current_app.get().is_none() {
                self.current_app.set(Some(appid));
                self.call_driver(command, arg1)
            } else {
                if app.pending_command == true {
                    ReturnCode::ENOMEM
                } else {
                    app.pending_command = true;
                    app.command = command;
                    app.arg1 = arg1;
                    ReturnCode::SUCCESS
                }
            })
            .unwrap_or(ReturnCode::FAIL)
    }

    fn call_driver(&self, command: NineDofCommand, _: usize) -> ReturnCode {
        match command {
            NineDofCommand::ReadAccelerometer => self.driver.read_accelerometer(),
            NineDofCommand::ReadMagnetometer => self.driver.read_magnetometer(),
            NineDofCommand::ReadGyroscope => self.driver.read_gyroscope(),
            _ => ReturnCode::FAIL,
        }
    }
}

impl<'a> hil::ninedof::NineDofClient for NineDof<'a> {
    fn callback(&self, arg1: usize, arg2: usize, arg3: usize) {
        // Notify the current application that the command finished.
        // Also keep track of what just finished to see if we can re-use
        // the result.
        let mut finished_command = NineDofCommand::Exists;
        let mut finished_command_arg = 0;
        self.current_app.get().map(|appid| {
            self.current_app.set(None);
            let _ = self.apps.enter(appid, |app, _| {
                app.pending_command = false;
                finished_command = app.command;
                finished_command_arg = app.arg1;
                app.callback.map(|mut cb| { cb.schedule(arg1, arg2, arg3); });
            });
        });

        // Check if there are any pending events.
        for cntr in self.apps.iter() {
            let started_command = cntr.enter(|app, _| {
                if app.pending_command && app.command == finished_command &&
                   app.arg1 == finished_command_arg {
                    // Don't bother re-issuing this command, just use
                    // the existing result.
                    app.pending_command = false;
                    app.callback.map(|mut cb| { cb.schedule(arg1, arg2, arg3); });
                    false
                } else if app.pending_command {
                    app.pending_command = false;
                    self.current_app.set(Some(app.appid()));
                    self.call_driver(app.command, app.arg1) == ReturnCode::SUCCESS
                } else {
                    false
                }
            });
            if started_command {
                break;
            }
        }
    }
}

impl<'a> Driver for NineDof<'a> {
    fn subscribe(&self, subscribe_num: usize, callback: Callback) -> ReturnCode {
        match subscribe_num {
            0 => {
                self.apps
                    .enter(callback.app_id(), |app, _| {
                        app.callback = Some(callback);
                        ReturnCode::SUCCESS
                    })
                    .unwrap_or(ReturnCode::FAIL)
            }
            _ => ReturnCode::ENOSUPPORT,
        }
    }

    fn command(&self, command_num: usize, arg1: usize, appid: AppId) -> ReturnCode {
        match command_num {
            0 => /* This driver exists. */ ReturnCode::SUCCESS,

            // Single acceleration reading.
            1 => self.enqueue_command(NineDofCommand::ReadAccelerometer, arg1, appid),

            // Single magnetometer reading.
            100 => self.enqueue_command(NineDofCommand::ReadMagnetometer, arg1, appid),

            // Single gyroscope reading.
            200 => self.enqueue_command(NineDofCommand::ReadGyroscope, arg1, appid),

            _ => ReturnCode::ENOSUPPORT,
        }
    }
}
