use once_cell::sync::OnceCell;
use rand::{Rng, SeedableRng};
use raylib::color::Color;
use raylib::prelude::*;
use std::thread;

use crate::game::{Game, GameResult, Player};

const HALF_LINEWIDTH: i32 = 1;
const SQUARE_SIZE: i32 = 32;

// Store the main thread ID to ensure rendering only happens on main thread
static MAIN_THREAD_ID: OnceCell<thread::ThreadId> = OnceCell::new();
const DECK_SIZE: usize = 2 * NUM_TETROMINOES; // To implement the 7-bag system
const NUM_PREVIEW: usize = 2;
const NUM_FLOAT_OBS: usize = 6;

#[repr(u8)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    #[default]
    NoOp = 0,
    Left = 1,
    Right = 2,
    Rotate = 3,
    SoftDrop = 4,
    HardDrop = 5,
    Hold = 6,
}

impl From<u8> for Action {
    fn from(value: u8) -> Self {
        match value {
            0 => Action::NoOp,
            1 => Action::Left,
            2 => Action::Right,
            3 => Action::Rotate,
            4 => Action::SoftDrop,
            5 => Action::HardDrop,
            6 => Action::Hold,
            _ => Action::NoOp, // Default to NoOp for invalid values
        }
    }
}

const NUM_ROWS: usize = 20;
const NUM_COLS: usize = 10;

const MAX_TICKS: usize = 10000;
const PERSONAL_BEST: usize = 67890;
const INITIAL_TICKS_PER_FALL: usize = 6; // how many ticks before the tetromino naturally falls down of one square

const LINES_PER_LEVEL: usize = 10;
// Revisit scoring with level. See https://tetris.wiki/Scoring
const SCORE_SOFT_DROP: usize = 1;
#[allow(dead_code)]
const REWARD_SOFT_DROP: f32 = 0.0;
const SCORE_HARD_DROP: usize = 2;
const REWARD_HARD_DROP: f32 = 0.02;
const REWARD_ROTATE: f32 = 0.01;
const REWARD_INVALID_ACTION: f32 = 0.0;

const SCORE_COMBO: [i32; 5] = [0, 100, 300, 500, 1000];
const REWARD_COMBO: [f32; 5] = [0.0, 0.1, 0.3, 0.5, 1.0];

#[derive(Debug)]
struct Client {
    total_cols: i32,
    total_rows: i32,
    ui_rows: i32,
    deck_rows: i32,
    rl: RaylibHandle,
    thread: RaylibThread,
}

#[derive(Debug, Clone)]
pub struct Tetris {
    rewards: f32,
    is_terminal: bool,
    n_rows: usize,
    n_cols: usize,
    grid: [i32; NUM_ROWS * NUM_COLS],
    rng: rand::rngs::SmallRng,
    tick: usize,
    tick_fall: usize,
    ticks_per_fall: usize,
    score: usize,
    can_swap: bool,
    tetromino_deck: [usize; DECK_SIZE],
    hold_tetromino: Option<usize>,
    cur_position_in_deck: usize,
    cur_tetromino: usize,
    cur_tetromino_row: usize,
    cur_tetromino_col: usize,
    cur_tetromino_rot: usize,
    ep_return: f32,
    lines_deleted: u32,
    count_combos: u32,
    game_level: u32,
    atn_count_hard_drop: u32,
    atn_count_soft_drop: u32,
    atn_count_rotate: u32,
    atn_count_hold: u32,
    tetromino_counts: [u32; NUM_TETROMINOES],
}

