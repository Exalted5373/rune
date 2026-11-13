//! Closing an app's only window starts its quit, and that window runs `App::on_quit` after its
//! `on_close`, as every window open when a quit starts does. Opens a native window, so it only
//! runs with RUNE_GUI_TESTS=1 in a logged-in GUI session.

use std::sync::Mutex;
use std::time::{Duration, Instant};
use rune::*;

struct Main;

static HOOKS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn view(_: &Main, _: &mut Cx<Main>) -> El { text("main") }

fn main() {
    if std::env::var_os("RUNE_GUI_TESTS").is_none() {
        println!("quit_last_window: skipped (set RUNE_GUI_TESTS=1 in a GUI session)");
        return;
    }
    std::thread::spawn(|| { std::thread::sleep(Duration::from_secs(60)); eprintln!("timeout"); std::process::exit(2) });
    // `run` returns, so the hooks it ran can be checked.
    orderly_exit();
    let app = App::new(AppSpec::new("quit last window", (240.0, 120.0), || Main, view))
        .args(Vec::new())
        .on_close(|_, _| HOOKS.lock().unwrap().push("close"))
        .on_quit(|_, _| HOOKS.lock().unwrap().push(if quitting() { "quit" } else { "quit before the quit started" }));
    let main = app.handle();
    std::thread::spawn(move || {
        let start = Instant::now();
        while !main.is_visible() {
            if start.elapsed() > Duration::from_secs(10) { eprintln!("the window never showed"); std::process::exit(2) }
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(200));
        // As the window manager's close does (`CloseRequested`).
        main.close();
    });
    app.run();
    assert_eq!(*HOOKS.lock().unwrap(), ["close", "quit"], "the last window's close starts the quit, and it runs on_quit");
    println!("quit_last_window: ok");
}
