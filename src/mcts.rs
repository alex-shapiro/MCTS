use crate::game::{GameResult, Player, TicTacToe};

pub struct MCTSAgent {
    mcts: Mcts,
    iterations: u32,
}

impl MCTSAgent {
    pub fn new(iterations: u32, exploration_constant: f64) -> Self {
        MCTSAgent {
            mcts: Mcts::new(exploration_constant),
            iterations,
        }
    }

    pub fn choose_move(&mut self, state: &TicTacToe) -> Option<usize> {
        self.mcts.search(state, self.iterations)
    }
}

struct Mcts {
    nodes: Vec<Node>,
    exploration_constant: f64,
}

impl Mcts {
    fn new(exploration_constant: f64) -> Self {
        Mcts {
            nodes: Vec::new(),
            exploration_constant,
        }
    }

    fn search(&mut self, state: &TicTacToe, iterations: u32) -> Option<usize> {
        self.nodes.clear();
        self.nodes.push(Node::new(state.clone(), None, None));

        let root_player = state.current_player();

        for _ in 0..iterations {
            let selected = self.select(0);
            let expanded = self.expand(selected);
            let result = self.simulate(expanded);
            self.backpropagate(expanded, result, root_player);
        }

        self.best_action(0)
    }

    /// Select the first viable node
    /// - terminal, OR
    /// - not fully expanded; OR
    /// - has no children
    fn select(&self, node_idx: usize) -> usize {
        let mut current = node_idx;

        loop {
            let node = &self.nodes[current];
            if node.is_terminal() {
                break;
            }
            if !node.is_fully_expanded() {
                break;
            }
            if node.children.is_empty() {
                break;
            }
            current = self.best_child(current);
        }

        current
    }

    fn best_child(&self, node_idx: usize) -> usize {
        let parent_visits = self.nodes[node_idx].visits;
        let exploration = self.exploration_constant;

        *self.nodes[node_idx]
            .children
            .iter()
            .max_by(|&&a, &&b| {
                let ucb_a = self.nodes[a].ucb1(parent_visits, exploration);
                let ucb_b = self.nodes[b].ucb1(parent_visits, exploration);
                ucb_a.partial_cmp(&ucb_b).unwrap()
            })
            .unwrap()
    }

    fn expand(&mut self, node_idx: usize) -> usize {
        if self.nodes[node_idx].is_terminal() {
            return node_idx;
        }

        if self.nodes[node_idx].untried_actions.is_empty() {
            return node_idx;
        }

        let action = self.nodes[node_idx].untried_actions.pop().unwrap();
        let mut new_state = self.nodes[node_idx].state.clone();
        new_state.make_move(action).unwrap();

        let new_node = Node::new(new_state, Some(node_idx), Some(action));
        let new_idx = self.nodes.len();
        self.nodes.push(new_node);
        self.nodes[node_idx].children.push(new_idx);

        new_idx
    }

    fn simulate(&self, node_idx: usize) -> Option<GameResult> {
        let mut state = self.nodes[node_idx].state.clone();

        while !state.is_terminal() {
            let moves = state.legal_moves();
            let random_move = moves[fastrand::usize(..moves.len())];
            state.make_move(random_move).unwrap();
        }

        state.result()
    }

    fn backpropagate(&mut self, node_idx: usize, result: Option<GameResult>, root_player: Player) {
        let mut current = Some(node_idx);

        while let Some(idx) = current {
            self.nodes[idx].visits += 1;

            let reward = match result {
                Some(GameResult::Win(winner)) => {
                    let node_player = if idx == 0 {
                        root_player
                    } else {
                        self.nodes[self.nodes[idx].parent.unwrap()]
                            .state
                            .current_player()
                    };
                    if winner == node_player { 1.0 } else { 0.0 }
                }
                Some(GameResult::Draw) => 0.5,
                None => 0.0,
            };

            self.nodes[idx].wins += reward;
            current = self.nodes[idx].parent;
        }
    }

    fn best_action(&self, node_idx: usize) -> Option<usize> {
        if self.nodes[node_idx].children.is_empty() {
            return None;
        }

        let best_child_idx = *self.nodes[node_idx]
            .children
            .iter()
            .max_by_key(|&&child_idx| self.nodes[child_idx].visits)
            .unwrap();

        self.nodes[best_child_idx].action
    }
}

#[derive(Clone)]
struct Node {
    state: TicTacToe,
    parent: Option<usize>,
    children: Vec<usize>,
    action: Option<usize>,
    visits: u32,
    wins: f64,
    untried_actions: Vec<usize>,
}

impl Node {
    fn new(state: TicTacToe, parent: Option<usize>, action: Option<usize>) -> Self {
        let untried_actions = state.legal_moves();
        Node {
            state,
            parent,
            children: Vec::new(),
            action,
            visits: 0,
            wins: 0.0,
            untried_actions,
        }
    }

    fn is_fully_expanded(&self) -> bool {
        self.untried_actions.is_empty()
    }

    fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    fn ucb1(&self, parent_visits: u32, exploration: f64) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        let exploitation = self.wins / f64::from(self.visits);
        let exploration_term =
            exploration * (f64::from(parent_visits).ln() / f64::from(self.visits)).sqrt();
        exploitation + exploration_term
    }
}
