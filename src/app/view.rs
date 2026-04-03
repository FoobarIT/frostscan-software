use iced::{
    Background, Border, Color, Element, Length,
    widget::{Space, button, column, container, pick_list, progress_bar, row, scrollable, text},
};

use crate::app::message::Message;
use crate::app::notice::NoticeLevel;
use crate::app::state::{FrostScanApp, SortBy, Tab};
use crate::i18n::{Locale, TextKey};
use crate::model::extension_summary::aggregate_extensions;
use crate::model::file_entry::{FileEntry, FileEntryKind};
use crate::ui::format::format_size;
use crate::ui::styles::*;
use crate::ui::treemap::{self, TreemapStrings, TreemapTileKind};

impl FrostScanApp {
    fn sorted_entries(&self) -> Vec<FileEntry> {
        let mut data = self.entries.clone();

        match self.sort_by {
            SortBy::Name => {
                data.sort_by_key(|e| (sort_group(e.kind), e.name.to_lowercase()));
                if self.sort_desc {
                    data.reverse();
                    data.sort_by_key(|e| sort_group(e.kind));
                }
            }
            SortBy::Size => {
                data.sort_by_key(|e| (sort_group(e.kind), e.size));
                if self.sort_desc {
                    data.reverse();
                    data.sort_by_key(|e| sort_group(e.kind));
                }
            }
        }

        data
    }

    fn view_welcome(&self) -> Element<'_, Message> {
        let selector = column![
            text(self.tr(TextKey::DiskSelectorTitle))
                .size(22)
                .color(Color::from_rgb8(230, 230, 235)),
            text(self.tr(TextKey::DiskSelectorDescription))
                .size(14)
                .color(Color::from_rgb8(150, 150, 160)),
            pick_list(
                self.disks.clone(),
                self.selected_disk.clone(),
                Message::SelectDisk
            )
            .placeholder(self.tr(TextKey::DiskSelectorSelect)),
            button(
                text(self.tr(TextKey::DiskSelectorButton))
                    .size(15)
                    .color(Color::WHITE)
            )
            .padding([10, 16])
            .style(primary_button_style)
            .on_press(Message::OpenSelectedDisk),
        ]
        .spacing(12);

        let card = container(selector)
            .padding(24)
            .width(Length::Fill)
            .style(panel_style);

