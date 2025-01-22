use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

pub trait Tracker {
    fn set_msg(&self, msg: String);
    fn finish(&self, msg: Option<String>);
}
pub struct ProgressTracker {
    pub pb: ProgressBar,
}

impl ProgressTracker {
    pub fn new(bytes: u64) -> Result<Self> {
        let pb = ProgressBar::new(bytes);
        let default_style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?;

        pb.set_style(default_style);
        Ok(Self { pb })
    }
}

impl Tracker for ProgressTracker {
    fn set_msg(&self, msg: String) {
        self.pb.set_message(msg);
    }

    fn finish(&self, msg: Option<String>) {
        if let Some(msg) = msg {
            self.pb.finish_with_message(msg);
        }
        self.pb.finish();
    }
}
