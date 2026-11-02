//! Microsoft sign-in on iOS (docs/ios.md): the desktop launcher's account code
//! (`launcher/src/account.rs`), as Android's launcher uses it, behind the touch UI's Account
//! button. A signed-in account becomes the game's session (`auth::set_current`), so online-mode
//! servers take it; the tokens stay in the Keychain (`ios/secure_store.rs`) between launches.
use super::*;
use crate::account::{self, Accounts, Endpoints, Secrets, SignIn, SignInState};
use horizon_client::auth::{self, AuthSession};
use horizon_client::ios_account_panel::{AccountPanel, PanelButton};
use std::cell::RefCell;
use std::sync::mpsc;
use winit::event::{Touch, TouchPhase};

/// The Microsoft app the Android port signs in with: a public client, not one registered for
/// Horizon (docs/android.md, "Accounts"). `HORIZON_MSA_CLIENT_ID` replaces it.
const MICROSOFT_CLIENT_ID: &str = "00000000441cc96b";

fn client_id() -> String {
    account::client_id().unwrap_or_else(|| MICROSOFT_CLIENT_ID.into())
}

#[derive(Default)]
struct State {
    panel: AccountPanel,
    sign_in: Option<SignIn>,
    /// The verification page and the code, while Microsoft waits for the player.
    code: Option<(String, String)>,
    /// A session being restored or finished on a worker: the signed-in account's, or none.
    session: Option<mpsc::Receiver<std::result::Result<Option<AuthSession>, String>>>,
    restored: bool,
    /// The last thing to tell the player (an error, "Signed out").
    message: Option<String>,
}

thread_local! {
    // The main thread's only: touches, frames and drawing all happen there.
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The selected account's session, its tokens renewed when they are close to expiring, as
/// Android's launcher prepares one (`android/login.rs`): through the game's session file, read
/// once and deleted.
fn restore() -> std::result::Result<Option<AuthSession>, String> {
    let mut accounts = Accounts::load();
    let Some(selected) = accounts.selected().cloned() else {
        return Ok(None);
    };
    let secrets: Secrets = account::store()
        .get(&selected.uuid)?
        .and_then(|s| serde_json::from_str(&s).ok())
        .ok_or("The Microsoft session ended. Sign in again.")?;
    let (renewed, secrets) = account::fresh(&Endpoints::live(), &client_id(), &selected, secrets)?;
    accounts.add(account::store(), renewed.clone(), &secrets)?;
    accounts.save()?;
    let dir = std::env::temp_dir().join("horizon-sessions");
    let path = account::write_session_file(&dir, &renewed, &secrets)?;
    let session = AuthSession::load(&path);
    let _ = std::fs::remove_file(&path);
    session.map(Some)
}

fn restore_on_worker() -> mpsc::Receiver<std::result::Result<Option<AuthSession>, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("Microsoft session".into())
        .spawn(move || {
            let _ = sender.send(restore());
        })
        .ok();
    receiver
}

