#![forbid(unsafe_code)]
mod bluetooth;
mod runtime;
#[cfg(feature = "software-bluetooth")]
mod software;
mod ui;
use gtk::{gio, prelude::*};
use std::{cell::RefCell, rc::Rc};
fn main() -> gtk::glib::ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| {
        a != "--browser-development"
            && !(cfg!(feature = "software-bluetooth") && a == "--software-conformance")
    }) {
        eprintln!("Usage: float-view-linux [--browser-development]");
        return gtk::glib::ExitCode::FAILURE;
    }
    let software = args.iter().any(|a| a == "--software-conformance");
    let browser = args.iter().any(|a| a == "--browser-development");
    let application = gtk::Application::builder()
        .application_id("fail.acheron.floatview")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let host = Rc::new(RefCell::new(None::<Rc<runtime::Host>>));
    let owner = host.clone();
    application.connect_activate(move |app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        let root = runtime::data_root(
            std::env::var("XDG_DATA_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        );
        let result = root.and_then(|root| {
            runtime::Host::open(
                &root,
                include_bytes!(concat!(env!("OUT_DIR"), "/index.html")),
                software,
            )
        });
        match result {
            Ok(value) => {
                let value = Rc::new(value);
                ui::open(app, value.clone(), browser);
                *owner.borrow_mut() = Some(value);
            }
            Err(error) => {
                eprintln!("{error}");
                let window = gtk::ApplicationWindow::builder()
                    .application(app)
                    .title("Float View — startup failed")
                    .default_width(520)
                    .default_height(180)
                    .build();
                let label = gtk::Label::new(Some(error));
                label.set_wrap(true);
                label.set_margin_top(24);
                label.set_margin_bottom(24);
                window.set_child(Some(&label));
                window.present();
            }
        }
    });
    application.connect_shutdown(move |app| {
        for window in app.windows() {
            window.close();
        }
        if let Some(host) = host.borrow_mut().take()
            && let Err(error) = host.shutdown()
        {
            eprintln!("{error}");
        }
    });
    for signal in [2, 15] {
        let app = application.clone();
        gtk::glib::unix_signal_add_local(signal, move || {
            app.quit();
            gtk::glib::ControlFlow::Break
        });
    }
    application.run_with_args::<&str>(&[])
}
