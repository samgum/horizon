//! A game started without its prepared files (standalone `.app` or executable
//! launched without pre-extracted resources): shows an interactive setup screen
//! supporting 1-click import from `~/Downloads`, native `.zip` file selection,
//! and drag-and-drop of `horizon-26.3-resources.zip`, then immediately launches
//! into the game once extraction finishes.
use super::*;
use horizon_client::no_resources::{Action, NoResources};
use std::sync::mpsc::{self, Receiver};
use winit::event::MouseButton;

enum ImportEvent {
    Cancelled,
    Progress { done: usize, total: usize, pct: u8 },
    Finished(std::result::Result<PathBuf, String>),
}

struct Missing {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gui: UiResources,
    screen: NoResources,
    zh: bool,
    target_dir: PathBuf,
    detected_zip: Option<PathBuf>,
    import_rx: Option<Receiver<ImportEvent>>,
    imported_dir: Option<PathBuf>,
    /// The pointer in physical pixels.
    pointer: (f64, f64),
    api: horizon_client::graphics::GraphicsApi,
    settings: Settings,
    error: Option<Box<dyn std::error::Error>>,
}

/// Shows the interactive resource-import screen until the player imports a `.zip`
/// archive (returning `Ok(Some(extracted_dir))`) or closes it (`Ok(None)`).
pub(super) fn show(looked_in: &std::path::Path, settings: &Settings) -> Result<Option<PathBuf>> {
    horizon_client::lang::set(horizon_client::lang::Language::load(
        &[],
        &settings.language,
    ));
    let zh = horizon_client::resource_zip::prefer_chinese(&settings.language);
    let detected_zip = horizon_client::resource_zip::find_downloads_resource_zip();
    let target_dir = if looked_in.is_absolute() {
        looked_in.to_path_buf()
    } else {
        horizon_client::resource_zip::default_extracted_resources_dir()
    };
    let mut app = Missing {
        window: None,
        renderer: None,
        gui: UiResources {
            atlas: HudAtlas::build(&horizon_assets::resource_source::ResourceSource::directory(
                looked_in.to_path_buf(),
            ))?,
            panorama: Arc::new(horizon_client::gfx::panorama::Faces::colour([0; 4])),
            splashes: Arc::from(""),
        },
        screen: NoResources::new_interactive(&target_dir, zh, detected_zip.as_deref()),
        zh,
        target_dir,
        detected_zip,
        import_rx: None,
        imported_dir: None,
        pointer: (0.0, 0.0),
        api: settings.horizon.api,
        settings: settings.clone(),
        error: None,
    };

    EventLoop::new()?.run_app(&mut app)?;
    match app.error {
        Some(err) => Err(err),
        None => Ok(app.imported_dir),
    }
}

impl Missing {
    fn size(&self) -> [u32; 2] {
        self.renderer
            .as_ref()
            .map_or([1280, 720], |r| [r.config.width, r.config.height])
    }

    fn with_ui<R>(
        &self,
        batch: &mut HudBatch,
        f: impl FnOnce(&mut horizon_client::ui_theme::Ui) -> R,
    ) -> R {
        let cursor = (
            self.pointer.0 as f32 / batch.scale,
            self.pointer.1 as f32 / batch.scale,
        );
        let mut ui = horizon_client::ui_theme::Ui::new(&self.gui.atlas, batch, cursor);
        let result = f(&mut ui);
        ui.finish();
        result
    }

