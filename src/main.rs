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
        .insert_resource(WireData::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                deploy_stake,
                handle_wire_length,
                move_player,
                handle_ground_sensor,
            ),
        )
        .run();
}

#[derive(Resource)]
pub struct WireData {
    deployed: bool,
    deployed_segments: usize,
    max_segments: usize,
}

impl Default for WireData {
    fn default() -> Self {
        WireData {
            deployed: false,
            deployed_segments: 0,
            max_segments: 20,
        }
    }
}

#[derive(Resource)]
pub struct WireAssets {
    pub stake: Handle<Scene>,
    pub segment: Handle<Scene>,
}

#[derive(Component, Default)]
pub struct Stake {
    furthest_segment: Option<Entity>,
    nearest_segment: Option<Entity>,
}

#[derive(Component)]
pub struct Segment {
    parent_anchor: Entity,
}

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
            velocity.linvel += lat_speed;
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
    mut wire_data: ResMut<WireData>,
    wire_assets: Res<WireAssets>,
    player_query: Query<(&Transform, &GroundSensor), With<Player>>,
) {
    if let Ok((transform, sensor)) = player_query.get_single() {
        if !wire_data.deployed && input.just_pressed(KeyCode::E) && sensor.grounded {
            let stake_transform = Transform::from_xyz(
                transform.translation.x,
                sensor.ground_height + 0.5,
                transform.translation.z,
            );
            wire_data.deployed = true;
            commands.spawn((
                SceneBundle {
                    transform: stake_transform,
                    scene: wire_assets.stake.clone(),
                    ..default()
                },
                RigidBody::Fixed,
                Collider::cuboid(0.25, 0.5, 0.25),
                Stake::default(),
            ));
        }
    }
}

fn handle_wire_length(
    mut commands: Commands,
    mut stake_query: Query<(&mut Stake, &Transform)>,
    segment_query: Query<Entity, With<Segment>>,
) {
    // Look at the distance between the nearest segment
    // if it is longer than the threshhold and the deployed segments is less than max
    // create a new segment and give it a joint anchoring it to the stake
    // remove the joint from the nearest segment and attach to the new segment
    // remove the sensor from the nearest segment
}

fn recall_wire() {
    // give sensors and a gravity scale of zero to the stake and all segments
    // start pulling the closest segment to the player
    // whenever a segment collides with the player remove it and start moving the next segment to
    // the player
    // once the stake collides with the player set deployed to false
}
