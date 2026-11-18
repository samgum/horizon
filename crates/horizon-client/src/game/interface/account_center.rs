//! Main Menu top-left Account Pill badge and in-window Account & Resource Center
//! modal: provides zero-gate Offline Mode (customizable `1..=16` char username)
//! alongside Microsoft OAuth 2.0 Device-Code genuine login and 1-click `.zip`
//! resource archive re-import.

use super::*;
use horizon_client::auth::msa;
use horizon_client::clipboard::SystemClipboard;
use horizon_client::hud::{HudAtlas, HudBatch, rgba};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{LazyLock, Mutex};
use winit::keyboard::ModifiersState;

enum WorkerMsg {
    Status {
        text: String,
        is_error: bool,
    },
    DeviceCode {
        user_code: String,
        verification_uri: String,
        status: String,
    },
    IdentityApplied {
        player_name: String,
        status: String,
        is_error: bool,
    },
    ResourceProgress {
        pct: u8,
        text: String,
    },
    ResourceDone(std::result::Result<PathBuf, String>),
}

#[derive(Default)]
struct State {
    open: bool,
    offline_input: String,
    input_focused: bool,
    status: Option<(String, bool)>,
    device_code: Option<(String, String)>,
    busy_progress: Option<u8>,
    rx: Option<Receiver<WorkerMsg>>,
    pending_player_name: Option<String>,
    pending_copy_clipboard: Option<String>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));
static SIGNIN_GEN: AtomicU64 = AtomicU64::new(0);

fn game_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| horizon_client::logging::game_home())
}

fn save_offline_player_file(name: &str) {
    let dir = horizon_client::logging::player_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(horizon_client::offline_name::FILE), format!("{name}\n"));
}

fn pill_rect_and_label(
    atlas: &HudAtlas,
    batch: &HudBatch,
    player_name: &str,
    zh: bool,
) -> ([f32; 4], String, bool) {
    let (label, genuine) = if let Some(sess) = horizon_client::auth::current() {
        let s = if zh {
            format!("正版: {} [账号]", sess.name)
        } else {
            format!("Microsoft: {} [Account]", sess.name)
        };
        (s, true)
    } else {
        let s = if zh {
            format!("离线: {player_name} [账号]")
        } else {
            format!("Offline: {player_name} [Account]")
        };
        (s, false)
    };
    let tw = HudBatch::text_width(atlas, &label);
    let w = (tw + 18.0).clamp(80.0, (batch.width * 0.55).max(90.0));
    ([6.0, 6.0, w, 18.0], label, genuine)
}

fn contains([rx, ry, rw, rh]: [f32; 4], (cx, cy): (f32, f32)) -> bool {
    cx >= rx && cx < rx + rw && cy >= ry && cy < ry + rh
}

fn draw_box(
    atlas: &HudAtlas,
    batch: &mut HudBatch,
    [x, y, w, h]: [f32; 4],
    bg: u32,
    border: u32,
) {
    batch.fill(atlas, x, y, w, h, bg);
    batch.fill(atlas, x, y, w, 1.0, border);
    batch.fill(atlas, x, y + h - 1.0, w, 1.0, border);
    batch.fill(atlas, x, y, 1.0, h, border);
    batch.fill(atlas, x + w - 1.0, y, 1.0, h, border);
}

fn draw_btn(
    atlas: &HudAtlas,
    batch: &mut HudBatch,
    rect: [f32; 4],
    label: &str,
    cursor: (f32, f32),
    accent: bool,
) {
    let hov = contains(rect, cursor);
    let (bg, border, fg) = match (accent, hov) {
        (true, false) => (rgba(37, 99, 235, 235), rgba(96, 165, 250, 255), rgba(255, 255, 255, 255)),
        (true, true) => (rgba(59, 130, 246, 245), rgba(147, 197, 253, 255), rgba(255, 255, 255, 255)),
        (false, false) => (rgba(30, 34, 46, 235), rgba(110, 120, 145, 215), rgba(235, 240, 248, 255)),
        (false, true) => (rgba(48, 55, 74, 245), rgba(180, 192, 218, 240), rgba(255, 255, 255, 255)),
    };
    draw_box(atlas, batch, rect, bg, border);
    draw_fitted_text(atlas, batch, label, rect, fg, true);
}

