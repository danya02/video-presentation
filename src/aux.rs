use gloo::events::EventListener;
use wasm_bindgen::prelude::*;
use web_sys::MessageEvent;
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn post_message(data: JsValue);
}

pub struct AuxApp {
    global_msg_listener: Option<EventListener>,
}

pub enum AuxAppMsg {
    ReceivedMessage(JsValue),
}

impl Component for AuxApp {
    type Message = AuxAppMsg;

    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            global_msg_listener: None,
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let send_msg = Callback::from(|ev: MouseEvent| {
            ev.prevent_default();
            post_message("hello".into());
        });

        html!(
            <button class="btn btn-success" onclick={send_msg}>{"Click!"}</button>
        )
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            // Set up a message receiver.

            let window = gloo::utils::window();
            self.global_msg_listener = Some(EventListener::new(&window, "message", {
                let cb = ctx
                    .link()
                    .callback(|value| AuxAppMsg::ReceivedMessage(value));
                move |e| {
                    let e: MessageEvent = e.clone().dyn_into().unwrap_throw();
                    cb.emit(e.data());
                }
            }))
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AuxAppMsg::ReceivedMessage(value) => gloo::console::log!(value),
        };
        true
    }
}