impl Game<'_> {
    /// Once a frame: the Account button, the sign-in's progress and the session to use.
    pub(super) fn ios_account_frame(&mut self) {
        self.pad.touch_ui.account_button = true;
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if !state.restored {
                state.restored = true;
                state.session = Some(restore_on_worker());
            }
            if let Some(receiver) = &state.session
                && let Ok(result) = receiver.try_recv()
            {
                state.session = None;
                match result {
                    Ok(Some(session)) => {
                        log::info!("Horizon iOS: signed in as {}", session.name);
                        auth::set_current(session);
                        // The chat key pair, ready before the first join.
                        auth::prepare_chat_keys(true);
                        state.message = None;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        log::warn!("Horizon iOS: Microsoft session: {e}");
                        state.message = Some(e);
                    }
                }
            }
            let progress = state.sign_in.as_ref().map(SignIn::state);
            match progress {
                Some(SignInState::Code(code)) => {
                    if state.code.is_none() {
                        // As on Android: the code is copied and the page opens with it filled in.
                        horizon_ios_sys::links::copy_text(&code.user_code);
                        horizon_ios_sys::links::open_url(&with_code(
                            &code.verification_uri,
                            &code.user_code,
                        ));
                    }
                    state.code = Some((code.verification_uri, code.user_code));
                }
                Some(SignInState::Done(result)) => {
                    state.sign_in = None;
                    state.code = None;
                    match result {
                        Ok(account) => {
                            state.message = Some(format!("Signed in as {}.", account.name));
                            state.session = Some(restore_on_worker());
                        }
                        Err(e) => state.message = Some(e),
                    }
                }
                _ => {}
            }
            let (lines, buttons) = panel_contents(&state);
            state.panel.lines = lines;
            state.panel.buttons = buttons;
        });
    }

    pub(super) fn ios_account_open(&mut self) {
        STATE.with(|state| state.borrow_mut().panel.visible = true);
    }

    pub(super) fn ios_account_draw(&self, hud: &mut HudBatch) {
        STATE.with(|state| state.borrow().panel.draw(hud, &self.gui.atlas));
    }

    /// A touch while the panel is open: it takes every touch, and its buttons act on release.
    pub(super) fn ios_account_touch(&mut self, t: &Touch) -> bool {
        let Some(window) = &self.window else {
            return false;
        };
        let size = window.inner_size();
        let b = HudBatch::new([size.width, size.height]);
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if !state.panel.visible {
                return false;
            }
            if t.phase != TouchPhase::Ended {
                return true;
            }
            match state.panel.hit(&self.gui.atlas, b.width, b.height, p) {
                Some(PanelButton::SignIn) => {
                    state.message = None;
                    state.code = None;
                    state.sign_in = Some(SignIn::start_with_client_id(Some(client_id())));
                }
                Some(PanelButton::OpenLink) => {
                    if let Some((uri, code)) = &state.code {
                        horizon_ios_sys::links::copy_text(code);
                        horizon_ios_sys::links::open_url(&with_code(uri, code));
                    }
                }
                Some(PanelButton::CopyCode) => {
                    if let Some((_, code)) = &state.code {
                        horizon_ios_sys::links::copy_text(code);
                    }
                }
                Some(PanelButton::SignOut) => {
                    let mut accounts = Accounts::load();
                    if let Some(selected) = accounts.selected().cloned() {
                        state.message = Some(
                            match accounts
                                .remove(account::store(), &selected.uuid)
                                .and_then(|()| accounts.save())
                            {
                                Ok(()) => "Signed out. Single player and LAN still work.".into(),
                                Err(e) => e,
                            },
                        );
                    }
                    auth::clear_current();
                }
                Some(PanelButton::Close) => {
                    if let Some(sign_in) = state.sign_in.take() {
                        sign_in.cancel();
                    }
                    state.code = None;
                    state.panel.visible = false;
                }
                None => {}
            }
            true
        })
    }
}

/// The verification page with the code filled in (`otc`), as Android opens it.
fn with_code(uri: &str, code: &str) -> String {
    match url::Url::parse(uri) {
        Ok(mut url) => {
            url.query_pairs_mut().append_pair("otc", code);
            url.into()
        }
        Err(_) => uri.to_owned(),
    }
}

fn panel_contents(state: &State) -> (Vec<String>, Vec<PanelButton>) {
    let mut lines = vec!["Microsoft account".to_owned()];
    let buttons;
    if let Some((uri, code)) = &state.code {
        lines.push(format!("1. Open {uri}"));
        lines.push(format!("2. Enter the code {code} (it is copied)."));
        lines.push("3. Sign in and allow access, then come back here.".into());
        buttons = vec![
            PanelButton::OpenLink,
            PanelButton::CopyCode,
            PanelButton::Close,
        ];
    } else if state.sign_in.is_some() {
        lines.push("Signing in...".into());
        buttons = vec![PanelButton::Close];
    } else if let Some(session) = auth::current() {
        lines.push(format!("Signed in as {}.", session.name));
        lines.push("Online-mode servers will accept you.".into());
        buttons = vec![PanelButton::SignOut, PanelButton::Close];
    } else {
        if state.session.is_some() {
            lines.push("Restoring your Microsoft session...".into());
        } else {
            lines.push(
                "Sign in with the Microsoft account that owns Minecraft: Java Edition to \
                 play on online-mode servers. Single player and LAN work without it."
                    .into(),
            );
        }
        buttons = vec![PanelButton::SignIn, PanelButton::Close];
    }
    if let Some(message) = &state.message {
        lines.push(message.clone());
    }
    (lines, buttons)
}
