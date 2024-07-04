use cosmic::{
    app::Settings,
    iced::{Limits, Size},
};

pub fn init() -> Settings {
    let settings = get_app_settings();
    settings
}

pub fn get_app_settings() -> Settings {
    let mut settings = Settings::default();

    settings = settings.size_limits(Limits::NONE.min_width(800.0).min_height(300.0));
    settings = settings.size(Size::new(1200.0, 800.0));
    settings = settings.debug(false);
    settings
}
