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
}
