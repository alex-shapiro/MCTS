#![warn(clippy::all, clippy::pedantic)]

mod game;
mod mcts;

use game::{GameResult, Player};
use mcts::Mcts;
use std::io::{self, Write};

use crate::game::{Game, tictactoe::TicTacToe};

fn main() {
    let mut game = TicTacToe::default();
    game.print_instructions();

    let mut agent = Mcts::new(10_000);

    loop {
        println!("{game}\n");

        match game.current_player() {
            Player::X => {
                print!("Your move (0-8): ");
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();

                if let Ok(pos) = input.trim().parse::<usize>() {
                    if let Err(e) = game.step(pos) {
                        println!("Invalid move: {e}");
                    }
                } else {
                    println!("Please enter a number 0-8");
                }
            }
            Player::O => {
                println!("MCTS is thinking...");
                if let Some(action) = agent.search(&game) {
                    println!("MCTS plays: {action}");
                    game.step(action).unwrap();
                }
            }
        }

        if let Some(result) = game.result() {
            match result {
                GameResult::Win(Player::X) => println!("You win!"),
                GameResult::Win(Player::O) => println!("MCTS wins!"),
                GameResult::Draw => println!("It's a draw!"),
            }
            println!("\nFinal board:\n{game}\n");
            break;
        }
    }
}