impl Tetris {
    pub fn new() -> Self {
        let n_rows = NUM_ROWS;
        let n_cols = NUM_COLS;

        let mut tetris = Self {
            rewards: 0.0,
            is_terminal: false,
            n_rows,
            n_cols,
            grid: [0; NUM_ROWS * NUM_COLS],
            rng: rand::rngs::SmallRng::seed_from_u64(rand::rng().random()),
            tick: 0,
            tick_fall: 0,
            ticks_per_fall: INITIAL_TICKS_PER_FALL,
            score: 0,
            can_swap: true,
            tetromino_deck: [0; DECK_SIZE],
            hold_tetromino: None,
            cur_position_in_deck: 0,
            cur_tetromino: 0,
            cur_tetromino_row: 0,
            cur_tetromino_col: 0,
            cur_tetromino_rot: 0,
            ep_return: 0.0,
            lines_deleted: 0,
            count_combos: 0,
            game_level: 1,
            atn_count_hard_drop: 0,
            atn_count_soft_drop: 0,
            atn_count_rotate: 0,
            atn_count_hold: 0,
            tetromino_counts: [0; NUM_TETROMINOES],
        };
        tetris.reset();
        tetris
    }

    fn restore_grid(&mut self) {
        self.grid.fill(0);
    }

    fn refill_and_shuffle(array: &mut [usize], rng: &mut rand::rngs::SmallRng) {
        // Hold can change the deck distribution, so need to refill
        for (i, item) in array.iter_mut().enumerate() {
            *item = i;
        }

        // Fisher-Yates shuffle
        for i in (1..NUM_TETROMINOES).rev() {
            let j = rng.random_range(0..=i);
            array.swap(i, j);
        }
    }

    fn initialize_deck(&mut self) {
        // Implements a 7-bag system. The deck is composed of two bags.
        Self::refill_and_shuffle(&mut self.tetromino_deck[0..NUM_TETROMINOES], &mut self.rng); // First bag
        Self::refill_and_shuffle(
            &mut self.tetromino_deck[NUM_TETROMINOES..DECK_SIZE],
            &mut self.rng,
        ); // Second bag
        self.cur_position_in_deck = 0;
        self.cur_tetromino = self.tetromino_deck[self.cur_position_in_deck];
    }

    fn spawn_new_tetromino(&mut self) {
        self.cur_position_in_deck = (self.cur_position_in_deck + 1) % DECK_SIZE;
        self.cur_tetromino = self.tetromino_deck[self.cur_position_in_deck];
        self.cur_tetromino_rot = 0;

        if self.cur_position_in_deck == 0 {
            // Now using the first bag, so shuffle the second bag
            Self::refill_and_shuffle(
                &mut self.tetromino_deck[NUM_TETROMINOES..DECK_SIZE],
                &mut self.rng,
            );
        } else if self.cur_position_in_deck == NUM_TETROMINOES {
            // Now using the second bag, so shuffle the first bag
            Self::refill_and_shuffle(&mut self.tetromino_deck[0..NUM_TETROMINOES], &mut self.rng);
        }

        self.cur_tetromino_col = self.n_cols / 2;
        self.cur_tetromino_row = 0;
        self.tick_fall = 0;
        self.tetromino_counts[self.cur_tetromino] += 1;
    }

    // This is only used to check if the game is done
    #[allow(clippy::needless_range_loop)]
    fn can_spawn_new_tetromino(&self) -> bool {
        let next_pos = (self.cur_position_in_deck + 1) % DECK_SIZE;
        let next_tetromino = self.tetromino_deck[next_pos];
        for c in 0..(TETROMINO_FILL_COLS[next_tetromino][0] as usize) {
            for r in 0..(TETROMINO_FILL_ROWS[next_tetromino][0] as usize) {
                if (self.grid[r * self.n_cols + c + self.n_cols / 2] != 0)
                    && (TETROMINOES[next_tetromino][0][r][c] == 1)
                {
                    return false;
                }
            }
        }
        true
    }

    #[allow(clippy::needless_range_loop)]
    fn can_soft_drop(&self) -> bool {
        if self.cur_tetromino_row
            == (self.n_rows
                - TETROMINO_FILL_ROWS[self.cur_tetromino][self.cur_tetromino_rot] as usize)
        {
            return false;
        }
        for c in 0..(TETROMINO_FILL_COLS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
            for r in 0..(TETROMINO_FILL_ROWS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
                if (self.grid
                    [(r + self.cur_tetromino_row + 1) * self.n_cols + c + self.cur_tetromino_col]
                    != 0)
                    && (TETROMINOES[self.cur_tetromino][self.cur_tetromino_rot][r][c] == 1)
                {
                    return false;
                }
            }
        }
        true
    }

