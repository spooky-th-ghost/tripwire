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
        .register_type::<WireInfo>()
        .insert_resource(WireInfo::default())
        .add_event::<NewSegmentEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    deploy_stake,
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

#[derive(Bundle)]
pub struct SegmentBundle {
    transform: TransformBundle,
    collider: Collider,
    rigidbody: RigidBody,
    impulse_joint: ImpulseJoint,
    segment: Segment,
    name: Name,
}

impl SegmentBundle {
    pub fn new(translation: Vec3, impulse_joint: ImpulseJoint) -> Self {
        SegmentBundle {
            transform: TransformBundle {
                local: Transform::from_translation(translation),
                ..default()
            },
            collider: Collider::capsule_x(0.4, 0.15),
            rigidbody: RigidBody::Dynamic,
            impulse_joint,
            segment: Segment,
            name: Name::from("Segment"),
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
            mesh: meshes.add(shape::Box::new(200.0, 4.0, 200.0).into()),
            material: materials.add(Color::GOLD.into()),
            ..default()
        },
        Collider::cuboid(100.0, 2.0, 100.0),
        RigidBody::Fixed,
    ));

    let mut next_entity = commands
        .spawn((
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
        ))
        .id();

    let mut joint = SphericalJointBuilder::new()
        .local_anchor1(Vec3::X * 0.35)
        .local_anchor2(Vec3::NEG_X * 0.35)
        .build();

    joint.set_contacts_enabled(false);

    let starting_translation = Vec3::new(-4.0, -1.0, 4.0);

    for i in 0..20 {
        let translation = (0.3 * i as f32 * Vec3::X) + starting_translation;
        next_entity = commands
            .spawn((
                SegmentBundle::new(translation, ImpulseJoint::new(next_entity, joint)),
                Damping {
                    linear_damping: 2.0,
                    angular_damping: 3.0,
                },
            ))
            .id();
    }

    let mut stake_joint = SphericalJointBuilder::new()
        .local_anchor1(Vec3::X)
        .local_anchor2(Vec3::ZERO)
        .build();

    stake_joint.set_contacts_enabled(false);

    commands.spawn((
        SceneBundle {
            transform: Transform::from_xyz(5.0, -1.5, 4.0),
            scene: asset_server.load("stake.glb#Scene0"),
            ..default()
        },
        RigidBody::Fixed,
        Collider::cuboid(0.25, 0.5, 0.25),
        Stake,
        Name::from("Stake"),
        ImpulseJoint::new(next_entity, stake_joint),
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
    mut wire_info: ResMut<WireInfo>,
    wire_assets: Res<WireAssets>,
    query: Query<(Entity, &Transform, &GroundSensor), With<Player>>,
) {
    if let Ok((entity, transform, sensor)) = query.get_single() {
        if !wire_info.deployed && input.just_pressed(KeyCode::E) && sensor.grounded {
            let stake_transform = Transform::from_xyz(
                transform.translation.x,
                sensor.ground_height + 0.5,
                transform.translation.z,
            );
            wire_info.deployed = true;
            let stake_entity = commands
                .spawn((
                    SceneBundle {
                        transform: stake_transform,
                        scene: wire_assets.stake.clone(),
                        ..default()
                    },
                    RigidBody::Fixed,
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

fn create_segments(
    mut commands: Commands,
    mut wire_info: ResMut<WireInfo>,
    mut segment_events: EventReader<NewSegmentEvent>,
) {
    for event in segment_events.read() {
        let NewSegmentEvent {
            primary_position,
            target_position,
            primary_entity,
            target_entity,
        } = event;

        let joint_to_primary = RopeJointBuilder::new()
            .local_anchor1(Vec3::ZERO)
            .local_anchor2(Vec3::ZERO)
            .limits([0.0, 0.51]);

        let joint_to_target = RopeJointBuilder::new()
            .local_anchor1(Vec3::ZERO)
            .local_anchor2(Vec3::ZERO)
            .limits([0.0, 0.51])
            .build();

        let new_position = primary_position.lerp(*target_position, 0.5);

        let new_entity = commands
            .spawn((
                SegmentBundle::new(
                    new_position,
                    ImpulseJoint::new(*target_entity, joint_to_target),
                ),
                WirePrimary,
            ))
            .id();
        wire_info.primary_entity = Some(new_entity);

        commands
            .entity(*primary_entity)
            .remove::<Sensor>()
            .remove::<ImpulseJoint>()
            .remove::<GravityScale>()
            .remove::<WirePrimary>()
            .insert(ImpulseJoint::new(new_entity, joint_to_primary));

        wire_info.deployed_segments += 1;
    }
}
