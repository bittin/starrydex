// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crate::core::api::Api;
use crate::core::image_cache::ImageCache;
use crate::fl;
use crate::utils::{capitalize_string, scale_numbers};
use cosmic::app::{Command, Core};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Pixels};
use cosmic::iced_core::text::LineHeight;
use cosmic::iced_widget::Column;
use cosmic::widget::{self, menu};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use rustemon::model::pokemon::{LocationAreaEncounter, Pokemon, PokemonStat, PokemonType};

const REPOSITORY: &str = "https://github.com/mariinkys/starrydex";
const POKEMON_PER_ROW: usize = 3;

pub struct StarryDex {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Api and Client
    api: Api,
    /// Currently selected Page
    current_page: Page,
    /// Page Status
    page_status: PageStatus,
    /// Settings Status
    settings_status: SettingsStatus,
    /// Contains the list of all Pokémon
    pokemon_list: Vec<CustomPokemon>,
    /// Contains the list of pokemon after searching
    filtered_pokemon_list: Vec<CustomPokemon>,
    /// Currently viewing Pokémon
    selected_pokemon: Option<CustomPokemon>,
    /// Holds the search input value
    search: String,
    /// Wants the pokemon details in the pokémon page?
    wants_pokemon_details: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    LaunchUrl(String),
    ToggleContextPage(ContextPage),
    Search(String),
    TogglePokemonDetails(bool),

    LoadPokemon(String),
    FixAllImages,
    DownloadAllImages,

    FirstRunSetupCompleted,
    LoadedPokemon(CustomPokemon),
    LoadedPokemonList(Vec<CustomPokemon>),
    DownloadedAllImages,
    AllImagesFixed,
}

/// Identifies a page in the application.
pub enum Page {
    LandingPage,
}

/// Identifies the status of a page in the application.
pub enum PageStatus {
    FirstRun,
    Loaded,
    Loading,
}

/// Identifies the status the settings context page in the application.
pub enum SettingsStatus {
    NotDownloading,
    Downloading,
}

/// Identifies a context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
    Settings,
    PokemonPage,
}

impl ContextPage {
    fn title(&self) -> String {
        match self {
            Self::About => fl!("about"),
            Self::Settings => fl!("settings"),
            Self::PokemonPage => fl!("pokemon-page"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
    Settings,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
            MenuAction::Settings => Message::ToggleContextPage(ContextPage::Settings),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CustomPokemon {
    pub pokemon: Pokemon,
    pub sprite_path: Option<String>,
    pub encounter_info: Option<Vec<LocationAreaEncounter>>,
}

impl Application for StarryDex {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "dev.mariinkys.StarryDex";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut commands = vec![];

        let mut app = StarryDex {
            core,
            context_page: ContextPage::default(),
            key_binds: HashMap::new(),
            api: Api::new(Self::APP_ID),
            current_page: Page::LandingPage,
            pokemon_list: Vec::<CustomPokemon>::new(),
            filtered_pokemon_list: Vec::<CustomPokemon>::new(),
            selected_pokemon: None,
            page_status: PageStatus::Loading,
            search: String::new(),
            settings_status: SettingsStatus::NotDownloading,
            wants_pokemon_details: false,
        };
        commands.push(app.update_titles());

        let api_clone = app.api.clone();

        let app_data_dir = dirs::data_dir().unwrap().join(Self::APP_ID);
        std::fs::create_dir_all(&app_data_dir).expect("Failed to create the app data directory");

        let first_run_file = app_data_dir.join("first_run.txt");
        if !first_run_file.exists() {
            let _file =
                std::fs::File::create(&first_run_file).expect("Failed to create first_run.txt");

            app.page_status = PageStatus::FirstRun;
            commands.push(cosmic::app::Command::perform(
                async move { api_clone.download_all_pokemon_sprites().await },
                |_| cosmic::app::message::app(Message::FirstRunSetupCompleted),
            ));
        } else {
            commands.push(cosmic::app::Command::perform(
                async move { api_clone.load_all_pokemon().await },
                |pokemon_list| cosmic::app::message::app(Message::LoadedPokemonList(pokemon_list)),
            ));
        }

        (app, Command::batch(commands))
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")),
            menu::items(
                &self.key_binds,
                vec![
                    menu::Item::Button(fl!("about"), MenuAction::About),
                    menu::Item::Button(fl!("settings"), MenuAction::Settings),
                ],
            ),
        )]);

        vec![menu_bar.into()]
    }

