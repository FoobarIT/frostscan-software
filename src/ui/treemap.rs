use iced::alignment;
use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::Path;
use iced::widget::text;
use iced::{Color, Element, Length, Pixels, Point, Rectangle, Renderer, Size, Theme};

use crate::app::message::Message;
use crate::app::state::Tab;
use crate::model::scan_node::{ScanNode, ScanNodeKind};
use crate::ui::format::format_size;
use ::treemap::{Mappable, Rect, TreemapLayout};

const TILE_PADDING: f32 = 3.0;
const LABEL_PADDING: f32 = 8.0;
const MAX_DEPTH: usize = 3;
const MAX_CHILDREN_PER_LEVEL: usize = 10;
const MIN_RECURSIVE_TILE_SIZE: f32 = 64.0;
const HEADER_FRACTION_TOP_LEVEL: f64 = 0.14;
const HEADER_FRACTION_NESTED: f64 = 0.10;
const HEADER_MIN_NORMALIZED: f64 = 0.035;
const CONTENT_INSET_NORMALIZED: f64 = 0.008;
const ROOT_RECURSIVE_SHARE_THRESHOLD: f64 = 0.12;
const NESTED_RECURSIVE_SHARE_THRESHOLD: f64 = 0.18;

#[derive(Debug, Clone)]
pub struct TreemapData {
    pub tiles: Vec<TreemapTile>,
    pub shown_count: usize,
    pub pending_nodes: usize,
    pub partial_items: usize,
    pub hidden_count: usize,
}

#[derive(Debug, Clone)]
pub struct TreemapTile {
    pub label: String,
    pub path: Option<std::path::PathBuf>,
    pub size: u64,
    pub kind: TreemapTileKind,
    pub partial: bool,
    pub pending: bool,
    pub depth: usize,
    pub bounds: Rect,
}

