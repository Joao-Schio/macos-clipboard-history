use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};

use clipboard_history::{
    Writer,
    clipboard_manager::{ClipboardManager, ClipboardRequest},
    content_manager::ContentManager,
    history::{HistoryManager, ManagerRequest},
};
use eframe::egui;
use tokio::runtime::Runtime;
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};

struct TrayState {
    _tray: TrayIcon,
    show_id: MenuId,
    hide_id: MenuId,
    quit_id: MenuId,
    tray_id: tray_icon::TrayIconId,
}

fn make_icon() -> Icon {
    let rgba = vec![
        0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
    ];

    Icon::from_rgba(rgba, 2, 2).expect("valid icon")
}

fn create_tray() -> TrayState {
    let menu = Menu::new();

    let show = MenuItem::new("Show", true, None);
    let hide = MenuItem::new("Hide", true, None);
    let quit = MenuItem::new("Quit", true, None);

    let show_id = show.id().clone();
    let hide_id = hide.id().clone();
    let quit_id = quit.id().clone();

    menu.append(&show).unwrap();
    menu.append(&hide).unwrap();
    menu.append(&quit).unwrap();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Clipboard History")
        .with_icon(make_icon())
        .with_icon_as_template(true)
        .with_menu_on_left_click(false)
        .build()
        .expect("tray icon creation failed");
    let tray_id = tray.id().clone();

    TrayState {
        _tray: tray,
        show_id,
        hide_id,
        quit_id,
        tray_id,
    }
}

#[cfg(target_os = "macos")]
fn detach_from_terminal() {
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

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "macos")]
    detach_from_terminal();

    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime"),
    );

    let (history_tx, clipboard_tx) = setup_backend(rt.clone());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Clipboard History")
            .with_inner_size([500.0, 650.0])
            .with_titlebar_buttons_shown(false)
            .with_visible(false),
        ..Default::default()
    };

    eframe::run_native(
        "Clipboard History",
        options,
        Box::new(move |_cc| {
            let tray = create_tray();

            Ok(Box::new(App::new(
                rt.clone(),
                history_tx.clone(),
                clipboard_tx.clone(),
                tray,
                _cc.egui_ctx.clone(),
            )))
        }),
    )
}

fn setup_backend(rt: Arc<Runtime>) -> (Writer<ManagerRequest>, Writer<ClipboardRequest>) {
    let (history_tx, history_rx) = tokio::sync::mpsc::channel(30);
    let (clipboard_tx, clipboard_rx) = tokio::sync::mpsc::channel(30);

    let mut history = HistoryManager::new(history_rx);
    let mut clipboard = ClipboardManager::new(clipboard_rx);
    let mut content = ContentManager::new(clipboard_tx.clone(), history_tx.clone());

    rt.spawn(async move {
        if let Err(err) = history.start().await {
            eprintln!("history manager failed: {err}");
        }
    });

    rt.spawn(async move {
        if let Err(err) = clipboard.start().await {
            eprintln!("clipboard manager failed: {err}");
        }
    });

    rt.spawn(async move {
        if let Err(err) = content.start().await {
            eprintln!("content manager failed: {err}");
        }
    });

    (history_tx, clipboard_tx)
}

struct App {
    rt: Arc<Runtime>,
    history_tx: Writer<ManagerRequest>,
    clipboard_tx: Writer<ClipboardRequest>,
    tray: TrayState,
    items: Vec<String>,
    status: String,
    window_visible: Arc<AtomicBool>,
    was_visible: bool,
}

impl App {
    fn new(
        rt: Arc<Runtime>,
        history_tx: Writer<ManagerRequest>,
        clipboard_tx: Writer<ClipboardRequest>,
        tray: TrayState,
        ctx: egui::Context,
    ) -> Self {
        let window_visible = Arc::new(AtomicBool::new(false));
        Self::start_tray_listener(ctx, &tray, window_visible.clone());

        Self {
            rt,
            history_tx,
            clipboard_tx,
            tray,
            items: Vec::new(),
            status: "Watching clipboard…".to_string(),
            window_visible,
            was_visible: false,
        }
    }

    fn start_tray_listener(ctx: egui::Context, tray: &TrayState, window_visible: Arc<AtomicBool>) {
        let tray_id = tray.tray_id.clone();
        let show_id = tray.show_id.clone();
        let hide_id = tray.hide_id.clone();
        let quit_id = tray.quit_id.clone();

        std::thread::spawn(move || {
            loop {
                let mut handled_event = false;

                while let Ok(event) = TrayIconEvent::receiver().try_recv() {
                    if let TrayIconEvent::Click {
                        id,
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if id == tray_id {
                            let should_show = !window_visible.load(Ordering::Relaxed);
                            window_visible.store(should_show, Ordering::Relaxed);

                            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(should_show));
                            if should_show {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                            }
                            ctx.request_repaint();
                        }
                    }
                    handled_event = true;
                }

                while let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == show_id {
                        window_visible.store(true, Ordering::Relaxed);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.request_repaint();
                    } else if event.id == hide_id {
                        window_visible.store(false, Ordering::Relaxed);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    } else if event.id == quit_id {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                    handled_event = true;
                }

                if !handled_event {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        });
    }

    fn refresh_history(&mut self) {
        let (tx, rx) = tokio::sync::oneshot::channel::<VecDeque<String>>();
        let history_tx = self.history_tx.clone();

        self.rt.spawn(async move {
            let _ = history_tx
                .send(ManagerRequest::Retrieve {
                    response_channel: tx,
                })
                .await;
        });

        match self.rt.block_on(rx) {
            Ok(snapshot) => {
                self.items = snapshot.into_iter().collect();
                self.status = "Watching clipboard…".to_string();
            }
            Err(_) => {
                self.status = "Could not refresh history".to_string();
            }
        }
    }

    fn restore_item(&mut self, content: String) {
        let clipboard_tx = self.clipboard_tx.clone();

        self.rt.spawn(async move {
            let _ = clipboard_tx.send(ClipboardRequest::Set { content }).await;
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let minimized = ui.ctx().input(|i| i.viewport().minimized).unwrap_or(false);

        if minimized {
            self.window_visible.store(false, Ordering::Relaxed);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        }

        if !self.window_visible.load(Ordering::Relaxed) {
            self.was_visible = false;
            return;
        }

        if !self.was_visible {
            self.refresh_history();
            self.was_visible = true;
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Clipboard History");

                if ui.button("Hide").clicked() {
                    self.window_visible.store(false, Ordering::Relaxed);
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
            });

            ui.label(&self.status);
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for item in self.items.clone() {
                    let preview = if item.len() > 80 {
                        format!("{}...", &item[..80])
                    } else {
                        item.clone()
                    };

                    if ui.button(preview).clicked() {
                        self.restore_item(item);
                    }
                }
            });
        });
    }
}
