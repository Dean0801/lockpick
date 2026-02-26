/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

pub struct I18n {
    pub lang: Lang,
}

impl I18n {
    /// Detect language with priority:
    /// 1. --lang CLI arg
    /// 2. LOCKPICK_LANG env
    /// 3. LANG/LC_ALL system env
    /// 4. Default En
    pub fn detect(cli_lang: Option<&str>) -> Self {
        let lang = cli_lang
            .and_then(parse_lang)
            .or_else(|| {
                std::env::var("LOCKPICK_LANG")
                    .ok()
                    .as_deref()
                    .and_then(parse_lang)
            })
            .or_else(detect_system_lang)
            .unwrap_or(Lang::En);

        I18n { lang }
    }

    pub fn t<'a>(&self, key: &'a str) -> &'a str {
        match self.lang {
            Lang::Zh => match key {
                "analyzing" => "正在分析项目...",
                "unused_deps" => "未使用的依赖",
                "vulns" => "安全漏洞",
                "found" => "发现",
                "package" => "包名",
                "version" => "版本",
                "type" => "类型",
                "prod" => "生产",
                "dev" => "开发",
                "severity" => "严重程度",
                "fix_version" => "修复版本",
                "critical" => "严重",
                "high" => "高危",
                "medium" => "中危",
                "low" => "低危",
                "none" => "无",
                "no_unused" => "未发现未使用的依赖",
                "no_vulns" => "未发现安全漏洞",
                "scan_complete" => "扫描完成",
                "network_error" => "网络请求失败，跳过漏洞扫描",
                "size_analysis" => "依赖体积分析",
                "total_size" => "总计",
                "size" => "体积",
                _ => key,
            },
            Lang::En => match key {
                "analyzing" => "Analyzing project...",
                "unused_deps" => "Unused Dependencies",
                "vulns" => "Vulnerabilities",
                "found" => "found",
                "package" => "Package",
                "version" => "Version",
                "type" => "Type",
                "prod" => "prod",
                "dev" => "dev",
                "severity" => "Severity",
                "fix_version" => "Fix Available",
                "critical" => "CRITICAL",
                "high" => "HIGH",
                "medium" => "MEDIUM",
                "low" => "LOW",
                "none" => "N/A",
                "no_unused" => "No unused dependencies found",
                "no_vulns" => "No vulnerabilities found",
                "scan_complete" => "Scan complete",
                "network_error" => "Network error, skipping vulnerability scan",
                "size_analysis" => "Dependency Size Analysis",
                "total_size" => "Total",
                "size" => "Size",
                _ => key,
            },
        }
    }
}

fn parse_lang(s: &str) -> Option<Lang> {
    match s.to_lowercase().as_str() {
        "zh" | "zh-cn" | "chinese" => Some(Lang::Zh),
        "en" | "english" => Some(Lang::En),
        _ => None,
    }
}

fn detect_system_lang() -> Option<Lang> {
    let lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    if lang.starts_with("zh") {
        Some(Lang::Zh)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cli_lang() {
        let i18n = I18n::detect(Some("zh"));
        assert_eq!(i18n.lang, Lang::Zh);
    }

    #[test]
    fn test_detect_default_en() {
        let i18n = I18n::detect(None);
        // Without env vars set, should default to En
        assert_eq!(i18n.lang, Lang::En);
    }

    #[test]
    fn test_translate_zh() {
        let i18n = I18n { lang: Lang::Zh };
        assert_eq!(i18n.t("package"), "包名");
        assert_eq!(i18n.t("severity"), "严重程度");
    }

    #[test]
    fn test_translate_en() {
        let i18n = I18n { lang: Lang::En };
        assert_eq!(i18n.t("package"), "Package");
        assert_eq!(i18n.t("severity"), "Severity");
    }
}
