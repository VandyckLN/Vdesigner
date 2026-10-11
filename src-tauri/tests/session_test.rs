use std::thread;
use vdesigner::session::TraceGeneration;

#[test]
fn a_trace_is_current_until_another_begins() {
    let g = TraceGeneration::default();
    let first = g.begin();
    assert!(!first());
    let second = g.begin();
    assert!(first());
    assert!(!second());
}

#[test]
fn the_previous_trace_is_superseded_when_another_begins() {
    let g = TraceGeneration::default();
    let first = g.begin();
    let _second = g.begin();
    assert!(thread::spawn(first).join().unwrap());
}
