use std::time::Duration;

use gloo::{events::EventListener, timers::callback::Interval};
use wasm_bindgen::prelude::*;
use web_sys::HtmlVideoElement;
use web_sys::{js_sys::wasm_bindgen, wasm_bindgen::JsCast};
use webvtt::{Block, Cue};
use yew::prelude::*;

struct App {
    subs: webvtt::File,
    current_block: usize,
    current_block_has_passed: bool,
    is_playing: bool,
    current_time: Duration,
    current_rate: f64,
    high_res_callback: Option<Closure<dyn FnMut(JsValue, JsValue)>>,
    video_el: NodeRef,
    deadline_block_idx: usize,
    global_keydown_listener: Option<EventListener>,
    interval_callback: Option<Interval>,
    block_timing_history: Vec<f64>,
    current_block_started_at: f64,
    target_rate: f64,
}

#[wasm_bindgen]
extern "C" {
    fn unixtime() -> f64;
}

enum Msg {
    Periodic,
    Playing(bool),
    RateChange,
    NextDeadline,
}

impl Component for App {
    type Message = Msg;

    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let text = include_str!("../media/subs-verbose.vtt");
        // log::info!("{text}");
        let subs = webvtt::parse_file(text).unwrap();
        Self {
            subs,
            current_block: 0,
            current_block_has_passed: false,
            is_playing: false,
            current_time: Duration::ZERO,
            current_rate: 1.0,
            high_res_callback: None,
            video_el: NodeRef::default(),
            deadline_block_idx: 0,
            global_keydown_listener: None,
            interval_callback: None,
            current_block_started_at: 0.0,
            block_timing_history: vec![],
            target_rate: 1.0,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let ontimeupdate = ctx.link().callback(|_ev| Msg::Periodic);
        let onplay = ctx.link().callback(|_ev| Msg::Playing(true));
        let onpause = ctx.link().callback(|_ev| Msg::Playing(false));
        let onratechange = ctx.link().callback(|_ev| Msg::RateChange);
        let advance_deadline_block = ctx.link().callback(|ev: MouseEvent| {
            ev.prevent_default();
            Msg::NextDeadline
        });

        html! {
            <div class="container">
                <video src="/media/vid.mp4" controls={true} ref={self.video_el.clone()} muted={true}
                {ontimeupdate} {onplay} {onpause} {onratechange}
                style="width: 100%;"/>

                <hr />
                <p>{"Current time: "}{format!("{:?}", self.current_time)}</p>
                <p>{"Current playback rate: "}{self.current_rate}</p>
                <p>{"Is playing: "}{self.is_playing}</p>
                <p>{"Current block: "}{format!("{:?}", &(self.subs.blocks[self.current_block]))}</p>
                <p>{"Current block is visible: "}{self.current_block_has_passed}</p>
                <p>{"Deadline block: "}{format!("{:?}", &(self.subs.blocks[self.deadline_block_idx]))}</p>
                <p>{"Duration history: "}{format!("{:?}", self.block_timing_history)}</p>
                <button class="btn btn-success" onclick={advance_deadline_block}>{"Advance deadline..."}</button>
            </div>
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Periodic => {
                self.periodic();
            }
            Msg::Playing(pl) => {
                self.is_playing = pl;
                if pl {
                    self.current_block_started_at = unixtime();
                }
            }
            Msg::RateChange => {
                self.current_rate = self
                    .video_el
                    .cast::<HtmlVideoElement>()
                    .unwrap()
                    .playback_rate();
            }
            Msg::NextDeadline => {
                let deadline_cue = b2c(&self.subs.blocks[self.deadline_block_idx]);
                self.deadline_block_idx += 1;
                let elapsed = unixtime() - self.current_block_started_at;
                self.current_block_started_at = unixtime();
                let true_duration = (deadline_cue.end - deadline_cue.start).as_secs_f64();
                log::info!(
                    "Latest block was read in {elapsed}, but was supposed to take {true_duration}"
                );
                self.block_timing_history.push(true_duration / elapsed);

                // Calculate the target rate by averaging the previous rates.
                self.target_rate = self.block_timing_history.iter().cloned().sum::<f64>()
                    / self.block_timing_history.len() as f64;
            }
        }
        true
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            // Here we'll set up the global event listener
            let advance_deadline_block = ctx.link().callback(|_| Msg::NextDeadline);

            let window = web_sys::window().unwrap();
            let listener = EventListener::new(&window, "keydown", {
                move |e| {
                    let e: KeyboardEvent = (e.clone()).dyn_into().unwrap();
                    let keycode = e.key_code();
                    log::info!("Pressed key {keycode}");
                    if keycode == 33 {
                        // page up (prev)
                    } else if keycode == 34 {
                        // page down (next)
                        log::info!("Sending advance event");
                        advance_deadline_block.emit(());
                    }
                }
            });
            self.global_keydown_listener = Some(listener);

            // Also set up the global interval
            let periodic = ctx.link().callback(|_| Msg::Periodic);
            self.interval_callback = Some(Interval::new(100, move || {
                periodic.emit(());
            }));

            // Also set up the high-resolution callback.
            #[wasm_bindgen]
            extern "C" {
                fn request_video_frame_callback(
                    this: &HtmlVideoElement,
                    cb: &Closure<dyn FnMut(JsValue, JsValue)>,
                );
                fn request_video_frame_callback_again();

            }

            let el = self.video_el.cast::<HtmlVideoElement>().unwrap();
            let periodic = ctx.link().callback(|_| Msg::Periodic);
            let cb = Closure::new(move |_now, _metadata| {
                periodic.emit(());
                request_video_frame_callback_again();
            });
            request_video_frame_callback(&el, &cb);
            self.high_res_callback = Some(cb);

            // Also mute the video.
            el.set_muted(true);
        }
    }
}

