#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    math::{vec2, vec3},
    prelude::*,
    render::camera::ScalingMode,
    window::{ExitCondition, PrimaryWindow},
};
use rand::prelude::*;

const HIGHLIGHT: Color = Color::srgb(0.7, 0.85, 0.9);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Circles".to_string(),
                ..Default::default()
            }),
            exit_condition: ExitCondition::OnPrimaryClosed,
            ..Default::default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (camera, click, particles, dust))
        .insert_resource(ClearColor(Color::srgb(0.07, 0.07, 0.1)))
        .init_resource::<Click>()
        .init_resource::<WorldCursor>()
        .run();
}

#[derive(Resource, Default)]
struct Click(f32);

#[derive(Resource, Default)]
struct WorldCursor(Vec2);

#[derive(Component)]
struct Particle {
    life: f32,
    opacity: f32,
    scale: f32,
}

#[derive(Component)]
struct Grain {
    life: f32,
    opacity: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Camera2d,
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::TonyMcMapface,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 1.0,
                min_height: 1.0,
            },
            near: 0.0,
            far: 1.0,
            ..OrthographicProjection::default_2d()
        }),
        Bloom::default(),
    ));

    let mut rng = rand::rng();

    let annulus = meshes.add(Annulus::new(0.024, 0.025));
    let circle = meshes.add(Circle::new(0.005));

    let color = ColorMaterial::from(HIGHLIGHT);

    for _ in 0..512 {
        commands.spawn((
            Mesh2d(annulus.clone()),
            MeshMaterial2d(materials.add(color.clone())),
            Transform::from_xyz(
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
                -1.0,
            ),
            Particle {
                life: rng.random(),
                opacity: rng.random_range(0.15..0.8),
                scale: rng.random(),
            },
        ));
    }

    for _ in 0..512 {
        commands.spawn((
            Mesh2d(circle.clone()),
            MeshMaterial2d(materials.add(color.clone())),
            Transform::from_xyz(
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
                -1.0,
            ),
            Grain {
                life: rng.random(),
                opacity: rng.random_range(0.15..0.4),
            },
        ));
    }
}

fn click(mut click: ResMut<Click>, buttons: Res<ButtonInput<MouseButton>>, time: Res<Time>) {
    click.0 = if buttons.just_pressed(MouseButton::Left) {
        1.0
    } else {
        click.0.lerp(0.0, (time.delta_secs() * 5.0).min(1.0))
    }
}

fn camera(
    window: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<&mut Transform, With<Camera>>,
    mut world_cursor: ResMut<WorldCursor>,
    time: Res<Time>,
) {
    if let Ok(window) = window.single() {
        let cursor = window
            .cursor_position()
            .map_or_else(Default::default, |cursor| {
                (cursor - window.size() * 0.5) / window.size().min_element() * vec2(1.0, -1.0)
            });

        let mut camera = camera.single_mut().unwrap();
        let mut translation = camera.translation.truncate();

        translation = translation.lerp(
            cursor
                .normalize_or_zero()
                .mul_add(Vec2::splat(0.5), translation),
            (cursor.length() * time.delta_secs()).min(1.0),
        );

        camera.translation = translation.extend(0.0);

        world_cursor.0 = cursor + translation;
    }
}

fn particles(
    mut particles: Query<
        (
            &mut Transform,
            &mut Particle,
            &MeshMaterial2d<ColorMaterial>,
        ),
        Without<Camera>,
    >,
    camera: Query<&Transform, With<Camera>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    click: Res<Click>,
    cursor: Res<WorldCursor>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    let camera_pos = camera.single().unwrap().translation.truncate();

    let mut rng = rand::rng();

    for (mut transform, mut particle, mat_handle) in &mut particles {
        let sub = 0.125 * delta;

        if particle.life > sub {
            particle.life -= sub;

            let translation = transform.translation.truncate();

            let mdist = click
                .0
                .mul_add(-0.05, translation.distance(cursor.0).mul_add(1.5, 1.0))
                .powi(16);

            let movement = vec2(
                rng.random_range(-0.00005..=0.00005),
                rng.random_range(-0.00005..=0.00005),
            ) + delta
                * 15.0
                * (translation - cursor.0).clamp(Vec2::splat(-0.01), Vec2::splat(0.01))
                / mdist
                * (click.0 + 1.0);

            transform.translation += movement.extend(0.0);

            let scale = (1.0 - particle.life) * particle.scale;
            transform.scale = vec3(scale, scale, 1.0);

            *materials.get_mut(mat_handle.id()).unwrap() = HIGHLIGHT
                .with_alpha(particle.life * particle.opacity)
                .into();
        } else {
            particle.life = 1.0;
            particle.scale = rng.random();
            particle.opacity = rng.random_range(0.15..0.8);

            transform.translation = vec3(
                rng.random_range(camera_pos.x - 1.0..=camera_pos.x + 1.0),
                rng.random_range(camera_pos.y - 1.0..=camera_pos.y + 1.0),
                -1.0,
            );
        }
    }
}

fn dust(
    mut dust: Query<(&mut Transform, &mut Grain, &MeshMaterial2d<ColorMaterial>), Without<Camera>>,
    camera: Query<&Transform, With<Camera>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    click: Res<Click>,
    cursor: Res<WorldCursor>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    let camera_pos = camera.single().unwrap().translation.truncate();

    let mut rng = rand::rng();

    for (mut transform, mut grain, mat_handle) in &mut dust {
        let sub = 0.125 * delta;

        if grain.life > sub {
            grain.life -= sub;

            let translation = transform.translation.truncate();

            let mdist = click
                .0
                .mul_add(-0.05, translation.distance(cursor.0).mul_add(1.5, 1.0))
                .powi(16);

            let movement = vec2(
                rng.random_range(-0.00005..=0.00005),
                rng.random_range(-0.00005..=0.00005),
            ) + delta
                * 15.0
                * (translation - cursor.0).clamp(Vec2::splat(-0.01), Vec2::splat(0.01))
                / mdist
                * (click.0 + 1.0);

            transform.translation += movement.extend(0.0);

            let scale = 1.0 - grain.life;
            transform.scale = vec3(scale, scale, 1.0);

            *materials.get_mut(mat_handle.id()).unwrap() =
                HIGHLIGHT.with_alpha(grain.life * grain.opacity).into();
        } else {
            grain.life = 1.0;
            grain.opacity = rng.random_range(0.15..0.8);

            transform.translation = vec3(
                rng.random_range(camera_pos.x - 1.0..=camera_pos.x + 1.0),
                rng.random_range(camera_pos.y - 1.0..=camera_pos.y + 1.0),
                -1.0,
            );
        }
    }
}
