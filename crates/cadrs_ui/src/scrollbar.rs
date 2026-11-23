//! A slim scrollbar for a scroll area, like gpui-component's `Scrollbar`: a rounded thumb over
//! the content's edge, with no track drawn and no steppers. Built on Bevy's headless
//! [`Scrollbar`] widget, so dragging the thumb and clicking the track work.
//!
//! The thumb is drawn [`SLIM`] px wide, but the pointer can take it anywhere in a [`HIT`] px
//! strip (the rest of the thumb's width is a transparent border: Bevy paints a background only
//! inside its border). While the pointer is over the strip, or the thumb is dragged, the thumb
//! widens to [`WIDE`] px, so it's easy to see what will be grabbed.
//!
//! The scrollbar hides itself while the content fits.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarDragState, ScrollbarThumb};

use crate::theme::Theme;

pub struct ScrollbarPlugin;

impl Plugin for ScrollbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, widen)
            .add_systems(
                PostUpdate,
                (hide_unneeded, sync_gutters).after(bevy::ui::UiSystems::Layout),
            )
            .add_observer(on_wheel);
    }
}

/// The width (px) of the strip along the edge where the pointer takes the scrollbar.
pub const HIT: f32 = 12.0;
/// The thumb's drawn width (px) at rest.
pub const SLIM: f32 = 6.0;
/// The thumb's drawn width (px) while the pointer is over the scrollbar or drags the thumb.
pub const WIDE: f32 = 10.0;

/// The thumb's transparent border, which leaves `drawn` px of the [`HIT`] strip painted, in the
/// middle.
fn inset(horizontal: bool, drawn: f32) -> UiRect {
    let side = Val::Px(0.5 * (HIT - drawn));
    if horizontal {
        UiRect::vertical(side)
    } else {
        UiRect::horizontal(side)
    }
}

/// The thumb: rounded to half the strip, so what's painted inside the border stays a pill at
/// either width.
fn thumb(name: &str, horizontal: bool, color: Color) -> impl Bundle {
    (
        Name::new(format!("{name}-thumb")),
        ScrollbarThumb {
            border_radius: BorderRadius::all(Val::Px(0.5 * HIT)),
            border: inset(horizontal, SLIM),
        },
        BackgroundColor(color),
    )
}

/// A thumb widens while the pointer is over its scrollbar or drags it.
fn widen(
    q_bar: Query<(&Hovered, Has<Horizontal>, &Children), With<SlimScrollbar>>,
    mut q_thumb: Query<(&mut ScrollbarThumb, &ScrollbarDragState)>,
) {
    for (hovered, horizontal, children) in &q_bar {
        for c in children.iter() {
            let Ok((mut thumb, drag)) = q_thumb.get_mut(c) else {
                continue;
            };
            let drawn = if hovered.get() || drag.dragging {
                WIDE
            } else {
                SLIM
            };
            let want = inset(horizontal, drawn);
            if thumb.border != want {
                thumb.border = want;
            }
        }
    }
}

/// A scrollbar spawned by [`vertical_scrollbar`] or [`horizontal_scrollbar`].
#[derive(Component, Debug, Clone, Copy)]
pub struct SlimScrollbar {
    pub target: Entity,
}

/// On a scroll area whose rows run to its right edge: while its content overflows (the
/// scrollbar shows), this much right padding (px) keeps the rows' ends clear of the scrollbar;
/// while it fits, none, so the rows keep their width.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScrollGutter(pub f32);

fn sync_gutters(mut q: Query<(&ScrollGutter, &ComputedNode, &mut Node)>) {
    for (g, computed, mut node) in &mut q {
        let overflows = computed.content_size().y > computed.size().y + 0.5;
        let want = Val::Px(if overflows { g.0 } else { 0.0 });
        if node.padding.right != want {
            node.padding.right = want;
        }
    }
}

/// A scroll area (`overflow: scroll` on y) that the mouse wheel scrolls: the wheel over it or
/// anything in it moves its [`ScrollPosition`], within its content.
#[derive(Component, Debug, Clone, Copy, Default)]
#[require(ScrollPosition)]
pub struct WheelScroll;

fn on_wheel(
    mut ev: On<PointerScroll>,
    mut q: Query<(&ComputedNode, &mut ScrollPosition), With<WheelScroll>>,
) {
    let Ok((node, mut pos)) = q.get_mut(ev.entity) else {
        return;
    };
    ev.propagate(false);
    let dy = match ev.unit {
        bevy::input::mouse::MouseScrollUnit::Line => ev.y * 24.0,
        bevy::input::mouse::MouseScrollUnit::Pixel => ev.y,
    };
    let max = (node.content_size().y - node.size().y).max(0.0) * node.inverse_scale_factor();
    let y = (pos.y - dy).clamp(0.0, max);
    if pos.y != y {
        pos.y = y;
    }
}