        container(
            column![
                text(self.tr(TextKey::Title))
                    .size(30)
                    .color(Color::from_rgb8(240, 240, 245)),
                text(self.tr(TextKey::Description))
                    .size(16)
                    .color(Color::from_rgb8(145, 145, 155)),
                Space::new().height(20),
                card
            ]
            .spacing(4)
            .max_width(700),
        )
        .width(Length::Fill)
        .padding(24)
        .center_x(Length::Fill)
        .into()
    }

    fn header_topbar(&self) -> Element<'_, Message> {
        let subtitle = text(self.tr(TextKey::Description))
            .size(13)
            .color(Color::from_rgb8(145, 145, 155));

        let nav_buttons = row![
            button(text(self.tr(TextKey::ActionBtnGoBack)))
                .padding([8, 12])
                .style(secondary_button_style)
                .on_press(Message::GoUp),
            button(text(self.tr(TextKey::ActionBtnRefresh)))
                .padding([8, 12])
                .style(secondary_button_style)
                .on_press(Message::Refresh),
        ]
        .spacing(10);

        let left = column![subtitle].spacing(2);

        container(
            row![left, Space::new().width(Length::Fill), nav_buttons]
                .align_y(iced::Alignment::Center),
        )
        .padding([16, 18])
        .style(topbar_style)
        .into()
    }

    fn view_path_bar(&self) -> Element<'_, Message> {
        let path_text = self
            .current_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| self.tr(TextKey::PathViewText).to_string());

        container(
            text(path_text)
                .size(14)
                .color(Color::from_rgb8(210, 210, 218)),
        )
        .padding([12, 14])
        .width(Length::Fill)
        .style(path_bar_style)
        .into()
    }

    fn view_table_header(&self) -> Element<'_, Message> {
        let name_label = match (self.sort_by, self.sort_desc) {
            (SortBy::Name, false) => {
                format!("{} ↑", self.tr(TextKey::ExplorerTableHeaderNameRow))
            }
            (SortBy::Name, true) => {
                format!("{} ↓", self.tr(TextKey::ExplorerTableHeaderNameRow))
            }
            _ => self.tr(TextKey::ExplorerTableHeaderNameRow).to_string(),
        };

        let size_label = match (self.sort_by, self.sort_desc) {
            (SortBy::Size, false) => {
                format!("{} ↑", self.tr(TextKey::ExplorerTableHeaderSizeRow))
            }
            (SortBy::Size, true) => {
                format!("{} ↓", self.tr(TextKey::ExplorerTableHeaderSizeRow))
            }
            _ => self.tr(TextKey::ExplorerTableHeaderSizeRow).to_string(),
        };

        container(
            row![
                button(text(name_label).size(14))
                    .style(header_button_style)
                    .on_press(Message::SortByName)
                    .width(Length::Fill),
                container(
                    text(self.tr(TextKey::ExplorerTableHeaderTypeRow))
                        .size(14)
                        .color(Color::from_rgb8(170, 170, 180)),
                )
                .width(120),
                button(text(size_label).size(14))
                    .style(header_button_style)
                    .on_press(Message::SortBySize)
                    .width(120),
            ]
            .spacing(16)
            .width(Length::Fill)
            .align_y(iced::Alignment::Center),
        )
        .padding([10, 14])
        .width(Length::Fill)
        .style(table_header_style)
        .into()
    }

    fn view_table(&self) -> Element<'_, Message> {
        let rows = self.sorted_entries().into_iter().map(|entry| {
            let (icon, kind) = match entry.kind {
                FileEntryKind::Directory => ("📁", self.tr(TextKey::ExplorerTableTypeFolder)),
                FileEntryKind::File => ("📄", self.tr(TextKey::ExplorerTableTypeFile)),
                FileEntryKind::Symlink => ("🔗", self.tr(TextKey::ExplorerTableTypeSymlink)),
            };

            let size = if matches!(entry.kind, FileEntryKind::Directory) && entry.size_pending {
                self.tr(TextKey::ExplorerTableCalculatingSize).to_string()
            } else if matches!(entry.kind, FileEntryKind::Directory) && entry.size_partial {
                format!(
                    "{} {}",
                    format_size(entry.size),
                    self.tr(TextKey::ExplorerTablePartialSize)
                )
            } else {
                format_size(entry.size)
            };

            let row_content = row![
                button(
                    text(format!("{icon}  {}", entry.name))
                        .size(14)
                        .color(Color::from_rgb8(232, 232, 238))
                )
                .style(row_button_style)
                .on_press(Message::OpenEntry(entry.path.clone()))
                .width(Length::Fill),
                container(text(kind).size(13).color(Color::from_rgb8(155, 155, 165))).width(120),
                container(text(size).size(13).color(Color::from_rgb8(205, 205, 212)))
                    .width(120)
                    .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(16)
            .width(Length::Fill)
            .align_y(iced::Alignment::Center);

            container(row_content)
                .padding([9, 14])
                .width(Length::Fill)
                .style(row_container_style)
                .into()
        });

        container(
            column![
                self.view_table_header(),
                scrollable(column(rows).spacing(6)).height(Length::Fill)
            ]
            .spacing(8),
        )
        .padding(10)
        .height(Length::Fill)
        .style(panel_style)
        .into()
    }

    fn view_explorer(&self) -> Element<'_, Message> {
        let count = text(
            self.tr(TextKey::ExplorerNumberOfElements)
                .replace("{}", &self.entries.len().to_string())
                .to_string(),
        )
        .size(13)
        .color(Color::from_rgb8(145, 145, 155));

        let mut content = column![self.view_path_bar()].spacing(12);

        if let Some(notice_banner) = self.view_notice_banner() {
            content = content.push(notice_banner);
        }

        content
            .push(count)
            .push(self.view_extension_overview())
            .push(self.view_table())
            .into()
    }

    fn view_extension_overview(&self) -> Element<'_, Message> {
        let summaries = aggregate_extensions(&self.entries);
        let total_size: u64 = summaries.iter().map(|summary| summary.total_size).sum();

        let content: Element<'_, Message> = if summaries.is_empty() {
            text(self.tr(TextKey::ExplorerExtensionOverviewEmpty))
                .size(13)
                .color(Color::from_rgb8(145, 145, 155))
                .into()
        } else {
            let cards = summaries.into_iter().take(5).map(|summary| {
                let extension_label = summary
                    .extension
                    .map(|ext| format!(".{ext}"))
                    .unwrap_or_else(|| self.tr(TextKey::ExplorerExtensionNoExtension).to_string());

                let file_count = self
                    .tr(TextKey::ExplorerExtensionFilesCount)
                    .replace("{count}", &summary.file_count.to_string());
                let percentage = if total_size == 0 {
                    0.0
                } else {
                    (summary.total_size as f64 / total_size as f64) * 100.0
                };
                let percentage_label = self
                    .tr(TextKey::ExplorerExtensionPercentage)
                    .replace("{value}", &format!("{percentage:.1}"));

                container(
                    column![
                        text(extension_label)
                            .size(16)
                            .color(Color::from_rgb8(235, 235, 240)),
                        text(format_size(summary.total_size))
                            .size(14)
                            .color(Color::from_rgb8(205, 205, 212)),
                        text(file_count)
                            .size(12)
                            .color(Color::from_rgb8(145, 145, 155)),
                        text(percentage_label)
                            .size(12)
                            .color(Color::from_rgb8(145, 145, 155)),
                    ]
                    .spacing(4),
                )
                .padding([12, 14])
                .width(Length::FillPortion(1))
                .style(row_container_style)
                .into()
            });

            row(cards).spacing(10).into()
        };

        container(
            column![
                text(self.tr(TextKey::ExplorerExtensionOverviewTitle))
                    .size(16)
                    .color(Color::from_rgb8(230, 230, 235)),
                content
            ]
            .spacing(10),
        )
        .padding(14)
        .width(Length::Fill)
        .style(panel_style)
        .into()
    }

    fn view_treemap(&self) -> Element<'_, Message> {
        if self.current_path.is_none() {
            return container(
                text(self.tr(TextKey::TreeMapNoFolder))
                    .size(14)
                    .color(Color::from_rgb8(145, 145, 155)),
            )
            .padding(20)
            .style(panel_style)
            .into();
        }

        let Some(tree) = self.scan_tree.as_ref() else {
            return container(
                text(self.tr(TextKey::TreeMapEmpty))
                    .size(14)
                    .color(Color::from_rgb8(145, 145, 155)),
            )
            .padding(20)
            .style(panel_style)
            .into();
        };

        let treemap_strings = TreemapStrings {
            others_label: self.tr(TextKey::TreeMapLegendOthers).to_string(),
            folder_label: self.tr(TextKey::TreeMapHoverFolder).to_string(),
            file_label: self.tr(TextKey::TreeMapHoverFile).to_string(),
            group_label: self.tr(TextKey::TreeMapHoverGroup).to_string(),
            level_label: self.tr(TextKey::TreeMapHoverLevel).to_string(),
            partial_label: self.tr(TextKey::TreeMapHoverPartial).to_string(),
            pending_label: self.tr(TextKey::TreeMapHoverPending).to_string(),
        };
        let data = treemap::TreemapData::from_root(tree, &treemap_strings);

        let body: Element<'_, Message> = if data.is_empty() {
            text(self.tr(TextKey::TreeMapEmpty))
                .size(14)
                .color(Color::from_rgb8(145, 145, 155))
                .into()
        } else {
            treemap::view(&data, &treemap_strings)
        };

        let count = self
            .tr(TextKey::TreeMapFilesCount)
            .replace("{count}", &data.shown_count.to_string());

        let mut content = column![
            text(self.tr(TextKey::TreeMapTitle))
                .size(24)
                .color(Color::from_rgb8(240, 240, 245)),
            text(self.tr(TextKey::TreeMapDescription))
                .size(14)
                .color(Color::from_rgb8(145, 145, 155)),
            self.view_treemap_legend(),
            text(count)
                .size(13)
                .color(Color::from_rgb8(145, 145, 155)),
            text(if data.pending_nodes > 0 {
                self.tr(TextKey::TreeMapScanLock)
            } else {
                self.tr(TextKey::TreeMapHint)
            })
                .size(13)
                .color(Color::from_rgb8(145, 145, 155)),
            body
        ]
        .spacing(10);

        if data.hidden_count > 0 {
            content = content.push(
                text(
                    self.tr(TextKey::TreeMapHiddenItems)
                        .replace("{count}", &data.hidden_count.to_string()),
                )
                .size(13)
                .color(Color::from_rgb8(145, 145, 155)),
            );
        }

        if data.partial_items > 0 {
            content = content.push(
                text(
                    self.tr(TextKey::TreeMapPartialItems)
                        .replace("{count}", &data.partial_items.to_string()),
                )
                .size(13)
                .color(Color::from_rgb8(214, 193, 118)),
            );
        }

        if data.pending_nodes > 0 {
            content = content.push(
                container(
                    text(self.tr(TextKey::TreeMapPending))
                        .size(13)
                        .color(Color::from_rgb8(255, 235, 190)),
                )
                .padding([10, 14])
                .style(|_| iced::widget::container::Style {
                    background: Some(Background::Color(Color::from_rgb8(70, 55, 20))),
                    border: Border {
                        radius: 10.0.into(),
                        width: 1.0,
                        color: Color::from_rgb8(150, 115, 40),
                    },
                    ..Default::default()
                }),
            );
        }

        container(content)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(panel_style)
            .into()
    }

    fn view_treemap_legend(&self) -> Element<'_, Message> {
        row![
            treemap_legend_chip(
                self.tr(TextKey::TreeMapLegendFolders),
                treemap::tile_kind_color(TreemapTileKind::Directory)
            ),
            treemap_legend_chip(
                self.tr(TextKey::TreeMapLegendFiles),
                treemap::tile_kind_color(TreemapTileKind::File)
            ),
            treemap_legend_chip(
                self.tr(TextKey::TreeMapLegendOthers),
                treemap::tile_kind_color(TreemapTileKind::Mixed)
            ),
        ]
        .spacing(10)
        .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let tabs = [
            (Tab::Accueil, self.tr(TextKey::NavigationTabHome)),
            (Tab::Explorer, self.tr(TextKey::NavigationTabExplorer)),
            (Tab::Vue, self.tr(TextKey::NavigationTabTreeMap)),
            (Tab::Options, self.tr(TextKey::NavigationTabSettings)),
            (Tab::Aide, self.tr(TextKey::NavigationTabHelp)),
        ];

        let items = tabs.into_iter().map(|(tab, label)| {
            let active = self.active_tab == tab;

            button(text(label).size(14).color(if active {
                Color::WHITE
            } else {
                Color::from_rgb8(150, 150, 160)
            }))
            .width(Length::Fill)
            .padding([10, 14])
            .style(move |_theme, status| {
                use iced::widget::button::Status;

                let background = match status {
                    Status::Hovered => {
                        if active {
                            Color::from_rgb8(48, 54, 72)
                        } else {
                            Color::from_rgb8(36, 40, 50)
                        }
                    }
                    Status::Pressed => {
                        if active {
                            Color::from_rgb8(42, 48, 64)
                        } else {
                            Color::from_rgb8(30, 34, 42)
                        }
                    }
                    _ => {
                        if active {
                            Color::from_rgb8(40, 45, 60)
                        } else {
                            Color::TRANSPARENT
                        }
                    }
                };

                iced::widget::button::Style {
                    background: Some(Background::Color(background)),
                    text_color: if active {
                        Color::WHITE
                    } else {
                        Color::from_rgb8(150, 150, 160)
                    },
                    border: Border {
                        radius: 10.0.into(),
                        width: if active { 1.0 } else { 0.0 },
                        color: if active {
                            Color::from_rgb8(80, 105, 190)
                        } else {
                            Color::TRANSPARENT
                        },
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::SwitchTab(tab))
            .into()
        });

        container(
            column![
                text(self.tr(TextKey::Title))
                    .size(20)
                    .color(Color::from_rgb8(235, 235, 240)),
                text(self.tr(TextKey::Version))
                    .size(12)
                    .color(Color::from_rgb8(130, 130, 140)),
                Space::new().height(Length::Fixed(16.0)),
                column(items).spacing(6),
                Space::new().height(Length::Fill),
            ]
            .spacing(4),
        )
        .width(220)
        .height(Length::Fill)
        .padding([18, 14])
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgb8(22, 24, 30))),
            border: Border {
                width: 1.0,
                color: Color::from_rgb8(38, 42, 52),
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn footer_scan_progress(&self) -> Option<Element<'_, Message>> {
        if !self.scan_in_progress() {
            return None;
        }

        let label = self
            .tr(TextKey::ScanFooterProgress)
            .replace("{done}", &self.scan_progress_done.to_string())
            .replace("{total}", &self.scan_progress_total.to_string());

        Some(
            container(
                row![
                    text(label)
                        .size(13)
                        .color(Color::from_rgb8(210, 210, 218)),
                    Space::new().width(Length::Fixed(16.0)),
                    container(progress_bar(0.0..=1.0, self.scan_progress_ratio()))
                        .width(Length::Fill),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding([12, 18])
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgb8(24, 26, 33))),
                border: Border {
                    width: 1.0,
                    color: Color::from_rgb8(38, 42, 52),
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into(),
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = match self.active_tab {
            Tab::Accueil => self.view_welcome(),
            Tab::Explorer => self.view_explorer(),
            Tab::Vue => self.view_treemap(),
            Tab::Options => self.view_options(),
            Tab::Aide => container(text(self.tr(TextKey::PlaceholderHelp)))
                .padding(20)
                .into(),
        };

        container(row![
            self.sidebar(),
            column![
                self.header_topbar(),
                container(content)
                    .padding(18)
                    .width(Length::Fill)
                    .height(Length::Fill),
                self.footer_scan_progress().unwrap_or_else(|| {
                    Space::new().height(Length::Shrink).into()
                })
            ]
            .width(Length::Fill)
            .height(Length::Fill)
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(app_background_style)
        .into()
    }

    fn view_options(&self) -> Element<'_, Message> {
        let current_language = match self.locale {
            Locale::En => self.tr(TextKey::OptionsLanguageEn),
            Locale::Fr => self.tr(TextKey::OptionsLanguageFr),
        };

        let language_buttons = row![
            button(text(self.tr(TextKey::OptionsLanguageEn)).size(14).color(
                if self.locale == Locale::En {
                    Color::WHITE
                } else {
                    Color::from_rgb8(210, 210, 218)
                }
            ))
            .padding([10, 16])
            .style(if self.locale == Locale::En {
                primary_button_style
            } else {
                secondary_button_style
            })
            .on_press(Message::SetLocale(Locale::En)),
            button(text(self.tr(TextKey::OptionsLanguageFr)).size(14).color(
                if self.locale == Locale::Fr {
                    Color::WHITE
                } else {
                    Color::from_rgb8(210, 210, 218)
                }
            ))
            .padding([10, 16])
            .style(if self.locale == Locale::Fr {
                primary_button_style
            } else {
                secondary_button_style
            })
            .on_press(Message::SetLocale(Locale::Fr)),
        ]
        .spacing(10);

        let language_card = container(
            column![
                text(self.tr(TextKey::OptionsLanguageLabel))
                    .size(18)
                    .color(Color::from_rgb8(230, 230, 235)),
                text(self.tr(TextKey::OptionsLanguageHelp))
                    .size(14)
                    .color(Color::from_rgb8(150, 150, 160)),
                text(current_language)
                    .size(13)
                    .color(Color::from_rgb8(145, 145, 155)),
                Space::new().height(8),
                language_buttons,
            ]
            .spacing(8),
        )
        .padding(24)
        .width(Length::Fill)
        .style(panel_style);

        container(
            column![
                text(self.tr(TextKey::OptionsOptionsTitle))
                    .size(28)
                    .color(Color::from_rgb8(240, 240, 245)),
                Space::new().height(12),
                language_card,
            ]
            .spacing(4)
            .max_width(700),
        )
        .width(Length::Fill)
        .padding(24)
        .center_x(Length::Fill)
        .into()
    }

    fn view_notice_banner(&self) -> Option<Element<'_, Message>> {
        self.notice.as_ref().map(|notice| {
            let (text_color, background, border_color) = match notice.level {
                NoticeLevel::Error => (
                    Color::from_rgb8(255, 210, 210),
                    Color::from_rgb8(70, 28, 28),
                    Color::from_rgb8(140, 55, 55),
                ),
                NoticeLevel::Warning => (
                    Color::from_rgb8(255, 235, 190),
                    Color::from_rgb8(70, 55, 20),
                    Color::from_rgb8(150, 115, 40),
                ),
            };

            container(text(self.tr(notice.text_key)).size(14).color(text_color))
            .padding([10, 14])
            .width(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(background)),
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: border_color,
                },
                ..Default::default()
            })
            .into()
        })
    }
}

