use iced::{Background, Border, Color, Theme};

pub fn app_background_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(18, 19, 24))),
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

pub fn topbar_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(24, 26, 33))),
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

pub fn panel_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(28, 30, 38))),
        text_color: Some(Color::WHITE),
        border: Border {
            width: 1.0,
            radius: 14.0.into(),
            color: Color::from_rgb8(44, 47, 58),
        },
        ..Default::default()
    }
}

pub fn path_bar_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(32, 34, 42))),
        text_color: Some(Color::WHITE),
        border: Border {
            width: 1.0,
            radius: 12.0.into(),
            color: Color::from_rgb8(48, 52, 64),
        },
        ..Default::default()
    }
}

pub fn table_header_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(36, 39, 49))),
        text_color: Some(Color::WHITE),
        border: Border {
            width: 1.0,
            radius: 10.0.into(),
            color: Color::from_rgb8(52, 56, 69),
        },
        ..Default::default()
    }
}

pub fn row_container_style(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb8(31, 33, 41))),
        text_color: Some(Color::WHITE),
        border: Border {
            width: 1.0,
            radius: 10.0.into(),
            color: Color::from_rgb8(42, 45, 56),
        },
        ..Default::default()
    }
}

pub fn primary_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(82, 108, 201),
        _ => Color::from_rgb8(70, 95, 180),
    };

    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: 10.0.into(),
            ..Border::default()
        },
        ..Default::default()
    }
}

pub fn secondary_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(52, 56, 69),
        _ => Color::from_rgb8(40, 43, 52),
    };

    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::from_rgb8(235, 235, 240),
        border: Border {
            radius: 10.0.into(),
            ..Border::default()
        },
        ..Default::default()
    }
}

pub fn header_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(48, 52, 65),
        _ => Color::TRANSPARENT,
    };

    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::from_rgb8(220, 220, 228),
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..Default::default()
    }
}

pub fn row_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(44, 48, 60),
        _ => Color::TRANSPARENT,
    };

    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..Default::default()
    }
}
