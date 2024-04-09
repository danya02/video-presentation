use std::time::Duration;

use shadow_clone::shadow_clone;
use web_sys::wasm_bindgen::JsCast;
use web_sys::HtmlVideoElement;
use webvtt::{Block, Cue};
use yew::prelude::*;

#[function_component]
fn App() -> Html {
    let subs = use_state(|| webvtt::parse_file(include_str!("../media/subs.vtt")).unwrap());
    let current_block = use_state_eq(|| (0usize, false));
    let is_playing = use_state_eq(|| false);
    let current_time = use_state(|| Duration::from_secs(0));

    fn b2c(b: &Block) -> &Cue {
        match b {
            Block::Cue(v) => v,
        }
    }

    fn fits(time: Duration, cue: &Cue) -> bool {
        return cue.start <= time && time <= cue.end;
    }

    let ontimeupdate = Callback::from({
        shadow_clone!(current_time, current_block, subs);
        move |ev: Event| {
            let subs = &*subs;
            let element: HtmlVideoElement = ev.target().unwrap().dyn_into().unwrap();
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
        }
    });

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
            <video src="/media/vid.webm" controls={true}
            {ontimeupdate} {onplay} {onpause}
            style="width: 100%;"/>

            <hr />
            <p>{"Current time: "}{format!("{:?}", *current_time)}</p>
            <p>{"Is playing: "}{*is_playing}</p>
            <p>{"Current block: "}{format!("{:?}", &(subs.blocks[(*current_block).0]))}</p>
            <p>{"Current block is visible: "}{(*current_block).1}</p>
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
