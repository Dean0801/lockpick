pub mod interactive;
pub mod json;
pub mod markdown;
pub mod terminal;

use crate::AnalysisResult;
use crate::error::LockpickError;
use crate::i18n::I18n;

/// Report output trait
pub trait Reporter {
    fn report(&self, result: &AnalysisResult, i18n: &I18n) -> Result<(), LockpickError>;
}

/// No-op reporter for --output mode (suppresses stdout)
pub struct NoopReporter;

impl Reporter for NoopReporter {
    fn report(&self, _result: &AnalysisResult, _i18n: &I18n) -> Result<(), LockpickError> {
        Ok(())
    }
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