    #[allow(clippy::needless_range_loop)]
    fn can_go_left(&self) -> bool {
        if self.cur_tetromino_col == 0 {
            return false;
        }
        for c in 0..(TETROMINO_FILL_COLS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
            for r in 0..(TETROMINO_FILL_ROWS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
                if (self.grid
                    [(r + self.cur_tetromino_row) * self.n_cols + c + self.cur_tetromino_col - 1]
                    != 0)
                    && (TETROMINOES[self.cur_tetromino][self.cur_tetromino_rot][r][c] == 1)
                {
                    return false;
                }
            }
        }
        true
    }

    #[allow(clippy::needless_range_loop)]
    fn can_go_right(&self) -> bool {
        if self.cur_tetromino_col
            == (self.n_cols
                - TETROMINO_FILL_COLS[self.cur_tetromino][self.cur_tetromino_rot] as usize)
        {
            return false;
        }

        for c in 0..(TETROMINO_FILL_COLS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
            for r in 0..(TETROMINO_FILL_ROWS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
                if (self.grid
                    [(r + self.cur_tetromino_row) * self.n_cols + c + self.cur_tetromino_col + 1]
                    != 0)
                    && (TETROMINOES[self.cur_tetromino][self.cur_tetromino_rot][r][c] == 1)
                {
                    return false;
                }
            }
        }

        true
    }

    #[allow(clippy::needless_range_loop)]
    fn can_hold(&self) -> bool {
        if !self.can_swap {
            return false;
        }
        let Some(held) = self.hold_tetromino else {
            return true;
        };
        let held_cols = TETROMINO_FILL_COLS[held][self.cur_tetromino_rot] as usize;
        let held_rows = TETROMINO_FILL_ROWS[held][self.cur_tetromino_rot] as usize;

        // Check if held piece would fit within bounds at current position
        if self.cur_tetromino_col + held_cols > self.n_cols {
            return false;
        }
        if self.cur_tetromino_row + held_rows > self.n_rows {
            return false;
        }

        for c in 0..held_cols {
            for r in 0..held_rows {
                if (self.grid
                    [(r + self.cur_tetromino_row) * self.n_cols + c + self.cur_tetromino_col]
                    != 0)
                    && (TETROMINOES[held][self.cur_tetromino_rot][r][c] == 1)
                {
                    return false;
                }
            }
        }
        true
    }

    #[allow(clippy::needless_range_loop)]
    fn can_rotate(&self) -> bool {
        let next_rot = (self.cur_tetromino_rot + 1) % NUM_ROTATIONS;
        if self.cur_tetromino_col
            > (self.n_cols - TETROMINO_FILL_COLS[self.cur_tetromino][next_rot] as usize)
        {
            return false;
        }
        if self.cur_tetromino_row
            > (self.n_rows - TETROMINO_FILL_ROWS[self.cur_tetromino][next_rot] as usize)
        {
            return false;
        }
        for c in 0..(TETROMINO_FILL_COLS[self.cur_tetromino][next_rot] as usize) {
            for r in 0..(TETROMINO_FILL_ROWS[self.cur_tetromino][next_rot] as usize) {
                if (self.grid
                    [(r + self.cur_tetromino_row) * self.n_cols + c + self.cur_tetromino_col]
                    != 0)
                    && (TETROMINOES[self.cur_tetromino][next_rot][r][c] == 1)
                {
                    return false;
                }
            }
        }
        true
    }

    fn is_full_row(&self, row: usize) -> bool {
        for c in 0..self.n_cols {
            if self.grid[row * self.n_cols + c] == 0 {
                return false;
            }
        }
        true
    }

    fn clear_row(&mut self, row: usize) {
        for r in (1..=row).rev() {
            for c in 0..self.n_cols {
                self.grid[r * self.n_cols + c] = self.grid[(r - 1) * self.n_cols + c];
            }
        }
        for c in 0..self.n_cols {
            self.grid[c] = 0;
        }
    }