    fn view(&self) -> Element<Self::Message> {
        let content = match self.current_page {
            Page::LandingPage => self.landing(),
        };

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::LaunchUrl(url) => {
                let _result = open::that_detached(url);
            }
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }

                // Set the title of the context drawer.
                self.set_context_title(context_page.title());
            }
            Message::LoadedPokemonList(pokemons) => {
                self.pokemon_list = pokemons.clone();
                self.filtered_pokemon_list = pokemons;
                self.page_status = PageStatus::Loaded;
            }
            Message::LoadedPokemon(pokemon) => {
                self.selected_pokemon = Some(pokemon);

                if self.context_page == ContextPage::PokemonPage {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = ContextPage::PokemonPage;
                    self.core.window.show_context = true;
                }

                // Set the title of the context drawer.
                self.set_context_title(ContextPage::PokemonPage.title());
            }
            Message::LoadPokemon(pokemon_name) => {
                let api_clone = self.api.clone();

                return cosmic::app::Command::perform(
                    async move { api_clone.load_pokemon(pokemon_name).await },
                    |pokemon| cosmic::app::message::app(Message::LoadedPokemon(pokemon)),
                );
            }
            Message::Search(new_value) => {
                self.search = new_value;
                self.filtered_pokemon_list = self
                    .pokemon_list
                    .clone()
                    .into_iter()
                    .filter(|pokemon| {
                        pokemon
                            .pokemon
                            .name
                            .to_lowercase()
                            .contains(&self.search.to_lowercase())
                    })
                    .collect();
            }
            Message::DownloadAllImages => {
                let api_clone = self.api.clone();

                self.settings_status = SettingsStatus::Downloading;

                return cosmic::app::Command::perform(
                    async move { api_clone.download_all_pokemon_sprites().await },
                    |_| cosmic::app::message::app(Message::DownloadedAllImages),
                );
            }
            Message::FixAllImages => {
                let api_clone = self.api.clone();

                self.settings_status = SettingsStatus::Downloading;

                return cosmic::app::Command::perform(
                    async move { api_clone.fix_all_sprites().await },
                    |_res| cosmic::app::message::app(Message::AllImagesFixed),
                );
            }
            Message::DownloadedAllImages => {
                let api_clone = self.api.clone();

                self.settings_status = SettingsStatus::NotDownloading;
                self.page_status = PageStatus::Loading;

                return cosmic::app::Command::perform(
                    async move { api_clone.load_all_pokemon().await },
                    |pokemon_list| {
                        cosmic::app::message::app(Message::LoadedPokemonList(pokemon_list))
                    },
                );
            }
            Message::AllImagesFixed => {
                let api_clone = self.api.clone();

                self.settings_status = SettingsStatus::NotDownloading;
                self.page_status = PageStatus::Loading;

                return cosmic::app::Command::perform(
                    async move { api_clone.load_all_pokemon().await },
                    |pokemon_list| {
                        cosmic::app::message::app(Message::LoadedPokemonList(pokemon_list))
                    },
                );
            }
            Message::FirstRunSetupCompleted => {
                let api_clone = self.api.clone();

                self.page_status = PageStatus::Loading;

                return cosmic::app::Command::perform(
                    async move { api_clone.load_all_pokemon().await },
                    |pokemon_list| {
                        cosmic::app::message::app(Message::LoadedPokemonList(pokemon_list))
                    },
                );
            }
            Message::TogglePokemonDetails(value) => self.wants_pokemon_details = value,
        }
        Command::none()
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<Element<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => self.about(),
            ContextPage::Settings => self.settings(),
            ContextPage::PokemonPage => self.pokemon_page(),
        })
    }
}

