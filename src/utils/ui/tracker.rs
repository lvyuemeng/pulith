use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;

pub trait Tracker {
    type Ctx: Clone;
    fn new(ctx: Self::Ctx) -> Self;
    fn finish(&self, msg: Option<String>);
}

const PB_STYLE: &str = "{spinner:.blue} [{elapsed_precise}] {wide_bar:.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta})";

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
    pub pb: ProgressBar,
}

#[derive(Debug, Clone)]
pub struct ProgressTrackerConfig {
    pub len: Option<u64>,
}

impl Tracker for ProgressTracker {
    type Ctx = ProgressTrackerConfig;

    fn new(ctx: Self::Ctx) -> Self {
        let pb = if let Some(len) = ctx.len {
            ProgressBar::new(len)
        } else {
            ProgressBar::no_length()
        };

        let pb_style = PB_TEMPLATE.as_ref().unwrap().clone();
        pb.set_style(pb_style);
        ProgressTracker { pb }
    }

    fn finish(&self, msg: Option<String>) {
        if let Some(msg) = msg {
            self.pb.finish_with_message(msg);
        }
        self.pb.finish();
    }
}
