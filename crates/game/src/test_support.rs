//! Helpers shared by the tests of the screens: synthetic pointer clicks and written messages.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::ecs::message::Messages;
use bevy::picking::backend::HitData;
use bevy::picking::events::{Pointer, PointerClick};
use bevy::picking::pointer::{Location, PointerButton, PointerId};
use bevy::prelude::*;

/// A mouse click with `button` landing on `entity`.
pub fn click_on(entity: Entity, button: PointerButton) -> PointerClick {
    PointerClick {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
        ),
        button,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        duration: Duration::ZERO,
        count: 1,
    }
}

/// Every message of type `M` written so far and still buffered.
pub fn written<M: Message + Clone>(app: &App) -> Vec<M> {
    let messages = app.world().resource::<Messages<M>>();
    messages.get_cursor().read(messages).cloned().collect()
}