#[derive(Debug, Clone)]
pub struct TreemapStrings {
    pub others_label: String,
    pub folder_label: String,
    pub file_label: String,
    pub group_label: String,
    pub level_label: String,
    pub partial_label: String,
    pub pending_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreemapTileKind {
    Directory,
    File,
    Mixed,
}

impl TreemapData {
    pub fn from_root(root: &ScanNode, strings: &TreemapStrings) -> Self {
        let mut tiles = Vec::new();
        let mut hidden_count = 0;

        build_tiles(
            &root.children,
            Rect::from_points(0.0, 0.0, 1.0, 1.0),
            0,
            strings,
            &mut tiles,
            &mut hidden_count,
        );

        Self {
            shown_count: tiles.len(),
            pending_nodes: root.pending_count(),
            partial_items: root.partial_count(),
            hidden_count,
            tiles,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

pub fn view<'a>(data: &TreemapData, strings: &TreemapStrings) -> Element<'a, Message> {
    canvas(DiskTreemapCanvas::new(
        data.tiles.clone(),
        data.pending_nodes == 0,
        strings.clone(),
    ))
    .width(Length::Fill)
    .height(440)
    .into()
}

pub fn tile_kind_color(kind: TreemapTileKind) -> Color {
    match kind {
        TreemapTileKind::Directory => Color::from_rgb8(47, 102, 153),
        TreemapTileKind::File => Color::from_rgb8(148, 103, 48),
        TreemapTileKind::Mixed => Color::from_rgb8(90, 97, 110),
    }
}

#[derive(Debug, Clone)]
struct LayoutItem {
    size: f64,
    bounds: Rect,
}

impl Mappable for LayoutItem {
    fn size(&self) -> f64 {
        self.size
    }

    fn bounds(&self) -> &Rect {
        &self.bounds
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }
}

fn build_tiles(
    children: &[ScanNode],
    bounds: Rect,
    depth: usize,
    strings: &TreemapStrings,
    output: &mut Vec<TreemapTile>,
    hidden_count: &mut usize,
) {
    let mut visible_children: Vec<&ScanNode> = children.iter().filter(|child| child.size > 0).collect();
    visible_children.sort_by(|left, right| {
        right
            .size
            .cmp(&left.size)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    if visible_children.is_empty() {
        return;
    }

    let total_size: u64 = visible_children.iter().map(|child| child.size).sum();
    if total_size == 0 {
        return;
    }

    let mut display_items: Vec<DisplayNode<'_>> = visible_children
        .iter()
        .take(MAX_CHILDREN_PER_LEVEL)
        .copied()
        .map(DisplayNode::Node)
        .collect();

    if visible_children.len() > MAX_CHILDREN_PER_LEVEL {
        let hidden = &visible_children[MAX_CHILDREN_PER_LEVEL..];
        let hidden_size = hidden.iter().map(|child| child.size).sum();
        let hidden_partial = hidden.iter().any(|child| child.partial);
        *hidden_count += hidden.len();
        display_items.push(DisplayNode::Others {
            label: &strings.others_label,
            size: hidden_size,
            partial: hidden_partial,
        });
    }

    let mut layout_items: Vec<_> = display_items
        .iter()
        .map(|item| LayoutItem {
            size: item.size() as f64,
            bounds: Rect::new(),
        })
        .collect();

    TreemapLayout::new().layout_items(&mut layout_items, bounds);

    for (item, layout) in display_items.into_iter().zip(layout_items.into_iter()) {
        let item_share = if total_size == 0 {
            0.0
        } else {
            item.size() as f64 / total_size as f64
        };
        let tile = TreemapTile {
            label: item.label(),
            path: item.path(),
            size: item.size(),
            kind: item.kind(),
            partial: item.partial(),
            pending: item.pending(),
            depth,
            bounds: layout.bounds,
        };

        output.push(tile.clone());

        if let DisplayNode::Node(node) = item
            && node.kind == ScanNodeKind::Directory
            && !node.children.is_empty()
            && depth + 1 < MAX_DEPTH
            && item_share >= recursive_share_threshold(depth)
        {
            let rect = scale_rect(layout.bounds, Size::new(1.0, 1.0));
            if rect.width >= 0.12 && rect.height >= 0.12 {
                let content_bounds = nested_content_bounds(layout.bounds, depth);
                if content_bounds.w > 0.0 && content_bounds.h > 0.0 {
                    build_tiles(
                        &node.children,
                        content_bounds,
                        depth + 1,
                        strings,
                        output,
                        hidden_count,
                    );
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DisplayNode<'a> {
    Node(&'a ScanNode),
    Others { label: &'a str, size: u64, partial: bool },
}

impl DisplayNode<'_> {
    fn label(self) -> String {
        match self {
            Self::Node(node) => node.name.clone(),
            Self::Others { label, .. } => label.to_string(),
        }
    }

    fn path(self) -> Option<std::path::PathBuf> {
        match self {
            Self::Node(node) if node.kind == ScanNodeKind::Directory => Some(node.path.clone()),
            _ => None,
        }
    }

    fn size(self) -> u64 {
        match self {
            Self::Node(node) => node.size,
            Self::Others { size, .. } => size,
        }
    }

    fn kind(self) -> TreemapTileKind {
        match self {
            Self::Node(node) => match node.kind {
                ScanNodeKind::Directory => TreemapTileKind::Directory,
                ScanNodeKind::File => TreemapTileKind::File,
            },
            Self::Others { .. } => TreemapTileKind::Mixed,
        }
    }

    fn partial(self) -> bool {
        match self {
            Self::Node(node) => node.partial,
            Self::Others { partial, .. } => partial,
        }
    }
    fn pending(self) -> bool {
        match self {
            Self::Node(node) => node.pending,
            Self::Others { .. } => false,
        }
    }
}

#[derive(Debug, Clone)]
struct DiskTreemapCanvas {
    tiles: Vec<TreemapTile>,
    navigation_enabled: bool,
    strings: TreemapStrings,
}

#[derive(Debug, Default)]
struct CanvasState {
    hovered_index: Option<usize>,
}

impl DiskTreemapCanvas {
    fn new(tiles: Vec<TreemapTile>, navigation_enabled: bool, strings: TreemapStrings) -> Self {
        Self {
            tiles,
            navigation_enabled,
            strings,
        }
    }

    fn hit_test(&self, point: Point, size: Size) -> Option<usize> {
        self.tiles.iter().enumerate().rev().find_map(|(index, tile)| {
            let rect = inset_rect(scale_rect(tile.bounds, size), TILE_PADDING);
            rect.contains(point).then_some(index)
        })
    }
}

impl canvas::Program<Message, Theme, Renderer> for DiskTreemapCanvas {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let hovered = cursor
            .position_in(bounds)
            .and_then(|point| self.hit_test(point, bounds.size()));

        if hovered != state.hovered_index {
            state.hovered_index = hovered;
            return Some(canvas::Action::request_redraw());
        }

        if !self.navigation_enabled {
            return None;
        }

        if let (
            Some(index),
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        ) = (hovered, event)
            && let Some(path) = self.tiles[index].path.clone()
        {
            return Some(
                canvas::Action::publish(Message::OpenEntryInTab(path, Tab::Vue)).and_capture(),
            );
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(24, 26, 33));

        for (index, tile) in self.tiles.iter().enumerate() {
            let rect = inset_rect(scale_rect(tile.bounds, bounds.size()), TILE_PADDING);
            if rect.width < 2.0 || rect.height < 2.0 {
                continue;
            }

            let mut color = depth_adjusted_color(tile.kind, tile.depth);

            if state.hovered_index == Some(index) {
                color = brighten(color, 0.08);
            }

            if tile.pending {
                color.a = 0.55;
            }

            frame.fill_rectangle(rect.position(), rect.size(), color);

            if matches!(tile.kind, TreemapTileKind::Directory) && tile.depth <= 1 {
                let header_height = if tile.depth == 0 { 18.0 } else { 12.0 };
                frame.fill_rectangle(
                    rect.position(),
                    Size::new(rect.width, header_height),
                    Color::from_rgba8(255, 255, 255, if tile.depth == 0 { 0.10 } else { 0.06 }),
                );
            }

            frame.stroke(
                &Path::rectangle(rect.position(), rect.size()),
                canvas::Stroke::default()
                    .with_width(if state.hovered_index == Some(index) {
                        2.0
                    } else if matches!(tile.kind, TreemapTileKind::Directory) && tile.depth == 0 {
                        1.5
                    } else {
                        1.0
                    })
                    .with_color(depth_border_color(tile.kind, tile.depth)),
            );

            if tile.partial {
                frame.fill_rectangle(
                    Point::new(rect.x, rect.y),
                    Size::new(rect.width, 5.0),
                    Color::from_rgba8(255, 196, 92, 0.95),
                );
            }

            draw_tile_label(&mut frame, rect, tile);
        }

        if let Some(index) = state.hovered_index
            && let Some(cursor_position) = cursor.position_in(bounds)
        {
            draw_hover_tooltip(
                &mut frame,
                bounds.size(),
                cursor_position,
                &self.tiles[index],
                &self.strings,
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if !self.navigation_enabled {
            return mouse::Interaction::default();
        }

        if cursor
            .position_in(bounds)
            .and_then(|point| self.hit_test(point, bounds.size()))
            .is_some_and(|index| self.tiles[index].path.is_some())
        {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::default()
    }
}

fn draw_tile_label(frame: &mut canvas::Frame<Renderer>, rect: Rectangle, tile: &TreemapTile) {
    if rect.width < MIN_RECURSIVE_TILE_SIZE || rect.height < MIN_RECURSIVE_TILE_SIZE {
        return;
    }

    let title_size = if tile.depth == 0 && rect.width > 200.0 && rect.height > 120.0 {
        16.0
    } else if tile.depth <= 1 {
        13.0
    } else {
        11.0
    };
    let max_width = (rect.width - LABEL_PADDING * 2.0).max(24.0);
    let label = truncate_label(&tile.label, max_width, title_size);

    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(rect.x + LABEL_PADDING, rect.y + LABEL_PADDING),
        color: Color::WHITE,
        size: Pixels::from(title_size),
        line_height: text::LineHeight::default(),
        font: iced::Font::DEFAULT,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        max_width,
    });

    if tile.depth > 0 || rect.width < 120.0 || rect.height < 90.0 {
        return;
    }

    frame.fill_text(canvas::Text {
        content: format_size(tile.size),
        position: Point::new(rect.x + LABEL_PADDING, rect.y + rect.height - 24.0),
        color: Color::from_rgb8(236, 236, 240),
        size: Pixels::from(11.0),
        line_height: text::LineHeight::default(),
        font: iced::Font::DEFAULT,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Basic,
        max_width,
    });
}

fn scale_rect(rect: Rect, bounds: Size) -> Rectangle {
    Rectangle {
        x: rect.x as f32 * bounds.width,
        y: rect.y as f32 * bounds.height,
        width: rect.w as f32 * bounds.width,
        height: rect.h as f32 * bounds.height,
    }
}

fn inset_rect(rect: Rectangle, padding: f32) -> Rectangle {
    Rectangle {
        x: rect.x + padding,
        y: rect.y + padding,
        width: (rect.width - padding * 2.0).max(0.0),
        height: (rect.height - padding * 2.0).max(0.0),
    }
}

fn inset_normalized_rect(rect: Rect, padding: f64) -> Rect {
    Rect {
        x: rect.x + padding,
        y: rect.y + padding,
        w: (rect.w - padding * 2.0).max(0.0),
        h: (rect.h - padding * 2.0).max(0.0),
    }
}

fn nested_content_bounds(rect: Rect, depth: usize) -> Rect {
    let inset = inset_normalized_rect(rect, CONTENT_INSET_NORMALIZED);
    let header_fraction = if depth == 0 {
        HEADER_FRACTION_TOP_LEVEL
    } else {
        HEADER_FRACTION_NESTED
    };
    let header_height = (inset.h * header_fraction).max(HEADER_MIN_NORMALIZED);

    Rect {
        x: inset.x,
        y: inset.y + header_height,
        w: inset.w,
        h: (inset.h - header_height).max(0.0),
    }
}

fn brighten(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r + amount).min(1.0),
        g: (color.g + amount).min(1.0),
        b: (color.b + amount).min(1.0),
        a: color.a,
    }
}

fn recursive_share_threshold(depth: usize) -> f64 {
    if depth == 0 {
        ROOT_RECURSIVE_SHARE_THRESHOLD
    } else {
        NESTED_RECURSIVE_SHARE_THRESHOLD
    }
}

fn depth_adjusted_color(kind: TreemapTileKind, depth: usize) -> Color {
    let base = tile_kind_color(kind);
    let depth_mix = (depth as f32 * 0.14).min(0.34);
    mix(base, Color::from_rgb8(21, 23, 28), depth_mix)
}

fn depth_border_color(kind: TreemapTileKind, depth: usize) -> Color {
    let base = tile_kind_color(kind);
    let bright = brighten(base, 0.24);
    let alpha = (0.34 - depth as f32 * 0.08).max(0.12);
    Color { a: alpha, ..bright }
}

fn mix(left: Color, right: Color, amount: f32) -> Color {
    let keep = 1.0 - amount;
    Color {
        r: left.r * keep + right.r * amount,
        g: left.g * keep + right.g * amount,
        b: left.b * keep + right.b * amount,
        a: left.a * keep + right.a * amount,
    }
}

fn truncate_label(label: &str, max_width: f32, font_size: f32) -> String {
    let max_chars = (max_width / (font_size * 0.58)).floor() as usize;
    if label.chars().count() <= max_chars.max(4) {
        return label.to_string();
    }

    let visible = max_chars.saturating_sub(1).max(3);
    let mut truncated = label.chars().take(visible).collect::<String>();
    truncated.push('…');
    truncated
}

fn draw_hover_tooltip(
    frame: &mut canvas::Frame<Renderer>,
    canvas_size: Size,
    cursor: Point,
    tile: &TreemapTile,
    strings: &TreemapStrings,
) {
    let lines = hover_lines(tile, strings);
    let width = 220.0;
    let height = 20.0 + lines.len() as f32 * 16.0;
    let x = (cursor.x + 16.0).min((canvas_size.width - width - 8.0).max(8.0));
    let y = if cursor.y + height + 16.0 > canvas_size.height {
        (cursor.y - height - 8.0).max(8.0)
    } else {
        cursor.y + 16.0
    };

    let background_rect = Rectangle {
        x,
        y,
        width,
        height,
    };

    frame.fill_rectangle(
        background_rect.position(),
        background_rect.size(),
        Color::from_rgba8(14, 16, 22, 0.94),
    );
    frame.stroke(
        &Path::rectangle(background_rect.position(), background_rect.size()),
        canvas::Stroke::default()
            .with_width(1.0)
            .with_color(Color::from_rgba8(255, 255, 255, 0.12)),
    );

    for (index, line) in lines.iter().enumerate() {
        frame.fill_text(canvas::Text {
            content: line.clone(),
            position: Point::new(x + 10.0, y + 10.0 + index as f32 * 16.0),
            color: if index == 0 {
                Color::WHITE
            } else {
                Color::from_rgb8(212, 216, 223)
            },
            size: Pixels::from(if index == 0 { 13.0 } else { 11.0 }),
            line_height: text::LineHeight::default(),
            font: iced::Font::DEFAULT,
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Basic,
            max_width: width - 20.0,
        });
    }
}

fn hover_lines(tile: &TreemapTile, strings: &TreemapStrings) -> Vec<String> {
    let mut lines = vec![tile.label.clone(), format_size(tile.size)];

    let kind = match tile.kind {
        TreemapTileKind::Directory => strings.folder_label.as_str(),
        TreemapTileKind::File => strings.file_label.as_str(),
        TreemapTileKind::Mixed => strings.group_label.as_str(),
    };
    lines.push(format!("{kind}  •  {} {}", strings.level_label, tile.depth + 1));

    if tile.partial {
        lines.push(strings.partial_label.clone());
    } else if tile.pending {
        lines.push(strings.pending_label.clone());
    }

    lines
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{TreemapData, TreemapStrings, truncate_label};
    use crate::model::scan_node::{ScanNode, ScanNodeKind};

    fn strings() -> TreemapStrings {
        TreemapStrings {
            others_label: "Others".to_string(),
            folder_label: "Folder".to_string(),
            file_label: "File".to_string(),
            group_label: "Group".to_string(),
            level_label: "level".to_string(),
            partial_label: "Partial scan".to_string(),
            pending_label: "Pending scan".to_string(),
        }
    }

    fn file(name: &str, size: u64) -> ScanNode {
        ScanNode {
            path: PathBuf::from(name),
            name: name.to_string(),
            kind: ScanNodeKind::File,
            size,
            partial: false,
            pending: false,
            children: Vec::new(),
        }
    }

    fn dir(name: &str, size: u64, children: Vec<ScanNode>) -> ScanNode {
        ScanNode {
            path: PathBuf::from(name),
            name: name.to_string(),
            kind: ScanNodeKind::Directory,
            size,
            partial: false,
            pending: false,
            children,
        }
    }

    #[test]
    fn creates_nested_tiles_from_tree() {
        let root = dir(
            "root",
            300,
            vec![
                dir("src", 200, vec![file("main.rs", 120), file("lib.rs", 80)]),
                file("readme.md", 100),
            ],
        );

        let data = TreemapData::from_root(&root, &strings());

        assert!(data.tiles.iter().any(|tile| tile.label == "src"));
        assert!(data.tiles.iter().any(|tile| tile.label == "main.rs"));
    }

    #[test]
    fn tracks_hidden_items_when_level_is_dense() {
        let root = dir(
            "root",
            500,
            (0..20)
                .map(|index| file(&format!("item-{index}.bin"), 100 - index))
                .collect(),
        );

        let data = TreemapData::from_root(&root, &strings());
        assert!(data.hidden_count > 0);
    }

    #[test]
    fn truncates_labels_when_needed() {
        let truncated = truncate_label("very-long-filename.iso", 70.0, 13.0);
        assert!(truncated.ends_with('…'));
    }
}
