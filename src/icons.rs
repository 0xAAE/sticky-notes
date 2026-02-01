use cosmic::widget::icon::Handle;
// embedded SVG bytes
#[cfg(not(feature = "xdg_icons"))]
pub mod inner {
    use cosmic::widget::icon::{self, Handle};

    const ICON_NOTES: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/notes.svg");
    const ICON_UNLOCKED: &[u8] =
        include_bytes!("../resources/icons/mono/scalable/changes-allow-symbolic.svg");
    const ICON_LOCKED: &[u8] =
        include_bytes!("../resources/icons/mono/scalable/changes-prevent-symbolic.svg");
    const ICON_NEW: &[u8] =
        include_bytes!("../resources/icons/mono/scalable/document-new-symbolic.svg");
    const ICON_DELETE: &[u8] =
        include_bytes!("../resources/icons/mono/scalable/edit-delete-symbolic.svg");
    const ICON_EDIT: &[u8] = include_bytes!("../resources/icons/mono/scalable/edit-symbolic.svg");
    const ICON_DOWN: &[u8] =
        include_bytes!("../resources/icons/mono/scalable/pan-down-symbolic.svg");
    const ICON_UNDO: &[u8] =
        include_bytes!("../resources/icons/mono/scalable/edit-undo-symbolic.svg");
    const ICON_CHECKED: &[u8] =
        include_bytes!("../resources/icons/mono/scalable/checkbox-checked-symbolic.svg");

    pub struct IconSet {
        pub notes: Handle,
        pub lock: Handle,
        pub unlock: Handle,
        pub edit: Handle,
        pub down: Handle,
        pub create: Handle,
        pub delete: Handle,
        pub undo: Handle,
        pub checked: Handle,
    }

    impl IconSet {
        pub fn new() -> Self {
            Self {
                notes: icon::from_svg_bytes(ICON_NOTES),
                lock: icon::from_svg_bytes(ICON_UNLOCKED),
                unlock: icon::from_svg_bytes(ICON_LOCKED),
                edit: icon::from_svg_bytes(ICON_EDIT),
                down: icon::from_svg_bytes(ICON_DOWN),
                create: icon::from_svg_bytes(ICON_NEW),
                delete: icon::from_svg_bytes(ICON_DELETE),
                undo: icon::from_svg_bytes(ICON_UNDO),
                checked: icon::from_svg_bytes(ICON_CHECKED),
            }
        }
    }
}

// system wide installed icons
#[cfg(feature = "xdg_icons")]
mod inner {
    use cosmic::widget::icon::{self, Handle};

    const ICON_NOTES: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/notes.svg");

    pub const XDG_UNLOCKED: &str = "changes-allow-symbolic";
    pub const XDG_LOCKED: &str = "changes-prevent-symbolic";
    pub const XDG_NEW: &str = "document-new-symbolic";
    pub const XDG_DELETE: &str = "edit-delete-symbolic";
    pub const XDG_EDIT: &str = "edit-symbolic";
    pub const XDG_DOWN: &str = "pan-down-symbolic";
    pub const XDG_UNDO: &str = "edit-undo-symbolic";
    pub const XDG_CHECKED: &str = "checkbox-checked-symbolic";

    pub struct IconSet {
        pub notes: Handle,
        pub lock: Handle,
        pub unlock: Handle,
        pub edit: Handle,
        pub down: Handle,
        pub create: Handle,
        pub delete: Handle,
        pub undo: Handle,
        pub checked: Handle,
    }

    impl IconSet {
        pub fn new() -> Self {
            Self {
                notes: icon::from_svg_bytes(ICON_NOTES),
                lock: icon::from_name(XDG_UNLOCKED).into(),
                unlock: icon::from_name(XDG_LOCKED).into(),
                edit: icon::from_name(XDG_EDIT).into(),
                down: icon::from_name(XDG_DOWN).into(),
                create: icon::from_name(XDG_NEW).into(),
                delete: icon::from_name(XDG_DELETE).into(),
                undo: icon::from_name(XDG_UNDO).into(),
                checked: icon::from_name(XDG_CHECKED).into(),
            }
        }
    }
}

pub struct IconSet {
    inner: inner::IconSet,
}

impl Default for IconSet {
    fn default() -> Self {
        Self::new()
    }
}

impl IconSet {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: inner::IconSet::new(),
        }
    }

    pub fn notes(&self) -> Handle {
        self.inner.notes.clone()
    }

    pub fn lock(&self) -> Handle {
        self.inner.lock.clone()
    }

    pub fn unlock(&self) -> Handle {
        self.inner.unlock.clone()
    }

    pub fn edit(&self) -> Handle {
        self.inner.edit.clone()
    }

    pub fn down(&self) -> Handle {
        self.inner.down.clone()
    }

    pub fn create(&self) -> Handle {
        self.inner.create.clone()
    }

    pub fn delete(&self) -> Handle {
        self.inner.delete.clone()
    }

    pub fn undo(&self) -> Handle {
        self.inner.undo.clone()
    }

    pub fn checked(&self) -> Handle {
        self.inner.checked.clone()
    }
}
