use std::path::Path;

use gtk::gio;
use gtk::prelude::*;

use crate::settings::HeaderMode;

pub(crate) fn watch(
    path: &Path,
    apply: impl Fn(HeaderMode) + 'static,
) -> Result<gio::FileMonitor, gtk::glib::Error> {
    let file = gio::File::for_path(path);
    // Watch the directory because settings saves replace the file atomically.
    let directory = file.parent().expect("settings file has a parent directory");
    let monitor = directory.monitor_directory(
        gio::FileMonitorFlags::WATCH_MOVES,
        None::<&gio::Cancellable>,
    )?;
    let path = path.to_owned();
    monitor.connect_changed(move |_, changed, other, _| {
        if !changed.equal(&file) && !other.is_some_and(|other| other.equal(&file)) {
            return;
        }
        // Ignore partial/invalid files instead of replacing the live mode with defaults.
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return;
        };
        let mode = match value.get("header_mode").and_then(|value| value.as_str()) {
            Some("full") => HeaderMode::Full,
            Some("mini") => HeaderMode::Mini,
            Some("hidden") => HeaderMode::Hidden,
            _ => return,
        };
        apply(mode);
    });
    Ok(monitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    #[test]
    fn watches_repeated_atomic_replacements_and_ignores_unrelated_files() {
        let directory =
            std::env::temp_dir().join(format!("zter-header-watch-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        std::fs::write(&path, r#"{"header_mode":"full"}"#).unwrap();
        let context = gtk::glib::MainContext::new();
        context
            .with_thread_default(|| {
                let received = Rc::new(RefCell::new(Vec::new()));
                let events = received.clone();
                let monitor = watch(&path, move |mode| events.borrow_mut().push(mode)).unwrap();
                let pump = || {
                    let deadline = Instant::now() + Duration::from_millis(350);
                    while Instant::now() < deadline {
                        while context.pending() {
                            context.iteration(false);
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                };
                for (value, expected) in [
                    ("mini", HeaderMode::Mini),
                    ("full", HeaderMode::Full),
                    ("hidden", HeaderMode::Hidden),
                ] {
                    received.borrow_mut().clear();
                    let temporary = directory.join("settings.tmp");
                    std::fs::write(&temporary, format!(r#"{{"header_mode":"{value}"}}"#)).unwrap();
                    std::fs::rename(temporary, &path).unwrap();
                    pump();
                    assert!(!received.borrow().is_empty());
                    assert!(received.borrow().iter().all(|mode| *mode == expected));
                }
                received.borrow_mut().clear();
                std::fs::write(
                    directory.join("settings.json.bak"),
                    r#"{"header_mode":"full"}"#,
                )
                .unwrap();
                pump();
                assert!(received.borrow().is_empty());
                for invalid in ["{", r#"{"header_mode":"bad"}"#, r#"{"font_size":14}"#] {
                    std::fs::write(&path, invalid).unwrap();
                    pump();
                    assert!(received.borrow().is_empty());
                }
                monitor.cancel();
            })
            .unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }
}
