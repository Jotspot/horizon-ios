//! The iOS app's Microsoft account panel (docs/ios.md): a few lines of text and a row of buttons
//! over the screen, drawn like the touch UI's buttons. What it says comes from the game's
//! sign-in (`game/ios_account.rs`); this is only its layout, drawing and hit tests.
use crate::hud::{HudAtlas, HudBatch, rgba};

/// What a button of the panel does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelButton {
    SignIn,
    OpenLink,
    CopyCode,
    SignOut,
    Close,
}

impl PanelButton {
    fn label(self) -> &'static str {
        match self {
            Self::SignIn => "Sign in",
            Self::OpenLink => "Open link",
            Self::CopyCode => "Copy code",
            Self::SignOut => "Sign out",
            Self::Close => "Close",
        }
    }
}

#[derive(Default)]
pub struct AccountPanel {
    pub visible: bool,
    pub lines: Vec<String>,
    pub buttons: Vec<PanelButton>,
}

const LINE: f32 = 11.0;
const PAD: f32 = 8.0;
const BUTTON_H: f32 = 20.0;
const GAP: f32 = 4.0;

struct Layout {
    panel: [f32; 4],
    lines: Vec<String>,
    buttons: Vec<[f32; 4]>,
}

impl AccountPanel {
    fn layout(&self, a: &HudAtlas, w: f32, h: f32) -> Layout {
        let width = 320.0_f32.min(w - 16.0).max(120.0);
        let lines: Vec<String> = self
            .lines
            .iter()
            .flat_map(|line| wrap(a, line, width - 2.0 * PAD))
            .collect();
        let height = PAD + lines.len() as f32 * LINE + PAD + BUTTON_H + PAD;
        let x = ((w - width) / 2.0).round();
        let y = ((h - height) / 2.0).max(4.0).round();
        let n = self.buttons.len().max(1) as f32;
        let bw = ((width - 2.0 * PAD - (n - 1.0) * GAP) / n).floor();
        let by = y + height - PAD - BUTTON_H;
        let buttons = (0..self.buttons.len())
            .map(|i| [x + PAD + i as f32 * (bw + GAP), by, bw, BUTTON_H])
            .collect();
        Layout {
            panel: [x, y, width, height],
            lines,
            buttons,
        }
    }

    pub fn draw(&self, b: &mut HudBatch, a: &HudAtlas) {
        if !self.visible {
            return;
        }
        let layout = self.layout(a, b.width, b.height);
        // Dim the screen behind, then the panel.
        b.fill(a, 0.0, 0.0, b.width, b.height, rgba(0, 0, 0, 140));
        let [x, y, w, h] = layout.panel;
        b.fill(a, x, y, w, h, rgba(16, 18, 24, 235));
        for (i, line) in layout.lines.iter().enumerate() {
            let color = if i == 0 {
                rgba(255, 255, 255, 255)
            } else {
                rgba(220, 220, 225, 255)
            };
            b.text(a, line, x + PAD, y + PAD + i as f32 * LINE, color, true);
        }
        for (button, [bx, by, bw, _]) in self.buttons.iter().zip(&layout.buttons) {
            crate::touch_ui::button(b, a, button.label(), *bx, *by, *bw);
        }
    }

    /// The button under `p` (GUI pixels), if any.
    pub fn hit(&self, a: &HudAtlas, w: f32, h: f32, p: [f32; 2]) -> Option<PanelButton> {
        let layout = self.layout(a, w, h);
        self.buttons
            .iter()
            .zip(&layout.buttons)
            .find(|(_, [x, y, bw, bh])| {
                (*x..x + bw).contains(&p[0]) && (*y..y + bh).contains(&p[1])
            })
            .map(|(button, _)| *button)
    }
}

/// `text` in lines no wider than `width`, broken at spaces.
fn wrap(a: &HudAtlas, text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && HudBatch::text_width(a, &candidate) > width {
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        } else {
            line = candidate;
        }
    }
    lines.push(line);
    lines
}