    fn draw(&mut self) {
        let mut batch = HudBatch::new(self.size(), self.settings.gui_scale_inputs());
        self.with_ui(&mut batch, |ui| self.screen.draw(ui));
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        match renderer.render(&loading_frame(&batch)) {
            Ok(true) => return,
            Ok(false) => {}
            Err(e) => log::warn!("Horizon: frame failed: {e}"),
        }
        horizon_client::ui_theme::reset_atlas();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn start_import(&mut self, source: Option<PathBuf>) {
        if self.import_rx.is_some() {
            return;
        }
        let zh = self.zh;
        let target_dir = self.target_dir.clone();
        self.screen.set_error(None);
        let initial_msg = if source.is_some() {
            if zh {
                "正在准备解压游戏资源…".to_owned()
            } else {
                "Preparing to extract resources…".to_owned()
            }
        } else if zh {
            "请在弹出的文件选择框中选择 .zip 资源压缩包…".to_owned()
        } else {
            "Select a .zip resource archive in the file dialog…".to_owned()
        };
        self.screen.set_progress(Some((1, initial_msg)));
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        let (tx, rx) = mpsc::channel();
        self.import_rx = Some(rx);
        let _ = std::thread::Builder::new()
            .name("horizon-res-import".into())
            .spawn(move || {
                let picked = match source {
                    Some(p) => p,
                    None => match horizon_client::resource_zip::pick_resource_zip_dialog(zh) {
                        Some(p) => p,
                        None => {
                            let _ = tx.send(ImportEvent::Cancelled);
                            return;
                        }
                    },
                };
                let res = horizon_client::resource_zip::import_resources(
                    &picked,
                    &target_dir,
                    true,
                    &|done, total, pct| {
                        let _ = tx.send(ImportEvent::Progress { done, total, pct });
                    },
                );
                let _ = tx.send(ImportEvent::Finished(res));
            });
    }

    fn poll_import(&mut self, event_loop: &ActiveEventLoop) {
        let Some(rx) = &self.import_rx else {
            return;
        };
        let mut changed = false;
        let mut clear_rx = false;
        while let Ok(ev) = rx.try_recv() {
            changed = true;
            match ev {
                ImportEvent::Cancelled => {
                    self.screen.set_progress(None);
                    clear_rx = true;
                }
                ImportEvent::Progress { done, total, pct } => {
                    let status = if self.zh {
                        format!("正在解压游戏资源: {pct}% ({done} / {total})…")
                    } else {
                        format!("Extracting game resources: {pct}% ({done} / {total})…")
                    };
                    self.screen.set_progress(Some((pct.max(2), status)));
                }
                ImportEvent::Finished(Ok(dir)) => {
                    let done_msg = if self.zh {
                        "解压完成！正在启动 Horizon…".to_owned()
                    } else {
                        "Extraction complete! Launching Horizon…".to_owned()
                    };
                    self.screen.set_progress(Some((100, done_msg)));
                    self.imported_dir = Some(dir);
                    clear_rx = true;
                    event_loop.exit();
                    break;
                }
                ImportEvent::Finished(Err(err)) => {
                    self.screen.set_progress(None);
                    self.screen.set_error(Some(err));
                    clear_rx = true;
                }
            }
        }
        if clear_rx {
            self.import_rx = None;
        }
        if changed && let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn click(&mut self, event_loop: &ActiveEventLoop) {
        let mut batch = HudBatch::new(self.size(), self.settings.gui_scale_inputs());
        match self.with_ui(&mut batch, |ui| self.screen.click(ui)) {
            Some(Action::Open(address)) => packs::open_uri(address),
            Some(Action::ImportDetected) => {
                if let Some(zip_path) = self.detected_zip.clone() {
                    self.start_import(Some(zip_path));
                }
            }
            Some(Action::PickZip) => self.start_import(None),
            Some(Action::Quit) => event_loop.exit(),
            None => {}
        }
    }
}

impl ApplicationHandler for Missing {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes =
            horizon_client::window::with_icon(Window::default_attributes().with_title("Horizon"))
                .with_inner_size(winit::dpi::LogicalSize::new(1040.0, 700.0));
        let made = event_loop
            .create_window(attributes)
            .map_err(Into::into)
            .and_then(|window| {
                let window = Arc::new(window);
                let renderer = loading_renderer(
                    window.clone(),
                    false,
                    Some(&self.gui.atlas),
                    Some(&self.gui.panorama),
                    None,
                    self.api,
                    false,
                )?;
                Ok((window, renderer))
            });
        match made {
            Ok((window, renderer)) => {
                window.request_redraw();
                self.window = Some(window);
                self.renderer = Some(renderer);
            }
            Err(e) => {
                self.error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone().filter(|w| w.id() == id) else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => {
                if self.import_rx.is_none() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::DroppedFile(path) => {
                self.start_import(Some(path));
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = (position.x, position.y);
                window.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.click(event_loop),
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                    && self.import_rx.is_none() =>
            {
                event_loop.exit()
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if horizon_threads::stop_requests() > 0 {
            event_loop.exit();
            return;
        }
        self.poll_import(event_loop);
        let delay = if self.import_rx.is_some() {
            Duration::from_millis(30)
        } else {
            Duration::from_millis(200)
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + delay));
    }
}