impl StarryDex {
    /// The about page for this app.
    pub fn about(&self) -> Element<Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(
            &include_bytes!("../res/icons/hicolor/128x128/apps/dev.mariinkys.StarryDex.svg")[..],
        ));

        let title = widget::text::title3(fl!("app-title"));

        let app_info = widget::text::text(fl!("app-info"));

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::LaunchUrl(REPOSITORY.to_string()))
            .padding(0);

        let pokeapi_text = widget::text::text(fl!("pokeapi-text"));

        let nintendo_text = widget::text::text(fl!("nintendo-text"));

        widget::column()
            .push(icon)
            .push(title)
            .push(app_info)
            .push(link)
            .push(pokeapi_text)
            .push(nintendo_text)
            .align_items(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    pub fn settings(&self) -> Element<Message> {
        let spacing = theme::active().cosmic().spacing;

        let download_row = widget::Row::new()
            .push(
                widget::column()
                    .push(widget::text::text(fl!("download-all-title")))
                    .push(widget::text::text(fl!("download-all-info")).size(10.0))
                    .width(Length::Fill),
            )
            .push(match self.settings_status {
                SettingsStatus::NotDownloading => {
                    widget::button(widget::text::text(fl!("download-button-text")))
                        .on_press(Message::DownloadAllImages)
                        .style(theme::Button::Suggested)
                        .width(Length::Shrink)
                }
                SettingsStatus::Downloading => {
                    widget::button(widget::text::text(fl!("download-button-text")))
                        .style(theme::Button::Suggested)
                        .width(Length::Shrink)
                }
            })
            .spacing(spacing.space_xxs)
            .padding([0, 5, 0, 5]);

        let fix_row = widget::Row::new()
            .push(
                widget::column()
                    .push(widget::text::text(fl!("fix-all-title")))
                    .push(widget::text::text(fl!("fix-all-info")).size(10.0))
                    .width(Length::Fill),
            )
            .push(match self.settings_status {
                SettingsStatus::NotDownloading => {
                    widget::button(widget::text::text(fl!("fix-button-text")))
                        .on_press(Message::FixAllImages)
                        .style(theme::Button::Destructive)
                        .width(Length::Shrink)
                }
                SettingsStatus::Downloading => {
                    widget::button(widget::text::text(fl!("fix-button-text")))
                        .style(theme::Button::Destructive)
                        .width(Length::Shrink)
                }
            })
            .spacing(spacing.space_xxs)
            .padding([0, 5, 0, 5]);

        match self.settings_status {
            SettingsStatus::NotDownloading => {
                widget::settings::view_column(vec![widget::settings::view_section(fl!("settings"))
                    .add(download_row)
                    .add(fix_row)
                    .into()])
                .into()
            }
            SettingsStatus::Downloading => {
                widget::settings::view_column(vec![widget::settings::view_section(fl!("settings"))
                    .add(download_row)
                    .add(fix_row)
                    .add(
                        widget::row()
                            .push(
                                widget::text(fl!("downloading-text"))
                                    .width(Length::Fill)
                                    .horizontal_alignment(Horizontal::Center),
                            )
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .into()])
                .into()
            }
        }
    }

    pub fn landing(&self) -> Element<Message> {
        let space_s = theme::active().cosmic().spacing.space_s;
        let spacing = theme::active().cosmic().spacing;

        match self.page_status {
            PageStatus::Loaded => {
                let mut pokemon_grid = widget::Grid::new().width(Length::Fill);

                for (index, pokemon) in self.filtered_pokemon_list.iter().enumerate() {
                    let pokemon_image = if let Some(path) = &pokemon.sprite_path {
                        widget::Image::new(path)
                            .content_fit(cosmic::iced::ContentFit::None)
                            .width(Length::Fixed(100.0))
                            .height(Length::Fixed(100.0))
                    } else {
                        widget::Image::new(ImageCache::get("fallback"))
                            .content_fit(cosmic::iced::ContentFit::None)
                            .width(Length::Fixed(100.0))
                            .height(Length::Fixed(100.0))
                    };

                    let pokemon_container = widget::button(
                        widget::Column::new()
                            .push(pokemon_image.width(Length::Shrink))
                            .push(
                                widget::text::text(capitalize_string(&pokemon.pokemon.name))
                                    .width(Length::Shrink)
                                    .line_height(LineHeight::Absolute(Pixels::from(15.0))),
                            )
                            .width(Length::Fill)
                            .align_items(Alignment::Center),
                    )
                    .width(Length::Fixed(200.0))
                    .height(Length::Fixed(135.0))
                    .on_press_down(Message::LoadPokemon(pokemon.pokemon.name.to_string()))
                    .style(theme::Button::Image)
                    .padding([spacing.space_none, spacing.space_s]);

                    if index % POKEMON_PER_ROW == 0 {
                        pokemon_grid = pokemon_grid.insert_row();
                    }

                    pokemon_grid = pokemon_grid.push(pokemon_container);
                }

                let search = widget::search_input(fl!("search"), &self.search)
                    .style(theme::TextInput::Search)
                    .on_input(Message::Search)
                    .on_clear(Message::Search(String::new()))
                    .width(Length::Fill);

                let search_row = widget::Row::new()
                    .push(search)
                    .width(Length::Fill)
                    .padding(5.0);

                widget::Column::new()
                    .push(search_row)
                    .push(
                        widget::scrollable(
                            widget::Container::new(pokemon_grid).align_x(Horizontal::Center),
                        )
                        .width(Length::Fill),
                    )
                    .width(Length::Fill)
                    .spacing(5.0)
                    .into()
            }
            PageStatus::Loading => Column::new()
                .push(widget::text::text(fl!("loading")))
                .align_items(Alignment::Center)
                .width(Length::Fill)
                .spacing(space_s)
                .into(),
            PageStatus::FirstRun => Column::new()
                .push(widget::text::text(fl!("downloading-sprites")))
                .push(widget::text::text(fl!("estimate")))
                .push(widget::text::text(fl!("once-message")))
                .align_items(Alignment::Center)
                .width(Length::Fill)
                .into(),
        }
    }

    pub fn pokemon_page(&self) -> Element<Message> {
        let spacing = theme::active().cosmic().spacing;

        let content: widget::Column<_> = match &self.selected_pokemon {
            Some(custom_pokemon) => {
                let page_title =
                    widget::text::title1(capitalize_string(custom_pokemon.pokemon.name.as_str()))
                        .width(Length::Fill)
                        .horizontal_alignment(Horizontal::Center);

                let pokemon_image = if let Some(path) = &custom_pokemon.sprite_path {
                    widget::Image::new(path).content_fit(cosmic::iced::ContentFit::Fill)
                } else {
                    widget::Image::new(ImageCache::get("fallback"))
                        .content_fit(cosmic::iced::ContentFit::Fill)
                };

                let pokemon_weight = widget::container::Container::new(
                    widget::Column::new()
                        .push(widget::text::title3(fl!("weight")))
                        .push(
                            widget::text::text(format!(
                                "{} Kg",
                                scale_numbers(custom_pokemon.pokemon.weight).to_string()
                            ))
                            .size(15.0),
                        )
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                )
                .style(theme::Container::ContextDrawer)
                .padding([spacing.space_none, spacing.space_xxs]);

                let pokemon_height = widget::container::Container::new(
                    widget::Column::new()
                        .push(widget::text::title3(fl!("height")))
                        .push(
                            widget::text::text(format!(
                                "{} m",
                                scale_numbers(custom_pokemon.pokemon.height).to_string()
                            ))
                            .size(15.0),
                        )
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                )
                .style(theme::Container::ContextDrawer)
                .padding([spacing.space_none, spacing.space_xxs]);

                let parsed_pokemon_types =
                    self.parse_pokemon_types(&custom_pokemon.pokemon.types, &spacing);

                let pokemon_first_row = widget::Row::new()
                    .push(pokemon_weight)
                    .push(pokemon_height)
                    .push(parsed_pokemon_types)
                    .spacing(8.0)
                    .align_items(Alignment::Center);

                let parsed_pokemon_stats =
                    self.parse_pokemon_stats(&custom_pokemon.pokemon.stats, &spacing);

                let show_details = widget::Checkbox::new(
                    "Show Encounter Details",
                    self.wants_pokemon_details,
                    Message::TogglePokemonDetails,
                );

                let encounter_info = match &custom_pokemon.encounter_info {
                    Some(info) => self.parse_encounter_info(info, &spacing),
                    None => widget::Container::new(widget::Text::new("No encounter info."))
                        .style(theme::Container::ContextDrawer)
                        .into(),
                };

                let mut result_col = widget::Column::new()
                    .push(page_title)
                    .push(pokemon_image)
                    .push(pokemon_first_row)
                    .push(parsed_pokemon_stats)
                    // .push(show_details)
                    // .push(encounter_info)
                    .align_items(Alignment::Center)
                    .spacing(10.0);

                if custom_pokemon.encounter_info.is_some() {
                    if custom_pokemon.encounter_info.clone().unwrap().is_empty() == false {
                        result_col = result_col.push(show_details);
                        if self.wants_pokemon_details {
                            result_col = result_col.push(encounter_info);
                        }
                    }
                }

                return result_col.into();
            }
            None => {
                let error = widget::text::title1(fl!("generic-error"))
                    .apply(widget::container)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center);

                widget::Column::new().push(error).into()
            }
        };

        widget::container(content).into()
    }

    /// Updates the header and window titles.
    pub fn update_titles(&mut self) -> Command<Message> {
        let mut window_title = fl!("app-title");
        let mut header_title = String::new();

        match self.current_page {
            Page::LandingPage => {
                window_title.push_str(" — ");
                window_title.push_str(fl!("landing-page-title").as_str());
                header_title.push_str(fl!("landing-page-title").as_str());
            }
        }

        self.set_header_title(header_title);
        self.set_window_title(window_title)
    }

    pub fn parse_pokemon_stats(
        &self,
        stats: &Vec<PokemonStat>,
        spacing: &cosmic_theme::Spacing,
    ) -> Element<Message> {
        let children = stats.iter().map(|pokemon_stats| {
            widget::Row::new()
                .push(widget::text(capitalize_string(&pokemon_stats.stat.name)).width(Length::Fill))
                .push(
                    widget::text(pokemon_stats.base_stat.to_string())
                        .horizontal_alignment(Horizontal::Left),
                )
                .into()
        });

        widget::container::Container::new(Column::with_children(children))
            .style(theme::Container::ContextDrawer)
            .padding([spacing.space_none, spacing.space_xxs])
            .into()
    }

    pub fn parse_pokemon_types(
        &self,
        types: &Vec<PokemonType>,
        spacing: &cosmic_theme::Spacing,
    ) -> Element<Message> {
        let children = types.iter().map(|pokemon_types| {
            widget::Row::new()
                .push(
                    widget::text(pokemon_types.type_.name.to_uppercase())
                        .width(Length::Fill)
                        .horizontal_alignment(Horizontal::Center),
                )
                .width(Length::Fill)
                .into()
        });

        widget::container::Container::new(Column::with_children(children))
            .style(theme::Container::ContextDrawer)
            .padding([spacing.space_none, spacing.space_xxs])
            .into()
    }

    pub fn parse_encounter_info(
        &self,
        encounter_info: &Vec<LocationAreaEncounter>,
        spacing: &cosmic_theme::Spacing,
    ) -> Element<Message> {
        let children = encounter_info.iter().map(|encounter_info| {
            let mut version_column = widget::Column::new().width(Length::Fill);
            version_column = version_column.push(
                widget::text(capitalize_string(&encounter_info.location_area.name))
                    .style(theme::Text::Accent)
                    .size(Pixels::from(15)),
            );

            for games_info in &encounter_info.version_details {
                let game_name = capitalize_string(&games_info.version.name);
                let mut method_name = String::new();
                // let mut conditions = String::new();

                for enc_details in &games_info.encounter_details {
                    method_name = capitalize_string(&enc_details.method.name);

                    // for condition in &enc_details.condition_values {
                    //     conditions = conditions + &capitalize_string(&condition.name);
                    // }
                }

                version_column =
                    version_column.push(widget::text(format!("{}: {}", game_name, method_name)))
            }

            version_column.into()
        });

        widget::container::Container::new(Column::with_children(children))
            .style(theme::Container::ContextDrawer)
            .padding([spacing.space_none, spacing.space_xxs])
            .into()
    }
}
