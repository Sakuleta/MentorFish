// ─── Profile Commands ───

use super::*;
use std::sync::Arc;
use tauri::State;


#[tauri::command]
pub async fn cmd_get_user_profile(
    state: State<'_, Arc<AppState>>,
) -> Result<UserProfileResponse, String> {
    let profile = load_or_create_profile(state.database.as_deref()).await;
    Ok(UserProfileResponse { profile })
}

#[tauri::command]
pub async fn cmd_generate_curriculum(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::agents::curriculum::StudyPlan, String> {
    let db = state.database.as_deref();
    let profile = load_or_create_profile(db).await;

    let plan = crate::agents::executor::run_curriculum(state.inference.as_ref(), &profile)
        .await
        .map_err(|e| e.to_string())?;

    Ok(plan)
}
