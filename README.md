# MCTS

Toy implementation of MCTS

## Getting Started

Play Tic Tac Toe

```sh
cargo r -- tictactoe
```

Play Connect4

```sh
cargo r -- connect4
```

Play Tetris (badly)

```sh
cargo r --release -- tetris
```

The problem with Tetris is: 

- the state space is much larger than Tic Tac Toe or Connect4
- MCTS simulation never improves beyond random plays

Without infinite compute, there is no reasonable way to guide vanilla MCTS toward the subtrees that lead to reasonable outcomes over time
This is where a better-than-random simulation policies (heavy playthroughs, NNs) become useful
