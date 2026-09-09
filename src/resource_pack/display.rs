//! ResourcePackDisplay's two unframed menus and selected-pack preview (Java 2.2.4).
use super::ResourcePack;
use crate::{
    assets::Assets,
    config::KeyBindings,
    gfx::{Screen, WIDTH},
    localization::Localization,
};

pub(crate) fn render_info(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    pack: Option<&ResourcePack>,
) {
    let (name, description) = pack.map_or(
        ("Default", "The built-in default resources in Minicraft+."),
        |pack| (pack.name.as_str(), pack.description.as_str()),
    );
    let lines: Vec<_> = [name, description]
        .into_iter()
        .flat_map(|text| {
            crate::content::paginate(localization.text(text), 33, 22)
                .into_iter()
                .flatten()
        })
        .take(22)
        .collect();
    let width = lines
        .iter()
        .map(|line| minicraft_client::gfx::font::text_width(&line))
        .max()
        .unwrap_or(0)
        + 16;
    let height = lines.len() as i32 * 8 + 16;
    let top = (192 - height) / 2;
    crate::world::render_menu_frame(screen, assets.hud(), (288 - width) / 2, top, width, height);
    for (row, line) in lines.iter().enumerate() {
        screen.centered_text(assets.font(), line, top + 8 + row as i32 * 8);
    }
}

pub(crate) fn render(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    packs: &[ResourcePack],
    enabled: &[String],
    selection: usize,
    bindings: &KeyBindings,
) {
    screen.clear(0);
    // Both menus start at y=60. Menu.Builder fits 13 rows before the canvas edge.
    // The indicator menu has no selectable entries, so its own offset stays zero.
    let count = packs.len() + 1;
    let visible = count.min(13);
    let offset = selection.saturating_sub(visible / 2).min(count - visible);
    for index in 0..visible {
        let default = index == packs.len();
        if default || enabled.contains(&packs[index].id) {
            screen.text_colored(
                assets.font(),
                "✓",
                20,
                60 + index as i32 * 10,
                if default { 0x999999 } else { 0xFFFFFF },
            );
        }
    }
    // Indicators bounds end at x=28; Java translates the names menu to x=38.
    // Selectable menus reserve another 16px on each side for arrows.
    let x = if packs.is_empty() { 38 } else { 54 };
    for index in offset..(offset + visible) {
        let name = packs
            .get(index)
            .map_or("Default", |pack| pack.name.as_str());
        let selected = index == selection;
        let y = 60 + (index - offset) as i32 * 10;
        screen.text_colored(
            assets.font(),
            name,
            x,
            y,
            if selected { 0xFFFFFF } else { 0x999999 },
        );
        if selected && index < packs.len() {
            screen.text(assets.font(), "> ", x - 16, y);
            screen.text(
                assets.font(),
                " <",
                x + minicraft_client::gfx::font::text_width(&name),
                y,
            );
        }
    }
    // Render order is menus, title/help, then logo; help can cover long menus in Java.
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.resource_packs.display.title"),
        6,
    );
    for (key, arguments, y) in [
        ("move", vec!["UP".to_owned(), "DOWN".to_owned()], 159),
        ("reorder", vec!["LEFT".to_owned(), "RIGHT".to_owned()], 167),
        ("select", vec![bindings.select.replace('|', "/")], 175),
        (
            "info",
            vec![bindings.display_value(7).replace('|', "/")],
            183,
        ),
    ] {
        let arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
        let text = localization.format(
            &format!("minicraft.displays.resource_packs.display.help.{key}"),
            &arguments,
        );
        screen.text_colored(
            assets.font(),
            &text,
            (WIDTH as i32 - minicraft_client::gfx::font::text_width(&text)) / 2,
            y,
            0x333333,
        );
    }
    let logo = packs
        .get(selection)
        .map_or(&assets.default_pack_logo, |pack| {
            pack.logo.as_ref().unwrap_or(&assets.fallback_pack_logo)
        });
    screen.blit(
        logo,
        (WIDTH as i32 - logo.width as i32) / 2,
        36 - logo.height as i32 / 2,
    );
}
