//! Headless Base UI state + shadcn/Tailwind v4 theme tokens for GPUI Starter Pack apps.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterButtonVariant {
    Default,
    Secondary,
    Outline,
    Ghost,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TogglePrimitiveState {
    pub checked: bool,
    pub disabled: bool,
}

impl TogglePrimitiveState {
    pub fn toggle(&mut self) -> bool {
        if !self.disabled {
            self.checked = !self.checked;
        }
        self.checked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_ui_toggle_respects_disabled_state() {
        let mut active = TogglePrimitiveState { checked: false, disabled: false };
        assert!(active.toggle());
        let mut locked = TogglePrimitiveState { checked: false, disabled: true };
        assert!(!locked.toggle());
    }
}
