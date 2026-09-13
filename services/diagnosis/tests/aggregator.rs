use diagnosis::aggregator::{build_snapshot, Sample, Trend, Window};

#[test]
fn stats_compute_mean_min_max_latest() {
    let mut w = Window::new("src1", "heart_rate", 60.0);
    for (t, v) in [(0.0, 70.0), (1.0, 80.0), (2.0, 90.0)] {
        w.add(Sample::valid(t, v));
    }
    let s = w.stats();
    assert_eq!(s.count, 3);
    assert_eq!(s.valid_count, 3);
    assert_eq!(s.mean, Some(80.0));
    assert_eq!(s.min, Some(70.0));
    assert_eq!(s.max, Some(90.0));
    assert_eq!(s.latest, Some(90.0));
    assert!(s.valid);
}

#[test]
fn invalid_samples_do_not_affect_stats_but_are_counted() {
    let mut w = Window::new("src1", "spo2", 60.0);
    w.add(Sample::valid(0.0, 98.0));
    w.add(Sample::invalid(1.0));
    let s = w.stats();
    assert_eq!(s.count, 2);
    assert_eq!(s.valid_count, 1);
    assert_eq!(s.mean, Some(98.0));
}

#[test]
fn empty_window_is_invalid() {
    let w = Window::new("src1", "spo2", 60.0);
    let s = w.stats();
    assert!(!s.valid);
    assert_eq!(s.mean, None);
    assert_eq!(s.trend, Trend::Unknown);
}

#[test]
fn prune_drops_out_of_window_samples() {
    let mut w = Window::new("src1", "spo2", 10.0);
    w.add(Sample::valid(0.0, 90.0));
    w.add(Sample::valid(5.0, 95.0));
    w.add(Sample::valid(12.0, 97.0));
    w.prune(12.0);
    assert_eq!(w.samples().len(), 2);
}

#[test]
fn trend_detects_direction() {
    let mut up = Window::new("s", "x", 100.0);
    for (t, v) in [(0.0, 10.0), (1.0, 11.0), (2.0, 20.0), (3.0, 21.0)] {
        up.add(Sample::valid(t, v));
    }
    assert_eq!(up.stats().trend, Trend::Increasing);

    let mut down = Window::new("s", "x", 100.0);
    for (t, v) in [(0.0, 20.0), (1.0, 21.0), (2.0, 10.0), (3.0, 11.0)] {
        down.add(Sample::valid(t, v));
    }
    assert_eq!(down.stats().trend, Trend::Decreasing);

    let mut flat = Window::new("s", "x", 100.0);
    for (t, v) in [(0.0, 10.0), (1.0, 10.1), (2.0, 10.0), (3.0, 10.1)] {
        flat.add(Sample::valid(t, v));
    }
    assert_eq!(flat.stats().trend, Trend::Stable);

    let mut short = Window::new("s", "x", 100.0);
    short.add(Sample::valid(0.0, 10.0));
    assert_eq!(short.stats().trend, Trend::Unknown);
}

#[test]
fn build_snapshot_includes_all_sources() {
    let mut a = Window::new("a", "spo2", 60.0);
    a.add(Sample::valid(0.0, 97.0));
    let mut b = Window::new("b", "heart_rate", 60.0);
    b.add(Sample::valid(0.0, 72.0));
    let snap = build_snapshot([&a, &b], "periodic");
    assert_eq!(snap.trigger_type, "periodic");
    assert_eq!(snap.sources.len(), 2);
    assert_eq!(snap.sources[0].data_src, "a");
    assert_eq!(snap.sources[1].data_type, "heart_rate");
}
