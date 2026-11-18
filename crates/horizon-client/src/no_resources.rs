//! The screen of a game started without its prepared files: the game build
//! run by hand instead of through a launcher. It says where to get them, and
//! quits. Drawn in Horizon's design language with the bundled fonts: none of
//! Minecraft's files are there.
//!
//! The page it sends the player to comes from a hook (`horizon_hooks::Hooks::get_the_game`,
//! Horizon's launcher in the official executable). With none, it says how to prepare the files
//! and has only Quit.
use crate::lang;
use crate::ui_theme::{self, Face, Ui, color};
use horizon_hooks::GetTheGame;

const COLUMN: f32 = 560.0;
const BUTTON_GAP: f32 = 12.0;

/// What a click on the screen does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open the hook's page.
    Open(&'static str),
    /// Import the auto-detected `.zip` from `~/Downloads` in one click.
    ImportDetected,
    /// Open a native file picker to choose a `.zip` resource archive.
    PickZip,
    Quit,
}

pub struct NoResources {
    title: String,
    paragraphs: Vec<String>,
    /// The buttons' labels and actions, the hook's first.
    buttons: Vec<(String, Action)>,
    /// Where the prepared files were looked for, for whoever starts the game by hand on purpose.
    looked_in: String,
    progress: Option<(u8, String)>,
    error_line: Option<String>,
}

impl NoResources {
    /// The screen in the current language, with `get`'s page when a hook answered
    /// (`horizon_hooks::engine::get_the_game`).
    pub fn new(looked_in: &std::path::Path, get: Option<GetTheGame>) -> Self {
        let shown = std::path::absolute(looked_in).unwrap_or_else(|_| looked_in.to_path_buf());
        let quit = (lang::get("horizon.no_resources.quit"), Action::Quit);
        let (title, paragraphs, buttons) = match get {
            Some(get) => {
                let site = get
                    .address
                    .split_once("://")
                    .map_or(get.address, |(_, s)| s);
                (
                    lang::get(get.title),
                    get.paragraphs
                        .iter()
                        .map(|p| lang::tr(p, &[&site]))
                        .collect(),
                    vec![(lang::get(get.button), Action::Open(get.address)), quit],
                )
            }
            None => (
                lang::get("horizon.no_resources.title"),
                vec![lang::get("horizon.no_resources.prepare")],
                vec![quit],
            ),
        };
        Self {
            title,
            paragraphs,
            buttons,
            looked_in: lang::tr("horizon.no_resources.looked_in", &[&shown.display()]),
            progress: None,
            error_line: None,
        }
    }

    /// Interactive standalone setup screen allowing direct `.zip` archive import
    /// (one-click from `~/Downloads`, file picker, or drag-and-drop onto the window).
    pub fn new_interactive(
        looked_in: &std::path::Path,
        zh: bool,
        detected_zip: Option<&std::path::Path>,
    ) -> Self {
        let shown = std::path::absolute(looked_in).unwrap_or_else(|_| looked_in.to_path_buf());
        let mut buttons = Vec::new();
        if let Some(zip_path) = detected_zip {
            let name = zip_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "resources.zip".into());
            let label = if zh {
                format!("一键导入 Downloads/{name}")
            } else {
                format!("Import Downloads/{name}")
            };
            buttons.push((label, Action::ImportDetected));
        }
        buttons.push((
            if zh {
                "选择资源压缩包 (.zip)…".to_owned()
            } else {
                "Choose Resources (.zip)…".to_owned()
            },
            Action::PickZip,
        ));
        buttons.push((
            if zh {
                "退出".to_owned()
            } else {
                lang::get("horizon.no_resources.quit")
            },
            Action::Quit,
        ));

        let (title, paragraphs, looked_in_str) = if zh {
            (
                format!("导入 {} 游戏资源即可畅玩", horizon_core::VANILLA_VERSION),
                vec![
                    format!(
                        "尚未检测到已准备的 Minecraft: Java Edition {} 资源包（如 horizon-{}-resources.zip）。",
                        horizon_core::VANILLA_VERSION,
                        horizon_core::VANILLA_VERSION
                    ),
                    "请点击下方按钮选择资源 .zip 压缩包，或直接将 .zip 压缩包拖入此窗口。解压完成后将自动进入离线模式主界面，后续启动秒进无需重复导入。".to_owned(),
                ],
                format!("资源存放目录: {}", shown.display()),
            )
        } else {
            (
                format!(
                    "Import {} Resources to Play Offline",
                    horizon_core::VANILLA_VERSION
                ),
                vec![
                    format!(
                        "Prepared Minecraft: Java Edition {} game files (e.g. horizon-{}-resources.zip) have not been imported yet.",
                        horizon_core::VANILLA_VERSION,
                        horizon_core::VANILLA_VERSION
                    ),
                    "Click below to choose a .zip archive, or drag and drop the .zip directly onto this window. Once extracted, Horizon will launch straight into Offline Mode.".to_owned(),
                ],
                lang::tr("horizon.no_resources.looked_in", &[&shown.display()]),
            )
        };

