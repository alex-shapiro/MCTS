#![warn(clippy::all, clippy::pedantic)]

mod game;
mod mcts;

use argh::FromArgs;
use game::{Game, GameResult, Player, connect4::Connect4, tictactoe::TicTacToe};
use mcts::Mcts;
use std::io::{self, Write};

use crate::game::tetris::Tetris;

#[derive(FromArgs)]
/// Play games against an MCTS agent
struct Args {
    #[argh(subcommand)]
    game: GameCommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum GameCommand {
    TicTacToe(TicTacToeCmd),
    Connect4(Connect4Cmd),
    Tetris(TetrisCmd),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "tictactoe")]
/// Play Tic-Tac-Toe
struct TicTacToeCmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "connect4")]
/// Play Connect 4
struct Connect4Cmd {}

#[derive(FromArgs)]
#[argh(subcommand, name = "tetris")]
/// Play Connect 4
struct TetrisCmd {}

fn main() {
    let args: Args = argh::from_env();

    match args.game {
        GameCommand::TicTacToe(_) => play_game(TicTacToe::default()),
        GameCommand::Connect4(_) => play_game(Connect4::default()),
        GameCommand::Tetris(_) => play_tetris(Tetris::new()),
    }
}

fn play_game<G: Game + std::fmt::Display>(mut game: G) {
    game.print_instructions();

    let mut agent = Mcts::new(10_000);

    loop {
        println!("{game}\n");

        match game.current_player() {
            Player::X => {
                let actions = game.allowed_actions();
                let max_action = actions.iter().max().unwrap_or(&0);
                print!("Your move (0-{max_action}): ");
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();

                if let Ok(pos) = input.trim().parse::<usize>() {
                    if let Err(e) = game.step(pos) {
                        println!("Invalid move: {e}");
                    }
                } else {
                    println!("Please enter a valid number");
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