    pub fn reset(&mut self) {
        self.score = 0;
        self.hold_tetromino = None;
        self.tick = 0;
        self.game_level = 1;
        self.ticks_per_fall = INITIAL_TICKS_PER_FALL;
        self.tick_fall = 0;
        self.can_swap = true;

        self.ep_return = 0.0;
        self.count_combos = 0;
        self.lines_deleted = 0;
        self.atn_count_hard_drop = 0;
        self.atn_count_soft_drop = 0;
        self.atn_count_rotate = 0;
        self.atn_count_hold = 0;
        self.tetromino_counts.fill(0);

        self.restore_grid();
        self.initialize_deck();
        self.spawn_new_tetromino();
    }

    #[allow(clippy::needless_range_loop)]
    fn place_tetromino(&mut self) {
        let mut row_to_check = self.cur_tetromino_row
            + TETROMINO_FILL_ROWS[self.cur_tetromino][self.cur_tetromino_rot] as usize
            - 1;
        let mut lines_deleted = 0;
        self.can_swap = true;

        // Fill the main grid with the tetromino
        for c in 0..(TETROMINO_FILL_COLS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
            for r in 0..(TETROMINO_FILL_ROWS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
                if TETROMINOES[self.cur_tetromino][self.cur_tetromino_rot][r][c] == 1 {
                    self.grid
                        [(r + self.cur_tetromino_row) * self.n_cols + c + self.cur_tetromino_col] =
                        (self.cur_tetromino + 1) as i32;
                }
            }
        }

        // Proceed to delete the complete rows
        for _ in 0..(TETROMINO_FILL_ROWS[self.cur_tetromino][self.cur_tetromino_rot] as usize) {
            if self.is_full_row(row_to_check) {
                self.clear_row(row_to_check);
                lines_deleted += 1;
            } else {
                row_to_check = row_to_check.saturating_sub(1);
            }
        }

        if lines_deleted > 0 {
            self.count_combos += 1;
            self.lines_deleted += lines_deleted;
            self.score += SCORE_COMBO[lines_deleted as usize] as usize;
            self.rewards += REWARD_COMBO[lines_deleted as usize];
            self.ep_return += REWARD_COMBO[lines_deleted as usize];

            // These determine the game difficulty. Consider making them args.
            self.game_level = 1 + self.lines_deleted / LINES_PER_LEVEL as u32;
            self.ticks_per_fall =
                (INITIAL_TICKS_PER_FALL as i32 - self.game_level as i32 / 4).max(3) as usize;
        }

        if self.can_spawn_new_tetromino() {
            self.spawn_new_tetromino();
        } else {
            self.is_terminal = true; // Game over
        }
    }

