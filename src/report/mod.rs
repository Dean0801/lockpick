pub mod json;
pub mod terminal;

use crate::AnalysisResult;
use crate::error::LockpickError;
use crate::i18n::I18n;

/// Report output trait
pub trait Reporter {
    fn report(&self, result: &AnalysisResult, i18n: &I18n) -> Result<(), LockpickError>;
}
