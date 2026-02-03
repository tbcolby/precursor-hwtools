#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

mod app;
mod gpio_tab;
mod storage;
mod system_tab;
mod uart_tab;
mod ui;

use app::HwToolsApp;
use num_traits::*;
use xous::Message;

const SERVER_NAME: &str = "_HW Tools_";
pub const APP_NAME: &str = "HW Tools";

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
    Pump,
    Quit,
}

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum PumpOp {
    Run,
    Stop,
    Pump,
    Quit,
}

const REFRESH_INTERVAL_MS: usize = 2000;

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("HW Tools starting, PID {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME, None).expect("can't register server");

    let mut app = HwToolsApp::new(&xns, sid);

    // Build pump thread for periodic updates
    let pump_sid = xous::create_server().unwrap();
    let cid_to_pump = xous::connect(pump_sid).unwrap();
    spawn_pump_thread(xous::connect(sid).unwrap(), pump_sid);

    let mut allow_redraw = true;
    let mut into_foreground = false;

    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => {
                if allow_redraw {
                    if into_foreground {
                        app.full_redraw();
                        into_foreground = false;
                    } else {
                        app.redraw();
                    }
                }
            }
            Some(AppOp::Pump) => {
                if allow_redraw {
                    app.refresh_data();
                    app.redraw();
                }
                xous::return_scalar(msg.sender, 1).expect("couldn't ack pump");
            }
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                let keys = [
                    core::char::from_u32(k1 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k2 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k3 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k4 as u32).unwrap_or('\u{0000}'),
                ];
                for &key in keys.iter() {
                    if key != '\u{0000}' {
                        if app.handle_key(key) {
                            // App wants to quit
                            xous::send_message(
                                cid_to_pump,
                                Message::new_blocking_scalar(
                                    PumpOp::Quit.to_usize().unwrap(),
                                    0,
                                    0,
                                    0,
                                    0,
                                ),
                            )
                            .ok();
                            unsafe { xous::disconnect(cid_to_pump).ok() };
                            break;
                        }
                    }
                }
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, new_state_code, _, _, _, {
                let new_state = gam::FocusState::convert_focus_change(new_state_code);
                log::info!("focus change: {:?}", new_state);
                match new_state {
                    gam::FocusState::Background => {
                        allow_redraw = false;
                        xous::send_message(
                            cid_to_pump,
                            Message::new_scalar(PumpOp::Stop.to_usize().unwrap(), 0, 0, 0, 0),
                        )
                        .ok();
                    }
                    gam::FocusState::Foreground => {
                        into_foreground = true;
                        allow_redraw = true;
                        app.refresh_data();
                        xous::send_message(
                            cid_to_pump,
                            Message::new_scalar(PumpOp::Run.to_usize().unwrap(), 0, 0, 0, 0),
                        )
                        .ok();
                    }
                }
            }),
            Some(AppOp::Quit) => xous::msg_blocking_scalar_unpack!(msg, _, _, _, _, {
                xous::send_message(
                    cid_to_pump,
                    Message::new_blocking_scalar(PumpOp::Quit.to_usize().unwrap(), 0, 0, 0, 0),
                )
                .ok();
                unsafe { xous::disconnect(cid_to_pump).ok() };
                xous::return_scalar(msg.sender, 1).ok();
                break;
            }),
            _ => log::error!("unknown opcode: {:?}", msg),
        }
    }

    app.save_settings();
    log::info!("HW Tools exiting");
    xns.unregister_server(sid).unwrap();
    xous::destroy_server(sid).unwrap();
    xous::terminate_process(0)
}

fn spawn_pump_thread(cid_to_main: xous::CID, pump_sid: xous::SID) {
    std::thread::spawn(move || {
        let tt = ticktimer_server::Ticktimer::new().unwrap();
        let cid_to_self = xous::connect(pump_sid).unwrap();
        let mut running = false;

        loop {
            let msg = xous::receive_message(pump_sid).unwrap();
            match FromPrimitive::from_usize(msg.body.id()) {
                Some(PumpOp::Run) => {
                    running = true;
                    xous::send_message(
                        cid_to_self,
                        Message::new_scalar(PumpOp::Pump.to_usize().unwrap(), 0, 0, 0, 0),
                    )
                    .ok();
                }
                Some(PumpOp::Stop) => {
                    running = false;
                }
                Some(PumpOp::Pump) => {
                    if running {
                        xous::send_message(
                            cid_to_main,
                            Message::new_blocking_scalar(
                                AppOp::Pump.to_usize().unwrap(),
                                0,
                                0,
                                0,
                                0,
                            ),
                        )
                        .ok();
                        tt.sleep_ms(REFRESH_INTERVAL_MS).unwrap();
                        xous::send_message(
                            cid_to_self,
                            Message::new_scalar(PumpOp::Pump.to_usize().unwrap(), 0, 0, 0, 0),
                        )
                        .ok();
                    }
                }
                Some(PumpOp::Quit) => {
                    xous::return_scalar(msg.sender, 1).ok();
                    break;
                }
                _ => {}
            }
        }
        xous::destroy_server(pump_sid).ok();
    });
}
