use std::fmt::{self, Debug};

pub type Action = usize;

pub trait Game: Debug + Clone {
    fn result(&self) -> Option<GameResult>;
    fn allowed_actions(&self) -> Vec<Action>;
    fn current_player(&self) -> Player;
    fn step(&mut self, action: Action) -> Result<(), &'static str>;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Player {
    X,
    O,
}

impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Player::X => write!(f, "X"),
            Player::O => write!(f, "O"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Empty,
    Occupied(Player),
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::Empty => write!(f, "."),
            Cell::Occupied(p) => write!(f, "{p}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameResult {
    Win(Player),
    Draw,
}

#[derive(Debug, Clone)]
pub struct TicTacToe {
    board: [Cell; 9],
    current_player: Player,
    result: Option<GameResult>,
}

impl TicTacToe {
    pub fn is_terminal(&self) -> bool {
        self.result.is_some()
    }

    fn update_result(&mut self) {
        const WIN_LINES: [[usize; 3]; 8] = [
            [0, 1, 2], // top row
            [3, 4, 5], // middle row
            [6, 7, 8], // bottom row
            [0, 3, 6], // left column
            [1, 4, 7], // middle column
            [2, 5, 8], // right column
            [0, 4, 8], // main diagonal
            [2, 4, 6], // anti-diagonal
        ];

        for line in WIN_LINES {
            let cells: Vec<Cell> = line.iter().map(|&i| self.board[i]).collect();
            if let Cell::Occupied(player) = cells[0]
                && cells.iter().all(|&c| c == Cell::Occupied(player))
            {
                self.result = Some(GameResult::Win(player));
                return;
            }
        }

        if self.board.iter().all(|&c| c != Cell::Empty) {
            self.result = Some(GameResult::Draw);
        }
    }
}

impl Default for TicTacToe {
    fn default() -> Self {
        TicTacToe {
            board: [Cell::Empty; 9],
            current_player: Player::X,
            result: None,
        }
    }
}

impl fmt::Display for TicTacToe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in 0..3 {
            for col in 0..3 {
                write!(f, "{}", self.board[row * 3 + col])?;
                if col < 2 {
                    write!(f, " ")?;
                }
            }
            if row < 2 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl Game for TicTacToe {
    fn result(&self) -> Option<GameResult> {
        self.result
    }

    fn allowed_actions(&self) -> Vec<Action> {
        if self.is_terminal() {
            return Vec::new();
        }
        self.board
            .iter()
            .enumerate()
            .filter(|(_, cell)| **cell == Cell::Empty)
            .map(|(i, _)| i)
            .collect()
    }

    fn current_player(&self) -> Player {
        self.current_player
    }

    fn step(&mut self, action: Action) -> Result<(), &'static str> {
        if action >= 9 {
            return Err("Position out of bounds");
        }
        if self.board[action] != Cell::Empty {
            return Err("Cell already occupied");
        }
        if self.is_terminal() {
            return Err("Game already finished");
        }

        self.board[action] = Cell::Occupied(self.current_player);
        self.update_result();
        self.current_player = self.current_player.opponent();
        Ok(())
    }
}
