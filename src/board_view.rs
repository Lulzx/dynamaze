//! Board view

use std::cmp;
use std::ops;

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d as Context;

use crate::{
    BoardController,
    colors::{self, Color}, Direction, PlayerID, Tile,
};
use crate::anim;
use crate::board_controller::TurnState;

#[derive(Clone, Debug)]
struct Diagonal {
    ll: [f64; 2],
    ur: [f64; 2],
}

impl ops::Add<f64> for Diagonal {
    type Output = Diagonal;

    fn add(self, rhs: f64) -> Self::Output {
        Diagonal {
            ll: [self.ll[0] + rhs, self.ll[1] + rhs],
            ur: [self.ur[0] + rhs, self.ur[1] + rhs],
        }
    }
}

impl ops::Sub<f64> for Diagonal {
    type Output = Diagonal;

    fn sub(self, rhs: f64) -> Self::Output {
        self + (-rhs)
    }
}

#[derive(Clone, Debug)]
struct Extents {
    north: f64,
    south: f64,
    east: f64,
    west: f64,
}

impl Extents {
    #[allow(dead_code)]
    fn center(&self) -> [f64; 2] {
        [
            (self.west + self.east) / 2.0,
            (self.north + self.south) / 2.0,
        ]
    }

    fn diagonal(&self) -> Diagonal {
        Diagonal {
            ll: [self.west, self.south],
            ur: [self.east, self.north],
        }
    }

    fn clamp_diagonal(&self, line: Diagonal) -> Diagonal {
        // find equation of line as x + y = k (works for either point since slope assumed to be 1)
        let ll = line.ll;
        let k = ll[0] + ll[1];
        // if k < west + north then too small so use northwest corner
        let (ll, ur) = if k < self.west + self.north {
            let point = [self.west, self.north];
            (point, point)
        } else if k > self.east + self.south {
            // if k > east + south then too big so use southwest corner
            let point = [self.east, self.south];
            (point, point)
        } else if k < self.north + self.east {
            // if less than halfway, before main diagonal, so trust north and west already
            let y_at_west = k - self.west;
            let x_at_north = k - self.north;
            ([self.west, y_at_west], [x_at_north, self.north])
        } else {
            // if more than halfway, after main diagonal, so trust south and east already
            let y_at_east = k - self.east;
            let x_at_south = k - self.south;
            ([x_at_south, self.south], [self.east, y_at_east])
        };
        Diagonal { ll, ur }
    }
}

impl ops::Sub<f64> for Extents {
    type Output = Extents;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn sub(self, rhs: f64) -> Extents {
        Extents {
            north: self.north + rhs,
            south: self.south - rhs,
            east: self.east - rhs,
            west: self.west + rhs,
        }
    }
}

impl ops::Sub<[f64; 2]> for Extents {
    type Output = Extents;

    fn sub(self, rhs: [f64; 2]) -> Self::Output {
        let [x, y] = rhs;
        Extents {
            north: self.north - y,
            south: self.south - y,
            east: self.east - x,
            west: self.west - x,
        }
    }
}

impl PartialEq<Extents> for [f64; 2] {
    fn eq(&self, other: &Extents) -> bool {
        self.partial_cmp(other) == Some(cmp::Ordering::Equal)
    }
}

impl PartialOrd<Extents> for [f64; 2] {
    fn partial_cmp(&self, other: &Extents) -> Option<cmp::Ordering> {
        use std::cmp::Ordering::*;
        let [x, y] = self;
        let result = match (
            x.partial_cmp(&other.west),
            x.partial_cmp(&other.east),
            y.partial_cmp(&other.north),
            y.partial_cmp(&other.south),
        ) {
            // too far west
            (Some(Less), _, _, _) => Greater,
            // too far east
            (_, Some(Greater), _, _) => Greater,
            // too far north
            (_, _, Some(Less), _) => Greater,
            // too far south
            (_, _, _, Some(Greater)) => Greater,
            // entirely within
            (Some(Greater), Some(Less), Some(Greater), Some(Less)) => Less,
            // on west edge
            (Some(Equal), _, _, _) => Equal,
            // on east edge
            (_, Some(Equal), _, _) => Equal,
            // on north edge
            (_, _, Some(Equal), _) => Equal,
            // on south edge
            (_, _, _, Some(Equal)) => Equal,
            // this really shouldn't be possible, and the rust compiler warns about an unreachable pattern!
            // thanks, rust!
            // (Some(_), Some(_), Some(_), Some(_)) => panic!("Implausible bounds check for point in extents"),
            // something is NaN or otherwise fucky
            _ => return None,
        };
        Some(result)
    }
}