    pub fn step(&mut self, action: Action) {
        self.is_terminal = false;
        self.rewards = 0.0;
        self.tick += 1;
        self.tick_fall += 1;

        match action {
            Action::Left => {
                if self.can_go_left() {
                    self.cur_tetromino_col -= 1;
                } else {
                    self.rewards += REWARD_INVALID_ACTION;
                    self.ep_return += REWARD_INVALID_ACTION;
                }
            }
            Action::Right => {
                if self.can_go_right() {
                    self.cur_tetromino_col += 1;
                } else {
                    self.rewards += REWARD_INVALID_ACTION;
                    self.ep_return += REWARD_INVALID_ACTION;
                }
            }
            Action::Rotate => {
                self.atn_count_rotate += 1;
                if self.can_rotate() {
                    self.cur_tetromino_rot = (self.cur_tetromino_rot + 1) % NUM_ROTATIONS;
                    self.rewards += REWARD_ROTATE;
                    self.ep_return += REWARD_ROTATE;
                } else {
                    self.rewards += REWARD_INVALID_ACTION;
                    self.ep_return += REWARD_INVALID_ACTION;
                }
            }
            Action::SoftDrop => {
                self.atn_count_soft_drop += 1;
                if self.can_soft_drop() {
                    self.cur_tetromino_row += 1;
                    self.score += SCORE_SOFT_DROP;
                } else {
                    self.rewards += REWARD_INVALID_ACTION;
                    self.ep_return += REWARD_INVALID_ACTION;
                }
            }
            Action::Hold => {
                self.atn_count_hold += 1;
                if self.can_hold() {
                    let t1 = self.cur_tetromino;
                    match self.hold_tetromino {
                        None => {
                            self.spawn_new_tetromino();
                            self.hold_tetromino = Some(t1);
                            self.can_swap = false;
                        }
                        Some(t2) => {
                            self.cur_tetromino = t2;
                            self.tetromino_deck[self.cur_position_in_deck] = t2;
                            self.hold_tetromino = Some(t1);
                            self.can_swap = false;
                            self.cur_tetromino_rot = 0;
                            self.cur_tetromino_col = self.n_cols / 2;
                            self.cur_tetromino_row = 0;
                            self.tick_fall = 0;
                        }
                    }
                } else {
                    self.rewards += REWARD_INVALID_ACTION;
                    self.ep_return += REWARD_INVALID_ACTION;
                }
            }
            Action::HardDrop => {
                self.atn_count_hard_drop += 1;
                while self.can_soft_drop() {
                    self.cur_tetromino_row += 1;
                    // NOTE: this seems to be a super effective reward trick
                    self.rewards += REWARD_HARD_DROP;
                    self.ep_return += REWARD_HARD_DROP;
                }
                self.score += SCORE_HARD_DROP;
                self.place_tetromino();
            }
            Action::NoOp => {} // No operation
        }

        if self.tick_fall >= self.ticks_per_fall {
            self.tick_fall = 0;
            if self.can_soft_drop() {
                self.cur_tetromino_row += 1;
            } else {
                self.place_tetromino();
            }
        }

        if self.is_terminal || (self.tick >= MAX_TICKS) {
            self.reset();
        }
    }

    /// Create a render client
    pub fn render_client(&self) -> Client {
        let ui_rows = 1;
        let deck_rows = SIZE as i32;
        let total_rows = 1 + ui_rows + 1 + deck_rows + 1 + self.n_rows as i32 + 1;
        let total_cols = (1 + self.n_cols + 1).max(1 + 3 * NUM_PREVIEW) as i32;

        let (rl, thread) = raylib::init()
            .size(SQUARE_SIZE * total_cols, SQUARE_SIZE * total_rows)
            .title("Tetris")
            .build();

        Client {
            total_cols,
            total_rows,
            ui_rows,
            deck_rows,
            rl,
            thread,
        }
    }

