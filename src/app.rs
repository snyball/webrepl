use std::io;

use gloo::{utils::{document, window}, timers::callback::Timeout};
use yew::{prelude::*, html::Scope};
use spaik::repl::REPL;
use web_sys::{HtmlElement, MutationObserver};

const STARTUP_CODE: [&'static str; 1] = [
    r#"(range (n (0 7)) (let ((th (nth (vec "th" "st" "nd" "rd") n "th"))) (println "Hello world, for the {n}{th} time!")))"#,
    // r#"(println "Hello, World!")"#,
];

#[derive(Debug)]
enum HistElem {
    Prompt(String),
    Result(String),
    Error(String),
    Output(String),
}

pub struct App {
    hist: Vec<HistElem>,
    link: Scope<Self>,
    prompt_ref: NodeRef,
    repl: REPL,
    hist_idx: Option<usize>,
}

pub enum Msg {
    Eval(String),
    Output(String),
    HistPrev,
    HistNext,
    ScrollBottom,
}

#[derive(Debug)]
struct OutWriter {
    link: Scope<App>,
    buffer: Vec<u8>,
}

impl OutWriter {
    fn new(link: Scope<App>) -> OutWriter {
        OutWriter {
            buffer: Vec::new(),
            link
        }
    }
}

impl io::Write for OutWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend(buf);

        if let Some(i) = self.buffer.iter().rposition(|x| *x == b'\n') {
            let (first, _last) = self.buffer.split_at(i+1);
            let s = String::from_utf8_lossy(first);
            self.link.send_message(Msg::Output(s.into_owned()));
            self.buffer.drain(..=i).for_each(drop);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let s = String::from_utf8_lossy(&self.buffer);
        self.link.send_message(Msg::Output(s.into_owned()));
        self.buffer.clear();

        Ok(())
    }
}

fn scroll_bottom() {
    let console = document().get_element_by_id("repl-console").unwrap();
    console.set_scroll_top(console.scroll_height());
}

fn scroll_timeout() {
    let console = document().get_element_by_id("repl-console").unwrap();
    let scroll_height = console.scroll_height();
    let scroll_top = console.scroll_top();
    if scroll_height != scroll_top {
        console.set_scroll_top(scroll_height);
    } else {
        Timeout::new(16, scroll_timeout).forget();
    }
}

impl Component for App {
    type Message = Msg;

    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let app = App {
            hist: Default::default(),
            link: ctx.link().clone(),
            prompt_ref: Default::default(),
            repl: REPL::new(Some(Box::new(OutWriter::new(ctx.link().clone())))),
            hist_idx: None,
        };
        for line in STARTUP_CODE {
            ctx.link().send_message(Msg::Eval(line.to_string()));
        }
        app
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Eval(code) => self.eval(code),
            Msg::Output(out) => {
                self.hist.push(HistElem::Output(out));
            },
            Msg::HistNext => self.hist_next(),
            Msg::HistPrev => self.hist_prev(),
            Msg::ScrollBottom => {
                scroll_bottom();
                return false
            },
        }
        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        let link = self.link.clone();
        let onkeydown = move |ev: KeyboardEvent| {
            link.send_message(Msg::ScrollBottom);
            match ev.key().as_str() {
                "Enter" => {
                    let elem = document().get_element_by_id("prompt").unwrap();
                    let text = elem.text_content().unwrap_or_default();
                    elem.set_inner_html("<br/>");
                    link.send_message(Msg::Eval(text));
                }
                "ArrowUp" => link.send_message(Msg::HistPrev),
                "ArrowDown" => link.send_message(Msg::HistNext),
                _ => return,
            }
            ev.prevent_default();
        };

        let prompt_ref = self.prompt_ref.clone();
        let onclick = move |_| {
            let is_range = window().get_selection().unwrap().map(|s| {
                s.type_() == "Range"
            }).unwrap_or_default();
            if !is_range {
                let elem: HtmlElement = prompt_ref.cast().unwrap();
                elem.focus().unwrap();
            }
        };

        Timeout::new(10, scroll_timeout).forget();

        html! {
            <div id="repl-console" class="repl-console" {onclick}>
                <ul class="history">
                    {for self.hist.iter().map(|h| self.view_hist(h))}
                </ul>
                <div id="prompt-container" class="prompt-container">
                    <div id="prompt" class="prompt" ref={&self.prompt_ref} contenteditable="true" {onkeydown} autofocus=true>
                        <br/>
                    </div>
                </div>
            </div>
        }
    }
}

impl App {
    fn view_hist(&self, h: &HistElem) -> Html {
        match h {
            HistElem::Prompt(s) => html! {
                <div class="prompt">{s}</div>
            },
            HistElem::Result(s) => html! {
                <div class="result">{s}</div>
            },
            HistElem::Error(e) => html! {
                <div class="error"><pre>{e}</pre></div>
            },
            HistElem::Output(out) => html! {
                <div class="output"><pre>{out}</pre></div>
            }
        }
    }

    fn eval(&mut self, code: String) {
        let res = self.repl.eval(&code);
        self.hist.push(HistElem::Prompt(code));
        match res {
            Ok(Some(res)) => self.hist.push(HistElem::Result(res)),
            Err(e) => self.hist.push(HistElem::Error(e)),
            Ok(None) => ()
        }
        self.hist_bottom();
    }

    fn set_prompt_text(&self, code: &str) {
        let elem: HtmlElement = self.prompt_ref.cast().unwrap();
        elem.set_inner_text(code);
        self.move_caret_end();
    }

    fn hist_prev(&mut self) {
        let mut idx = self.hist_idx.unwrap_or_else(|| self.hist.len()) as isize - 1;
        if idx == -1 { return }
        let p = loop {
            if let HistElem::Prompt(p) = &self.hist[idx as usize] {
                if !p.trim().is_empty() {
                    break p;
                }
            }
            if idx == 0 { return }
            idx -= 1;
        };
        self.set_prompt_text(p);
        self.hist_idx = Some(idx as usize);
    }

    fn hist_next(&mut self) {
        let Some(mut idx) = self.hist_idx else { return };
        while idx+1 < self.hist.len() {
            idx += 1;
            if let HistElem::Prompt(p) = &self.hist[idx] {
                if !p.trim().is_empty() {
                    self.set_prompt_text(p);
                    self.hist_idx = Some(idx);
                    return;
                }
            }
        }
    }

    fn hist_bottom(&mut self) {
        self.hist_idx = None
    }

    fn move_caret_end(&self) {
        let range = document().create_range().unwrap();
        let Some(node) = self.prompt_ref.get() else { return };
        range.select_node_contents(&node).unwrap();
        range.collapse();
        let selection = window().get_selection().unwrap().unwrap();
        selection.remove_all_ranges().unwrap();
        selection.add_range(&range).unwrap();
    }
}
