use crate::runtime::{Host, Ingress};
use float_view_backend_service::os::{Effect, Fact};
use gtk::{gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
use webkit6::prelude::*;

fn allowed_navigation(uri: &str, origin: &str) -> bool {
    uri == origin
        || uri
            .strip_prefix(origin)
            .is_some_and(|tail| tail.starts_with('/') || tail.starts_with('#'))
}
pub fn open(
    app: &gtk::Application,
    host: Rc<Host>,
    browser: bool,
) -> (gtk::ApplicationWindow, webkit6::WebView) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Float View")
        .default_width(480)
        .default_height(860)
        .build();
    let origin = format!("http://127.0.0.1:{}", host.server.port());
    let web = webkit6::WebView::new();
    let crashes = Rc::new(Cell::new(0));
    let app_origin = origin.clone();
    let navigation_window = window.downgrade();
    web.connect_decide_policy(move |_, decision, kind| {
        if let Some(navigation) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() {
            let mut action = navigation.navigation_action();
            let user_gesture = action.as_mut().is_some_and(|a| a.is_user_gesture());
            let uri = action.and_then(|mut a| a.request()).and_then(|r| r.uri());
            if uri
                .as_deref()
                .is_none_or(|uri| !allowed_navigation(uri, &app_origin))
            {
                decision.ignore();
                if user_gesture
                    && let Some(uri) = uri
                    && (uri.starts_with("https://") || uri.starts_with("http://"))
                {
                    gtk::UriLauncher::new(&uri).launch(
                        navigation_window.upgrade().as_ref(),
                        None::<&gio::Cancellable>,
                        |_| {},
                    );
                }
                return true;
            }
        }
        if kind == webkit6::PolicyDecisionType::NewWindowAction {
            decision.ignore();
            return true;
        }
        false
    });
    let failed_window = window.downgrade();
    web.connect_web_process_terminated(move |web, _| {
        if crashes.get() < 3 {
            crashes.set(crashes.get() + 1);
            web.reload();
        } else if let Some(window) = failed_window.upgrade() {
            let label = gtk::Label::new(Some(
                "The display stopped responding. Close and reopen Float View.",
            ));
            label.set_wrap(true);
            window.set_child(Some(&label));
        }
    });
    let preference = Rc::new(RefCell::new(
        host.process
            .service
            .lock()
            .expect("service")
            .published_snapshot()
            .settings
            .theme_preference,
    ));
    let system_dark = Rc::new(Cell::new(
        gtk::Settings::default().is_some_and(|s| s.is_gtk_application_prefer_dark_theme()),
    ));
    let css = gtk::CssProvider::new();
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    apply_theme(&web, &css, &preference.borrow(), system_dark.get());
    subscribe_theme(
        web.clone(),
        css.clone(),
        preference.clone(),
        system_dark.clone(),
    );
    if browser {
        let label = gtk::Label::new(Some(
            "Float View browser development is running.\nClosing this window stops the backend.",
        ));
        label.set_wrap(true);
        window.set_child(Some(&label));
    } else {
        window.set_child(Some(&web));
        web.load_uri(&origin);
    }
    let input = host.ingress.clone();
    window.connect_is_active_notify(move |w| {
        let _ = input.send(Fact::Foreground {
            foreground: w.is_active(),
        });
    });
    let dialogs = Rc::new(RefCell::new(Vec::<(u64, gio::Cancellable)>::new()));
    let cancel = dialogs.clone();
    let input = host.ingress.clone();
    window.connect_close_request(move |_| {
        for (id, cancel) in cancel.borrow_mut().drain(..) {
            cancel.cancel();
            let _ = input.cleanup(Fact::PresentationFinished { id });
        }
        glib::Propagation::Proceed
    });
    subscribe_suspend(host.ingress.clone());
    let result = (window.clone(), web.clone());
    let weak_window = window.downgrade();
    glib::timeout_add_local(Duration::from_millis(30), move || {
        let Some(window) = weak_window.upgrade() else {
            return glib::ControlFlow::Break;
        };
        for effect in host.ui.try_iter().take(8) {
            match effect {
                Effect::HostState {
                    theme_preference, ..
                } => {
                    *preference.borrow_mut() = theme_preference;
                    apply_theme(&web, &css, &preference.borrow(), system_dark.get());
                }
                Effect::PresentFile {
                    id,
                    path,
                    file_name,
                } => present(&window, &host.ingress, &dialogs, id, &path, &file_name),
                _ => {}
            }
        }
        glib::ControlFlow::Continue
    });
    window.present();
    result
}
// Lifecycle subscription is installed separately so it retains only the bounded ingress.
pub fn subscribe_suspend(input: Ingress) {
    glib::MainContext::default().spawn_local(async move {
        if let Ok(bus) = gio::bus_get_future(gio::BusType::System).await {
            let subscription = bus.subscribe_to_signal(
                Some("org.freedesktop.login1"),
                Some("org.freedesktop.login1.Manager"),
                Some("PrepareForSleep"),
                Some("/org/freedesktop/login1"),
                None,
                gio::DBusSignalFlags::NONE,
                move |signal| {
                    if let Some((sleeping,)) = signal.parameters.get::<(bool,)>() {
                        let _ = input.cleanup(Fact::Bluetooth {
                            radio: if sleeping {
                                float_view_backend_service::os::Radio::Resetting
                            } else {
                                float_view_backend_service::os::Radio::Unknown
                            },
                            scan: float_view_backend_service::os::Permission::Unknown,
                            connect: float_view_backend_service::os::Permission::NotRequired,
                        });
                    }
                },
            );
            // The subscription's connection must outlive this future.
            std::future::pending::<()>().await;
            drop(subscription);
            drop(bus);
        }
    });
}
fn apply_theme(
    web: &webkit6::WebView,
    css: &gtk::CssProvider,
    preference: &str,
    system_dark: bool,
) {
    let dark = preference == "dark" || preference == "system" && system_dark;
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_application_prefer_dark_theme(dark);
    }
    let color = if dark { "#020617" } else { "#f8fafc" };
    web.set_background_color(&gtk::gdk::RGBA::parse(color).expect("canvas"));
    css.load_from_data(&format!("window {{ background-color: {color}; }}"));
}
type Dialogs = Rc<RefCell<Vec<(u64, gio::Cancellable)>>>;
fn finish_presentation(dialogs: &Dialogs, input: &Ingress, id: u64) {
    let mut active = dialogs.borrow_mut();
    let found = active.iter().any(|(key, _)| *key == id);
    active.retain(|(key, _)| *key != id);
    if found {
        let _ = input.cleanup(Fact::PresentationFinished { id });
    }
}
fn present(
    window: &gtk::ApplicationWindow,
    input: &Ingress,
    dialogs: &Dialogs,
    id: u64,
    path: &str,
    name: &str,
) {
    if !dialogs.borrow().is_empty() {
        let _ = input.cleanup(Fact::PresentationStarted {
            id,
            accepted: false,
        });
        return;
    }
    let cancel = gio::Cancellable::new();
    dialogs.borrow_mut().push((id, cancel.clone()));
    let dialog = gtk::FileDialog::builder()
        .title("Save export")
        .initial_name(name)
        .build();
    let input = input.clone();
    let dialogs = dialogs.clone();
    let source = gio::File::for_path(path);
    let parent = window.clone();
    let _ = input.cleanup(Fact::PresentationStarted { id, accepted: true });
    dialog.save(Some(window), Some(&cancel.clone()), move |result| {
        match result {
            Ok(destination) => {
                let final_input = input.clone(); let final_dialogs = dialogs.clone();
                source.copy_async(&destination, gio::FileCopyFlags::OVERWRITE, glib::Priority::DEFAULT,
                    Some(&cancel), None, move |result| {
                        if result.is_err() && parent.is_visible() {
                            gtk::AlertDialog::builder().message("Could not save export").detail("The source session remains available. Choose another destination and try again.").build().show(Some(&parent));
                        }
                        finish_presentation(&final_dialogs, &final_input, id);
                    });
            }
            Err(_) => finish_presentation(&dialogs, &input, id),
        }
    });
}

