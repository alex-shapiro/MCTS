use std::fmt;

use super::{Action, Game, GameResult, Player};

const ROWS: usize = 6;
const COLS: usize = 7;

type Cell = Option<Player>;

#[derive(Debug, Clone)]
pub struct Connect4 {
    board: [[Cell; COLS]; ROWS],
    current_player: Player,
    result: Option<GameResult>,
}

impl Connect4 {
    pub fn is_terminal(&self) -> bool {
        self.result.is_some()
    }

    fn update_result(&mut self) {
        // Check horizontal wins
        for row in 0..ROWS {
            for col in 0..COLS - 3 {
                if let Some(player) = self.board[row][col] {
                    if (0..4).all(|i| self.board[row][col + i] == Some(player)) {
                        self.result = Some(GameResult::Win(player));
                        return;
                    }
                }
            }
        }

        // Check vertical wins
        for row in 0..ROWS - 3 {
            for col in 0..COLS {
                if let Some(player) = self.board[row][col] {
                    if (0..4).all(|i| self.board[row + i][col] == Some(player)) {
                        self.result = Some(GameResult::Win(player));
                        return;
                    }
                }
            }
        }

        // Check diagonal wins (bottom-left to top-right)
        for row in 3..ROWS {
            for col in 0..COLS - 3 {
                if let Some(player) = self.board[row][col] {
                    if (0..4).all(|i| self.board[row - i][col + i] == Some(player)) {
                        self.result = Some(GameResult::Win(player));
                        return;
                    }
                }
            }
        }

        // Check diagonal wins (top-left to bottom-right)
        for row in 0..ROWS - 3 {
            for col in 0..COLS - 3 {
                if let Some(player) = self.board[row][col] {
                    if (0..4).all(|i| self.board[row + i][col + i] == Some(player)) {
                        self.result = Some(GameResult::Win(player));
                        return;
                    }
                }
            }
        }

        // Check for draw (board full)
        if self.board[0].iter().all(Option::is_some) {
            self.result = Some(GameResult::Draw);
        }
    }

    fn drop_piece(&mut self, col: usize) -> Result<(), &'static str> {
        // Find the lowest empty row in this column
        for row in (0..ROWS).rev() {
            if self.board[row][col].is_none() {
                self.board[row][col] = Some(self.current_player);
                return Ok(());
            }
        }
        Err("Column is full")
    }
}

impl Default for Connect4 {
    fn default() -> Self {
        Connect4 {
            board: [[None; COLS]; ROWS],
            current_player: Player::X,
            result: None,
        }
    }
}

impl fmt::Display for Connect4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print column numbers
        for col in 0..COLS {
            write!(f, "{col} ")?;
        }
        writeln!(f)?;

        // Print board
        for row in 0..ROWS {
            for col in 0..COLS {
                if let Some(player) = self.board[row][col] {
                    write!(f, "{player}")?;
                } else {
                    write!(f, ".")?;
                }
                write!(f, " ")?;
            }
            if row < ROWS - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl Game for Connect4 {
    fn print_instructions(&self) {
        println!("Connect 4 with MCTS Agent");
        println!("=========================");
        println!("You are X, MCTS agent is O");
        println!("Enter column number (0-6) to drop your piece.");
        println!("Connect 4 pieces horizontally, vertically, or diagonally to win!");
        println!();
    }

    fn result(&self) -> Option<GameResult> {
        self.result
    }

    fn allowed_actions(&self) -> Vec<Action> {
        if self.is_terminal() {
            return Vec::new();
        }
        // A column is playable if the top cell is empty
        (0..COLS)
            .filter(|&col| self.board[0][col].is_none())
            .collect()
    }

    fn current_player(&self) -> Player {
        self.current_player
    }

    fn step(&mut self, action: Action) -> Result<(), &'static str> {
        if action >= COLS {
            return Err("Column out of bounds");
        }
        if self.board[0][action].is_some() {
            return Err("Column is full");
        }
        if self.is_terminal() {
            return Err("Game already finished");
        }

        self.drop_piece(action)?;
        self.update_result();
        self.current_player = self.current_player.opponent();
        Ok(())
    }
}
