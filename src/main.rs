use bevy::prelude::*;
use rand::prelude::*;

const GRID_W: usize = 32;
const GRID_H: usize = 32;
const TILE_SIZE: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TileType {
    Sand,
    Water,
    Grass,
}

impl TileType {
    fn color(&self) -> Color {
        match self {
            TileType::Sand => Color::srgb(0.9, 0.8, 0.5),
            TileType::Water => Color::srgb(0.2, 0.4, 0.9),
            TileType::Grass => Color::srgb(0.2, 0.8, 0.3),
        }
    }
}

#[derive(Component)]
struct Tile {
    possible: Vec<TileType>,
    collapsed: bool,
    x: usize,
    y: usize,
}

#[derive(Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, collapse_step)
        .add_systems(Update, refresh_on_r)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    for y in 0..GRID_H {
        for x in 0..GRID_W {
            commands.spawn((
                Tile {
                    possible: vec![TileType::Sand, TileType::Water, TileType::Grass],
                    collapsed: false,
                    x,
                    y,
                },
                Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(
                    x as f32 * TILE_SIZE - GRID_W as f32 * TILE_SIZE / 2.0,
                    y as f32 * TILE_SIZE - GRID_H as f32 * TILE_SIZE / 2.0,
                    0.0,
                ),
                GlobalTransform::default(),
            ));
        }
    }
}

fn collapse_step(mut tiles: Query<(Entity, &mut Tile, &mut Sprite)>) {
    let snapshot: Vec<(Entity, usize, usize, Vec<TileType>, bool)> = tiles
        .iter()
        .map(|(e, t, _)| (e, t.x, t.y, t.possible.clone(), t.collapsed))
        .collect();

    let mut candidates: Vec<_> = snapshot
        .iter()
        .filter(|(_, _, _, possible, collapsed)| !collapsed && !possible.is_empty())
        .map(|(e, _, _, possible, _)| (*e, possible.len()))
        .collect();

    if candidates.is_empty() {
        return;
    }

    candidates.sort_by_key(|(_, len)| *len);
    let entity_to_collapse = candidates[0].0;

    let collapsed_choice = {
        let (_, mut tile, mut sprite) = tiles.get_mut(entity_to_collapse).unwrap();

        let valid_choices: Vec<TileType> = tile
            .possible
            .iter()
            .copied()
            .filter(|&choice| {
                neighbor_coords(tile.x, tile.y)
                    .iter()
                    .all(|&(nx, ny)| {
                        if let Some(neighbor_entity) = entity_at(nx, ny, &snapshot) {
                            let neighbor_possible = snapshot
                                .iter()
                                .find(|(e, _, _, _, _)| *e == neighbor_entity)
                                .unwrap()
                                .3
                                .clone();
                            neighbor_possible.iter().any(|&n| {
                                allowed_neighbor(
                                    choice,
                                    n,
                                    neighbor_direction(tile.x, tile.y, nx, ny).unwrap(),
                                )
                            })
                        } else {
                            true
                        }
                    })
            })
            .collect();

        let mut rng = rand::rng();
        let choice = if valid_choices.is_empty() {
            *tile.possible.choose(&mut rng).unwrap()
        } else {
            *valid_choices.choose(&mut rng).unwrap()
        };

        tile.possible = vec![choice];
        tile.collapsed = true;
        sprite.color = choice.color();
        choice
    };

    let collapsed_tile_info = snapshot
        .iter()
        .find(|(e, _, _, _, _)| *e == entity_to_collapse)
        .unwrap();
    let collapsed_x = collapsed_tile_info.1;
    let collapsed_y = collapsed_tile_info.2;

    for (entity, x, y, _possible, collapsed) in snapshot {
        if entity == entity_to_collapse || collapsed {
            continue;
        }

        if let Some(dir) = neighbor_direction(collapsed_x, collapsed_y, x, y) {
            let (_, mut other_tile, _) = tiles.get_mut(entity).unwrap();
            other_tile.possible = other_tile
                .possible
                .iter()
                .copied()
                .filter(|&n| allowed_neighbor(collapsed_choice, n, dir))
                .collect();

            if other_tile.possible.is_empty() {
                other_tile.possible = vec![TileType::Sand, TileType::Water, TileType::Grass];
            }
        }
    }
}

fn neighbor_coords(x: usize, y: usize) -> Vec<(usize, usize)> {
    let mut neighbors = Vec::new();
    if y + 1 < GRID_H {
        neighbors.push((x, y + 1));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if x + 1 < GRID_W {
        neighbors.push((x + 1, y));
    }
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    neighbors
}

fn entity_at(x: usize, y: usize, snapshot: &[(Entity, usize, usize, Vec<TileType>, bool)]) -> Option<Entity> {
    snapshot.iter().find(|(_, sx, sy, _, _)| *sx == x && *sy == y).map(|(e, _, _, _, _)| *e)
}

fn neighbor_direction(x1: usize, y1: usize, x2: usize, y2: usize) -> Option<Direction> {
    if x1 == x2 && y1 + 1 == y2 {
        Some(Direction::Up)
    } else if x1 == x2 && y1 == y2 + 1 {
        Some(Direction::Down)
    } else if x1 + 1 == x2 && y1 == y2 {
        Some(Direction::Right)
    } else if x1 == x2 + 1 && y1 == y2 {
        Some(Direction::Left)
    } else {
        None
    }
}

fn allowed_neighbor(tile: TileType, neighbor: TileType, _dir: Direction) -> bool {
    match tile {
        TileType::Water => matches!(neighbor, TileType::Water | TileType::Sand),
        TileType::Sand => true,
        TileType::Grass => matches!(neighbor, TileType::Grass | TileType::Sand),
    }
}

fn refresh_on_r(
    mut commands: Commands,
    tiles: Query<Entity, With<Tile>>,
    cameras: Query<Entity, With<Camera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        for tile_entity in tiles.iter() {
            commands.entity(tile_entity).despawn();
        }
        for camera_entity in cameras.iter() {
            commands.entity(camera_entity).despawn();
        }
        setup(commands);
    }
}

