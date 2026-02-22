use std::sync::{Arc, Mutex};

use edgelord::port::{Event, Notifier};

/// Thread-safe event collector for notification assertions in tests.
#[derive(Clone, Default)]
pub struct RecordingNotifier {
    events: Arc<Mutex<Vec<Event>>>,
}

impl RecordingNotifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.events.lock().expect("lock notifier events").len()
    }
}

impl Notifier for RecordingNotifier {
    fn notify(&self, event: Event) {
        self.events
            .lock()
            .expect("lock notifier events")
            .push(event);
    }
}
