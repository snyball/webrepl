mod app;

use app::App;

fn main() {
    let elem = gloo::utils::document().get_element_by_id("repl-container").unwrap();
    yew::Renderer::<App>::with_root(elem).render();
}
