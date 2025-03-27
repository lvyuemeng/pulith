use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;

pub trait TrackerBuilder<T: Tracker<U>, U> {
    fn build(self) -> T;
}

pub trait Tracker<Inc> {
    fn step(&self, step: Inc) -> &Self;
    fn finish(self);
}

// TODO!: detect console width and ident to adjust.
const PB_STYLE: &str = "{spinner:.blue} {prefix:>12.cyan.bold} [{elapsed_precise}] {wide_bar:.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {wide_msg}";

const TICK: &str = "⠁⠂⠄⡀⢀⠠⠐⠈ ";

const PB_CHARS: &str = "█▓▒░  ";

static PB_TEMPLATE: Lazy<Option<ProgressStyle>> = Lazy::new(|| {
    let pb_style = match ProgressStyle::with_template(PB_STYLE) {
        Ok(pb_style) => pb_style.tick_chars(TICK).progress_chars(PB_CHARS),
        Err(_) => return None,
    };

    Some(pb_style)
});

pub struct ProgressTracker {
    pb: ProgressBar,
    finish: Option<String>,
}

impl Tracker<u64> for ProgressTracker {

    fn step(&self, len: u64) -> &Self {
        self.pb.inc(len);
        self
    }
    fn finish(self) {
        if let Some(msg) = self.finish {
            self.pb.finish_with_message(msg);
        }
        self.pb.finish();
    }
}

#[derive(Debug, Clone,Default)]
pub struct ProgressTrackerBuilder {
    len: Option<u64>,
    prefix: Option<String>,
    finish: Option<String>,
}

impl ProgressTrackerBuilder {
    pub fn with_len(mut self, len: u64) -> Self {
        self.len = Some(len);
        self
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = Some(prefix.to_string());
        self
    }

    pub fn with_finish(mut self, finish: &str) -> Self {
        self.finish = Some(finish.to_string());
        self
    }
}

impl TrackerBuilder<ProgressTracker,u64> for ProgressTrackerBuilder {
    fn build(self) -> ProgressTracker {
        let pb = if let Some(len) = self.len {
            ProgressBar::new(len)
        } else {
            ProgressBar::new_spinner()
        };
        let pb = if let Some(style) = PB_TEMPLATE.as_ref() {
            pb.with_style(style.clone())
        } else {
            pb
        };

        if let Some(prefix) = self.prefix {
            pb.set_prefix(prefix);
        }
        ProgressTracker {
            pb,
            finish: self.finish,
        }
    }
}
