#![warn(clippy::all, clippy::pedantic)]

mod game;
mod mcts;

use game::{GameResult, Player, TicTacToe};
use mcts::MCTSAgent;
use std::io::{self, Write};

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

    let mut game = TicTacToe::new();
    let mut agent = MCTSAgent::new(10000, 1.41);

    while !game.is_terminal() {
        println!("{game}\n");

        match game.current_player() {
            Player::X => {
                print!("Your move (0-8): ");
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();

                if let Ok(pos) = input.trim().parse::<usize>() {
                    if let Err(e) = game.make_move(pos) {
                        println!("Invalid move: {e}");
                    }
                } else {
                    println!("Please enter a number 0-8");
                }
            }
            Player::O => {
                println!("MCTS is thinking...");
                if let Some(action) = agent.choose_move(&game) {
                    println!("MCTS plays: {action}");
                    game.make_move(action).unwrap();
                }
            }
        }
    }

    println!("\nFinal board:\n{game}\n");

    match game.result() {
        GameResult::Win(Player::X) => println!("You win!"),
        GameResult::Win(Player::O) => println!("MCTS wins!"),
        GameResult::Draw => println!("It's a draw!"),
        GameResult::InProgress => unreachable!(),
    }
}