        Self {
            title,
            paragraphs,
            buttons,
            looked_in: looked_in_str,
            progress: None,
            error_line: None,
        }
    }

    pub fn set_progress(&mut self, progress: Option<(u8, String)>) {
        self.progress = progress;
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error_line = error;
    }

    /// The buttons' labels, actions and rectangles in dp, and the paragraphs' wrapped
    /// lines: one layout for drawing and clicks.
    fn layout(&self, ui: &Ui) -> Layout {
        let width = COLUMN.min(ui.w - 64.0).max(160.0);
        let x = ((ui.w - width) / 2.0).round();
        let lines: Vec<Vec<String>> = self
            .paragraphs
            .iter()
            .map(|p| wrap(ui, Face::Regular, body(), p, width))
            .collect();
        let text_h = lines.iter().map(|l| l.len() as f32 * LINE).sum::<f32>()
            + (lines.len() - 1) as f32 * PARAGRAPH_GAP;
        let buttons_start_y = TITLE_LINE + 16.0 + text_h + 28.0;
        let mut temp_buttons: Vec<(String, Action, f32, f32, f32)> = Vec::new();
        let mut bx = 0.0f32;
        let mut row_y = 0.0f32;
        for (label, action) in &self.buttons {
            let w = (ui.measure(Face::Medium, button_text(), label) + 36.0)
                .round()
                .min(width);
            if bx > 0.0 && bx + w > width {
                bx = 0.0;
                row_y += control() + BUTTON_GAP;
            }
            temp_buttons.push((label.clone(), *action, bx, row_y, w));
            bx += w + BUTTON_GAP;
        }
        let buttons_total_h = row_y + control();
        let height = buttons_start_y + buttons_total_h + 20.0 + SMALL_LINE;
        let y = ((ui.h - height) / 2.0).max(24.0).round();
        let buttons_y = y + buttons_start_y;
        let buttons = temp_buttons
            .into_iter()
            .map(|(label, action, rx, ry, w)| Button {
                label,
                action,
                x: x + rx,
                y: buttons_y + ry,
                w,
            })
            .collect();
        Layout {
            x,
            y,
            width,
            lines,
            buttons,
        }
    }

    pub fn draw(&self, ui: &mut Ui) {
        let layout = self.layout(ui);
        ui.rounded(0.0, 0.0, ui.w, ui.h, 0.0, color::bg());
        let mut y = layout.y;
        ui.text_line(
            Face::Medium,
            title_size(),
            TITLE_LINE,
            layout.x,
            y,
            color::text(),
            &self.title,
        );
        y += TITLE_LINE + 16.0;
        for paragraph in &layout.lines {
            for line in paragraph {
                ui.text_line(
                    Face::Regular,
                    body(),
                    LINE,
                    layout.x,
                    y,
                    color::text(),
                    line,
                );
                y += LINE;
            }
            y += PARAGRAPH_GAP;
        }
        let mut bottom_controls_y = y;
        if let Some((pct, status)) = &self.progress {
            let bar_y = layout
                .buttons
                .first()
                .map_or(y + 12.0, |b| b.y);
            let bar_h = 12.0;
            ui.rounded(layout.x, bar_y, layout.width, bar_h, bar_h / 2.0, color::raised());
            let fill_w = (layout.width * (*pct as f32 / 100.0)).clamp(0.0, layout.width);
            if fill_w > 1.0 {
                ui.rounded(layout.x, bar_y, fill_w, bar_h, bar_h / 2.0, color::accent());
            }
            ui.text_line(
                Face::Medium,
                ui_theme::small(),
                SMALL_LINE,
                layout.x,
                bar_y + bar_h + 8.0,
                color::text(),
                status,
            );
            bottom_controls_y = bar_y + bar_h + 8.0 + SMALL_LINE;
        } else {
            let primary_action = self
                .buttons
                .first()
                .map(|(_, a)| *a)
                .filter(|a| *a != Action::Quit);
            for button in &layout.buttons {
                let hovered = button.contains(ui.cursor);
                let is_primary = Some(button.action) == primary_action
                    || matches!(button.action, Action::Open(_));
                let (plate, ink) = match (is_primary, hovered) {
                    (true, false) => (color::accent(), color::on_accent()),
                    (true, true) => (color::with_alpha(color::accent(), 0.85), color::on_accent()),
                    (false, false) => (color::raised(), color::text()),
                    (false, true) => (color::hover(), color::text()),
                };
                let h = control();
                ui.rounded(button.x, button.y, button.w, h, h / 2.0, plate);
                let lw = ui.measure(Face::Medium, button_text(), &button.label);
                let lx = button.x + ((button.w - lw) / 2.0).round();
                ui.text_line(
                    Face::Medium,
                    button_text(),
                    h,
                    lx,
                    button.y,
                    ink,
                    &button.label,
                );
                bottom_controls_y = bottom_controls_y.max(button.y + h);
            }
        }
        let mut small_y = bottom_controls_y + 20.0;
        if let Some(err) = &self.error_line {
            for line in wrap(ui, Face::Medium, ui_theme::small(), err, layout.width) {
                ui.text_line(
                    Face::Medium,
                    ui_theme::small(),
                    SMALL_LINE,
                    layout.x,
                    small_y,
                    crate::hud::rgba(248, 113, 113, 255),
                    &line,
                );
                small_y += SMALL_LINE;
            }
            small_y += 6.0;
        }
        for (i, line) in wrap(
            ui,
            Face::Regular,
            ui_theme::small(),
            &self.looked_in,
            layout.width,
        )
        .iter()
        .enumerate()
        {
            let line_y = small_y + i as f32 * SMALL_LINE;
            ui.text_line(
                Face::Regular,
                ui_theme::small(),
                SMALL_LINE,
                layout.x,
                line_y,
                color::muted(),
                line,
            );
        }
    }

    /// The button under the cursor (`ui.cursor`, in dp), if any.
    pub fn click(&self, ui: &Ui) -> Option<Action> {
        if self.progress.is_some() {
            return None;
        }
        self.layout(ui)
            .buttons
            .iter()
            .find(|b| b.contains(ui.cursor))
            .map(|b| b.action)
    }
}

