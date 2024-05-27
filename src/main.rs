use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_3d::prelude::*;
use chain::ChainPlugin;
use player::PlayerPlugin;
use victimless_physics::prelude::*;

mod chain;
mod player;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            VictimlessPhysicsPlugin,
            WorldInspectorPlugin::default(),
        ))
        .add_plugins((PlayerPlugin, ChainPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 30.0, -45.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn((
        PbrBundle {
            transform: Transform::from_translation(Vec3::NEG_Y * 4.0),
            mesh: meshes.add(Cuboid::new(100.0, 2.0, 100.0)),
            material: materials.add(Color::GOLD),
            ..default()
        },
        Collider::cuboid(100.0, 2.0, 100.0),
        CollisionLayers::new(
            ColLayer::Terrain,
            [
                ColLayer::Character,
                ColLayer::PlayerWidget,
                ColLayer::Object,
            ],
        ),
        RigidBody::Static,
    ));

    commands.spawn((
        PbrBundle {
            transform: Transform::from_xyz(10.0, 5.0, 0.0),
            mesh: meshes.add(Cuboid::new(5.0, 5.0, 5.0)),
            material: materials.add(Color::PURPLE),
            ..default()
        },
        Collider::cuboid(5.0, 5.0, 5.0),
        CollisionLayers::new(
            ColLayer::Object,
            [
                ColLayer::Character,
                ColLayer::PlayerWidget,
                ColLayer::Terrain,
            ],
        ),
        RigidBody::Dynamic,
    ));
}
