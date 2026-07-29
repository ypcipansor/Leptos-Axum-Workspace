#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppRoute {
    Dashboard,
}

impl AppRoute {
    pub const fn path(self) -> &'static str {
        match self {
            Self::Dashboard => "/",
        }
    }

    pub const fn template_text(self) -> &'static str {
        match self {
            Self::Dashboard => "Template sistem informasi manajemen sederhana siap pakai.",
        }
    }
}
