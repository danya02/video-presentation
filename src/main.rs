use std::time::Duration;

use shadow_clone::shadow_clone;
use wasm_bindgen::{closure::Closure, convert::IntoWasmAbi, JsValue};
use web_sys::wasm_bindgen::JsCast;
use web_sys::HtmlVideoElement;
use webvtt::{Block, Cue};
use yew::prelude::*;
use yew_hooks::{use_interval, use_is_first_mount, use_state_ptr_eq};

#[function_component]
fn App() -> Html {
    let subs = use_state(|| webvtt::parse_file(include_str!("../media/subs.vtt")).unwrap());
    let current_block = use_state_eq(|| (0usize, false));
    let is_playing = use_state_eq(|| false);
    let current_time = use_state(|| Duration::from_secs(0));
    let current_rate = use_state(|| 1.0);
    let high_res_callback = use_state(|| None);
    let video_el = use_node_ref();
    let deadline_block_idx = use_state(|| 0usize);

    fn b2c(b: &Block) -> &Cue {
        match b {
            Block::Cue(v) => v,
        }
    }

    fn fits(time: Duration, cue: &Cue) -> bool {
        return cue.start <= time && time <= cue.end;
    }

    let onratechange = Callback::from({
        shadow_clone!(current_rate);
        move |ev: Event| {
            let element: HtmlVideoElement = ev.target().unwrap().dyn_into().unwrap();
            current_rate.set(element.playback_rate());
        }
    });

    let ontimeupdate = Callback::from({
        shadow_clone!(
            current_time,
            current_block,
            subs,
            video_el,
            deadline_block_idx
        );
        move |ev: Event| {
            let subs = &*subs;
            let element: HtmlVideoElement;
            if let Some(v) = video_el.get() {
                element = v.dyn_into().unwrap();
            } else {
                element = ev.target().unwrap().dyn_into().unwrap();
            }
            let now = Duration::from_secs_f64(element.current_time());
            current_time.set(now);

            let sub_list = &(*subs.blocks);
            let (mut current_block_idx, mut current_block_is_visible) = *current_block;

            // Loop over the blocks to find one that the value matches.
            let idxs = (current_block_idx..sub_list.len()).chain(0..current_block_idx);
            for idx in idxs {
                let cue = b2c(&sub_list[idx]);
                // If this cue fits, set this as the current block.
                if fits(now, cue) {
                    current_block_idx = idx;
                    current_block_is_visible = true;
                    break;
                }
                // Otherwise, if now is after the cue,
                // then this cue is the last visible one.
                if cue.end < now {
                    current_block_idx = idx.max(current_block_idx);
                    current_block_is_visible = false;
                }
            }

            current_block.set((current_block_idx, current_block_is_visible));

            // Set the playback rate based on the time left until the end of the deadline block.
            let deadline_block = b2c(&sub_list[*deadline_block_idx]);
            let time_until_end = deadline_block.end.checked_sub(now).unwrap_or_default();

            let rate_fn = |time: Duration| {
                let time_s = time.as_secs_f64();
                let k = 2.0;
                2.0 * (1.0 / (1.0 + std::f64::consts::E.powf(-k * time_s)) - 0.5)
            };
            element.set_playback_rate(rate_fn(time_until_end));
        }
    });

    let advance_deadline_block = {
        shadow_clone!(deadline_block_idx);
        Callback::from(move |ev: MouseEvent| {
            log::info!("Advancing deadline block idx: was {}", *deadline_block_idx);
            deadline_block_idx.set(*deadline_block_idx + 1);
            ev.prevent_default();
        })
    };

    use_interval(
        {
            shadow_clone!(ontimeupdate);
            move || ontimeupdate.emit(Event::new("none").unwrap())
        },
        100,
    );

    let is_first = use_is_first_mount();
    let global_keydown_handler = use_state_ptr_eq(|| None);
    if is_first {
        let window = web_sys::window().unwrap();
        let listener = gloo::events::EventListener::new(&window, "keydown", {
            shadow_clone!(advance_deadline_block);
            move |e| {
                let e: KeyboardEvent = (e.clone()).dyn_into().unwrap();
                let keycode = e.key_code();
                log::info!("Pressed key {keycode}");
                if keycode == 33 {
                    // page up (prev)
                } else if keycode == 34 {
                    // page down (next)
                    log::info!("Sending advance event");
                    advance_deadline_block.emit(MouseEvent::new("none").unwrap());
                }
            }
        });
        global_keydown_handler.set(Some(listener));
    }

    if high_res_callback.is_none() {
        #[wasm_bindgen::prelude::wasm_bindgen]
        extern "C" {
            fn request_video_frame_callback(
                this: &HtmlVideoElement,
                cb: &Closure<dyn FnMut(JsValue, JsValue)>,
            );
            fn request_video_frame_callback_again();

        }

        match video_el.cast::<HtmlVideoElement>() {
            Some(el) => {
                shadow_clone!(ontimeupdate);
                let cb = Closure::new(move |now, metadata| {
                    log::info!("Frame!");
                    ontimeupdate.emit(Event::new("none").unwrap());
                    request_video_frame_callback_again();
                });
                request_video_frame_callback(&el, &cb);
                high_res_callback.set(Some(cb));
            }
            None => {}
        }
    }

    let onplay = Callback::from({
        shadow_clone!(is_playing);
        move |_ev: Event| {
            is_playing.set(true);
        }
    });
    let onpause = Callback::from({
        shadow_clone!(is_playing);
        move |_ev: Event| {
            is_playing.set(false);
        }
    });

    let subs = &*subs;

    html! {
        <div class="container">
            <video src="/media/vid.webm" controls={true} ref={video_el} muted={true}
            {ontimeupdate} {onplay} {onpause} {onratechange}
            style="width: 100%;"/>

            <hr />
            <p>{"Current time: "}{format!("{:?}", *current_time)}</p>
            <p>{"Current playback rate: "}{*current_rate}</p>
            <p>{"Is playing: "}{*is_playing}</p>
            <p>{"Current block: "}{format!("{:?}", &(subs.blocks[(*current_block).0]))}</p>
            <p>{"Current block is visible: "}{(*current_block).1}</p>
            <button class="btn btn-success" onclick={advance_deadline_block}>{"Advance deadline..."}</button>
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