/// Marks a [`SlimScrollbar`] that scrolls sideways.
#[derive(Component, Debug, Clone, Copy)]
struct Horizontal;

/// A slim horizontal scrollbar for `target` (a node with `overflow: scroll` on x), positioned
/// absolutely along the bottom edge of its parent, over the content (a wide table in a narrow
/// panel). It hides while the content fits. Name it with `name`.
pub fn horizontal_scrollbar(name: &str, target: Entity) -> impl Bundle {
    (
        Name::new(name.to_string()),
        SlimScrollbar { target },
        Horizontal,
        Hovered::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(2.0),
            right: Val::Px(2.0),
            bottom: Val::Px(0.0),
            height: Val::Px(HIT),
            ..default()
        },
        Scrollbar::new(target, ControlOrientation::Horizontal, 24.0),
        children![thumb(name, true, Color::srgba_u8(0x80, 0x80, 0x80, 0xb0))],
    )
}

/// A vertical scrollbar for `target` (a node with `overflow: scroll_y` and `ScrollArea`),
/// positioned absolutely along the right edge of its parent, over the content's last [`HIT`] px.
/// Name it with `name`.
pub fn vertical_scrollbar(theme: &Theme, name: &str, target: Entity) -> impl Bundle {
    (
        Name::new(name.to_string()),
        SlimScrollbar { target },
        Hovered::default(),
        // At the right edge, the thumb's middle 6 px in, as before; clear of a rounded panel's
        // corners.
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Px(6.0),
            bottom: Val::Px(6.0),
            width: Val::Px(HIT),
            ..default()
        },
        Scrollbar::new(target, ControlOrientation::Vertical, 24.0),
        children![thumb(name, false, theme.scrollbar_thumb)],
    )
}

fn hide_unneeded(
    mut q: Query<(&SlimScrollbar, Has<Horizontal>, &mut Visibility)>,
    q_target: Query<&ComputedNode>,
) {
    for (bar, horizontal, mut vis) in &mut q {
        let Ok(node) = q_target.get(bar.target) else {
            continue;
        };
        let overflows = if horizontal {
            node.content_size().x > node.size().x + 0.5
        } else {
            node.content_size().y > node.size().y + 0.5
        };
        vis.set_if_neq(if overflows {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// How wide the thumb's paint is: the strip less its transparent border on both sides.
    fn painted(world: &World, bar: Entity) -> f32 {
        let thumb = world.get::<Children>(bar).unwrap()[0];
        let b = world.get::<ScrollbarThumb>(thumb).unwrap().border;
        let px = |v: Val| match v {
            Val::Px(p) => p,
            v => panic!("a thumb's border is in px: {v:?}"),
        };
        HIT - [b.left, b.right, b.top, b.bottom]
            .map(px)
            .iter()
            .sum::<f32>()
    }

    /// Both scrollbars take the pointer across the whole strip and paint a slim thumb in it,
    /// wider while the pointer is over the scrollbar or drags the thumb, and slim again after.
    #[test]
    fn a_thumb_widens_while_hovered_or_dragged() {
        let mut world = World::new();
        let target = world.spawn(Node::default()).id();
        let theme = Theme::light();
        let bars = [
            world.spawn(vertical_scrollbar(&theme, "v", target)).id(),
            world.spawn(horizontal_scrollbar("h", target)).id(),
        ];
        let node = world.get::<Node>(bars[0]).unwrap();
        assert_eq!(node.width, Val::Px(HIT));
        assert_eq!(world.get::<Node>(bars[1]).unwrap().height, Val::Px(HIT));
        let run = |world: &mut World| world.run_system_once(widen).unwrap();
        for bar in bars {
            run(&mut world);
            assert_eq!(painted(&world, bar), SLIM);
            world.entity_mut(bar).insert(Hovered(true));
            run(&mut world);
            assert_eq!(painted(&world, bar), WIDE);
            world.entity_mut(bar).insert(Hovered(false));
            let thumb = world.get::<Children>(bar).unwrap()[0];
            world.get_mut::<ScrollbarDragState>(thumb).unwrap().dragging = true;
            run(&mut world);
            assert_eq!(
                painted(&world, bar),
                WIDE,
                "still wide while dragged off the strip"
            );
            world.get_mut::<ScrollbarDragState>(thumb).unwrap().dragging = false;
            run(&mut world);
            assert_eq!(painted(&world, bar), SLIM);
        }
    }
}
