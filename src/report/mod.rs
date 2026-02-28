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
