use tauri::State;

use getagrip_query_engine::HistoryEntry;

use crate::state::AppState;

#[tauri::command]
pub async fn list_history(state: State<'_, AppState>) -> Result<Vec<HistoryEntry>, String> {
    Ok(state.history.all())
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history.clear();
    Ok(())
}
