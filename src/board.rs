//! Board logic

use std::collections::{BTreeMap, HashSet};
use crate::{Direction, Player, PlayerID, Shape, Tile};
use rand::prelude::*;

/// Information about a player's token on the board
pub struct PlayerToken {
    /// ID of player the token is for
    pub player_id: PlayerID,
    /// Position of token (row, col)
    pub position: (usize, usize),
}

impl PlayerToken {
    /// Create a new token for the given player at the given position
    pub fn new(player: &Player, position: (usize, usize)) -> PlayerToken {
        PlayerToken {
            player_id: player.id,
            position
        }
    }
}

/// Information about board state
pub struct Board {
    /// Cells
    cells: Vec<Vec<Tile>>,
    /// Loose tile
    pub loose_tile: Tile,
    /// Loose tile position
    pub loose_tile_position: Option<(Direction, usize)>,
    /// Player tokens
    pub player_tokens: BTreeMap<PlayerID, PlayerToken>,
}

impl Board {
    /// Creates a new board
    pub fn new(width: usize, height: usize, players: &BTreeMap<PlayerID, Player>) -> Board {
        let mut cells = vec![];
        for _ in 0..height {
            let mut row = vec![];
            for _ in 0..width {
                row.push(random());
            }
            cells.push(row);
        }
        cells[0][0] = Tile{shape: Shape::L, orientation: Direction::East};
        cells[0][width - 1] = Tile{shape: Shape::L, orientation: Direction::South};
        cells[height - 1][0] = Tile{shape: Shape::L, orientation: Direction::North};
        cells[height - 1][width - 1] = Tile{shape: Shape::L, orientation: Direction::West};
        let player_tokens = players.iter().enumerate().map(|(i, (_, player))| {
            let position = match i {
                0 => (0, 0),
                1 => (height - 1, width - 1),
                2 => (0, width - 1),
                3 => (height - 1, 0),
                _ => panic!("Too many players"),
            };
            (player.id, PlayerToken::new(player, position))
        }).collect();
        Board {
            cells,
            loose_tile: random(),
            loose_tile_position: None,
            player_tokens,
        }
    }

    /// Gets a cell from the board
    pub fn get(&self, ind: [usize; 2]) -> &Tile {
        &self.cells[ind[1]][ind[0]]
    }

    /// Gets the width of the board
    pub fn width(&self) -> usize {
        self.cells[0].len()
    }

    /// Gets the height of the board
    pub fn height(&self) -> usize {
        self.cells.len()
    }

    fn valid(&self, ind: (usize, usize), dir: Direction) -> bool {
        let (j, i) = ind;
        match dir {
            Direction::North => j > 0,
            Direction::South => j < self.height() - 1,
            Direction::West => i > 0,
            Direction::East => i < self.width() - 1,
        }
    }

    /// Inserts the loose tile at its current position
    pub fn insert_loose_tile(&mut self) {
        if let Some((dir, guide_idx)) = self.loose_tile_position {
            let target_idx = 2 * guide_idx + 1;
            // general process: copy into the current position, so start opposite correct margin
            let (mut j, mut i) = match dir {
                Direction::North => (self.height() - 1, target_idx),
                Direction::South => (0, target_idx),
                Direction::West => (target_idx, self.width() - 1),
                Direction::East => (target_idx, 0),
            };
            let next_loose_tile = self.cells[j][i].clone();
            while self.valid((j, i), dir) {
                let (next_j, next_i) = (j, i) + dir;
                self.cells[j][i] = self.cells[next_j][next_i].clone();
                j = next_j;
                i = next_i;
            }
            self.cells[j][i] = self.loose_tile.clone();
            self.loose_tile = next_loose_tile;
        }
    }

    /// Gets all the coordinates reachable from the given (row, col)
    pub fn reachable_coords(&self, from: (usize, usize)) -> HashSet<(usize, usize)> {
        // result contains everything seen, frontier contains only things not yet scanned
        let mut result = HashSet::new();
        result.insert(from);
        let mut frontier = vec![from];
        // while frontier is nonempty...
        while let Some((curr_row, curr_col)) = frontier.pop() {
            // for each reachable direction...
            for dir in self.cells[curr_row][curr_col].paths() {
                // if it doesn't fall off the board...
                if self.valid((curr_row, curr_col), dir) {
                    // find the connecting tile
                    let (next_row, next_col) = (curr_row, curr_col) + dir;
                    // if that tile connects up as well...
                    if self.cells[next_row][next_col].paths().contains(&(dir * Direction::South)) {
                        // if we've never seen that location before...
                        if !result.contains(&(next_row, next_col)) {
                            // add it to frontier and result
                            frontier.push((next_row, next_col));
                            result.insert((next_row, next_col));
                        }
                    }
                }
            }
        }
        result
    }
}
