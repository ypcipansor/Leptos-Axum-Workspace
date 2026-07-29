#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppRoute {
    Dashboard,
    Login,
    Register,
}

impl AppRoute {
    pub const fn path(self) -> &'static str {
        match self {
            Self::Dashboard => "/",
            Self::Login => "/login",
            Self::Register => "/register",
        }
    }

    pub const fn template_text(self) -> &'static str {
        match self {
            Self::Dashboard => "Sistem Informasi Manajemen - Dashboard",
            Self::Login => "Sistem Informasi Manajemen - Login",
            Self::Register => "Sistem Informasi Manajemen - Register",
        }
    }
}
