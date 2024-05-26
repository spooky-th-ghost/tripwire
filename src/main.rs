use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_3d::prelude::*;
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
        .add_plugins(PlayerPlugin)
        .register_type::<GroundSensor>()
        .register_type::<WireInfo>()
        .insert_resource(WireInfo::default())
        .add_event::<NewSegmentEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            ((deploy_stake, handle_wire_length), apply_deferred).chain(),
        )
        .run();
}

#[derive(Event)]
pub struct NewSegmentEvent {
    primary_position: Vec3,
    primary_entity: Entity,
    target_position: Vec3,
    target_entity: Entity,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct WireInfo {
    deployed: bool,
    deployed_segments: usize,
    max_segments: usize,
    distance_to_target: f32,
    distance_threshold: f32,
    primary_entity: Option<Entity>,
    target_entity: Option<Entity>,
    spawn_timer: Timer,
}

impl WireInfo {
    pub fn should_extend(&self) -> bool {
        self.distance_to_target > self.distance_threshold
            && self.deployed_segments < self.max_segments
            && self.primary_entity.is_some()
            && self.target_entity.is_some()
            && self.spawn_timer.finished()
    }

    pub fn tick(&mut self, delta: std::time::Duration) {
        self.spawn_timer.tick(delta);
    }

    pub fn reset_timer(&mut self) {
        self.spawn_timer = Timer::from_seconds(0.2, TimerMode::Once);
    }
}

impl Default for WireInfo {
    fn default() -> Self {
        WireInfo {
            deployed: false,
            deployed_segments: 0,
            max_segments: 10,
            distance_to_target: 0.0,
            distance_threshold: 0.25,
            primary_entity: None,
            target_entity: None,
            spawn_timer: Timer::from_seconds(0.2, TimerMode::Once),
        }
    }
}

#[derive(Resource)]
pub struct WireAssets {
    pub stake: Handle<Scene>,
    pub segment: Handle<Scene>,
}

#[derive(Component)]
pub struct Stake;

#[derive(Component)]
pub struct WireTarget;

#[derive(Component)]
pub struct WirePrimary;

#[derive(Component)]
pub struct Segment;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct GroundSensor {
    grounded: bool,
    ground_height: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let chain_layers = CollisionLayers::new(
        ColLayer::PlayerWidget,
        [ColLayer::Object, ColLayer::Terrain],
    );

    commands.insert_resource(WireAssets {
        stake: asset_server.load("stake.glb#Scene0"),
        segment: asset_server.load("segment.glb#Scene0"),
    });

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

    fn segment_translation(i: usize) -> Vec3 {
        let starting_translation = Vec3::new(-4.0, -1.0, 4.0);
        (0.3 * i as f32 * Vec3::X) + starting_translation
    }

    // let chain_iterations = 20;

    // for i in 0..chain_iterations {
    //     let previous_entity = next_entity;
    //     let translation = segment_translation(i);
    //     next_entity = commands
    //         .spawn((
    //             PbrBundle {
    //                 mesh: meshes.add(Capsule3d {
    //                     radius: 0.125,
    //                     half_length: 0.25,
    //                 }),
    //                 material: materials.add(Color::BLUE),
    //                 transform: Transform::from_translation(translation)
    //                     .with_rotation(Quat::from_axis_angle(Vec3::Z, 90.0_f32.to_radians())),
    //                 ..default()
    //             },
    //             RigidBody::Dynamic,
    //             Collider::capsule(0.5, 0.125),
    //             Friction::new(1.0),
    //             chain_layers,
    //             Name::from("Link"),
    //         ))
    //         .id();

    //     commands.spawn(
    //         DistanceJoint::new(previous_entity, next_entity)
    //             .with_limits(0.3, 0.5)
    //             .with_local_anchor_1(Vec3::Y * 0.5)
    //             .with_local_anchor_2(Vec3::NEG_Y * 0.5)
    //             .with_compliance(0.0), // .with_linear_velocity_damping(500.0), // .with_rest_length(0.01),
    //     );
    // }

    // let chain_last_entity = next_entity;

    // next_entity = commands
    //     .spawn((
    //         PbrBundle {
    //             mesh: meshes.add(Cuboid::new(0.5, 2.0, 0.5)),
    //             material: materials.add(Color::GRAY),
    //             transform: Transform::from_translation(segment_translation(chain_iterations)),
    //             ..default()
    //         },
    //         chain_layers,
    //         Name::from("Stake"),
    //         Collider::cuboid(0.5, 2.0, 0.5),
    //         RigidBody::Static,
    //     ))
    //     .id();

    // commands.spawn(
    //     DistanceJoint::new(chain_last_entity, next_entity)
    //         .with_local_anchor_1(Vec3::Y)
    //         .with_local_anchor_2(Vec3::Y)
    //         .with_limits(1.0, 2.0)
    //         .with_compliance(0.0), // .with_linear_velocity_damping(500.0), // .with_rest_length(0.01),
    // );
}

fn deploy_stake(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut wire_info: ResMut<WireInfo>,
    wire_assets: Res<WireAssets>,
    query: Query<(Entity, &Transform), With<player::Player>>,
) {
    if let Ok((entity, transform)) = query.get_single() {
        if !wire_info.deployed && input.just_pressed(KeyCode::KeyE) {
            let stake_transform = Transform::from_translation(transform.translation);
            wire_info.deployed = true;
            let stake_entity = commands
                .spawn((
                    SceneBundle {
                        transform: stake_transform,
                        scene: wire_assets.stake.clone(),
                        ..default()
                    },
                    RigidBody::Static,
                    WirePrimary,
                    Collider::cuboid(0.25, 0.5, 0.25),
                    Stake,
                    Name::from("Stake"),
                    Sensor,
                ))
                .id();

            commands.entity(entity).insert(WireTarget);
            wire_info.primary_entity = Some(stake_entity);
            wire_info.target_entity = Some(entity);
        }
    }
}

fn handle_wire_length(
    time: Res<Time>,
    mut segment_events: EventWriter<NewSegmentEvent>,
    mut wire_info: ResMut<WireInfo>,
    primary_query: Query<&Transform, With<WirePrimary>>,
    target_query: Query<&Transform, With<WireTarget>>,
) {
    wire_info.tick(time.delta());
    if let (Some(primary_entity), Some(target_entity)) =
        (wire_info.primary_entity, wire_info.target_entity)
    {
        println!("Starting length loop");
        if let Ok(primary_transform) = primary_query.get(primary_entity) {
            if let Ok(target_transform) = target_query.get(target_entity) {
                wire_info.distance_to_target = target_transform
                    .translation
                    .distance(primary_transform.translation);

                if wire_info.should_extend() {
                    segment_events.send(NewSegmentEvent {
                        primary_position: primary_transform.translation,
                        target_position: target_transform.translation,
                        target_entity,
                        primary_entity,
                    });
                    println!(
                        "Sending event\nprimary: {:?}\ntarget: {:?}",
                        primary_entity, target_entity
                    );
                    wire_info.primary_entity = None;
                    wire_info.reset_timer();
                }
            }
        }
    }
}