fn draw_fitted_text(
    atlas: &HudAtlas,
    batch: &mut HudBatch,
    text: &str,
    [rx, ry, rw, rh]: [f32; 4],
    color: u32,
    centered: bool,
) {
    let tw = HudBatch::text_width(atlas, text).max(1.0);
    let max_w = (rw - 8.0).max(4.0);
    let scale = (max_w / tw).min(1.0);
    let shown_w = tw * scale;
    let tx = if centered {
        rx + (rw - shown_w) * 0.5
    } else {
        rx + 5.0
    };
    let ty = ry + (rh - 8.0) * 0.5;
    let start = batch.vertices.len();
    batch.text(atlas, text, tx, ty, color, true);
    if scale < 1.0 {
        batch.scale_since(start, tx, ry + rh * 0.5, scale, scale);
    }
}

#[derive(Clone)]
struct ModalLayout {
    card: [f32; 4],
    close_btn: [f32; 4],
    input_box: [f32; 4],
    apply_offline_btn: [f32; 4],
    account_rows: Vec<(String, String, bool, [f32; 4], [f32; 4])>,
    msa_primary_btn: [f32; 4],
    open_browser_btn: Option<[f32; 4]>,
    paste_cid_btn: [f32; 4],
    paste_session_btn: [f32; 4],
    reimport_zip_btn: [f32; 4],
}

fn compute_modal_layout(batch: &HudBatch, store: &msa::AccountStore, has_dc: bool) -> ModalLayout {
    let cw = 310.0f32.min((batch.width - 16.0).max(200.0));
    let ch = 204.0f32.min((batch.height - 16.0).max(168.0));
    let cx = ((batch.width - cw) * 0.5).floor();
    let cy = ((batch.height - ch) * 0.5).floor();
    let pad = 8.0;
    let inner_w = cw - pad * 2.0;

    let close_btn = [cx + cw - 22.0, cy + 5.0, 16.0, 16.0];
    let off_y = cy + 38.0;
    let apply_w = 88.0f32.min(inner_w * 0.38);
    let input_w = (inner_w - apply_w - 6.0).max(60.0);
    let input_box = [cx + pad, off_y, input_w, 18.0];
    let apply_offline_btn = [cx + pad + input_w + 6.0, off_y, apply_w, 18.0];

    let active_uuid = match &store.mode {
        msa::AccountMode::Offline => "",
        msa::AccountMode::Microsoft { uuid } => uuid.as_str(),
    };
    let mut account_rows = Vec::new();
    let mut row_y = cy + 74.0;
    for acc in store.accounts.iter().take(2) {
        let active = acc.uuid == active_uuid;
        let use_rect = [cx + cw - pad - 76.0, row_y, 38.0, 15.0];
        let del_rect = [cx + cw - pad - 34.0, row_y, 34.0, 15.0];
        account_rows.push((acc.uuid.clone(), acc.name.clone(), active, use_rect, del_rect));
        row_y += 17.0;
    }

    let msa_btn_y = cy + ch - 68.0;
    let (msa_primary_btn, open_browser_btn) = if has_dc {
        let half = (inner_w - 6.0) * 0.5;
        (
            [cx + pad + half + 6.0, msa_btn_y, half, 17.0],
            Some([cx + pad, msa_btn_y, half, 17.0]),
        )
    } else {
        ([cx + pad, msa_btn_y, inner_w, 17.0], None)
    };

    let adv_y = cy + ch - 47.0;
    let half_w = (inner_w - 6.0) * 0.5;
    let paste_cid_btn = [cx + pad, adv_y, half_w, 16.0];
    let paste_session_btn = [cx + pad + half_w + 6.0, adv_y, half_w, 16.0];

    let reimport_zip_btn = [cx + pad, cy + ch - 26.0, inner_w, 17.0];

    ModalLayout {
        card: [cx, cy, cw, ch],
        close_btn,
        input_box,
        apply_offline_btn,
        account_rows,
        msa_primary_btn,
        open_browser_btn,
        paste_cid_btn,
        paste_session_btn,
        reimport_zip_btn,
    }
}

