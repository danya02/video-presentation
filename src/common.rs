use wasm_bindgen::JsCast;
use yew::prelude::*;

use crate::aux::AuxToMainInterop;

pub fn event_handler(send: Callback<AuxToMainInterop>) -> impl FnMut(&Event) {
    move |e| {
        let e: KeyboardEvent = (e.clone()).dyn_into().unwrap();
        let keycode = e.key_code();
        log::info!("Pressed key {keycode}");
        if keycode == 33 {
            // page up (prev)
        } else if keycode == 34 {
            // page down (next)
            log::info!("Sending advance event");
            send.emit(AuxToMainInterop::AdvanceDeadline);
        } else if keycode == 27 {
            // Esc (pause)
            log::info!("Sending pause event");
            send.emit(AuxToMainInterop::SetIsPlaying(false))
        } else if keycode == 66 {
            // B (cancel)
            log::info!("Sending pause event and rate reset");
            send.emit(AuxToMainInterop::SetIsPlaying(false));
            send.emit(AuxToMainInterop::ResetRate)
        } else if keycode == 91 {
            // P (play)
            log::info!("Sending play event");
            send.emit(AuxToMainInterop::SetIsPlaying(true))
        }
    }
}
