use std::cell::RefCell;

use indicatif::ProgressBar;
use sinowealth_isp::Progress;

/// Renders [`Progress`] events from the ISP protocol onto the terminal: status
/// lines go to stderr and counted tasks drive an `indicatif` progress bar.
pub struct ProgressReporter {
    bar: RefCell<Option<ProgressBar>>,
}

impl ProgressReporter {
    pub fn new() -> Self {
        Self {
            bar: RefCell::new(None),
        }
    }

    pub fn report(&self, event: Progress) {
        match event {
            Progress::Status(message) => eprintln!("{}", message),
            Progress::TaskStart { label, total } => {
                eprintln!("{}", label);
                *self.bar.borrow_mut() = Some(ProgressBar::new(total as u64));
            }
            Progress::TaskAdvance => {
                if let Some(bar) = self.bar.borrow().as_ref() {
                    bar.inc(1);
                }
            }
            Progress::TaskFinish => {
                if let Some(bar) = self.bar.borrow_mut().take() {
                    bar.finish();
                }
            }
        }
    }
}
