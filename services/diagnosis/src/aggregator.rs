//! Windowed aggregation of physiological samples (RFC-009 §6).

use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub t: f64,
    pub value: f64,
    pub valid: bool,
}

impl Sample {
    pub fn valid(t: f64, value: f64) -> Self {
        Self {
            t,
            value,
            valid: true,
        }
    }

    pub fn invalid(t: f64) -> Self {
        Self {
            t,
            value: 0.0,
            valid: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Unknown,
    Stable,
    Increasing,
    Decreasing,
}

impl Trend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Stable => "stable",
            Self::Increasing => "increasing",
            Self::Decreasing => "decreasing",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSnapshot {
    pub data_src: String,
    pub data_type: String,
    pub mean: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub latest: Option<f64>,
    pub trend: String,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub trigger_type: String,
    pub sources: Vec<SourceSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowStats {
    pub count: usize,
    pub valid_count: usize,
    pub mean: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub latest: Option<f64>,
    pub trend: Trend,
    pub valid: bool,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub data_src: String,
    pub data_type: String,
    pub window_seconds: f64,
    samples: Vec<Sample>,
}

impl Window {
    pub fn new(
        data_src: impl Into<String>,
        data_type: impl Into<String>,
        window_seconds: f64,
    ) -> Self {
        Self {
            data_src: data_src.into(),
            data_type: data_type.into(),
            window_seconds,
            samples: Vec::new(),
        }
    }

    pub fn add(&mut self, sample: Sample) {
        self.samples.push(sample);
    }

    /// Drop samples that fell out of the rolling window.
    pub fn prune(&mut self, now: f64) {
        let cutoff = now - self.window_seconds;
        self.samples.retain(|s| s.t >= cutoff);
    }

    pub fn samples(&self) -> &[Sample] {
        &self.samples
    }

    pub fn stats(&self) -> WindowStats {
        let valid: Vec<&Sample> = self.samples.iter().filter(|s| s.valid).collect();
        if valid.is_empty() {
            return WindowStats {
                count: self.samples.len(),
                valid_count: 0,
                mean: None,
                min: None,
                max: None,
                latest: None,
                trend: Trend::Unknown,
                valid: false,
            };
        }
        let values: Vec<f64> = valid.iter().map(|s| s.value).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        WindowStats {
            count: self.samples.len(),
            valid_count: valid.len(),
            mean: Some(round2(mean)),
            min: Some(round2(values.iter().cloned().fold(f64::INFINITY, f64::min))),
            max: Some(round2(
                values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            )),
            latest: Some(round2(*values.last().unwrap())),
            trend: trend(&valid),
            valid: true,
        }
    }
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn trend(valid: &[&Sample]) -> Trend {
    let n = valid.len();
    if n < 4 {
        return Trend::Unknown;
    }
    let half = n / 2;
    let older = valid[..half].iter().map(|s| s.value).sum::<f64>() / half as f64;
    let recent = valid[half..].iter().map(|s| s.value).sum::<f64>() / (n - half) as f64;
    let delta = recent - older;
    if delta.abs() < 0.5 {
        Trend::Stable
    } else if delta > 0.0 {
        Trend::Increasing
    } else {
        Trend::Decreasing
    }
}

/// Assemble a multi-source structured snapshot for RAG + LLM consumption.
pub fn build_snapshot<'a>(
    windows: impl IntoIterator<Item = &'a Window>,
    trigger_type: &str,
) -> Snapshot {
    let sources = windows
        .into_iter()
        .map(|w| {
            let st = w.stats();
            SourceSnapshot {
                data_src: w.data_src.clone(),
                data_type: w.data_type.clone(),
                mean: st.mean,
                min: st.min,
                max: st.max,
                latest: st.latest,
                trend: st.trend.as_str().to_string(),
                valid: st.valid,
            }
        })
        .collect();
    Snapshot {
        trigger_type: trigger_type.to_string(),
        sources,
    }
}