impl App {
    fn periodic(&mut self) {
        let element: HtmlVideoElement;
        if let Some(v) = self.video_el.get() {
            element = v.dyn_into().unwrap();
        } else {
            return;
        }
        let now = Duration::from_secs_f64(element.current_time());
        self.current_time = now;

        let sub_list = &self.subs.blocks;

        // Loop over the blocks to find one that the value matches.
        let idxs = (self.current_block..sub_list.len()).chain(0..self.current_block);
        for idx in idxs {
            let cue = b2c(&sub_list[idx]);
            // If this cue fits, set this as the current block.
            if fits(now, cue) {
                self.current_block = idx;
                self.current_block_has_passed = false;
                break;
            }
            // Otherwise, if now is after the cue,
            // then this cue is the last visible one.
            if cue.end < now {
                self.current_block = idx.max(self.current_block);
                self.current_block_has_passed = true;
            }
        }

        // Set the playback rate based on the time left until the end of the deadline block.
        let deadline_block = b2c(&sub_list[self.deadline_block_idx]);
        let time_until_end = deadline_block.end.checked_sub(now).unwrap_or_default();
        // let near_curve =
        //     bezier_rs::Bezier::from_cubic_coordinates(0.0, 0.0, 0.0, 0.25, 1.0, 0.0, 1.0, 1.0);
        // let far_curve =
        //     bezier_rs::Bezier::from_cubic_coordinates(1.0, 1.0, 1.6, 1.0, 2.0, 1.5, 2.0, 2.0);

        let rate_fn = |time: Duration| {
            let time_s = time.as_secs_f64();
            let deadline_block_duration = (deadline_block.end - deadline_block.start).as_secs_f64();
            let x = time_s / deadline_block_duration;
            // let advanced_rate = |x: f64, target_rate: f64| {
            //     let k = 5.0;
            //     2.0 * target_rate * (1.0 / (1.0 + std::f64::consts::E.powf(-k * x)) - 0.5)
            // };

            // If the current block on the screen is not the current deadline block, we seek fast to it.
            if self.current_block != self.deadline_block_idx {
                return 2.0 * (time_s / deadline_block_duration) * self.target_rate;
            }

            let slow_threshold = 0.2;

            // If we're past the deadline, stop entirely.
            if x < 0.0 {
                0.0
            } else if x > slow_threshold {
                self.target_rate
            } else {
                // We're in the zone where we need to start slowing down.
                self.target_rate * (x / slow_threshold)
            }

            // if x > 1.0 {
            //     far_curve
            //         .evaluate(bezier_rs::TValue::Parametric((x - 1.0).min(1.0)))
            //         .y
            //         * self.target_rate
            // } else {
            //     advanced_rate(x, self.target_rate)
            // }

            // if x > 1.0 {
            //     return x * advanced_rate(2.0, 1.0);
            // } else {
            //     advanced_rate(x, self.target_rate)
            // }
            // if x > 1.0 {
            //     2.0
            // } else {
            //     advanced_rate(x, self.target_rate)
            // }
        };

        #[wasm_bindgen]
        extern "C" {
            fn try_set_playback_rate(el: &HtmlVideoElement, rate: f64) -> bool;
        }
        if !try_set_playback_rate(&element, rate_fn(time_until_end)) {
            // Playback rate was bad, but we can ignore that.
        }
    }
}

fn b2c(b: &Block) -> &Cue {
    match b {
        Block::Cue(v) => v,
    }
}

fn fits(time: Duration, cue: &Cue) -> bool {
    return cue.start <= time && time <= cue.end;
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
