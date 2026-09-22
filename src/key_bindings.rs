use gtk::gdk;
use gtk::gdk::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum KeyAction {
    NewTab,
    PreviousTab,
    NextTab,
    Copy,
    Paste,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

impl KeyAction {
    const ALL: [Self; 8] = [
        Self::NewTab,
        Self::PreviousTab,
        Self::NextTab,
        Self::Copy,
        Self::Paste,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::ZoomReset,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::NewTab => "new_tab",
            Self::PreviousTab => "previous_tab",
            Self::NextTab => "next_tab",
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::ZoomIn => "zoom_in",
            Self::ZoomOut => "zoom_out",
            Self::ZoomReset => "zoom_reset",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeyBindings {
    new_tab: Vec<KeyBinding>,
    previous_tab: Vec<KeyBinding>,
    next_tab: Vec<KeyBinding>,
    copy: Vec<KeyBinding>,
    paste: Vec<KeyBinding>,
    zoom_in: Vec<KeyBinding>,
    zoom_out: Vec<KeyBinding>,
    zoom_reset: Vec<KeyBinding>,
}

impl KeyBindings {
    pub(crate) fn bindings(&self, action: KeyAction) -> &[KeyBinding] {
        match action {
            KeyAction::NewTab => &self.new_tab,
            KeyAction::PreviousTab => &self.previous_tab,
            KeyAction::NextTab => &self.next_tab,
            KeyAction::Copy => &self.copy,
            KeyAction::Paste => &self.paste,
            KeyAction::ZoomIn => &self.zoom_in,
            KeyAction::ZoomOut => &self.zoom_out,
            KeyAction::ZoomReset => &self.zoom_reset,
        }
    }

    pub(crate) fn primary_label(&self, action: KeyAction) -> Option<String> {
        self.bindings(action).first().map(KeyBinding::label)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        let mut seen: Vec<ResolvedBinding> = Vec::new();
        for action in KeyAction::ALL {
            for binding in self.bindings(action) {
                let resolved = binding.resolve(action)?;
                validate_reserved_binding(action, &resolved)?;

                if let Some(previous) = seen.iter().find(|previous| {
                    previous.key == resolved.key && previous.modifiers == resolved.modifiers
                }) {
                    return Err(format!(
                        "key binding {} conflicts with {}",
                        action.name(),
                        previous.action.name()
                    ));
                }
                seen.push(resolved);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeyBinding {
    key: String,
    #[serde(default)]
    modifiers: Vec<KeyModifier>,
    #[serde(default, rename = "match")]
    match_mode: KeyMatchMode,
}

impl KeyBinding {
    fn resolve(&self, action: KeyAction) -> Result<ResolvedBinding, String> {
        let key = gdk::Key::from_name(self.key.trim())
            .map(|key| key.to_lower())
            .ok_or_else(|| format!("unknown key name {:?}", self.key))?;
        let mut modifiers = gdk::ModifierType::empty();
        for modifier in &self.modifiers {
            let flag = modifier.flag();
            if modifiers.contains(flag) {
                return Err(format!(
                    "duplicate modifier in key binding for {:?}",
                    self.key
                ));
            }
            modifiers.insert(flag);
        }

        Ok(ResolvedBinding {
            action,
            key,
            modifiers,
            match_mode: self.match_mode,
        })
    }

    fn label(&self) -> String {
        let mut parts: Vec<String> = self
            .modifiers
            .iter()
            .map(|modifier| modifier.label().to_owned())
            .collect();
        parts.push(display_key_name(&self.key));
        parts.join("+")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum KeyModifier {
    Control,
    Shift,
    Alt,
    Super,
    Meta,
    Hyper,
}

impl KeyModifier {
    fn flag(self) -> gdk::ModifierType {
        match self {
            Self::Control => gdk::ModifierType::CONTROL_MASK,
            Self::Shift => gdk::ModifierType::SHIFT_MASK,
            Self::Alt => gdk::ModifierType::ALT_MASK,
            Self::Super => gdk::ModifierType::SUPER_MASK,
            Self::Meta => gdk::ModifierType::META_MASK,
            Self::Hyper => gdk::ModifierType::HYPER_MASK,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Control => "Ctrl",
            Self::Shift => "Shift",
            Self::Alt => "Alt",
            Self::Super => "Super",
            Self::Meta => "Meta",
            Self::Hyper => "Hyper",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum KeyMatchMode {
    #[default]
    Logical,
    Physical,
}

#[derive(Clone, Debug)]
struct ResolvedBinding {
    action: KeyAction,
    key: gdk::Key,
    modifiers: gdk::ModifierType,
    match_mode: KeyMatchMode,
}

#[derive(Clone, Debug)]
struct RuntimeBinding {
    resolved: ResolvedBinding,
    keycodes: Vec<u32>,
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimeKeyBindings {
    bindings: Vec<RuntimeBinding>,
}

impl RuntimeKeyBindings {
    pub(crate) fn new(bindings: &KeyBindings, display: &gdk::Display) -> Self {
        Self::with_keycode_resolver(bindings, |key| {
            display.map_keyval(key).map_or_else(Vec::new, |keys| {
                let mut keycodes: Vec<u32> = keys.into_iter().map(|key| key.keycode()).collect();
                keycodes.sort_unstable();
                keycodes.dedup();
                keycodes
            })
        })
    }

    fn with_keycode_resolver(
        bindings: &KeyBindings,
        mut keycodes_for: impl FnMut(gdk::Key) -> Vec<u32>,
    ) -> Self {
        let mut runtime = Vec::new();
        for action in KeyAction::ALL {
            for binding in bindings.bindings(action) {
                let resolved = binding
                    .resolve(action)
                    .expect("runtime key bindings must be validated before use");
                let keycodes = if resolved.match_mode == KeyMatchMode::Physical {
                    keycodes_for(resolved.key)
                } else {
                    Vec::new()
                };
                runtime.push(RuntimeBinding { resolved, keycodes });
            }
        }
        Self { bindings: runtime }
    }

    pub(crate) fn action_for_event(
        &self,
        key: gdk::Key,
        keycode: u32,
        modifiers: gdk::ModifierType,
    ) -> Option<KeyAction> {
        let key = key.to_lower();
        let modifiers = relevant_modifiers(modifiers);
        self.bindings.iter().find_map(|binding| {
            let same_key = match binding.resolved.match_mode {
                KeyMatchMode::Logical => binding.resolved.key == key,
                KeyMatchMode::Physical => binding.keycodes.contains(&keycode),
            };
            (same_key && binding.resolved.modifiers == modifiers).then_some(binding.resolved.action)
        })
    }
}

pub(crate) fn normalize_key_bindings(value: &Value, defaults: &Value) -> Option<Value> {
    let user = value.as_object()?;
    let mut resolved = defaults.as_object()?.clone();
    if user.keys().any(|key| !resolved.contains_key(key)) {
        return None;
    }
    for (key, value) in user {
        resolved.insert(key.clone(), value.clone());
    }

    let value = Value::Object(resolved);
    let bindings: KeyBindings = serde_json::from_value(value.clone()).ok()?;
    bindings.validate().ok()?;
    Some(value)
}

fn validate_reserved_binding(action: KeyAction, binding: &ResolvedBinding) -> Result<(), String> {
    let control = gdk::ModifierType::CONTROL_MASK;
    let control_shift = control | gdk::ModifierType::SHIFT_MASK;
    if binding.modifiers == control && binding.key == gdk::Key::d {
        return Err("Ctrl+D is reserved for terminal end-of-input handling".to_owned());
    }
    if (binding.modifiers == control || binding.modifiers == control_shift)
        && binding.key == gdk::Key::z
    {
        return Err(
            "Ctrl+Z and Ctrl+Shift+Z are reserved for terminal suspend handling".to_owned(),
        );
    }
    if binding.modifiers == control && binding.key == gdk::Key::c && action != KeyAction::Copy {
        return Err("Ctrl+C is reserved for copy and terminal interrupt handling".to_owned());
    }
    Ok(())
}

fn relevant_modifiers(modifiers: gdk::ModifierType) -> gdk::ModifierType {
    modifiers
        & (gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SHIFT_MASK
            | gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::SUPER_MASK
            | gdk::ModifierType::META_MASK
            | gdk::ModifierType::HYPER_MASK)
}

fn display_key_name(key: &str) -> String {
    match key {
        "equal" => "=".to_owned(),
        "minus" => "-".to_owned(),
        "Page_Up" => "PageUp".to_owned(),
        "Page_Down" => "PageDown".to_owned(),
        value if value.chars().count() == 1 => value.to_uppercase(),
        value => value.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bindings(value: Value) -> KeyBindings {
        serde_json::from_value(value).unwrap()
    }

    fn empty_bindings() -> Value {
        serde_json::json!({
            "new_tab": [],
            "previous_tab": [],
            "next_tab": [],
            "copy": [],
            "paste": [],
            "zoom_in": [],
            "zoom_out": [],
            "zoom_reset": []
        })
    }

    #[test]
    fn logical_and_physical_bindings_match_their_requested_inputs() {
        let mut value = empty_bindings();
        value["new_tab"] = serde_json::json!([{
            "key": "t",
            "modifiers": ["control"]
        }, {
            "key": "n",
            "modifiers": ["control", "shift"]
        }]);
        value["copy"] = serde_json::json!([{
            "key": "c",
            "modifiers": ["control"],
            "match": "physical"
        }]);
        let bindings = bindings(value);
        let runtime = RuntimeKeyBindings::with_keycode_resolver(&bindings, |key| {
            if key == gdk::Key::c { vec![54] } else { vec![] }
        });
        let control = gdk::ModifierType::CONTROL_MASK;

        assert_eq!(
            runtime.action_for_event(gdk::Key::t, 28, control),
            Some(KeyAction::NewTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::T, 28, control),
            Some(KeyAction::NewTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::t, 28, control | gdk::ModifierType::SHIFT_MASK),
            None
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::t, 28, control | gdk::ModifierType::ALT_MASK),
            None
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::n, 57, control | gdk::ModifierType::SHIFT_MASK),
            Some(KeyAction::NewTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::Thai_saraae, 54, control),
            Some(KeyAction::Copy)
        );
        assert_eq!(
            runtime.action_for_event(
                gdk::Key::Thai_saraae,
                54,
                control | gdk::ModifierType::SHIFT_MASK
            ),
            None
        );
        assert_eq!(runtime.action_for_event(gdk::Key::c, 55, control), None);
    }

    #[test]
    fn empty_binding_list_disables_an_action() {
        let bindings = bindings(empty_bindings());
        let runtime = RuntimeKeyBindings::with_keycode_resolver(&bindings, |_| vec![]);

        assert_eq!(
            runtime.action_for_event(gdk::Key::t, 28, gdk::ModifierType::CONTROL_MASK),
            None
        );
    }

    #[test]
    fn missing_actions_are_filled_from_defaults() {
        let defaults = empty_bindings();
        let user = serde_json::json!({
            "new_tab": [{"key": "n", "modifiers": ["control"]}]
        });

        let normalized = normalize_key_bindings(&user, &defaults).unwrap();

        assert_eq!(normalized["new_tab"][0]["key"], "n");
        assert_eq!(normalized["copy"], serde_json::json!([]));
    }

    #[test]
    fn conflicting_bindings_are_rejected() {
        let mut value = empty_bindings();
        value["new_tab"] = serde_json::json!([{"key": "t", "modifiers": ["control"]}]);
        value["next_tab"] = serde_json::json!([{"key": "T", "modifiers": ["control"]}]);

        let error = bindings(value).validate().unwrap_err();

        assert!(error.contains("next_tab conflicts with new_tab"));
    }

    #[test]
    fn terminal_control_bindings_are_reserved() {
        let mut value = empty_bindings();
        value["new_tab"] = serde_json::json!([{"key": "d", "modifiers": ["control"]}]);

        assert!(bindings(value).validate().is_err());
    }

    #[test]
    fn labels_use_compact_modifier_and_key_names() {
        let binding: KeyBinding = serde_json::from_value(serde_json::json!({
            "key": "Page_Up",
            "modifiers": ["control", "shift"]
        }))
        .unwrap();

        assert_eq!(binding.label(), "Ctrl+Shift+PageUp");

        let copy: KeyBinding = serde_json::from_value(serde_json::json!({
            "key": "c",
            "modifiers": ["control"]
        }))
        .unwrap();
        assert_eq!(copy.label(), "Ctrl+C");
    }

    #[test]
    fn project_defaults_preserve_existing_shortcuts() {
        let project: Value = serde_json::from_str(include_str!("../config/settings.json")).unwrap();
        let bindings: KeyBindings =
            serde_json::from_value(project["key_bindings"].clone()).unwrap();
        bindings.validate().unwrap();
        let runtime = RuntimeKeyBindings::with_keycode_resolver(&bindings, |key| match key {
            gdk::Key::c => vec![54],
            gdk::Key::v => vec![55],
            _ => Vec::new(),
        });
        let control = gdk::ModifierType::CONTROL_MASK;
        let super_key = gdk::ModifierType::SUPER_MASK;

        assert_eq!(
            runtime.action_for_event(gdk::Key::t, 28, control),
            Some(KeyAction::NewTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::Page_Up, 112, control),
            Some(KeyAction::PreviousTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::h, 43, super_key),
            Some(KeyAction::PreviousTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::H, 43, super_key),
            Some(KeyAction::PreviousTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::Page_Down, 117, control),
            Some(KeyAction::NextTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::l, 46, super_key),
            Some(KeyAction::NextTab)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::h, 43, super_key | control),
            None
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::Thai_saraae, 54, control),
            Some(KeyAction::Copy)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::Thai_oang, 55, control),
            Some(KeyAction::Paste)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::equal, 21, control),
            Some(KeyAction::ZoomIn)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::minus, 20, control),
            Some(KeyAction::ZoomOut)
        );
        assert_eq!(
            runtime.action_for_event(gdk::Key::_0, 19, control),
            Some(KeyAction::ZoomReset)
        );
    }
}