/// Stores board view settings
pub struct BoardViewSettings {
    /// Background color
    pub background_color: Color,
    /// Reachable background color
    pub reachable_background_color: Color,
    /// Border color
    pub border_color: Color,
    /// Edge color around the whole board
    pub board_edge_color: Color,
    /// Edge color between cells
    pub cell_edge_color: Color,
    /// Edge radius around the whole board
    pub board_edge_radius: f64,
    /// Edge radius between cells
    pub cell_edge_radius: f64,
    /// Text color
    pub text_color: Color,
    /// Wall color
    pub wall_color: Color,
    /// Tile wall width as percentage of tile size
    pub wall_width: f64,
    /// Insert guide color
    pub insert_guide_color: Color,
    /// UI margin size, south pane
    pub ui_margin_south: f64,
    /// UI margin size, east pane
    pub ui_margin_east: f64,
    /// Font size
    pub font_size: u32,
}

impl BoardViewSettings {
    /// Creates new board view settings
    pub fn new() -> BoardViewSettings {
        BoardViewSettings {
            background_color: colors::TEAL,
            reachable_background_color: colors::LIGHT,
            border_color: colors::DARK,
            board_edge_color: colors::DARK,
            cell_edge_color: colors::DARK,
            board_edge_radius: 3.0,
            cell_edge_radius: 1.0,
            text_color: colors::DARK,
            wall_color: colors::BLUE,
            wall_width: 0.3,
            insert_guide_color: colors::PURPLE,
            ui_margin_south: 100.0,
            ui_margin_east: 300.0,
            font_size: 25,
        }
    }
}

impl Default for BoardViewSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum DrawMode {
    All,
    OnlySelf,
}

/// Stores visual information about a board
pub struct BoardView {
    /// Stores board view settings
    pub settings: BoardViewSettings,
}

impl BoardView {
    /// Creates a new board view
    pub fn new(settings: BoardViewSettings) -> BoardView {
        BoardView { settings }
    }

    /// Gets the size of an individual tile and the x and y padding values
    fn tile_padding(&self, controller: &BoardController, ctx: &Context) -> (f64, f64, f64) {
        let settings = &self.settings;
        let canvas = ctx.canvas().unwrap_throw();
        let cell_max_height = (canvas.height() as f64 - settings.ui_margin_south)
            / (controller.board.height() as f64 + 2.0);
        let cell_max_width = (canvas.width() as f64 - settings.ui_margin_east)
            / (controller.board.width() as f64 + 2.0);
        if cell_max_height < cell_max_width {
            let space_used_x =
                cell_max_height * (controller.board.width() as f64 + 2.0) + settings.ui_margin_east;
            (
                cell_max_height,
                (canvas.width() as f64 - space_used_x) / 2.0,
                0.0,
            )
        } else {
            let space_used_y = cell_max_width * (controller.board.height() as f64 + 2.0)
                + settings.ui_margin_south;
            (
                cell_max_width,
                0.0,
                (canvas.height() as f64 - space_used_y) / 2.0,
            )
        }
    }

    /// Gets the extents of the game and board
    fn game_extents(&self, controller: &BoardController, ctx: &Context) -> (Extents, Extents) {
        let settings = &self.settings;
        let canvas = ctx.canvas().unwrap_throw();
        let (cell_size, x_padding, y_padding) = self.tile_padding(controller, ctx);
        let game = Extents {
            west: x_padding,
            east: canvas.width() as f64 - x_padding - settings.ui_margin_east,
            north: y_padding,
            south: canvas.height() as f64 - y_padding - settings.ui_margin_south,
        };
        let board = game.clone() - cell_size;
        (game, board)
    }

