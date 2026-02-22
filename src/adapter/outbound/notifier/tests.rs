use crate::port::{
    outbound::notifier::Event, outbound::notifier::Notifier, outbound::notifier::NotifierRegistry,
    outbound::notifier::NullNotifier,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct CountingNotifier {
    count: Arc<AtomicUsize>,
}

impl Notifier for CountingNotifier {
    fn notify(&self, _event: Event) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_registry_notify_all() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut registry = NotifierRegistry::new();

    registry.register(Box::new(CountingNotifier {
        count: count.clone(),
    }));
    registry.register(Box::new(CountingNotifier {
        count: count.clone(),
    }));

    registry.notify_all(Event::CircuitBreakerReset);

    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_null_notifier() {
    let notifier = NullNotifier;
    notifier.notify(Event::CircuitBreakerReset);
}

#[test]
fn test_registry_len_and_is_empty() {
    let mut registry = NotifierRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);

    registry.register(Box::new(NullNotifier));
    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
}