fn sort_group(kind: FileEntryKind) -> u8 {
    match kind {
        FileEntryKind::Directory => 0,
        FileEntryKind::File => 1,
        FileEntryKind::Symlink => 2,
    }
}

fn treemap_legend_chip<'a>(label: &'a str, color: Color) -> Element<'a, Message> {
    container(
        row![
            container(Space::new().width(10).height(10))
                .width(10)
                .height(10)
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(color)),
                    border: Border {
                        radius: 99.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                    ..Default::default()
                }),
            text(label).size(13).color(Color::from_rgb8(205, 205, 212)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
    )
    .padding([6, 10])
    .style(row_container_style)
    .into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::app::state::{FrostScanApp, SortBy};
    use crate::model::file_entry::{FileEntry, FileEntryKind};

    fn entry(name: &str, kind: FileEntryKind, size: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(name),
            name: name.to_string(),
            kind,
            size,
            size_pending: false,
            size_partial: false,
        }
    }

    fn app_with_entries(entries: Vec<FileEntry>, sort_by: SortBy, sort_desc: bool) -> FrostScanApp {
        let (mut app, _) = FrostScanApp::new();
        app.entries = entries;
        app.sort_by = sort_by;
        app.sort_desc = sort_desc;
        app
    }

    #[test]
    fn sorts_by_name_with_directories_first() {
        let app = app_with_entries(
            vec![
                entry("zeta.txt", FileEntryKind::File, 10),
                entry("beta", FileEntryKind::Directory, 0),
                entry("alpha", FileEntryKind::Directory, 0),
                entry("gamma.txt", FileEntryKind::File, 5),
            ],
            SortBy::Name,
            false,
        );

        let names: Vec<_> = app.sorted_entries().into_iter().map(|entry| entry.name).collect();
        assert_eq!(names, vec!["alpha", "beta", "gamma.txt", "zeta.txt"]);
    }

    #[test]
    fn sorts_by_size_desc_with_directories_first() {
        let app = app_with_entries(
            vec![
                entry("small-dir", FileEntryKind::Directory, 10),
                entry("large-file.bin", FileEntryKind::File, 100),
                entry("large-dir", FileEntryKind::Directory, 90),
                entry("small-file.txt", FileEntryKind::File, 5),
            ],
            SortBy::Size,
            true,
        );

        let names: Vec<_> = app.sorted_entries().into_iter().map(|entry| entry.name).collect();
        assert_eq!(
            names,
            vec!["large-dir", "small-dir", "large-file.bin", "small-file.txt"]
        );
    }

    #[test]
    fn sorts_by_name_desc_without_mixing_files_before_directories() {
        let app = app_with_entries(
            vec![
                entry("alpha", FileEntryKind::Directory, 0),
                entry("beta", FileEntryKind::Directory, 0),
                entry("a.txt", FileEntryKind::File, 1),
                entry("z.txt", FileEntryKind::File, 2),
            ],
            SortBy::Name,
            true,
        );

        let names: Vec<_> = app.sorted_entries().into_iter().map(|entry| entry.name).collect();
        assert_eq!(names, vec!["beta", "alpha", "z.txt", "a.txt"]);
    }

    #[test]
    fn sorts_symlinks_after_directories_and_files() {
        let app = app_with_entries(
            vec![
                entry("docs-link", FileEntryKind::Symlink, 0),
                entry("src", FileEntryKind::Directory, 0),
                entry("main.rs", FileEntryKind::File, 20),
            ],
            SortBy::Name,
            false,
        );

        let names: Vec<_> = app.sorted_entries().into_iter().map(|entry| entry.name).collect();
        assert_eq!(names, vec!["src", "main.rs", "docs-link"]);
    }
}