fn subscribe_theme(
    web: webkit6::WebView,
    css: gtk::CssProvider,
    preference: Rc<RefCell<String>>,
    dark: Rc<Cell<bool>>,
) {
    glib::MainContext::default().spawn_local(async move {
        let Ok(bus) = gio::bus_get_future(gio::BusType::Session).await else {
            return;
        };
        let parameters = ("org.freedesktop.appearance", "color-scheme").to_variant();
        if let Ok(reply) = bus
            .call_future(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                "org.freedesktop.portal.Settings",
                "Read",
                Some(&parameters),
                None,
                gio::DBusCallFlags::NONE,
                2_000,
            )
            .await
            && let Some((value,)) = reply.get::<(glib::Variant,)>()
            && let Some(value) = value.get::<u32>()
        {
            dark.set(value == 1);
            apply_theme(&web, &css, &preference.borrow(), dark.get());
        }
        let subscription = bus.subscribe_to_signal(
            Some("org.freedesktop.portal.Desktop"),
            Some("org.freedesktop.portal.Settings"),
            Some("SettingChanged"),
            Some("/org/freedesktop/portal/desktop"),
            None,
            gio::DBusSignalFlags::NONE,
            move |signal| {
                if let Some((namespace, key, value)) =
                    signal.parameters.get::<(String, String, glib::Variant)>()
                    && namespace == "org.freedesktop.appearance"
                    && key == "color-scheme"
                    && let Some(value) = value.get::<u32>()
                {
                    dark.set(value == 1);
                    apply_theme(&web, &css, &preference.borrow(), dark.get());
                }
            },
        );
        std::future::pending::<()>().await;
        drop(subscription);
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_completion_after_cancel_is_single_use() {
        let (ordinary, _ordinary_rx) = tokio::sync::mpsc::channel(1);
        let (cleanup, mut cleanup_rx) = tokio::sync::mpsc::channel(2);
        let input = Ingress { ordinary, cleanup };
        let dialogs = Rc::new(RefCell::new(vec![(42, gio::Cancellable::new())]));
        finish_presentation(&dialogs, &input, 41);
        assert!(cleanup_rx.try_recv().is_err());
        finish_presentation(&dialogs, &input, 42);
        finish_presentation(&dialogs, &input, 42);
        assert!(matches!(
            cleanup_rx.try_recv(),
            Ok(Fact::PresentationFinished { id: 42 })
        ));
        assert!(cleanup_rx.try_recv().is_err());
        assert!(dialogs.borrow().is_empty());
    }
    #[test]
    fn navigation_requires_exact_origin_boundary() {
        let origin = "http://127.0.0.1:11013";
        assert!(allowed_navigation("http://127.0.0.1:11013/", origin));
        for uri in [
            "http://127.0.0.1:110130/",
            "http://127.0.0.1:11013.evil/",
            "https://example.com",
            "file:///tmp/a",
        ] {
            assert!(!allowed_navigation(uri, origin));
        }
    }
}

#[cfg(test)]
mod desktop_tests {
    use super::*;
    #[test]
    #[ignore = "requires a real desktop session; writes WebKit snapshots under /tmp"]
    fn desktop_webview_theme_reload_and_navigation() {
        gtk::init().expect("desktop GTK session");
        let app = gtk::Application::builder()
            .application_id("fail.acheron.floatview.desktopcheck")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let root =
            std::env::temp_dir().join(format!("float-view-linux-desktop-{}", std::process::id()));
        let host = Rc::new(
            Host::open_on(
                &root,
                include_bytes!(concat!(env!("OUT_DIR"), "/index.html")),
                false,
                0,
            )
            .unwrap(),
        );
        let lifetime = host.process.service.lock().unwrap().lifetime();
        assert_eq!(
            host.process
                .service
                .lock()
                .unwrap()
                .settings()
                .unwrap()
                .theme_preference,
            "system"
        );
        let initially_dark = gtk::Settings::default()
            .unwrap()
            .is_gtk_application_prefer_dark_theme();
        let (window, web) = open(&app, host.clone(), false);
        assert_eq!(
            web.background_color(),
            gtk::gdk::RGBA::parse(if initially_dark { "#020617" } else { "#f8fafc" }).unwrap()
        );
        let context = glib::MainContext::default();
        context.block_on(async {
            for _ in 0..100 {
                glib::timeout_future(Duration::from_millis(100)).await;
                if web.evaluate_javascript_future("!!document.querySelector('nav')", None, None).await.is_ok_and(|v| v.to_boolean()) { break; }
            }
            assert!(web.evaluate_javascript_future("!!document.querySelector('nav')", None, None).await.unwrap().to_boolean(), "frontend navigation rendered");
            for theme in ["dark", "light"] {
                let script = format!(r#"window.linuxCheckDone = false;
                    (async () => {{
                        const b = await (await fetch('/api/bootstrap')).json();
                        const ws = new WebSocket(b.webSocketUrl + '?token=' + encodeURIComponent(b.sessionToken));
                        let id = 0;
                        const pending = new Map();
                        ws.onmessage = e => {{ const v = JSON.parse(e.data); if (v.id) {{ const p = pending.get(v.id); if (p) {{ pending.delete(v.id); v.error ? p.reject(v.error) : p.resolve(v.result); }} }} }};
                        await new Promise((resolve, reject) => {{ ws.onopen = resolve; ws.onerror = reject; }});
                        const request = (method, params) => new Promise((resolve, reject) => {{ const key = ++id; pending.set(key, {{resolve, reject}}); ws.send(JSON.stringify({{id:key, method, params}})); }});
                        const state = await request('state.get', {{}});
                        await request('settings.update', {{expectedRevision:state.settings.revision, patch:{{themePreference:'{theme}'}}}});
                        ws.close(); window.linuxCheckDone = true;
                    }})().catch(() => {{ window.linuxCheckDone = 'failed'; }}); true;"#);
                web.evaluate_javascript_future(&script, None, None).await.unwrap();
                for _ in 0..50 {
                    glib::timeout_future(Duration::from_millis(100)).await;
                    if web.evaluate_javascript_future("window.linuxCheckDone === true", None, None).await.unwrap().to_boolean() { break; }
                }
                assert!(web.evaluate_javascript_future("window.linuxCheckDone === true", None, None).await.unwrap().to_boolean());
                glib::timeout_future(Duration::from_millis(300)).await;
                let expected = gtk::gdk::RGBA::parse(if theme == "dark" { "#020617" } else { "#f8fafc" }).unwrap();
                assert_eq!(web.background_color(), expected);
                web.snapshot_future(webkit6::SnapshotRegion::Visible, webkit6::SnapshotOptions::NONE).await.unwrap()
                    .save_to_png(format!("/tmp/float-view-linux-{theme}.png")).unwrap();
            }
            web.reload();
            glib::timeout_future(Duration::from_secs(1)).await;
            assert!(web.evaluate_javascript_future("!!document.querySelector('nav')", None, None).await.unwrap().to_boolean());
            web.terminate_web_process();
            glib::timeout_future(Duration::from_secs(2)).await;
            assert!(web.evaluate_javascript_future("!!document.querySelector('nav')", None, None).await.unwrap().to_boolean());
            assert_eq!(host.process.service.lock().unwrap().lifetime(), lifetime);
            web.load_uri("https://example.invalid/");
            glib::timeout_future(Duration::from_millis(300)).await;
            assert!(web.uri().unwrap().starts_with("http://127.0.0.1:"));
            window.minimize();
            glib::timeout_future(Duration::from_millis(300)).await;
            assert_eq!(host.process.service.lock().unwrap().lifetime(), lifetime);
            window.present();
        });
        window.close();
        while context.pending() {
            context.iteration(false);
        }
        host.shutdown().unwrap();
        drop(host);
        drop(window);
        drop(web);
        context.block_on(glib::timeout_future(Duration::from_millis(60)));
        // A persisted explicit choice must win before the new document renders,
        // even when the native toolkit initially reports the opposite appearance.
        gtk::Settings::default()
            .unwrap()
            .set_gtk_application_prefer_dark_theme(true);
        let restarted = Rc::new(
            Host::open_on(
                &root,
                include_bytes!(concat!(env!("OUT_DIR"), "/index.html")),
                false,
                0,
            )
            .unwrap(),
        );
        let (window, web) = open(&app, restarted.clone(), false);
        assert_eq!(
            web.background_color(),
            gtk::gdk::RGBA::parse("#f8fafc").unwrap()
        );
        assert!(
            !gtk::Settings::default()
                .unwrap()
                .is_gtk_application_prefer_dark_theme()
        );
        {
            let mut service = restarted.process.service.lock().unwrap();
            let revision = service.settings().unwrap().revision;
            service
                .dispatch(
                    1,
                    float_view_backend_service::operations::Operation::UpdateSettings {
                        revision,
                        patch_json: r#"{"themePreference":"dark"}"#.into(),
                    },
                )
                .unwrap();
        }
        window.close();
        restarted.shutdown().unwrap();
        drop(window);
        drop(web);
        drop(restarted);
        context.block_on(glib::timeout_future(Duration::from_millis(60)));
        gtk::Settings::default()
            .unwrap()
            .set_gtk_application_prefer_dark_theme(false);
        let restarted = Rc::new(
            Host::open_on(
                &root,
                include_bytes!(concat!(env!("OUT_DIR"), "/index.html")),
                false,
                0,
            )
            .unwrap(),
        );
        let (window, web) = open(&app, restarted.clone(), false);
        assert_eq!(
            web.background_color(),
            gtk::gdk::RGBA::parse("#020617").unwrap()
        );
        assert!(
            gtk::Settings::default()
                .unwrap()
                .is_gtk_application_prefer_dark_theme()
        );
        window.close();
        restarted.shutdown().unwrap();
        drop(window);
        drop(web);
        drop(restarted);
        context.block_on(glib::timeout_future(Duration::from_millis(60)));
        std::fs::remove_dir_all(root).unwrap();
    }
}
