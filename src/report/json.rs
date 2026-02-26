use super::Reporter;
use crate::AnalysisResult;
use crate::i18n::I18n;

pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn report(&self, result: &AnalysisResult, _i18n: &I18n) -> Result<(), String> {
        let json = serde_json::to_string_pretty(result)
            .map_err(|e| format!("JSON serialization error: {e}"))?;
        println!("{json}");
        Ok(())
    }
}