    /// Gets the extents of the south and east UI panels
    fn ui_extents(&self, ctx: &Context) -> (Extents, Extents) {
        let settings = &self.settings;
        let canvas = ctx.canvas().unwrap_throw();
        let global = Extents {
            north: 0.0,
            south: canvas.height() as f64,
            west: 0.0,
            east: canvas.width() as f64,
        };
        let south = Extents {
            north: global.south - settings.ui_margin_south,
            south: global.south,
            west: global.west,
            east: global.east,
        };
        let east = Extents {
            north: global.north,
            south: south.north,
            west: global.east - settings.ui_margin_east,
            east: global.east,
        };
        (south, east)
    }

    /// Draw board
    pub fn draw(&self, controller: &BoardController, local_id: PlayerID, ctx: &Context) {
        // if a child is coming up soon, pretend we are them instead
        let local_id = controller.effective_local_id(local_id);

        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();

        let settings = &self.settings;
        let (cell_size, _, _) = self.tile_padding(controller, ctx);

        // draw board
        let (game, board) = self.game_extents(controller, ctx);
        let board_width = cell_size * board_tile_width as f64;
        let board_height = cell_size * board_tile_height as f64;

        ctx.save();

        // draw the tiles
        self.draw_tiles(controller, local_id, ctx);

        // draw tile edges
        ctx.set_line_width(settings.cell_edge_radius);
        ctx.set_stroke_style(&settings.cell_edge_color.into());
        for i in 0..board_tile_width {
            let x = board.west + i as f64 * cell_size;
            ctx.begin_path();
            ctx.move_to(x, board.north);
            ctx.line_to(x, board.south);
            ctx.stroke();
        }
        for j in 0..board_tile_height {
            let y = game.north + (j + 1) as f64 * cell_size;
            ctx.begin_path();
            ctx.move_to(board.west, y);
            ctx.line_to(board.east, y);
            ctx.stroke();
        }

        // draw board edge
        ctx.set_line_width(settings.board_edge_radius);
        ctx.set_stroke_style(&settings.board_edge_color.into());
        ctx.stroke_rect(board.west, board.north, board_width, board_height);

        // draw insert guides
        self.draw_insert_guides(controller, local_id, ctx);

        // draw player tokens
        self.draw_player_tokens(DrawMode::All, controller, local_id, ctx);

        // draw own token on top of others
        self.draw_player_tokens(DrawMode::OnlySelf, controller, local_id, ctx);

        // draw UI
        self.draw_ui(controller, local_id, ctx);

        ctx.restore();
    }

    fn tile_extents(
        &self,
        controller: &BoardController,
        row: usize,
        col: usize,
        ctx: &Context,
    ) -> Extents {
        let (cell_size, _, _) = self.tile_padding(controller, ctx);
        let (_, board) = self.game_extents(controller, ctx);
        let north = board.north + row as f64 * cell_size;
        let south = north + cell_size;
        let west = board.west + col as f64 * cell_size;
        let east = west + cell_size;
        Extents {
            north,
            south,
            east,
            west,
        }
    }

    /// Checks if a given position is within a tile, and returns that tile's (row, col)
    pub fn in_tile(
        &self,
        pos: &[f64; 2],
        controller: &BoardController,
        ctx: &Context,
    ) -> Option<(usize, usize)> {
        // TODO don't do this dumb thing

        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();

        for j in 0..board_tile_height {
            for i in 0..board_tile_width {
                let cell = self.tile_extents(controller, j, i, ctx);
                if pos < &cell {
                    return Some((j, i));
                }
            }
        }
        None
    }

