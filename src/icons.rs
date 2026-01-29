// embedded SVG bytes
#[cfg(not(feature = "xdg_icons"))]
pub mod inner {
    use cosmic::widget::icon::{self, Handle};

    const ICON_NOTES: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/notes.svg");
    const ICON_UNLOCKED: &[u8] =
        include_bytes!("../resources/icons/hicolor/scalable/changes-allow-symbolic.svg");
    const ICON_LOCKED: &[u8] =
        include_bytes!("../resources/icons/hicolor/scalable/changes-prevent-symbolic.svg");
    const ICON_NEW: &[u8] =
        include_bytes!("../resources/icons/hicolor/scalable/document-new-symbolic.svg");
    const ICON_DELETE: &[u8] =
        include_bytes!("../resources/icons/hicolor/scalable/edit-delete-symbolic.svg");
    const ICON_EDIT: &[u8] =
        include_bytes!("../resources/icons/hicolor/scalable/edit-symbolic.svg");
    const ICON_DOWN: &[u8] =
        include_bytes!("../resources/icons/hicolor/scalable/pan-down-symbolic.svg");
    const ICON_UNDO: &[u8] =
        include_bytes!("../resources/icons/hicolor/scalable/edit-undo-symbolic.svg");

    pub struct IconSet {
        notes: Handle,
        lock: Handle,
        unlock: Handle,
        edit: Handle,
        down: Handle,
        create: Handle,
        delete: Handle,
        undo: Handle,
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
            }
        }

        pub fn notes(&self) -> Handle {
            self.notes.clone()
        }

        pub fn lock(&self) -> Handle {
            self.lock.clone()
        }

        pub fn unlock(&self) -> Handle {
            self.unlock.clone()
        }

        pub fn edit(&self) -> Handle {
            self.edit.clone()
        }

        pub fn down(&self) -> Handle {
            self.down.clone()
        }

        pub fn create(&self) -> Handle {
            self.create.clone()
        }

        pub fn delete(&self) -> Handle {
            self.delete.clone()
        }

        pub fn undo(&self) -> Handle {
            self.undo.clone()
        }
    }
}

// system wide installed icons
#[cfg(feature = "xdg_icons")]
mod inner {
    use cosmic::widget::icon::{self, Handle, Named};

    const ICON_NOTES: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/notes.svg");

    pub const XDG_UNLOCKED: &str = "changes-allow-symbolic";
    pub const XDG_LOCKED: &str = "changes-prevent-symbolic";
    pub const XDG_NEW: &str = "document-new-symbolic";
    pub const XDG_DELETE: &str = "edit-delete-symbolic";
    pub const XDG_EDIT: &str = "edit-symbolic";
    pub const XDG_DOWN: &str = "pan-down-symbolic";
    pub const XDG_UNDO: &str = "edit-undo-symbolic";

    pub struct IconSet {
        notes: Handle,
        lock: Named,
        unlock: Named,
        edit: Named,
        down: Named,
        create: Named,
        delete: Named,
        undo: Named,
    }

    impl IconSet {
        pub fn new() -> Self {
            Self {
                notes: icon::from_svg_bytes(ICON_NOTES),
                lock: icon::from_name(XDG_UNLOCKED),
                unlock: icon::from_name(XDG_LOCKED),
                edit: icon::from_name(XDG_EDIT),
                down: icon::from_name(XDG_DOWN),
                create: icon::from_name(XDG_NEW),
                delete: icon::from_name(XDG_DELETE),
                undo: icon::from_name(XDG_UNDO),
            }
        }

        pub fn notes(&self) -> Handle {
            self.notes.clone()
        }

        pub fn lock(&self) -> Handle {
            self.lock.clone().into()
        }

        pub fn unlock(&self) -> Handle {
            self.unlock.clone().into()
        }

        pub fn edit(&self) -> Handle {
            self.edit.clone().into()
        }

        pub fn down(&self) -> Handle {
            self.down.clone().into()
        }

        pub fn create(&self) -> Handle {
            self.create.clone().into()
        }

        pub fn delete(&self) -> Handle {
            self.delete.clone().into()
        }

        pub fn undo(&self) -> Handle {
            self.undo.clone().into()
        }
    }
}

pub use inner::IconSet;
