use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;
use victimless_physics::prelude::ColLayer;

pub struct ChainPlugin;

impl Plugin for ChainPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ChainInfo>()
            .insert_resource(ChainInfo::default())
            .add_systems(Update, (handle_chain_info, extend_chain, deploy_stake));
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ChainInfo {
    deployed: bool,
    locked: bool,
    deployed_segments: usize,
    max_segments: usize,
    distance_to_target: f32,
    distance_threshold: f32,
    rest_length: f32,
    max_distance: f32,
    final_joint_entity: Option<Entity>,
    stake_entity: Option<Entity>,
    primary_entity: Option<Entity>,
    target_entity: Option<Entity>,
}

impl ChainInfo {
    pub fn should_extend(&self) -> bool {
        self.distance_to_target > self.distance_threshold
            && self.deployed_segments < self.max_segments
            && self.stake_entity.is_some()
            && self.primary_entity.is_some()
            && self.target_entity.is_some()
            && self.deployed
            && !self.locked
    }

    pub fn deploy_segment(&mut self, primary_entity: Entity, final_joint_entity: Entity) {
        self.primary_entity = Some(primary_entity);
        self.final_joint_entity = Some(final_joint_entity);
        self.deployed_segments += 1;
    }

    pub fn deploy_stake(&mut self, final_joint_entity: Entity) {
        self.deployed = true;
        self.deployed_segments = 0;
        self.final_joint_entity = Some(final_joint_entity);
    }
}

impl Default for ChainInfo {
    fn default() -> Self {
        ChainInfo {
            deployed: false,
            locked: false,
            deployed_segments: 0,
            max_segments: 10,
            distance_to_target: 0.0,
            distance_threshold: 0.25,
            rest_length: 1.0,
            max_distance: 20.0,
            final_joint_entity: None,
            stake_entity: None,
            primary_entity: None,
            target_entity: None,
        }
    }
}

#[derive(Resource)]
pub struct WireAssets {
    pub stake: Handle<Scene>,
}

#[derive(Component)]
pub struct Stake;

#[derive(Component)]
pub struct ChainPrimary;

#[derive(Component)]
pub struct ChainTarget;

#[derive(Component)]
pub struct FinalJoint;

#[derive(Component)]
pub struct TetherJoint;

#[derive(Component)]
pub struct Segment;

fn handle_chain_info(
    mut chain_info: ResMut<ChainInfo>,
    primary_query: Query<&Transform, With<ChainPrimary>>,
    target_query: Query<&Transform, With<ChainTarget>>,
) {
    if let Ok(primary_transform) = primary_query.get_single() {
        if let Ok(target_transform) = target_query.get_single() {
            chain_info.distance_to_target = primary_transform
                .translation
                .distance(target_transform.translation);
        }
    }
}

fn extend_chain(
    mut commands: Commands,
    mut chain_info: ResMut<ChainInfo>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut final_joint_query: Query<(Entity, &mut DistanceJoint), With<FinalJoint>>,
    primary_query: Query<(Entity, &Transform), (With<ChainPrimary>, Without<FinalJoint>)>,
    target_query: Query<(Entity, &Transform), (With<ChainTarget>, Without<FinalJoint>)>,
) {
    if chain_info.should_extend() {
        if let (
            Ok((primary_entity, primary_transform)),
            Ok((target_entity, target_transform)),
            Ok((final_joint_entity, mut final_joint)),
        ) = (
            primary_query.get(chain_info.primary_entity.unwrap()),
            target_query.get(chain_info.target_entity.unwrap()),
            final_joint_query.get_mut(chain_info.final_joint_entity.unwrap()),
        ) {
            let spawn_offset = (target_transform.translation - primary_transform.translation)
                .normalize_or_zero()
                * chain_info.rest_length;

            let spawn_translation = primary_transform.translation + spawn_offset;

            let new_primary = commands
                .spawn((
                    PbrBundle {
                        mesh: meshes.add(Capsule3d {
                            radius: 0.125,
                            half_length: 0.25,
                        }),
                        material: materials.add(Color::BLUE),
                        transform: Transform::from_translation(spawn_translation)
                            .with_rotation(Quat::from_axis_angle(Vec3::Z, 90.0_f32.to_radians())),
                        ..default()
                    },
                    RigidBody::Dynamic,
                    Collider::capsule(0.5, 0.125),
                    Friction::new(1.0),
                    CollisionLayers::new(
                        ColLayer::PlayerWidget,
                        [ColLayer::Terrain, ColLayer::Object],
                    ),
                    Segment,
                    Name::from("Link"),
                ))
                .id();

            commands.entity(primary_entity).remove::<ChainPrimary>();
            commands.entity(final_joint_entity).remove::<FinalJoint>();

            final_joint.entity2 = new_primary;

            let new_final_joint_entity = commands
                .spawn((
                    DistanceJoint::new(new_primary, target_entity)
                        .with_limits(0.3, 0.5)
                        .with_local_anchor_1(Vec3::Y * 0.5)
                        .with_local_anchor_2(Vec3::NEG_Y * 0.5)
                        .with_compliance(0.0),
                    FinalJoint,
                ))
                .id();

            chain_info.deploy_segment(new_primary, new_final_joint_entity);
        }
    }
}

fn deploy_stake(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut chain_info: ResMut<ChainInfo>,
    wire_assets: Res<WireAssets>,
    query: Query<(Entity, &Transform), With<crate::player::Player>>,
) {
    if let Ok((entity, transform)) = query.get_single() {
        if !chain_info.deployed && input.just_pressed(KeyCode::KeyE) {
            let stake_transform = Transform::from_translation(transform.translation);
            chain_info.deployed = true;
            let stake_entity = commands
                .spawn((
                    SceneBundle {
                        transform: stake_transform,
                        scene: wire_assets.stake.clone(),
                        ..default()
                    },
                    RigidBody::Static,
                    ChainPrimary,
                    Collider::cuboid(0.25, 0.5, 0.25),
                    Stake,
                    Name::from("Stake"),
                    Sensor,
                ))
                .id();

            commands.entity(entity).insert(ChainTarget);
            chain_info.primary_entity = Some(stake_entity);
            chain_info.target_entity = Some(entity);

            // Normal Segment Joint
            let final_joint_entity = commands
                .spawn((
                    DistanceJoint::new(stake_entity, entity)
                        .with_limits(0.0, 0.5)
                        .with_local_anchor_1(Vec3::Y)
                        .with_local_anchor_2(Vec3::X * 0.5)
                        .with_compliance(0.001),
                    FinalJoint,
                ))
                .id();

            // Tether Joint
            commands.spawn((
                DistanceJoint::new(stake_entity, entity)
                    .with_limits(0.0, chain_info.max_distance)
                    .with_local_anchor_1(Vec3::Y)
                    .with_local_anchor_2(Vec3::X * 0.5)
                    .with_compliance(0.0),
                TetherJoint,
            ));

            chain_info.deploy_stake(final_joint_entity);
        }
    }
}