fn title_size() -> f32 {
    ui_theme::sizes().title
}
const TITLE_LINE: f32 = 40.0;
fn body() -> f32 {
    ui_theme::sizes().body
}
const LINE: f32 = 24.0;
const PARAGRAPH_GAP: f32 = 10.0;
fn button_text() -> f32 {
    ui_theme::sizes().ui
}
const SMALL_LINE: f32 = 18.0;
fn control() -> f32 {
    ui_theme::sizes().control_large
}

struct Layout {
    x: f32,
    y: f32,
    width: f32,
    lines: Vec<Vec<String>>,
    buttons: Vec<Button>,
}

struct Button {
    label: String,
    action: Action,
    x: f32,
    y: f32,
    w: f32,
}

impl Button {
    fn contains(&self, (cx, cy): (f32, f32)) -> bool {
        cx >= self.x && cx < self.x + self.w && cy >= self.y && cy < self.y + control()
    }
}

/// `text` broken at spaces into lines no wider than `width` dp (a longer word,
/// such as a path, gets a line of its own).
fn wrap(ui: &Ui, face: Face, size: f32, text: &str, width: f32) -> Vec<String> {
    if !text.is_ascii() && !text.contains(' ') {
        let mut lines = Vec::new();
        let mut line = String::new();
        for ch in text.chars() {
            let mut candidate = line.clone();
            candidate.push(ch);
            if !line.is_empty() && ui.measure(face, size, &candidate) > width {
                lines.push(std::mem::take(&mut line));
                line.push(ch);
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        return lines;
    }
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        if !word.is_ascii() && ui.measure(face, size, word) > width {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            for ch in word.chars() {
                let mut candidate = line.clone();
                candidate.push(ch);
                if !line.is_empty() && ui.measure(face, size, &candidate) > width {
                    lines.push(std::mem::take(&mut line));
                    line.push(ch);
                } else {
                    line = candidate;
                }
            }
            continue;
        }
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && ui.measure(face, size, &candidate) > width {
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{HudAtlas, HudBatch};

    /// A Ui over a `size` window at GUI scale 1, the cursor at `cursor` pixels.
    fn with_ui<R>(size: [u32; 2], cursor: (f32, f32), f: impl FnOnce(&mut Ui) -> R) -> R {
        // Fonts cache glyphs internally, so reuse the atlas only on this test thread.
        // Window size and cursor still belong to a fresh batch for every call.
        thread_local! {
            static ATLAS: HudAtlas = HudAtlas::build(
                &horizon_assets::resource_source::ResourceSource::directory(
                    std::path::Path::new("/nonexistent/horizon-test-assets").to_owned(),
                ),
            ).unwrap();
        }
        ATLAS.with(|atlas| {
            let mut batch = HudBatch::scratch(size);
            let cursor = (cursor.0 / batch.scale, cursor.1 / batch.scale);
            let mut ui = Ui::new(atlas, &mut batch, cursor);
            f(&mut ui)
        })
    }

    /// A hook's answer, with the game's own Horizon strings and another page.
    const GET: GetTheGame = GetTheGame {
        address: "https://example.org/get",
        title: "horizon.get_the_game.title",
        paragraphs: &[
            "horizon.get_the_game.about",
            "horizon.get_the_game.minecraft",
            "horizon.get_the_game.download",
        ],
        button: "horizon.get_the_game.button",
    };

    /// The screen in English, every key it asks for there.
    fn screen(get: Option<GetTheGame>) -> NoResources {
        let (screen, missed) = lang::scoped(lang::Language::load(&[], "en_us"), || {
            NoResources::new(std::path::Path::new(".local/horizon-client/26.3"), get)
        });
        assert_eq!(missed, Vec::<String>::new());
        screen
    }

    #[test]
    fn without_a_hook_it_names_no_site_and_opens_no_page() {
        let screen = screen(None);
        let all = [&screen.title, &screen.looked_in]
            .into_iter()
            .chain(&screen.paragraphs)
            .chain(screen.buttons.iter().map(|(label, _)| label));
        for text in all {
            assert!(!text.to_lowercase().contains("playhorizon"), "{text}");
            assert!(!text.contains("launcher"), "{text}");
        }
        assert!(screen.paragraphs[0].contains("horizon-prepare --prepare"));
        assert_eq!(
            screen.buttons,
            [("Quit".to_owned(), Action::Quit)],
            "only Quit"
        );
    }

    #[test]
    fn a_hook_gives_the_text_and_the_page() {
        let screen = screen(Some(GET));
        assert_eq!(screen.title, "Start Horizon from the launcher");
        assert_eq!(
            screen.paragraphs[2],
            "Download the launcher at example.org/get, then press Play there."
        );
        assert_eq!(
            screen.buttons[0],
            (
                "Download the launcher".to_owned(),
                Action::Open("https://example.org/get")
            )
        );
    }

    #[test]
    fn the_buttons_are_where_they_are_drawn() {
        let screen = screen(Some(GET));
        for size in [[1280, 720], [800, 600], [2560, 1440]] {
            let (open, quit, density) = with_ui(size, (0.0, 0.0), |ui| {
                let layout = screen.layout(ui);
                let center = |b: &Button| (b.x + b.w / 2.0, b.y + control() / 2.0);
                (
                    center(&layout.buttons[0]),
                    center(&layout.buttons[1]),
                    ui.density,
                )
            });
            let at = |p: (f32, f32)| (p.0 * density, p.1 * density);
            assert_eq!(
                with_ui(size, at(open), |ui| screen.click(ui)),
                Some(Action::Open(GET.address)),
                "{size:?}"
            );
            assert_eq!(
                with_ui(size, at(quit), |ui| screen.click(ui)),
                Some(Action::Quit)
            );
            assert_eq!(with_ui(size, (1.0, 1.0), |ui| screen.click(ui)), None);
            // Everything fits in the window.
            with_ui(size, (0.0, 0.0), |ui| {
                let layout = screen.layout(ui);
                let last = layout.buttons.last().unwrap();
                assert!(
                    last.x + last.w <= ui.w && last.y + control() <= ui.h,
                    "{size:?}"
                );
            });
        }
    }

    #[test]
    fn scaled_draw_and_click_agree() {
        let screen = screen(Some(GET));
        let atlas = HudAtlas::build(&horizon_assets::resource_source::ResourceSource::directory(
            std::path::PathBuf::from("/nonexistent/horizon-test-assets"),
        ))
        .unwrap();
        for unicode in [false, true] {
            let settings = crate::hud::GuiScale {
                requested: 3,
                unicode,
            };
            let mut draw = HudBatch::new([960, 720], settings);
            assert_eq!(draw.scale, if unicode { 4.0 } else { 3.0 });
            let mut ui = Ui::new(&atlas, &mut draw, (0.0, 0.0));
            let layout = screen.layout(&ui);
            let button = &layout.buttons[0];
            let physical = (
                (button.x + button.w / 2.0) * ui.density,
                (button.y + control() / 2.0) * ui.density,
            );
            screen.draw(&mut ui);
            let mut hit = HudBatch::new([960, 720], settings);
            let cursor = (physical.0 / hit.scale, physical.1 / hit.scale);
            let ui = Ui::new(&atlas, &mut hit, cursor);
            assert_eq!(screen.click(&ui), Some(Action::Open(GET.address)));
        }
    }

    #[test]
    fn text_wraps_within_the_column_and_keeps_every_word() {
        let screen = screen(Some(GET));
        with_ui([1280, 720], (0.0, 0.0), |ui| {
            for paragraph in &screen.paragraphs {
                let lines = wrap(ui, Face::Regular, body(), paragraph, 300.0);
                assert!(lines.len() > 1, "{paragraph}");
                for line in &lines {
                    assert!(ui.measure(Face::Regular, body(), line) <= 300.0, "{line}");
                }
                assert_eq!(&lines.join(" "), paragraph);
            }
        });
    }
}