/// Polls background sign-in / resource-import events and returns `Some(new_player_name)`
/// whenever the active player name changed.
pub(crate) fn poll_updates(clipboard: &SystemClipboard) -> Option<String> {
    let Ok(mut st) = STATE.lock() else {
        return None;
    };
    if let Some(code) = st.pending_copy_clipboard.take() {
        clipboard.set_text(&code);
    }
    let msgs: Vec<WorkerMsg> = st
        .rx
        .as_ref()
        .map(|rx| rx.try_iter().collect())
        .unwrap_or_default();
    let mut clear_rx = false;
    for msg in msgs {
        match msg {
            WorkerMsg::Status { text, is_error } => {
                st.status = Some((text, is_error));
                if is_error {
                    st.device_code = None;
                    clear_rx = true;
                }
            }
            WorkerMsg::DeviceCode {
                user_code,
                verification_uri,
                status,
            } => {
                clipboard.set_text(&user_code);
                packs::open_uri(&verification_uri);
                st.device_code = Some((user_code, verification_uri));
                st.status = Some((status, false));
            }
            WorkerMsg::IdentityApplied {
                player_name,
                status,
                is_error,
            } => {
                st.pending_player_name = Some(player_name);
                st.device_code = None;
                st.status = Some((status, is_error));
                clear_rx = true;
            }
            WorkerMsg::ResourceProgress { pct, text } => {
                st.busy_progress = Some(pct);
                st.status = Some((text, false));
            }
            WorkerMsg::ResourceDone(res) => {
                st.busy_progress = None;
                match res {
                    Ok(dir) => {
                        st.status = Some((
                            format!("资源已更新至 {}（重启后生效）", dir.display()),
                            false,
                        ));
                    }
                    Err(err) => {
                        st.status = Some((err, true));
                    }
                }
                clear_rx = true;
            }
        }
    }
    if clear_rx {
        st.rx = None;
    }
    st.pending_player_name.take()
}

