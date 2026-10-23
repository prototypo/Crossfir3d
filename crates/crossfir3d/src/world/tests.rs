//! Unit tests for the world module.

use super::*;

#[test]
fn the_picture_shows_only_if_the_model_fails() {
    assert_eq!(on_load(Load::Loaded, true), Some(false), "a loaded thing's picture goes");
    assert_eq!(on_load(Load::Loaded, false), Some(true), "and its parts show");
    assert_eq!(on_load(Load::Failed, true), Some(true), "a failed one's picture shows");
    assert_eq!(on_load(Load::Failed, false), Some(false), "and its parts go");
    assert_eq!(on_load(Load::Loading, true), None);
}

#[test]
fn leaving_the_game_forgets_face_pictures() {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = World::new();
    let mut textures = Textures::default();
    textures.images.insert(7, (Handle::default(), Vec2::ONE, Color::WHITE));
    world.insert_resource(textures);
    world.run_system_once(forget_faces).unwrap();
    assert!(world.resource::<Textures>().images.is_empty());
}

#[test]
fn mean_colour_skips_see_through_pixels() {
    let px = [255, 0, 0, 255, 0, 0, 255, 255, 0, 255, 0, 0];
    assert_eq!(mean_colour(&px), Color::srgb(0.5, 0.0, 0.5));
    assert_eq!(mean_colour(&[]), Color::srgb(0.5, 0.5, 0.5));
}

#[test]
fn huge_or_empty_images_rejected() {
    assert!(usable_size(UVec2::new(32, 64)));
    assert!(!usable_size(UVec2::new(16385, 1)));
    assert!(!usable_size(UVec2::new(0, 32)));
}
