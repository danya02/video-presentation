use shadow_clone::shadow_clone;
use web_sys::wasm_bindgen::JsCast;
use web_sys::HtmlVideoElement;
use yew::prelude::*;

#[function_component]
fn App() -> Html {
    let is_playing = use_state(|| false);
    let current_time = use_state(|| 0.0);

    let ontimeupdate = Callback::from({
        shadow_clone!(current_time);
        move |ev: Event| {
            let element: HtmlVideoElement = ev.target().unwrap().dyn_into().unwrap();
            current_time.set(element.current_time());
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

    html! {
        <div class="container">
            <video src="http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4" controls={true}
            {ontimeupdate} {onplay} {onpause}/>
            <hr />
            <p>{"Current time: "}{*current_time}</p>
            <p>{"Is playing: "}{*is_playing}</p>
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
