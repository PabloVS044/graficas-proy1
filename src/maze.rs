use raylib::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Cells that the player (and rays) can pass through.
pub const SPAWN: char = 'p';
pub const GOAL: char = 'g';
const EMPTY: char = ' ';

/// The maze as a 2D grid of chars, plus its dimensions so callers don't have to
/// re-measure the rows every time. Indexing is `(i, j)` = `(column, row)`, the
/// same convention the caster uses.
pub struct Maze {
    grid: Vec<Vec<char>>,
    pub width: usize,
    pub height: usize,
}

impl Maze {
    pub fn load(filename: &str) -> Self {
        let file = File::open(filename)
            .unwrap_or_else(|e| panic!("could not open maze file '{filename}': {e}"));
        let reader = BufReader::new(file);

        let mut grid: Vec<Vec<char>> = reader
            .lines()
            .map(|line| line.unwrap().trim_end_matches('\r').chars().collect())
            .collect();

        // Drop trailing blank lines and pad every row to the same length, so a
        // hand-edited file with ragged rows still indexes safely.
        while grid.last().is_some_and(|row| row.is_empty()) {
            grid.pop();
        }
        let width = grid.iter().map(|row| row.len()).max().unwrap_or(0);
        for row in &mut grid {
            row.resize(width, EMPTY);
        }

        let height = grid.len();
        assert!(width > 0 && height > 0, "maze file '{filename}' is empty");

        Maze {
            grid,
            width,
            height,
        }
    }

    /// The char at a cell. Anything outside the grid counts as wall, which is
    /// what stops a ray that escapes through a hole in the border.
    pub fn cell(&self, i: usize, j: usize) -> char {
        if j >= self.height || i >= self.width {
            return '+';
        }
        self.grid[j][i]
    }

    /// Walls are every char that isn't walkable: `+`, `-` and `|` in our file.
    pub fn is_wall(&self, i: usize, j: usize) -> bool {
        !matches!(self.cell(i, j), EMPTY | SPAWN | GOAL)
    }

    /// Same test, but taking world coordinates in pixels. Negative coordinates
    /// are outside the maze, so they count as wall.
    pub fn is_wall_at_pixel(&self, x: f32, y: f32, block_size: usize) -> bool {
        if x < 0.0 || y < 0.0 {
            return true;
        }
        self.is_wall(x as usize / block_size, y as usize / block_size)
    }

    /// First cell holding `target`, scanning top-down / left-right. Used to find
    /// the `p` spawn and the `g` goal.
    pub fn find(&self, target: char) -> Option<(usize, usize)> {
        for (j, row) in self.grid.iter().enumerate() {
            for (i, &cell) in row.iter().enumerate() {
                if cell == target {
                    return Some((i, j));
                }
            }
        }
        None
    }

    pub fn cell_center(&self, i: usize, j: usize, block_size: usize) -> Vector2 {
        let half = block_size as f32 / 2.0;
        Vector2::new(
            i as f32 * block_size as f32 + half,
            j as f32 * block_size as f32 + half,
        )
    }

    /// Which cell a world position falls in.
    pub fn cell_at_pixel(&self, pos: Vector2, block_size: usize) -> (usize, usize) {
        (
            pos.x.max(0.0) as usize / block_size,
            pos.y.max(0.0) as usize / block_size,
        )
    }

    pub fn world_width(&self, block_size: usize) -> f32 {
        (self.width * block_size) as f32
    }

    pub fn world_height(&self, block_size: usize) -> f32 {
        (self.height * block_size) as f32
    }
}
