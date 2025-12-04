#![warn(clippy::all, clippy::pedantic)]

mod game;
mod mcts;

use game::{GameResult, Player, TicTacToe};
use mcts::Mcts;
use std::io::{self, Write};

use crate::game::Game;

fn main() {
    println!("Tic-Tac-Toe with MCTS Agent");
    println!("============================");
    println!("You are X, MCTS agent is O");
    println!("Enter positions 0-8:");
    println!("0 | 1 | 2");
    println!("---------");
    println!("3 | 4 | 5");
    println!("---------");
    println!("6 | 7 | 8");
    println!();

    let mut game = TicTacToe::default();
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
