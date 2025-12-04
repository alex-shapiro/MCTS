pub mod tictactoe;

use std::fmt::{self, Debug};

pub type Action = usize;

pub trait Game: Debug + Clone {
    fn print_instructions(&self);
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
pub enum GameResult {
    Win(Player),
    Draw,
}