    /// Render with the render client
    pub fn render(&mut self, client: &mut Client) {
        // Ensure we're on the main thread
        let main_thread_id = MAIN_THREAD_ID.get_or_init(|| thread::current().id());
        assert_eq!(
            *main_thread_id,
            thread::current().id(),
            "Rendering must be called from the main thread"
        );

        // Check for window close or escape key
        if client.rl.window_should_close() || client.rl.is_key_down(KeyboardKey::KEY_ESCAPE) {
            return;
        }

        // Toggle fullscreen with TAB
        if client.rl.is_key_pressed(KeyboardKey::KEY_TAB) {
            client.rl.toggle_fullscreen();
        }

        // Colors
        let border_color = Color::new(100, 100, 100, 255);
        let dash_color = Color::new(80, 80, 80, 255);
        let dash_color_bright = Color::new(150, 150, 150, 255);
        let dash_color_dark = Color::new(50, 50, 50, 255);

        let mut d = client.rl.begin_drawing(&client.thread);
        d.clear_background(Color::BLACK);

        // Draw outer grid border
        for r in 0..client.total_rows {
            for c in 0..client.total_cols {
                let x = c * SQUARE_SIZE;
                let y = r * SQUARE_SIZE;

                if (c == 0)
                    || (c == client.total_cols - 1)
                    || ((r > 1 + client.ui_rows) && (r < 1 + client.ui_rows + 1 + client.deck_rows))
                    || ((r > 1 + client.ui_rows + client.deck_rows + 1)
                        && (c >= self.n_rows as i32))
                    || (r == 0)
                    || (r == 1 + client.ui_rows)
                    || (r == 1 + client.ui_rows + 1 + client.deck_rows)
                    || (r == client.total_rows - 1)
                {
                    d.draw_rectangle(
                        x + HALF_LINEWIDTH,
                        y + HALF_LINEWIDTH,
                        SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                        SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                        border_color,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        2 * HALF_LINEWIDTH,
                        dash_color_dark,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y + SQUARE_SIZE - HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        2 * HALF_LINEWIDTH,
                        dash_color_dark,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        2 * HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        dash_color_dark,
                    );
                    d.draw_rectangle(
                        x + SQUARE_SIZE - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        2 * HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        dash_color_dark,
                    );
                }
            }
        }

        // Draw main grid
        for r in 0..self.n_rows {
            for c in 0..self.n_cols {
                let x = (c + 1) as i32 * SQUARE_SIZE;
                let y = (1 + client.ui_rows + 1 + client.deck_rows + 1 + r as i32) * SQUARE_SIZE;
                let block_id = self.grid[r * self.n_cols + c];

                let color = if block_id == 0 {
                    Color::BLACK
                } else if block_id < 0 {
                    TETROMINO_COLORS[(-block_id - 1) as usize]
                } else {
                    TETROMINO_COLORS[(block_id - 1) as usize]
                };

                d.draw_rectangle(
                    x + HALF_LINEWIDTH,
                    y + HALF_LINEWIDTH,
                    SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                    SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                    color,
                );
                d.draw_rectangle(
                    x - HALF_LINEWIDTH,
                    y - HALF_LINEWIDTH,
                    SQUARE_SIZE,
                    2 * HALF_LINEWIDTH,
                    dash_color,
                );
                d.draw_rectangle(
                    x - HALF_LINEWIDTH,
                    y + SQUARE_SIZE - HALF_LINEWIDTH,
                    SQUARE_SIZE,
                    2 * HALF_LINEWIDTH,
                    dash_color,
                );
                d.draw_rectangle(
                    x - HALF_LINEWIDTH,
                    y - HALF_LINEWIDTH,
                    2 * HALF_LINEWIDTH,
                    SQUARE_SIZE,
                    dash_color,
                );
                d.draw_rectangle(
                    x + SQUARE_SIZE - HALF_LINEWIDTH,
                    y - HALF_LINEWIDTH,
                    2 * HALF_LINEWIDTH,
                    SQUARE_SIZE,
                    dash_color,
                );
            }
        }

        // Draw current tetromino
        for r in 0..SIZE {
            for c in 0..SIZE {
                if TETROMINOES[self.cur_tetromino][self.cur_tetromino_rot][r][c] == 1 {
                    let x = (c + self.cur_tetromino_col + 1) as i32 * SQUARE_SIZE;
                    let y = (1
                        + client.ui_rows
                        + 1
                        + client.deck_rows
                        + 1
                        + r as i32
                        + self.cur_tetromino_row as i32)
                        * SQUARE_SIZE;
                    let color = TETROMINO_COLORS[self.cur_tetromino];

                    d.draw_rectangle(
                        x + HALF_LINEWIDTH,
                        y + HALF_LINEWIDTH,
                        SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                        SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                        color,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        2 * HALF_LINEWIDTH,
                        dash_color,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y + SQUARE_SIZE - HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        2 * HALF_LINEWIDTH,
                        dash_color,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        2 * HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        dash_color,
                    );
                    d.draw_rectangle(
                        x + SQUARE_SIZE - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        2 * HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        dash_color,
                    );
                }
            }
        }

        // Draw deck preview (next pieces)
        for i in 0..NUM_PREVIEW {
            let deck_idx = (self.cur_position_in_deck + 1 + i) % DECK_SIZE;
            let tetromino_id = self.tetromino_deck[deck_idx];
            for r in 0..SIZE {
                for c in 0..2 {
                    let x = (c + 1 + 3 * i) as i32 * SQUARE_SIZE;
                    let y = (1 + client.ui_rows + 1 + r as i32) * SQUARE_SIZE;
                    let r_offset = SIZE - TETROMINO_FILL_ROWS[tetromino_id][0] as usize;

                    let color = if r < r_offset {
                        Color::BLACK
                    } else if TETROMINOES[tetromino_id][0][r - r_offset][c] == 0 {
                        Color::BLACK
                    } else {
                        TETROMINO_COLORS[tetromino_id]
                    };

                    d.draw_rectangle(
                        x + HALF_LINEWIDTH,
                        y + HALF_LINEWIDTH,
                        SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                        SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                        color,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        2 * HALF_LINEWIDTH,
                        dash_color_bright,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y + SQUARE_SIZE - HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        2 * HALF_LINEWIDTH,
                        dash_color_bright,
                    );
                    d.draw_rectangle(
                        x - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        2 * HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        dash_color_bright,
                    );
                    d.draw_rectangle(
                        x + SQUARE_SIZE - HALF_LINEWIDTH,
                        y - HALF_LINEWIDTH,
                        2 * HALF_LINEWIDTH,
                        SQUARE_SIZE,
                        dash_color_bright,
                    );
                }
            }
        }

        // Draw hold tetromino
        for r in 0..SIZE {
            for c in 0..2 {
                let x = (client.total_cols - 3 + c as i32) * SQUARE_SIZE;
                let y = (1 + client.ui_rows + 1 + r as i32) * SQUARE_SIZE;

                let color = if let Some(hold_id) = self.hold_tetromino {
                    let r_offset = SIZE - TETROMINO_FILL_ROWS[hold_id][0] as usize;
                    if r < r_offset || TETROMINOES[hold_id][0][r - r_offset][c] == 0 {
                        Color::BLACK
                    } else {
                        TETROMINO_COLORS[hold_id]
                    }
                } else {
                    Color::BLACK
                };

                d.draw_rectangle(
                    x + HALF_LINEWIDTH,
                    y + HALF_LINEWIDTH,
                    SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                    SQUARE_SIZE - 2 * HALF_LINEWIDTH,
                    color,
                );
                d.draw_rectangle(
                    x - HALF_LINEWIDTH,
                    y - HALF_LINEWIDTH,
                    SQUARE_SIZE,
                    2 * HALF_LINEWIDTH,
                    dash_color_bright,
                );
                d.draw_rectangle(
                    x - HALF_LINEWIDTH,
                    y + SQUARE_SIZE - HALF_LINEWIDTH,
                    SQUARE_SIZE,
                    2 * HALF_LINEWIDTH,
                    dash_color_bright,
                );
                d.draw_rectangle(
                    x - HALF_LINEWIDTH,
                    y - HALF_LINEWIDTH,
                    2 * HALF_LINEWIDTH,
                    SQUARE_SIZE,
                    dash_color_bright,
                );
                d.draw_rectangle(
                    x + SQUARE_SIZE - HALF_LINEWIDTH,
                    y - HALF_LINEWIDTH,
                    2 * HALF_LINEWIDTH,
                    SQUARE_SIZE,
                    dash_color_bright,
                );
            }
        }

        // Draw UI text
        d.draw_text(
            &format!("Score: {}", self.score),
            SQUARE_SIZE + 4,
            SQUARE_SIZE + 4,
            28,
            Color::new(255, 160, 160, 255),
        );
        d.draw_text(
            &format!("Lvl: {}", self.game_level),
            (client.total_cols - 4) * SQUARE_SIZE,
            SQUARE_SIZE + 4,
            28,
            Color::new(160, 255, 160, 255),
        );
    }
}