pub(crate) fn draw_on_main(
    atlas: &HudAtlas,
    batch: &mut HudBatch,
    cursor: (f32, f32),
    player_name: &str,
    settings_lang: &str,
) {
    if cfg!(test) {
        return;
    }
    let zh = horizon_client::resource_zip::prefer_chinese(settings_lang);
    let (pill_rect, pill_label, genuine) = pill_rect_and_label(atlas, batch, player_name, zh);
    let pill_hov = contains(pill_rect, cursor);
    let bg = if pill_hov {
        rgba(24, 28, 38, 225)
    } else {
        rgba(14, 16, 22, 198)
    };
    let border = if genuine {
        rgba(74, 222, 128, 220)
    } else {
        rgba(180, 190, 208, 195)
    };
    draw_box(atlas, batch, pill_rect, bg, border);
    let dot = if genuine {
        rgba(74, 222, 128, 255)
    } else {
        rgba(250, 204, 21, 240)
    };
    batch.fill(
        atlas,
        pill_rect[0] + 4.0,
        pill_rect[1] + (pill_rect[3] - 4.0) * 0.5,
        4.0,
        4.0,
        dot,
    );
    draw_fitted_text(
        atlas,
        batch,
        &pill_label,
        [
            pill_rect[0] + 8.0,
            pill_rect[1],
            pill_rect[2] - 10.0,
            pill_rect[3],
        ],
        rgba(245, 248, 252, 248),
        false,
    );

    let Ok(st) = STATE.lock() else {
        return;
    };
    if !st.open {
        return;
    }

    // Dim backdrop behind modal.
    batch.fill(atlas, 0.0, 0.0, batch.width, batch.height, rgba(6, 8, 12, 175));
    let store = msa::AccountStore::load(&game_dir());
    let layout = compute_modal_layout(batch, &store, st.device_code.is_some());
    let [cx, cy, cw, _ch] = layout.card;
    draw_box(
        atlas,
        batch,
        layout.card,
        rgba(16, 18, 26, 246),
        rgba(96, 110, 140, 235),
    );

    // Header
    let header = if zh {
        "账号与资源中心（免登录离线 / 微软正版）"
    } else {
        "Account & Resources (Offline / Microsoft)"
    };
    draw_fitted_text(
        atlas,
        batch,
        header,
        [cx + 8.0, cy + 5.0, cw - 34.0, 16.0],
        rgba(248, 250, 252, 255),
        false,
    );
    draw_btn(atlas, batch, layout.close_btn, "X", cursor, false);

    // Offline section label
    let off_title = if zh {
        "离线玩家名 (1~16 位字母/数字/_，免登录单机与离线服)："
    } else {
        "Offline Name (1..16 A-Z/0-9/_, instant offline play):"
    };
    draw_fitted_text(
        atlas,
        batch,
        off_title,
        [cx + 8.0, cy + 23.0, cw - 16.0, 13.0],
        rgba(176, 186, 204, 245),
        false,
    );

    // Offline input box
    let input_border = if st.input_focused {
        rgba(96, 165, 250, 255)
    } else {
        rgba(100, 112, 136, 210)
    };
    draw_box(
        atlas,
        batch,
        layout.input_box,
        rgba(10, 12, 18, 250),
        input_border,
    );
    let shown_input = if st.input_focused {
        format!("{}_", st.offline_input)
    } else {
        st.offline_input.clone()
    };
    draw_fitted_text(
        atlas,
        batch,
        &shown_input,
        layout.input_box,
        rgba(255, 255, 255, 255),
        false,
    );
    draw_btn(
        atlas,
        batch,
        layout.apply_offline_btn,
        if zh { "启用离线模式" } else { "Use Offline" },
        cursor,
        matches!(store.mode, msa::AccountMode::Offline),
    );

    // Microsoft accounts section
    let ms_title = if zh {
        "微软正版账号（支持正版联机服务器）："
    } else {
        "Microsoft Genuine Accounts (online servers):"
    };
    draw_fitted_text(
        atlas,
        batch,
        ms_title,
        [cx + 8.0, cy + 59.0, cw - 16.0, 13.0],
        rgba(176, 186, 204, 245),
        false,
    );

    if let Some((code, _uri)) = &st.device_code {
        let code_line = if zh {
            format!("设备验证码: {code}（已自动复制，请在浏览器粘贴授权）")
        } else {
            format!("Device Code: {code} (copied to clipboard)")
        };
        draw_fitted_text(
            atlas,
            batch,
            &code_line,
            [cx + 8.0, cy + 76.0, cw - 16.0, 18.0],
            rgba(250, 204, 21, 255),
            false,
        );
    } else if layout.account_rows.is_empty() {
        let empty = if zh {
            "尚未登录正版账号（当前可直接离线游玩）"
        } else {
            "No Microsoft account signed in (Offline ready)"
        };
        draw_fitted_text(
            atlas,
            batch,
            empty,
            [cx + 8.0, cy + 76.0, cw - 16.0, 15.0],
            rgba(140, 150, 168, 235),
            false,
        );
    } else {
        for (_uuid, name, active, use_rect, del_rect) in &layout.account_rows {
            let tag = if *active {
                if zh {
                    format!("● {name} (当前使用)")
                } else {
                    format!("● {name} (Active)")
                }
            } else {
                format!("○ {name}")
            };
            draw_fitted_text(
                atlas,
                batch,
                &tag,
                [cx + 8.0, use_rect[1], use_rect[0] - cx - 12.0, 15.0],
                if *active {
                    rgba(74, 222, 128, 255)
                } else {
                    rgba(230, 236, 245, 245)
                },
                false,
            );
            draw_btn(
                atlas,
                batch,
                *use_rect,
                if zh { "启用" } else { "Use" },
                cursor,
                *active,
            );
            draw_btn(
                atlas,
                batch,
                *del_rect,
                if zh { "移除" } else { "Del" },
                cursor,
                false,
            );
        }
    }

    // Status message line
    if let Some((msg, is_err)) = &st.status {
        let col = if *is_err {
            rgba(248, 113, 113, 255)
        } else {
            rgba(134, 239, 172, 255)
        };
        draw_fitted_text(
            atlas,
            batch,
            msg,
            [cx + 8.0, layout.msa_primary_btn[1] - 16.0, cw - 16.0, 14.0],
            col,
            false,
        );
    }

    if let Some(open_rect) = layout.open_browser_btn {
        draw_btn(
            atlas,
            batch,
            open_rect,
            if zh { "打开授权网页" } else { "Open Browser" },
            cursor,
            true,
        );
        draw_btn(
            atlas,
            batch,
            layout.msa_primary_btn,
            if zh { "取消登录" } else { "Cancel" },
            cursor,
            false,
        );
    } else {
        draw_btn(
            atlas,
            batch,
            layout.msa_primary_btn,
            if zh {
                "微软正版设备码登录"
            } else {
                "Sign in with Microsoft (Device Code)"
            },
            cursor,
            !matches!(store.mode, msa::AccountMode::Offline),
        );
    }

    let cid_label = if store.effective_client_id().is_some() {
        if zh {
            "粘贴新 Client ID"
        } else {
            "Paste Client ID"
        }
    } else if zh {
        "粘贴 Azure Client ID"
    } else {
        "Set Azure Client ID"
    };
    draw_btn(atlas, batch, layout.paste_cid_btn, cid_label, cursor, false);
    draw_btn(
        atlas,
        batch,
        layout.paste_session_btn,
        if zh {
            "粘贴 Session JSON"
        } else {
            "Paste Session JSON"
        },
        cursor,
        false,
    );

    let zip_label = if let Some(pct) = st.busy_progress {
        if zh {
            format!("正在导入资源包 ({pct}%)…")
        } else {
            format!("Importing resources ({pct}%)…")
        }
    } else if zh {
        "导入 / 更换游戏资源包 (.zip)…".to_owned()
    } else {
        "Import / Replace Resources (.zip)…".to_owned()
    };
    draw_btn(
        atlas,
        batch,
        layout.reimport_zip_btn,
        &zip_label,
        cursor,
        false,
    );
}

