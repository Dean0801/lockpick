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
                "duplicate_deps" => "重复依赖",
                "versions" => "版本列表",
                "no_duplicates" => "未发现重复依赖",
                "scan_config_complete" => "配置文件和脚本扫描完成",
                "monorepo_detected" => "检测到 monorepo，共",
                "workspace_packages" => "个工作区包",
                "workspace_package" => "工作区包",
                "skip_no_deps" => "跳过（无依赖声明）",
                "license_report" => "许可证报告",
                "license" => "许可证",
                "license_violations" => "许可证违规",
                "no_license_violations" => "未发现许可证违规",
                "violation_denied" => "在黑名单中",
                "violation_not_allowed" => "不在白名单",
                "violation_unknown" => "无法识别",
                "reason" => "原因",
                "fix_dry_run" => "试运行模式，以下依赖将被移除：",
                "fix_confirm" => "确认移除以上依赖？",
                "fix_confirm_prompt" => "确认删除？(y/N) ",
                "fix_will_remove" => "即将删除以下未使用依赖",
                "fix_removing" => "正在移除",
                "fix_done" => "移除完成",
                "fix_failed" => "移除失败",
                "fix_nothing" => "没有需要移除的依赖",
                "diff_summary" => "差异摘要",
                "diff_new" => "新增",
                "diff_resolved" => "已解决",
                "outdated_deps" => "过时依赖",
                "no_outdated" => "所有依赖均为最新版本",
                "supply_chain_analysis" => "供应链安全分析",
                "supply_chain_risks" => "供应链风险",
                "no_supply_chain_risks" => "未发现供应链风险",
                "duplicates" => "重复依赖",
                "current" => "当前版本",
                "latest" => "最新版本",
                "level" => "级别",
                "priority" => "优先级",
                "summary" => "摘要",
                "risk" => "风险",
                "typosquat" => "仿冒包",
                "scope_confusion" => "作用域混淆",
                "version_anomaly" => "版本异常",
                "interactive_prompt" => "选择要查看的部分（输入数字）：",
                "interactive_select" => "输入选项（0 退出）：",
                "interactive_exit" => "退出",
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
                "duplicate_deps" => "Duplicate Dependencies",
                "versions" => "Versions",
                "no_duplicates" => "No duplicate dependencies found",
                "scan_config_complete" => "Config and scripts scan complete",
                "monorepo_detected" => "Monorepo detected,",
                "workspace_packages" => "workspace packages",
                "workspace_package" => "Workspace package",
                "skip_no_deps" => "Skipped (no dependencies declared)",
                "license_report" => "License Report",
                "license" => "License",
                "license_violations" => "License Violations",
                "no_license_violations" => "No license violations",
                "violation_denied" => "Denied",
                "violation_not_allowed" => "Not in allowlist",
                "violation_unknown" => "Unknown license",
                "reason" => "Reason",
                "fix_dry_run" => "Dry run mode, the following deps would be removed:",
                "fix_confirm" => "Confirm removal of the above dependencies?",
                "fix_confirm_prompt" => "Confirm removal? (y/N) ",
                "fix_will_remove" => "The following unused dependencies will be removed",
                "fix_removing" => "Removing",
                "fix_done" => "Removal complete",
                "fix_failed" => "Removal failed",
                "fix_nothing" => "No dependencies to remove",
                "diff_summary" => "Diff Summary",
                "diff_new" => "new",
                "diff_resolved" => "resolved",
                "outdated_deps" => "Outdated Dependencies",
                "no_outdated" => "All dependencies are up to date",
                "supply_chain_analysis" => "Supply Chain Analysis",
                "supply_chain_risks" => "Supply Chain Risks",
                "no_supply_chain_risks" => "No supply chain risks found",
                "duplicates" => "Duplicates",
                "current" => "Current",
                "latest" => "Latest",
                "level" => "Level",
                "priority" => "Priority",
                "summary" => "Summary",
                "risk" => "Risk",
                "typosquat" => "Typosquat",
                "scope_confusion" => "Scope confusion",
                "version_anomaly" => "Version anomaly",
                "interactive_prompt" => "Select sections to view (enter number):",
                "interactive_select" => "Enter choice (0 to exit):",
                "interactive_exit" => "Exit",
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
