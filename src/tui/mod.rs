pub mod app;
pub mod events;
pub mod ui;

use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::error::LockpickError;
use crate::i18n::I18n;
use crate::runner::RunConfig;
use app::{App, MenuItem, Screen};
use events::{AppEvent, poll_event};

pub fn run() -> Result<(), LockpickError> {
    enable_raw_mode().map_err(LockpickError::Io)?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(LockpickError::Io)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(LockpickError::Io)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode().map_err(LockpickError::Io)?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(LockpickError::Io)?;
    terminal.show_cursor().map_err(LockpickError::Io)?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), LockpickError> {
    loop {
        terminal
            .draw(|f| ui::render(f, app))
            .map_err(LockpickError::Io)?;

        match poll_event().map_err(LockpickError::Io)? {
            AppEvent::Quit => {
                app.should_quit = true;
            }
            AppEvent::Up => {
                if app.screen == Screen::Menu {
                    app.prev_menu();
                }
            }
            AppEvent::Down => {
                if app.screen == Screen::Menu {
                    app.next_menu();
                }
            }
            AppEvent::Select => {
                if app.screen == Screen::Menu {
                    handle_selection(app);
                } else if app.screen == Screen::Results {
                    app.screen = Screen::Menu;
                }
            }
            AppEvent::Back => {
                if app.screen == Screen::Results || app.screen == Screen::Settings {
                    app.screen = Screen::Menu;
                }
            }
            AppEvent::None => {}
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_selection(app: &mut App) {
    use app::Screen;

    match app.selected_item() {
        MenuItem::Exit => {
            app.should_quit = true;
        }
        MenuItem::FullScan => {
            app.screen = Screen::Scanning;
            app.scan_status = "正在分析依赖...".to_string();
            app.scan_progress = 0;

            if let Ok(result) = run_scan(app, true, true) {
                app.result = Some(result);
                app.screen = Screen::Results;
            } else {
                app.screen = Screen::Menu;
            }
        }
        MenuItem::Unused => {
            app.screen = Screen::Scanning;
            app.scan_status = "检测未使用依赖...".to_string();
            app.scan_progress = 0;

            if let Ok(result) = run_scan(app, true, false) {
                app.result = Some(result);
                app.screen = Screen::Results;
            } else {
                app.screen = Screen::Menu;
            }
        }
        MenuItem::Audit => {
            app.screen = Screen::Scanning;
            app.scan_status = "扫描漏洞...".to_string();
            app.scan_progress = 0;

            if let Ok(result) = run_scan(app, false, true) {
                app.result = Some(result);
                app.screen = Screen::Results;
            } else {
                app.screen = Screen::Menu;
            }
        }
        MenuItem::Fix => {
            app.screen = Screen::Scanning;
            app.scan_status = "自动修复中...".to_string();
            app.scan_progress = 0;

            if let Ok(result) = run_fix(app) {
                app.result = Some(result);
                app.screen = Screen::Results;
            } else {
                app.screen = Screen::Menu;
            }
        }
        MenuItem::Tree => {
            app.screen = Screen::Scanning;
            app.scan_status = "生成依赖树...".to_string();
            app.scan_progress = 50;

            // Tree 功能暂时跳过，因为它需要特殊的可视化界面
            app.screen = Screen::Menu;
        }
        MenuItem::Outdated => {
            app.screen = Screen::Scanning;
            app.scan_status = "检查过时依赖...".to_string();
            app.scan_progress = 0;

            if let Ok(result) = run_outdated(app) {
                app.result = Some(result);
                app.screen = Screen::Results;
            } else {
                app.screen = Screen::Menu;
            }
        }
        MenuItem::SupplyChain => {
            app.screen = Screen::Scanning;
            app.scan_status = "供应链安全分析...".to_string();
            app.scan_progress = 0;

            if let Ok(result) = run_supply_chain(app) {
                app.result = Some(result);
                app.screen = Screen::Results;
            } else {
                app.screen = Screen::Menu;
            }
        }
        MenuItem::Settings => {
            app.screen = Screen::Settings;
        }
    }
}

fn run_scan(
    app: &mut App,
    run_unused: bool,
    run_audit: bool,
) -> Result<crate::AnalysisResult, LockpickError> {
    let project_path = PathBuf::from(".");
    let i18n = I18n::detect(None);

    app.scan_progress = 10;

    let graph = crate::lockfile::detect_and_parse(&project_path)?;
    app.scan_progress = 30;

    let ignore = HashSet::new();
    let extra_configs = vec![];

    let cfg = RunConfig {
        skip_dev: false,
        ignore: &ignore,
        extra_configs: &extra_configs,
        license_policy: None,
        run_unused,
        run_audit,
        run_fix: false,
        run_supply_chain: false,
        verbose: false,
        dry_run: false,
        yes: false,
        no_cache: false,
        cache_ttl: None,
    };

    app.scan_progress = 50;

    let reporter = crate::report::NoopReporter;
    let rt = tokio::runtime::Runtime::new().map_err(LockpickError::Io)?;

    app.scan_progress = 70;

    let (_has_issues, result) = rt.block_on(async {
        crate::runner::run_single(&project_path, &graph, &cfg, &i18n, &reporter).await
    });

    app.scan_progress = 100;

    result.ok_or_else(|| LockpickError::Report("扫描失败".to_string()))
}

fn run_fix(app: &mut App) -> Result<crate::AnalysisResult, LockpickError> {
    let project_path = PathBuf::from(".");
    let i18n = I18n::detect(None);

    app.scan_progress = 20;

    let graph = crate::lockfile::detect_and_parse(&project_path)?;
    app.scan_progress = 40;

    let ignore = HashSet::new();
    let extra_configs = vec![];

    let cfg = RunConfig {
        skip_dev: false,
        ignore: &ignore,
        extra_configs: &extra_configs,
        license_policy: None,
        run_unused: true,
        run_audit: false,
        run_fix: true,
        run_supply_chain: false,
        verbose: false,
        dry_run: false,
        yes: true,
        no_cache: false,
        cache_ttl: None,
    };

    app.scan_progress = 60;

    let reporter = crate::report::NoopReporter;
    let rt = tokio::runtime::Runtime::new().map_err(LockpickError::Io)?;

    app.scan_progress = 80;

    let (_has_issues, result) = rt.block_on(async {
        crate::runner::run_single(&project_path, &graph, &cfg, &i18n, &reporter).await
    });

    app.scan_progress = 100;

    result.ok_or_else(|| LockpickError::Report("修复失败".to_string()))
}

fn run_outdated(app: &mut App) -> Result<crate::AnalysisResult, LockpickError> {
    let project_path = PathBuf::from(".");

    app.scan_progress = 20;

    let graph = crate::lockfile::detect_and_parse(&project_path)?;
    app.scan_progress = 40;

    let rt = tokio::runtime::Runtime::new().map_err(LockpickError::Io)?;

    app.scan_progress = 60;

    let report = rt.block_on(async {
        crate::outdated::check_outdated(&graph, None, false, None, false, None).await
    })?;

    app.scan_progress = 100;

    Ok(crate::AnalysisResult {
        unused: None,
        vulns: None,
        duplicates: None,
        size: None,
        license: None,
        outdated: Some(report),
        supply_chain: None,
    })
}

fn run_supply_chain(app: &mut App) -> Result<crate::AnalysisResult, LockpickError> {
    let project_path = PathBuf::from(".");

    app.scan_progress = 30;

    let graph = crate::lockfile::detect_and_parse(&project_path)?;
    app.scan_progress = 60;

    let report = crate::supply_chain::analyze(&graph);
    app.scan_progress = 100;

    Ok(crate::AnalysisResult {
        unused: None,
        vulns: None,
        duplicates: None,
        size: None,
        license: None,
        outdated: None,
        supply_chain: Some(report),
    })
}
