use enigo::{Direction::Click, Enigo, Key, Keyboard, Settings};

/// Corrects a mistyped chord: backspaces the `press_count` literal
/// characters the OS already typed (which may differ from the matched
/// word's length — see dictionary.rs on repeated letters), then types the
/// matched word followed by a space.
pub struct Injector {
    enigo: Enigo,
}

impl Injector {
    pub fn new() -> anyhow::Result<Self> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("failed to init input injector: {e}"))?;
        Ok(Self { enigo })
    }

    pub fn replace(&mut self, press_count: usize, word: &str) -> anyhow::Result<()> {
        for _ in 0..press_count {
            self.enigo
                .key(Key::Backspace, Click)
                .map_err(|e| anyhow::anyhow!("backspace failed: {e}"))?;
        }
        self.enigo
            .text(&format!("{word} "))
            .map_err(|e| anyhow::anyhow!("text injection failed: {e}"))?;
        Ok(())
    }

    /// Sends a synthetic Enter/Return press — used to force-submit a
    /// benchmark drill at a hard time limit instead of waiting on the user
    /// to press it themselves.
    pub fn press_enter(&mut self) -> anyhow::Result<()> {
        self.enigo
            .key(Key::Return, Click)
            .map_err(|e| anyhow::anyhow!("enter injection failed: {e}"))
    }

    /// Sends a synthetic Space press — the auto-delimiter injected the
    /// instant a chord releases in the race benchmark.
    pub fn press_space(&mut self) -> anyhow::Result<()> {
        self.enigo
            .key(Key::Space, Click)
            .map_err(|e| anyhow::anyhow!("space injection failed: {e}"))
    }

    /// Sends a synthetic Backspace press. Paired with press_space in the
    /// resolution test (the space it deletes keeps the line net-neutral).
    pub fn press_backspace(&mut self) -> anyhow::Result<()> {
        self.enigo
            .key(Key::Backspace, Click)
            .map_err(|e| anyhow::anyhow!("backspace injection failed: {e}"))
    }
}
