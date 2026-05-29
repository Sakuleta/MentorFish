// ─── Config & Utility Commands ───

use super::*;
use std::sync::Arc;
use tauri::State;



#[tauri::command]
pub async fn cmd_get_config() -> Result<crate::config::AppConfig, String> {
    crate::config::AppConfig::load().map_err(|e| format!("Failed to load config: {}", e))
}

#[tauri::command]
pub async fn cmd_save_config(config: crate::config::AppConfig) -> Result<(), String> {
    config
        .save()
        .map_err(|e| format!("Failed to save config: {}", e))
}

#[tauri::command]
pub async fn cmd_health_check(state: State<'_, Arc<AppState>>) -> Result<HealthCheckResponse, String> {
    let engine_ok = state.engine.health_check().await.unwrap_or(false);

    // Check inference by verifying Ollama is reachable
    let inference_ok = state.inference.health_check().await.unwrap_or(false);

    let database_ok = state.database.is_some();

    Ok(HealthCheckResponse {
        engine_ok,
        inference_ok,
        database_ok,
    })
}

/// Accept an error report from the frontend ErrorBoundary and write it to the log.
#[tauri::command]
pub async fn cmd_report_error(report: ErrorReport) -> Result<(), String> {
    log::error!(
        "Frontend error [{}]: {} (stack: {})",
        report.component.as_deref().unwrap_or("unknown"),
        report.message,
        report.stack.as_deref().unwrap_or("none")
    );
    Ok(())
}