fn apply_offline_name(st: &mut State, zh: bool) -> Option<String> {
    let Some(clean) = msa::sanitize_offline_name(&st.offline_input) else {
        st.status = Some((
            if zh {
                "离线名需为 1~16 位英文字母、数字或下划线 (_)".to_owned()
            } else {
                "Offline name must be 1..16 ASCII letters, digits, or '_'.".to_owned()
            },
            true,
        ));
        return None;
    };
    let dir = game_dir();
    let mut store = msa::AccountStore::load(&dir);
    store.offline_name = clean.clone();
    store.mode = msa::AccountMode::Offline;
    if let Err(e) = store.save(&dir) {
        st.status = Some((format!("保存失败: {e}"), true));
        return None;
    }
    save_offline_player_file(&clean);
    let resolved = msa::resolve_and_activate(&dir, &msa::Endpoints::live(), false);
    st.offline_input = resolved.player_name.clone();
    st.status = Some((
        if zh {
            format!("已切换为离线模式: {}", resolved.player_name)
        } else {
            format!("Switched to Offline Mode: {}", resolved.player_name)
        },
        false,
    ));
    Some(resolved.player_name)
}

fn start_msa_signin(st: &mut State, zh: bool) {
    let dir = game_dir();
    let store = msa::AccountStore::load(&dir);
    let Some(cid) = store.effective_client_id() else {
        st.status = Some((
            if zh {
                "请先复制你的 Azure 应用 Client ID，然后点击下方「粘贴 Azure Client ID」".to_owned()
            } else {
                "Copy your Azure public-client ID first, then click 'Set Azure Client ID'.".to_owned()
            },
            true,
        ));
        return;
    };
    let generation = SIGNIN_GEN.fetch_add(1, Ordering::AcqRel) + 1;
    let (tx, rx) = mpsc::channel();
    st.rx = Some(rx);
    st.status = Some((
        if zh {
            "正在向 Microsoft 请求设备登录码…".to_owned()
        } else {
            "Requesting Microsoft device code…".to_owned()
        },
        false,
    ));

    let _ = std::thread::Builder::new()
        .name("horizon-msa-login".into())
        .spawn(move || {
            let ep = msa::Endpoints::live();
            let dc = match msa::request_device_code(&ep, &cid) {
                Ok(c) => c,
                Err(text) => {
                    if SIGNIN_GEN.load(Ordering::Acquire) == generation {
                        let _ = tx.send(WorkerMsg::Status {
                            text,
                            is_error: true,
                        });
                    }
                    return;
                }
            };
            if SIGNIN_GEN.load(Ordering::Acquire) != generation {
                return;
            }
            let _ = tx.send(WorkerMsg::DeviceCode {
                user_code: dc.user_code.clone(),
                verification_uri: dc.verification_uri.clone(),
                status: if zh {
                    "已复制验证码并打开浏览器，完成授权后将自动登录…".into()
                } else {
                    "Code copied & browser opened; waiting for authorization…".into()
                },
            });
            let tokens = match msa::wait_for_device_tokens(&ep, &cid, &dc, &|| {
                SIGNIN_GEN.load(Ordering::Acquire) != generation
            }) {
                Ok(t) => t,
                Err(text) => {
                    if SIGNIN_GEN.load(Ordering::Acquire) == generation {
                        let _ = tx.send(WorkerMsg::Status {
                            text,
                            is_error: true,
                        });
                    }
                    return;
                }
            };
            if SIGNIN_GEN.load(Ordering::Acquire) != generation {
                return;
            }
            let finished = msa::finish_sign_in(&ep, tokens, &|phase| {
                if SIGNIN_GEN.load(Ordering::Acquire) == generation {
                    let _ = tx.send(WorkerMsg::Status {
                        text: phase.to_owned(),
                        is_error: false,
                    });
                }
            });
            if SIGNIN_GEN.load(Ordering::Acquire) != generation {
                return;
            }
            match finished {
                Ok((account, secrets)) => {
                    msa::save_secrets(&account.uuid, &secrets);
                    let mut cur = msa::AccountStore::load(&dir);
                    cur.upsert_account(account.clone());
                    cur.mode = msa::AccountMode::Microsoft {
                        uuid: account.uuid.clone(),
                    };
                    let _ = cur.save(&dir);
                    let resolved = msa::resolve_and_activate(&dir, &ep, false);
                    let _ = tx.send(WorkerMsg::IdentityApplied {
                        status: format!("正版登录成功: {}", resolved.player_name),
                        player_name: resolved.player_name,
                        is_error: false,
                    });
                }
                Err(text) => {
                    let _ = tx.send(WorkerMsg::Status {
                        text,
                        is_error: true,
                    });
                }
            }
        });
}

