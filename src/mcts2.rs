use std::f64;

use crate::game::{Action, Game, GameResult};

#[derive(Debug)]
pub struct Mcts<S> {
    num_iters: u32,
    nodes: Vec<Node<S>>,
}

impl<S: Game> Mcts<S> {
    pub fn new(num_iters: u32, _: f64) -> Self {
        Self {
            num_iters,
            nodes: vec![],
        }
    }

    pub fn search(&mut self, state: &S) -> Option<Action> {
        self.nodes.clear();
        self.nodes.push(Node::new(None, state.clone(), None));

        for _ in 0..self.num_iters {
            let node_idx = self.select();
            let node_idx = self.expand(node_idx);
            let result = self.simulate(node_idx);
            self.backup(node_idx, result);
        }

        self.best_action()
    }

    /// Walk the tree to find the node that should be expanded.
    ///
    /// - Always start with the root node
    /// - Stop if the node is terminal or not fully expanded.
    /// - If the node is nonterminal and fully expanded,
    ///   walk to the child with the highest UCT score.
    fn select(&self) -> usize {
        let mut idx = 0;
        loop {
            let node = &self.nodes[idx];
            if node.is_terminal() {
                return idx;
            } else if node.is_fully_expanded() {
                idx = self.best_child(idx);
            } else {
                return idx;
            }
        }
    }

    /// Expand the node iff it is nonterminal and not fully expanded
    fn expand(&mut self, node_idx: usize) -> usize {
        // if the node is terminal or fully expanded, return the node idx
        let node = &mut self.nodes[node_idx];
        if node.is_terminal() {
            return node_idx;
        }

        // step the game state with the next untried action
        let Some(action) = node.untried_actions.pop() else {
            return node_idx;
        };

        let mut state = node.state.clone();
        state.step(action).unwrap();

        // insert the new child node into the tree
        let child_node = Node::new(Some(node_idx), state, Some(action));
        let child_idx = self.nodes.len();
        self.nodes.push(child_node);
        self.nodes[node_idx].children.push(child_idx);
        child_idx
    }

    fn simulate(&self, node_idx: usize) -> GameResult {
        let mut state = self.nodes[node_idx].state.clone();
        loop {
            if let Some(result) = state.result() {
                return result;
            }
            let actions = state.allowed_actions();
            let action = actions[fastrand::usize(..actions.len())];
            state.step(action).unwrap();
        }
    }

    fn backup(&mut self, node_idx: usize, result: GameResult) {
        let mut current = Some(node_idx);

        while let Some(idx) = current {
            let node = &mut self.nodes[idx];
            let actor = node.state.current_player().opponent();
            node.visits += 1.0;
            node.reward += match result {
                GameResult::Win(winner) => {
                    if winner == actor {
                        1.0
                    } else {
                        0.0
                    }
                }
                GameResult::Draw => 0.5,
            };
            current = node.parent;
        }
    }

    fn best_action(&self) -> Option<Action> {
        self.nodes[0]
            .children
            .iter()
            .map(|idx| &self.nodes[*idx])
            .max_by(|a, b| a.visits.partial_cmp(&b.visits).unwrap())
            .unwrap()
            .action
    }

    /// Find the child with highest UCT score
    fn best_child(&self, node_idx: usize) -> usize {
        let node = &self.nodes[node_idx];
        let node_visits = node.visits;
        *node
            .children
            .iter()
            .map(|idx| (idx, self.nodes[*idx].uct(node_visits)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap()
            .0
    }
}

#[derive(Debug)]
struct Node<S> {
    state: S,
    parent: Option<usize>,
    children: Vec<usize>,
    action: Option<Action>,
    visits: f64,
    reward: f64,
    untried_actions: Vec<Action>,
}

impl<S: Game> Node<S> {
    fn new(parent: Option<usize>, state: S, action: Option<usize>) -> Self {
        let untried_actions = state.allowed_actions();
        Self {
            state,
            parent,
            children: vec![],
            action,
            visits: 0.0,
            reward: 0.0,
            untried_actions,
        }
    }

    fn is_terminal(&self) -> bool {
        self.state.result().is_some()
    }

    fn is_fully_expanded(&self) -> bool {
        self.untried_actions.is_empty()
    }

    fn uct(&self, parent_visits: f64) -> f64 {
        if self.visits == 0.0 {
            return f64::INFINITY;
        }
        let r_exploit = self.reward / self.visits;
        let r_explore = (2.0 * parent_visits.ln() / self.visits).sqrt();
        r_exploit + r_explore
    }
}
