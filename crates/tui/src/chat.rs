//! Full-screen chat over the router, drawn by OpenTUI: Enter sends, Tab cycles the policy (auto, auto:local,
//! auto:free, auto:cheap), Esc or Ctrl+C quits. The router decides which offer answers and fails over between them;
//! tokens stream into the transcript as they arrive.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use ai_router::Router;
use opentui::prelude::*;
use serde_json::{Value, json};

pub const POLICIES: [&str; 4] = ["auto", "auto:local", "auto:free", "auto:cheap"];

/// A streamed piece of the answer.
struct Token(String);
/// The end of an answer: the router's summary (offer, attempts or error).
struct Done(Value);

/// One-line description of a finished answer.
pub fn describe(summary: &Value) -> String {
    match summary.get("error") {
        Some(error) => format!("failed: {}", error["message"].as_str().unwrap_or("")),
        None => format!("{} ({} attempt(s))", summary["router"]["offer"].as_str().unwrap_or(""), summary["router"]["attempts"].as_array().map_or(0, Vec::len)),
    }
}

/// Builds the chat scene on `app` and returns the input, for tests.
pub fn build(app: &App, router: Arc<Router>) -> Input {
    let theme = app.theme();
    let header = Text::new(app, format!(" open · {} usable offers", router.models("auto").len())).fg(theme.accent).bold();
    let log = ScrollBox::new(app).sticky(true).grow(1.0);
    let status = Text::new(app, " Enter send · Tab policy · Esc quit").fg(theme.muted);
    let policy_label = Text::new(app, format!(" {} ", POLICIES[0])).fg(theme.accent);
    let input = Input::new(app).placeholder("Ask anything").grow(1.0);
    let prompt = View::new(app).row().border(BorderStyle::Rounded).border_color(theme.accent).child(&policy_label).child(&input);
    app.root().add(&header).add(&log).add(&prompt).add(&status);

    let policy = Rc::new(Cell::new(0usize));
    let turns: Rc<RefCell<Vec<Value>>> = Rc::default();
    let answer: Rc<RefCell<Option<(Text, String)>>> = Rc::default();
    let busy = Rc::new(Cell::new(false));

    app.on_message({
        let answer = answer.clone();
        move |_cx, Token(piece): &Token| {
            if let Some((text, so_far)) = answer.borrow_mut().as_mut() {
                so_far.push_str(piece);
                text.set_text(so_far.clone());
            }
        }
    });
    app.on_message({
        let (answer, turns, busy, status) = (answer.clone(), turns.clone(), busy.clone(), status.clone());
        move |_cx, Done(summary): &Done| {
            if let Some((_, text)) = answer.borrow_mut().take() {
                turns.borrow_mut().push(json!({ "role": "assistant", "content": text }));
            }
            busy.set(false);
            status.set_text(format!(" {}", describe(summary)));
        }
    });

    input.on_submit({
        let (app, log, input, status, policy, turns, answer, busy) = (app.clone(), log.clone(), input.clone(), status.clone(), policy.clone(), turns.clone(), answer.clone(), busy.clone());
        move |_cx, value| {
            let line = value.trim().to_owned();
            if line.is_empty() || busy.get() {
                return;
            }
            input.set_value("");
            log.add(&Text::new(&app, format!("You: {line}")).fg(theme.foreground).bold());
            let reply = Text::new(&app, "").fg(theme.foreground);
            log.add(&reply);
            *answer.borrow_mut() = Some((reply, String::new()));
            turns.borrow_mut().push(json!({ "role": "user", "content": line }));
            let request = json!({ "model": POLICIES[policy.get()], "messages": *turns.borrow() });
            busy.set(true);
            status.set_text(" answering");
            let (proxy, router) = (app.proxy(), router.clone());
            std::thread::spawn(move || {
                let summary = router.chat_stream(&request, &mut |chunk| {
                    if let Some(piece) = chunk["choices"][0]["delta"]["content"].as_str() {
                        let _ = proxy.send(Token(piece.to_owned()));
                    }
                });
                let _ = proxy.send(Done(summary));
            });
        }
    });
    app.on_key_capture({
        let policy_label = policy_label.clone();
        move |cx, key| {
            if key.matches("escape") || key.matches("ctrl+c") {
                cx.quit();
            } else if key.matches("tab") {
                policy.set((policy.get() + 1) % POLICIES.len());
                policy_label.set_text(format!(" {} ", POLICIES[policy.get()]));
                cx.stop_propagation();
            }
        }
    });
    input.focus();
    input
}

/// Runs the chat on the terminal until the user quits.
pub fn run(router: Arc<Router>) -> opentui::Result<()> {
    let app = App::new(AppOptions::default())?;
    build(&app, router);
    app.run()
}
