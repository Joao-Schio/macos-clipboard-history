use tauri::{CustomMenuItem, SystemTray, SystemTrayMenu, SystemTrayEvent, Manager};


// Using the type aliases defined in your lib.rs[cite: 9]
use clipboard_history::{
    clipboard_manager::{ClipboardManager, ClipboardRequest},
    history::{HistoryManager, ManagerRequest},
    content_manager::ContentManager,
};

#[derive(Clone)]
struct AppState {
    history_tx: tokio::sync::mpsc::Sender<ManagerRequest>,
    clipboard_tx: tokio::sync::mpsc::Sender<ClipboardRequest>,
}
#[tauri::command]
async fn set_item(content: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Sends the request to the ClipboardManager actor
    state.clipboard_tx.send(ClipboardRequest::Set { content })
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
async fn get_history(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    
    // Request history from the HistoryManager actor[cite: 8, 10]
    state.history_tx.send(ManagerRequest::Retrieve { response_channel: tx })
        .await
        .map_err(|e| e.to_string())?;

    let history = rx.await.map_err(|e| e.to_string())?;
    Ok(history.into())
}

#[tauri::command]
async fn clean_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .history_tx
        .send(ManagerRequest::Clean)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(target_os = "macos")]
fn detach_from_terminal() {
    use std::process::{Command, Stdio};

    if std::env::var_os("CLIPBOARD_HISTORY_DAEMONIZED").is_some() {
        return;
    }

    if std::env::var_os("TERM").is_none() {
        return;
    }

    let Ok(executable) = std::env::current_exe() else {
        return;
    };

    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut command = Command::new(executable);
    command
        .args(args)
        .env("CLIPBOARD_HISTORY_DAEMONIZED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if command.spawn().is_ok() {
        std::process::exit(0);
    }
}
#[tokio::main]
async fn main() {
    #[cfg(target_os = "macos")]
    detach_from_terminal();
    let (clip_tx, clip_rx) = tokio::sync::mpsc::channel(100);
    let (hist_tx, hist_rx) = tokio::sync::mpsc::channel(100);


    let mut clipboard_mngr = ClipboardManager::new(clip_rx);

    let mut args = std::env::args().skip(1);

    let size = match args.next() {
        Some(val) => {
            val.trim()
            .parse()
            .expect("Error when converting")
        }
        None => {
            50usize
        }
    };

    let mut history_mngr = HistoryManager::new_with_size(hist_rx, size);
    let mut content_mngr = ContentManager::new(clip_tx.clone(), hist_tx.clone());


    tokio::spawn(async move {
        let _ = clipboard_mngr.start().await;
    });

    tokio::spawn(async move {
        let _ = history_mngr.start().await;
    });

    tokio::spawn(async move {
        let _ = content_mngr.start().await;
    });


    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let show = CustomMenuItem::new("show".to_string(), "Show History");
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);


    tauri::Builder::default()
            .manage(AppState {
                history_tx: hist_tx,
                clipboard_tx: clip_tx,
            })
            .system_tray(system_tray)
            .on_system_tray_event(|app, event| match event {
                SystemTrayEvent::MenuItemClick { id, .. } => {
                    match id.as_str() {
                        "quit" => std::process::exit(0),
                        "show" => {
                            let window = app.get_window("main").unwrap();
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                        _ => {}
                    }
                }
                _ => {}
            })
            .on_window_event(|event| {
                match event.event() {
                    tauri::WindowEvent::Focused(focused) => {
                        // If the window is no longer focused, hide it
                        if !focused {
                            event.window().hide().unwrap();
                        }
                    }
                    _ => {}
                }
            })
            .setup(|app| {
                #[cfg(target_os = "macos")]
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![get_history, set_item, clean_history])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
}