    fn draw_tiles(&self, controller: &BoardController, local_id: PlayerID, ctx: &Context) {
        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();

        let (cell_size, _, _) = self.tile_padding(controller, ctx);
        let current_player_pos = controller.board.player_pos(local_id);
        let reachable = controller.board.reachable_coords(current_player_pos);
        let loose_insert = &anim::STATE.read().unwrap().loose_insert;

        let [offset_x, offset_y] =
            [0.0, loose_insert.distance_left * cell_size] * loose_insert.offset_dir;

        for j in 0..board_tile_height {
            for i in 0..board_tile_width {
                let cell = self.tile_extents(controller, j, i, ctx);
                let color = if reachable.contains(&(j, i)) {
                    self.settings.reachable_background_color
                } else {
                    self.settings.background_color
                };
                let is_highlighted = controller.highlighted_tile == (j, i);
                ctx.save();
                if loose_insert.applies_to_pos((j, i)) {
                    ctx.translate(offset_x, offset_y).unwrap_throw();
                };
                self.draw_tile(
                    controller.board.get([i, j]),
                    cell,
                    color,
                    is_highlighted,
                    false,
                    controller,
                    local_id,
                    ctx,
                );
                ctx.restore();
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_tile(
        &self,
        tile: &Tile,
        outer: Extents,
        background_color: Color,
        draw_border: bool,
        is_loose: bool,
        controller: &BoardController,
        local_id: PlayerID,
        ctx: &Context,
    ) {
        let settings = &self.settings;

        let (cell_size, _, _) = self.tile_padding(controller, ctx);
        let wall_width = cell_size * settings.wall_width;
        let anim_state = anim::STATE.read().unwrap();

        ctx.save();

        let [x, y] = outer.center();
        ctx.translate(x, y).unwrap_throw();
        ctx.rotate(if is_loose {
            anim_state.loose_rotate.angle
        } else {
            0.0
        })
            .unwrap_throw();

        let outer = outer.clone() - outer.center();
        let inner = outer.clone() - wall_width;

        ctx.set_fill_style(&background_color.into());
        ctx.fill_rect(outer.west, outer.north, cell_size, cell_size);

        if let Some(whose_target) = tile.whose_target {
            let color = controller.players[&whose_target].color;

            // TODO tilt based on something so less reliant on color

            let anim_offset = if tile.whose_target == Some(local_id) {
                anim_state.target_stripe.pct_offset() * cell_size / 3.0
            } else {
                0.0
            };

            let diagonal = outer.diagonal();
            let diagonals = (-4..4)
                .map(|x| cell_size * f64::from(x) / 6.0 + anim_offset)
                .map(|x| diagonal.clone() + x)
                .map(|x| outer.clamp_diagonal(x));
            let polys = diagonals
                .clone()
                .step_by(2)
                .zip(diagonals.skip(1).step_by(2));

            ctx.set_fill_style(&color.into());
            for stripe in polys {
                ctx.begin_path();
                let [x, y] = stripe.0.ur;
                ctx.move_to(x, y);
                let [x, y] = stripe.1.ur;
                ctx.line_to(x, y);
                let [x, y] = stripe.1.ll;
                ctx.line_to(x, y);
                let [x, y] = stripe.0.ll;
                ctx.line_to(x, y);
                ctx.fill();
            }
        }

        ctx.set_fill_style(&settings.wall_color.into());
        ctx.fill_rect(outer.west, outer.north, wall_width, wall_width);
        ctx.fill_rect(inner.east, outer.north, wall_width, wall_width);
        ctx.fill_rect(outer.west, inner.south, wall_width, wall_width);
        ctx.fill_rect(inner.east, inner.south, wall_width, wall_width);
        let walled_directions = tile.walls();
        for d in walled_directions {
            let (x, y, w, h) = match d {
                Direction::North => (outer.west, outer.north, cell_size, wall_width),
                Direction::South => (outer.west, inner.south, cell_size, wall_width),
                Direction::East => (inner.east, outer.north, wall_width, cell_size),
                Direction::West => (outer.west, outer.north, wall_width, cell_size),
            };
            ctx.fill_rect(x, y, w, h);
        }

        if draw_border {
            let border_width = wall_width / 3.0;
            let inner = outer.clone() - border_width;
            ctx.set_fill_style(&settings.text_color.into());
            ctx.fill_rect(outer.west, outer.north, cell_size, border_width);
            ctx.fill_rect(outer.west, inner.south, cell_size, border_width);
            ctx.fill_rect(inner.east, outer.north, border_width, cell_size);
            ctx.fill_rect(outer.west, outer.north, border_width, cell_size);
        }

        ctx.restore();
    }

    fn insert_guides(
        &self,
        controller: &BoardController,
        ctx: &Context,
    ) -> Vec<(Direction, Vec<Extents>)> {
        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();
        let (cell_size, _, _) = self.tile_padding(controller, ctx);
        let (game, board) = self.game_extents(controller, ctx);

        let mut result = vec![];

        let mut north = vec![];
        let mut south = vec![];
        for i in 0..(board_tile_width / 2) {
            let west = board.west + (2 * i + 1) as f64 * cell_size;
            let east = west + cell_size;

            let north_extents = Extents {
                north: game.north,
                south: board.north,
                west,
                east,
            };
            north.push(north_extents);

            let south_extents = Extents {
                north: board.south,
                south: game.south,
                east,
                west,
            };
            south.push(south_extents);
        }
        result.push((Direction::North, north));
        result.push((Direction::South, south));
        let mut east = vec![];
        let mut west = vec![];
        for j in 0..(board_tile_height / 2) {
            let north = board.north + (2 * j + 1) as f64 * cell_size;
            let south = north + cell_size;

            let west_extents = Extents {
                north,
                south,
                west: game.west,
                east: board.west,
            };
            west.push(west_extents);

            let east_extents = Extents {
                north,
                south,
                west: board.east,
                east: game.east,
            };
            east.push(east_extents);
        }
        result.push((Direction::East, east));
        result.push((Direction::West, west));
        result
    }

    fn draw_insert_guides(&self, controller: &BoardController, _local_id: PlayerID, ctx: &Context) {
        let settings = &self.settings;

        let (cell_size, _, _) = self.tile_padding(controller, ctx);
        let wall_width = cell_size * settings.wall_width;

        ctx.save();

        ctx.set_fill_style(&settings.insert_guide_color.into());
        for (dir, guides) in self.insert_guides(controller, ctx) {
            for guide in guides {
                let guide = guide - wall_width;
                let mid_x = (guide.east + guide.west) / 2.0;
                let mid_y = (guide.north + guide.south) / 2.0;
                let ((x0, y0), (x1, y1), (x2, y2)) = match dir {
                    Direction::North => (
                        (guide.west, guide.north),
                        (mid_x, guide.south),
                        (guide.east, guide.north),
                    ),
                    Direction::South => (
                        (guide.west, guide.south),
                        (mid_x, guide.north),
                        (guide.east, guide.south),
                    ),
                    Direction::West => (
                        (guide.west, guide.north),
                        (guide.east, mid_y),
                        (guide.west, guide.south),
                    ),
                    Direction::East => (
                        (guide.east, guide.north),
                        (guide.west, mid_y),
                        (guide.east, guide.south),
                    ),
                };
                ctx.begin_path();
                ctx.move_to(x0, y0);
                ctx.line_to(x1, y1);
                ctx.line_to(x2, y2);
                ctx.close_path();
                ctx.fill();
            }
        }

        ctx.restore();
    }

    /// Checks if the given position is in an insert guide or not
    pub fn in_insert_guide(
        &self,
        pos: &[f64; 2],
        controller: &BoardController,
        ctx: &Context,
    ) -> Option<(Direction, usize)> {
        for (dir, guides) in self.insert_guides(controller, ctx) {
            for (i, guide) in guides.into_iter().enumerate() {
                if pos < &guide {
                    return Some((dir, i));
                }
            }
        }
        None
    }

    fn loose_tile_extents(&self, controller: &BoardController, ctx: &Context) -> Extents {
        let (target_dir, idx) = controller.board.loose_tile_position;
        for (dir, guides) in self.insert_guides(controller, ctx) {
            if dir == target_dir {
                return guides[idx].clone();
            }
        }
        unreachable!()
    }

    /// Check if the given position is within the loose tile area
    pub fn in_loose_tile(
        &self,
        pos: &[f64; 2],
        controller: &BoardController,
        ctx: &Context,
    ) -> bool {
        let cell = self.loose_tile_extents(controller, ctx);
        pos < &cell
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_player_tokens(
        &self,
        mode: DrawMode,
        controller: &BoardController,
        local_id: PlayerID,
        ctx: &Context,
    ) {
        let settings = &self.settings;

        let (cell_size, _, _) = self.tile_padding(controller, ctx);
        let wall_width = cell_size * settings.wall_width;
        let anim_state = anim::STATE.read().unwrap();
        let token_radius = cell_size / 2.0 - wall_width;

        for token in controller.board.player_tokens.values() {
            let (row, col) = token.position;
            let player = match controller.players.get(&token.player_id) {
                Some(x) => x,
                None => continue,
            };
            let tile = self.tile_extents(controller, row, col, ctx);

            ctx.save();

            if anim_state.loose_insert.applies_to_pos((row, col)) {
                let [x, y] = [0.0, anim_state.loose_insert.distance_left * cell_size]
                    * anim_state.loose_insert.offset_dir;
                ctx.translate(x, y).unwrap_throw();
            };

            let should = mode == DrawMode::All || token.player_id == local_id;
            if should {
                ctx.begin_path();
                ctx.set_fill_style(&player.color.into());
                let [x, y] = tile.center();
                ctx.ellipse(
                    x,
                    y,
                    token_radius,
                    token_radius,
                    0.0,
                    0.0,
                    ::std::f64::consts::PI * 2.0,
                )
                    .unwrap_throw();
                ctx.fill();
                if token.player_id == local_id {
                    let dot_radius = token_radius - wall_width / 2.0;
                    ctx.begin_path();
                    ctx.set_fill_style(&JsValue::from_str("black"));
                    ctx.ellipse(
                        x,
                        y,
                        dot_radius,
                        dot_radius,
                        0.0,
                        0.0,
                        ::std::f64::consts::PI * 2.0,
                    )
                        .unwrap_throw();
                    ctx.fill();
                }
            }

            ctx.restore();
        }
    }

    fn draw_ui(&self, controller: &BoardController, local_id: PlayerID, ctx: &Context) {
        let (cell_size, _, _) = self.tile_padding(controller, ctx);
        let anim_state = anim::STATE.read().unwrap();

        // draw loose tile
        {
            let cell = self.loose_tile_extents(controller, ctx);
            ctx.save();
            if anim_state
                .loose_insert
                .applies_to_loose(controller.board.loose_tile_position)
            {
                let [x, y] = [0.0, anim_state.loose_insert.distance_left * cell_size]
                    * anim_state.loose_insert.offset_dir;
                ctx.translate(x, y).unwrap_throw();
            };
            self.draw_tile(
                &controller.board.loose_tile,
                cell,
                self.settings.background_color,
                false,
                true,
                controller,
                local_id,
                ctx,
            );
            ctx.restore();
        }

        // draw player target
        {
            let (south_panel, _) = self.ui_extents(ctx);
            let my_turn = controller.local_turn(local_id);
            let whose_turn = controller.active_player();
            ctx.save();

            ctx.set_fill_style(&self.settings.text_color.into());
            ctx.set_font("20px sans-serif");
            let text = format!("It is {}'s turn", whose_turn.name);
            let x = south_panel.west;
            let y = south_panel.north + 20.0;
            ctx.fill_text(&text, x, y).unwrap_throw();
            if my_turn {
                let text = match controller.turn_state {
                    TurnState::InsertTile => {
                        "Right-click at a triangle to rotate, left-click to insert"
                    }
                    TurnState::MoveToken => "Click on any reachable tile, or yourself to not move",
                };
                let y = y + 30.0;
                ctx.fill_text(&text, x, y).unwrap_throw();
            }
            if let Some(tutorial_step) = &controller.board.tutorial_step {
                let text = tutorial_step.text();
                let y = y + 60.0;
                ctx.fill_text(&text, x, y).unwrap_throw();
            }

            ctx.restore();
        }

        // draw player list
        {
            let (_, east_panel) = self.ui_extents(ctx);
            ctx.save();

            ctx.set_font("15px sans-serif");

            let x = east_panel.west;
            let mut y = east_panel.north + 20.0;
            for player_id in &controller.turn_order {
                let player = &controller.players[player_id];
                let token = &controller.board.player_tokens[player_id];

                ctx.set_fill_style(&self.settings.text_color.into());
                ctx.fill_text(&player.name, x, y).unwrap_throw();
                y += 10.0;

                ctx.begin_path();
                ctx.set_fill_style(&player.color.into());
                ctx.ellipse(
                    x + 7.5,
                    y + 7.5,
                    7.5,
                    7.5,
                    0.0,
                    0.0,
                    ::std::f64::consts::PI * 2.0,
                )
                    .unwrap_throw();
                ctx.fill();
                ctx.set_fill_style(&self.settings.text_color.into());
                let text = format!("score: {}", token.score);
                ctx.fill_text(&text, x + 20.0, y + 10.0).unwrap_throw();
                y += 40.0;
            }

            ctx.restore();
        }
    }
}
