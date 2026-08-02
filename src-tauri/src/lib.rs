mod commands;
mod error;
mod models;

use commands::vault::VaultState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Jednorazowa migracja istniejących danych do układu wielo-vaultowego
    // (przeniesienie do podkatalogu `default/`). Bezpieczna, idempotentna.
    commands::vaults::migrate_to_multi_vault();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(VaultState::default())
        .invoke_handler(tauri::generate_handler![
            commands::vault::create_vault,
            commands::vault::unlock_vault,
            commands::vault::lock_vault,
            commands::vault::is_vault_unlocked,
            commands::vault::vault_exists,
            commands::vault::get_vault_params,
            commands::vault::set_duress_password,
            commands::vault::duress_configured,
            commands::vaults::list_vaults,
            commands::vaults::create_new_vault,
            commands::vaults::switch_vault,
            commands::vaults::delete_vault,
            commands::vaults::rename_vault,
            commands::security::capture_protection_supported,
            commands::security::toggle_capture_protection,
            commands::attachments::save_attachment,
            commands::attachments::load_attachment,
            commands::attachments::load_attachment_data_url,
            commands::attachments::delete_attachment,
            commands::attachments::list_attachments_for_note,
            commands::attachments::cleanup_orphaned_attachments,
            commands::attachments::open_attachment,
            commands::attachments::cleanup_temp_previews,
            commands::export::export_vault,
            commands::export::import_vault,
            commands::export::export_note_to_docx,
            commands::export::export_note_html,
            commands::backup::perform_backup,
            commands::backup::get_backup_list,
            commands::notes::save_note,
            commands::notes::load_note,
            commands::notes::list_notes,
            commands::notes::search_notes,
            commands::notes::find_backlinks,
            commands::notes::get_graph_data,
            commands::notes::delete_note,
            commands::search::build_search_index,
            commands::search::update_search_index,
            commands::search::remove_from_search_index,
            commands::folders::save_folders,
            commands::folders::load_folders,
            commands::kanban::load_kanban,
            commands::kanban::save_kanban,
            commands::kanban::add_note_to_kanban,
            commands::kanban::move_kanban_card,
            commands::kanban::remove_note_from_kanban,
            commands::calendar::get_calendar_notes,
            commands::time_tracking::load_time_entries,
            commands::time_tracking::start_timer,
            commands::time_tracking::stop_timer,
            commands::time_tracking::delete_time_entry,
            commands::time_tracking::get_time_summary,
            commands::whiteboard::list_whiteboards,
            commands::whiteboard::load_whiteboard,
            commands::whiteboard::save_whiteboard,
            commands::whiteboard::delete_whiteboard,
            commands::whiteboard::open_whiteboard_window,
            commands::whiteboard::save_whiteboard_from_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