const NUM_TETROMINOES: usize = 7;
const NUM_ROTATIONS: usize = 4;
const SIZE: usize = 4;

#[allow(dead_code)]
const TETROMINO_COLORS: [Color; 8] = [
    Color::new(255, 255, 0, 255), // Yellow
    Color::new(255, 255, 0, 255), // Yellow
    Color::new(0, 255, 255, 255), // Cyan
    Color::new(0, 255, 0, 255),   // Green
    Color::new(255, 0, 0, 255),   // Red
    Color::new(128, 0, 128, 255), // Purple
    Color::new(255, 165, 0, 255), // Orange
    Color::new(0, 0, 255, 255),   // Blue
];

const TETROMINOES: [[[[u8; SIZE]; SIZE]; NUM_ROTATIONS]; NUM_TETROMINOES] = [
    [
        [[1, 1, 0, 0], [1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 0, 0], [1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 0, 0], [1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 0, 0], [1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
    ],
    [
        [[1, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0]],
        [[1, 1, 1, 1], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0]],
        [[1, 1, 1, 1], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
    ],
    [
        [[1, 0, 0, 0], [1, 1, 0, 0], [0, 1, 0, 0], [0, 0, 0, 0]],
        [[0, 1, 1, 0], [1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 0, 0, 0], [1, 1, 0, 0], [0, 1, 0, 0], [0, 0, 0, 0]],
        [[0, 1, 1, 0], [1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
    ],
    [
        [[0, 1, 0, 0], [1, 1, 0, 0], [1, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 0, 0], [0, 1, 1, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[0, 1, 0, 0], [1, 1, 0, 0], [1, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 0, 0], [0, 1, 1, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
    ],
    [
        [[0, 1, 0, 0], [1, 1, 0, 0], [0, 1, 0, 0], [0, 0, 0, 0]],
        [[0, 1, 0, 0], [1, 1, 1, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 0, 0, 0], [1, 1, 0, 0], [1, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 1, 0], [0, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
    ],
    [
        [[1, 0, 0, 0], [1, 0, 0, 0], [1, 1, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 1, 0], [1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 0, 0], [0, 1, 0, 0], [0, 1, 0, 0], [0, 0, 0, 0]],
        [[0, 0, 1, 0], [1, 1, 1, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
    ],
    [
        [[0, 1, 0, 0], [0, 1, 0, 0], [1, 1, 0, 0], [0, 0, 0, 0]],
        [[1, 0, 0, 0], [1, 1, 1, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0], [0, 0, 0, 0]],
        [[1, 1, 1, 0], [0, 0, 1, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
    ],
];

const TETROMINO_FILL_COLS: [[u8; NUM_ROTATIONS]; NUM_TETROMINOES] = [
    [2, 2, 2, 2],
    [1, 4, 1, 4],
    [2, 3, 2, 3],
    [2, 3, 2, 3],
    [2, 3, 2, 3],
    [2, 3, 2, 3],
    [2, 3, 2, 3],
];

const TETROMINO_FILL_ROWS: [[u8; NUM_ROTATIONS]; NUM_TETROMINOES] = [
    [2, 2, 2, 2],
    [4, 1, 4, 1],
    [3, 2, 3, 2],
    [3, 2, 3, 2],
    [3, 2, 3, 2],
    [3, 2, 3, 2],
    [3, 2, 3, 2],
];

impl Game for Tetris {
    fn print_instructions(&self) {
        println!("Tetris with MCTS Agent");
        println!("======================");
        println!("Watch it go...");
    }

    fn current_reward(&self) -> f64 {
        self.rewards as f64
    }

    fn result(&self) -> Option<GameResult> {
        if self.is_terminal {
            Some(GameResult::End(self.rewards as f64))
        } else {
            None
        }
    }

    fn allowed_actions(&self) -> Vec<super::Action> {
        let mut actions = Vec::with_capacity(7);
        actions.push(Action::NoOp as usize);
        if self.can_go_left() {
            actions.push(Action::Left as usize);
        }
        if self.can_go_right() {
            actions.push(Action::Right as usize);
        }
        if self.can_rotate() {
            actions.push(Action::Rotate as usize);
        }
        if self.can_soft_drop() {
            actions.push(Action::SoftDrop as usize);
        }
        if self.can_spawn_new_tetromino() {
            actions.push(Action::HardDrop as usize);
        }
        if self.can_hold() {
            actions.push(Action::Hold as usize);
        }
        actions
    }

    fn current_player(&self) -> super::Player {
        Player::X
    }

    fn step(&mut self, action: super::Action) -> Result<(), &'static str> {
        let action = Action::from(action as u8);
        self.step(action);
        Ok(())
    }
}
