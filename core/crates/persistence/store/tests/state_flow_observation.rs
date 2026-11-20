use operit_store::PreferencesDataStore::StateFlow;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// Publishes a terminal state while a subscriber is still handling its initial snapshot.
fn terminal_update_during_subscription(compare_and_set: bool) {
    let progress = StateFlow::new("loading".to_string());
    let observed = Arc::new(Mutex::new(Vec::new()));
    let (entered_sender, entered_receiver) = mpsc::channel();
    let (resume_sender, resume_receiver) = mpsc::channel();
    let subscribed_progress = progress.clone();
    let recorded = observed.clone();
    let subscriber = std::thread::spawn(move || {
        subscribed_progress.subscribe(move |value| {
            recorded.lock().unwrap().push(value.clone());
            if value == "loading" {
                entered_sender.send(()).unwrap();
                resume_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
            }
        })
    });
    entered_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    let updated_progress = progress.clone();
    let updater = std::thread::spawn(move || {
        if compare_and_set {
            assert!(updated_progress.compare_and_set("loading".into(), "hidden".into()));
        } else {
            updated_progress.set_value("hidden".into());
        }
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while progress.value() != "hidden" {
        assert!(Instant::now() < deadline, "Terminal state was not published");
        std::thread::yield_now();
    }
    resume_sender.send(()).unwrap();
    let subscription = subscriber.join().unwrap();
    updater.join().unwrap();
    let values = observed.lock().unwrap().clone();
    progress.unsubscribe(subscription);
    assert_eq!(values.first().map(String::as_str), Some("loading"));
    assert_eq!(values.last().map(String::as_str), Some("hidden"));
}

/// Keeps the final loading state observable without requiring another update to wake the watch.
#[test]
fn subscription_delivers_terminal_update_during_initial_snapshot() {
    terminal_update_during_subscription(false);
}

/// Applies the same subscription handoff guarantee to atomic state transformations.
#[test]
fn subscription_delivers_atomic_terminal_update_during_initial_snapshot() {
    terminal_update_during_subscription(true);
}

/// Preserves callbacks that synchronously change the source while handling their initial value.
#[test]
fn initial_callback_can_publish_the_next_state() {
    let source = StateFlow::new(0);
    let publisher = source.clone();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let recorded = observed.clone();
    let subscription = source.subscribe(move |value| {
        recorded.lock().unwrap().push(value);
        if value == 0 {
            publisher.set_value(1);
        }
    });
    source.unsubscribe(subscription);
    assert_eq!(*observed.lock().unwrap(), vec![0, 1]);
}

/// Waits for a publication independently of a callback that is intentionally held busy.
fn wait_for_value(source: &StateFlow<i32>, expected: i32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while source.value() != expected {
        assert!(Instant::now() < deadline, "State {expected} was not published");
        std::thread::yield_now();
    }
}

/// Conflates queued notifications so an older writer cannot restore stale progress after completion.
#[test]
fn concurrent_writers_deliver_the_latest_state_after_a_busy_callback() {
    let source = StateFlow::new(0);
    let observed = Arc::new(Mutex::new(Vec::new()));
    let recorded = observed.clone();
    let (entered_sender, entered_receiver) = mpsc::channel();
    let (resume_sender, resume_receiver) = mpsc::channel();
    let subscription = source.subscribe(move |value| {
        recorded.lock().unwrap().push(value);
        if value == 1 {
            entered_sender.send(()).unwrap();
            resume_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        }
    });
    let first_source = source.clone();
    let first = std::thread::spawn(move || first_source.set_value(1));
    entered_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    let second_source = source.clone();
    let second = std::thread::spawn(move || second_source.set_value(2));
    wait_for_value(&source, 2);
    let third_source = source.clone();
    let third = std::thread::spawn(move || assert!(third_source.compare_and_set(2, 3)));
    wait_for_value(&source, 3);
    resume_sender.send(()).unwrap();
    first.join().unwrap();
    second.join().unwrap();
    third.join().unwrap();
    source.unsubscribe(subscription);
    assert_eq!(*observed.lock().unwrap(), vec![0, 1, 3]);
}
