use gloo::{events::EventListener, utils::format::JsValueSerdeExt};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::MessageEvent;
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn post_message(data: JsValue);
}

#[derive(Default)]
pub struct AuxApp {
    global_msg_listener: Option<EventListener>,
    global_keydown_listener: Option<EventListener>,
    current_video_time: f64,
    context: CueContext,
    current_video_rate: f64,
    is_playing: bool,
}

pub enum AuxAppMsg {
    ReceivedMessage(MainToAuxInterop),
    WantingToSend(AuxToMainInterop),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CueContext {
    pub prev: Vec<String>,
    pub current: String,
    pub current_idx: i32,
    pub next: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MainToAuxInterop {
    CurrentStatus { time: f64, rate: f64, playing: bool },
    CueContext(CueContext),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuxToMainInterop {
    AdvanceDeadline,
    SetIsPlaying(bool),
    ResetRate,
}

impl Component for AuxApp {
    type Message = AuxAppMsg;

    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            global_msg_listener: None,
            current_video_time: 0.0,
            context: CueContext::default(),
            ..Default::default()
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let next_deadline = ctx.link().callback(|ev: MouseEvent| {
            ev.prevent_default();
            AuxAppMsg::WantingToSend(AuxToMainInterop::AdvanceDeadline)
        });

        let do_play = ctx.link().callback(|ev: MouseEvent| {
            ev.prevent_default();
            AuxAppMsg::WantingToSend(AuxToMainInterop::SetIsPlaying(true))
        });
        let do_pause = ctx.link().callback(|ev: MouseEvent| {
            ev.prevent_default();
            AuxAppMsg::WantingToSend(AuxToMainInterop::SetIsPlaying(false))
        });
        let do_reset = ctx.link().callback(|ev: MouseEvent| {
            ev.prevent_default();
            AuxAppMsg::WantingToSend(AuxToMainInterop::ResetRate)
        });

        let cues_prev = self
            .context
            .prev
            .iter()
            .map(|v| html!(<p>{v}</p>))
            .collect::<Html>();
        let cues_next = self
            .context
            .next
            .iter()
            .map(|v| html!(<p>{v}</p>))
            .collect::<Html>();

        html!(
            <div class="container">
                <h1>{"Presenter view"}</h1>
                <p>{"Video time: "}{self.current_video_time}</p>
                <p>{"Playback rate: "}{self.current_video_rate}</p>
                <p>{"Is playing: "}{self.is_playing}</p>
                <button class="btn btn-success" onclick={next_deadline}>{"Next"}</button>
                <button class="btn btn-primary" onclick={do_play}>{"Play"}</button>
                <button class="btn btn-warning" onclick={do_pause}>{"Pause"}</button>
                <button class="btn btn-outline-danger" onclick={do_reset}>{"Reset rate"}</button>
                <div>{cues_prev}</div>
                <p class="text-danger">{&self.context.current}</p>
                <div>{cues_next}</div>
            </div>
        )
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            // Send the keyboard receiver.
            // Here we'll set up the global event listener
            let window = web_sys::window().unwrap();
            let listener = EventListener::new(&window, "keydown", {
                crate::common::event_handler(ctx.link().callback(AuxAppMsg::WantingToSend))
            });
            self.global_keydown_listener = Some(listener);

            // Set up a message receiver.

            let window = gloo::utils::window();
            self.global_msg_listener = Some(EventListener::new(&window, "message", {
                let cb = ctx
                    .link()
                    .callback(|value| AuxAppMsg::ReceivedMessage(value));
                move |e| {
                    let e: MessageEvent = e.clone().dyn_into().unwrap_throw();
                    cb.emit(e.data().into_serde().unwrap());
                }
            }))
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AuxAppMsg::ReceivedMessage(value) => match value {
                MainToAuxInterop::CurrentStatus {
                    time,
                    rate,
                    playing,
                } => {
                    self.current_video_time = time;
                    self.current_video_rate = rate;
                    self.is_playing = playing;
                }
                MainToAuxInterop::CueContext(ctx) => self.context = ctx,
            },
            AuxAppMsg::WantingToSend(value) => {
                post_message(JsValue::from_serde(&value).unwrap_throw())
            }
        };
        true
    }
}