fn start_resource_reimport(st: &mut State, source: Option<PathBuf>, zh: bool) {
    if st.busy_progress.is_some() {
        return;
    }
    st.busy_progress = Some(1);
    let (tx, rx) = mpsc::channel();
    st.rx = Some(rx);
    let target = horizon_client::resource_zip::default_extracted_resources_dir();
    let _ = std::thread::Builder::new()
        .name("horizon-reimport-zip".into())
        .spawn(move || {
            let chosen = match source {
                Some(p) => p,
                None => match horizon_client::resource_zip::pick_resource_zip_dialog(zh) {
                    Some(p) => p,
                    None => {
                        let _ = tx.send(WorkerMsg::ResourceDone(Err(if zh {
                            "已取消选择资源压缩包".into()
                        } else {
                            "Resource selection cancelled".into()
                        })));
                        return;
                    }
                },
            };
            let res = horizon_client::resource_zip::import_resources(
                &chosen,
                &target,
                true,
                &|done, total, pct| {
                    let _ = tx.send(WorkerMsg::ResourceProgress {
                        pct,
                        text: format!("正在解压资源: {pct}% ({done}/{total})…"),
                    });
                },
            );
            let _ = tx.send(WorkerMsg::ResourceDone(res));
        });
}

/// Handles mouse/keyboard/drop events for the Account Pill & Modal on `Mode::Main`.
/// Returns `(consumed, updated_player_name)`.
pub(crate) fn handle_main_menu_event(
    event: &WindowEvent,
    atlas: &HudAtlas,
    batch: &HudBatch,
    cursor: (f32, f32),
    modifiers: ModifiersState,
    clipboard: &SystemClipboard,
    player_name: &str,
    settings_lang: &str,
) -> (bool, Option<String>) {
    if cfg!(test) {
        return (false, None);
    }
    let zh = horizon_client::resource_zip::prefer_chinese(settings_lang);
    let (pill_rect, _, _) = pill_rect_and_label(atlas, batch, player_name, zh);

    let Ok(mut st) = STATE.lock() else {
        return (false, None);
    };

    if !st.open {
        if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } = event
            && contains(pill_rect, cursor)
        {
            let store = msa::AccountStore::load(&game_dir());
            st.offline_input = if matches!(store.mode, msa::AccountMode::Offline)
                && !player_name.is_empty()
            {
                player_name.to_owned()
            } else {
                store.offline_name
            };
            st.input_focused = true;
            st.status = None;
            st.open = true;
            return (true, None);
        }
        return (false, None);
    }

    // Modal is open: consume all mouse, wheel, keyboard, IME, and file-drop input.
    match event {
        WindowEvent::DroppedFile(path) => {
            start_resource_reimport(&mut st, Some(path.clone()), zh);
            return (true, None);
        }
        WindowEvent::MouseWheel { .. } => return (true, None),
        WindowEvent::MouseInput {
            state: ElementState::Released,
            ..
        } => return (true, None),
        WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } => {
            let dir = game_dir();
            let store = msa::AccountStore::load(&dir);
            let layout = compute_modal_layout(batch, &store, st.device_code.is_some());

            if !contains(layout.card, cursor) || contains(layout.close_btn, cursor) {
                if st.busy_progress.is_none() {
                    st.open = false;
                }
                return (true, None);
            }
            st.input_focused = contains(layout.input_box, cursor);
            if contains(layout.apply_offline_btn, cursor) {
                let updated = apply_offline_name(&mut st, zh);
                return (true, updated);
            }
            for (uuid, _name, _active, use_rect, del_rect) in &layout.account_rows {
                if contains(*use_rect, cursor) {
                    let mut cur = msa::AccountStore::load(&dir);
                    cur.mode = msa::AccountMode::Microsoft { uuid: uuid.clone() };
                    let _ = cur.save(&dir);
                    let resolved = msa::resolve_and_activate(&dir, &msa::Endpoints::live(), false);
                    st.status = Some((
                        resolved
                            .notice
                            .unwrap_or_else(|| format!("已切换正版账号: {}", resolved.player_name)),
                        false,
                    ));
                    return (true, Some(resolved.player_name));
                }
                if contains(*del_rect, cursor) {
                    msa::delete_secrets(uuid);
                    let mut cur = msa::AccountStore::load(&dir);
                    cur.remove_account(uuid);
                    let _ = cur.save(&dir);
                    let resolved = msa::resolve_and_activate(&dir, &msa::Endpoints::live(), false);
                    st.status = Some(("已移除该微软账号".into(), false));
                    return (true, Some(resolved.player_name));
                }
            }
            if let Some(open_rect) = layout.open_browser_btn
                && contains(open_rect, cursor)
            {
                if let Some((code, uri)) = &st.device_code {
                    clipboard.set_text(code);
                    packs::open_uri(uri);
                }
                return (true, None);
            }
            if contains(layout.msa_primary_btn, cursor) {
                if st.device_code.is_some() {
                    SIGNIN_GEN.fetch_add(1, Ordering::AcqRel);
                    st.device_code = None;
                    st.rx = None;
                    st.status = Some(("已取消微软登录".into(), false));
                } else {
                    start_msa_signin(&mut st, zh);
                }
                return (true, None);
            }
            if contains(layout.paste_cid_btn, cursor) {
                let pasted = clipboard.text().trim().to_owned();
                if pasted.is_empty() {
                    st.status = Some(("剪贴板为空，请先复制 Azure Client ID".into(), true));
                } else {
                    let mut cur = msa::AccountStore::load(&dir);
                    cur.custom_client_id = pasted;
                    let _ = cur.save(&dir);
                    st.status = Some(("已保存自定义 Azure Client ID！现在可点击登录".into(), false));
                }
                return (true, None);
            }
            if contains(layout.paste_session_btn, cursor) {
                let pasted = clipboard.text();
                match msa::import_session_json(&dir, &pasted) {
                    Ok(resolved) => {
                        st.status = Some((format!("已导入正版会话: {}", resolved.player_name), false));
                        return (true, Some(resolved.player_name));
                    }
                    Err(e) => {
                        st.status = Some((format!("Session JSON 导入失败: {e}"), true));
                    }
                }
                return (true, None);
            }
            if contains(layout.reimport_zip_btn, cursor) {
                start_resource_reimport(&mut st, None, zh);
                return (true, None);
            }
            return (true, None);
        }
        WindowEvent::MouseInput { .. } => return (true, None),
        WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
            if st.input_focused {
                for ch in text.chars() {
                    if (ch.is_ascii_alphanumeric() || ch == '_') && st.offline_input.len() < 16 {
                        st.offline_input.push(ch);
                    }
                }
            }
            return (true, None);
        }
        WindowEvent::Ime(_) => return (true, None),
        WindowEvent::KeyboardInput {
            event: key_event, ..
        } => {
            if key_event.state != ElementState::Pressed {
                return (true, None);
            }
            if key_event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                if st.busy_progress.is_none() {
                    st.open = false;
                }
                return (true, None);
            }
            if key_event.physical_key == PhysicalKey::Code(KeyCode::Enter)
                || key_event.physical_key == PhysicalKey::Code(KeyCode::NumpadEnter)
            {
                let updated = apply_offline_name(&mut st, zh);
                return (true, updated);
            }
            if st.input_focused {
                if key_event.physical_key == PhysicalKey::Code(KeyCode::Backspace) {
                    st.offline_input.pop();
                    return (true, None);
                }
                let cmd_or_ctrl = if cfg!(target_os = "macos") {
                    modifiers.super_key()
                } else {
                    modifiers.control_key()
                };
                if cmd_or_ctrl && key_event.physical_key == PhysicalKey::Code(KeyCode::KeyV) {
                    for ch in clipboard.text().trim().chars() {
                        if (ch.is_ascii_alphanumeric() || ch == '_') && st.offline_input.len() < 16
                        {
                            st.offline_input.push(ch);
                        }
                    }
                    return (true, None);
                }
                if !cmd_or_ctrl
                    && let Some(txt) = key_event.text.as_deref()
                {
                    for ch in txt.chars() {
                        if (ch.is_ascii_alphanumeric() || ch == '_') && st.offline_input.len() < 16
                        {
                            st.offline_input.push(ch);
                        }
                    }
                }
            }
            return (true, None);
        }
        _ => {}
    }

    (false, None)
}

impl super::Game<'_> {
    pub(super) fn main_menu_account_pill_active(&self) -> bool {
        self.title
            .as_ref()
            .is_some_and(|t| matches!(t.mode, horizon_client::title::Mode::Main))
            && self.screen == Screen::Playing
            && self.pending_entry.is_none()
            && self.integrated_starting.is_none()
            && self.pack_reload.is_none()
            && self.pending_save.is_none()
            && !self.dialog_open()
    }

    pub(super) fn apply_account_center_name(&mut self, new_name: String) {
        self.options.player_name = Some(new_name.clone());
        if let Some(title) = self.title.take() {
            self.title = Some(title.with_user(move || new_name));
        }
    }
}
