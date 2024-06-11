use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;
use victimless_bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, set_player_direction);
    }
}

// types
#[derive(Component)]
pub struct Player;

// systems
fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Capsule3d {
                radius: 0.5,
                half_length: 0.5,
            }),
            material: materials.add(Color::RED),
            ..default()
        },
        CharacterBundle::player().with_move_speed(200.0),
        Player,
        Name::from("Player"),
    ));
}

fn set_player_direction(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut MoveDirection, &mut MoveSpeed, Has<Grounded>), With<Player>>,
) {
    for (mut direction, mut speed, is_grounded) in &mut query {
        if is_grounded {
            let mut lat_dir = Vec3::ZERO;

            if input.pressed(KeyCode::KeyA) {
                lat_dir.x += 1.0;
            }
            if input.pressed(KeyCode::KeyD) {
                lat_dir.x -= 1.0;
            }
            if input.pressed(KeyCode::KeyW) {
                lat_dir.z += 1.0;
            }
            if input.pressed(KeyCode::KeyS) {
                lat_dir.z -= 1.0;
            }

            direction.set(lat_dir);

            if direction.started_moving() {
                speed.start_moving();
            }

            if direction.stopped_moving() {
                speed.stop_moving();
            }
        }
    }
}
