use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
            WorldInspectorPlugin::default(),
        ))
        .insert_resource(RapierConfiguration {
            gravity: Vec3::NEG_Y * 30.0,
            ..default()
        })
        .register_type::<GroundSensor>()
        .register_type::<StakeInfo>()
        .insert_resource(StakeInfo::default())
        .add_event::<NewSegmentEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    deploy_stake,
                    update_stake_info,
                    move_player,
                    handle_ground_sensor,
                    (handle_wire_length, create_segments).chain(),
                ),
                apply_deferred,
            )
                .chain(),
        )
        .run();
}

#[derive(Event)]
pub struct NewSegmentEvent {
    stake_position: Vec3,
    target_position: Vec3,
    stake_entity: Entity,
    segment_entity: Entity,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct StakeInfo {
    deployed: bool,
    deployed_segments: usize,
    max_segments: usize,
    distance_to_target: f32,
}

impl Default for StakeInfo {
    fn default() -> Self {
        StakeInfo {
            deployed: false,
            deployed_segments: 0,
            max_segments: 10,
            distance_to_target: 0.0,
        }
    }
}

#[derive(Resource)]
pub struct WireAssets {
    pub stake: Handle<Scene>,
    pub segment: Handle<Scene>,
}

#[derive(Component)]
pub struct Stake {
    target: Entity,
}

#[derive(Component)]
pub struct Segment;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct GroundSensor {
    grounded: bool,
    ground_height: f32,
}

#[derive(Component)]
pub struct Player;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(WireAssets {
        stake: asset_server.load("stake.glb#Scene0"),
        segment: asset_server.load("segment.glb#Scene0"),
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 8.0, -15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn((
        PbrBundle {
            transform: Transform::from_translation(Vec3::NEG_Y * 4.0),
            mesh: meshes.add(shape::Box::new(20.0, 0.5, 20.0).into()),
            material: materials.add(Color::GOLD.into()),
            ..default()
        },
        Collider::cuboid(10.0, 0.25, 10.0),
        RigidBody::Fixed,
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(
                shape::Capsule {
                    radius: 0.5,
                    depth: 1.0,
                    rings: 4,
                    ..default()
                }
                .into(),
            ),
            material: materials.add(Color::BLUE.into()),
            ..default()
        },
        Player,
        LockedAxes::ROTATION_LOCKED,
        RigidBody::Dynamic,
        Velocity::default(),
        Collider::capsule_y(0.5, 0.5),
        GroundSensor::default(),
    ));
}

fn move_player(
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut Velocity, With<Player>>,
) {
    if let Ok(mut velocity) = query.get_single_mut() {
        let mut lat_speed = Vec3::ZERO;

        if input.pressed(KeyCode::A) {
            lat_speed.x += 30.0 * time.delta_seconds();
        }
        if input.pressed(KeyCode::D) {
            lat_speed.x -= 30.0 * time.delta_seconds();
        }
        if input.pressed(KeyCode::W) {
            lat_speed.z += 30.0 * time.delta_seconds();
        }
        if input.pressed(KeyCode::S) {
            lat_speed.z -= 30.0 * time.delta_seconds();
        }

        if lat_speed != Vec3::ZERO {
            velocity.linvel += lat_speed.normalize_or_zero();
        }
    }
}

fn handle_ground_sensor(
    rapier_context: Res<RapierContext>,
    mut query: Query<(Entity, &mut GroundSensor, &Transform)>,
) {
    for (entity, mut ground_sensor, transform) in &mut query {
        if let Some((_, hit_distance)) = rapier_context.cast_ray(
            transform.translation,
            Vec3::NEG_Y,
            1.3,
            true,
            QueryFilter::default().exclude_collider(entity),
        ) {
            ground_sensor.grounded = true;
            ground_sensor.ground_height = transform.translation.y - hit_distance;
        } else {
            ground_sensor.grounded = false;
        }
    }
}

fn deploy_stake(
    mut commands: Commands,
    input: Res<Input<KeyCode>>,
    mut stake_info: ResMut<StakeInfo>,
    wire_assets: Res<WireAssets>,
    player_query: Query<(Entity, &Transform, &GroundSensor), With<Player>>,
) {
    if let Ok((entity, transform, sensor)) = player_query.get_single() {
        if !stake_info.deployed && input.just_pressed(KeyCode::E) && sensor.grounded {
            let stake_transform = Transform::from_xyz(
                transform.translation.x,
                sensor.ground_height + 0.5,
                transform.translation.z,
            );
            stake_info.deployed = true;
            commands.spawn((
                SceneBundle {
                    transform: stake_transform,
                    scene: wire_assets.stake.clone(),
                    ..default()
                },
                RigidBody::Fixed,
                Collider::cuboid(0.25, 0.5, 0.25),
                Stake { target: entity },
                Name::from("Stake"),
                Sensor,
            ));
        }
    }
}

fn update_stake_info(
    mut stake_info: ResMut<StakeInfo>,
    target_query: Query<&Transform, Or<(With<Player>, With<Segment>)>>,
    stake_query: Query<(&Transform, &Stake)>,
) {
    if let Ok((stake_transform, stake)) = stake_query.get_single() {
        if let Ok(target_transform) = target_query.get(stake.target) {
            stake_info.distance_to_target = target_transform
                .translation
                .distance(stake_transform.translation);
        }
    }
}

fn handle_wire_length(
    mut segment_events: EventWriter<NewSegmentEvent>,
    stake_info: Res<StakeInfo>,
    stake_query: Query<(Entity, &Transform, &Stake)>,
    target_query: Query<(Entity, &Transform), Or<(With<Segment>, With<Player>)>>,
) {
    if let Ok((stake_entity, stake_transform, stake)) = stake_query.get_single() {
        if stake_info.distance_to_target >= 0.50
            && stake_info.deployed_segments < stake_info.max_segments
        {
            if let Ok((target_entity, target_transform)) = target_query.get(stake.target) {
                segment_events.send(NewSegmentEvent {
                    stake_position: stake_transform.translation,
                    target_position: target_transform.translation,
                    segment_entity: target_entity,
                    stake_entity,
                });
            }
        }
    }
}

fn create_segments(
    mut commands: Commands,
    mut stake_info: ResMut<StakeInfo>,
    mut segment_events: EventReader<NewSegmentEvent>,
    mut stake_query: Query<&mut Stake>,
) {
    if let Ok(mut stake) = stake_query.get_single_mut() {
        for event in segment_events.read() {
            let NewSegmentEvent {
                stake_position,
                target_position,
                stake_entity,
                segment_entity,
            } = event;
            let joint_to_stake = RopeJointBuilder::new()
                .local_anchor1(Vec3::Y * 0.75)
                .local_anchor2(Vec3::ZERO)
                .limits([0.0, 0.51]);

            let joint_to_segment = RopeJointBuilder::new()
                .local_anchor1(Vec3::ZERO)
                .local_anchor2(Vec3::ZERO)
                .limits([0.0, 0.51])
                .build();

            let new_transform =
                Transform::from_translation(stake_position.lerp(*target_position, 0.5));

            let new_entity = commands
                .spawn((
                    TransformBundle {
                        local: new_transform,
                        ..default()
                    },
                    Segment,
                    RigidBody::Dynamic,
                    Collider::ball(0.125),
                    Sensor,
                    Name::from("Segment"),
                    ImpulseJoint::new(*stake_entity, joint_to_stake),
                ))
                .id();

            commands
                .entity(*segment_entity)
                .remove::<Sensor>()
                .remove::<ImpulseJoint>()
                .insert(ImpulseJoint::new(new_entity, joint_to_segment));

            stake.target = new_entity;
            stake_info.deployed_segments += 1;
        }
    }
}
